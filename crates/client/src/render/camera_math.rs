use bevy::prelude::*;

pub const TP_DISTANCE: f32 = 6.0;
pub const TP_PIVOT_HEIGHT: f32 = 1.45;
pub const TP_MIN_CLEARANCE: f32 = 0.75;
pub const TP_PITCH_MIN: f32 = -0.65;
pub const TP_PITCH_MAX: f32 = 0.65;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ThirdPersonCameraDebug {
    pub target: Vec3,
    pub translation: Vec3,
    pub forward: Vec3,
    pub pitch: f32,
}

pub fn compute_third_person_camera(
    player_pos: Vec3,
    yaw: f32,
    pitch: f32,
    ground_at_camera: f32,
) -> ThirdPersonCameraDebug {
    let (sy, cy) = yaw.sin_cos();
    let pitch = pitch.clamp(TP_PITCH_MIN, TP_PITCH_MAX);
    let (sp, cp) = pitch.sin_cos();
    let target = player_pos + Vec3::Y * TP_PIVOT_HEIGHT;
    let orbit = Vec3::new(-sy * cp, -sp, cy * cp) * TP_DISTANCE;
    let mut translation = target + orbit;
    translation.y = translation.y.max(ground_at_camera + TP_MIN_CLEARANCE);
    let forward = (target - translation).normalize_or_zero();
    ThirdPersonCameraDebug { target, translation, forward, pitch }
}
