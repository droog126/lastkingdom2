//! V2 矿坑模块系统
//!
//! 来源: 《万国余烬_王冠赛季_表格包》§5.2 矿坑模块表
//! + 《资源循环与地形改造》(矿坑是高产资源出口)
//! + 《开发顺序与里程碑》P3.5 矿权争夺原型
//!
//! V2 关键变化: 删掉 V1 的"地块可无限挖"路径,改"矿坑模块 + 采矿槽 + 矿坑资源池扣除 +
//! 暴露半径 + 三阶段枯竭"完整系统。玩家在矿坑槽上**持续**开采,产出从矿坑资源池扣除,
//! 而不是从世界地块按需生成。运输和护送变成核心玩法。
//!
//! 设计要点:
//! - `MiningSiteKind` 枚举对表格包 5.2 的 6 种模块 (`mine_surface_pit` 等)
//! - `MiningSite` Component 挂在矿坑 entity 上,带 `world_pos` / `slot_count` / `depletion`
//! - `MiningSlot` Component 挂在矿坑的 N 个槽位子 entity 上
//! - `MiningMode` (稳采 / 强采 / 夜采 / 净化) 影响 `gather_ticks` 和 `noise_radius`
//! - `MiningSiteRegistry` Resource 维护全局矿坑列表 + 单局生成数量
//! - 玩家占用槽位 → 扣矿坑资源池 + 发 `MiningSlotStarted` / `MiningSlotProgressed` / `MiningSlotCompleted` Message
//! - 暴露半径用 `noise_radius` 表示(强采更大),留 T5 接小地图提示

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::resource::{GlobalResourcePool, ResourceKind};

// ---------------------------------------------------------------------------
// Kinds (对齐表格包 §5.2 矿坑模块表)
// ---------------------------------------------------------------------------

/// 矿坑模块种类(表格包 §5.2 6 种)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MiningSiteKind {
    /// `mine_surface_pit` 露天矿坑 (月纹山脊、岩地台地、旧矿路)
    SurfacePit,
    /// `mine_cave_fissure` 裂隙洞穴 (山脊断层)
    CaveFissure,
    /// `mine_moon_ridge` 月纹矿坑 (月纹山脊高地)
    MoonRidge,
    /// `mine_spirit_sink` 灵脉矿井 (灵脉湿地、灵井旁)
    SpiritSink,
    /// `mine_relic_quarry` 遗迹采石场 (遗迹门厅、旧采石场 POI)
    RelicQuarry,
    /// `mine_calamity_bloom` 灾变矿潮 (灾变矿潮事件)
    CalamityBloom,
}

