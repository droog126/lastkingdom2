use std::{io::Write, path::PathBuf};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::eco_cycle::EcoCycle;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::resource::GlobalResourcePool;
use lk2_core::world::World as GameWorld;

use crate::OnlineCommandDiagnostics;
use crate::pretty::{PlayerReadabilityMarker, WorldGroundFallback};
use crate::render::{
    CameraAngles, CameraMode, NestMarker, RenderLightingTelemetry, RenderTelemetry, TerrainChunk,
};
use crate::ui::ClientRunMode;

pub const FIRST_SCREENSHOT_MIN_PROGRESS: u64 = 100;
const FIRST_SCREENSHOT_MIN_WALL_SECS: f32 = 4.0;
const SCREENSHOT_DIR: &str = "screenshots";
const OBSERVATION_TRACE: &str = "screenshots/observation_trace.jsonl";
const EVENT_TRACE: &str = "screenshots/event_trace.jsonl";
const ACTION_TRACE: &str = "screenshots/action_trace.jsonl";

fn capture_enabled() -> bool {
    std::env::var("LK2_CAPTURE").is_ok() || std::env::args().any(|a| a == "--auto-demo")
}

fn first_screenshot_min_progress() -> u64 {
    first_screenshot_min_progress_from_env(
        std::env::var("LK2_FIRST_SCREENSHOT_PROGRESS").ok().as_deref(),
    )
}

fn first_screenshot_min_progress_from_env(value: Option<&str>) -> u64 {
    value
        .and_then(|s| s.parse::<u64>().ok())
        .map(|progress| progress.max(FIRST_SCREENSHOT_MIN_PROGRESS))
        .unwrap_or(FIRST_SCREENSHOT_MIN_PROGRESS)
}

fn screenshot_gate_ready(
    run_mode: ClientRunMode,
    capture_tick: u64,
    first_progress: u64,
    wall_secs: f32,
) -> bool {
    if wall_secs < FIRST_SCREENSHOT_MIN_WALL_SECS {
        return false;
    }
    run_mode != ClientRunMode::Offline || capture_tick >= first_progress
}

#[derive(Resource, Default)]
pub struct TickRecorder {
    pub last_dump_tick: u64,
    pub current_iter: u32,
}

#[derive(Default)]
pub(crate) struct PerceptionTraceState {
    sample_index: u64,
    last_observation: Option<serde_json::Value>,
}

