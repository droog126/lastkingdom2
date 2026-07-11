//! Shared procedural rig helpers for articulated entities.

use bevy::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct TwoBoneLimb {
    pub root: Vec3,
    pub target: Vec3,
    pub pole: Vec3,
    pub upper_len: f32,
    pub lower_len: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct TwoBoneSolution {
    pub root: Vec3,
    pub joint: Vec3,
    pub target: Vec3,
}

impl TwoBoneLimb {
    pub fn solve(self) -> TwoBoneSolution {
        TwoBoneSolution {
            root: self.root,
            joint: solve_two_bone(
                self.root,
                self.target,
                self.pole,
                self.upper_len,
                self.lower_len,
            ),
            target: self.target,
        }
    }
}

pub fn solve_two_bone(root: Vec3, target: Vec3, pole: Vec3, upper: f32, lower: f32) -> Vec3 {
    let target_delta = target - root;
    let distance = target_delta.length().clamp(0.001, upper + lower - 0.001);
    let direction = target_delta.normalize_or_zero();
    let pole_delta = pole - root;
    let pole_direction = (pole_delta - direction * pole_delta.dot(direction)).normalize_or_zero();
    let bend_direction = if pole_direction.length_squared() > 0.0 {
        pole_direction
    } else {
        Vec3::Y
    };
    let along = ((upper * upper + distance * distance - lower * lower) / (2.0 * distance))
        .clamp(-upper, upper);
    let height = (upper * upper - along * along).max(0.0).sqrt();
    root + direction * along + bend_direction * height
}

pub fn local_to_world(root: &Transform, local: Vec3) -> Vec3 {
    root.translation + root.rotation * local
}

pub fn apply_local_pose(
    transform: &mut Transform,
    root: &Transform,
    local_translation: Vec3,
    local_rotation: Quat,
    local_scale: Vec3,
) {
    transform.translation = local_to_world(root, local_translation);
    transform.rotation = root.rotation * local_rotation;
    transform.scale = local_scale;
}

pub fn apply_segment_between(transform: &mut Transform, start: Vec3, end: Vec3) {
    let delta = end - start;
    let length = delta.length().max(0.001);
    transform.translation = start + delta * 0.5;
    transform.rotation = Quat::from_rotation_arc(Vec3::Y, delta / length);
    transform.scale = Vec3::new(1.0, length, 1.0);
}

pub fn apply_local_segment(
    transform: &mut Transform,
    root: &Transform,
    local_start: Vec3,
    local_end: Vec3,
) {
    apply_segment_between(
        transform,
        local_to_world(root, local_start),
        local_to_world(root, local_end),
    );
}