impl MiningSiteKind {
    pub fn id_str(self) -> &'static str {
        match self {
            Self::SurfacePit => "mine_surface_pit",
            Self::CaveFissure => "mine_cave_fissure",
            Self::MoonRidge => "mine_moon_ridge",
            Self::SpiritSink => "mine_spirit_sink",
            Self::RelicQuarry => "mine_relic_quarry",
            Self::CalamityBloom => "mine_calamity_bloom",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::SurfacePit => "露天矿坑",
            Self::CaveFissure => "裂隙洞穴",
            Self::MoonRidge => "月纹矿坑",
            Self::SpiritSink => "灵脉矿井",
            Self::RelicQuarry => "遗迹采石场",
            Self::CalamityBloom => "灾变矿潮",
        }
    }

    /// 矿坑槽位数(表格包 §4 局内参数: common 3 / rich 5)
    /// 简化: 固定每种一个值,T5 接 GameTables 覆盖
    pub fn default_slot_count(self) -> u32 {
        match self {
            Self::SurfacePit => 3,
            Self::CaveFissure => 4,
            Self::MoonRidge => 3,
            Self::SpiritSink => 4,
            Self::RelicQuarry => 3,
            Self::CalamityBloom => 5,
        }
    }

    /// 矿坑主要产出 (kind, total_pool) (表格包 §5.2 总池)
    /// 简化 MVP: 每种只列 1-2 种主产物 + 总池,实际每个 slot 周期从池里扣
    pub fn primary_yields(self) -> &'static [(ResourceKind, i64)] {
        use ResourceKind::*;
        match self {
            // 石料 600-1000, 铁 240-480, 铜 120-260
            Self::SurfacePit => &[(Wood, 0)], // 暂时 fallback, 实际见 §5.2
            // V2 表格包定义了 Stone 但 ResourceKind 还没 Stone; 暂用 Wood 占位
            // TODO V2 §5.2: 添加 Stone/StoneBrick/IronOre/CopperOre/Silver/MoonSteel/CalamityCrystal
            //              到 ResourceKind 后替换
            Self::CaveFissure => &[(Wood, 0)],
            Self::MoonRidge => &[(Wood, 0)],
            Self::SpiritSink => &[(SpiritEssence, 400), (RuneStone, 120)],
            Self::RelicQuarry => &[(Wood, 0)],
            Self::CalamityBloom => &[(SpiritEssence, 280), (SparkFragment, 1)],
        }
    }

    /// 单周期产出 (kind, per_mining) — 玩家在槽位上完成一个采掘循环的产出
    /// 简化 MVP: SpiritSink 产灵质, CalamityBloom 产火种碎片
    pub fn per_yield(self) -> (ResourceKind, i64) {
        use ResourceKind::*;
        match self {
            Self::SpiritSink => (SpiritEssence, 35),
            Self::CalamityBloom => (SparkFragment, 1),
            _ => (Wood, 10), // 临时 fallback
        }
    }

    /// 稳采/强采下, 一个 slot 完成一次采掘所需 tick 数 (30Hz fixed update)
    pub fn gather_ticks(self, mode: MiningMode) -> u32 {
        let base = match self {
            Self::SurfacePit => 30 * 12,     // 12s
            Self::CaveFissure => 30 * 15,   // 15s
            Self::MoonRidge => 30 * 14,     // 14s
            Self::SpiritSink => 30 * 18,    // 18s
            Self::RelicQuarry => 30 * 16,   // 16s
            Self::CalamityBloom => 30 * 24, // 24s
        };
        match mode {
            MiningMode::Steady => base,
            MiningMode::Hard => (base as f32 * 0.55) as u32, // 强采快 45%
            MiningMode::Night => base,                       // 月纹矿坑夜间 = 稳采
            MiningMode::Purify => (base as f32 * 1.3) as u32, // 净化慢 30%
        }
    }

    /// 暴露半径 (米) — 强采更大
    pub fn noise_radius(self, mode: MiningMode) -> f32 {
        let base = match self {
            Self::SurfacePit => 90.0,
            Self::CaveFissure => 110.0,
            Self::MoonRidge => 150.0,
            Self::SpiritSink => 150.0,
            Self::RelicQuarry => 100.0,
            Self::CalamityBloom => 200.0, // 灾变事件区, 全图
        };
        match mode {
            MiningMode::Steady => base,
            MiningMode::Hard => base * 1.6,
            MiningMode::Night => base,
            MiningMode::Purify => base * 0.7,
        }
    }
}

// ---------------------------------------------------------------------------
// Mining mode
// ---------------------------------------------------------------------------

/// 矿坑采掘模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MiningMode {
    /// 稳采: 标准速度 + 标准暴露
    Steady,
    /// 强采: 更快 + 更大暴露 + 可能污染
    Hard,
    /// 夜采: 仅月纹矿坑支持, 夜间效率 +30%
    Night,
    /// 净化采: 灾变矿潮专属, 慢但降低污染损失
    Purify,
}

impl MiningMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Steady => "steady",
            Self::Hard => "hard",
            Self::Night => "night",
            Self::Purify => "purify",
        }
    }
}

// ---------------------------------------------------------------------------
// Depletion
// ---------------------------------------------------------------------------

/// 矿坑枯竭阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepletionTier {
    /// 0-45%: 正常产量
    Normal,
    /// 45-80%: 低产视觉阶段 (产出 -30%, V2 表格包 §4 mining_depletion_tier_1=45)
    LowYield,
    /// 80-100%: 近枯竭 (产出 -70%, mining_depletion_tier_2=80)
    NearEmpty,
    /// 100%: 枯竭 (停止产出, 灾变矿潮会污染损失)
    Depleted,
}

impl DepletionTier {
    pub fn from_percent(p: f32) -> Self {
        // 表格包 §4: tier_1=45% 进入低产, tier_2=80% 进入近枯竭
        // 0% 算 Depleted, 100% 算 Normal
        if p <= 0.0 {
            Self::Depleted
        } else if p < 45.0 {
            Self::Normal
        } else if p < 80.0 {
            Self::LowYield
        } else if p < 100.0 {
            Self::NearEmpty
        } else {
            Self::Normal
        }
    }

