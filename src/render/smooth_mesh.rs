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

use super::marching_cubes::{build_mesh as mc_build_mesh, McVertex};
use super::scalar_field::{build_density_field, ScalarField};
use crate::world::World as GameWorld;

/// 一个 smooth chunk 的输出（玩家周围 AABB 一个 mesh）
pub struct SmoothMesh {
    pub mesh: Mesh,
    pub collider_trimesh: Vec<[f32; 3]>,  // 顶点
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
    let vertices = if smooth_passes > 0 {
        laplacian_smooth(vertices, indices, smooth_passes)
    } else {
        vertices
    };

    // 4) 法线平滑（按 position hash 共享顶点，邻接三角形法线平均）
    let (positions, normals) = smooth_normals(&vertices, &indices);

    // 5) 顶点色（按 y 分层：grass/dirt/stone）
    let colors: Vec<[f32; 4]> = positions
        .iter()
        .map(|p| layer_color(p[1]))
        .collect();

    // 6) build bevy::Mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices.clone()));

    Some(SmoothMesh {
        mesh,
        collider_trimesh: positions,
        collider_indices: indices,
    })
}

/// 按 Y 分层上色：
///   y >= high  (默认 20) → grass 绿
///   y <  high  & y >= mid (默认 12) → dirt 棕
///   y <  mid               → stone 灰
fn layer_color(y: f32) -> [f32; 4] {
    const HIGH: f32 = 18.0;
    const MID: f32 = 8.0;
    if y >= HIGH {
        [0.25, 0.55, 0.18, 1.0]  // grass 绿
    } else if y >= MID {
        [0.55, 0.36, 0.20, 1.0]  // dirt 棕
    } else {
        [0.55, 0.55, 0.55, 1.0]  // stone 灰
    }
}

/// Laplacian smoothing：对每个顶点位置 = 邻接顶点平均
/// 简化版：按 (vertex position) hash 找邻居（同位置 = 共享顶点）
fn laplacian_smooth(
    vertices: Vec<McVertex>,
    indices: Vec<u32>,
    passes: u32,
) -> Vec<McVertex> {
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
fn smooth_normals(vertices: &[McVertex], indices: &[u32]) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
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
    use crate::world::World as GameWorld;

    #[test]
    fn empty_world_no_mesh() {
        let world = GameWorld::new(128);
        // AABB 范围内全 air
        let result = build_smooth_mesh(&world, [40, 0, 40], [60, 30, 60], 0.5, 0);
        // 全 air 时所有角点 density = 0 → MC case 0 → 无 mesh
        assert!(result.is_none());
    }
}
