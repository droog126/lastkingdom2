









#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;









#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceKind {

    Wood,
    HardenedWood,
    Apple,
    WheatSeeds,
    Carrot,
    Potato,

    Food,
    Soul,

    Sunstone,
    Frostcore,
    LivingRoot,

    BloodthistleSeeds,
    FrostleafSeeds,

    VoidEssence,

    GripOfFirelord,
    CoreOfIceGiant,
    WhisperOfTreant,
    EyeOfTheDeep,
    SandsOfTime,
    WraithFiber,
    GuardianFragment,
    StormCore,
    EarthRune,
    VampireFang,
    PhoenixFeather,

    SpiritEssence,
    RuneStone,
    RunePowder,
    StarSand,
    RelicCore,
    SovereignSpark,
    SparkFragment,
}

impl ResourceKind {

    pub const fn max(self) -> i64 {
        use ResourceKind::*;
        match self {
            Wood => 10_000,
            HardenedWood => 500,
            Apple => 5_000,
            WheatSeeds => 1_000,
            Carrot => 2_000,
            Potato => 2_000,
            Food => 20_000,
            Soul => 1_000,
            Sunstone => 200,
            Frostcore => 200,
            LivingRoot => 200,
            BloodthistleSeeds => 200,
            FrostleafSeeds => 200,
            VoidEssence => 100,
            GripOfFirelord => 50,
            CoreOfIceGiant => 50,
            WhisperOfTreant => 50,
            EyeOfTheDeep => 10,
            SandsOfTime => 20,
            WraithFiber => 20,
            GuardianFragment => 20,
            StormCore => 20,
            EarthRune => 20,
            VampireFang => 20,
            PhoenixFeather => 10,


            SpiritEssence => 2_400,
            RuneStone => 1_400,
            RunePowder => 900,
            StarSand => 900,
            RelicCore => 48,
            SovereignSpark => 6,
            SparkFragment => 18,
        }
    }




    pub const fn demo_initial_amount(self) -> i64 {
        let max = self.max();
        let half = max / 2;
        let capped = if half < 50 { half } else { 50 };
        let floored = if capped > 1 { capped } else { 1 };
        if floored < max { floored } else { max }
    }

    pub const fn label_zh(self) -> &'static str {
        use ResourceKind::*;
        match self {
            Wood => "木头",
            HardenedWood => "硬化木材",
            Apple => "苹果",
            WheatSeeds => "小麦种子",
            Carrot => "胡萝卜",
            Potato => "土豆",
            Food => "食物",
            Soul => "灵魂",
            Sunstone => "阳炎石",
            Frostcore => "霜心晶体",
            LivingRoot => "活根",
            BloodthistleSeeds => "血蓟种子",
            FrostleafSeeds => "霜叶草种子",
            VoidEssence => "虚空精华",
            GripOfFirelord => "炎魔的握柄",
            CoreOfIceGiant => "冰霜巨人核心",
            WhisperOfTreant => "古树低语",
            EyeOfTheDeep => "深海之眼",
            SandsOfTime => "时光沙",
            WraithFiber => "幽魂纤维",
            GuardianFragment => "守护者碎片",
            StormCore => "风暴核心",
            EarthRune => "大地符文",
            VampireFang => "吸血鬼之牙",
            PhoenixFeather => "凤凰羽毛",

            SpiritEssence => "灵质",
            RuneStone => "符文石",
            RunePowder => "符文粉",
            StarSand => "星砂",
            RelicCore => "遗迹核心",
            SovereignSpark => "王权火种",
            SparkFragment => "火种碎片",
        }
    }


    pub const ALL: &'static [ResourceKind] = &[
        ResourceKind::Wood,
        ResourceKind::HardenedWood,
        ResourceKind::Apple,
        ResourceKind::WheatSeeds,
        ResourceKind::Carrot,
        ResourceKind::Potato,
        ResourceKind::Food,
        ResourceKind::Soul,
        ResourceKind::Sunstone,
        ResourceKind::Frostcore,
        ResourceKind::LivingRoot,
        ResourceKind::BloodthistleSeeds,
        ResourceKind::FrostleafSeeds,
        ResourceKind::VoidEssence,
        ResourceKind::GripOfFirelord,
        ResourceKind::CoreOfIceGiant,
        ResourceKind::WhisperOfTreant,
        ResourceKind::EyeOfTheDeep,
        ResourceKind::SandsOfTime,
        ResourceKind::WraithFiber,
        ResourceKind::GuardianFragment,
        ResourceKind::StormCore,
        ResourceKind::EarthRune,
        ResourceKind::VampireFang,
        ResourceKind::PhoenixFeather,

        ResourceKind::SpiritEssence,
        ResourceKind::RuneStone,
        ResourceKind::RunePowder,
        ResourceKind::StarSand,
        ResourceKind::RelicCore,
        ResourceKind::SovereignSpark,
        ResourceKind::SparkFragment,
    ];
}