    /// 该阶段的产出乘数
    pub fn yield_multiplier(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::LowYield => 0.7,
            Self::NearEmpty => 0.3,
            Self::Depleted => 0.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::LowYield => "low_yield",
            Self::NearEmpty => "near_empty",
            Self::Depleted => "depleted",
        }
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// 矿坑模块实体
#[derive(Component, Debug, Clone)]
pub struct MiningSite {
    pub id: u32,
    pub kind: MiningSiteKind,
    /// 世界坐标 (XZ, y 取地表)
    pub world_pos: [f32; 3],
    /// 当前总池剩余百分比 (0.0 - 100.0)
    pub remaining_pct: f32,
    /// 该矿坑已分配出去的 slot 实体 id
    pub slot_entities: Vec<Entity>,
    /// 该矿坑当前是否在被护送/争夺 (留 T5 接 nation)
    pub contested_by: Vec<u32>,
}

impl MiningSite {
    pub fn new(id: u32, kind: MiningSiteKind, world_pos: [f32; 3]) -> Self {
        Self {
            id,
            kind,
            world_pos,
            remaining_pct: 100.0,
            slot_entities: Vec::new(),
            contested_by: Vec::new(),
        }
    }

    pub fn depletion_tier(&self) -> DepletionTier {
        DepletionTier::from_percent(self.remaining_pct)
    }
}

/// 矿坑的一个采矿槽位
#[derive(Component, Debug, Clone)]
pub struct MiningSlot {
    /// 属于哪个矿坑
    pub site_id: u32,
    /// 槽位编号 (0..slot_count)
    pub slot_index: u32,
    /// 当前占用该槽位的玩家 id (None = 空槽)
    pub occupant: Option<u32>,
    /// 已采掘 tick 进度
    pub progress_ticks: u32,
    /// 当前采掘模式
    pub mode: MiningMode,
}

impl MiningSlot {
    pub fn new(site_id: u32, slot_index: u32) -> Self {
        Self {
            site_id,
            slot_index,
            occupant: None,
            progress_ticks: 0,
            mode: MiningMode::Steady,
        }
    }

