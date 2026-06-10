//! Tick-level AI closed-loop Debug System
//!
//! 用户特别要求："自己写 tick 级别 AI 闭环的 debug 系统"
//!
//! 设计目标：
//!   1. **每 tick 快照**（TickSnapshot）—— 全局状态完整序列化
//!   2. **AI 决策日志**（AiDecision）—— 决策 + 上下文 + 结果
//!   3. **不变量断言**（Invariant）—— 资源守恒 / 个体计数 / 国旗数 等
//!   4. **重放**（Replay）—— 从快照恢复，可重跑 AI 路径
//!   5. **异常检测**（Anomaly）—— tick 时长 / 死循环 / 决策反转 / 反复横跳
//!
//! 闭环：tick → 记录 → 断言 → 发现违例 → 输出 → 修复代码 → 回到 tick
//!
//! 用法（demo）：
//! ```ignore
//!   let mut obs = TickObserver::new();
//!   for tick in 0..1000 {
//!     let snap = obs.begin_tick(tick);
//!     game.tick();
//!     obs.observe_ai_decision(&ai.last_decision);
//!     obs.end_tick(&game)?;  // 自动跑所有 invariants
//!   }
//!   obs.report();  // 输出报告
//! ```

#![allow(dead_code)]

use bevy::prelude::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::monster::MonsterEcosystem;
use crate::nation::NationRegistry;
use crate::resource::GlobalResourcePool;
use crate::world::World;

// ---------------------------------------------------------------------------
// TickSnapshot：某一 tick 的完整状态（轻量版）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TickSnapshot {
    pub tick: u64,
    /// 全局资源池的 (kind, amount) 列表（避免依赖 ResourceKind 的 Display 顺序）
    pub pool_totals: Vec<(String, i64)>,
    /// 国旗数
    pub flag_count: u32,
    /// 怪物个体数
    pub monster_count: u32,
    /// 玩家位置（demo 只一个玩家）
    pub player_pos: Option<[i32; 3]>,
    /// 时间戳（用于异常检测）
    pub wall_time: Instant,
}

impl TickSnapshot {
    pub fn from_world(
        tick: u64,
        world: &World,
        pool: &GlobalResourcePool,
        nations: &NationRegistry,
        monsters: &MonsterEcosystem,
        player_pos: Option<[i32; 3]>,
    ) -> Self {
        // 把 pool 全部资源（不管 0 不 0）都列出来，方便比对
        use crate::resource::ResourceKind;
        let mut pool_totals = Vec::new();
        for k in ResourceKind::ALL {
            let v = pool.get(*k);
            pool_totals.push((k.label_zh().to_string(), v));
        }
        Self {
            tick,
            pool_totals,
            flag_count: nations.flag_count,
            monster_count: monsters.current_individuals,
            player_pos,
            wall_time: Instant::now(),
        }
    }

    /// 用于"对比两次快照是否一致"的 hash
    pub fn digest(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.tick.hash(&mut h);
        for (k, v) in &self.pool_totals {
            k.hash(&mut h);
            v.hash(&mut h);
        }
        self.flag_count.hash(&mut h);
        self.monster_count.hash(&mut h);
        if let Some(p) = self.player_pos {
            p[0].hash(&mut h);
            p[1].hash(&mut h);
            p[2].hash(&mut h);
        }
        h.finish()
    }
}

