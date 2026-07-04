use bevy::prelude::*;

#[derive(Component, Clone, Debug, Reflect)]
#[require(Transform)]
pub struct PvPController {
    pub speed: f32,

    pub jump_impulse: f32,

    pub air_control: f32,

    pub gravity_scale: f32,

    pub step_height: f32,

    pub step_speed: f32,

    pub is_grounded: bool,

    pub last_grounded_time: f32,

    pub ground_normal: Option<Vec3>,

    pub knockback_resistance: f32,

    pub knockback_velocity: Vec3,

    pub knockback_stun: f32,

    pub knockback_input_penalty: f32,

    pub move_input: Vec2,

    pub jump_requested: bool,

    pub is_sprinting: bool,
}

impl Default for PvPController {
    fn default() -> Self {
        Self {
            speed: 5.0,
            jump_impulse: 8.0,
            air_control: 0.3,
            gravity_scale: 1.0,

            step_height: 0.6,
            step_speed: 3.0,

            is_grounded: false,
            last_grounded_time: 0.0,
            ground_normal: None,

            knockback_resistance: 0.0,
            knockback_velocity: Vec3::ZERO,
            knockback_stun: 0.0,
            knockback_input_penalty: 0.3,

            move_input: Vec2::ZERO,
            jump_requested: false,
            is_sprinting: false,
        }
    }
}

impl PvPController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    pub fn with_jump(mut self, impulse: f32) -> Self {
        self.jump_impulse = impulse;
        self
    }

    pub fn with_knockback_resistance(mut self, resistance: f32) -> Self {
        self.knockback_resistance = resistance.clamp(0.0, 1.0);
        self
    }

    pub fn apply_knockback(&mut self, velocity: Vec3) {
        let effective = velocity * (1.0 - self.knockback_resistance);
        self.knockback_velocity += effective;
        self.knockback_stun = 0.5;
    }

    pub fn is_stunned(&self) -> bool {
        self.knockback_stun > 0.0
    }

    pub fn input_multiplier(&self) -> f32 {
        if self.is_stunned() {
            self.knockback_input_penalty
        } else {
            1.0
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct PlayerCollider {
    pub radius: f32,

    pub half_height: f32,

    pub eye_height: f32,
}

impl Default for PlayerCollider {
    fn default() -> Self {
        Self { radius: 0.3, half_height: 0.9, eye_height: 1.62 }
    }
}

#[derive(Clone, Debug)]
pub struct GroundHit {
    pub point: Vec3,

    pub normal: Vec3,

    pub distance: f32,

    pub entity: Option<Entity>,
}

pub mod components {
    pub use super::{GroundHit, PlayerCollider, PvPController};
}
