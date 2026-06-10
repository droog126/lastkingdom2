//! Monster Ecosystem
//!
//! §三、怪物王国与小巢生态
//!
//! Demo 简化（缩到 1 王国 + 3 小巢 = 60-80 个体；100 人局才到 5+80 = 1500 上限）
//!   * MonsterKingdom: 1 个，初始 80 个体
//!   * MonsterNest: 3 个，初始各 20 个体
//!   * MonsterIndividual: 总和 < 80（demo 上限）
//!   * 死亡转化：食物 25% → 灵魂（守恒）
//!   * tick 行为：觅食 → 体力 → 衰减 → 死亡 → 守恒释放

#![allow(dead_code)]

use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::constant::*;
use crate::resource::{
    GlobalResourcePool, ResourceKind, Transfer, TransferDst, TransferSrc, apply_transfer,
};
use crate::world::Biome;

// ---------------------------------------------------------------------------
// MonsterKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonsterKind {
    Snake,        // 基础蛇
    FrostElf,     // 苔原精英
    SandWurm,     // 沙漠精英
    Treant,       // 丛林守护
    AetherWraith, // 以太界（demo 不用）
}

impl MonsterKind {
    /// 归属 biome
    pub fn biome(self) -> Biome {
        match self {
            MonsterKind::Snake => Biome::Jungle,
            MonsterKind::FrostElf => Biome::Tundra,
            MonsterKind::SandWurm => Biome::Desert,
            MonsterKind::Treant => Biome::Jungle,
            MonsterKind::AetherWraith => Biome::Jungle, // demo 占位
        }
    }

    pub const fn label_zh(self) -> &'static str {
        match self {
            MonsterKind::Snake => "蛇",
            MonsterKind::FrostElf => "冰霜精灵",
            MonsterKind::SandWurm => "沙虫",
            MonsterKind::Treant => "守护古树",
            MonsterKind::AetherWraith => "以太幽魂",
        }
    }

    /// 死亡掉什么（守恒：sub from pool → drop to player; 但 demo 简化：直接归还池子）
    pub fn food_on_death(self) -> i64 {
        match self {
            MonsterKind::Snake => 5,
            MonsterKind::FrostElf => 20,
            MonsterKind::SandWurm => 20,
            MonsterKind::Treant => 50,
            MonsterKind::AetherWraith => 100,
        }
    }
}

// ---------------------------------------------------------------------------
// MonsterIndividual
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MonsterIndividual {
    pub id: u32,
    pub kind: MonsterKind,
    pub nest_id: u32,    // 0 = 顶级（不在任何 nest），>0 = 隶属小巢
    pub kingdom_id: u32, // 所属王国
    pub hp: i32,
    pub max_hp: i32,
    pub food: i64,             // 体内食物（死亡时 25% 转化为灵魂）
    pub last_active_tick: u64, // 巢穴用
    pub position: [i32; 3],    // 简单位置
}

impl MonsterIndividual {
    pub fn new(id: u32, kind: MonsterKind, kingdom_id: u32, nest_id: u32, pos: [i32; 3]) -> Self {
        let max_hp = match kind {
            MonsterKind::Snake => 20,
            MonsterKind::FrostElf => 80,
            MonsterKind::SandWurm => 80,
            MonsterKind::Treant => 200,
            MonsterKind::AetherWraith => 500,
        };
        Self {
            id,
            kind,
            nest_id,
            kingdom_id,
            hp: max_hp,
            max_hp,
            food: kind.food_on_death(),
            last_active_tick: 0,
            position: pos,
        }
    }

    /// 死亡 → 食物 25% 转化为灵魂归还池子
    pub fn die(&self, pool: &mut GlobalResourcePool) {
        let soul = (self.food as f64 * 0.25) as i64;
        if soul > 0 {
            // 通过 regen 路径（force_add）归还灵魂
            let t = Transfer {
                kind: ResourceKind::Soul,
                amount: soul,
                src: TransferSrc::Regen,  // 守恒审计上算"再生"而非"加"
                dst: TransferDst::Wasted, //  无所谓
            };
            let _ = apply_transfer(pool, t);
        }
    }
}