#[derive(Resource, Default, Clone)]
pub struct StaticWorldVisualSnapshot {
    pub first_player_pos: Option<Vec3>,
    pub current_player_pos: Option<Vec3>,
    pub first_positions: Vec<(&'static str, Vec3)>,
    pub current_positions: Vec<(&'static str, Vec3)>,
    pub player_marker_count: usize,
    pub player_marker_max_distance: Option<f32>,
    pub camera_transform: Option<Transform>,
}

#[derive(SystemParam)]
pub struct CaptureStateParams<'w> {
    time: Res<'w, Time>,
    player: Res<'w, PlayerState>,
    pool: Res<'w, GlobalResourcePool>,
    nations: Res<'w, NationRegistry>,
    monsters: Res<'w, MonsterEcosystem>,
    eco: Res<'w, EcoCycle>,
    obs: Res<'w, TickObserver>,
    game_world: Res<'w, GameWorld>,
    run_mode: Res<'w, ClientRunMode>,
    camera_angles: Res<'w, CameraAngles>,
    camera_mode: Res<'w, CameraMode>,
    online_commands: Res<'w, OnlineCommandDiagnostics>,
    static_world_visuals: Res<'w, StaticWorldVisualSnapshot>,
    render_telemetry: Res<'w, RenderTelemetry>,
    lighting_telemetry: Res<'w, RenderLightingTelemetry>,
}

pub fn update_static_world_visual_snapshot(
    mut snapshot: ResMut<StaticWorldVisualSnapshot>,
    player: Res<PlayerState>,
    ground_fallback: Query<&Transform, With<WorldGroundFallback>>,
    player_markers: Query<&Transform, With<PlayerReadabilityMarker>>,
    nest_markers: Query<&Transform, With<NestMarker>>,
    terrain_chunks: Query<&Transform, With<TerrainChunk>>,
    camera: Query<&Transform, With<Camera3d>>,
) {
    let mut current_positions = Vec::new();
    if let Some(tf) = ground_fallback.iter().next() {
        current_positions.push(("ground_fallback", tf.translation));
    }
    if let Some(tf) = nest_markers.iter().next() {
        current_positions.push(("nest_marker_0", tf.translation));
    }
    if let Some(tf) = terrain_chunks.iter().next() {
        current_positions.push(("terrain_chunk_0", tf.translation));
    }

    snapshot.current_player_pos = Some(player.pos);
    snapshot.current_positions = current_positions.clone();
    snapshot.player_marker_count = player_markers.iter().count();
    snapshot.player_marker_max_distance = player_markers
        .iter()
        .map(|tf| {
            Vec2::new(
                tf.translation.x - player.pos.x,
                tf.translation.z - player.pos.z,
            )
            .length()
        })
        .max_by(|a, b| a.total_cmp(b));
    snapshot.camera_transform = camera.iter().next().cloned();
    if snapshot.first_player_pos.is_none() && !current_positions.is_empty() {
        snapshot.first_player_pos = Some(player.pos);
        snapshot.first_positions = current_positions;
    }
}

pub fn periodic_screenshot(
    mut clock: ResMut<SimClock>,
    mut commands: Commands,
    params: CaptureStateParams,
) {
    if !capture_enabled() {
        return;
    }

    let now = {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        let start = START.get_or_init(std::time::Instant::now);
        start.elapsed().as_secs_f32()
    };
    let _ = &params.time;
    let flicker_probe_dir = std::env::var("LK2_FLICKER_PROBE_DIR").ok();
    let first_progress = first_screenshot_min_progress();
    let capture_tick = if *params.run_mode == ClientRunMode::Offline {
        clock.tick
    } else {
        clock.frame_tick
    };
    if flicker_probe_dir.is_none() && capture_tick == first_progress {
        eprintln!(
            "[shot] gate reached frame={} sim_tick={} capture_tick={} wall={:.1} mode={:?}",
            clock.frame_tick, clock.tick, capture_tick, now, *params.camera_mode
        );
    }

    if flicker_probe_dir.is_none()
        && capture_tick >= first_progress
        && now - clock.last_screenshot_wall >= 4.0
    {
        eprintln!(
            "[shot] fire frame={} sim_tick={} capture_tick={} wall={:.1} last={:.1}",
            clock.frame_tick, clock.tick, capture_tick, now, clock.last_screenshot_wall
        );
    }

    if let Some(probe_dir) = flicker_probe_dir.as_deref() {
        if now < 1.0 {
            return;
        }
        if !write_flicker_probe_capture(&mut clock, &mut commands, &params, probe_dir, now) {
            return;
        }
        return;
    }

    if !screenshot_gate_ready(*params.run_mode, capture_tick, first_progress, now) {
        // iter_210: in online mode SimClock.tick doesn't advance (server runs
        // the authoritative sim), so the offline-only first screenshot progress
        // gate would block every screenshot forever. Fall back to wall-clock
        // so online first-person can still produce a screenshot.
        return;
    }

    let interval = std::env::var("LK2_SCREENSHOT_INTERVAL")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(4.0);
    if now - clock.last_screenshot_wall < interval {
        return;
    }
    clock.last_screenshot_wall = now;
    if clock.screenshot_count == 0 {
        clock.screenshot_count = latest_existing_iter_id().unwrap_or(0);
    }
    clock.screenshot_count += 1;
    let iter_id = clock.screenshot_count;
    let iter_dir = format!("{SCREENSHOT_DIR}/iter_{:02}", iter_id);
    let _ = std::fs::create_dir_all(&iter_dir);
    let png_path: PathBuf = format!("{}/iter_{:02}.png", iter_dir, iter_id).into();
    let state_path = format!("{}/final_state.json", iter_dir);
    let diff_path = format!("{}/diff.json", iter_dir);
    let perception_manifest_path = format!("{}/perception_manifest.json", iter_dir);

    info!("📸 截图 #{} → {}", iter_id, png_path.display());
    commands
        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(png_path));

    let state = build_state_json(
        &params.time,
        &clock,
        &params.player,
        &params.pool,
        &params.nations,
        &params.monsters,
        &params.eco,
        &params.obs,
        &params.game_world,
        *params.run_mode,
        &params.camera_angles,
        *params.camera_mode,
        &params.online_commands,
        &params.static_world_visuals,
        &params.render_telemetry,
        &params.lighting_telemetry,
    );
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        if let Err(e) = std::fs::write(&state_path, s) {
            warn!("写 final_state.json 失败: {}", e);
        } else {
            info!("📝 final_state dumped → {}", state_path);
        }
    }
    if let Some(diff) = build_state_diff_for_iter(iter_id, &state) {
        if let Ok(s) = serde_json::to_string_pretty(&diff) {
            if let Err(e) = std::fs::write(&diff_path, s) {
                warn!("write diff.json failed: {}", e);
            } else {
                info!("diff.json dumped -> {}", diff_path);
            }
        }
    }
    let perception_manifest = build_perception_manifest(iter_id, &state);
    if let Ok(s) = serde_json::to_string_pretty(&perception_manifest) {
        if let Err(e) = std::fs::write(&perception_manifest_path, s) {
            warn!("write perception_manifest.json failed: {}", e);
        } else {
            info!("perception manifest dumped -> {}", perception_manifest_path);
        }
    }
}

