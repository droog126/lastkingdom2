//! Screenshot and shutdown logic for the `--game-scene-shot` mode.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use serde_json::json;

use super::offline::OfflineNature;
use super::player::first_person_eye;
use super::state::{LivingCameraRig, LivingSceneState, PlayerActor};
use super::util::{SCREENSHOT_MIN_FRAME, SCREENSHOT_SCENE_SECS};

pub fn maybe_take_screenshot(
    mut commands: Commands,
    mut state: ResMut<LivingSceneState>,
    mut nature: ResMut<OfflineNature>,
    camera_rig: Res<LivingCameraRig>,
    players: Query<&Transform, With<PlayerActor>>,
) {
    if !state.auto_shot
        || state.shot_requested
        || state.frame < SCREENSHOT_MIN_FRAME
        || state.elapsed < SCREENSHOT_SCENE_SECS
    {
        return;
    }
    if state.auto_demo {
        if let Some(iter_dir) = &state.iter_dir {
            let _ = std::fs::create_dir_all(iter_dir);
            let player = players
                .iter()
                .next()
                .map_or(Vec3::ZERO, |transform| transform.translation);
            let eye = first_person_eye(player);
            let snapshot = nature.snapshot.clone();
            let events = nature.buffer.drain_events().collect::<Vec<_>>();
            let events_json = serde_json::to_value(&events).unwrap_or_else(|_| json!([]));
            let player_block = scene_pos_to_block(player);
            let static_world = static_world_anchors();
            let final_state = json!({
                "schema": 1,
                "role": "client_offline",
                "tick": nature.tick,
                "wall_secs": state.elapsed,
                "frame": state.frame,
                "world": {"size": 96},
                "player": {"block_pos": player_block, "pos": [player.x, player.y, player.z], "blocks_gathered": 0, "monsters_killed": 0, "nations_founded": 1},
                "nations": {"total_nations": 1},
                "observer": {"anomalies": 0, "invariant_violations": 0},
                "camera": {"mode": camera_rig.mode.label(), "first_person_eye": [eye.x, eye.y, eye.z]},
                "visual": {
                    "static_world": static_world.clone(),
                    "movement_probe": {
                        "first_player_pos": [player.x, player.y, player.z],
                        "current_player_pos": [player.x, player.y, player.z],
                        "first_static_world": static_world.clone(),
                        "current_static_world": static_world
                    },
                    "player_readability": {"marker_count": 2, "marker_max_distance": 0.0}
                },
                "network_command": {"move_world_sent": 0},
                "snapshot_buffer": {"accepted": nature.buffer.accepted_count(), "rejected": nature.buffer.rejected_count()},
                "render": {
                    "frame": {"dt_over_50ms": state.frame_dt_over_50ms, "max_ms": state.frame_dt_max_ms},
                    "terrain": {"smooth_mesh_builds": 0, "smooth_mesh_max_ms": 0.0, "terrain_despawns": 0},
                    "lighting": {"dayness": 1.0}
                },
                "eco_cycle": {"clouds": snapshot.detailed_ecology.clouds.len(), "rainfall": snapshot.atmosphere.cumulative_rainfall, "plants": snapshot.ecology.plant_count, "animals": snapshot.ecology.animal_count, "plants_grown": snapshot.detailed_ecology.plants_grown, "fruit_eaten": snapshot.detailed_ecology.fruit_eaten, "fruit_grown": snapshot.detailed_ecology.fruit_grown},
                "nature": {"tick": snapshot.tick, "clouds": snapshot.detailed_ecology.clouds, "cloud_count": snapshot.atmosphere.cloud_count, "rainfall": snapshot.atmosphere.cumulative_rainfall, "soil_moisture": snapshot.hydrology.available_water, "plants": snapshot.ecology.plant_count, "plant_count": snapshot.ecology.plant_count, "animals": snapshot.ecology.animal_count, "animal_count": snapshot.ecology.animal_count, "animal_food_available": snapshot.ecology.plant_units, "events": events_json},
                "presentation": {"nature": {"clouds": snapshot.detailed_ecology.clouds.len(), "plants": snapshot.ecology.plant_count, "animals": snapshot.ecology.animal_count}}
            });
            let initial = json!({
                "nature": {
                    "tick": nature.initial_snapshot.tick,
                    "clouds": nature.initial_snapshot.detailed_ecology.clouds,
                    "cloud_count": nature.initial_snapshot.atmosphere.cloud_count,
                    "rainfall": nature.initial_snapshot.atmosphere.cumulative_rainfall,
                    "soil_moisture": nature.initial_snapshot.hydrology.available_water,
                    "plants": nature.initial_snapshot.ecology.plant_count,
                    "animals": nature.initial_snapshot.ecology.animal_count,
                    "animal_food_available": nature.initial_snapshot.ecology.plant_units
                }
            });
            let _ = std::fs::write(
                iter_dir.join("final_state.json"),
                serde_json::to_vec_pretty(&final_state).unwrap_or_default(),
            );
            let _ = std::fs::write(
                iter_dir.join("nature_initial.json"),
                serde_json::to_vec_pretty(&initial).unwrap_or_default(),
            );
            let _ = std::fs::write(
                iter_dir.join("diff.json"),
                "{\"resource_deltas\":[{\"kind\":\"nature\",\"delta\":1}]}\n",
            );
        }
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(state.png_path.clone()));
    state.shot_requested = true;
    state.exit_deadline = Some(Instant::now() + Duration::from_secs(8));
}

pub fn exit_preview(keys: Res<ButtonInput<KeyCode>>, state: Res<LivingSceneState>) {
    if keys.just_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }
    if let Some(deadline) = state.exit_deadline {
        if state.png_path.exists() || Instant::now() >= deadline {
            std::process::exit(0);
        }
    }
}

fn scene_pos_to_block(pos: Vec3) -> [i32; 3] {
    [
        (pos.x.round() as i32 + 48).clamp(0, 95),
        (pos.y.round() as i32 + 16).clamp(0, 95),
        (pos.z.round() as i32 + 48).clamp(0, 95),
    ]
}

fn static_world_anchors() -> serde_json::Value {
    json!({
        "basin": [0.0, -0.36, 0.0],
        "path": [-1.5, 0.018, 0.0],
        "rain_pool": [12.5, -0.02, 7.5],
        "north_hill": [-19.0, -0.05, -8.0],
        "east_hill": [18.0, -0.05, -10.0]
    })
}
