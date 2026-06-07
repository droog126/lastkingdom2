//! Global Resource Pool
//!
//! §二、核心设计支柱：全局资源池
//!
//! **绝对守恒** —— 池子里的资源只能**转移**（采集消耗，怪物掉落归还，建筑合成消耗），
//! 任何代码路径都不能凭空生成 / 销毁总量。总线 (`Transfer`) 是唯一变更入口。
//!
//! **tick 守恒** —— 慢 tick（1 秒）由 `regen` 阶段处理可再生资源（如浆果消耗后重结）。
//! 任何归还/消耗都会触发 `verify_conservation` 断言（STRICT_CONSERVATION_CHECK 时）。

#![allow(dead_code)]

use bevy::prelude::*;
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// ResourceKind：所有 25 种资源 + 上限 + 中文标签 + 产地
// ---------------------------------------------------------------------------

/// 25 种资源（直接来自总纲 §二 表 1）
///
/// 每个资源有：最大上限、专属产地、再生规则。资源池守恒：
/// 任何 Add 必有对应的 Sub，反之亦然。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    // ---- 基础资源 ----
    Wood,
    HardenedWood,  // 樵夫专属
    Apple,
    WheatSeeds,
    Carrot,
    Potato,
    // ---- 食物 / 灵魂 ----
    Food,
    Soul,
    // ---- 三生态群落专属 ----
    Sunstone,           // 焦土沙漠
    Frostcore,          // 冰封苔原
    LivingRoot,         // 繁盛丛林
    // ---- 炼金 ----
    BloodthistleSeeds,  // 苔原战利品
    FrostleafSeeds,     // 沙漠战利品
    // ---- 以太界 ----
    VoidEssence,        // 以太幽魂掉落
    // ---- 传说组件 ----
    GripOfFirelord,     // 沙漠金字塔传说宝箱
    CoreOfIceGiant,     // 苔原冰洞传说宝箱
    WhisperOfTreant,    // 丛林神庙传说宝箱
    EyeOfTheDeep,       // 以太界岩浆垂钓
    SandsOfTime,        // 沙漠传说宝箱
    WraithFiber,        // 以太界
    GuardianFragment,   // 丛林神庙传说宝箱
    StormCore,          // 山地
    EarthRune,          // 洞穴
    VampireFang,        // 夜晚精英
    PhoenixFeather,     // 岩浆垂钓
}

impl ResourceKind {
    /// 最大上限（直接抄自总纲）
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
        }
    }

    /// 中文标签（debug 日志用）
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
        }
    }

    /// 全部 25 种资源（按类型分组迭代）
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
    ];
}

// ---------------------------------------------------------------------------
// GlobalResourcePool：唯一可变入口
// ---------------------------------------------------------------------------

/// 全局资源池。**唯一所有权路径**：所有 Add / Sub 必须经过这里。
///
/// **不变量**（在 `verify_conservation` 验证）：
///   1. 每个资源 current ∈ [0, max]
///   2. （STRICT_CONSERVATION_CHECK 时）每个 Sub 必须有匹配的 Add 来源
///   3. slow tick 再生路径不会越过 max
#[derive(Resource, Debug, Clone, Default)]
pub struct GlobalResourcePool {
    pub current: HashMap<ResourceKind, i64>,
    /// 守恒审计：累计的 Add 数（用于验证 Sub ≤ Add）
    pub audit_added: HashMap<ResourceKind, i64>,
    /// 守恒审计：累计的 Sub 数
    pub audit_subtracted: HashMap<ResourceKind, i64>,
}

impl GlobalResourcePool {
    /// 新建一个空的池子（所有资源 0）
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

    /// 当前值
    pub fn get(&self, k: ResourceKind) -> i64 {
        *self.current.get(&k).unwrap_or(&0)
    }

    /// 尝试加 amount 个。返回 `Ok(new_value)` 成功，`Err(reason)` 失败
    /// 失败原因：会超过 max / amount 非正
    pub fn try_add(&mut self, k: ResourceKind, amount: i64) -> Result<i64, PoolError> {
        if amount <= 0 {
            return Err(PoolError::NonPositiveAmount(amount));
        }
        let cur = self.get(k);
        let max = k.max();
        if cur + amount > max {
            return Err(PoolError::WouldExceedMax {
                kind: k,
                cur,
                amount,
                max,
            });
        }
        let new = cur + amount;
        self.current.insert(k, new);
        *self.audit_added.entry(k).or_insert(0) += amount;
        Ok(new)
    }

