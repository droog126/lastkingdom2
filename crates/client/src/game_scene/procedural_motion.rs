//! Shared procedural animation helpers for whole-entity motion.

use bevy::prelude::*;

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