fn write_flicker_probe_capture(
    clock: &mut SimClock,
    commands: &mut Commands,
    params: &CaptureStateParams<'_>,
    probe_dir: &str,
    now: f32,
) -> bool {
    let interval = std::env::var("LK2_SCREENSHOT_INTERVAL")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.25);
    if now - clock.last_screenshot_wall < interval {
        return false;
    }
    clock.last_screenshot_wall = now;
    clock.screenshot_count = clock.screenshot_count.saturating_add(1);
    let sample_id = clock.screenshot_count;
    let _ = std::fs::create_dir_all(probe_dir);
    let png_path: PathBuf = format!("{probe_dir}/frame_{sample_id:04}.png").into();
    let state_path = format!("{probe_dir}/state_{sample_id:04}.json");

    commands
        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(png_path));

    let state = build_state_json(
        &params.time,
        clock,
        &params.player,
        &params.pool,
        &params.nations,
        &params.monsters,
        &params.eco,
        &params.obs,
        &params.game_world,
        *params.run_mode,
        &params.camera_angles,
        *params.camera_mode,
        &params.online_commands,
        &params.static_world_visuals,
        &params.render_telemetry,
        &params.lighting_telemetry,
    );
    if let Ok(s) = serde_json::to_string_pretty(&state)
        && let Err(e) = std::fs::write(&state_path, s)
    {
        warn!("write flicker probe state failed: {}", e);
    }
    true
}

pub fn tick_recorder(
    mut rec: ResMut<TickRecorder>,
    clock: Res<SimClock>,
    params: CaptureStateParams,
    mut perception: Local<PerceptionTraceState>,
) {
    if !capture_enabled() {
        return;
    }

    use std::sync::atomic::{AtomicU64, Ordering};
    static LOCAL_FRAME_TICK: AtomicU64 = AtomicU64::new(0);
    let local_frame_tick = LOCAL_FRAME_TICK.fetch_add(1, Ordering::Relaxed) + 1;
    let sample_tick = clock.tick.max(clock.frame_tick / 60).max(local_frame_tick / 60);
    if sample_tick == 0 || sample_tick % 5 != 0 || sample_tick == rec.last_dump_tick {
        return;
    }
    rec.last_dump_tick = sample_tick;
    rec.current_iter = sample_tick as u32;
    let path = format!("screenshots/state_t{}.json", sample_tick);
    let state = build_state_json(
        &params.time,
        &clock,
        &params.player,
        &params.pool,
        &params.nations,
        &params.monsters,
        &params.eco,
        &params.obs,
        &params.game_world,
        *params.run_mode,
        &params.camera_angles,
        *params.camera_mode,
        &params.online_commands,
        &params.static_world_visuals,
        &params.render_telemetry,
        &params.lighting_telemetry,
    );
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, s);
        info!("📝 tick state dumped → {}", path);
    }
    record_perception_tick(&mut perception, sample_tick, &state);
}

fn latest_existing_iter_id() -> Option<u32> {
    std::fs::read_dir(SCREENSHOT_DIR)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_dir() {
                return None;
            }
            let name = entry.file_name();
            name.to_str()?.strip_prefix("iter_")?.parse::<u32>().ok()
        })
        .max()
}

fn build_state_json(
    time: &Time,
    clock: &SimClock,
    player: &PlayerState,
    pool: &GlobalResourcePool,
    nations: &NationRegistry,
    monsters: &MonsterEcosystem,
    eco: &EcoCycle,
    obs: &TickObserver,
    game_world: &GameWorld,
    run_mode: ClientRunMode,
    camera_angles: &CameraAngles,
    camera_mode: CameraMode,
    online_commands: &OnlineCommandDiagnostics,
    static_world_visuals: &StaticWorldVisualSnapshot,
    render_telemetry: &RenderTelemetry,
    lighting_telemetry: &RenderLightingTelemetry,
) -> serde_json::Value {
    let mut state = lk2_core::diagnostics::build_state_json(
        time,
        clock,
        player,
        pool,
        nations,
        monsters,
        eco,
        obs,
        game_world,
        run_mode.snapshot_role(),
    );
    if let Some(obj) = state.as_object_mut() {
        let first_person_forward = first_person_forward(camera_angles.yaw, camera_angles.pitch);
        let eye = player.pos + Vec3::Y * 1.7;
        let (center_ray_origin, center_ray_forward) = static_world_visuals
            .camera_transform
            .as_ref()
            .map(|tf| {
                (
                    tf.translation,
                    tf.rotation.mul_vec3(Vec3::NEG_Z).normalize_or_zero(),
                )
            })
            .unwrap_or((eye, first_person_forward));
        let hit = ray_voxel_first_hit(game_world, center_ray_origin, center_ray_forward, 160.0);
        obj.insert(
            "camera".to_string(),
            serde_json::json!({
                "mode": format!("{:?}", camera_mode),
                "yaw": camera_angles.yaw,
                "pitch": camera_angles.pitch,
                "first_person_eye": [eye.x, eye.y, eye.z],
                "first_person_forward": [first_person_forward.x, first_person_forward.y, first_person_forward.z],
                "center_ray_origin": [center_ray_origin.x, center_ray_origin.y, center_ray_origin.z],
                "center_ray_forward": [center_ray_forward.x, center_ray_forward.y, center_ray_forward.z],
                "center_ray_hit": hit.map(|h| serde_json::json!({
                    "block_pos": h.0,
                    "point": [h.1.x, h.1.y, h.1.z],
                    "distance": h.2,
                    "block": format!("{:?}", h.3),
                })),
            }),
        );
        obj.insert(
            "network_command".to_string(),
            serde_json::json!({
                "sender_entities": online_commands.sender_entities,
                "move_world_sent": online_commands.move_world_sent,
                "last_dx_milli": online_commands.last_dx_milli,
                "last_dz_milli": online_commands.last_dz_milli,
            }),
        );
        obj.insert(
            "visual".to_string(),
            serde_json::json!({
                "static_world": static_world_visual_json(&static_world_visuals.current_positions),
                "movement_probe": movement_probe_json(static_world_visuals),
                "player_readability": player_readability_json(static_world_visuals),
            }),
        );
        obj.insert(
            "render".to_string(),
            render_telemetry_json(render_telemetry, lighting_telemetry),
        );
    }
    state
}