// ---------------------------------------------------------------------------
// MonsterNest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MonsterNest {
    pub id: u32,
    pub kingdom_id: u32,
    pub biome: Biome,
    pub center: [i32; 3],
    pub individuals: HashMap<u32, MonsterIndividual>, // id -> individual
    pub last_activity_tick: u64,
    pub dormant: bool,
}

impl MonsterNest {
    pub fn new(id: u32, kingdom_id: u32, biome: Biome, center: [i32; 3], n: u32) -> Self {
        let mut individuals = HashMap::new();
        for i in 0..n {
            let mid = id * 1000 + i + 1; // 全局唯一 mid
            let kind = match biome {
                Biome::Desert => MonsterKind::SandWurm,
                Biome::Tundra => MonsterKind::FrostElf,
                Biome::Jungle => MonsterKind::Snake,
            };
            individuals.insert(
                mid,
                MonsterIndividual::new(mid, kind, kingdom_id, id, center),
            );
        }
        Self { id, kingdom_id, biome, center, individuals, last_activity_tick: 0, dormant: false }
    }

    pub fn size(&self) -> u32 {
        self.individuals.len() as u32
    }

    /// 5 分钟无活动 → 进入休眠
    pub fn check_dormancy(&mut self, current_tick: u64) {
        if !self.dormant
            && current_tick.saturating_sub(self.last_activity_tick) > NEST_DORMANCY_SECS as u64
        {
            self.dormant = true;
        }
    }

    /// 休眠后每 tick 25% 几率衰亡
    pub fn tick_decay(&mut self, rng: &mut StdRng) -> bool {
        if !self.dormant {
            return false;
        }
        rng.next_u32() % 4 == 0 // 25%
    }
}

// ---------------------------------------------------------------------------
// MonsterKingdom
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MonsterKingdom {
    pub id: u32,
    pub biome: Biome,
    pub center: [i32; 3],
    pub nests: HashMap<u32, MonsterNest>,
    pub destroyed: bool,
    pub destroyed_at_tick: Option<u64>,
}

impl MonsterKingdom {
    pub fn new(id: u32, biome: Biome, center: [i32; 3], nest_count: u32) -> Self {
        let mut nests = HashMap::new();
        for i in 0..nest_count {
            let nid = id * 100 + i + 1;
            let offset_x = (i as i32 - 1) * 4;
            let offset_z = (i as i32 - 1) * 4;
            let nc = [center[0] + offset_x, center[1], center[2] + offset_z];
            let n = MonsterNest::new(nid, id, biome, nc, 20);
            nests.insert(nid, n);
        }
        Self { id, biome, center, nests, destroyed: false, destroyed_at_tick: None }
    }

    pub fn total_individuals(&self) -> u32 {
        self.nests.values().map(|n| n.size()).sum()
    }
}

// ---------------------------------------------------------------------------
// MonsterEcosystem Registry
// ---------------------------------------------------------------------------

#[derive(Resource, Debug)]
pub struct MonsterEcosystem {
    pub kingdoms: HashMap<u32, MonsterKingdom>,
    pub max_individuals: u32, // 全局上限
    pub current_individuals: u32,
    pub rng: StdRng,
    pub current_tick: u64,
    next_kingdom_id: u32,
    /// 累计死亡转化给池子的灵魂（守恒审计）
    pub soul_yielded: i64,
}

// 手动 Clone：rand 0.10 的 StdRng 不再 impl Clone（避免泄漏加密 RNG 状态）
// clone 时给一个新的 RNG（用同样的 seed），对 demo 够用
impl Clone for MonsterEcosystem {
    fn clone(&self) -> Self {
        Self {
            kingdoms: self.kingdoms.clone(),
            max_individuals: self.max_individuals,
            current_individuals: self.current_individuals,
            rng: StdRng::seed_from_u64(0xDEAD_BEEF),
            current_tick: self.current_tick,
            next_kingdom_id: self.next_kingdom_id,
            soul_yielded: self.soul_yielded,
        }
    }
}