#[derive(Resource, Debug, Clone, Default)]
pub struct GlobalResourcePool {
    pub current: HashMap<ResourceKind, i64>,

    pub audit_added: HashMap<ResourceKind, i64>,

    pub audit_subtracted: HashMap<ResourceKind, i64>,
}

impl GlobalResourcePool {

    pub fn new() -> Self {
        let mut current = HashMap::new();
        let mut audit_added = HashMap::new();
        let mut audit_subtracted = HashMap::new();
        for k in ResourceKind::ALL {
            current.insert(*k, 0);
            audit_added.insert(*k, 0);
            audit_subtracted.insert(*k, 0);
        }
        Self { current, audit_added, audit_subtracted }
    }


    pub fn get(&self, k: ResourceKind) -> i64 {
        *self.current.get(&k).unwrap_or(&0)
    }



    pub fn try_add(&mut self, k: ResourceKind, amount: i64) -> Result<i64, PoolError> {
        if amount <= 0 {
            return Err(PoolError::NonPositiveAmount(amount));
        }
        let cur = self.get(k);
        let max = k.max();
        if cur + amount > max {
            return Err(PoolError::WouldExceedMax { kind: k, cur, amount, max });
        }
        let new = cur + amount;
        self.current.insert(k, new);
        *self.audit_added.entry(k).or_insert(0) += amount;
        Ok(new)
    }


    pub fn force_add(&mut self, k: ResourceKind, amount: i64) -> i64 {
        debug_assert!(amount >= 0, "force_add amount must be >= 0, got {}", amount);
        let new = self.get(k) + amount;
        self.current.insert(k, new);
        *self.audit_added.entry(k).or_insert(0) += amount;
        new
    }


    pub fn try_sub(&mut self, k: ResourceKind, amount: i64) -> Result<i64, PoolError> {
        if amount <= 0 {
            return Err(PoolError::NonPositiveAmount(amount));
        }
        let cur = self.get(k);
        if amount > cur {
            return Err(PoolError::Insufficient { kind: k, cur, amount });
        }
        let new = cur - amount;
        self.current.insert(k, new);
        *self.audit_subtracted.entry(k).or_insert(0) += amount;
        Ok(new)
    }



    pub fn verify_conservation(&self) -> Result<(), String> {
        let mut errs: Vec<String> = Vec::new();
        for k in ResourceKind::ALL {
            let cur = self.get(*k);
            let max = k.max();
            if cur < 0 {
                errs.push(format!("[{}] cur={} < 0", k.label_zh(), cur));
            }
            if cur > max {
                errs.push(format!("[{}] cur={} > max={}", k.label_zh(), cur, max));
            }
            let added = self.audit_added.get(k).copied().unwrap_or(0);
            let subbed = self.audit_subtracted.get(k).copied().unwrap_or(0);
            if subbed > added {
                errs.push(format!(
                    "[{}] audit broken: subbed={} > added={} (凭空消失 {} 个)",
                    k.label_zh(),
                    subbed,
                    added,
                    subbed - added
                ));
            }


        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join("\n"))
        }
    }


    pub fn reset_for_tests(&mut self) {
        for k in ResourceKind::ALL {
            self.current.insert(*k, 0);
            self.audit_added.insert(*k, 0);
            self.audit_subtracted.insert(*k, 0);
        }
    }


    pub fn non_zero_count(&self) -> usize {
        self.current.values().filter(|&&v| v > 0).count()
    }
}

