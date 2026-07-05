use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fmt;

use crate::clock::SimClock;
use crate::constant::*;
use crate::resource::{
    GlobalResourcePool, PoolError, ResourceKind, Transfer, TransferDst, TransferSrc,
};

// =============================================================================
// 自动 upkeep（资源循环闭环）
//
// 每个 nation 每 NATION_UPKEEP_INTERVAL_TICKS tick 自动消耗 Wood + Food 维持 flag，
// 缺资源时 flag_hp 扣 NATION_UPKEEP_MISS_HP_LOSS，flag_hp 归零就 dissolve。
//
// 设计：放在 server 的 FixedUpdate chain (after `simulation_tick`)，
// `simulation_tick` 已经会 try_add(Apple, +1) / try_add(Food, +2)，
// nation 消耗 Wood/Food 让 pool 有真"流出"，配合玩家挖矿 / 战斗产 Wood/Soul，
// 形成 Wood/Soul 流向 nation、Food 从 sim tick 流回 nation 的资源循环。
// =============================================================================
pub const NATION_UPKEEP_WOOD_PER_TICK: i64 = 1;
pub const NATION_UPKEEP_FOOD_PER_TICK: i64 = 1;
pub const NATION_UPKEEP_INTERVAL_TICKS: u64 = 30;
pub const NATION_UPKEEP_MISS_HP_LOSS: u32 = 5;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpkeepReport {
    pub checked: usize,
    pub kept: usize,
    pub missed: usize,
    pub dissolved: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NationId(pub u32);

impl fmt::Display for NationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Nation#{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Nation {
    pub id: NationId,
    pub name: String,

    pub king: u32,

    pub members: BTreeSet<u32>,

    pub pop_cap: u32,

    pub flag_pos: [i32; 3],

    pub flag_hp: u32,

    pub flag_hp_max: u32,

    pub war_exhaustion: i32,

    pub created_at_tick: u64,

    pub founding_order: u32,
}

impl Nation {
    pub fn new(
        id: NationId,
        name: String,
        king: u32,
        flag_pos: [i32; 3],
        tick: u64,
        order: u32,
    ) -> Self {
        let mut members = BTreeSet::new();
        members.insert(king);
        Self {
            id,
            name,
            king,
            members,
            pop_cap: INITIAL_POP_CAP,
            flag_pos,
            flag_hp: FLAG_HP,
            flag_hp_max: FLAG_HP,
            war_exhaustion: 0,
            created_at_tick: tick,
            founding_order: order,
        }
    }

    pub fn is_member(&self, player_id: u32) -> bool {
        self.members.contains(&player_id)
    }

    pub fn size(&self) -> usize {
        self.members.len()
    }

    pub fn can_upgrade_to_10(&self, pool: &GlobalResourcePool) -> bool {
        pool.get(ResourceKind::Wood) >= POP_UPGRADE_10_COST.0 as i64
            && pool.get(ResourceKind::Food) >= POP_UPGRADE_10_COST.1 as i64
            && pool.get(ResourceKind::Soul) >= POP_UPGRADE_10_COST.2 as i64
    }

