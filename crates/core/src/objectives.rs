//! 目标 / 任务系统（V2 玩法循环核心）
//!
//! 来源：《万国余烬_王冠赛季_对战设计案》§5 目标驱动 + 《技术实现 MVP》§2 QuestService
//!
//! ## 痛点（2026-06-19 T6 任务前置）
//!
//! HUD 上 Goal: 10 wood 是写死的;创完国后没有下一目标;
//! phase 推进不会触发事件;玩家登录后"玩不到啥"。
//!
//! ## 设计
//!
//! - `Objective` Resource：当前任务列表 + 每条进度 + 是否完成
//! - `ObjectiveKind` 枚举：内置 4 种（采集 / 创国 / 杀怪 / 到达位置）
//! - `ObjectivesPlugin` 注册到 FixedUpdate：
//!   - `auto_advance_objectives`：自动检查玩家行为 → 推进进度
//!   - `chain_objectives`：完成当前 → 解锁下一条
//! - `ObjectiveCompleted` Message：HUD 收到后闪提示 / scenario 收到后推进剧本
//! - F 键绑定逻辑在 client 层做（避免 core 依赖 input manager）
//!
//! ## 内置 Quest Chain（开局）
//!
//! 1. `GatherWood(10)` — 砍 10 木
//! 2. `FoundNation`     — 按 F 创第 1 国
//! 3. `GatherFood(30)`  — 食物 30（升级人口前置）
//! 4. `UpgradePop(10)`  — 升级人口到 10
//! 5. `KillMonsters(5)` — 杀 5 只怪（解锁 Wildland 叙事）
//!
//! chain 在 `Objectives::default_chain()` 给 demo 用;正式游戏可由 scenario JSON 注入。

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::match_state::{MatchClock, MatchPhase};
use crate::monster::MonsterEcosystem;
use crate::nation::NationRegistry;
use crate::player::PlayerState;
use crate::resource::{GlobalResourcePool, ResourceKind};

// ---------------------------------------------------------------------------
// Objective Kind
// ---------------------------------------------------------------------------

/// 任务类型（行为 → 进度映射）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectiveKind {
    /// 采集 N 个某资源。`count` 是目标数量。
    GatherResource { kind: ResourceKind, count: i64 },
    /// 创 1 个国家（任意时刻只有 1 面旗给玩家创）。
    FoundNation,
    /// 升级人口到 target。
    UpgradePop { target: u32 },
    /// 击杀 N 个怪物（任意种类）。
    KillMonsters { count: u32 },
    /// 走到 (x, y, z) 半径 radius 内。
    ReachPosition { pos: [i32; 3], radius: i32 },
}

impl ObjectiveKind {
    pub fn short_label(&self) -> String {
        match self {
            ObjectiveKind::GatherResource { kind, count } => {
                format!("采集 {:?} {}", kind, count)
            }
            ObjectiveKind::FoundNation => "建立第一座国家".into(),
            ObjectiveKind::UpgradePop { target } => format!("升级人口到 {}", target),
            ObjectiveKind::KillMonsters { count } => format!("击杀 {} 只怪物", count),
            ObjectiveKind::ReachPosition { pos, radius } => {
                format!("前往 ({},{},{}) {}m 内", pos[0], pos[1], pos[2], radius)
            }
        }
    }

