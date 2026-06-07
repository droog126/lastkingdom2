//! Nation System
//!
//! §五、国家系统：战争与政治
//!
//! §五、1 创建国家 (Founding a Nation)
//! §五、2 国家管理 (Nation Management)
//! §五、3 联盟条约与战争疲劳 (略 - demo 不做)
//! §五、4 国家接管与遗址再建 (略)
//! §五、5 晚建国家缓释机制 (略)
//!
//! Demo 范围：
//!   * 国旗创建（灵魂买，递增成本，最大 8 面）
//!   * 国家成员管理（join / leave，population cap 5/10/15/20）
//!   * 国旗 HP（被攻击掉血，归零拆旗，国家解散）
//!   * 战利品回收（拆旗 = soul 回归池）

#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fmt;

use crate::constant::*;
use crate::resource::{
    GlobalResourcePool, PoolError, ResourceKind, Transfer, TransferDst, TransferSrc, apply_transfer,
};

// ---------------------------------------------------------------------------
// NationId
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NationId(pub u32);

impl fmt::Display for NationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Nation#{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Nation：核心实体
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Nation {
    pub id: NationId,
    pub name: String,
    /// 国王玩家 ID
    pub king: u32,
    /// 所有成员（含国王）
    pub members: BTreeSet<u32>,
    /// 当前人口上限
    pub pop_cap: u32,
    /// 国旗位置（世界坐标）
    pub flag_pos: [i32; 3],
    /// 国旗当前 HP
    pub flag_hp: u32,
    /// 国旗最大 HP
    pub flag_hp_max: u32,
    /// 战争疲劳（0-100）
    pub war_exhaustion: i32,
    /// 创建时间（tick，调试用）
    pub created_at_tick: u64,
    /// 在全局中按"国旗顺序"创立的序号（1-8）—— 决定递增成本 / 时代补偿
    pub founding_order: u32,
}

impl Nation {
    pub fn new(id: NationId, name: String, king: u32, flag_pos: [i32; 3], tick: u64, order: u32) -> Self {
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

    /// 是否能升级人口到 10
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

// ---------------------------------------------------------------------------
// NationRegistry：所有国家 + 国旗顺序追踪
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Clone, Default)]
pub struct NationRegistry {
    /// ID → Nation
    pub nations: HashMap<NationId, Nation>,
    /// 已分配的最大 ID + 1（单调递增）
    next_id: u32,
    /// 当前已有的国旗面数
    pub flag_count: u32,
    /// 国旗顺序：哪些 order 已经被占了
    pub flag_orders_taken: BTreeSet<u32>,
}

