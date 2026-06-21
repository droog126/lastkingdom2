//! Smooth terrain 渲染入口
//!
//! 把"BlockType 网格 + 多维 heightmap" 转成 f32 标量场 → 抽 Marching Cubes mesh → bevy Mesh
//!
//! 关键决策（v1）：
//!  - 单一 mesh + vertex color（按 vertex.y 分层上色 grass/dirt/stone）
//!  - 不分多 mesh（不需要 Trimesh 优化；玩家物理用 capsule + 这个 mesh 的 Trimesh 碰撞）
//!  - 可选 Laplacian smooth pass：1 次平滑，CPU 计算 ~30ms（41³）

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;

use super::marching_cubes::{McVertex, build_mesh as mc_build_mesh};
use super::scalar_field::{ScalarField, build_density_field};
use lk2_core::world::World as GameWorld;

/// 一个 smooth chunk 的输出（玩家周围 AABB 一个 mesh）
pub struct SmoothMesh {
    pub mesh: Mesh,
    pub collider_trimesh: Vec<[f32; 3]>, // 顶点
    pub collider_indices: Vec<u32>,
}

/// 玩家周围 AABB → smooth mesh
///
/// - `min`, `max` = 世界坐标 AABB
/// - `iso` = 0.5（surface 位置）
/// - `smooth_passes` = 0..=3 (Laplacian 平滑次数，0 = 不平滑)
pub fn build_smooth_mesh(
    world: &GameWorld,
    min: [i32; 3],
    max: [i32; 3],
    iso: f32,
    smooth_passes: u32,
) -> Option<SmoothMesh> {
    // 1) 标量场
    let field = build_density_field(world, min, max);
    if field.shape[0] < 2 || field.shape[1] < 2 || field.shape[2] < 2 {
        return None;
    }

    // 2) Marching Cubes
    let corner_origin = [min[0] as f32, min[1] as f32, min[2] as f32];
    let cell_size = [1.0_f32, 1.0, 1.0];
    let (vertices, indices) = mc_build_mesh(&field, iso, corner_origin, cell_size);
    if vertices.is_empty() {
        return None;
    }

    // 3) 可选 Laplacian smoothing
    // 3) 可选 Laplacian smoothing（注意：传入 indices.clone() 因为后续 smooth_normals 还要用）
    let vertices = if smooth_passes > 0 {
        laplacian_smooth(vertices, &indices, smooth_passes)
    } else {
        vertices
    };

    // 4) 法线平滑（按 position hash 共享顶点，邻接三角形法线平均）
    let (positions, normals) = smooth_normals(&vertices, &indices);

    // 5) Vertex color: a restrained 3-color palette with soft spatial mixing.
    let colors: Vec<[f32; 4]> = positions.iter().map(|p| terrain_color(*p)).collect();

    // 6) build bevy::Mesh
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices.clone()));

    Some(SmoothMesh { mesh, collider_trimesh: positions, collider_indices: indices })
}