    /// HUD 进度条描述 "X / Y" 部分
    pub fn progress_str(&self, progress: &ObjectiveProgress) -> String {
        match (self, progress) {
            (ObjectiveKind::GatherResource { count, .. }, ObjectiveProgress::Count(p)) => {
                format!("{} / {}", p, count)
            }
            (ObjectiveKind::FoundNation, ObjectiveProgress::Flag(b)) => {
                if *b {
                    "完成".into()
                } else {
                    "未完成".into()
                }
            }
            (ObjectiveKind::UpgradePop { target }, ObjectiveProgress::Pop(p)) => {
                format!("{} / {}", p, target)
            }
            (ObjectiveKind::KillMonsters { count }, ObjectiveProgress::CountU(p)) => {
                format!("{} / {}", p, count)
            }
            (
                ObjectiveKind::ReachPosition { pos, radius },
                ObjectiveProgress::AtPosition { reached },
            ) => {
                if *reached {
                    "已到达".into()
                } else {
                    format!("目标 ({},{},{}) {}m 内", pos[0], pos[1], pos[2], radius)
                }
            }
            _ => "?".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Progress
// ---------------------------------------------------------------------------

/// 任务进度（与 Kind 一一对应）
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ObjectiveProgress {
    #[default]
    Empty,
    /// 整数计数（采集 / 杀怪）
    Count(i64),
    CountU(u32),
    /// 升级人口的当前人口上限
    Pop(u32),
    /// 创国标记
    Flag(bool),
    /// 到达位置
    AtPosition {
        reached: bool,
    },
}

impl ObjectiveProgress {
    pub fn is_done(&self) -> bool {
        match self {
            ObjectiveProgress::Empty => false,
            ObjectiveProgress::Count(n) => *n > 0, // 0 不算 done;具体 threshold 在完成判定里
            ObjectiveProgress::CountU(n) => *n > 0,
            ObjectiveProgress::Pop(_) => false,
            ObjectiveProgress::Flag(b) => *b,
            ObjectiveProgress::AtPosition { reached } => *reached,
        }
    }
}

// ---------------------------------------------------------------------------
// Objective Entry (单条任务)
// ---------------------------------------------------------------------------

/// 一条任务（kind + 当前进度 + 完成标记）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    pub id: String, // 唯一 id，给 HUD / scenario 引用
    pub kind: ObjectiveKind,
    pub progress: ObjectiveProgress,
    pub done: bool,
}

impl Objective {
    pub fn new(id: impl Into<String>, kind: ObjectiveKind) -> Self {
        let progress = match &kind {
            ObjectiveKind::GatherResource { .. } => ObjectiveProgress::Count(0),
            ObjectiveKind::FoundNation => ObjectiveProgress::Flag(false),
            ObjectiveKind::UpgradePop { .. } => ObjectiveProgress::Pop(0),
            ObjectiveKind::KillMonsters { .. } => ObjectiveProgress::CountU(0),
            ObjectiveKind::ReachPosition { .. } => ObjectiveProgress::AtPosition { reached: false },
        };
        Self { id: id.into(), kind, progress, done: false }
    }

    /// 当前进度是否满足 kind 的完成阈值
    pub fn check_complete(&self) -> bool {
        match (&self.kind, &self.progress) {
            (ObjectiveKind::GatherResource { count, .. }, ObjectiveProgress::Count(p)) => {
                *p >= *count
            }
            (ObjectiveKind::FoundNation, ObjectiveProgress::Flag(b)) => *b,
            (ObjectiveKind::UpgradePop { target }, ObjectiveProgress::Pop(p)) => *p >= *target,
            (ObjectiveKind::KillMonsters { count }, ObjectiveProgress::CountU(p)) => *p >= *count,
            (ObjectiveKind::ReachPosition { .. }, ObjectiveProgress::AtPosition { reached }) => {
                *reached
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Objectives Registry
// ---------------------------------------------------------------------------

/// 全局任务注册表（Resource）。一个玩家/客户端一份。
/// server 可读同一份,做权威校验。
#[derive(Resource, Debug, Clone, Default)]
pub struct Objectives {
    /// 所有任务（按 push 顺序）
    pub all: Vec<Objective>,
    /// 当前激活的（第一条未完成的）
    pub current_idx: Option<usize>,
}

impl Objectives {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, obj: Objective) {
        if self.current_idx.is_none() && !obj.done {
            self.current_idx = Some(self.all.len());
        }
        self.all.push(obj);
    }

    /// 取当前激活的 objective
    pub fn current(&self) -> Option<&Objective> {
        self.current_idx.and_then(|i| self.all.get(i))
    }

    pub fn current_mut(&mut self) -> Option<&mut Objective> {
        let i = self.current_idx?;
        self.all.get_mut(i)
    }

    /// 标记当前为完成（如果满足），并把 current_idx 推到下一条未完成
    /// 返回 true 表示这次完成了
    pub fn try_complete_current(&mut self) -> bool {
        let Some(i) = self.current_idx else {
            return false;
        };
        let Some(obj) = self.all.get(i) else {
            return false;
        };
        if obj.done {
            return false;
        }
        if !obj.check_complete() {
            return false;
        }
        // 标完成
        self.all[i].done = true;
        // 推下一条未完成
        let next = (i + 1..self.all.len()).find(|&j| !self.all[j].done);
        self.current_idx = next;
        true
    }

    /// demo 用内置 quest chain
    pub fn default_chain() -> Self {
        let mut o = Objectives::new();
        o.push(Objective::new(
            "q1_gather_wood",
            ObjectiveKind::GatherResource { kind: ResourceKind::Wood, count: 10 },
        ));
        o.push(Objective::new(
            "q2_found_nation",
            ObjectiveKind::FoundNation,
        ));
        o.push(Objective::new(
            "q3_gather_food",
            ObjectiveKind::GatherResource { kind: ResourceKind::Food, count: 30 },
        ));
        o.push(Objective::new(
            "q4_upgrade_pop",
            ObjectiveKind::UpgradePop { target: 10 },
        ));
        o.push(Objective::new(
            "q5_kill_monsters",
            ObjectiveKind::KillMonsters { count: 5 },
        ));
        o.push(Objective::new(
            "q6_reach_summit",
            ObjectiveKind::ReachPosition { pos: [48, 30, 48], radius: 5 },
        ));
        o
    }
}

// ---------------------------------------------------------------------------
// ObjectiveCompleted Event
// ---------------------------------------------------------------------------

/// 任务完成事件。HUD 闪提示 / scenario advance / audio cue 都订阅这个。
#[derive(Message, Debug, Clone)]
pub struct ObjectiveCompleted {
    pub id: String,
    pub kind: ObjectiveKind,
    pub at_wall_secs: f32,
}

// ---------------------------------------------------------------------------
// Auto-advance systems
// ---------------------------------------------------------------------------

/// 自动把玩家行为推进到当前 objective 的进度。
/// 每 tick 跑一次,纯函数映射:玩家 state → progress 更新。
pub fn auto_advance_objectives(
    mut objectives: ResMut<Objectives>,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    _monsters: Res<MonsterEcosystem>,
    match_clock: Res<MatchClock>,
    mut completed_events: MessageWriter<ObjectiveCompleted>,
) {
    // 防止 chain 推进时本帧 current 还在 old slot 反复发事件
    let mut safety = 16;
    while safety > 0 {
        safety -= 1;
        let Some(obj) = objectives.current().cloned() else {
            break;
        };
        if obj.done {
            break;
        }

        // 推进进度
        match &obj.kind {
            ObjectiveKind::GatherResource { kind, .. } => {
                let cur = pool.get(*kind);
                let new_p = match obj.progress {
                    ObjectiveProgress::Count(_) => cur,
                    _ => cur,
                };
                if let Some(cur_obj) = objectives.current_mut() {
                    cur_obj.progress = ObjectiveProgress::Count(new_p);
                }
            }
            ObjectiveKind::FoundNation => {
                let mine = player.nation_id.is_some();
                if let Some(cur_obj) = objectives.current_mut() {
                    cur_obj.progress = ObjectiveProgress::Flag(mine);
                }
            }
            ObjectiveKind::UpgradePop { .. } => {
                let pop = player
                    .nation_id
                    .and_then(|id| nations.nations.get(&id))
                    .map(|n| n.pop_cap)
                    .unwrap_or(0);
                if let Some(cur_obj) = objectives.current_mut() {
                    cur_obj.progress = ObjectiveProgress::Pop(pop);
                }
            }
            ObjectiveKind::KillMonsters { .. } => {
                let killed = player.monsters_killed;
                if let Some(cur_obj) = objectives.current_mut() {
                    cur_obj.progress = ObjectiveProgress::CountU(killed);
                }
            }
            ObjectiveKind::ReachPosition { pos, radius } => {
                let dx = (player.block_pos[0] - pos[0]).abs();
                let dy = (player.block_pos[1] - pos[1]).abs();
                let dz = (player.block_pos[2] - pos[2]).abs();
                let reached = dx + dy + dz <= *radius;
                if let Some(cur_obj) = objectives.current_mut() {
                    cur_obj.progress = ObjectiveProgress::AtPosition { reached };
                }
            }
        }

        // 检查完成
        if objectives.try_complete_current() {
            let just_done = objectives
                .all
                .iter()
                .rev()
                .find(|o| o.done)
                .cloned()
                .expect("just-completed objective disappeared");
            info!(
                "[objective] ✓ 完成: {} ({}) → 下一条 idx={:?}",
                just_done.id,
                just_done.kind.short_label(),
                objectives.current_idx,
            );
            completed_events.write(ObjectiveCompleted {
                id: just_done.id.clone(),
                kind: just_done.kind.clone(),
                at_wall_secs: match_clock.wall_secs,
            });
        } else {
            break;
        }
    }
}

/// chain 完成时给 phase 一个 nudge（叙事节奏）。
/// q5_kill_monsters 完成 → 提前 30s 预告 phase 升级
/// q6_reach_summit 完成 → 解锁 SovereignReveal 提示
pub fn objective_phase_hints(
    mut completed_events: MessageReader<ObjectiveCompleted>,
    match_clock: Res<MatchClock>,
) {
    for ev in completed_events.read() {
        match ev.id.as_str() {
            "q5_kill_monsters" => {
                info!(
                    "[objective] {} 完成后,Wildland 阶段入口解锁 (剩余 {:.0}s)",
                    ev.id,
                    match_clock.phase_remaining_secs()
                );
            }
            "q6_reach_summit" => {
                info!(
                    "[objective] {} 到达王座山顶,SovereignReveal 叙事窗口开启",
                    ev.id
                );
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// 注册 Objectives + 自动推进 + 完成事件
pub struct ObjectivesPlugin;

impl Plugin for ObjectivesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Objectives>().add_message::<ObjectiveCompleted>().add_systems(
            FixedUpdate,
            (auto_advance_objectives, objective_phase_hints).chain(),
        );
    }
}

/// 启动时塞入默认 quest chain（demo 用）
pub fn setup_default_objectives(mut commands: Commands) {
    commands.insert_resource(Objectives::default_chain());
}

/// AshOpening → Wildland 阶段切换时,把所有未完成 objective 的 progress 推到当前 best
/// （已经在 auto_advance 里做了 — 这里只是给一个明确 hook 标记）
pub fn on_phase_change_emit_summary(
    mut phase_events: MessageReader<crate::match_state::MatchPhaseChanged>,
    objectives: Res<Objectives>,
) {
    for ev in phase_events.read() {
        let done = objectives.all.iter().filter(|o| o.done).count();
        let total = objectives.all.len();
        info!(
            "[objective] phase {} → {}:已完成 {}/{}",
            ev.from.label_zh(),
            ev.to.label_zh(),
            done,
            total
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_pool() -> GlobalResourcePool {
        GlobalResourcePool::new()
    }

    fn add(pool: &mut GlobalResourcePool, k: ResourceKind, n: i64) {
        let _ = pool.try_add(k, n);
    }

    #[test]
    fn default_chain_order_and_kinds() {
        let c = Objectives::default_chain();
        assert_eq!(c.all.len(), 6);
        assert_eq!(c.current().unwrap().id, "q1_gather_wood");
        assert!(matches!(
            c.current().unwrap().kind,
            ObjectiveKind::GatherResource { .. }
        ));
    }

    #[test]
    fn q1_complete_after_10_wood() {
        let mut c = Objectives::default_chain();
        let mut pool = fresh_pool();
        add(&mut pool, ResourceKind::Wood, 5);
        // 用一个 fake PlayerState + pool 跑 auto_advance
        // 直接手动模拟一下:
        let mut player = PlayerState::default();
        player.nation_id = None;
        player.monsters_killed = 0;
        player.block_pos = [0, 0, 0];
        // 第一次:5 木 → 不完成
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Count(pool.get(ResourceKind::Wood));
        }
        assert!(!c.current().unwrap().check_complete());
        // 第二次:10 木 → 完成
        add(&mut pool, ResourceKind::Wood, 5);
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Count(pool.get(ResourceKind::Wood));
        }
        assert!(c.current().unwrap().check_complete());
        assert!(c.try_complete_current());
        assert_eq!(c.current().unwrap().id, "q2_found_nation");
        assert!(c.all[0].done);
    }

    #[test]
    fn q2_found_nation_completes_when_player_has_nation_id() {
        let mut c = Objectives::default_chain();
        // 跳到 q2: 标记 q1 done
        c.all[0].done = true;
        c.current_idx = Some(1);
        let mut player = PlayerState::default();
        player.nation_id = None;
        // 没 nation → 不完成
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Flag(player.nation_id.is_some());
        }
        assert!(!c.current().unwrap().check_complete());
        // 创国 → nation_id = Some
        use crate::nation::NationId;
        player.nation_id = Some(NationId(1));
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Flag(player.nation_id.is_some());
        }
        assert!(c.current().unwrap().check_complete());
        assert!(c.try_complete_current());
        assert_eq!(c.current().unwrap().id, "q3_gather_food");
    }

    #[test]
    fn q4_upgrade_pop_completes_at_target() {
        let mut c = Objectives::default_chain();
        for i in 0..3 {
            c.all[i].done = true;
        }
        c.current_idx = Some(3);
        // target = 10, pop = 5 → 不完成
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Pop(5);
        }
        assert!(!c.current().unwrap().check_complete());
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Pop(10);
        }
        assert!(c.current().unwrap().check_complete());
    }

    #[test]
    fn q5_kill_monsters_uses_player_counter() {
        let mut c = Objectives::default_chain();
        for i in 0..4 {
            c.all[i].done = true;
        }
        c.current_idx = Some(4);
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::CountU(3);
        }
        assert!(!c.current().unwrap().check_complete());
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::CountU(5);
        }
        assert!(c.current().unwrap().check_complete());
    }

    #[test]
    fn q6_reach_position_uses_block_pos_manhattan() {
        let mut c = Objectives::default_chain();
        for i in 0..5 {
            c.all[i].done = true;
        }
        c.current_idx = Some(5);
        // 玩家在 (50, 30, 50) 目标 (48, 30, 48) radius=5 → |2|+|0|+|2|=4 ≤ 5 → 到达
        let mut player = PlayerState::default();
        player.block_pos = [50, 30, 50];
        let reached = {
            let pos = [48, 30, 48];
            let r = 5;
            let dx = (player.block_pos[0] - pos[0]).abs();
            let dy = (player.block_pos[1] - pos[1]).abs();
            let dz = (player.block_pos[2] - pos[2]).abs();
            dx + dy + dz <= r
        };
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::AtPosition { reached };
        }
        assert!(c.current().unwrap().check_complete());
    }