    pub fn pop_upgrade_cost(target: u32) -> (u64, u64, u64) {
        match target {
            10 => POP_UPGRADE_10_COST,
            15 => POP_UPGRADE_15_COST,
            20 => POP_UPGRADE_20_COST,
            _ => panic!("no upgrade path to {} population", target),
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct NationRegistry {
    pub nations: HashMap<NationId, Nation>,

    next_id: u32,

    pub flag_count: u32,

    pub flag_orders_taken: BTreeSet<u32>,
}

impl NationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.nations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nations.is_empty()
    }

    pub fn can_found_new(&self) -> bool {
        self.flag_count < MAX_NATIONAL_FLAGS
    }

    pub fn next_flag_cost(&self) -> i64 {
        let i = self.flag_count as usize;
        if i >= FLAG_COSTS_SOULS.len() {
            FLAG_COSTS_SOULS[FLAG_COSTS_SOULS.len() - 1] as i64
        } else {
            FLAG_COSTS_SOULS[i] as i64
        }
    }

    pub fn found(
        &mut self,
        pool: &mut GlobalResourcePool,
        player_id: u32,
        name: String,
        flag_pos: [i32; 3],
        tick: u64,
    ) -> Result<NationId, FoundError> {
        if !self.can_found_new() {
            return Err(FoundError::MaxFlagsReached(self.flag_count));
        }
        if self.find_nation_by_player(player_id).is_some() {
            return Err(FoundError::AlreadyInNation);
        }
        let cost = self.next_flag_cost();
        if pool.get(ResourceKind::Soul) < cost {
            return Err(FoundError::InsufficientSouls {
                have: pool.get(ResourceKind::Soul),
                need: cost,
            });
        }

        let _t = Transfer {
            kind: ResourceKind::Soul,
            amount: cost,
            src: TransferSrc::PlayerGather(player_id),
            dst: TransferDst::Wasted,
        };

        pool.try_sub(ResourceKind::Soul, cost).map_err(|e| FoundError::PoolError(e))?;

        let id = NationId(self.next_id);
        self.next_id += 1;

        let order = (self.flag_count + 1) as u32;
        self.flag_orders_taken.insert(order);

        let nation = Nation::new(id, name, player_id, flag_pos, tick, order);
        self.nations.insert(id, nation);
        self.flag_count += 1;

        Ok(id)
    }

    pub fn join(&mut self, nation_id: NationId, player_id: u32) -> Result<(), JoinError> {
        if self.find_nation_by_player(player_id).is_some() {
            return Err(JoinError::AlreadyInNation);
        }
        let n = self.nations.get_mut(&nation_id).ok_or(JoinError::NoSuchNation)?;
        if n.size() as u32 >= n.pop_cap {
            return Err(JoinError::PopulationFull { current: n.size() as u32, cap: n.pop_cap });
        }
        n.members.insert(player_id);
        Ok(())
    }

    pub fn leave(&mut self, player_id: u32) -> Result<NationId, LeaveError> {
        let id = self.find_nation_by_player(player_id).ok_or(LeaveError::NotInNation)?;
        let n = self.nations.get_mut(&id).unwrap();
        if player_id == n.king {
            n.flag_hp = 0;
        } else {
            n.members.remove(&player_id);
        }
        Ok(id)
    }

    pub fn find_nation_by_player(&self, player_id: u32) -> Option<NationId> {
        self.nations.values().find(|n| n.is_member(player_id)).map(|n| n.id)
    }

    pub fn damage_flag(&mut self, nation_id: NationId, dmg: u32) -> u32 {
        if let Some(n) = self.nations.get_mut(&nation_id) {
            n.flag_hp = n.flag_hp.saturating_sub(dmg);
            if n.flag_hp == 0 {
                self.dissolve(nation_id);
                return 0;
            }
            n.flag_hp
        } else {
            0
        }
    }

    fn dissolve(&mut self, nation_id: NationId) {
        if let Some(n) = self.nations.get(&nation_id) {
            self.flag_orders_taken.remove(&n.founding_order);
        }

        self.nations.remove(&nation_id);

        self.flag_count = self.flag_count.saturating_sub(1);
    }

    pub fn upgrade_population(
        &mut self,
        pool: &mut GlobalResourcePool,
        nation_id: NationId,
        target: u32,
    ) -> Result<(), UpgradeError> {
        let n = self.nations.get_mut(&nation_id).ok_or(UpgradeError::NoSuchNation)?;
        let current_cap = n.pop_cap;
        if target <= current_cap {
            return Err(UpgradeError::AlreadyAtOrAbove { current: current_cap, target });
        }
        if !matches!(target, 10 | 15 | 20) {
            return Err(UpgradeError::InvalidTarget(target));
        }
        let (wood, food, soul) = Nation::pop_upgrade_cost(target);

        if pool.get(ResourceKind::Wood) < wood as i64
            || pool.get(ResourceKind::Food) < food as i64
            || pool.get(ResourceKind::Soul) < soul as i64
        {
            return Err(UpgradeError::InsufficientResources);
        }

        if wood > 0 {
            pool.try_sub(ResourceKind::Wood, wood as i64)
                .map_err(|e: PoolError| UpgradeError::PoolError(e))?;
        }
        if food > 0 {
            pool.try_sub(ResourceKind::Food, food as i64)
                .map_err(|e: PoolError| UpgradeError::PoolError(e))?;
        }
        if soul > 0 {
            pool.try_sub(ResourceKind::Soul, soul as i64)
                .map_err(|e: PoolError| UpgradeError::PoolError(e))?;
        }

        n.pop_cap = target;
        Ok(())
    }

    /// 自动 upkeep：每 `NATION_UPKEEP_INTERVAL_TICKS` tick 对每个 nation 扣
    /// `NATION_UPKEEP_WOOD_PER_TICK` Wood + `NATION_UPKEEP_FOOD_PER_TICK` Food。
    /// 池不够 → flag_hp 扣 `NATION_UPKEEP_MISS_HP_LOSS`，归零就 dissolve。
    ///
    /// 返回 [`UpkeepReport`]（不写日志，避免每 tick spam；日志由调用方按需 throttle）。
    pub fn tick_upkeep(
        &mut self,
        pool: &mut GlobalResourcePool,
        current_tick: u64,
    ) -> UpkeepReport {
        let mut report = UpkeepReport::default();
        if current_tick % NATION_UPKEEP_INTERVAL_TICKS != 0 {
            return report;
        }
        let ids: Vec<NationId> = self.nations.keys().copied().collect();
        let mut to_dissolve: Vec<NationId> = Vec::new();
        for id in ids {
            report.checked += 1;
            let have_wood = pool.get(ResourceKind::Wood) >= NATION_UPKEEP_WOOD_PER_TICK;
            let have_food = pool.get(ResourceKind::Food) >= NATION_UPKEEP_FOOD_PER_TICK;
            if have_wood && have_food {
                let _ = pool.try_sub(ResourceKind::Wood, NATION_UPKEEP_WOOD_PER_TICK);
                let _ = pool.try_sub(ResourceKind::Food, NATION_UPKEEP_FOOD_PER_TICK);
                report.kept += 1;
            } else if let Some(n) = self.nations.get_mut(&id) {
                n.flag_hp = n.flag_hp.saturating_sub(NATION_UPKEEP_MISS_HP_LOSS);
                report.missed += 1;
                if n.flag_hp == 0 {
                    to_dissolve.push(id);
                }
            }
        }
        for id in to_dissolve {
            self.dissolve(id);
            report.dissolved += 1;
        }
        report
    }
}

/// Bevy system 包装 — 在 server 的 FixedUpdate 调一次，自动 throttle 日志。
pub fn tick_nations_upkeep_system(
    mut pool: ResMut<GlobalResourcePool>,
    mut registry: ResMut<NationRegistry>,
    clock: Res<SimClock>,
) {
    let report = registry.tick_upkeep(&mut pool, clock.tick);
    if report.checked > 0 && clock.tick % (NATION_UPKEEP_INTERVAL_TICKS * 10) == 0 {
        info!(
            "[nation-upkeep] tick={} checked={} kept={} missed={} dissolved={}",
            clock.tick, report.checked, report.kept, report.missed, report.dissolved
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoundError {
    MaxFlagsReached(u32),
    AlreadyInNation,
    InsufficientSouls { have: i64, need: i64 },
    PoolError(PoolError),
}

impl fmt::Display for FoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FoundError::MaxFlagsReached(n) => write!(f, "已达国家数量上限: {}", n),
            FoundError::AlreadyInNation => write!(f, "你已在一个国家里"),
            FoundError::InsufficientSouls { have, need } => {
                write!(f, "灵魂不足: 有 {} 需要 {}", have, need)
            }
            FoundError::PoolError(e) => write!(f, "Pool: {}", e),
        }
    }
}

impl std::error::Error for FoundError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinError {
    AlreadyInNation,
    NoSuchNation,
    PopulationFull { current: u32, cap: u32 },
}

impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JoinError::AlreadyInNation => write!(f, "你已在一个国家里"),
            JoinError::NoSuchNation => write!(f, "国家不存在"),
            JoinError::PopulationFull { current, cap } => {
                write!(f, "人口上限: {}/{}", current, cap)
            }
        }
    }
}

