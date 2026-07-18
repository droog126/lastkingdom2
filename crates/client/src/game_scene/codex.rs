//! Codex action driver for the complete Living Forest scene.

use std::{fs, path::PathBuf, time::Duration};

use bevy::prelude::*;
use lk2_core::resource::ResourceKind;
use serde_json::json;

use crate::codex_client::{CodexObservation, CodexRequest, CodexWorker};

use super::offline::OfflineNature;
use super::state::{CodexSceneInput, PlayerActor};

const DECISION_INTERVAL_SECS: f32 = 3.0;

#[derive(Resource)]
pub struct CodexSceneState {
    pub elapsed: f32,
    pub last_request_at: f32,
    pub request_sequence: u64,
    pub active_request: Option<u64>,
    pub requests: u64,
    pub successful_decisions: u64,
    pub failed_decisions: u64,
    pub last_intent: String,
    pub last_reason: String,
    pub last_error: String,
    pub max_runtime: Option<Duration>,
    pub output_path: PathBuf,
    pub state_written_at: f32,
    pub first_position: Option<Vec3>,
    pub current_position: Option<Vec3>,
    pub finished: bool,
}

impl CodexSceneState {
    pub fn from_args(args: &[String]) -> Self {
        let max_runtime = args
            .iter()
            .find_map(|arg| arg.strip_prefix("--seconds=")?.parse::<u64>().ok())
            .filter(|seconds| *seconds > 0)
            .map(Duration::from_secs);
        let output_path = args
            .iter()
            .find_map(|arg| arg.strip_prefix("--codex-output=").map(PathBuf::from))
            .or_else(|| {
                std::env::var_os("LK2_CODEX_OUTPUT")
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("LK2_ITER_DIR")
                            .map(PathBuf::from)
                            .map(|dir| dir.join("codex_main_scene.json"))
                    })
            })
            .unwrap_or_else(|| PathBuf::from("screenshots/codex_main_scene.json"));
        Self {
            max_runtime,
            output_path,
            ..default()
        }
    }
}

impl Default for CodexSceneState {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            last_request_at: 0.0,
            request_sequence: 0,
            active_request: None,
            requests: 0,
            successful_decisions: 0,
            failed_decisions: 0,
            last_intent: String::new(),
            last_reason: String::new(),
            last_error: String::new(),
            max_runtime: None,
            output_path: PathBuf::from("screenshots/codex_main_scene.json"),
            state_written_at: 0.0,
            first_position: None,
            current_position: None,
            finished: false,
        }
    }
}

pub fn observe_decide_act(
    time: Res<Time>,
    worker: Option<Res<CodexWorker>>,
    mut state: ResMut<CodexSceneState>,
    mut input: ResMut<CodexSceneInput>,
    nature: Res<OfflineNature>,
    players: Query<&Transform, With<PlayerActor>>,
) {
    state.elapsed += time.delta_secs();
    input.pulse_secs = (input.pulse_secs - time.delta_secs()).max(0.0);
    if input.pulse_secs <= 0.0 {
        input.jump = false;
    }

    if let Some(worker) = worker.as_ref() {
        let responses = worker
            .responses
            .lock()
            .map(|mut queue| queue.drain(..).collect::<Vec<_>>())
            .unwrap_or_default();
        for response in responses {
            if state.active_request != Some(response.request_id) {
                continue;
            }
            state.active_request = None;
            match response.result {
                Ok(decision) => {
                    state.successful_decisions = state.successful_decisions.saturating_add(1);
                    state.last_intent = decision.intent.clone();
                    state.last_reason = decision.reason.unwrap_or_default();
                    input.motion = Vec2::new(
                        decision.dx_milli as f32 / 1000.0,
                        decision.dz_milli as f32 / 1000.0,
                    )
                    .clamp_length_max(1.0);
                    input.jump = decision.intent == "jump";
                    input.mine = decision.intent == "mine";
                    input.attack = matches!(decision.intent.as_str(), "kill" | "attack");
                    input.pulse_secs = 0.25;
                }
                Err(error) => {
                    state.failed_decisions = state.failed_decisions.saturating_add(1);
                    state.last_error = error;
                    // Keep the visible player moving even while a cold Codex
                    // request is warming up or the provider is unavailable.
                    input.motion = fallback_motion(state.elapsed);
                }
            }
        }
    }

    let Ok(player) = players.single() else {
        return;
    };
    if state.first_position.is_none() {
        state.first_position = Some(player.translation);
    }
    state.current_position = Some(player.translation);
    if state.elapsed - state.state_written_at >= 1.0 {
        write_scene_artifact(&state, player.translation, nature.tick);
        state.state_written_at = state.elapsed;
    }

    if state.active_request.is_some()
        || state.elapsed - state.last_request_at < DECISION_INTERVAL_SECS
    {
        return;
    }
    let Some(worker) = worker else {
        return;
    };

    state.request_sequence = state.request_sequence.saturating_add(1).max(1);
    let request_id = state.request_sequence;
    let observation = scene_observation(player.translation, &nature, &state, input.motion);
    if worker
        .requests
        .send(CodexRequest {
            request_id,
            observation,
        })
        .is_err()
    {
        state.last_error = "Codex worker channel closed".to_string();
        input.motion = fallback_motion(state.elapsed);
        return;
    }
    state.active_request = Some(request_id);
    state.last_request_at = state.elapsed;
    state.requests = state.requests.saturating_add(1);
}

