//! 标量场：把 BlockType 网格软化成 f32 density，给 marching cubes 用
//!
//! ## 核心思想
//!
//! 不直接用 `World.get(x, y, z)` 的 Boolean，而是把"角点"采样成 f32 ∈ [0, 1]：
//!
//! ```text
//!   cell 角点 (x, y, z) 的 density = 周围 8 个 cell 中 solid 占比
//!   = if 8/8 solid → 1.0
//!   = if 4/8 solid → 0.5  ← 玩家脚下"软"边界
//!   = if 0/8 solid → 0.0
//! ```
//!
//! 沿 cell edge 5 个采样点会自然出现 "0 - 0.1 - 0.2 - 0.2 - 1" 这样的渐变
//! —— 字面意义实现了用户说的"曲线函数连接"。
//!
//! ## 多维 heightmap
//!
//! 玩家脚下的"软高度"由 `pipeline.surface_f32(x, z)` 拿到（多模块叠加）。
//! 跟角点 density 一起作为标量场输入 → cave / dome / heightmap 三个维度统一表达。
//!
//! ## 性能
//!
//! 41³ cell = 42³ 角点 = 74088 元素 = 296KB（Vec 放堆上），stack 安全。
//! 一次 re-mesh 50-100ms（AABB 41³，1.5s 节流够用）。

use lk2_core::world::BlockType;
use lk2_core::world::World as GameWorld;

/// 标量场：3D f32 数组，shape = (sx, sy, sz) 角点数，origin = 角点 (0,0,0) 对应的世界坐标
#[derive(Debug, Clone)]
pub struct ScalarField {
    pub data: Vec<f32>,
    pub shape: [usize; 3],
    /// 角点 (0,0,0) 对应的世界坐标（角点不是 cell 中心）
    pub origin: [i32; 3],
}

impl ScalarField {
    #[inline]
    pub fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        (y * self.shape[2] + z) * self.shape[0] + x
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> f32 {
        self.data[self.idx(x, y, z)]
    }
}

/// 在 AABB 范围 (min..max) 内构造标量场
///
/// - cell 数 = (max[0]-min[0]) × (max[1]-min[1]) × (max[2]-min[2])
/// - 角点数 = cell 数 + 1 每维
/// - 每个角点的 density = 周围 8 个 cell 中 solid（排除 Air + Water）的占比 ∈ [0, 1]
pub fn build_density_field(
    world: &GameWorld,
    min: [i32; 3],
    max: [i32; 3],
) -> ScalarField {
    let cell_size = [
        (max[0] - min[0]).max(1) as usize,
        (max[1] - min[1]).max(1) as usize,
        (max[2] - min[2]).max(1) as usize,
    ];
    let corner_shape = [cell_size[0] + 1, cell_size[1] + 1, cell_size[2] + 1];
    let n = corner_shape[0] * corner_shape[1] * corner_shape[2];
    let mut data = vec![0.0_f32; n];

    // 角点 (cx, cy, cz) 对应世界坐标 (min[0] + cx, min[1] + cy, min[2] + cz)
    // 它周围有 8 个 cell: (cx-1..cx) × (cy-1..cy) × (cz-1..cz)
    for cz in 0..corner_shape[2] {
        for cy in 0..corner_shape[1] {
            for cx in 0..corner_shape[0] {
                let mut solid = 0u32;
                let mut total = 0u32;
                for dz in -1i32..=0 {
                    for dy in -1i32..=0 {
                        for dx in -1i32..=0 {
                            let wx = min[0] + cx as i32 + dx;
                            let wy = min[1] + cy as i32 + dy;
                            let wz = min[2] + cz as i32 + dz;
                            if !world.in_bounds(wx, wy, wz) {
                                continue; // 越界跳过
                            }
                            total += 1;
                            // solid = 非空非水（水的视觉边界由海洋 mesh 处理，不进地表）
                            let b = world.get(wx, wy, wz);
                            if b.is_solid() && !matches!(b, BlockType::Water) {
                                solid += 1;
                            }
                        }
                    }
                }
                let density = if total == 0 {
                    0.0
                } else {
                    solid as f32 / total as f32
                };
                let i = (cy * corner_shape[2] + cz) * corner_shape[0] + cx;
                data[i] = density;
            }
        }
    }

    ScalarField {
        data,
        shape: corner_shape,
        origin: min,
    }
}

/// 玩家脚下"软"地表高度（f32）— 用于 try_player_move 的"跨度 0.5"判定
///
/// 实现：4 邻居 cell 中心的地表高度（pipeline.surface_f32）做双线性插值
/// 玩家在 (x, z) 这个 integer 位置 → 等于 4 邻居平均（tx=tz=0.5）
/// 但每个邻居 surface_f32 是 f32，结果也是 f32 → 跨格差可以是 0.3/0.5/0.7
pub fn effective_ground_height(world: &GameWorld, x: i32, z: i32) -> f32 {
    // 4 邻居 + 中心点 (5 个采样点) 的地表高度，用 noise 自身的 f32 渐变
    // 不再需要插值——直接取 (x, z) 处的 surface_f32 即可
    // 但 surface_f32 在 SpawnHill 圆顶内 vs 圆顶外差异大，要确保一致
    match world.pipeline.surface_f32(x, z) {
        Some(h) => h,
        // fallback: 用 (x, z) 列上第一个 solid 的 y
        None => find_first_solid_y(world, x, z) as f32,
    }
}

/// (x, z) 列上第一个 solid 块的 y 值（i32）— effective_ground_height fallback
fn find_first_solid_y(world: &GameWorld, x: i32, z: i32) -> i32 {
    for y in 0..world.size {
        if world.get(x, y, z).is_solid() {
            return y;
        }
    }
    world.size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_field_41x41x41() {
        let world = GameWorld::new(128);
        let field = build_density_field(&world, [0, 0, 0], [40, 40, 40]);
        assert_eq!(field.shape, [41, 41, 41]);
        // 内部全 air → 全部角点 density 应该接近 0
        let max = field.data.iter().cloned().fold(0.0_f32, f32::max);
        assert!(max < 0.5, "全 air 时角点最大 density 应 < 0.5, got {}", max);
    }
}