impl fmt::Display for GlobalResourcePool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== GlobalResourcePool ===")?;
        for k in ResourceKind::ALL {
            let cur = self.get(*k);
            let max = k.max();
            let bar = (cur as f64 / max as f64 * 30.0) as usize;
            let mut bar_str: String = "█".repeat(bar);
            bar_str.push_str(&"░".repeat(30 - bar));
            writeln!(
                f,
                "  {:<14} {:>6}/{:<6} [{}] {}",
                k.label_zh(),
                cur,
                max,
                bar_str,
                if cur == max { " (FULL)" } else { "" }
            )?;
        }
        Ok(())
    }
}





#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    Insufficient {
        kind: ResourceKind,
        cur: i64,
        amount: i64,
    },
    WouldExceedMax {
        kind: ResourceKind,
        cur: i64,
        amount: i64,
        max: i64,
    },
    NonPositiveAmount(i64),
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoolError::Insufficient { kind, cur, amount } => write!(
                f,
                "[{}] 余额不足: cur={} < amount={}",
                kind.label_zh(),
                cur,
                amount
            ),
            PoolError::WouldExceedMax { kind, cur, amount, max } => write!(
                f,
                "[{}] 越过 max: cur={} + amount={} > max={}",
                kind.label_zh(),
                cur,
                amount,
                max
            ),
            PoolError::NonPositiveAmount(n) => write!(f, "amount 必须为正: {}", n),
        }
    }
}

impl std::error::Error for PoolError {}