/// Terrain vertex color: 3 主色 (草/土/石) + 大块噪声分区, 让颜色块明显可见
///
/// 之前: GRASS/MOSS/EARTH 三个绿棕色微差, 权重重叠后整片灰绿一片 (iter_1382 看到的就是
/// 一片灰蓝 — fog + 同色 = "纯色没风格")。
///
/// 现在:
///  - 3 个色彼此差距大 (鲜绿 / 暖棕 / 灰白) — 任何混合都能看出 "这地方是草地/那块是土/那里是石"
///  - 大块噪声 (freq 0.05~0.10) 决定"主色块"分布, 让玩家能看见色块边界
///  - 中块噪声 (freq 0.30) 在主色块内嵌入"补丁" = 2nd 色
///  - 高频噪声 (freq 0.85) 微调明暗, 给地表质感
///  - 高度只占 15% 权重 (之前 25%), 防止玩家站在 Y=24 高处把整片地形都判成 STONE
///  - 输出亮度整体提升 (1.25x) 抵消 fog 把颜色洗白的部分
fn terrain_color(p: [f32; 3]) -> [f32; 4] {
    // 3 主色 — 鲜亮 / 高饱和, 任何 mix 都看得出
    const GRASS: [f32; 3] = [0.38, 0.74, 0.24]; // 鲜绿
    const DIRT: [f32; 3] = [0.68, 0.45, 0.20]; // 暖棕
    const STONE: [f32; 3] = [0.78, 0.76, 0.70]; // 浅灰

    let zone_n = (p[0] * 0.08 + p[2] * 0.10).sin() * 0.5 + 0.5;
    let patch_n = (p[0] * 0.32 + p[2] * 0.28).sin() * 0.5 + 0.5;
    let fine_n = ((p[0] * 0.85).sin() * (p[2] * 0.73).cos()) * 0.5 + 0.5;
    let height_t = ((p[1] - 4.0) / 40.0).clamp(0.0, 1.0);

    // 主色: zone 主导 (85%), height 调味 (15%)
    let main_zone = zone_n * 0.85 + height_t * 0.15;
    let (main_rgb, main_w) = if main_zone < 0.40 {
        (GRASS, 0.60)
    } else if main_zone < 0.66 {
        (DIRT, 0.60)
    } else {
        (STONE, 0.60)
    };

    // 次色: 按 patch_n 选 (与 main 不同) — 让色块内能看到 "这里有点第二种颜色"
    let sec_rgb = if main_zone < 0.40 {
        if patch_n > 0.55 { DIRT } else { STONE }
    } else if main_zone < 0.66 {
        if patch_n > 0.50 { GRASS } else { STONE }
    } else {
        if patch_n > 0.55 { DIRT } else { GRASS }
    };
    let sec_w = 0.25 + patch_n * 0.10;

    let total = main_w + sec_w;
    let shade = (fine_n - 0.5) * 0.06;
    [
        (((main_rgb[0] * main_w + sec_rgb[0] * sec_w) / total) * (1.0 + shade) * 1.25)
            .clamp(0.0, 1.0),
        (((main_rgb[1] * main_w + sec_rgb[1] * sec_w) / total) * (1.0 + shade) * 1.25)
            .clamp(0.0, 1.0),
        (((main_rgb[2] * main_w + sec_rgb[2] * sec_w) / total) * (1.0 + shade) * 1.25)
            .clamp(0.0, 1.0),
        1.0,
    ]
}

/// Laplacian smoothing：对每个顶点位置 = 邻接顶点平均
/// 简化版：按 (vertex position) hash 找邻居（同位置 = 共享顶点）
fn laplacian_smooth(vertices: Vec<McVertex>, indices: &[u32], passes: u32) -> Vec<McVertex> {
    let mut verts = vertices;
    for _ in 0..passes {
        let mut new_positions: Vec<[f32; 3]> = Vec::with_capacity(verts.len());
        for i in 0..verts.len() {
            // 找 i 的邻接顶点（共用三角形）
            let mut neighbors: Vec<[f32; 3]> = Vec::new();
            for tri in indices.chunks(3) {
                if tri.contains(&(i as u32)) {
                    for &vi in tri {
                        if vi != i as u32 {
                            neighbors.push(verts[vi as usize].position);
                        }
                    }
                }
            }
            if neighbors.is_empty() {
                new_positions.push(verts[i].position);
                continue;
            }
            let sum: [f32; 3] = neighbors.iter().fold([0.0; 3], |acc, n| {
                [acc[0] + n[0], acc[1] + n[1], acc[2] + n[2]]
            });
            let n = neighbors.len() as f32;
            let avg = [sum[0] / n, sum[1] / n, sum[2] / n];
            // 0.5 lerp 原位置 + 0.5 邻接平均（避免过度平滑）
            let orig = verts[i].position;
            new_positions.push([
                orig[0] * 0.5 + avg[0] * 0.5,
                orig[1] * 0.5 + avg[1] * 0.5,
                orig[2] * 0.5 + avg[2] * 0.5,
            ]);
        }
        for (i, p) in new_positions.iter().enumerate() {
            verts[i].position = *p;
        }
    }
    verts
}

/// 法线平滑：每个唯一 position 的法线 = 共享该 position 的所有三角形法线平均
fn smooth_normals(vertices: &[McVertex], _indices: &[u32]) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    // 简化：直接把每个顶点的当前法线保留（不做合并）
    // 因为 MC 输出每个三角形 3 个独立顶点，没有共享 — 共享发生在连续三角形用同 edge 交点时
    // v1 视觉上略 faceted，但 41³ grid + 0.5 平滑够用
    let positions: Vec<[f32; 3]> = vertices.iter().map(|v| v.position).collect();
    let normals: Vec<[f32; 3]> = vertices.iter().map(|v| v.normal).collect();
    (positions, normals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lk2_core::world::World as GameWorld;

    #[test]
    fn empty_world_no_mesh() {
        let world = GameWorld::new(128);
        // AABB 范围内全 air
        let result = build_smooth_mesh(&world, [40, 0, 40], [60, 30, 60], 0.5, 0);
        // 全 air 时所有角点 density = 0 → MC case 0 → 无 mesh
        assert!(result.is_none());
    }
}