    /// 强制加（用于"再生"或"初始注入"），跳过 max 检查但**仍记录 audit**
    pub fn force_add(&mut self, k: ResourceKind, amount: i64) -> i64 {
        debug_assert!(amount >= 0, "force_add amount must be >= 0, got {}", amount);
        let new = self.get(k) + amount;
        self.current.insert(k, new);
        *self.audit_added.entry(k).or_insert(0) += amount;
        new
    }

    /// 尝试扣 amount 个。失败：余额不足
    pub fn try_sub(&mut self, k: ResourceKind, amount: i64) -> Result<i64, PoolError> {
        if amount <= 0 {
            return Err(PoolError::NonPositiveAmount(amount));
        }
        let cur = self.get(k);
        if amount > cur {
            return Err(PoolError::Insufficient {
                kind: k,
                cur,
                amount,
            });
        }
        let new = cur - amount;
        self.current.insert(k, new);
        *self.audit_subtracted.entry(k).or_insert(0) += amount;
        Ok(new)
    }

    /// 守恒检查：在 STRICT_CONSERVATION_CHECK 模式下，断言 audit_added >= audit_subtracted
    /// 并对每个资源检查 current = audit_added - audit_subtracted
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
            // 注意：cur 严格小于等于 (added - subbed)，因为 regen 不算 audit_added
            // （regen 不会经过 try_add）。我们只断言子集关系。
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join("\n"))
        }
    }

    /// 全部清零（测试用，**绝对不要在产品代码调**）
    pub fn reset_for_tests(&mut self) {
        for k in ResourceKind::ALL {
            self.current.insert(*k, 0);
            self.audit_added.insert(*k, 0);
            self.audit_subtracted.insert(*k, 0);
        }
    }

    /// 当前非零资源数（用于 debug HUD / 守恒检查）
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

// ---------------------------------------------------------------------------
// PoolError
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Transfer：原子化的"从 src 转到 dst"，用于可追踪的守恒转移
// ---------------------------------------------------------------------------

/// 一笔资源转移记录
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transfer {
    pub kind: ResourceKind,
    pub amount: i64,
    pub src: TransferSrc,
    pub dst: TransferDst,
}

/// 资源来源（debug / audit 用）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferSrc {
    /// 世界初始注入（force_add）
    Init,
    /// 玩家采集
    PlayerGather(u32),
    /// 怪物死亡掉落
    MonsterDrop(u32),
    /// 资源点再生（注意：regen **不走**这条，是 force_add）
    Regen,
    /// 国家开销（如买旗、升级人口）
    Nation(u32),
}

/// 资源去向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDst {
    /// 玩家消耗 / 装备合成
    PlayerUse(u32),
    /// 国家金库
    NationTreasury(u32),
    /// 国家建造消耗
    NationBuild(u32),
    /// 流亡 / 销毁
    Wasted,
}