#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transfer {
    pub kind: ResourceKind,
    pub amount: i64,
    pub src: TransferSrc,
    pub dst: TransferDst,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferSrc {

    Init,

    PlayerGather(u32),

    MonsterDrop(u32),

    Regen,

    Nation(u32),
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDst {

    PlayerUse(u32),

    NationTreasury(u32),

    NationBuild(u32),

    Wasted,
}









pub fn apply_transfer(pool: &mut GlobalResourcePool, t: Transfer) -> Result<i64, PoolError> {
    debug_assert!(t.amount > 0, "transfer amount must be > 0");
    match t.src {
        TransferSrc::Regen
        | TransferSrc::Init
        | TransferSrc::PlayerGather(_)
        | TransferSrc::MonsterDrop(_) => {


            pool.force_add(t.kind, t.amount);
        }
        TransferSrc::Nation(nation_id) => {

            pool.try_sub(t.kind, t.amount)?;

            match t.dst {
                TransferDst::Wasted => {

                }
                TransferDst::PlayerUse(_)
                | TransferDst::NationTreasury(_)
                | TransferDst::NationBuild(_) => {

                    let _ = pool.try_add(t.kind, t.amount)?;
                }
            }
            let _ = nation_id;
        }
    }
    Ok(pool.get(t.kind))
}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pool_is_all_zero() {
        let p = GlobalResourcePool::new();
        for k in ResourceKind::ALL {
            assert_eq!(p.get(*k), 0, "new pool should be zero for {}", k.label_zh());
        }
    }

    #[test]
    fn add_then_get_works() {
        let mut p = GlobalResourcePool::new();
        assert_eq!(p.try_add(ResourceKind::Wood, 100), Ok(100));
        assert_eq!(p.get(ResourceKind::Wood), 100);
    }

    #[test]
    fn cannot_exceed_max() {
        let mut p = GlobalResourcePool::new();

        p.try_add(ResourceKind::Wood, 9_999).unwrap();
        let err = p.try_add(ResourceKind::Wood, 2).unwrap_err();
        match err {
            PoolError::WouldExceedMax { kind, cur, amount, max } => {
                assert_eq!(kind, ResourceKind::Wood);
                assert_eq!(cur, 9_999);
                assert_eq!(amount, 2);
                assert_eq!(max, 10_000);
            }
            _ => panic!("expected WouldExceedMax, got {:?}", err),
        }
    }

    #[test]
    fn cannot_subtract_more_than_have() {
        let mut p = GlobalResourcePool::new();
        p.try_add(ResourceKind::Apple, 5).unwrap();
        let err = p.try_sub(ResourceKind::Apple, 10).unwrap_err();
        assert!(matches!(err, PoolError::Insufficient { .. }));
    }

    #[test]
    fn all_resource_maxes_match_doc() {

        assert_eq!(ResourceKind::Wood.max(), 10_000);
        assert_eq!(ResourceKind::HardenedWood.max(), 500);
        assert_eq!(ResourceKind::Apple.max(), 5_000);
        assert_eq!(ResourceKind::WheatSeeds.max(), 1_000);
        assert_eq!(ResourceKind::Carrot.max(), 2_000);
        assert_eq!(ResourceKind::Potato.max(), 2_000);
        assert_eq!(ResourceKind::Food.max(), 20_000);
        assert_eq!(ResourceKind::Soul.max(), 1_000);
        assert_eq!(ResourceKind::Sunstone.max(), 200);
        assert_eq!(ResourceKind::Frostcore.max(), 200);
        assert_eq!(ResourceKind::LivingRoot.max(), 200);
        assert_eq!(ResourceKind::VoidEssence.max(), 100);
        assert_eq!(ResourceKind::GripOfFirelord.max(), 50);
        assert_eq!(ResourceKind::EyeOfTheDeep.max(), 10);
        assert_eq!(ResourceKind::PhoenixFeather.max(), 10);
    }

    #[test]
    fn all_25_resources_present() {

        assert_eq!(ResourceKind::ALL.len(), 32);
    }

    #[test]
    fn demo_initial_amount_never_exceeds_resource_max() {
        for k in ResourceKind::ALL {
            let init = k.demo_initial_amount();
            assert!(init > 0, "demo init should seed {}", k.label_zh());
            assert!(
                init <= k.max(),
                "demo init for {} should stay <= max, got {} > {}",
                k.label_zh(),
                init,
                k.max()
            );
        }
        assert_eq!(ResourceKind::SovereignSpark.demo_initial_amount(), 3);
    }

    #[test]
    fn conservation_audit_works() {
        let mut p = GlobalResourcePool::new();

        p.try_add(ResourceKind::Wood, 1000).unwrap();
        p.try_sub(ResourceKind::Wood, 300).unwrap();
        p.try_sub(ResourceKind::Wood, 200).unwrap();
        p.try_add(ResourceKind::Wood, 500).unwrap();

        assert_eq!(p.get(ResourceKind::Wood), 1_000);

        assert_eq!(p.audit_added.get(&ResourceKind::Wood), Some(&1_500));
        assert_eq!(p.audit_subtracted.get(&ResourceKind::Wood), Some(&500));
        p.verify_conservation().expect("pool should be conserved");
    }

    #[test]
    fn conservation_fails_on_overdraw() {
        let mut p = GlobalResourcePool::new();
        p.try_add(ResourceKind::Food, 100).unwrap();
        p.try_sub(ResourceKind::Food, 50).unwrap();
        p.try_sub(ResourceKind::Food, 60).unwrap_err();
        p.verify_conservation().expect("failed sub shouldn't break conservation");
    }

    #[test]
    fn transfer_waste_removes_from_pool() {
        let mut p = GlobalResourcePool::new();
        p.try_add(ResourceKind::Wood, 100).unwrap();

        let t = Transfer {
            kind: ResourceKind::Wood,
            amount: 30,
            src: TransferSrc::Nation(0),
            dst: TransferDst::Wasted,
        };


        apply_transfer(&mut p, t).unwrap();
        assert_eq!(p.get(ResourceKind::Wood), 70);
    }

    #[test]
    fn regen_via_force_add_increases() {
        let mut p = GlobalResourcePool::new();

        apply_transfer(
            &mut p,
            Transfer {
                kind: ResourceKind::Apple,
                amount: 5,
                src: TransferSrc::Regen,
                dst: TransferDst::Wasted,
            },
        )
        .unwrap();
        assert_eq!(p.get(ResourceKind::Apple), 5);
    }

    #[test]
    fn cannot_force_add_past_max_during_init() {

        let mut p = GlobalResourcePool::new();
        p.force_add(ResourceKind::Sunstone, 200);

        p.force_add(ResourceKind::Sunstone, 1);

        let err = p.verify_conservation().unwrap_err();
        assert!(
            err.contains("阳炎石") && err.contains("> max"),
            "expected '阳炎石' and '> max' in error, got: {}",
            err
        );
    }
}