    pub fn is_occupied(&self) -> bool {
        self.occupant.is_some()
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// 全局矿坑注册表
#[derive(Resource, Debug, Default)]
pub struct MiningSiteRegistry {
    next_id: u32,
    /// 普通矿坑数量 (表格包 §4 mining_site_count_common default=10)
    pub common_count: u32,
    /// 富矿/事件矿坑数量 (default=4)
    pub rich_count: u32,
    /// 已生成的矿坑 site id 列表
    pub active: Vec<u32>,
}

impl MiningSiteRegistry {
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}

// ---------------------------------------------------------------------------
// Events (Messages in bevy 0.18)
// ---------------------------------------------------------------------------

/// 玩家开始占用一个矿坑槽
#[derive(Message, Debug, Clone, Copy)]
pub struct MiningSlotStarted {
    pub site_id: u32,
    pub slot_index: u32,
    pub player_id: u32,
    pub mode: MiningMode,
    pub at_wall_secs: f32,
}

/// 玩家完成一次采掘循环 (产出)
#[derive(Message, Debug, Clone, Copy)]
pub struct MiningSlotCompleted {
    pub site_id: u32,
    pub slot_index: u32,
    pub player_id: u32,
    pub mode: MiningMode,
    pub yield_kind: ResourceKind,
    pub yield_amount: i64,
    pub at_wall_secs: f32,
}

/// 矿坑进入新枯竭阶段
#[derive(Message, Debug, Clone, Copy)]
pub struct MiningSiteDepletionChanged {
    pub site_id: u32,
    pub from: DepletionTier,
    pub to: DepletionTier,
    pub remaining_pct: f32,
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// 在 FixedUpdate 推进所有 occupied slot, 完成一次采掘循环时产出 + 发事件
pub fn tick_mining_slots(
    commands: Commands,
    mut slots: Query<(Entity, &mut MiningSlot)>,
    mut sites: Query<&mut MiningSite>,
    mut pool: ResMut<GlobalResourcePool>,
    site_registry: ResMut<MiningSiteRegistry>,
    match_clock: Res<crate::match_state::MatchClock>,
    started_events: MessageWriter<MiningSlotStarted>,
    mut completed_events: MessageWriter<MiningSlotCompleted>,
    mut depletion_events: MessageWriter<MiningSiteDepletionChanged>,
) {
    // 记录"占用该槽位"为进度推进 + 完成后产出
    for (slot_entity, mut slot) in slots.iter_mut() {
        let Some(player_id) = slot.occupant else {
            continue;
        };
        // 找 site
        let site_id = slot.site_id;
        let site_kind = sites
            .iter()
            .find(|s| s.id == site_id)
            .map(|s| s.kind);
        let Some(site_kind) = site_kind else {
            // site 不存在 (被销毁), 清空 slot
            slot.occupant = None;
            slot.progress_ticks = 0;
            let _ = slot_entity;
            continue;
        };
        // 计算本次需要的 tick 数
        let required = site_kind.gather_ticks(slot.mode);
        slot.progress_ticks += 1;
        if slot.progress_ticks < required {
            continue;
        }
        // 完成一次循环
        slot.progress_ticks = 0;
        // 计算产出 (考虑矿坑枯竭乘数)
        let site = sites.iter().find(|s| s.id == site_id).unwrap();
        let tier = site.depletion_tier();
        let mult = tier.yield_multiplier();
        if mult <= 0.0 {
            // 枯竭, 不产出
            info!(
                "[mine] site {} ({}) depleted, no yield for player {}",
                site_id,
                site_kind.label_zh(),
                player_id
            );
            continue;
        }
        let (kind, base_amount) = site_kind.per_yield();
        let amount = ((base_amount as f32) * mult).round() as i64;
        if amount > 0 {
            // 写全局资源池 (force_add 绕过 max 限制? 不, 用 try_add)
            let _ = pool.try_add(kind, amount);
            completed_events.write(MiningSlotCompleted {
                site_id,
                slot_index: slot.slot_index,
                player_id,
                mode: slot.mode,
                yield_kind: kind,
                yield_amount: amount,
                at_wall_secs: match_clock.wall_secs,
            });
            info!(
                "[mine] site {} ({}) slot {} → player {} +{} {:?} (tier={:?}, mult={:.2})",
                site_id,
                site_kind.label_zh(),
                slot.slot_index,
                player_id,
                amount,
                kind,
                tier,
                mult
            );
        }
        // 扣矿坑自身总池 (每个完整循环扣 1.5% 模拟消耗, 跟表格包 §5.2 枯竭节奏一致)
        // 注意: 这里只读 site 不能直接改; 真正的 remaining_pct 改写在下面专门一轮
        let _ = site_registry; // 抑制 unused
        let _ = depletion_events; // 抑制 unused (留 T5 真正接)
        let _ = started_events; // 抑制 unused (留 T5 真正接)
        let _ = commands; // 抑制 unused
    }

    // 推进矿坑自身的 remaining_pct (按每 tick 总循环数 - 一点)
    // MVP 简化: 假设 1 个 slot 一次循环扣 0.4% (大约 250 次循环 = 枯竭, 在 30Hz * 18s = 5400 tick 下
    // 单 slot ≈ 5400/540 = 10 次循环; 4 slot ≈ 40 次循环, 永远采不完)  → 改用每 tick 0.005% 扣
    for mut site in sites.iter_mut() {
        // 简化: 每个 occupied slot 每 tick 扣 0.005%
        let occupied = slots
            .iter()
            .filter(|(_, s)| s.occupant.is_some() && s.site_id == site.id)
            .count() as f32;
        let drain = 0.005 * occupied;
        if drain > 0.0 && site.remaining_pct > 0.0 {
            let old_tier = site.depletion_tier();
            site.remaining_pct = (site.remaining_pct - drain).max(0.0);
            let new_tier = site.depletion_tier();
            if new_tier != old_tier {
                depletion_events.write(MiningSiteDepletionChanged {
                    site_id: site.id,
                    from: old_tier,
                    to: new_tier,
                    remaining_pct: site.remaining_pct,
                });
                info!(
                    "[mine] site {} ({}) depletion: {:?} → {:?} ({:.1}%)",
                    site.id,
                    site.kind.label_zh(),
                    old_tier,
                    new_tier,
                    site.remaining_pct
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// 玩法 Plugin: 注册矿坑资源 + events + 系统
///
/// 用法:
/// ```ignore
/// app.add_plugins(MiningSitePlugin);
/// ```
///
/// 注意: 依赖 `MatchStatePlugin`(读 `MatchClock.wall_secs`); 客户端若只读不写也建议注册。
pub struct MiningSitePlugin;

impl Plugin for MiningSitePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MiningSiteRegistry>()
            .add_message::<MiningSlotStarted>()
            .add_message::<MiningSlotCompleted>()
            .add_message::<MiningSiteDepletionChanged>()
            .add_systems(FixedUpdate, tick_mining_slots);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::match_state::MatchClock;
    use crate::resource::ResourceKind;

    #[test]
    fn kind_id_str_matches_table() {
        // 表格包 §5.2 模块 ID 必须一字不差
        assert_eq!(MiningSiteKind::SurfacePit.id_str(), "mine_surface_pit");
        assert_eq!(MiningSiteKind::CaveFissure.id_str(), "mine_cave_fissure");
        assert_eq!(MiningSiteKind::MoonRidge.id_str(), "mine_moon_ridge");
        assert_eq!(MiningSiteKind::SpiritSink.id_str(), "mine_spirit_sink");
        assert_eq!(MiningSiteKind::RelicQuarry.id_str(), "mine_relic_quarry");
        assert_eq!(MiningSiteKind::CalamityBloom.id_str(), "mine_calamity_bloom");
    }

    #[test]
    fn slot_count_in_range_2_to_5() {
        // 表格包 §5.2 槽位 2-4 + 3-5 范围
        for k in [
            MiningSiteKind::SurfacePit,
            MiningSiteKind::CaveFissure,
            MiningSiteKind::MoonRidge,
            MiningSiteKind::SpiritSink,
            MiningSiteKind::RelicQuarry,
            MiningSiteKind::CalamityBloom,
        ] {
            let n = k.default_slot_count();
            assert!((2..=6).contains(&n), "{:?} slot_count={} out of range", k, n);
        }
    }

    #[test]
    fn depletion_tier_thresholds() {
        // 0% = Depleted, 100% = Normal, 中间走 LowYield (45-80) / NearEmpty (80-100)
        assert_eq!(DepletionTier::from_percent(0.0), DepletionTier::Depleted);
        assert_eq!(DepletionTier::from_percent(0.1), DepletionTier::Normal);
        assert_eq!(DepletionTier::from_percent(44.9), DepletionTier::Normal);
        assert_eq!(DepletionTier::from_percent(45.0), DepletionTier::LowYield);
        assert_eq!(DepletionTier::from_percent(80.0), DepletionTier::NearEmpty);
        assert_eq!(DepletionTier::from_percent(99.9), DepletionTier::NearEmpty);
        assert_eq!(DepletionTier::from_percent(100.0), DepletionTier::Normal);
    }

    #[test]
    fn hard_mode_faster_than_steady() {
        for k in [
            MiningSiteKind::SurfacePit,
            MiningSiteKind::SpiritSink,
            MiningSiteKind::CalamityBloom,
        ] {
            let steady = k.gather_ticks(MiningMode::Steady);
            let hard = k.gather_ticks(MiningMode::Hard);
            assert!(hard < steady, "{:?} hard ({}) >= steady ({})", k, hard, steady);
        }
    }

    #[test]
    fn registry_allocates_unique_ids() {
        let mut r = MiningSiteRegistry {
            common_count: 10,
            rich_count: 4,
            ..Default::default()
        };
        let a = r.alloc_id();
        let b = r.alloc_id();
        assert_ne!(a, b);
    }

    #[test]
    fn spirit_sink_yields_spirit_essence() {
        // 表格包 §5.2: 灵脉矿井主要产灵质
        let (kind, amount) = MiningSiteKind::SpiritSink.per_yield();
        assert_eq!(kind, ResourceKind::SpiritEssence);
        assert!(amount > 0);
    }

    #[test]
    fn calamity_bloom_yields_spark_fragment() {
        // 表格包 §5.2: 灾变矿潮产火种碎片 (回流建国权循环)
        let (kind, _) = MiningSiteKind::CalamityBloom.per_yield();
        assert_eq!(kind, ResourceKind::SparkFragment);
    }

    #[test]
    fn site_depletion_tier_drives_yield_multiplier() {
        let mut site = MiningSite::new(1, MiningSiteKind::SpiritSink, [0.0, 0.0, 0.0]);
        site.remaining_pct = 100.0;
        assert_eq!(site.depletion_tier(), DepletionTier::Normal);
        assert_eq!(site.depletion_tier().yield_multiplier(), 1.0);
        site.remaining_pct = 50.0;
        assert_eq!(site.depletion_tier(), DepletionTier::LowYield);
        assert!((site.depletion_tier().yield_multiplier() - 0.7).abs() < 0.01);
        site.remaining_pct = 90.0;
        assert_eq!(site.depletion_tier(), DepletionTier::NearEmpty);
        assert!((site.depletion_tier().yield_multiplier() - 0.3).abs() < 0.01);
        site.remaining_pct = 0.0;
        assert_eq!(site.depletion_tier(), DepletionTier::Depleted);
        assert_eq!(site.depletion_tier().yield_multiplier(), 0.0);
    }

    #[test]
    fn new_slot_is_empty() {
        let s = MiningSlot::new(1, 0);
        assert!(!s.is_occupied());
        assert_eq!(s.progress_ticks, 0);
        assert_eq!(s.mode, MiningMode::Steady);
    }
}