impl std::error::Error for JoinError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaveError {
    NotInNation,
}

impl fmt::Display for LeaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LeaveError::NotInNation => write!(f, "你不在任何国家里"),
        }
    }
}

impl std::error::Error for LeaveError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeError {
    NoSuchNation,
    AlreadyAtOrAbove { current: u32, target: u32 },
    InvalidTarget(u32),
    InsufficientResources,
    PoolError(PoolError),
}

impl fmt::Display for UpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpgradeError::NoSuchNation => write!(f, "国家不存在"),
            UpgradeError::AlreadyAtOrAbove { current, target } => {
                write!(f, "当前人口上限 {} >= 目标 {}", current, target)
            }
            UpgradeError::InvalidTarget(t) => write!(f, "无效目标人口上限: {}", t),
            UpgradeError::InsufficientResources => write!(f, "资源不足"),
            UpgradeError::PoolError(e) => write!(f, "Pool: {}", e),
        }
    }
}

impl std::error::Error for UpgradeError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg_with_souls(souls: i64) -> (NationRegistry, GlobalResourcePool) {
        let reg = NationRegistry::new();
        let mut pool = GlobalResourcePool::new();
        pool.force_add(ResourceKind::Soul, souls);
        (reg, pool)
    }

    #[test]
    fn new_registry_is_empty() {
        let reg = NationRegistry::new();
        assert_eq!(reg.len(), 0);
        assert_eq!(reg.flag_count, 0);
    }

    #[test]
    fn found_first_nation_costs_10_souls() {
        let (mut reg, mut pool) = reg_with_souls(10);
        let id = reg.found(&mut pool, 1, "TestNation".into(), [16, 8, 16], 0).unwrap();
        assert_eq!(id, NationId(0));
        assert_eq!(reg.flag_count, 1);
        assert_eq!(pool.get(ResourceKind::Soul), 0);
        assert_eq!(reg.nations.get(&id).unwrap().king, 1);
        assert_eq!(reg.nations.get(&id).unwrap().founding_order, 1);
    }

    #[test]
    fn found_nation_incremental_cost() {
        let (mut reg, mut pool) = reg_with_souls(10 + 15);
        let _ = reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();

        let _ = reg.found(&mut pool, 2, "B".into(), [2, 1, 1], 0).unwrap();
        assert_eq!(pool.get(ResourceKind::Soul), 0);
        assert_eq!(reg.flag_count, 2);
    }

    #[test]
    fn cannot_exceed_8_flags() {
        let (mut reg, mut pool) = reg_with_souls(10 + 15 + 20 + 25 + 30 + 40 + 50 + 60);
        for i in 1..=8 {
            reg.found(&mut pool, i as u32, format!("N{}", i), [i, 1, 1], 0).unwrap();
        }
        assert_eq!(reg.flag_count, 8);
        assert!(!reg.can_found_new());

        let err = reg.found(&mut pool, 9, "N9".into(), [9, 1, 1], 0).unwrap_err();
        assert!(matches!(err, FoundError::MaxFlagsReached(8)));
    }

    #[test]
    fn insufficient_souls_fails() {
        let (mut reg, mut pool) = reg_with_souls(5);
        let err = reg.found(&mut pool, 1, "N".into(), [1, 1, 1], 0).unwrap_err();
        assert!(matches!(
            err,
            FoundError::InsufficientSouls { have: 5, need: 10 }
        ));
    }

    #[test]
    fn cannot_found_if_already_in_nation() {
        let (mut reg, mut pool) = reg_with_souls(100);
        reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();

        let err = reg.found(&mut pool, 1, "B".into(), [2, 1, 1], 0).unwrap_err();
        assert!(matches!(err, FoundError::AlreadyInNation));
    }

    #[test]
    fn join_respects_population_cap() {
        let (mut reg, mut pool) = reg_with_souls(10);
        let id = reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();

        for i in 2..=5 {
            reg.join(id, i).unwrap();
        }

        let err = reg.join(id, 6).unwrap_err();
        assert!(matches!(
            err,
            JoinError::PopulationFull { current: 5, cap: 5 }
        ));
    }

    #[test]
    fn upgrade_population_consumes_resources() {
        let (mut reg, mut pool) = reg_with_souls(10 + 25);
        pool.force_add(ResourceKind::Wood, 3_000);
        pool.force_add(ResourceKind::Food, 2_000);
        let id = reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();

        reg.upgrade_population(&mut pool, id, 10).unwrap();
        assert_eq!(reg.nations.get(&id).unwrap().pop_cap, 10);
        assert_eq!(pool.get(ResourceKind::Wood), 3_000 - 500);
        assert_eq!(pool.get(ResourceKind::Food), 2_000 - 200);

        reg.upgrade_population(&mut pool, id, 15).unwrap();
        assert_eq!(reg.nations.get(&id).unwrap().pop_cap, 15);
    }

    #[test]
    fn damage_flag_dissolves_nation_at_zero_hp() {
        let (mut reg, mut pool) = reg_with_souls(10);
        let id = reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();
        let hp = reg.damage_flag(id, 50);
        assert_eq!(hp, 50);
        assert!(reg.nations.contains_key(&id));

        let hp = reg.damage_flag(id, 60);
        assert_eq!(hp, 0);
        assert!(!reg.nations.contains_key(&id));
        assert_eq!(reg.flag_count, 0);
    }

    #[test]
    fn dissolve_releases_founding_order() {
        let (mut reg, mut pool) = reg_with_souls(25);
        let id1 = reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();
        let _id2 = reg.found(&mut pool, 2, "B".into(), [2, 1, 1], 0).unwrap();
        assert_eq!(reg.flag_count, 2);
        reg.damage_flag(id1, FLAG_HP);
        assert_eq!(reg.flag_count, 1);

        assert!(!reg.flag_orders_taken.contains(&id1.0));
    }

    #[test]
    fn flag_costs_follow_doc_table() {
        assert_eq!(FLAG_COSTS_SOULS, [10, 15, 20, 25, 30, 40, 50, 60]);
    }

    #[test]
    fn found_increments_id() {
        let (mut reg, mut pool) = reg_with_souls(50);
        let id1 = reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();
        let id2 = reg.found(&mut pool, 2, "B".into(), [2, 1, 1], 0).unwrap();
        let id3 = reg.found(&mut pool, 3, "C".into(), [3, 1, 1], 0).unwrap();
        assert_eq!(id1, NationId(0));
        assert_eq!(id2, NationId(1));
        assert_eq!(id3, NationId(2));
    }

    #[test]
    fn find_player_returns_correct_nation() {
        let (mut reg, _pool) = reg_with_souls(50);
        let mut p = _pool;
        let id_a = reg.found(&mut p, 1, "A".into(), [1, 1, 1], 0).unwrap();
        let id_b = reg.found(&mut p, 2, "B".into(), [2, 1, 1], 0).unwrap();
        reg.join(id_a, 3).unwrap();
        reg.join(id_b, 4).unwrap();
        assert_eq!(reg.find_nation_by_player(1), Some(id_a));
        assert_eq!(reg.find_nation_by_player(3), Some(id_a));
        assert_eq!(reg.find_nation_by_player(2), Some(id_b));
        assert_eq!(reg.find_nation_by_player(4), Some(id_b));
        assert_eq!(reg.find_nation_by_player(99), None);
    }

    fn reg_with_two_nations_and_pool(
        wood: i64,
        food: i64,
    ) -> (NationRegistry, GlobalResourcePool, NationId, NationId) {
        let mut reg = NationRegistry::new();
        let mut pool = GlobalResourcePool::new();
        pool.force_add(ResourceKind::Soul, 10 + 15);
        pool.force_add(ResourceKind::Wood, wood);
        pool.force_add(ResourceKind::Food, food);
        let id_a = reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();
        let id_b = reg.found(&mut pool, 2, "B".into(), [2, 1, 1], 0).unwrap();
        (reg, pool, id_a, id_b)
    }

    #[test]
    fn upkeep_consumes_wood_and_food_per_interval_tick() {
        let (mut reg, mut pool, _id_a, _id_b) = reg_with_two_nations_and_pool(100, 100);
        let wood_before = pool.get(ResourceKind::Wood);
        let food_before = pool.get(ResourceKind::Food);

        let report = reg.tick_upkeep(&mut pool, NATION_UPKEEP_INTERVAL_TICKS);

        assert_eq!(report.checked, 2);
        assert_eq!(report.kept, 2);
        assert_eq!(report.missed, 0);
        assert_eq!(report.dissolved, 0);
        assert_eq!(
            pool.get(ResourceKind::Wood),
            wood_before - 2 * NATION_UPKEEP_WOOD_PER_TICK
        );
        assert_eq!(
            pool.get(ResourceKind::Food),
            food_before - 2 * NATION_UPKEEP_FOOD_PER_TICK
        );
    }

    #[test]
    fn upkeep_is_noop_off_interval() {
        let (mut reg, mut pool, _id_a, _id_b) = reg_with_two_nations_and_pool(100, 100);
        let wood_before = pool.get(ResourceKind::Wood);
        let food_before = pool.get(ResourceKind::Food);
        let report = reg.tick_upkeep(&mut pool, NATION_UPKEEP_INTERVAL_TICKS + 1);
        assert_eq!(report, UpkeepReport::default());
        assert_eq!(pool.get(ResourceKind::Wood), wood_before);
        assert_eq!(pool.get(ResourceKind::Food), food_before);
    }

    #[test]
    fn upkeep_decreases_flag_hp_when_pool_low() {
        let (mut reg, mut pool, id_a, _id_b) = reg_with_two_nations_and_pool(0, 0);
        let hp_before = reg.nations.get(&id_a).unwrap().flag_hp;

        let report = reg.tick_upkeep(&mut pool, NATION_UPKEEP_INTERVAL_TICKS);

        assert_eq!(report.checked, 2);
        assert_eq!(report.kept, 0);
        assert_eq!(report.missed, 2);
        assert_eq!(report.dissolved, 0);
        let hp_after = reg.nations.get(&id_a).unwrap().flag_hp;
        assert_eq!(hp_after, hp_before - NATION_UPKEEP_MISS_HP_LOSS);
    }

    #[test]
    fn upkeep_dissolves_nation_when_flag_hp_reaches_zero() {
        let (mut reg, mut pool, id_a, _id_b) = reg_with_two_nations_and_pool(0, 0);

        let start_hp = reg.nations.get(&id_a).unwrap().flag_hp;
        let ticks_needed = start_hp.div_ceil(NATION_UPKEEP_MISS_HP_LOSS);
        for i in 0..ticks_needed {
            reg.tick_upkeep(&mut pool, (i as u64 + 1) * NATION_UPKEEP_INTERVAL_TICKS);
        }
        assert!(!reg.nations.contains_key(&id_a));
        assert_eq!(reg.flag_count, 0);
    }
}
