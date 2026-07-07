use bevy::prelude::*;

pub const TP_DISTANCE: f32 = 8.2;
pub const TP_PIVOT_HEIGHT: f32 = 1.75;
pub const TP_MIN_CLEARANCE: f32 = 0.75;
pub const TP_PITCH_MIN: f32 = -0.48;
pub const TP_PITCH_MAX: f32 = 0.42;

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
    let display_pitch = (pitch - 0.34).clamp(TP_PITCH_MIN, TP_PITCH_MAX);
    let (sp, cp) = display_pitch.sin_cos();
    let target = player_pos + Vec3::Y * TP_PIVOT_HEIGHT;
    let orbit = Vec3::new(-sy * cp, -sp, cy * cp) * TP_DISTANCE;
    let mut translation = target + orbit;
    translation.y = translation.y.max(ground_at_camera + TP_MIN_CLEARANCE);
    let forward = (target - translation).normalize_or_zero();
    ThirdPersonCameraDebug { target, translation, forward, pitch: display_pitch }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn third_person_camera_uses_showroom_composition() {
        let player = Vec3::new(48.5, 16.0, 48.5);
        let camera = compute_third_person_camera(player, std::f32::consts::FRAC_PI_2, 0.0, 14.0);

        assert!(camera.translation.distance(player) > 8.0);
        assert!(camera.translation.y > player.y + 3.0);
        assert!((camera.target - (player + Vec3::Y * TP_PIVOT_HEIGHT)).length() < 0.001);
        assert!(
            (camera.forward - (camera.target - camera.translation).normalize()).length() < 0.001
        );
    }
}