fn player_readability_json(snapshot: &StaticWorldVisualSnapshot) -> serde_json::Value {
    serde_json::json!({
        "marker_count": snapshot.player_marker_count,
        "marker_max_distance": snapshot.player_marker_max_distance,
    })
}

fn static_world_visual_json(positions: &[(&'static str, Vec3)]) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (name, pos) in positions {
        obj.insert((*name).to_string(), vec3_json(*pos));
    }
    serde_json::Value::Object(obj)
}

fn movement_probe_json(snapshot: &StaticWorldVisualSnapshot) -> serde_json::Value {
    serde_json::json!({
        "first_player_pos": snapshot.first_player_pos.map(|p| vec3_json(p)),
        "current_player_pos": snapshot.current_player_pos.map(|p| vec3_json(p)),
        "first_static_world": static_world_visual_json(&snapshot.first_positions),
        "current_static_world": static_world_visual_json(&snapshot.current_positions),
    })
}

fn render_telemetry_json(
    telemetry: &RenderTelemetry,
    lighting: &RenderLightingTelemetry,
) -> serde_json::Value {
    serde_json::json!({
        "frame": {
            "samples": telemetry.frame_samples,
            "dt_max_ms": telemetry.frame_dt_max_ms,
            "dt_over_50ms": telemetry.frame_dt_over_50ms,
        },
        "terrain": {
            "smooth_mesh_builds": telemetry.smooth_mesh_builds,
            "smooth_mesh_total_ms": telemetry.smooth_mesh_total_ms,
            "smooth_mesh_max_ms": telemetry.smooth_mesh_max_ms,
            "smooth_mesh_total_tris": telemetry.smooth_mesh_total_tris,
            "greedy_mesh_builds": telemetry.greedy_mesh_builds,
            "greedy_mesh_total_ms": telemetry.greedy_mesh_total_ms,
            "greedy_mesh_max_ms": telemetry.greedy_mesh_max_ms,
            "terrain_despawns": telemetry.terrain_despawns,
            "collider_rebuilds": telemetry.collider_rebuilds,
        },
        "lighting": {
            "time_of_day": lighting.time_of_day,
            "dayness": lighting.dayness,
            "atmosphere_time_of_day": lighting.atmosphere_time_of_day,
            "atmosphere_dayness": lighting.atmosphere_dayness,
            "readable_dayness": lighting.readable_dayness,
            "sunset_glow": lighting.sunset_glow,
            "sun_illuminance": lighting.sun_illuminance,
            "fill_illuminance": lighting.fill_illuminance,
            "camera_mode": lighting.camera_mode,
        }
    })
}

fn vec3_json(v: Vec3) -> serde_json::Value {
    serde_json::json!([v.x, v.y, v.z])
}

fn build_perception_manifest(iter_id: u32, state: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "schema": "lk2.perception.manifest.v1",
        "iter": iter_id,
        "tick": value_at_path(state, "tick").and_then(|v| v.as_u64()),
        "frame_tick": value_at_path(state, "frame_tick").and_then(|v| v.as_u64()),
        "observation_trace": OBSERVATION_TRACE,
        "event_trace": EVENT_TRACE,
        "action_trace": ACTION_TRACE,
        "snapshot_observation": build_observation_sample(iter_id as u64, 0, state),
    })
}

fn record_perception_tick(
    trace: &mut PerceptionTraceState,
    sample_tick: u64,
    state: &serde_json::Value,
) {
    let observation = build_observation_sample(trace.sample_index, sample_tick, state);
    if let Some(prev) = trace.last_observation.as_ref() {
        for event in infer_perception_events(prev, &observation) {
            append_jsonl(EVENT_TRACE, &event);
        }
        if let Some(action) = infer_perception_action(prev, &observation) {
            append_jsonl(ACTION_TRACE, &action);
        }
    }
    append_jsonl(OBSERVATION_TRACE, &observation);
    trace.sample_index = trace.sample_index.saturating_add(1);
    trace.last_observation = Some(observation);
}

