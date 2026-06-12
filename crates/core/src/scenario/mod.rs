//! Scenario Harness — 玩家模拟 + 录制系统
//!
//! 用法：写一个 JSON 剧本（player actions sequence），传给 binary，
//! harness 会按剧本执行，并在指定 tick 窗口录制 state 到 JSON。
//!
//! 剧本格式 (scenario.json)：
//! ```json
//! {
//!   "name": "iter07_flat_test",
//!   "record_window": [0, 100],   // tick 0..100 录制
//!   "steps": [
//!     { "type": "move_to", "pos": [20, 14, 20] },
//!     { "type": "record_begin" },
//!     { "type": "gather", "count": 3 },
//!     { "type": "wait_ticks", "ticks": 30 },
//!     { "type": "found_nation" },
//!     { "type": "screenshot", "name": "after_founding" },
//!     { "type": "record_end" }
//!   ]
//! }
//! ```
//!
//! 启动方式：binary 第一个参数 = 剧本路径

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::clock::SimClock;
use crate::monster::MonsterEcosystem;
use crate::nation::NationRegistry;
use crate::player::PlayerState;
use crate::resource::GlobalResourcePool;
use crate::world::BlockType;
use crate::world::World as GameWorld;

// ---------------------------------------------------------------------------
// Scenario 定义
// ---------------------------------------------------------------------------

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
    /// 玩家走到指定坐标（3D A* 简单 BFS）
    #[serde(rename = "move_to")]
    MoveTo { pos: [i32; 3] },
    /// 玩家向上/下/北/南/东/西移动一格
    #[serde(rename = "step")]
    Step { dir: [i32; 3] },
    /// 采集当前方块 N 次
    #[serde(rename = "gather")]
    Gather { count: u32 },
    /// 攻击最近的怪物
    #[serde(rename = "attack")]
    Attack,
    /// 创国
    #[serde(rename = "found_nation")]
    FoundNation,
    /// 升级人口
    #[serde(rename = "upgrade_pop")]
    UpgradePop { target: u32 },
    /// 等待 N tick
    #[serde(rename = "wait_ticks")]
    WaitTicks { ticks: u64 },
    /// 截屏（保存到 screenshots/<name>.png）
    #[serde(rename = "screenshot")]
    Screenshot { name: String },
    /// 开始录制（之后每 tick 写 state 到 record log）
    #[serde(rename = "record_begin")]
    RecordBegin,
    /// 结束录制
    #[serde(rename = "record_end")]
    RecordEnd,
    /// 打印一行日志
    #[serde(rename = "log")]
    Log { msg: String },
    /// 退出
    #[serde(rename = "quit")]
    Quit,
}

// ---------------------------------------------------------------------------
// Scenario State
// ---------------------------------------------------------------------------

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
    /// MoveTo step 开始时的 tick；用于超时强制 advance（避免 fallback 死循环）
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

// ---------------------------------------------------------------------------
// 加载：要么从 CLI 参数，要么从 scenarios/default.json
// ---------------------------------------------------------------------------

