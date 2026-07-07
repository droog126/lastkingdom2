use std::path::PathBuf;

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
use crate::render::{CameraAngles, CameraMode, NestMarker, RenderTelemetry, TerrainChunk};
use crate::ui::ClientRunMode;

pub const FIRST_SCREENSHOT_MIN_PROGRESS: u64 = 100;
const FIRST_SCREENSHOT_MIN_WALL_SECS: f32 = 4.0;
const SCREENSHOT_DIR: &str = "screenshots";

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
    let first_progress = first_screenshot_min_progress();
    let capture_tick = if *params.run_mode == ClientRunMode::Offline {
        clock.tick
    } else {
        clock.frame_tick
    };
    if capture_tick == first_progress {
        eprintln!(
            "[shot] gate reached frame={} sim_tick={} capture_tick={} wall={:.1} mode={:?}",
            clock.frame_tick, clock.tick, capture_tick, now, *params.camera_mode
        );
    }

    if capture_tick >= first_progress && now - clock.last_screenshot_wall >= 4.0 {
        eprintln!(
            "[shot] fire frame={} sim_tick={} capture_tick={} wall={:.1} last={:.1}",
            clock.frame_tick, clock.tick, capture_tick, now, clock.last_screenshot_wall
        );
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
}

pub fn tick_recorder(
    mut rec: ResMut<TickRecorder>,
    clock: Res<SimClock>,
    params: CaptureStateParams,
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
    );
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, s);
        info!("📝 tick state dumped → {}", path);
    }
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
            render_telemetry_json(render_telemetry),
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

fn render_telemetry_json(telemetry: &RenderTelemetry) -> serde_json::Value {
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
        }
    })
}

fn vec3_json(v: Vec3) -> serde_json::Value {
    serde_json::json!([v.x, v.y, v.z])
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
