use bevy::prelude::*;
#[cfg(feature = "client-render")]
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use std::path::PathBuf;

use crate::clock::SimClock;
use crate::monster::MonsterEcosystem;
use crate::nation::NationRegistry;
use crate::player::PlayerState;
use crate::resource::{GlobalResourcePool, PoolError, ResourceKind};
use crate::world::BlockType;
use crate::world::World as GameWorld;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Scenario {
    pub name: String,
    #[serde(default)]
    pub record_window: Option<(u64, u64)>,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum ScenarioStep {
    #[serde(rename = "move_to")]
    MoveTo { pos: [i32; 3] },

    #[serde(rename = "step")]
    Step { dir: [i32; 3] },

    #[serde(rename = "gather")]
    Gather { count: u32 },

    #[serde(rename = "attack")]
    Attack,

    #[serde(rename = "found_nation")]
    FoundNation,

    #[serde(rename = "upgrade_pop")]
    UpgradePop { target: u32 },

    #[serde(rename = "wait_ticks")]
    WaitTicks { ticks: u64 },

    #[serde(rename = "screenshot")]
    Screenshot { name: String },

    #[serde(rename = "record_begin")]
    RecordBegin,

    #[serde(rename = "record_end")]
    RecordEnd,

    #[serde(rename = "log")]
    Log { msg: String },

    #[serde(rename = "quit")]
    Quit,

    #[serde(rename = "add_geo_layer")]
    AddGeoLayer {
        layer: crate::world::terrain::ShapeLayer,
    },

    #[serde(rename = "remove_geo_layer")]
    RemoveGeoLayer { name: String },

    #[serde(rename = "clear_geo_overlay")]
    ClearGeoOverlay,
}

#[derive(Resource, Default)]
pub struct ScenarioState {
    pub scenario: Option<Scenario>,
    pub current_step: usize,
    pub last_step_done_tick: u64,
    pub step_in_progress: bool,
    pub recording: bool,
    pub record_buffer: Vec<RecordedTick>,
    pub record_path: PathBuf,
    pub current_dir: [i32; 3],
    pub pending_target: Option<[i32; 3]>,
    pub pending_gather_left: u32,
    pub end_requested: bool,

    pub move_to_started_at_tick: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecordedTick {
    pub tick: u64,
    pub player: [f32; 3],
    pub player_block: [i32; 3],
    pub wood: i64,
    pub food: i64,
    pub apple: i64,
    pub soul: i64,
    pub flags: u32,
    pub monsters: u32,
    pub nation_id: Option<u32>,
    pub blocks_gathered: u32,
    pub nations_founded: u32,
    pub monsters_killed: u32,
    pub step_label: String,
}

impl Default for Scenario {
    fn default() -> Self {
        Self { name: "default".into(), record_window: Some((0, 30)), steps: vec![] }
    }
}

impl ScenarioState {
    pub fn from_scenario(s: Scenario) -> Self {
        Self {
            scenario: Some(s),
            current_step: 0,
            last_step_done_tick: 0,
            step_in_progress: false,
            recording: false,
            record_buffer: Vec::new(),
            record_path: PathBuf::from("record.json"),
            current_dir: [1, 0, 0],
            pending_target: None,
            pending_gather_left: 0,
            end_requested: false,
            move_to_started_at_tick: None,
        }
    }
}

#[cfg(test)]
mod smoke_tests {
    use super::*;

    #[test]
    fn scenario_default() {
        let s = Scenario::default();
        assert_eq!(s.name, "default");
        assert!(s.record_window.is_some());
        assert!(s.steps.is_empty());
    }

    #[test]
    fn scenario_state_from_scenario() {
        let sc = Scenario {
            name: "test".into(),
            record_window: None,
            steps: vec![ScenarioStep::WaitTicks { ticks: 10 }],
        };
        let state = ScenarioState::from_scenario(sc);
        assert_eq!(state.current_step, 0);
        assert_eq!(state.last_step_done_tick, 0);
        assert_eq!(state.step_in_progress, false);
        assert_eq!(state.recording, false);
        assert_eq!(state.current_dir, [1, 0, 0]);
        assert_eq!(state.pending_target, None);
        assert_eq!(state.pending_gather_left, 0);
        assert_eq!(state.end_requested, false);
    }

    #[test]
    fn award_gathered_resource_adds_to_pool_and_player() {
        let mut pool = GlobalResourcePool::new();
        let mut player = PlayerState::default();

        let result = award_gathered_resource(&mut pool, &mut player, ResourceKind::Wood, 10);
        assert!(result.is_ok());
        assert_eq!(pool.get(ResourceKind::Wood), 10);
        assert_eq!(player.inventory.get(&ResourceKind::Wood), Some(&10));
        assert_eq!(player.blocks_gathered, 10);
    }

    #[test]
    fn award_gathered_resource_exceeds_pool_cap_fails() {
        let mut pool = GlobalResourcePool::new();
        pool.force_add(ResourceKind::Wood, ResourceKind::Wood.max());
        let mut player = PlayerState::default();

        let result = award_gathered_resource(&mut pool, &mut player, ResourceKind::Wood, 1);
        assert!(result.is_err());
        assert_eq!(player.blocks_gathered, 0);
    }

    #[test]
    fn load_scenario_from_args_or_default_uses_default_when_no_args() {
        let sc = load_scenario_from_args_or_default(&[]);
        assert_eq!(sc.name, "default");
        assert!(sc.steps.len() > 0);
    }

    #[test]
    fn load_scenario_from_args_or_default_uses_default_on_error() {
        let sc =
            load_scenario_from_args_or_default(&["--offline".into(), "nonexistent.json".into()]);
        assert_eq!(sc.name, "default");
    }

    #[test]
    fn scenario_step_variants() {
        assert!(matches!(
            ScenarioStep::MoveTo { pos: [10, 5, 10] },
            ScenarioStep::MoveTo { .. }
        ));
        assert!(matches!(
            ScenarioStep::Step { dir: [1, 0, 0] },
            ScenarioStep::Step { .. }
        ));
        assert!(matches!(
            ScenarioStep::Gather { count: 3 },
            ScenarioStep::Gather { .. }
        ));
        assert!(matches!(ScenarioStep::Attack, ScenarioStep::Attack));
        assert!(matches!(
            ScenarioStep::FoundNation,
            ScenarioStep::FoundNation
        ));
        assert!(matches!(
            ScenarioStep::UpgradePop { target: 10 },
            ScenarioStep::UpgradePop { .. }
        ));
        assert!(matches!(
            ScenarioStep::WaitTicks { ticks: 5 },
            ScenarioStep::WaitTicks { .. }
        ));
        assert!(matches!(
            ScenarioStep::Screenshot { name: "test".into() },
            ScenarioStep::Screenshot { .. }
        ));
        assert!(matches!(
            ScenarioStep::RecordBegin,
            ScenarioStep::RecordBegin
        ));
        assert!(matches!(ScenarioStep::RecordEnd, ScenarioStep::RecordEnd));
        assert!(matches!(
            ScenarioStep::Log { msg: "test".into() },
            ScenarioStep::Log { .. }
        ));
        assert!(matches!(ScenarioStep::Quit, ScenarioStep::Quit));
    }

    #[test]
    fn recorded_tick_fields() {
        let rt = RecordedTick {
            tick: 100,
            player: [10.0, 5.0, 20.0],
            player_block: [10, 5, 20],
            wood: 100,
            food: 50,
            apple: 30,
            soul: 10,
            flags: 2,
            monsters: 50,
            nation_id: Some(0),
            blocks_gathered: 100,
            nations_founded: 1,
            monsters_killed: 5,
            step_label: "test".into(),
        };
        assert_eq!(rt.tick, 100);
        assert_eq!(rt.flags, 2);
        assert_eq!(rt.monsters, 50);
        assert_eq!(rt.nation_id, Some(0));
    }
}

pub fn award_gathered_resource(
    pool: &mut GlobalResourcePool,
    player: &mut PlayerState,
    kind: ResourceKind,
    amount: i64,
) -> Result<i64, PoolError> {
    let new_value = pool.try_add(kind, amount)?;
    *player.inventory.entry(kind).or_insert(0) += amount;
    player.blocks_gathered += amount as u32;
    Ok(new_value)
}

pub fn load_scenario_from_args_or_default(args: &[String]) -> Scenario {
    let path = args.iter().skip(1).find(|a| !a.starts_with("--") && a.ends_with(".json")).cloned();

    if let Some(p) = path {
        match std::fs::read_to_string(&p) {
            Ok(s) => match serde_json::from_str::<Scenario>(&s) {
                Ok(sc) => {
                    info!("📜 加载剧本: {} ({} steps)", p, sc.steps.len());
                    return sc;
                }
                Err(e) => {
                    warn!("⚠ 解析剧本失败: {} — 用默认", e);
                }
            },
            Err(e) => warn!("⚠ 读取剧本失败: {} — 用默认", e),
        }
    }

    Scenario {
        name: "default".into(),
        record_window: Some((0, 60)),
        steps: vec![
            ScenarioStep::Log { msg: "=== 默认剧本启动 ===".into() },
            ScenarioStep::WaitTicks { ticks: 2 },
            ScenarioStep::Screenshot { name: "spawn".into() },
            ScenarioStep::RecordBegin,
            ScenarioStep::MoveTo { pos: [20, 14, 20] },
            ScenarioStep::Gather { count: 3 },
            ScenarioStep::WaitTicks { ticks: 10 },
            ScenarioStep::Screenshot { name: "after_gather".into() },
            ScenarioStep::FoundNation,
            ScenarioStep::WaitTicks { ticks: 5 },
            ScenarioStep::Screenshot { name: "after_founded".into() },
            ScenarioStep::RecordEnd,
            ScenarioStep::WaitTicks { ticks: 5 },
            ScenarioStep::Log { msg: "=== 结束 ===".into() },
            ScenarioStep::Quit,
        ],
    }
}

pub fn scenario_runner(
    time: Res<Time>,
    mut state: ResMut<ScenarioState>,
    clock: Res<SimClock>,
    mut commands: Commands,
    mut game_world: ResMut<GameWorld>,
) {
    let Some(scenario) = state.scenario.clone() else {
        return;
    };

    if state.end_requested {
        if clock.tick > state.last_step_done_tick + 3 {
            std::process::exit(0);
        }
        return;
    }

    if state.step_in_progress {
        return;
    }

    if state.current_step >= scenario.steps.len() {
        if clock.tick > state.last_step_done_tick + 3 {
            std::process::exit(0);
        }
        return;
    }

    let step = &scenario.steps[state.current_step];
    state.step_in_progress = true;

    match step {
        ScenarioStep::Log { msg } => {
            info!("📝 {}", msg);
            advance_step(&mut state);
        }
        ScenarioStep::WaitTicks { ticks } => {
            if clock.tick >= state.last_step_done_tick + ticks {
                advance_step(&mut state);
            } else {
                state.step_in_progress = false;
            }
        }
        ScenarioStep::RecordBegin => {
            if scenario.name == "default" || scenario.name == "idle" {
                state.recording = false;
                state.record_buffer.clear();
                info!(
                    "⏭ 跳过录制 (scenario={}, 默认不录以避免 29MB 泄漏)",
                    scenario.name
                );
                advance_step(&mut state);
                return;
            }
            state.recording = true;
            state.record_buffer.clear();
            let path = format!("screenshots/record_{}.jsonl", scenario.name);

            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.len() > 1_048_576 {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let rotated = format!("screenshots/record_{}.{}.jsonl", scenario.name, ts);
                    let _ = std::fs::rename(&path, &rotated);
                    info!("🔁 录制文件超 1MB, 已轮转 → {}", rotated);
                }
            }
            state.record_path = PathBuf::from(path);
            info!("🔴 开始录制 → {}", state.record_path.display());
            advance_step(&mut state);
        }
        ScenarioStep::RecordEnd => {
            state.recording = false;

            if !state.record_path.as_os_str().is_empty() {
                let jsonl: Vec<String> = state
                    .record_buffer
                    .iter()
                    .filter_map(|r| serde_json::to_string(r).ok())
                    .collect();
                let body = jsonl.join("\n");
                let _ = std::fs::write(&state.record_path, body);
                info!(
                    "⏹ 停止录制，写入 {} ({} ticks)",
                    state.record_path.display(),
                    state.record_buffer.len()
                );
            }
            advance_step(&mut state);
        }
        ScenarioStep::Screenshot { name } => {
            let path = format!("screenshots/{}_{}.png", scenario.name, name);
            info!("📸 截图 → {}", path);
            #[cfg(feature = "client-render")]
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
            #[cfg(not(feature = "client-render"))]
            let _ = (&mut commands, &path);

            if clock.tick > state.last_step_done_tick {
                advance_step(&mut state);
            } else {
                state.step_in_progress = false;
            }
        }
        ScenarioStep::MoveTo { pos } => {
            state.pending_target = Some(*pos);
            state.move_to_started_at_tick = Some(clock.tick);
            info!("🚶 走向 {:?}（tick {}）", pos, clock.tick);
        }
        ScenarioStep::Step { dir } => {
            state.current_dir = *dir;
            info!("👣 步 {:?}", dir);
            advance_step(&mut state);
        }
        ScenarioStep::Gather { count } => {
            state.pending_gather_left = *count;
            info!("⛏ 采掘 {} 次", count);
            state.step_in_progress = false;
        }
        ScenarioStep::Attack => {
            info!("⚔ 攻击最近怪物");
            advance_step(&mut state);
        }
        ScenarioStep::FoundNation => {
            info!("🏴 创国");
            advance_step(&mut state);
        }
        ScenarioStep::UpgradePop { target } => {
            info!("📈 升级人口到 {}", target);
            advance_step(&mut state);
        }
        ScenarioStep::Quit => {
            info!("🏁 剧本结束");
            state.end_requested = true;
            state.step_in_progress = false;
        }
        ScenarioStep::AddGeoLayer { layer } => {
            let n = layer.name.clone();
            game_world.push_geo_layer(layer.clone());
            info!(
                "🧩 add_geo_layer: {} (overlay 现 {} 层)",
                n,
                game_world.geo_overlay.len()
            );
            advance_step(&mut state);
        }
        ScenarioStep::RemoveGeoLayer { name } => {
            let removed = game_world.remove_geo_layer(&name);
            info!(
                "🗑 remove_geo_layer: {} ({}), overlay 现 {} 层",
                name,
                if removed { "found" } else { "not found" },
                game_world.geo_overlay.len()
            );
            advance_step(&mut state);
        }
        ScenarioStep::ClearGeoOverlay => {
            game_world.clear_geo_overlay();
            info!("🧹 clear_geo_overlay");
            advance_step(&mut state);
        }
    }
    let _ = time;
}

fn advance_step(state: &mut ScenarioState) {
    state.current_step += 1;
    state.step_in_progress = false;
}

pub fn simulate_player_actions(
    mut player: ResMut<PlayerState>,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut nations: ResMut<NationRegistry>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut state: ResMut<ScenarioState>,
    clock: Res<SimClock>,
) {
    if let Some(target) = state.pending_target {
        let cur = player.block_pos;

        if let Some(start) = state.move_to_started_at_tick {
            if clock.tick.saturating_sub(start) > 200 {
                warn!(
                    "⏱ MoveTo 到 {:?} 超时（已走 {} tick），强制 advance",
                    target,
                    clock.tick.saturating_sub(start)
                );
                state.pending_target = None;
                state.move_to_started_at_tick = None;
                advance_step(&mut state);
                return;
            }
        }
        let d = [
            (target[0] - cur[0]).signum(),
            (target[1] - cur[1]).signum(),
            (target[2] - cur[2]).signum(),
        ];

        let dir = if d[0] != 0 {
            [d[0], 0, 0]
        } else if d[2] != 0 {
            [0, 0, d[2]]
        } else if d[1] != 0 {
            [0, d[1], 0]
        } else {
            [0, 0, 0]
        };

        if dir == [0, 0, 0] {
            state.pending_target = None;
            state.move_to_started_at_tick = None;
            info!("✓ 到达 {:?}", target);
            advance_step(&mut state);
        } else {
            try_move_with_fallback(&mut player, &mut game_world, dir, target);
        }
    } else if state.pending_gather_left > 0 {
        let (x, y, z) = (
            player.block_pos[0],
            player.block_pos[1],
            player.block_pos[2],
        );
        let b = game_world.get(x, y, z);
        if let Some((res, _)) = b.yields() {
            if b.is_solid() {
                match award_gathered_resource(&mut pool, &mut player, res, 1) {
                    Ok(_) => {
                        game_world.set(x, y, z, BlockType::Air);
                        state.pending_gather_left -= 1;
                        info!("⛏ 采掘 {:?} (还 {} 次)", res, state.pending_gather_left);
                    }
                    Err(err) => {
                        warn!("[scenario] failed to award gathered {:?}: {}", res, err);
                        state.pending_gather_left -= 1;
                        warn!(
                            "[scenario] skipped gather step for {:?} (还 {} 次)",
                            res, state.pending_gather_left
                        );
                    }
                }
            } else {
                info!("方块不可采掘，找下一个");

                let dirs: [[i32; 3]; 6] = [
                    [1, 0, 0],
                    [-1, 0, 0],
                    [0, 0, 1],
                    [0, 0, -1],
                    [0, 1, 0],
                    [0, -1, 0],
                ];
                let (cx, cy, cz) = (
                    player.block_pos[0],
                    player.block_pos[1],
                    player.block_pos[2],
                );
                for d in dirs {
                    let np = [cx + d[0], cy + d[1], cz + d[2]];
                    if !game_world.in_bounds(np[0], np[1], np[2]) {
                        continue;
                    }
                    if attempt_move(&mut player, &mut game_world, d) {
                        break;
                    }
                }
            }
        } else {
            info!("当前位置无可采掘物，找下一个");

            let dirs: [[i32; 3]; 6] = [
                [1, 0, 0],
                [-1, 0, 0],
                [0, 0, 1],
                [0, 0, -1],
                [0, 1, 0],
                [0, -1, 0],
            ];
            let (cx, cy, cz) = (
                player.block_pos[0],
                player.block_pos[1],
                player.block_pos[2],
            );
            for d in dirs {
                let np = [cx + d[0], cy + d[1], cz + d[2]];
                if !game_world.in_bounds(np[0], np[1], np[2]) {
                    continue;
                }
                if attempt_move(&mut player, &mut game_world, d) {
                    break;
                }
            }
            state.pending_gather_left -= 1;
        }
        if state.pending_gather_left == 0 {
            state.current_step += 1;
        }
    } else if let Some(_s) = step_active(&state, ScenarioStepKind::Attack) {
        let mut best: Option<(u32, u32, u32, f32)> = None;
        for (kid, k) in monsters.kingdoms.iter() {
            if k.destroyed {
                continue;
            }
            for (nid, n) in k.nests.iter() {
                for (iid, ind) in n.individuals.iter() {
                    let dx = (ind.position[0] - player.block_pos[0]) as f32;
                    let dy = (ind.position[1] - player.block_pos[1]) as f32;
                    let dz = (ind.position[2] - player.block_pos[2]) as f32;
                    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                    if dist < 4.0 && (best.is_none() || dist < best.unwrap().3) {
                        best = Some((*kid, *nid, *iid, dist));
                    }
                }
            }
        }
        if let Some((kid, nid, iid, _)) = best {
            let removed = monsters.kill_individual(kid, nid, iid, &mut pool);
            if removed {
                player.monsters_killed += 1;
                info!("⚔ 击杀 monster #{iid}");
            }
        } else {
            info!("附近 4 格无怪物");
        }
        state.current_step += 1;
    } else if let Some(_s) = step_active(&state, ScenarioStepKind::FoundNation) {
        if nations.can_found_new() {
            let cost = nations.next_flag_cost() as u64;
            let flag_count = nations.flag_count;
            if let Ok(used_id) = nations.found(
                &mut pool,
                0,
                format!("PlayerNation#{}", flag_count + 1),
                player.block_pos,
                0,
            ) {
                player.nation_id = Some(used_id);
                player.nations_founded += 1;
                info!("🏴 创国成功 id={} cost={}", used_id.0, cost);
            }
        } else {
            info!("国旗已满 8");
        }
        state.current_step += 1;
    } else if let Some(target) = step_active(&state, ScenarioStepKind::UpgradePop) {
        if let Some(my_id) = player.nation_id {
            if let Some(n) = nations.nations.get_mut(&my_id) {
                if n.pop_cap < target {
                    let cost = if n.pop_cap < 10 {
                        crate::constant::POP_UPGRADE_10_COST
                    } else if n.pop_cap < 15 {
                        crate::constant::POP_UPGRADE_15_COST
                    } else {
                        crate::constant::POP_UPGRADE_20_COST
                    };
                    let (w, f, s) = cost;
                    if pool.get(ResourceKind::Wood) >= w as i64
                        && pool.get(ResourceKind::Food) >= f as i64
                        && pool.get(ResourceKind::Soul) >= s as i64
                    {
                        let _ = pool.try_sub(ResourceKind::Wood, w as i64);
                        let _ = pool.try_sub(ResourceKind::Food, f as i64);
                        let _ = pool.try_sub(ResourceKind::Soul, s as i64);
                        n.pop_cap = if n.pop_cap < 10 {
                            10
                        } else if n.pop_cap < 15 {
                            15
                        } else {
                            20
                        };
                        info!("📈 升级人口到 {}", n.pop_cap);
                    } else {
                        warn!("资源不足升级人口");
                    }
                }
            }
        } else {
            warn!("未创国，无法升级");
        }
        state.current_step += 1;
    }
}

#[derive(Debug, Clone, Copy)]
enum ScenarioStepKind {
    MoveTo,
    Step,
    Gather,
    Attack,
    FoundNation,
    UpgradePop,
    WaitTicks,
    Screenshot,
    RecordBegin,
    RecordEnd,
    Log,
    Quit,
    AddGeoLayer,
    RemoveGeoLayer,
    ClearGeoOverlay,
}

fn step_active(state: &ScenarioState, kind: ScenarioStepKind) -> Option<u32> {
    let scenario = state.scenario.as_ref()?;
    let step = scenario.steps.get(state.current_step)?;
    let actual_kind = match step {
        ScenarioStep::MoveTo { .. } => ScenarioStepKind::MoveTo,
        ScenarioStep::Step { .. } => ScenarioStepKind::Step,
        ScenarioStep::Gather { .. } => ScenarioStepKind::Gather,
        ScenarioStep::Attack => ScenarioStepKind::Attack,
        ScenarioStep::FoundNation => ScenarioStepKind::FoundNation,
        ScenarioStep::UpgradePop { .. } => ScenarioStepKind::UpgradePop,
        ScenarioStep::WaitTicks { .. } => ScenarioStepKind::WaitTicks,
        ScenarioStep::Screenshot { .. } => ScenarioStepKind::Screenshot,
        ScenarioStep::RecordBegin => ScenarioStepKind::RecordBegin,
        ScenarioStep::RecordEnd => ScenarioStepKind::RecordEnd,
        ScenarioStep::Log { .. } => ScenarioStepKind::Log,
        ScenarioStep::Quit => ScenarioStepKind::Quit,
        ScenarioStep::AddGeoLayer { .. } => ScenarioStepKind::AddGeoLayer,
        ScenarioStep::RemoveGeoLayer { .. } => ScenarioStepKind::RemoveGeoLayer,
        ScenarioStep::ClearGeoOverlay => ScenarioStepKind::ClearGeoOverlay,
    };
    if actual_kind as u32 == kind as u32 {
        Some(0)
    } else {
        None
    }
}

fn try_move_with_fallback(
    player: &mut PlayerState,
    game_world: &mut GameWorld,
    preferred: [i32; 3],
    target: [i32; 3],
) -> bool {
    if attempt_move(player, game_world, preferred) {
        return true;
    }
    const CANDIDATES: [[i32; 3]; 6] = [
        [1, 0, 0],
        [-1, 0, 0],
        [0, 0, 1],
        [0, 0, -1],
        [0, 1, 0],
        [0, -1, 0],
    ];
    let cur = player.block_pos;
    let best = CANDIDATES
        .iter()
        .filter(|d| **d != preferred)
        .filter(|d| {
            let np = [cur[0] + d[0], cur[1] + d[1], cur[2] + d[2]];
            game_world.in_bounds(np[0], np[1], np[2])
        })
        .min_by_key(|d| {
            let np = [cur[0] + d[0], cur[1] + d[1], cur[2] + d[2]];
            (np[0] - target[0]).abs() + (np[1] - target[1]).abs() + (np[2] - target[2]).abs()
        });
    if let Some(d) = best {
        attempt_move(player, game_world, *d)
    } else {
        false
    }
}

fn attempt_move(player: &mut PlayerState, game_world: &mut GameWorld, d: [i32; 3]) -> bool {
    let new_pos = [
        player.block_pos[0] + d[0],
        player.block_pos[1] + d[1],
        player.block_pos[2] + d[2],
    ];
    if !game_world.in_bounds(new_pos[0], new_pos[1], new_pos[2]) {
        info!(
            "❌ move {:?} -> {:?} OUT OF BOUNDS",
            player.block_pos, new_pos
        );
        return false;
    }
    let b = game_world.get(new_pos[0], new_pos[1], new_pos[2]);
    if b.is_solid() {
        for up in 1..=4 {
            let try_pos = [new_pos[0], new_pos[1] + up, new_pos[2]];
            if game_world.in_bounds(try_pos[0], try_pos[1], try_pos[2])
                && !game_world.get(try_pos[0], try_pos[1], try_pos[2]).is_solid()
            {
                player.block_pos = try_pos;
                player.pos = Vec3::new(
                    try_pos[0] as f32 + 0.5,
                    try_pos[1] as f32 + 0.5,
                    try_pos[2] as f32 + 0.5,
                );
                info!("↗ jump {:?} -> {:?}", player.block_pos, try_pos);
                return true;
            }
        }
        info!(
            "❌ move {:?} -> {:?} BLOCKED + no fly room",
            player.block_pos, new_pos
        );
        return false;
    }
    player.block_pos = new_pos;
    player.pos = Vec3::new(
        new_pos[0] as f32 + 0.5,
        new_pos[1] as f32 + 0.5,
        new_pos[2] as f32 + 0.5,
    );
    info!("→ move to {:?}", new_pos);
    true
}

pub fn scenario_tick_recorder(
    clock: Res<SimClock>,
    state: Res<ScenarioState>,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
) {
    if !state.recording {
        return;
    }
    let step_label = state
        .scenario
        .as_ref()
        .and_then(|s| s.steps.get(state.current_step))
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "?".into());
    let rec = RecordedTick {
        tick: clock.tick,
        player: [player.pos.x, player.pos.y, player.pos.z],
        player_block: player.block_pos,
        wood: pool.get(ResourceKind::Wood),
        food: pool.get(ResourceKind::Food),
        apple: pool.get(ResourceKind::Apple),
        soul: pool.get(ResourceKind::Soul),
        flags: nations.flag_count,
        monsters: monsters.current_individuals,
        nation_id: player.nation_id.map(|n| n.0),
        blocks_gathered: player.blocks_gathered,
        nations_founded: player.nations_founded,
        monsters_killed: player.monsters_killed,
        step_label,
    };

    if let Ok(line) = serde_json::to_string(&rec) {
        use std::io::Write;
        if let Ok(mut f) =
            std::fs::OpenOptions::new().create(true).append(true).open(&state.record_path)
        {
            let _ = writeln!(f, "{}", line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ResourceKind;
    use crate::world::terrain::{CylinderShape, ShapeLayer, ShapeSpec};

    #[test]
    fn award_gathered_resource_reports_full_pool_without_stats() {
        let mut pool = GlobalResourcePool::new();
        let mut player = PlayerState::default();
        pool.try_add(ResourceKind::Wood, ResourceKind::Wood.max()).unwrap();

        let err =
            award_gathered_resource(&mut pool, &mut player, ResourceKind::Wood, 1).unwrap_err();

        assert!(matches!(
            err,
            PoolError::WouldExceedMax { kind: ResourceKind::Wood, .. }
        ));
        assert_eq!(pool.get(ResourceKind::Wood), ResourceKind::Wood.max());
        assert_eq!(player.blocks_gathered, 0);
        assert_eq!(player.inventory.get(&ResourceKind::Wood), None);
    }

    #[test]
    fn scenario_parses_add_geo_layer() {
        let json = r#"{
            "name": "t",
            "steps": [
                { "type": "add_geo_layer", "layer": {
                    "name": "tower",
                    "weight": 12.0,
                    "fill": { "Replace": "Wood" },
                    "shapes": [
                        { "kind": "cylinder", "name": "b", "center_x": 50, "center_z": 48, "y_min": 0, "y_max": 40, "radius": 2.0 }
                    ],
                    "biome_override": null,
                    "enabled": true
                }},
                { "type": "remove_geo_layer", "name": "tower" },
                { "type": "clear_geo_overlay" }
            ]
        }"#;
        let s: Scenario = serde_json::from_str(json).expect("parse");
        assert_eq!(s.steps.len(), 3);
        match &s.steps[0] {
            ScenarioStep::AddGeoLayer { layer } => {
                assert_eq!(layer.name, "tower");
                assert_eq!(layer.weight, 12.0);
                assert_eq!(layer.shapes.len(), 1);
                match &layer.shapes[0] {
                    ShapeSpec::Cylinder(c) => {
                        assert_eq!(c.center_x, 50);
                        assert_eq!(c.y_max, 40);
                        assert!((c.radius - 2.0).abs() < 1e-6);
                    }
                    _ => panic!("expected cylinder"),
                }
            }
            _ => panic!("expected add_geo_layer"),
        }
        match &s.steps[1] {
            ScenarioStep::RemoveGeoLayer { name } => assert_eq!(name, "tower"),
            _ => panic!("expected remove_geo_layer"),
        }
        match &s.steps[2] {
            ScenarioStep::ClearGeoOverlay => {}
            _ => panic!("expected clear_geo_overlay"),
        }
    }

    #[test]
    fn world_geo_overlay_push_and_remove() {
        use crate::world::World;
        let mut w = World::new(8);
        let layer = ShapeLayer {
            name: "a".into(),
            weight: 5.0,
            fill: crate::world::terrain::FillMode::Replace(crate::world::BlockType::Stone),
            shapes: vec![ShapeSpec::Cylinder(CylinderShape {
                name: "c".into(),
                center_x: 0,
                center_z: 0,
                y_min: 0,
                y_max: 5,
                radius: 1.0,
            })],
            biome_override: None,
            enabled: true,
        };
        w.push_geo_layer(layer.clone());
        assert_eq!(w.geo_overlay.len(), 1);

        let mut layer2 = layer.clone();
        layer2.weight = 9.0;
        w.push_geo_layer(layer2);
        assert_eq!(w.geo_overlay.len(), 1);
        assert_eq!(w.geo_overlay[0].weight, 9.0);

        assert!(w.remove_geo_layer("a"));
        assert_eq!(w.geo_overlay.len(), 0);
        assert!(!w.remove_geo_layer("a"));

        w.push_geo_layer(layer);
        w.clear_geo_overlay();
        assert_eq!(w.geo_overlay.len(), 0);
    }

    #[test]
    fn world_generate_voxel_uses_overlay_first() {
        use crate::world::{BlockType, World};

        let mut w = World::new(8);
        w.procedural = true;

        let baseline = w.generate_voxel(3, 1, 3);

        let layer = ShapeLayer {
            name: "override".into(),
            weight: 99.0,
            fill: crate::world::terrain::FillMode::Replace(BlockType::Wood),
            shapes: vec![ShapeSpec::Box(crate::world::terrain::BoxShape {
                name: "b".into(),
                min: [0, 0, 0],
                max: [5, 5, 5],
            })],
            biome_override: None,
            enabled: true,
        };
        w.push_geo_layer(layer);
        let with_overlay = w.generate_voxel(3, 1, 3);
        assert_eq!(
            with_overlay,
            BlockType::Wood,
            "overlay 应该覆盖 default pipeline"
        );

        let _ = baseline;
    }
}