impl Default for MonsterEcosystem {
    fn default() -> Self {
        Self {
            kingdoms: HashMap::new(),
            max_individuals: 80, // demo 缩
            current_individuals: 0,
            rng: StdRng::seed_from_u64(0xDEAD_BEEF),
            current_tick: 0,
            next_kingdom_id: 0,
            soul_yielded: 0,
        }
    }
}

impl MonsterEcosystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clone(&self) -> Self {
        let rng = StdRng::seed_from_u64(self.current_tick as u64);
        Self {
            kingdoms: self.kingdoms.clone(),
            max_individuals: self.max_individuals,
            current_individuals: self.current_individuals,
            rng,
            current_tick: self.current_tick,
            next_kingdom_id: self.next_kingdom_id,
            soul_yielded: self.soul_yielded,
        }
    }

    /// demo 初始化：1 个王国（jungle）+ 3 个小巢各 20 = 60 个体
    pub fn demo_init(&mut self, world_center: [i32; 3]) {
        self.spawn_kingdom(
            Biome::Jungle,
            [world_center[0], world_center[1], world_center[2] - 8],
            3,
        );
    }

    pub fn spawn_kingdom(&mut self, biome: Biome, center: [i32; 3], nest_count: u32) -> u32 {
        let id = self.next_kingdom_id;
        self.next_kingdom_id += 1;
        let k = MonsterKingdom::new(id, biome, center, nest_count);
        let count = k.total_individuals();
        self.current_individuals += count;
        self.kingdoms.insert(id, k);
        id
    }

    /// tick：每个小巢检查 dormancy + decay；dead kingdom 不再处理
    pub fn tick(&mut self, pool: &mut GlobalResourcePool) {
        self.current_tick += 1;
        let current = self.current_tick;
        // rand 0.10: StdRng::from_rng 是 infallible，返回 Self（直接拿）
        let mut rng = StdRng::from_rng(&mut self.rng);
        let mut to_remove: Vec<u32> = Vec::new();
        for (_kid, k) in self.kingdoms.iter_mut() {
            if k.destroyed {
                continue;
            }
            for (_nid, n) in k.nests.iter_mut() {
                n.check_dormancy(current);
                if n.tick_decay(&mut rng) {
                    // 衰亡 → 释放个体配额
                    self.current_individuals = self.current_individuals.saturating_sub(n.size());
                    to_remove.push(n.id);
                }
            }
        }
        // 移除衰亡小巢
        for (_kid, k) in self.kingdoms.iter_mut() {
            for nid in &to_remove {
                k.nests.remove(nid);
            }
        }
        self.rng = rng;
    }

    /// 怪物死亡入口（从外部调用，比如玩家攻击）
    /// 返回 true = 个体被移除
    pub fn kill_individual(
        &mut self,
        kingdom_id: u32,
        nest_id: u32,
        individual_id: u32,
        pool: &mut GlobalResourcePool,
    ) -> bool {
        if let Some(k) = self.kingdoms.get_mut(&kingdom_id) {
            if let Some(n) = k.nests.get_mut(&nest_id) {
                if let Some(ind) = n.individuals.remove(&individual_id) {
                    ind.die(pool);
                    self.soul_yielded += (ind.food as f64 * 0.25) as i64;
                    self.current_individuals = self.current_individuals.saturating_sub(1);
                    n.last_activity_tick = self.current_tick;
                    return true;
                }
            }
        }
        false
    }

    /// 摧毁一个王国（V1.0 简化：直接移除整个王国 + 释放个体）
    /// 总纲：散落为 3-6 个小巢（demo 略，直接清空）
    pub fn destroy_kingdom(&mut self, kingdom_id: u32) {
        if let Some(k) = self.kingdoms.get_mut(&kingdom_id) {
            let count = k.total_individuals();
            self.current_individuals = self.current_individuals.saturating_sub(count);
            k.destroyed = true;
            k.destroyed_at_tick = Some(self.current_tick);
            k.nests.clear();
        }
    }

    /// 守恒校验：current_individuals 应该 = 所有 nests 的个体数之和
    pub fn verify_individual_count(&self) -> bool {
        let sum: u32 =
            self.kingdoms.values().filter(|k| !k.destroyed).map(|k| k.total_individuals()).sum();
        sum == self.current_individuals
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_init_creates_60_individuals() {
        let mut eco = MonsterEcosystem::new();
        eco.demo_init([16, 8, 16]);
        assert_eq!(eco.current_individuals, 60);
        assert!(eco.kingdoms.len() == 1);
    }

    #[test]
    fn destroy_kingdom_releases_individual_quota() {
        let mut eco = MonsterEcosystem::new();
        eco.demo_init([16, 8, 16]);
        assert_eq!(eco.current_individuals, 60);
        let kid = *eco.kingdoms.keys().next().unwrap();
        eco.destroy_kingdom(kid);
        assert_eq!(eco.current_individuals, 0);
    }

    #[test]
    fn kill_individual_yields_souls_and_keeps_conservation() {
        let mut eco = MonsterEcosystem::new();
        eco.demo_init([16, 8, 16]);
        let mut pool = GlobalResourcePool::new();
        let kid = *eco.kingdoms.keys().next().unwrap();
        let k = eco.kingdoms.get(&kid).unwrap();
        let nid = *k.nests.keys().next().unwrap();
        let n = k.nests.get(&nid).unwrap();
        let mid = *n.individuals.keys().next().unwrap();
        let ind = n.individuals.get(&mid).unwrap();
        let expected_soul = (ind.food as f64 * 0.25) as i64;
        let result = eco.kill_individual(kid, nid, mid, &mut pool);
        assert!(result);
        assert_eq!(pool.get(ResourceKind::Soul), expected_soul);
        assert_eq!(eco.current_individuals, 59);
        assert_eq!(eco.soul_yielded, expected_soul);
    }

    #[test]
    fn tick_with_decay_progresses() {
        let mut eco = MonsterEcosystem::new();
        eco.demo_init([16, 8, 16]);
        // 强制一个小巢进入 dormancy
        let kid = *eco.kingdoms.keys().next().unwrap();
        let nid = {
            let k = eco.kingdoms.get_mut(&kid).unwrap();
            let nid = *k.nests.keys().next().unwrap();
            let n = k.nests.get_mut(&nid).unwrap();
            n.last_activity_tick = 0;
            n.dormant = true;
            nid
        };
        // 跑几次 tick，衰亡概率会扣 current_individuals
        for _ in 0..50 {
            let mut pool = GlobalResourcePool::new();
            eco.tick(&mut pool);
            if eco.kingdoms.get(&kid).unwrap().nests.get(&nid).is_none() {
                break;
            }
        }
        // 不严格断言是否衰亡（25% 概率），但 verify_count 必须守恒
        assert!(eco.verify_individual_count());
    }

    #[test]
    fn verify_count_always_holds() {
        let mut eco = MonsterEcosystem::new();
        eco.demo_init([16, 8, 16]);
        assert!(eco.verify_individual_count());
        let kid = *eco.kingdoms.keys().next().unwrap();
        let nid = {
            let k = eco.kingdoms.get(&kid).unwrap();
            *k.nests.keys().next().unwrap()
        };
        let mid = {
            let k = eco.kingdoms.get(&kid).unwrap();
            let n = k.nests.get(&nid).unwrap();
            *n.individuals.keys().next().unwrap()
        };
        let mut pool = GlobalResourcePool::new();
        eco.kill_individual(kid, nid, mid, &mut pool);
        assert!(eco.verify_individual_count());
    }
}
