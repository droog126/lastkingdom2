//! 体素世界的视线检测（DDA 算法）
//!
//! Digital Differential Analyzer — 从体素世界中的 A 点向 B 点发射射线，
//! 步进到每个体素格，判断是否有 solid 方块阻挡视线。
//!
//! 用于 PvP 命中判定中的"隔着墙不能打"逻辑。

use lk2_core::world::World as GameWorld;
use lk2_core::world::BlockType;
use bevy::prelude::*;

/// 视线检测结果
#[derive(Clone, Debug)]
pub struct LosResult {
    /// 是否被阻挡
    pub blocked: bool,
    /// 如果 blocked=true，阻挡点的 voxel 坐标
    pub block_pos: Option<[i32; 3]>,
    /// 射线总长度（格）
    pub total_dist: f32,
    /// 实际到达的距离（格）
    pub travel_dist: f32,
}

/// 从 from 到 to 做体素视线检测
///
/// - `step_size` 步进精度（格），越小越精确但越慢，默认 0.05（5cm）
/// - 返回 `LosResult`
pub fn line_of_sight(
    world: &GameWorld,
    from: Vec3,
    to: Vec3,
    step_size: f32,
) -> LosResult {
    let dir = to - from;
    let total_dist = dir.length();
    if total_dist < 0.001 {
        return LosResult {
            blocked: false,
            block_pos: None,
            total_dist: 0.0,
            travel_dist: 0.0,
        };
    }
    let step = dir.normalize() * step_size;
    let mut pos = from;
    let mut travel = 0.0f32;

    while travel < total_dist {
        pos += step;
        travel += step_size;

        let bx = pos.x.floor() as i32;
        let by = pos.y.floor() as i32;
        let bz = pos.z.floor() as i32;

        if !world.in_bounds(bx, by, bz) {
            continue;
        }
        let block = world.get(bx, by, bz);
        if block.is_solid() {
            return LosResult {
                blocked: true,
                block_pos: Some([bx, by, bz]),
                total_dist,
                travel_dist: travel,
            };
        }
    }

    LosResult {
        blocked: false,
        block_pos: None,
        total_dist,
        travel_dist: total_dist,
    }
}

/// 从眼睛位置（player_eye + height）朝向 forward 做扇形命中检测
/// 返回扇形内的所有格坐标（粗筛）
pub fn sector_voxels(
    eye: Vec3,
    forward: Vec3,
    reach: f32,
    sweep_angle_deg: f32,
    step_size: f32,
) -> Vec<[i32; 3]> {
    let half_angle = sweep_angle_deg.to_radians() / 2.0;
    let right = forward.cross(Vec3::Y).normalize();
    let mut voxels = Vec::new();
    let mut checked = std::collections::HashSet::new();

    // 沿射线步进
    let mut pos = eye;
    let steps = (reach / step_size) as i32;
    for _ in 0..steps {
        pos += forward * step_size;
        let bx = pos.x.floor() as i32;
        let by = pos.y.floor() as i32;
        let bz = pos.z.floor() as i32;

        if checked.insert((bx, by, bz)) {
            // 粗筛：在扇形角度内
            let to_voxel = pos - eye;
            let dist = to_voxel.length();
            if dist > 0.1 {
                let dir = to_voxel / dist;
                let angle = forward.angle_between(dir);
                if angle <= half_angle {
                    voxels.push([bx, by, bz]);
                }
            }
        }
    }

    voxels
}

#[cfg(test)]
mod tests {
    use super::*;
    use lk2_core::world::World as GameWorld;

    fn dummy_world() -> GameWorld {
        // 16x4x16 的空地，底下一层草
        let mut w = GameWorld::new(16);
        for x in 0..16 {
            for z in 0..16 {
                w.set(x, 0, z, BlockType::Grass);
            }
        }
        w
    }

    #[test]
    fn test_los_clear() {
        let w = dummy_world();
        let r = line_of_sight(&w, Vec3::new(5.5, 2.0, 5.5), Vec3::new(10.5, 2.0, 5.5), 0.1);
        assert!(!r.blocked);
    }

    #[test]
    fn test_los_blocked() {
        let mut w = dummy_world();
        w.set(8, 1, 5, BlockType::Stone); // 中间放一块石头
        let r = line_of_sight(&w, Vec3::new(5.5, 2.0, 5.5), Vec3::new(10.5, 2.0, 5.5), 0.1);
        assert!(r.blocked);
    }
}