fn build_observation_sample(
    sample_index: u64,
    sample_tick: u64,
    state: &serde_json::Value,
) -> serde_json::Value {
    let pool = value_at_path(state, "pool").cloned().unwrap_or_else(|| serde_json::json!({}));
    let nations = value_at_path(state, "nations").cloned().unwrap_or_else(|| serde_json::json!({}));
    let monsters =
        value_at_path(state, "monsters").cloned().unwrap_or_else(|| serde_json::json!({}));
    let creatures =
        value_at_path(state, "creatures").cloned().unwrap_or_else(|| serde_json::json!({}));
    let observer =
        value_at_path(state, "observer").cloned().unwrap_or_else(|| serde_json::json!({}));
    let network_command =
        value_at_path(state, "network_command").cloned().unwrap_or_else(|| serde_json::json!({}));
    let render = value_at_path(state, "render").cloned().unwrap_or_else(|| serde_json::json!({}));

    serde_json::json!({
        "schema": "lk2.perception.observation.v1",
        "sample_index": sample_index,
        "sample_tick": sample_tick,
        "tick": value_at_path(state, "tick").and_then(|v| v.as_u64()),
        "frame_tick": value_at_path(state, "frame_tick").and_then(|v| v.as_u64()),
        "wall_secs": value_at_path(state, "wall_secs").and_then(|v| v.as_f64()),
        "role": value_at_path(state, "role").and_then(|v| v.as_str()),
        "player": {
            "pos": value_at_path(state, "player.pos").cloned(),
            "block_pos": value_at_path(state, "player.block_pos").cloned(),
            "nation_id": value_at_path(state, "player.nation_id").cloned(),
        },
        "camera": value_at_path(state, "camera").cloned().unwrap_or_else(|| serde_json::json!({})),
        "metrics": {
            "pool": pool,
            "nations": nations,
            "monsters": monsters,
            "creatures": creatures,
            "observer": observer,
            "network_command": network_command,
        },
        "render": render,
        "valid_actions": valid_actions_for_state(state),
    })
}

fn valid_actions_for_state(state: &serde_json::Value) -> Vec<&'static str> {
    let mut actions = vec!["wait", "look", "walk_forward", "turn", "gather_foot_block"];
    let wood = value_at_path(state, "pool.wood").and_then(|v| v.as_i64()).unwrap_or(0);
    if wood > 0 {
        actions.push("place_wood_foot_block");
    }
    actions.push("found_nation");
    if value_at_path(state, "monsters.current").and_then(|v| v.as_u64()).unwrap_or(0) > 0 {
        actions.push("attack_nearest");
    }
    if value_at_path(state, "role").and_then(|v| v.as_str()) == Some("client_online") {
        actions.push("send_move_world");
    }
    actions
}

fn infer_perception_action(
    prev: &serde_json::Value,
    current: &serde_json::Value,
) -> Option<serde_json::Value> {
    let tick = value_at_path(current, "tick").and_then(|v| v.as_u64());
    let sent_move =
        numeric_delta(prev, current, "metrics.network_command.move_world_sent").unwrap_or(0.0);
    if sent_move > 0.0 {
        return Some(serde_json::json!({
            "schema": "lk2.perception.action.v1",
            "tick": tick,
            "action": "send_move_world",
            "status": "sent",
            "count_delta": sent_move,
        }));
    }

    if let Some(distance) = position_distance(prev, current, "player.pos") {
        if distance > 0.05 {
            return Some(serde_json::json!({
                "schema": "lk2.perception.action.v1",
                "tick": tick,
                "action": "walk_forward",
                "status": "observed_progress",
                "distance": distance,
            }));
        }
    }

    if numeric_delta(prev, current, "tick").unwrap_or(0.0) > 0.0 {
        return Some(serde_json::json!({
            "schema": "lk2.perception.action.v1",
            "tick": tick,
            "action": "wait",
            "status": "tick_progress",
        }));
    }
    None
}

fn infer_perception_events(
    prev: &serde_json::Value,
    current: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let tick = value_at_path(current, "tick").and_then(|v| v.as_u64());
    let mut events = Vec::new();
    if let Some(distance) = position_distance(prev, current, "player.pos") {
        if distance > 0.05 {
            events.push(serde_json::json!({
                "schema": "lk2.perception.event.v1",
                "tick": tick,
                "event": "player_moved",
                "distance": distance,
                "from": value_at_path(prev, "player.pos").cloned(),
                "to": value_at_path(current, "player.pos").cloned(),
            }));
        }
    }

    for (path, event) in [
        ("metrics.pool.wood", "pool_wood_changed"),
        ("metrics.pool.food", "pool_food_changed"),
        ("metrics.pool.apple", "pool_apple_changed"),
        ("metrics.pool.soul", "pool_soul_changed"),
        ("metrics.nations.flag_count", "flag_count_changed"),
        ("metrics.nations.total_nations", "nation_count_changed"),
        ("metrics.monsters.current", "monster_count_changed"),
        (
            "metrics.creatures.passive_current",
            "creature_count_changed",
        ),
        ("metrics.observer.anomalies", "observer_anomalies_changed"),
        (
            "metrics.observer.invariant_violations",
            "observer_invariant_violations_changed",
        ),
    ] {
        if let Some(delta) = numeric_delta(prev, current, path) {
            if delta.abs() > f64::EPSILON {
                events.push(serde_json::json!({
                    "schema": "lk2.perception.event.v1",
                    "tick": tick,
                    "event": event,
                    "path": path,
                    "delta": delta,
                    "previous": value_at_path(prev, path).cloned(),
                    "current": value_at_path(current, path).cloned(),
                }));
            }
        }
    }
    events
}

