//! Player-triggered milestone screenshots for rendered gameplay scenes.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

pub const MILESTONE_CAPTURE_KEY: KeyCode = KeyCode::F12;
pub const MILESTONE_OUTPUT_DIR: &str = "milestones";

pub struct MilestoneCapturePlugin;

impl Plugin for MilestoneCapturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MilestoneCaptureState>()
            .add_systems(Update, capture_milestone_on_key);
    }
}

#[derive(Resource, Default)]
struct MilestoneCaptureState {
    sequence: u64,
}

fn capture_milestone_on_key(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MilestoneCaptureState>,
) {
    if !keys.just_pressed(MILESTONE_CAPTURE_KEY) {
        return;
    }

    let output_dir = PathBuf::from(MILESTONE_OUTPUT_DIR);
    if let Err(error) = std::fs::create_dir_all(&output_dir) {
        error!(
            "[milestone] 无法创建截图目录 {}: {error}",
            output_dir.display()
        );
        return;
    }

    state.sequence = state.sequence.saturating_add(1);
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let path = milestone_capture_path(timestamp_ms, state.sequence);
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()));
    info!("[milestone] F12 截图已请求: {}", path.display());
}

fn milestone_capture_path(timestamp_ms: u128, sequence: u64) -> PathBuf {
    PathBuf::from(MILESTONE_OUTPUT_DIR).join(format!("milestone_{timestamp_ms}_{sequence:03}.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestone_capture_uses_f12_and_durable_output_directory() {
        assert_eq!(MILESTONE_CAPTURE_KEY, KeyCode::F12);
        assert_eq!(
            milestone_capture_path(1_234, 7),
            PathBuf::from("milestones/milestone_1234_007.png")
        );
    }
}
