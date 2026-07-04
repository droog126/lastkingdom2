use bevy::prelude::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::monster::MonsterEcosystem;
use crate::nation::NationRegistry;
use crate::resource::GlobalResourcePool;
use crate::world::World;

#[derive(Debug, Clone)]
pub struct TickSnapshot {
    pub tick: u64,

    pub pool_totals: Vec<(String, i64)>,

    pub flag_count: u32,

    pub monster_count: u32,

    pub player_pos: Option<[i32; 3]>,

    pub wall_time: Instant,
}

impl TickSnapshot {
    pub fn from_world(
        tick: u64,
        _world: &World,
        pool: &GlobalResourcePool,
        nations: &NationRegistry,
        monsters: &MonsterEcosystem,
        player_pos: Option<[i32; 3]>,
    ) -> Self {
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

#[derive(Debug, Clone)]
pub struct AiDecision {
    pub tick: u64,

    pub agent_id: u32,

    pub kind: AiDecisionKind,

    pub context_digest: u64,

    pub result: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiDecisionKind {
    MonsterMove,

    NestDormancy,

    NestDecay,

    MonsterKilled,

    PlayerInput,

    NationFounded,

    NationDissolved,

    PlayerGather,

    ResourceRegen,

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

#[derive(Debug, Clone)]
pub struct Invariant {
    pub name: String,
    pub kind: InvariantKind,
    pub last_violation_tick: Option<u64>,
    pub total_violations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvariantKind {
    ResourceConservation,

    MonsterCountConsistency,

    FlagCountCap,

    PlayerInBounds,

    TickDurationBounded,
}

impl InvariantKind {
    pub const fn label_zh(self) -> &'static str {
        match self {
            InvariantKind::ResourceConservation => "资源守恒",
            InvariantKind::MonsterCountConsistency => "怪物计数一致",
            InvariantKind::FlagCountCap => "国旗上限",
            InvariantKind::PlayerInBounds => "玩家在世界内",
            InvariantKind::TickDurationBounded => "Tick 时长 <= 50ms",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Anomaly {
    pub tick: u64,
    pub kind: AnomalyKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnomalyKind {
    Oscillation,

    TickSpike,

    ResourceJump,

    StructuralChange,

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

#[derive(Resource)]
pub struct TickObserver {
    pub snapshots: Vec<TickSnapshot>,
    pub decisions: Vec<AiDecision>,
    pub invariants: HashMap<InvariantKind, Invariant>,
    pub anomalies: Vec<Anomaly>,

    max_snapshots: usize,

    tick_start: Option<Instant>,

    last_tick_end: Option<Instant>,

    tick_durations: Vec<Duration>,

    agent_decision_history: HashMap<u32, Vec<(u64, AiDecisionKind)>>,

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

    pub fn begin_tick(&mut self) {
        self.tick_start = Some(Instant::now());
    }

    pub fn observe_ai_decision(&mut self, dec: AiDecision) {
        let history = self.agent_decision_history.entry(dec.agent_id).or_insert_with(Vec::new);
        history.push((dec.tick, dec.kind));
        if history.len() > 10 {
            history.remove(0);
        }

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

    pub fn end_tick(
        &mut self,
        tick: u64,
        world: &World,
        pool: &GlobalResourcePool,
        nations: &NationRegistry,
        monsters: &MonsterEcosystem,
        player_pos: Option<[i32; 3]>,
    ) -> Result<(), Vec<String>> {
        let dur = self.tick_start.map(|s| s.elapsed()).unwrap_or_default();
        self.tick_durations.push(dur);
        if self.tick_durations.len() > 100 {
            self.tick_durations.remove(0);
        }
        self.tick_start = None;
        self.last_tick_end = Some(Instant::now());

        let snap = TickSnapshot::from_world(tick, world, pool, nations, monsters, player_pos);

        let new_digest = snap.digest();
        if let Some(prev) = self.last_snapshot_digest {
            if new_digest != prev {}
        }
        self.last_snapshot_digest = Some(new_digest);
        if self.snapshots.len() < self.max_snapshots {
            self.snapshots.push(snap);
        }

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
                "[怪物计数 @ tick {}] current={} != sum-of-nests={}",
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
        if dur > Duration::from_millis(50) {
            self.anomalies.push(Anomaly {
                tick,
                kind: AnomalyKind::TickSpike,
                detail: format!("dur={:?} > 50ms", dur),
            });
            return;
        }
        let inv = self.get_or_register(InvariantKind::TickDurationBounded);
        if dur > Duration::from_millis(50) {
            inv.last_violation_tick = Some(tick);
            inv.total_violations += 1;
            errors.push(format!("[Tick 卡顿 @ tick {}] dur={:?} > 50ms", tick, dur));
        }
    }

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

    pub fn snapshot_at(&self, tick: u64) -> Option<&TickSnapshot> {
        self.snapshots.iter().find(|s| s.tick == tick)
    }

    pub fn replay_script(&self) -> Vec<&AiDecision> {
        let mut v: Vec<&AiDecision> = self.decisions.iter().collect();
        v.sort_by_key(|d| d.tick);
        v
    }
}

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
        let (world, mut pool, nations, mut monsters) = fresh_world();
        let mut obs = TickObserver::new();
        for tick in 0..10 {
            obs.begin_tick();

            monsters.tick(&mut pool);
            obs.end_tick(tick, &world, &pool, &nations, &monsters, Some([8, 8, 8]))
                .expect("clean state should not violate invariants");
        }
    }

    #[test]
    fn end_tick_detects_overdrawn_pool() {
        let (world, mut pool, nations, monsters) = fresh_world();

        pool.force_add(ResourceKind::Wood, 1000);
        pool.try_sub(ResourceKind::Wood, 100).unwrap();

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
        let (world, pool, mut nations, monsters) = fresh_world();

        nations.flag_count = crate::constant::MAX_NATIONAL_FLAGS + 1;
        let mut obs = TickObserver::new();
        obs.begin_tick();
        let err = obs.end_tick(0, &world, &pool, &nations, &monsters, Some([8, 8, 8])).unwrap_err();
        assert!(err.iter().any(|e| e.contains("国旗上限")));
    }

    #[test]
    fn end_tick_detects_player_out_of_bounds() {
        let (world, pool, nations, monsters) = fresh_world();
        let mut obs = TickObserver::new();
        obs.begin_tick();
        let err =
            obs.end_tick(0, &world, &pool, &nations, &monsters, Some([100, 100, 100])).unwrap_err();
        assert!(err.iter().any(|e| e.contains("玩家出界")));
    }

    #[test]
    fn end_tick_detects_monster_count_mismatch() {
        let (world, pool, nations, mut monsters) = fresh_world();
        monsters.current_individuals += 10;
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
    fn slow_tick_is_anomaly_not_invariant_error() {
        let mut obs = TickObserver::new();
        let mut errors = Vec::new();
        obs.check_tick_duration(Duration::from_millis(75), 12, &mut errors);

        assert!(errors.is_empty(), "slow tick should not fail invariants: {:?}", errors);
        assert_eq!(obs.anomalies.len(), 1);
        assert_eq!(obs.anomalies[0].tick, 12);
        assert_eq!(obs.anomalies[0].kind, AnomalyKind::TickSpike);
        assert!(obs.invariants.get(&InvariantKind::TickDurationBounded).is_none());
    }

    #[test]
    fn report_includes_inv_count_and_anomalies() {
        let (world, pool, mut nations, monsters) = fresh_world();
        let mut obs = TickObserver::new();
        obs.begin_tick();
        obs.end_tick(0, &world, &pool, &nations, &monsters, Some([8, 8, 8])).unwrap();
        obs.begin_tick();
        obs.end_tick(1, &world, &pool, &nations, &monsters, Some([8, 8, 8])).unwrap();

        nations.flag_count = 99;
        obs.begin_tick();
        let _ = obs.end_tick(2, &world, &pool, &nations, &monsters, Some([8, 8, 8]));
        let r = obs.report();
        assert!(r.contains("国旗上限"));
        assert!(r.contains("TickObserver 报告"));
    }
}