pub fn consume_action_pulses(mut input: ResMut<CodexSceneInput>) {
    input.mine = false;
    input.attack = false;
}

pub fn stop_after_requested_runtime(time: Res<Time>, mut state: ResMut<CodexSceneState>) {
    let Some(max_runtime) = state.max_runtime else {
        return;
    };
    if state.finished || time.elapsed() < max_runtime {
        return;
    }
    if let Some(position) = state.current_position {
        write_scene_artifact(&state, position, 0);
    }
    state.finished = true;
    std::process::exit(0);
}

fn write_scene_artifact(state: &CodexSceneState, position: Vec3, tick: u64) {
    if let Some(parent) = state.output_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let payload = json!({
        "schema": 1,
        "role": "codex_main_scene",
        "scene": "living_forest",
        "provider": "codex_cli",
        "elapsed_secs": state.elapsed,
        "tick": tick,
        "decision": {
            "requests": state.requests,
            "successful_decisions": state.successful_decisions,
            "failed_decisions": state.failed_decisions,
            "last_intent": state.last_intent,
            "last_reason": state.last_reason,
            "last_error": state.last_error
        },
        "movement_probe": {
            "first_pos": state.first_position.map(|value| [value.x, value.y, value.z]),
            "current_pos": [position.x, position.y, position.z]
        }
    });
    if let Ok(text) = serde_json::to_string_pretty(&payload) {
        let _ = fs::write(&state.output_path, text);
    }
}

fn scene_observation(
    position: Vec3,
    nature: &OfflineNature,
    state: &CodexSceneState,
    motion: Vec2,
) -> CodexObservation {
    let block_pos = [
        48 + position.x.floor() as i32,
        position.y.floor() as i32,
        48 + position.z.floor() as i32,
    ];
    let resource = |kind| nature.resources.get(kind);
    CodexObservation {
        tick: nature.tick,
        position: [position.x, position.y, position.z],
        block_pos,
        nation_id: None,
        monsters_killed: 0,
        blocks_gathered: 0,
        nations_founded: 0,
        inventory_wood: resource(ResourceKind::Wood),
        inventory_food: resource(ResourceKind::Food),
        inventory_apple: resource(ResourceKind::Apple),
        inventory_soul: resource(ResourceKind::Soul),
        pool_wood: resource(ResourceKind::Wood),
        pool_food: resource(ResourceKind::Food),
        pool_apple: resource(ResourceKind::Apple),
        pool_soul: resource(ResourceKind::Soul),
        flag_count: 0,
        total_nations: 0,
        monster_count: 0,
        status_line: "Living Forest 主场景".to_string(),
        last_feedback: String::new(),
        last_intent: if motion.length_squared() > 0.01 {
            "move".to_string()
        } else {
            state.last_intent.clone()
        },
    }
}

fn fallback_motion(elapsed: f32) -> Vec2 {
    match ((elapsed / 6.0).floor() as u32) % 4 {
        0 => Vec2::Y,
        1 => Vec2::X,
        2 => -Vec2::Y,
        _ => -Vec2::X,
    }
}