pub fn load_scenario_from_args_or_default(args: &[String]) -> Scenario {
    // 第一个非 cargo 参数是剧本路径
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

    // 默认剧本：spawn 中心 + wander + record
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

// ---------------------------------------------------------------------------
// 剧本执行 system
// ---------------------------------------------------------------------------

pub fn scenario_runner(
    time: Res<Time>,
    mut state: ResMut<ScenarioState>,
    clock: Res<SimClock>,
    mut commands: Commands,
) {
    let Some(scenario) = state.scenario.clone() else {
        return;
    };

    if state.end_requested {
        // 等几 tick 让 screenshot 完成
        if clock.tick > state.last_step_done_tick + 3 {
            std::process::exit(0);
        }
        return;
    }

    if state.step_in_progress {
        return; // 当前 step 还在跑
    }

    if state.current_step >= scenario.steps.len() {
        // 跑完了，退出
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
            // 在 wait_ticks tick 之后推进
            if clock.tick >= state.last_step_done_tick + ticks {
                advance_step(&mut state);
            } else {
                state.step_in_progress = false; // 每 tick 重试
            }
        }
        ScenarioStep::RecordBegin => {
            // 修泄漏 (Sprint 1): 默认 scenario 不录 — 它会跨会话累积成几十 MB。
            // 真实剧本（user-supplied *.json）才需要录制。
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
            // 1MB 安全网: 若文件已超 1MB, 用 ts 后缀轮转, 避免老剧本反复跑累积。
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
            // 默认 scenario 没在录, buffer 应该是空的
            if !state.record_path.as_os_str().is_empty() {
                // dump record_buffer 到文件
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
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
            // 等 1 tick 让 screenshot 完成
            if clock.tick > state.last_step_done_tick {
                advance_step(&mut state);
            } else {
                state.step_in_progress = false;
            }
        }
        ScenarioStep::MoveTo { pos } => {
            // 之前这里写 step_in_progress = false，导致 scenario_runner 下一帧又跑
            // 同一行 MoveTo handler → log "🚶 走向" 几百次（见 baseline_check.log 之前的 spam）
            // 改成：保持 step_in_progress=true，由 simulate_player_actions 到达时 advance
            state.pending_target = Some(*pos);
            state.move_to_started_at_tick = Some(clock.tick);
            info!("🚶 走向 {:?}（tick {}）", pos, clock.tick);
        }
        ScenarioStep::Step { dir } => {
            // 立即生效，advance
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
    }
    let _ = time; // silence
}

fn advance_step(state: &mut ScenarioState) {
    state.current_step += 1;
    state.step_in_progress = false;
}

// ---------------------------------------------------------------------------
// 玩家模拟：完成 pending target / gather
// ---------------------------------------------------------------------------

pub fn simulate_player_actions(
    mut player: ResMut<PlayerState>,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut nations: ResMut<NationRegistry>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut state: ResMut<ScenarioState>,
    clock: Res<SimClock>,
) {
    // 1. 走 target
    if let Some(target) = state.pending_target {
        let cur = player.block_pos;
        // 超时：MoveTo 跑了 >200 tick 还没到 → 强制 advance（避免 fallback 卡死）
        // 1 tick ≈ 1s（sim 60s 现实 1 tick），200 tick = 200 sim 秒 = 真实 12 秒
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
        // 优先 XZ，再 Y
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
            // 到达
            state.pending_target = None;
            state.move_to_started_at_tick = None;
            info!("✓ 到达 {:?}", target);
            advance_step(&mut state); // 同时清 step_in_progress=false + current_step+=1
        } else {
            // 走 preferred 方向；OOB/被挡时回退 6 cardinal（修死循环）
            try_move_with_fallback(&mut player, &mut game_world, dir, target);
        }
    }
    // 2. 采掘
    else if state.pending_gather_left > 0 {
        let (x, y, z) = (
            player.block_pos[0],
            player.block_pos[1],
            player.block_pos[2],
        );
        let b = game_world.get(x, y, z);
        if let Some((res, _)) = b.yields() {
            if b.is_solid() {
                game_world.set(x, y, z, BlockType::Air);
                let _ = pool.try_add(res, 1);
                *player.inventory.entry(res).or_insert(0) += 1;
                player.blocks_gathered += 1;
                state.pending_gather_left -= 1;
                info!("⛏ 采掘 {:?} (还 {} 次)", res, state.pending_gather_left);
            } else {
                info!("方块不可采掘，找下一个");
                // 走一格再试（先过滤掉 OOB，避免再触发 OUT OF BOUNDS 日志）
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
            // 走一格再试（先过滤掉 OOB，避免再触发 OUT OF BOUNDS 日志）
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
            state.pending_gather_left -= 1; // skip
        }
        if state.pending_gather_left == 0 {
            state.current_step += 1;
        }
    }
    // 3. 攻击
    else if let Some(_s) = step_active(&state, ScenarioStepKind::Attack) {
        // 找最近
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
    }
    // 4. 创国
    else if let Some(_s) = step_active(&state, ScenarioStepKind::FoundNation) {
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
    }
    // 5. 升级人口
    else if let Some(target) = step_active(&state, ScenarioStepKind::UpgradePop) {
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
                    if pool.get(crate::resource::ResourceKind::Wood) >= w as i64
                        && pool.get(crate::resource::ResourceKind::Food) >= f as i64
                        && pool.get(crate::resource::ResourceKind::Soul) >= s as i64
                    {
                        let _ = pool.try_sub(crate::resource::ResourceKind::Wood, w as i64);
                        let _ = pool.try_sub(crate::resource::ResourceKind::Food, f as i64);
                        let _ = pool.try_sub(crate::resource::ResourceKind::Soul, s as i64);
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
    };
    if actual_kind as u32 == kind as u32 {
        Some(0)
    } else {
        None
    }
}

/// 走 preferred 方向；失败（OOB / 被挡）就回退到 6 个 cardinal 方向，
/// 挑 in-bounds 且到 target 曼哈顿距离最近的那个。彻底解决 MoveTo 死循环。
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
        // 找上方的空位
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

// ---------------------------------------------------------------------------
// Tick 录制：scenario 开启录制时，每 tick 写一行 JSONL
// ---------------------------------------------------------------------------

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
        wood: pool.get(crate::resource::ResourceKind::Wood),
        food: pool.get(crate::resource::ResourceKind::Food),
        apple: pool.get(crate::resource::ResourceKind::Apple),
        soul: pool.get(crate::resource::ResourceKind::Soul),
        flags: nations.flag_count,
        monsters: monsters.current_individuals,
        nation_id: player.nation_id.map(|n| n.0),
        blocks_gathered: player.blocks_gathered,
        nations_founded: player.nations_founded,
        monsters_killed: player.monsters_killed,
        step_label,
    };
    // 写到独立行（追加）
    if let Ok(line) = serde_json::to_string(&rec) {
        use std::io::Write;
        if let Ok(mut f) =
            std::fs::OpenOptions::new().create(true).append(true).open(&state.record_path)
        {
            let _ = writeln!(f, "{}", line);
        }
    }
}