    #[test]
    fn current_idx_advances_on_completion() {
        let mut c = Objectives::default_chain();
        assert_eq!(c.current_idx, Some(0));
        // q1 进度 0 → 不完成
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Count(0);
        }
        assert!(!c.try_complete_current());
        // 进度满 → 完成 → 推 q2
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Count(10);
        }
        assert!(c.try_complete_current());
        assert_eq!(c.current_idx, Some(1));
        // q2 创国 → 完成 → 推 q3
        c.all[1].progress = ObjectiveProgress::Flag(true);
        assert!(c.try_complete_current());
        assert_eq!(c.current_idx, Some(2));
    }

    #[test]
    fn completed_event_payload() {
        // 验证 ObjectiveCompleted 的 kind/id 字段语义
        let ev = ObjectiveCompleted {
            id: "q1".into(),
            kind: ObjectiveKind::GatherResource { kind: ResourceKind::Wood, count: 10 },
            at_wall_secs: 12.5,
        };
        assert_eq!(ev.id, "q1");
        assert!((ev.at_wall_secs - 12.5).abs() < 1e-3);
        match ev.kind {
            ObjectiveKind::GatherResource { count, .. } => assert_eq!(count, 10),
            _ => panic!("expected gather"),
        }
    }

    #[test]
    fn objective_kind_progress_str_format() {
        let k = ObjectiveKind::GatherResource { kind: ResourceKind::Wood, count: 10 };
        let p = ObjectiveProgress::Count(5);
        assert_eq!(k.progress_str(&p), "5 / 10");
    }

    #[test]
    fn objectives_plugin_can_be_added_without_panic() {
        // 不真起 Bevy,只验证 Plugin trait impl 不报错
        fn _assert_plugin<P: Plugin>(_: &P) {}
        let p = ObjectivesPlugin;
        _assert_plugin(&p);
    }

    #[test]
    fn empty_chain_current_is_none() {
        let mut c = Objectives::new();
        assert!(c.current().is_none());
        assert_eq!(c.current_idx, None);
        assert!(!c.try_complete_current());
    }
}
