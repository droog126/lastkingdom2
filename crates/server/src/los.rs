

use bevy::prelude::*;
use lk2_core::world::BlockType;
use lk2_core::world::World as GameWorld;

#[derive(Clone, Debug)]
pub struct LosResult {

    pub blocked: bool,

    pub block_pos: Option<[i32; 3]>,

    pub total_dist: f32,

    pub travel_dist: f32,
}

pub fn line_of_sight(world: &GameWorld, from: Vec3, to: Vec3, step_size: f32) -> LosResult {
    let dir = to - from;
    let total_dist = dir.length();
    if total_dist < 0.001 {
        return LosResult { blocked: false, block_pos: None, total_dist: 0.0, travel_dist: 0.0 };
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

    LosResult { blocked: false, block_pos: None, total_dist, travel_dist: total_dist }
}

pub fn sector_voxels(
    eye: Vec3,
    forward: Vec3,
    reach: f32,
    sweep_angle_deg: f32,
    step_size: f32,
) -> Vec<[i32; 3]> {
    let half_angle = sweep_angle_deg.to_radians() / 2.0;
    let _right = forward.cross(Vec3::Y).normalize();
    let mut voxels = Vec::new();
    let mut checked = std::collections::HashSet::new();

    let mut pos = eye;
    let steps = (reach / step_size) as i32;
    for _ in 0..steps {
        pos += forward * step_size;
        let bx = pos.x.floor() as i32;
        let by = pos.y.floor() as i32;
        let bz = pos.z.floor() as i32;

        if checked.insert((bx, by, bz)) {
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