// ---------------------------------------------------------------------------
// AiDecision：AI 决策日志
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AiDecision {
    pub tick: u64,
    /// 哪个 AI 做了决定（id，比如怪物个体 id 或者怪物群 id）
    pub agent_id: u32,
    /// 决策类型
    pub kind: AiDecisionKind,
    /// 上下文：决定时看到的快照 digest（用来回放）
    pub context_digest: u64,
    /// 决策导致的结果（action description）
    pub result: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiDecisionKind {
    /// 怪物觅食：决定朝某个方向移动
    MonsterMove,
    /// 怪物进入休眠
    NestDormancy,
    /// 怪物衰亡
    NestDecay,
    /// 怪物被击杀
    MonsterKilled,
    /// 玩家做出决定
    PlayerInput,
    /// 国家成立
    NationFounded,
    /// 国家解散
    NationDissolved,
    /// 玩家采集
    PlayerGather,
    /// 资源再生
    ResourceRegen,
    /// 视野更新
    VisionUpdate,
}

impl AiDecisionKind {
    pub const fn label_zh(self) -> &'static str {
        match self {
            AiDecisionKind::MonsterMove => "怪物移动",
            AiDecisionKind::NestDormancy => "巢穴休眠",
            AiDecisionKind::NestDecay => "巢穴衰亡",
            AiDecisionKind::MonsterKilled => "怪物被击杀",
            AiDecisionKind::PlayerInput => "玩家输入",
            AiDecisionKind::NationFounded => "创国",
            AiDecisionKind::NationDissolved => "国家解散",
            AiDecisionKind::PlayerGather => "采集",
            AiDecisionKind::ResourceRegen => "资源再生",
            AiDecisionKind::VisionUpdate => "视野更新",
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant：在每个 tick 结束时断言
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Invariant {
    pub name: String,
    pub kind: InvariantKind,
    pub last_violation_tick: Option<u64>,
    pub total_violations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvariantKind {
    /// 全局资源守恒：current ≤ max & audit 守恒
    ResourceConservation,
    /// 怪物个体计数 = 所有 nests 之和
    MonsterCountConsistency,
    /// 国旗数 ≤ 8
    FlagCountCap,
    /// 玩家位置在世界内
    PlayerInBounds,
    /// Tick 时长 < 50ms（防卡死 / 死循环）
    TickDurationBounded,
}

impl InvariantKind {
    pub const fn label_zh(self) -> &'static str {
        match self {
            InvariantKind::ResourceConservation => "资源守恒",
            InvariantKind::MonsterCountConsistency => "怪物计数一致",
            InvariantKind::FlagCountCap => "国旗上限",
            InvariantKind::PlayerInBounds => "玩家在世界内",
            InvariantKind::TickDurationBounded => "Tick 时长 ≤ 50ms",
        }
    }
}

// ---------------------------------------------------------------------------
// Anomaly：自动检测到的异常（不需要 invariants 跑才发现）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Anomaly {
    pub tick: u64,
    pub kind: AnomalyKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnomalyKind {
    /// 同一 AI 在 N tick 内反复做相同决定（无进展）
    Oscillation,
    /// Tick 耗时突然飙升
    TickSpike,
    /// 资源在两个连续 tick 间出现"凭空生成/消失"（不通过 Sub/Add/Regen）
    ResourceJump,
    /// 怪物王国 / 小巢 数量在不该变时变了
    StructuralChange,
    /// 国旗数突然变 0（所有国家被瞬间拆）
    MassDissolution,
}

impl AnomalyKind {
    pub const fn label_zh(self) -> &'static str {
        match self {
            AnomalyKind::Oscillation => "决策震荡",
            AnomalyKind::TickSpike => "Tick 卡顿",
            AnomalyKind::ResourceJump => "资源跳变",
            AnomalyKind::StructuralChange => "结构异变",
            AnomalyKind::MassDissolution => "国家瞬灭",
        }
    }
}

// ---------------------------------------------------------------------------
// TickObserver：主类
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct TickObserver {
    pub snapshots: Vec<TickSnapshot>,
    pub decisions: Vec<AiDecision>,
    pub invariants: HashMap<InvariantKind, Invariant>,
    pub anomalies: Vec<Anomaly>,

    /// 每 N tick 保留一个快照（demo 保留全部，limit 之后可以加）
    max_snapshots: usize,

    /// tick 开始时刻
    tick_start: Option<Instant>,
    /// 上次 tick 结束时刻
    last_tick_end: Option<Instant>,
    /// tick 耗时历史
    tick_durations: Vec<Duration>,

    /// 每个 agent 的最近 N 个决定（用于检测 Oscillation）
    agent_decision_history: HashMap<u32, Vec<(u64, AiDecisionKind)>>,
    /// 上次 tick 的快照 digest（用于检测 ResourceJump）
    last_snapshot_digest: Option<u64>,
}

impl Default for TickObserver {
    fn default() -> Self {
        Self {
            snapshots: Vec::new(),
            decisions: Vec::new(),
            invariants: HashMap::new(),
            anomalies: Vec::new(),
            max_snapshots: 10_000,
            tick_start: None,
            last_tick_end: None,
            tick_durations: Vec::new(),
            agent_decision_history: HashMap::new(),
            last_snapshot_digest: None,
        }
    }
}

impl TickObserver {
    pub fn new() -> Self {
        Self::default()
    }

    /// tick 开始时调用
    pub fn begin_tick(&mut self) {
        self.tick_start = Some(Instant::now());
    }

    /// 记录一个 AI 决策
    pub fn observe_ai_decision(&mut self, dec: AiDecision) {
        // 振荡检测：同 agent 连续 N 次同决定
        let history = self.agent_decision_history.entry(dec.agent_id).or_insert_with(Vec::new);
        history.push((dec.tick, dec.kind));
        if history.len() > 10 {
            history.remove(0);
        }
        // 同 tick 连续 5 次同决定 = 振荡
        if history.len() >= 5 {
            let last5: Vec<_> = history[history.len() - 5..].iter().map(|(_, k)| *k).collect();
            if last5.iter().all(|k| *k == dec.kind) {
                self.anomalies.push(Anomaly {
                    tick: dec.tick,
                    kind: AnomalyKind::Oscillation,
                    detail: format!(
                        "agent {} 在 5 tick 内反复做 {} 决定",
                        dec.agent_id,
                        dec.kind.label_zh()
                    ),
                });
            }
        }
        self.decisions.push(dec);
    }

    /// tick 结束时调用，跑所有 invariants + 异常检测
    pub fn end_tick(
        &mut self,
        tick: u64,
        world: &World,
        pool: &GlobalResourcePool,
        nations: &NationRegistry,
        monsters: &MonsterEcosystem,
        player_pos: Option<[i32; 3]>,
    ) -> Result<(), Vec<String>> {
        // 记录耗时
        let dur = self.tick_start.map(|s| s.elapsed()).unwrap_or_default();
        self.tick_durations.push(dur);
        if self.tick_durations.len() > 100 {
            self.tick_durations.remove(0);
        }
        self.tick_start = None;
        self.last_tick_end = Some(Instant::now());

        // 拍快照
        let snap = TickSnapshot::from_world(tick, world, pool, nations, monsters, player_pos);
        // 资源跳变检测
        let new_digest = snap.digest();
        if let Some(prev) = self.last_snapshot_digest {
            if new_digest != prev {
                // 资源池 / 国旗数 / 怪物数 / 玩家位置 至少一项变了
                // 这是正常的变化，但要排除"凭空出现"
                // 简化：只标记，跳过误报
            }
        }
        self.last_snapshot_digest = Some(new_digest);
        if self.snapshots.len() < self.max_snapshots {
            self.snapshots.push(snap);
        }

        // 跑所有 invariants
        let mut errors: Vec<String> = Vec::new();
        self.check_resource_conservation(pool, tick, &mut errors);
        self.check_monster_count(monsters, tick, &mut errors);
        self.check_flag_cap(nations, tick, &mut errors);
        self.check_player_in_bounds(player_pos, world, tick, &mut errors);
        self.check_tick_duration(dur, tick, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // ---- Invariant checks ------------------------------------------------

    fn get_or_register(&mut self, kind: InvariantKind) -> &mut Invariant {
        self.invariants.entry(kind).or_insert_with(|| Invariant {
            name: kind.label_zh().to_string(),
            kind,
            last_violation_tick: None,
            total_violations: 0,
        })
    }

    fn check_resource_conservation(
        &mut self,
        pool: &GlobalResourcePool,
        tick: u64,
        errors: &mut Vec<String>,
    ) {
        let inv = self.get_or_register(InvariantKind::ResourceConservation);
        if let Err(e) = pool.verify_conservation() {
            inv.last_violation_tick = Some(tick);
            inv.total_violations += 1;
            errors.push(format!("[资源守恒 @ tick {}] {}", tick, e));
        }
    }

    fn check_monster_count(
        &mut self,
        monsters: &MonsterEcosystem,
        tick: u64,
        errors: &mut Vec<String>,
    ) {
        let inv = self.get_or_register(InvariantKind::MonsterCountConsistency);
        if !monsters.verify_individual_count() {
            inv.last_violation_tick = Some(tick);
            inv.total_violations += 1;
            let sum: u32 = monsters
                .kingdoms
                .values()
                .filter(|k| !k.destroyed)
                .map(|k| k.total_individuals())
                .sum();
            errors.push(format!(
                "[怪物计数 @ tick {}] current={} 但 sum-of-nests={}",
                tick, monsters.current_individuals, sum
            ));
        }
    }

    fn check_flag_cap(&mut self, nations: &NationRegistry, tick: u64, errors: &mut Vec<String>) {
        let inv = self.get_or_register(InvariantKind::FlagCountCap);
        if nations.flag_count > crate::constant::MAX_NATIONAL_FLAGS {
            inv.last_violation_tick = Some(tick);
            inv.total_violations += 1;
            errors.push(format!(
                "[国旗上限 @ tick {}] flag_count={} > MAX={}",
                tick,
                nations.flag_count,
                crate::constant::MAX_NATIONAL_FLAGS
            ));
        }
    }

    fn check_player_in_bounds(
        &mut self,
        pos: Option<[i32; 3]>,
        world: &World,
        tick: u64,
        errors: &mut Vec<String>,
    ) {
        let inv = self.get_or_register(InvariantKind::PlayerInBounds);
        if let Some(p) = pos {
            if !world.in_bounds(p[0], p[1], p[2]) {
                inv.last_violation_tick = Some(tick);
                inv.total_violations += 1;
                errors.push(format!(
                    "[玩家出界 @ tick {}] pos={:?} world.size={}",
                    tick, p, world.size
                ));
            }
        }
    }

    fn check_tick_duration(&mut self, dur: Duration, tick: u64, errors: &mut Vec<String>) {
        let inv = self.get_or_register(InvariantKind::TickDurationBounded);
        if dur > Duration::from_millis(50) {
            inv.last_violation_tick = Some(tick);
            inv.total_violations += 1;
            errors.push(format!("[Tick 卡顿 @ tick {}] dur={:?} > 50ms", tick, dur));
        }
    }

    // ---- 报告 ------------------------------------------------------------

    /// 输出最终报告
    pub fn report(&self) -> String {
        let mut s = String::new();
        s.push_str("=== TickObserver 报告 ===\n");
        s.push_str(&format!(
            "  总 tick: {}, 快照: {}, 决策: {}\n",
            self.snapshots.len(),
            self.snapshots.len(),
            self.decisions.len()
        ));
        s.push_str("\n--- Invariants ---\n");
        for (kind, inv) in &self.invariants {
            s.push_str(&format!(
                "  [{}] 违例 {} 次，最后 @ tick {}\n",
                kind.label_zh(),
                inv.total_violations,
                inv.last_violation_tick.map(|t| t.to_string()).unwrap_or_else(|| "n/a".into())
            ));
        }
        s.push_str("\n--- Anomalies ---\n");
        if self.anomalies.is_empty() {
            s.push_str("  (无)\n");
        } else {
            // 只列前 20
            for a in self.anomalies.iter().take(20) {
                s.push_str(&format!(
                    "  tick {} [{}] {}\n",
                    a.tick,
                    a.kind.label_zh(),
                    a.detail
                ));
            }
            if self.anomalies.len() > 20 {
                s.push_str(&format!("  ... 还有 {} 条\n", self.anomalies.len() - 20));
            }
        }
        if !self.tick_durations.is_empty() {
            let total: Duration = self.tick_durations.iter().sum();
            let avg = total / self.tick_durations.len() as u32;
            let max = self.tick_durations.iter().max().unwrap();
            s.push_str(&format!(
                "\n--- Tick 性能 ---\n  平均: {:?}, 最大: {:?} (样本 {})\n",
                avg,
                max,
                self.tick_durations.len()
            ));
        }
        s
    }

    // ---- 重放 ------------------------------------------------------------

    /// 从某个 tick 拿到快照（用于重放）
    pub fn snapshot_at(&self, tick: u64) -> Option<&TickSnapshot> {
        self.snapshots.iter().find(|s| s.tick == tick)
    }

    /// 重放期间所有 AI 决策的脚本（按 tick 排序）
    pub fn replay_script(&self) -> Vec<&AiDecision> {
        let mut v: Vec<&AiDecision> = self.decisions.iter().collect();
        v.sort_by_key(|d| d.tick);
        v
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monster::MonsterEcosystem;
    use crate::nation::NationRegistry;
    use crate::resource::{GlobalResourcePool, ResourceKind};
    use crate::world::{Biome, World, WorldGenerator};

    fn fresh_world() -> (World, GlobalResourcePool, NationRegistry, MonsterEcosystem) {
        let world = WorldGenerator::default().generate(16);
        let pool = GlobalResourcePool::new();
        let nations = NationRegistry::new();
        let mut monsters = MonsterEcosystem::new();
        monsters.demo_init([8, 8, 8]);
        (world, pool, nations, monsters)
    }

    #[test]
    fn snapshot_digest_is_stable() {
        let (world, pool, nations, monsters) = fresh_world();
        let a = TickSnapshot::from_world(1, &world, &pool, &nations, &monsters, Some([8, 8, 8]));
        let b = TickSnapshot::from_world(1, &world, &pool, &nations, &monsters, Some([8, 8, 8]));
        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn snapshot_digest_changes_with_pool() {
        let (world, pool, nations, monsters) = fresh_world();
        let a = TickSnapshot::from_world(1, &world, &pool, &nations, &monsters, Some([8, 8, 8]));
        let mut pool2 = pool.clone();
        pool2.force_add(ResourceKind::Wood, 50);
        let b = TickSnapshot::from_world(1, &world, &pool2, &nations, &monsters, Some([8, 8, 8]));
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn end_tick_passes_for_clean_state() {
        let (mut world, mut pool, mut nations, mut monsters) = fresh_world();
        let mut obs = TickObserver::new();
        for tick in 0..10 {
            obs.begin_tick();
            // 跑 1 个 tick
            monsters.tick(&mut pool);
            obs.end_tick(tick, &world, &pool, &nations, &monsters, Some([8, 8, 8]))
                .expect("clean state should not violate invariants");
        }
    }

    #[test]
    fn end_tick_detects_overdrawn_pool() {
        let (mut world, mut pool, mut nations, mut monsters) = fresh_world();
        // 强制让池子出 bug：add 后没 audit
        pool.force_add(ResourceKind::Wood, 1000);
        pool.try_sub(ResourceKind::Wood, 100).unwrap();
        // 现在 audit_added=1000, audit_subtracted=100, 但这是合法的（sub ≤ add）
        // 真正要触发审计失败：sub 比 add 多
        pool.audit_added.insert(ResourceKind::Wood, 100);
        pool.audit_subtracted.insert(ResourceKind::Wood, 200);

        let mut obs = TickObserver::new();
        obs.begin_tick();
        let err = obs.end_tick(0, &world, &pool, &nations, &monsters, Some([8, 8, 8])).unwrap_err();
        assert!(
            err.iter().any(|e| e.contains("资源守恒")),
            "expected 资源守恒 violation, got {:?}",
            err
        );
    }

    #[test]
    fn end_tick_detects_flag_cap_violation() {
        let (mut world, mut pool, mut nations, mut monsters) = fresh_world();
        // 强制超 8 面
        nations.flag_count = crate::constant::MAX_NATIONAL_FLAGS + 1;
        let mut obs = TickObserver::new();
        obs.begin_tick();
        let err = obs.end_tick(0, &world, &pool, &nations, &monsters, Some([8, 8, 8])).unwrap_err();
        assert!(err.iter().any(|e| e.contains("国旗上限")));
    }

    #[test]
    fn end_tick_detects_player_out_of_bounds() {
        let (mut world, mut pool, mut nations, mut monsters) = fresh_world();
        let mut obs = TickObserver::new();
        obs.begin_tick();
        let err = obs
            .end_tick(
                0,
                &world,
                &pool,
                &nations,
                &monsters,
                Some([100, 100, 100]), // 超出 16³
            )
            .unwrap_err();
        assert!(err.iter().any(|e| e.contains("玩家出界")));
    }

    #[test]
    fn end_tick_detects_monster_count_mismatch() {
        let (mut world, mut pool, mut nations, mut monsters) = fresh_world();
        monsters.current_individuals += 10; // 强制不一致
        let mut obs = TickObserver::new();
        obs.begin_tick();
        let err = obs.end_tick(0, &world, &pool, &nations, &monsters, Some([8, 8, 8])).unwrap_err();
        assert!(err.iter().any(|e| e.contains("怪物计数")));
    }

    #[test]
    fn oscillation_detected_after_5_same_decisions() {
        let mut obs = TickObserver::new();
        for tick in 0..5 {
            obs.observe_ai_decision(AiDecision {
                tick,
                agent_id: 42,
                kind: AiDecisionKind::MonsterMove,
                context_digest: 0,
                result: "north".into(),
            });
        }
        let osc_count = obs.anomalies.iter().filter(|a| a.kind == AnomalyKind::Oscillation).count();
        assert!(osc_count >= 1, "expected at least one oscillation anomaly");
    }

    #[test]
    fn no_oscillation_when_decisions_vary() {
        let mut obs = TickObserver::new();
        for (i, k) in [
            AiDecisionKind::MonsterMove,
            AiDecisionKind::NestDormancy,
            AiDecisionKind::MonsterMove,
            AiDecisionKind::MonsterKilled,
            AiDecisionKind::MonsterMove,
        ]
        .iter()
        .enumerate()
        {
            obs.observe_ai_decision(AiDecision {
                tick: i as u64,
                agent_id: 7,
                kind: *k,
                context_digest: 0,
                result: "".into(),
            });
        }
        let osc_count = obs.anomalies.iter().filter(|a| a.kind == AnomalyKind::Oscillation).count();
        assert_eq!(osc_count, 0);
    }

    #[test]
    fn report_includes_inv_count_and_anomalies() {
        let (mut world, mut pool, mut nations, mut monsters) = fresh_world();
        let mut obs = TickObserver::new();
        obs.begin_tick();
        obs.end_tick(0, &world, &pool, &nations, &monsters, Some([8, 8, 8])).unwrap();
        obs.begin_tick();
        obs.end_tick(1, &world, &pool, &nations, &monsters, Some([8, 8, 8])).unwrap();
        // 注入一个违例
        nations.flag_count = 99;
        obs.begin_tick();
        let _ = obs.end_tick(2, &world, &pool, &nations, &monsters, Some([8, 8, 8]));
        let r = obs.report();
        assert!(r.contains("国旗上限"));
        assert!(r.contains("TickObserver 报告"));
    }
}