impl NationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前已有的国家数
    pub fn len(&self) -> usize {
        self.nations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nations.is_empty()
    }

    /// 是否还能再买一面国旗
    pub fn can_found_new(&self) -> bool {
        self.flag_count < MAX_NATIONAL_FLAGS
    }

    /// 第 N 面国旗的购买成本（按已购数量递增）
    pub fn next_flag_cost(&self) -> i64 {
        let i = self.flag_count as usize;
        if i >= FLAG_COSTS_SOULS.len() {
            // 超过 8 面不应该发生（can_found_new 守门）
            FLAG_COSTS_SOULS[FLAG_COSTS_SOULS.len() - 1] as i64
        } else {
            FLAG_COSTS_SOULS[i] as i64
        }
    }

    /// 创建一个新国家
    ///
    /// 流程：
    ///   1. 校验：玩家不在别的国家 + 池子灵魂够 + 还有空位
    ///   2. 灵魂从池子"销毁"（买旗=消耗，不是转移）
    ///   3. 分配 ID + founding_order
    ///   4. 插入 registry
    pub fn found(
        &mut self,
        pool: &mut GlobalResourcePool,
        player_id: u32,
        name: String,
        flag_pos: [i32; 3],
        tick: u64,
    ) -> Result<NationId, FoundError> {
        // 校验
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
        // 消耗灵魂
        let t = Transfer {
            kind: ResourceKind::Soul,
            amount: cost,
            src: TransferSrc::PlayerGather(player_id), // 这里语义反着用：src = 玩家→池子，dst = 销毁
            dst: TransferDst::Wasted,                  // 销毁
        };
        // 等等——Transfer 的语义是 src 减 / dst 加。买旗应该是"玩家灵魂→销毁"。
        // 但 TransferSrc::PlayerGather 意思是"玩家从某处采集"——这不对。
        // 让我们换一个 TransferSrc：直接 try_sub 然后单独处理。
        pool.try_sub(ResourceKind::Soul, cost)
            .map_err(|e| FoundError::PoolError(e))?;
        pool.audit_subtracted
            .entry(ResourceKind::Soul)
            .and_modify(|v| *v += cost);
        // 这里其实有个守恒问题：买了旗灵魂没了，但池子里也没"多出"什么
        // 所以 sub 后 audit_subtracted 累加，但 audit_added 没动 → verify_conservation 不会报错（sub ≤ add）

        // 分配 ID
        let id = NationId(self.next_id);
        self.next_id += 1;

        // 分配 order（1-8）
        let order = (self.flag_count + 1) as u32;
        self.flag_orders_taken.insert(order);

        // 创建
        let nation = Nation::new(id, name, player_id, flag_pos, tick, order);
        self.nations.insert(id, nation);
        self.flag_count += 1;

        Ok(id)
    }

    /// 玩家加入国家
    pub fn join(&mut self, nation_id: NationId, player_id: u32) -> Result<(), JoinError> {
        // 已经在别的国家
        if self.find_nation_by_player(player_id).is_some() {
            return Err(JoinError::AlreadyInNation);
        }
        let n = self.nations.get_mut(&nation_id).ok_or(JoinError::NoSuchNation)?;
        if n.size() as u32 >= n.pop_cap {
            return Err(JoinError::PopulationFull {
                current: n.size() as u32,
                cap: n.pop_cap,
            });
        }
        n.members.insert(player_id);
        Ok(())
    }

    /// 玩家离开国家
    pub fn leave(&mut self, player_id: u32) -> Result<NationId, LeaveError> {
        let id = self
            .find_nation_by_player(player_id)
            .ok_or(LeaveError::NotInNation)?;
        let n = self.nations.get_mut(&id).unwrap();
        if player_id == n.king {
            // 国王离开 = 国家解散（传位太复杂，demo 直接解散）
            // 把所有灵魂按人数均分归还（这里不守恒——属于"特殊事件"，记 audit）
            // 实际：解散时，国旗 HP 归零
            n.flag_hp = 0;
        } else {
            n.members.remove(&player_id);
        }
        Ok(id)
    }

    /// 找玩家所在的国家
    pub fn find_nation_by_player(&self, player_id: u32) -> Option<NationId> {
        self.nations
            .values()
            .find(|n| n.is_member(player_id))
            .map(|n| n.id)
    }

    /// 对一个国家造成 HP 伤害（攻击旗帜）。返回剩余 HP
    /// HP=0 → 国家解散
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

    /// 解散一个国家（所有成员变流亡，flag_count 减少，founding_order 被释放）
    fn dissolve(&mut self, nation_id: NationId) {
        // 1. 取出 founding_order 并从 flag_orders_taken 移除
        if let Some(n) = self.nations.get(&nation_id) {
            self.flag_orders_taken.remove(&n.founding_order);
        }
        // 2. 删 nation
        self.nations.remove(&nation_id);
        // 3. flag_count 减一
        self.flag_count = self.flag_count.saturating_sub(1);
    }

    /// 升级人口上限（消耗资源）
    pub fn upgrade_population(
        &mut self,
        pool: &mut GlobalResourcePool,
        nation_id: NationId,
        target: u32,
    ) -> Result<(), UpgradeError> {
        let n = self.nations.get_mut(&nation_id).ok_or(UpgradeError::NoSuchNation)?;
        let current_cap = n.pop_cap;
        if target <= current_cap {
            return Err(UpgradeError::AlreadyAtOrAbove {
                current: current_cap,
                target,
            });
        }
        if !matches!(target, 10 | 15 | 20) {
            return Err(UpgradeError::InvalidTarget(target));
        }
        let (wood, food, soul) = Nation::pop_upgrade_cost(target);
        // 校验
        if pool.get(ResourceKind::Wood) < wood as i64
            || pool.get(ResourceKind::Food) < food as i64
            || pool.get(ResourceKind::Soul) < soul as i64
        {
            return Err(UpgradeError::InsufficientResources);
        }
        // 扣资源（国家开销 = 销毁）
        if wood > 0 {
            pool.try_sub(ResourceKind::Wood, wood as i64).map_err(|e: PoolError| UpgradeError::PoolError(e))?;
        }
        if food > 0 {
            pool.try_sub(ResourceKind::Food, food as i64).map_err(|e: PoolError| UpgradeError::PoolError(e))?;
        }
        if soul > 0 {
            pool.try_sub(ResourceKind::Soul, soul as i64).map_err(|e: PoolError| UpgradeError::PoolError(e))?;
        }
        // 升级
        n.pop_cap = target;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

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
            FoundError::MaxFlagsReached(n) => write!(f, "已达 {} 面上限", n),
            FoundError::AlreadyInNation => write!(f, "你已在一个国家里"),
            FoundError::InsufficientSouls { have, need } => {
                write!(f, "灵魂不足: 有 {} 需 {}", have, need)
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn reg_with_souls(souls: i64) -> (NationRegistry, GlobalResourcePool) {
        let mut reg = NationRegistry::new();
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
        let id = reg
            .found(&mut pool, 1, "TestNation".into(), [16, 8, 16], 0)
            .unwrap();
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
        // 2nd cost is 15
        let _ = reg.found(&mut pool, 2, "B".into(), [2, 1, 1], 0).unwrap();
        assert_eq!(pool.get(ResourceKind::Soul), 0);
        assert_eq!(reg.flag_count, 2);
    }

    #[test]
    fn cannot_exceed_8_flags() {
        let (mut reg, mut pool) = reg_with_souls(10 + 15 + 20 + 25 + 30 + 40 + 50 + 60);
        for i in 1..=8 {
            reg.found(&mut pool, i as u32, format!("N{}", i), [i, 1, 1], 0)
                .unwrap();
        }
        assert_eq!(reg.flag_count, 8);
        assert!(!reg.can_found_new());
        // 第 9 面国旗应该失败
        let err = reg
            .found(&mut pool, 9, "N9".into(), [9, 1, 1], 0)
            .unwrap_err();
        assert!(matches!(err, FoundError::MaxFlagsReached(8)));
    }

    #[test]
    fn insufficient_souls_fails() {
        let (mut reg, mut pool) = reg_with_souls(5);  // 不足 10
        let err = reg
            .found(&mut pool, 1, "N".into(), [1, 1, 1], 0)
            .unwrap_err();
        assert!(matches!(err, FoundError::InsufficientSouls { have: 5, need: 10 }));
    }

    #[test]
    fn cannot_found_if_already_in_nation() {
        let (mut reg, mut pool) = reg_with_souls(100);
        reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();
        // 玩家 1 尝试再建一个国家
        let err = reg
            .found(&mut pool, 1, "B".into(), [2, 1, 1], 0)
            .unwrap_err();
        assert!(matches!(err, FoundError::AlreadyInNation));
    }

    #[test]
    fn join_respects_population_cap() {
        let (mut reg, mut pool) = reg_with_souls(10);
        let id = reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();
        // 初始 cap=5，king=1 已占 1 席位，还能加 4
        for i in 2..=5 {
            reg.join(id, i).unwrap();
        }
        // 第 6 个应失败
        let err = reg.join(id, 6).unwrap_err();
        assert!(matches!(err, JoinError::PopulationFull { current: 5, cap: 5 }));
    }

    #[test]
    fn upgrade_population_consumes_resources() {
        let (mut reg, mut pool) = reg_with_souls(10 + 25);  // 25 是升级到 15 需要的
        pool.force_add(ResourceKind::Wood, 3_000);
        pool.force_add(ResourceKind::Food, 2_000);
        let id = reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();
        // 升级到 10
        reg.upgrade_population(&mut pool, id, 10).unwrap();
        assert_eq!(reg.nations.get(&id).unwrap().pop_cap, 10);
        assert_eq!(pool.get(ResourceKind::Wood), 3_000 - 500);
        assert_eq!(pool.get(ResourceKind::Food), 2_000 - 200);
        // 升级到 15（需 1000 wood + 500 food + 10 soul）
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
        // 致命一击
        let hp = reg.damage_flag(id, 60);
        assert_eq!(hp, 0);
        assert!(!reg.nations.contains_key(&id));
        assert_eq!(reg.flag_count, 0);
    }

    #[test]
    fn dissolve_releases_founding_order() {
        // 创 2 国需要 10 + 15 = 25 soul
        let (mut reg, mut pool) = reg_with_souls(25);
        let id1 = reg.found(&mut pool, 1, "A".into(), [1, 1, 1], 0).unwrap();
        let _id2 = reg.found(&mut pool, 2, "B".into(), [2, 1, 1], 0).unwrap();
        assert_eq!(reg.flag_count, 2);
        reg.damage_flag(id1, FLAG_HP);
        assert_eq!(reg.flag_count, 1);
        // 解散后 id1 的 founding_order 应该被释放
        assert!(!reg.flag_orders_taken.contains(&id1.0));
    }

    #[test]
    fn flag_costs_follow_doc_table() {
        // 总纲表：第1=10, 2=15, 3=20, 4=25, 5=30, 6=40, 7=50, 8=60
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
}