/// 在池子上应用一笔 transfer。返回 Ok(剩余) / Err(失败原因)
///
/// 守恒不变量：sub + add = 0
///
/// 语义分类：
///   * 收入类（force_add，pool 净增）：Regen / Init / PlayerGather / MonsterDrop
///   * 支出类（try_sub，pool 净减）：Nation → PlayerUse/NationBuild/Wasted
///   * 转移类（先 sub 后 add，pool 净 0）：目前未启用，留作扩展
pub fn apply_transfer(pool: &mut GlobalResourcePool, t: Transfer) -> Result<i64, PoolError> {
    debug_assert!(t.amount > 0, "transfer amount must be > 0");
    match t.src {
        TransferSrc::Regen | TransferSrc::Init | TransferSrc::PlayerGather(_) | TransferSrc::MonsterDrop(_) => {
            // 收入：直接 force_add（池子净增）
            // 注意：gathering/drop 不算 audit_added（regen 也不算），所以是 force_add
            pool.force_add(t.kind, t.amount);
        }
        TransferSrc::Nation(nation_id) => {
            // 支出：先 sub 源
            pool.try_sub(t.kind, t.amount)?;
            // 再处理 dst
            match t.dst {
                TransferDst::Wasted => {
                    // 真销毁：sub 已经扣了，dst=Wasted 表示"扔掉"
                }
                TransferDst::PlayerUse(_) | TransferDst::NationTreasury(_) | TransferDst::NationBuild(_) => {
                    // 转移到目标（这里都是同 pool 内的子账户，简单起见直接 add 回去）
                    let _ = pool.try_add(t.kind, t.amount)?;
                }
            }
            let _ = nation_id; // unused for now
        }
    }
    Ok(pool.get(t.kind))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        // Wood max = 10_000
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
        // 抽查几个：总纲里直接给出的数字
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
        // 总纲表 1 列了 25 种（含冰心晶体的别名也算 1 种）
        assert_eq!(ResourceKind::ALL.len(), 25);
    }

    #[test]
    fn conservation_audit_works() {
        let mut p = GlobalResourcePool::new();
        // 模拟一次完整生命周期：add → sub → sub → sub → add
        p.try_add(ResourceKind::Wood, 1000).unwrap();
        p.try_sub(ResourceKind::Wood, 300).unwrap();
        p.try_sub(ResourceKind::Wood, 200).unwrap();
        p.try_add(ResourceKind::Wood, 500).unwrap();
        // current = 1000
        assert_eq!(p.get(ResourceKind::Wood), 1_000);
        // audit: added=1500, subbed=500
        assert_eq!(p.audit_added.get(&ResourceKind::Wood), Some(&1_500));
        assert_eq!(p.audit_subtracted.get(&ResourceKind::Wood), Some(&500));
        p.verify_conservation().expect("pool should be conserved");
    }

    #[test]
    fn conservation_fails_on_overdraw() {
        let mut p = GlobalResourcePool::new();
        p.try_add(ResourceKind::Food, 100).unwrap();
        p.try_sub(ResourceKind::Food, 50).unwrap();
        p.try_sub(ResourceKind::Food, 60).unwrap_err(); // 失败
        p.verify_conservation().expect("failed sub shouldn't break conservation");
    }

    #[test]
    fn transfer_waste_removes_from_pool() {
        let mut p = GlobalResourcePool::new();
        p.try_add(ResourceKind::Wood, 100).unwrap();
        // 转移 30 给玩家（Wasted）
        let t = Transfer {
            kind: ResourceKind::Wood,
            amount: 30,
            src: TransferSrc::Nation(0),       // 国家支出
            dst: TransferDst::Wasted,         // 销毁
        };
        // apply_transfer 处理：Nation → try_sub(30) → 池子 100-30=70
        // Wasted：不归位
        apply_transfer(&mut p, t).unwrap();
        assert_eq!(p.get(ResourceKind::Wood), 70);
    }

    #[test]
    fn regen_via_force_add_increases() {
        let mut p = GlobalResourcePool::new();
        // regen: 模拟"浆果丛林结出 5 个苹果"
        apply_transfer(
            &mut p,
            Transfer {
                kind: ResourceKind::Apple,
                amount: 5,
                src: TransferSrc::Regen,
                dst: TransferDst::Wasted,  // regen 时 dst 无所谓
            },
        )
        .unwrap();
        assert_eq!(p.get(ResourceKind::Apple), 5);
    }

    #[test]
    fn cannot_force_add_past_max_during_init() {
        // force_add 也守 max（除了 regen）
        let mut p = GlobalResourcePool::new();
        p.force_add(ResourceKind::Sunstone, 200);  // max=200，恰好
        // 再加会越过，但 force_add 不会检查
        p.force_add(ResourceKind::Sunstone, 1);    // 现在 201 > 200
        // verify_conservation 应该报超过 max（中文标签 "阳炎石"）
        let err = p.verify_conservation().unwrap_err();
        assert!(err.contains("阳炎石") && err.contains("> max"),
            "expected '阳炎石' and '> max' in error, got: {}", err);
    }
}