fn numeric_delta(prev: &serde_json::Value, current: &serde_json::Value, path: &str) -> Option<f64> {
    Some(value_at_path(current, path)?.as_f64()? - value_at_path(prev, path)?.as_f64()?)
}

fn position_distance(
    prev: &serde_json::Value,
    current: &serde_json::Value,
    path: &str,
) -> Option<f64> {
    let prev = value_at_path(prev, path)?.as_array()?;
    let current = value_at_path(current, path)?.as_array()?;
    if prev.len() != 3 || current.len() != 3 {
        return None;
    }
    let dx = current[0].as_f64()? - prev[0].as_f64()?;
    let dy = current[1].as_f64()? - prev[1].as_f64()?;
    let dz = current[2].as_f64()? - prev[2].as_f64()?;
    Some((dx * dx + dy * dy + dz * dz).sqrt())
}

fn append_jsonl(path: &str, value: &serde_json::Value) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => {
            if let Err(e) = writeln!(file, "{value}") {
                warn!("append {path} failed: {e}");
            }
        }
        Err(e) => warn!("open {path} failed: {e}"),
    }
}

fn first_person_forward(yaw: f32, pitch: f32) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    Vec3::new(sy * cp, sp, -cy * cp).normalize_or_zero()
}

fn ray_voxel_first_hit(
    world: &GameWorld,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> Option<([i32; 3], Vec3, f32, lk2_core::world::BlockType)> {
    let step = 0.05;
    let steps = (max_dist / step) as i32;
    for i in 1..=steps {
        let t = i as f32 * step;
        let p = origin + dir * t;
        let block = [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32];
        if !world.in_bounds(block[0], block[1], block[2]) {
            continue;
        }
        let kind = world.get(block[0], block[1], block[2]);
        if kind.is_solid() {
            return Some((block, p, t, kind));
        }
    }
    None
}

fn build_state_diff_for_iter(iter_id: u32, state: &serde_json::Value) -> Option<serde_json::Value> {
    if iter_id <= 1 {
        return None;
    }
    let prev_path = format!("screenshots/iter_{:02}/final_state.json", iter_id - 1);
    let prev_raw = std::fs::read_to_string(prev_path).ok()?;
    let prev_state: serde_json::Value = serde_json::from_str(&prev_raw).ok()?;
    Some(build_state_diff(&prev_state, state))
}

fn build_state_diff(prev: &serde_json::Value, current: &serde_json::Value) -> serde_json::Value {
    let mut deltas = Vec::new();
    for path in [
        "tick",
        "player.monsters_killed",
        "player.blocks_gathered",
        "player.nations_founded",
        "pool.wood",
        "pool.food",
        "pool.apple",
        "pool.soul",
        "nations.flag_count",
        "nations.total_nations",
        "monsters.current",
        "monsters.kingdoms",
        "monsters.nests",
        "creatures.passive_current",
        "eco_cycle.clouds",
        "eco_cycle.rainfall",
        "eco_cycle.plants_grown",
        "eco_cycle.rabbits_born",
        "eco_cycle.wildlife_born",
        "eco_cycle.rabbits",
        "eco_cycle.berry_bushes",
        "eco_cycle.fruit",
        "eco_cycle.fruit_eaten",
        "eco_cycle.fruit_grown",
        "observer.anomalies",
        "observer.invariant_violations",
    ] {
        if let Some(delta) = diff_numeric_path(prev, current, path) {
            deltas.push(delta);
        }
    }

    deltas.sort_by(|a, b| {
        let a_abs = a["delta_abs"].as_f64().unwrap_or(0.0);
        let b_abs = b["delta_abs"].as_f64().unwrap_or(0.0);
        b_abs.partial_cmp(&a_abs).unwrap_or(std::cmp::Ordering::Equal).then_with(|| {
            a["path"].as_str().unwrap_or_default().cmp(b["path"].as_str().unwrap_or_default())
        })
    });

    serde_json::json!({
        "prev_tick": value_at_path(prev, "tick").and_then(|v| v.as_u64()),
        "tick": value_at_path(current, "tick").and_then(|v| v.as_u64()),
        "resource_deltas": deltas,
    })
}

fn diff_numeric_path(
    prev: &serde_json::Value,
    current: &serde_json::Value,
    path: &str,
) -> Option<serde_json::Value> {
    let prev_num = value_at_path(prev, path)?.as_f64()?;
    let current_num = value_at_path(current, path)?.as_f64()?;
    let delta = current_num - prev_num;
    if delta.abs() < f64::EPSILON {
        return None;
    }
    Some(serde_json::json!({
        "path": path,
        "previous": prev_num,
        "current": current_num,
        "delta": delta,
        "delta_abs": delta.abs(),
    }))
}

fn value_at_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perception_test_state() -> serde_json::Value {
        serde_json::json!({
            "frame_tick": 600,
            "tick": 100,
            "wall_secs": 5.0,
            "role": "client_offline",
            "player": {
                "block_pos": [10, 20, 30],
                "pos": [10.0, 20.0, 30.0],
                "nation_id": null,
            },
            "pool": {
                "wood": 3,
                "food": 9,
                "apple": 1,
                "soul": 0,
            },
            "nations": {
                "flag_count": 1,
                "total_nations": 1,
            },
            "monsters": {
                "current": 2,
                "kingdoms": 1,
                "nests": 1,
            },
            "creatures": {
                "passive_current": 4,
            },
            "observer": {
                "anomalies": 0,
                "invariant_violations": 0,
            },
            "camera": {
                "mode": "TopDown",
                "center_ray_hit": {
                    "block": "Grass",
                    "distance": 12.0,
                },
            },
            "network_command": {
                "move_world_sent": 0,
            },
            "render": {
                "lighting": {
                    "time_of_day": 0.42,
                    "dayness": 0.97,
                    "readable_dayness": 0.97,
                    "sunset_glow": 0.59,
                    "sun_illuminance": 110000.0,
                    "fill_illuminance": 7000.0,
                    "camera_mode": "TopDown"
                }
            },
        })
    }

    #[test]
    fn observation_sample_exposes_semantic_state_and_valid_actions() {
        let obs = build_observation_sample(7, 105, &perception_test_state());

        assert_eq!(obs["schema"], "lk2.perception.observation.v1");
        assert_eq!(obs["sample_index"], 7);
        assert_eq!(obs["sample_tick"], 105);
        assert_eq!(obs["player"]["block_pos"], serde_json::json!([10, 20, 30]));
        assert_eq!(obs["camera"]["mode"], "TopDown");
        assert_eq!(obs["metrics"]["pool"]["wood"], 3);
        assert_eq!(obs["render"]["lighting"]["time_of_day"], 0.42);
        assert_eq!(obs["render"]["lighting"]["camera_mode"], "TopDown");
        let actions = obs["valid_actions"].as_array().unwrap();
        assert!(actions.iter().any(|a| a == "walk_forward"));
        assert!(actions.iter().any(|a| a == "place_wood_foot_block"));
        assert!(actions.iter().any(|a| a == "attack_nearest"));
    }

    #[test]
    fn perception_events_report_movement_and_metric_deltas() {
        let prev = build_observation_sample(0, 100, &perception_test_state());
        let mut changed_state = perception_test_state();
        changed_state["tick"] = serde_json::json!(105);
        changed_state["player"]["pos"] = serde_json::json!([11.0, 20.0, 30.0]);
        changed_state["pool"]["wood"] = serde_json::json!(1);
        changed_state["nations"]["total_nations"] = serde_json::json!(2);
        let current = build_observation_sample(1, 105, &changed_state);

        let events = infer_perception_events(&prev, &current);
        let names = events.iter().map(|e| e["event"].as_str().unwrap()).collect::<Vec<_>>();
        assert!(names.contains(&"player_moved"));
        assert!(names.contains(&"pool_wood_changed"));
        assert!(names.contains(&"nation_count_changed"));

        let wood = events.iter().find(|e| e["event"] == "pool_wood_changed").unwrap();
        assert_eq!(wood["delta"], -2.0);
    }

    #[test]
    fn perception_action_prefers_explicit_online_move_then_movement() {
        let mut prev_state = perception_test_state();
        prev_state["role"] = serde_json::json!("client_online");
        prev_state["network_command"]["move_world_sent"] = serde_json::json!(2);
        let prev = build_observation_sample(0, 100, &prev_state);

        let mut current_state = prev_state.clone();
        current_state["tick"] = serde_json::json!(105);
        current_state["network_command"]["move_world_sent"] = serde_json::json!(3);
        current_state["player"]["pos"] = serde_json::json!([11.0, 20.0, 30.0]);
        let current = build_observation_sample(1, 105, &current_state);

        let action = infer_perception_action(&prev, &current).unwrap();
        assert_eq!(action["action"], "send_move_world");
        assert_eq!(action["status"], "sent");

        let mut moved_state = perception_test_state();
        moved_state["tick"] = serde_json::json!(105);
        moved_state["player"]["pos"] = serde_json::json!([10.0, 20.0, 31.0]);
        let moved = build_observation_sample(1, 105, &moved_state);
        let action = infer_perception_action(
            &build_observation_sample(0, 100, &perception_test_state()),
            &moved,
        )
        .unwrap();
        assert_eq!(action["action"], "walk_forward");
        assert_eq!(action["status"], "observed_progress");
    }

    #[test]
    fn first_screenshot_progress_env_cannot_disable_offline_progress_gate() {
        assert_eq!(
            first_screenshot_min_progress_from_env(Some("0")),
            FIRST_SCREENSHOT_MIN_PROGRESS
        );
        assert_eq!(
            first_screenshot_min_progress_from_env(Some("12")),
            FIRST_SCREENSHOT_MIN_PROGRESS
        );
        assert_eq!(first_screenshot_min_progress_from_env(Some("250")), 250);
    }

    #[test]
    fn screenshot_gate_requires_wall_time_and_offline_progress() {
        assert!(!screenshot_gate_ready(
            ClientRunMode::Offline,
            FIRST_SCREENSHOT_MIN_PROGRESS,
            FIRST_SCREENSHOT_MIN_PROGRESS,
            FIRST_SCREENSHOT_MIN_WALL_SECS - 0.1,
        ));
        assert!(!screenshot_gate_ready(
            ClientRunMode::Offline,
            FIRST_SCREENSHOT_MIN_PROGRESS - 1,
            FIRST_SCREENSHOT_MIN_PROGRESS,
            FIRST_SCREENSHOT_MIN_WALL_SECS,
        ));
        assert!(screenshot_gate_ready(
            ClientRunMode::Offline,
            FIRST_SCREENSHOT_MIN_PROGRESS,
            FIRST_SCREENSHOT_MIN_PROGRESS,
            FIRST_SCREENSHOT_MIN_WALL_SECS,
        ));
        assert!(!screenshot_gate_ready(
            ClientRunMode::Offline,
            20,
            FIRST_SCREENSHOT_MIN_PROGRESS,
            FIRST_SCREENSHOT_MIN_WALL_SECS,
        ));
        assert!(screenshot_gate_ready(
            ClientRunMode::Online,
            0,
            FIRST_SCREENSHOT_MIN_PROGRESS,
            FIRST_SCREENSHOT_MIN_WALL_SECS,
        ));
    }

    #[test]
    fn state_diff_reports_sorted_numeric_deltas() {
        let prev = serde_json::json!({
            "tick": 100,
            "player": {
                "monsters_killed": 1,
                "blocks_gathered": 4,
                "nations_founded": 1
            },
            "pool": {
                "wood": 50,
                "food": 90,
                "apple": 70,
                "soul": 60
            },
            "nations": {
                "flag_count": 7,
                "total_nations": 7
            },
            "monsters": {
                "current": 60,
                "kingdoms": 1,
                "nests": 3
            },
            "creatures": {
                "passive_current": 5
            },
            "eco_cycle": {
                "rabbits": 5,
                "berry_bushes": 10,
                "fruit": 1,
                "fruit_eaten": 20,
                "fruit_grown": 11
            },
            "observer": {
                "anomalies": 0,
                "invariant_violations": 0
            }
        });
        let current = serde_json::json!({
            "tick": 125,
            "player": {
                "monsters_killed": 2,
                "blocks_gathered": 6,
                "nations_founded": 1
            },
            "pool": {
                "wood": 45,
                "food": 93,
                "apple": 70,
                "soul": 60
            },
            "nations": {
                "flag_count": 8,
                "total_nations": 8
            },
            "monsters": {
                "current": 58,
                "kingdoms": 1,
                "nests": 3
            },
            "creatures": {
                "passive_current": 4
            },
            "eco_cycle": {
                "rabbits": 5,
                "berry_bushes": 10,
                "fruit": 0,
                "fruit_eaten": 23,
                "fruit_grown": 13
            },
            "observer": {
                "anomalies": 0,
                "invariant_violations": 0
            }
        });

        let diff = build_state_diff(&prev, &current);
        assert_eq!(diff["prev_tick"], 100);
        assert_eq!(diff["tick"], 125);

        let deltas = diff["resource_deltas"].as_array().unwrap();
        let paths = deltas.iter().map(|delta| delta["path"].as_str().unwrap()).collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![
                "tick",
                "pool.wood",
                "eco_cycle.fruit_eaten",
                "pool.food",
                "eco_cycle.fruit_grown",
                "monsters.current",
                "player.blocks_gathered",
                "creatures.passive_current",
                "eco_cycle.fruit",
                "nations.flag_count",
                "nations.total_nations",
                "player.monsters_killed"
            ]
        );

        let wood = deltas.iter().find(|delta| delta["path"] == "pool.wood").unwrap();
        assert_eq!(wood["previous"], 50.0);
        assert_eq!(wood["current"], 45.0);
        assert_eq!(wood["delta"], -5.0);
    }
}
