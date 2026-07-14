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

#[derive(Clone, Copy, Debug)]
pub struct TwoBoneLimbSpec {
    pub root: Vec3,
    pub upper_len: f32,
    pub lower_len: f32,
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

impl TwoBoneLimbSpec {
    pub fn with_target(self, target: Vec3, pole: Vec3) -> TwoBoneLimb {
        TwoBoneLimb {
            root: self.root,
            target,
            pole,
            upper_len: self.upper_len,
            lower_len: self.lower_len,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HumanoidRig {
    pub left_arm: TwoBoneLimbSpec,
    pub right_arm: TwoBoneLimbSpec,
    pub left_leg: TwoBoneLimbSpec,
    pub right_leg: TwoBoneLimbSpec,
}

#[derive(Clone, Copy, Debug)]
pub struct HumanoidTargets {
    pub left_hand: Vec3,
    pub right_hand: Vec3,
    pub left_foot: Vec3,
    pub right_foot: Vec3,
    pub left_elbow_pole: Vec3,
    pub right_elbow_pole: Vec3,
    pub left_knee_pole: Vec3,
    pub right_knee_pole: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct HumanoidSolvedRig {
    pub left_arm: TwoBoneSolution,
    pub right_arm: TwoBoneSolution,
    pub left_leg: TwoBoneSolution,
    pub right_leg: TwoBoneSolution,
}

#[derive(Clone, Copy, Debug)]
pub struct QuadrupedRig {
    pub front_left: TwoBoneLimbSpec,
    pub front_right: TwoBoneLimbSpec,
    pub hind_left: TwoBoneLimbSpec,
    pub hind_right: TwoBoneLimbSpec,
}

#[derive(Clone, Copy, Debug)]
pub struct QuadrupedTargets {
    pub front_left_foot: Vec3,
    pub front_right_foot: Vec3,
    pub hind_left_foot: Vec3,
    pub hind_right_foot: Vec3,
    pub front_left_pole: Vec3,
    pub front_right_pole: Vec3,
    pub hind_left_pole: Vec3,
    pub hind_right_pole: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct QuadrupedSolvedRig {
    pub front_left: TwoBoneSolution,
    pub front_right: TwoBoneSolution,
    pub hind_left: TwoBoneSolution,
    pub hind_right: TwoBoneSolution,
}

#[derive(Clone, Copy, Debug)]
pub struct DragonRig {
    pub front_left_leg: TwoBoneLimbSpec,
    pub front_right_leg: TwoBoneLimbSpec,
    pub hind_left_leg: TwoBoneLimbSpec,
    pub hind_right_leg: TwoBoneLimbSpec,
    pub left_wing: TwoBoneLimbSpec,
    pub right_wing: TwoBoneLimbSpec,
}

#[derive(Clone, Copy, Debug)]
pub struct DragonTargets {
    pub front_left_foot: Vec3,
    pub front_right_foot: Vec3,
    pub hind_left_foot: Vec3,
    pub hind_right_foot: Vec3,
    pub left_wing_tip: Vec3,
    pub right_wing_tip: Vec3,
    pub front_left_pole: Vec3,
    pub front_right_pole: Vec3,
    pub hind_left_pole: Vec3,
    pub hind_right_pole: Vec3,
    pub left_wing_pole: Vec3,
    pub right_wing_pole: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct DragonSolvedRig {
    pub front_left_leg: TwoBoneSolution,
    pub front_right_leg: TwoBoneSolution,
    pub hind_left_leg: TwoBoneSolution,
    pub hind_right_leg: TwoBoneSolution,
    pub left_wing: TwoBoneSolution,
    pub right_wing: TwoBoneSolution,
}

impl HumanoidRig {
    pub fn player_avatar() -> Self {
        Self {
            left_arm: TwoBoneLimbSpec {
                root: Vec3::new(-0.31, 1.02, -0.02),
                upper_len: 0.255,
                lower_len: 0.255,
            },
            right_arm: TwoBoneLimbSpec {
                root: Vec3::new(0.31, 1.02, -0.02),
                upper_len: 0.255,
                lower_len: 0.255,
            },
            left_leg: TwoBoneLimbSpec {
                root: Vec3::new(-0.15, 0.50, 0.03),
                upper_len: 0.225,
                lower_len: 0.225,
            },
            right_leg: TwoBoneLimbSpec {
                root: Vec3::new(0.15, 0.50, 0.03),
                upper_len: 0.225,
                lower_len: 0.225,
            },
        }
    }

    pub fn solve(self, targets: HumanoidTargets) -> HumanoidSolvedRig {
        HumanoidSolvedRig {
            left_arm: self
                .left_arm
                .with_target(targets.left_hand, targets.left_elbow_pole)
                .solve(),
            right_arm: self
                .right_arm
                .with_target(targets.right_hand, targets.right_elbow_pole)
                .solve(),
            left_leg: self
                .left_leg
                .with_target(targets.left_foot, targets.left_knee_pole)
                .solve(),
            right_leg: self
                .right_leg
                .with_target(targets.right_foot, targets.right_knee_pole)
                .solve(),
        }
    }
}

impl QuadrupedRig {
    pub fn rabbit() -> Self {
        Self {
            front_left: TwoBoneLimbSpec {
                root: Vec3::new(-0.12, 0.32, -0.18),
                upper_len: 0.18,
                lower_len: 0.18,
            },
            front_right: TwoBoneLimbSpec {
                root: Vec3::new(0.12, 0.32, -0.18),
                upper_len: 0.18,
                lower_len: 0.18,
            },
            hind_left: TwoBoneLimbSpec {
                root: Vec3::new(-0.16, 0.34, 0.20),
                upper_len: 0.24,
                lower_len: 0.22,
            },
            hind_right: TwoBoneLimbSpec {
                root: Vec3::new(0.16, 0.34, 0.20),
                upper_len: 0.24,
                lower_len: 0.22,
            },
        }
    }

    pub fn wolf() -> Self {
        Self {
            front_left: TwoBoneLimbSpec {
                root: Vec3::new(-0.20, 0.48, -0.34),
                upper_len: 0.34,
                lower_len: 0.32,
            },
            front_right: TwoBoneLimbSpec {
                root: Vec3::new(0.20, 0.48, -0.34),
                upper_len: 0.34,
                lower_len: 0.32,
            },
            hind_left: TwoBoneLimbSpec {
                root: Vec3::new(-0.22, 0.50, 0.38),
                upper_len: 0.38,
                lower_len: 0.36,
            },
            hind_right: TwoBoneLimbSpec {
                root: Vec3::new(0.22, 0.50, 0.38),
                upper_len: 0.38,
                lower_len: 0.36,
            },
        }
    }

    pub fn solve(self, targets: QuadrupedTargets) -> QuadrupedSolvedRig {
        QuadrupedSolvedRig {
            front_left: self
                .front_left
                .with_target(targets.front_left_foot, targets.front_left_pole)
                .solve(),
            front_right: self
                .front_right
                .with_target(targets.front_right_foot, targets.front_right_pole)
                .solve(),
            hind_left: self
                .hind_left
                .with_target(targets.hind_left_foot, targets.hind_left_pole)
                .solve(),
            hind_right: self
                .hind_right
                .with_target(targets.hind_right_foot, targets.hind_right_pole)
                .solve(),
        }
    }
}

impl DragonRig {
    pub fn hoplite_boss() -> Self {
        Self {
            front_left_leg: TwoBoneLimbSpec {
                root: Vec3::new(-0.42, 0.84, -0.60),
                upper_len: 0.50,
                lower_len: 0.50,
            },
            front_right_leg: TwoBoneLimbSpec {
                root: Vec3::new(0.42, 0.84, -0.60),
                upper_len: 0.50,
                lower_len: 0.50,
            },
            hind_left_leg: TwoBoneLimbSpec {
                root: Vec3::new(-0.52, 0.78, 0.48),
                upper_len: 0.58,
                lower_len: 0.52,
            },
            hind_right_leg: TwoBoneLimbSpec {
                root: Vec3::new(0.52, 0.78, 0.48),
                upper_len: 0.58,
                lower_len: 0.52,
            },
            left_wing: TwoBoneLimbSpec {
                root: Vec3::new(-0.52, 1.20, -0.12),
                upper_len: 1.04,
                lower_len: 1.02,
            },
            right_wing: TwoBoneLimbSpec {
                root: Vec3::new(0.52, 1.20, -0.12),
                upper_len: 1.04,
                lower_len: 1.02,
            },
        }
    }

    pub fn solve(self, targets: DragonTargets) -> DragonSolvedRig {
        DragonSolvedRig {
            front_left_leg: self
                .front_left_leg
                .with_target(targets.front_left_foot, targets.front_left_pole)
                .solve(),
            front_right_leg: self
                .front_right_leg
                .with_target(targets.front_right_foot, targets.front_right_pole)
                .solve(),
            hind_left_leg: self
                .hind_left_leg
                .with_target(targets.hind_left_foot, targets.hind_left_pole)
                .solve(),
            hind_right_leg: self
                .hind_right_leg
                .with_target(targets.hind_right_foot, targets.hind_right_pole)
                .solve(),
            left_wing: self
                .left_wing
                .with_target(targets.left_wing_tip, targets.left_wing_pole)
                .solve(),
            right_wing: self
                .right_wing
                .with_target(targets.right_wing_tip, targets.right_wing_pole)
                .solve(),
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
