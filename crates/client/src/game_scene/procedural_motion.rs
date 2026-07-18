//! Shared procedural animation helpers for whole-entity motion.

use bevy::prelude::*;

/// Runtime tuning values for the presentation-only humanoid IK pose.
///
/// These values intentionally live outside the authoritative player state. The
/// settings panel can change them while the scene is running, which makes it
/// possible to tune the silhouette without rebuilding the player asset.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct ProceduralAnimationConfig {
    pub stride_length: f32,
    pub foot_lift: f32,
    pub torso_lean: f32,
    pub body_bob: f32,
    pub arm_swing: f32,
    pub ik_leg_upper_length: f32,
    pub ik_leg_lower_length: f32,
    pub crouch_depth: f32,
    pub crouch_lean: f32,
    pub crouch_arm_drop: f32,
    pub crouch_knee_forward: f32,
    pub crouch_transition_speed: f32,
}

impl Default for ProceduralAnimationConfig {
    fn default() -> Self {
        Self {
            stride_length: 0.30,
            foot_lift: 0.18,
            torso_lean: 0.13,
            body_bob: 0.040,
            arm_swing: 0.08,
            ik_leg_upper_length: 0.25,
            ik_leg_lower_length: 0.25,
            crouch_depth: 0.32,
            crouch_lean: 0.18,
            crouch_arm_drop: 0.10,
            crouch_knee_forward: 0.20,
            crouch_transition_speed: 10.0,
        }
    }
}

/// One presentation-only wind field shared by all vegetation.
#[derive(Resource, Clone, Copy, Debug)]
pub struct WindField {
    pub direction: Vec2,
    pub strength: f32,
    pub speed: f32,
}

impl Default for WindField {
    fn default() -> Self {
        Self {
            direction: Vec2::new(0.86, 0.34).normalize(),
            strength: 1.0,
            speed: 1.0,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct GrassWind {
    pub base_yaw: f32,
    pub phase: f32,
    pub strength: f32,
}

pub fn grass_wind_rotation(elapsed: f32, wind: WindField, sway: GrassWind) -> Quat {
    let t = elapsed * wind.speed + sway.phase;
    let gust = (t * 0.73).sin() * 0.35 + (t * 1.61 + 1.7).sin() * 0.12;
    let amplitude = sway.strength * wind.strength * (0.72 + 0.28 * gust);
    let wind_bias = wind.direction.x * 0.75 + wind.direction.y * 0.45;
    Quat::from_rotation_y(sway.base_yaw)
        * Quat::from_rotation_z(gust * amplitude)
        * Quat::from_rotation_x((t * 0.91).cos() * amplitude * 0.55 + wind_bias * amplitude * 0.35)
}

#[derive(Component, Clone, Copy, Debug)]
pub struct ProceduralTreeSway {
    pub base_translation: Vec3,
    pub base_yaw: f32,
    pub base_scale: f32,
    pub phase: f32,
    pub strength: f32,
}

pub fn phase(elapsed: f32, rate: f32, offset: f32) -> f32 {
    elapsed * rate + offset
}

pub fn smooth_follow_alpha(dt: f32, responsiveness: f32) -> f32 {
    1.0 - (-dt * responsiveness.max(0.0)).exp()
}

pub fn idle_drift(elapsed: f32, phase: f32, x_radius: f32, z_radius: f32) -> Vec3 {
    Vec3::new(
        (elapsed * 0.6 + phase).sin() * x_radius,
        0.0,
        (elapsed * 0.5 + phase).cos() * z_radius,
    )
}

pub fn orbit_bob(
    elapsed: f32,
    offset: f32,
    x_radius: f32,
    z_radius: f32,
    y_amplitude: f32,
) -> Vec3 {
    let t = phase(elapsed, 0.8, offset);
    Vec3::new(
        t.sin() * x_radius,
        t.sin().max(0.0) * y_amplitude,
        t.cos() * z_radius,
    )
}

pub fn hop_height(phase: f32, amplitude: f32) -> f32 {
    phase.sin().max(0.0) * amplitude
}

/// A bounded, periodic foot offset for virtual quadruped rigs. The player has
/// a stateful planted-foot controller; virtual rigs use this continuous curve
/// so a phase wrap cannot teleport a visible part.
pub fn alternating_step_offset(phase: f32, stride: f32, opposite: bool) -> f32 {
    let phase = if opposite {
        phase + std::f32::consts::PI
    } else {
        phase
    }
    .rem_euclid(std::f32::consts::TAU);
    phase.sin() * stride * 0.5
}

pub fn alternating_step_lift(phase: f32, height: f32, opposite: bool) -> f32 {
    let phase = if opposite {
        phase + std::f32::consts::PI
    } else {
        phase
    }
    .rem_euclid(std::f32::consts::TAU);
    phase.sin().max(0.0) * height
}

pub fn heading_yaw(from: Vec3, target: Vec3, fallback: f32) -> f32 {
    let heading = Vec3::new(target.x - from.x, 0.0, target.z - from.z);
    if heading.length_squared() > 0.001 {
        heading.x.atan2(heading.z)
    } else {
        fallback
    }
}

pub fn wind_sway(elapsed: f32, sway: &ProceduralTreeSway) -> Quat {
    let slow = phase(elapsed, 0.72, sway.phase);
    let quick = phase(elapsed, 1.83, sway.phase * 0.37);
    let roll = slow.sin() * sway.strength + quick.sin() * sway.strength * 0.28;
    let pitch = (slow * 0.7).cos() * sway.strength * 0.55;
    Quat::from_rotation_y(sway.base_yaw)
        * Quat::from_rotation_z(roll)
        * Quat::from_rotation_x(pitch)
}

pub fn wind_scale(elapsed: f32, sway: &ProceduralTreeSway) -> Vec3 {
    let breathe = (phase(elapsed, 1.1, sway.phase) * 0.5).sin() * sway.strength * 0.45;
    Vec3::splat(sway.base_scale)
        * Vec3::new(1.0 - breathe * 0.15, 1.0 + breathe, 1.0 - breathe * 0.15)
}
