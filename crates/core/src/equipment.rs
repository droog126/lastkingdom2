//! V2 装备系统 — 7 装备槽 + 5 品质 + 耐久状态机
//!
//! 来源: 《万国余烬_王冠赛季_表格包》§18 装备系统详表
//!
//! ## 设计原则
//!
//! 装备系统只服务单局内的战术分工。装备不能从经营世界带入公开赛;
//! 死亡时装备按物品表掉落;装备强度来自局内资源、路线和风险, 不来自长期养成。
//!
//! ## 7 装备槽 (对齐 §18.1)
//!
//! | 槽位ID | 槽位 | 可装备类型 | 默认数量 | 主要影响 | 限制 |
//! | --- | --- | --- | --- | --- | --- |
//! | `slot_main_hand` | 主手 | 剑、工具、法器 | 1 | 攻击/采集/施法动作 | 手持大件时不可用 |
//! | `slot_off_hand` | 副手 | 盾/法器/轻工具 | 1 | 格挡/结界/施法稳定 | 双手搬运时不可用 |
//! | `slot_body` | 身体 | 行衣/短袍/仪卫甲 | 1 | 减伤/负重/疾跑蓄势 | 重甲限制秘剑和法术 |
//! | `slot_cloak` | 披肩 | 结界披肩/月纱 | 1 | 法术抗性/灵视 | 与重甲效果递减 |
//! | `slot_back` | 背部 | 背篓/束带/鞍具包 | 1 | 背包容量/负重 | 与肩扛大件叠加惩罚 |
//! | `slot_focus` | 法术焦点 | 法术环/调灵器 | 1 | MP/吟唱/法术学派 | 需解锁 |
//! | `slot_relic` | 临时遗物 | 小型遗物/王座碎片 | 1 | 事件交互/秘仪加成 | 全局唯一神器不进此槽 |
//!
//! ## 5 品质 (对齐 §18.2)
//!
//! | ID | 名称 | 数值修正 | 耐久修正 |
//! | --- | --- | --- | --- |
//! | q_crude | 粗制 | 无 | -20% |
//! | q_crafted | 制式 | 无 | 无 |
//! | q_refined | 精制 | +8% | +15% |
//! | q_arcane | 秘仪 | +12% | +10% |
//! | q_unique | 唯一 | 规则专属 | 不使用普通耐久 |
//!
//! ## MVP 边界
//!
//! - 不接 §18.3 装备词条表 (剑具/防具具体数据 — 留 GameTables)
//! - 不接死亡掉落 (留 scenario)
//! - 不接 unique 神器规则 (留 T6+ 范围)
//! - 不接装备实例的具体 mesh / 渲染 (留 client render)
//! - 提供 Component / 耐久状态机 / equip API

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Slot (7 种)
// ---------------------------------------------------------------------------

/// 7 个装备槽位 (对齐表格包 §18.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    /// `slot_main_hand` 主手 — 剑、工具、法器
    MainHand,
    /// `slot_off_hand` 副手 — 盾、法器、轻工具
    OffHand,
    /// `slot_body` 身体 — 行衣、短袍、仪卫甲
    Body,
    /// `slot_cloak` 披肩 — 结界披肩、月纱、根须披帛
    Cloak,
    /// `slot_back` 背部 — 背篓、束带、鞍具包
    Back,
    /// `slot_focus` 法术焦点 — 法术环、调灵器、符文坠
    Focus,
    /// `slot_relic` 临时遗物 — 小型遗物、王座碎片
    Relic,
}

impl EquipmentSlot {
    /// 表格包 §18.1 槽位 ID
    pub fn id_str(self) -> &'static str {
        match self {
            Self::MainHand => "slot_main_hand",
            Self::OffHand => "slot_off_hand",
            Self::Body => "slot_body",
            Self::Cloak => "slot_cloak",
            Self::Back => "slot_back",
            Self::Focus => "slot_focus",
            Self::Relic => "slot_relic",
        }
    }

    /// 中文标签
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::MainHand => "主手",
            Self::OffHand => "副手",
            Self::Body => "身体",
            Self::Cloak => "披肩",
            Self::Back => "背部",
            Self::Focus => "法术焦点",
            Self::Relic => "临时遗物",
        }
    }

    /// 所有 7 槽位 (固定顺序,UI 显示用)
    pub const ALL: [EquipmentSlot; 7] = [
        Self::MainHand,
        Self::OffHand,
        Self::Body,
        Self::Cloak,
        Self::Back,
        Self::Focus,
        Self::Relic,
    ];
}

// ---------------------------------------------------------------------------
// Quality (5 种)
// ---------------------------------------------------------------------------

/// 装备品质 (对齐表格包 §18.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentQuality {
    /// `q_crude` 粗制 — 手工、荒野箱
    Crude,
    /// `q_crafted` 制式 — 营地制作
    Crafted,
    /// `q_refined` 精制 — 国家设施、路线奖励
    Refined,
    /// `q_arcane` 秘仪 — 遗迹、灵脉、剑阁
    Arcane,
    /// `q_unique` 唯一 — 全局唯一神器 (规则专属,不走普通耐久)
    Unique,
}

impl EquipmentQuality {
    pub fn id_str(self) -> &'static str {
        match self {
            Self::Crude => "q_crude",
            Self::Crafted => "q_crafted",
            Self::Refined => "q_refined",
            Self::Arcane => "q_arcane",
            Self::Unique => "q_unique",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Crude => "粗制",
            Self::Crafted => "制式",
            Self::Refined => "精制",
            Self::Arcane => "秘仪",
            Self::Unique => "唯一",
        }
    }

    /// 数值修正 (主属性百分比加成)
    pub fn stat_multiplier(self) -> f32 {
        match self {
            Self::Crude => 1.0,
            Self::Crafted => 1.0,
            Self::Refined => 1.08,
            Self::Arcane => 1.12,
            Self::Unique => 1.0, // 规则专属, 走特殊 path
        }
    }

    /// 耐久修正 (百分比加成;Crude -20% / Refined +15% / Arcane +10%)
    pub fn durability_multiplier(self) -> f32 {
        match self {
            Self::Crude => 0.80,
            Self::Crafted => 1.0,
            Self::Refined => 1.15,
            Self::Arcane => 1.10,
            Self::Unique => 1.0, // Unique 不走普通耐久
        }
    }

    /// Unique 装备不使用普通耐久状态机
    pub fn uses_normal_durability(self) -> bool {
        !matches!(self, Self::Unique)
    }
}

// ---------------------------------------------------------------------------
// Durability 状态机
// ---------------------------------------------------------------------------

/// 耐久状态 (表格包 §18.2: 25% 进入受损, 0% 进入破裂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurabilityState {
    /// 25%-100%: 正常
    Normal,
    /// 1%-25%: 受损 — 武器结构伤害 -20% / 盾牌格挡 -20% / 护甲减伤 -30%
    Damaged,
    /// 0%: 破裂 — 主要效果失效,可拆解回收部分材料
    Broken,
}

impl DurabilityState {
    /// 从当前耐久 / max 算状态
    pub fn from_percent(p: f32) -> Self {
        if p <= 0.0 {
            Self::Broken
        } else if p < 0.25 {
            Self::Damaged
        } else {
            Self::Normal
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Damaged => "damaged",
            Self::Broken => "broken",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Normal => "正常",
            Self::Damaged => "受损",
            Self::Broken => "破裂",
        }
    }

    /// 该状态下, 武器 / 盾 / 护甲的伤害 / 格挡 / 减伤乘数
    /// (表格包 §18.2 末尾: 受损 -20% 武器 / -20% 盾 / -30% 护甲)
    /// 简化: 取平均 -25% 乘数, 实际 GameTables 会按装备分类细分
    pub fn effectiveness(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Damaged => 0.75,
            Self::Broken => 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// EquipmentInstance (单件装备)
// ---------------------------------------------------------------------------

/// 单件装备实例
///
/// 不挂 entity (MVP 阶段由 `EquipmentState` 用 `HashMap<slot, EquipmentInstance>` 维护)。
/// `id` 是装备的唯一 ID(网络同步用),不是玩家 ID。
#[derive(Component, Debug, Clone)]
pub struct EquipmentInstance {
    /// 装备唯一 ID
    pub id: u32,
    /// 装备物品 ID (对齐表格包 §18.3/§18.4 ID,如 `blade_plain_duelist`)
    pub item_id: String,
    /// 装备槽位
    pub slot: EquipmentSlot,
    /// 品质
    pub quality: EquipmentQuality,
    /// 当前耐久
    pub current_durability: f32,
    /// 最大耐久 (受 quality.durability_multiplier 修正)
    pub max_durability: f32,
    /// 重量 (单位 W, 表格包 §18.3-§18.5 weight 列)
    pub weight: f32,
}

impl EquipmentInstance {
    /// 构造一个新装备 (auto 算 max_durability = base * quality.durability_multiplier)
    pub fn new(
        id: u32,
        item_id: &str,
        slot: EquipmentSlot,
        quality: EquipmentQuality,
        base_max: f32,
        weight: f32,
    ) -> Self {
        let max = base_max * quality.durability_multiplier();
        Self {
            id,
            item_id: item_id.to_string(),
            slot,
            quality,
            current_durability: max,
            max_durability: max,
            weight,
        }
    }

    /// 当前耐久状态
    pub fn durability_state(&self) -> DurabilityState {
        if !self.quality.uses_normal_durability() {
            // Unique 不走普通耐久, 永远算 Normal
            return DurabilityState::Normal;
        }
        DurabilityState::from_percent(self.current_durability / self.max_durability.max(1.0))
    }

    /// 扣耐久 (clamp 到 0)
    pub fn damage(&mut self, amount: f32) {
        if !self.quality.uses_normal_durability() {
            return;
        }
        self.current_durability = (self.current_durability - amount).max(0.0);
    }

    /// 修复 (clamp 到 max)
    pub fn repair(&mut self, amount: f32) {
        if !self.quality.uses_normal_durability() {
            return;
        }
        self.current_durability = (self.current_durability + amount).min(self.max_durability);
    }

    /// 是否已破裂 (完全失效)
    pub fn is_broken(&self) -> bool {
        self.durability_state() == DurabilityState::Broken
    }

    /// 当前综合效果乘数 (品质 * 耐久)
    pub fn effective_multiplier(&self) -> f32 {
        if !self.quality.uses_normal_durability() {
            return 1.0;
        }
        self.quality.stat_multiplier() * self.durability_state().effectiveness()
    }
}

// ---------------------------------------------------------------------------
// EquipmentState (挂在玩家 entity / Resource)
// ---------------------------------------------------------------------------

/// 玩家装备状态
///
/// 用 `HashMap<slot, EquipmentInstance>` 维护 7 个槽。
/// 简化: 不存 inventory(留 §18.6 物品表),只管已装备的。
#[derive(Component, Debug, Default, Clone)]
pub struct EquipmentState {
    /// 槽位 → 装备实例
    pub slots: std::collections::HashMap<EquipmentSlot, EquipmentInstance>,
}

impl EquipmentState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 装备一件 (覆盖同槽已有装备,返回被替换的旧装备)
    pub fn equip(&mut self, item: EquipmentInstance) -> Option<EquipmentInstance> {
        // 同一 item_id 不重复装备
        if let Some(existing) = self.slots.get(&item.slot) {
            if existing.id == item.id {
                return None;
            }
        }
        self.slots.insert(item.slot, item)
    }

    /// 卸下一件 (返回被卸下的装备)
    pub fn unequip(&mut self, slot: EquipmentSlot) -> Option<EquipmentInstance> {
        self.slots.remove(&slot)
    }

    /// 取某槽的装备
    pub fn get(&self, slot: EquipmentSlot) -> Option<&EquipmentInstance> {
        self.slots.get(&slot)
    }

    /// 取某槽的装备 (mutable)
    pub fn get_mut(&mut self, slot: EquipmentSlot) -> Option<&mut EquipmentInstance> {
        self.slots.get_mut(&slot)
    }

    /// 已装备数量 (0..=7)
    pub fn count(&self) -> usize {
        self.slots.len()
    }

    /// 全部已装备的总重量
    pub fn total_weight(&self) -> f32 {
        self.slots.values().map(|e| e.weight).sum()
    }

    /// 全部已破裂的装备 ID (留死亡掉落 / 拆分回收用)
    pub fn broken_ids(&self) -> Vec<u32> {
        self.slots.values().filter(|e| e.is_broken()).map(|e| e.id).collect()
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

/// 装备变更 (装备 / 卸下 / 替换)
#[derive(Message, Debug, Clone)]
pub struct EquipmentChangedMsg {
    pub player_id: u32,
    pub slot: EquipmentSlot,
    pub new_item_id: Option<u32>, // None = 卸下
}

/// 装备耐久变化 (受击 / 修复)
#[derive(Message, Debug, Clone)]
pub struct EquipmentDurabilityChangedMsg {
    pub player_id: u32,
    pub slot: EquipmentSlot,
    pub item_id: u32,
    pub new_durability: f32,
    pub new_state: DurabilityState,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// 玩法 Plugin: 注册装备资源 + events
///
/// 用法:
/// ```ignore
/// app.add_plugins(EquipmentPlugin);
/// ```
///
/// 不挂 system (MVP),`EquipmentState` 由 scenario / worldgen 直接 spawn 在玩家 entity 上。
pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EquipmentChangedMsg>().add_message::<EquipmentDurabilityChangedMsg>();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_id_str_matches_doc() {
        // 表格包 §18.1 槽位 ID 必须一字不差
        assert_eq!(EquipmentSlot::MainHand.id_str(), "slot_main_hand");
        assert_eq!(EquipmentSlot::OffHand.id_str(), "slot_off_hand");
        assert_eq!(EquipmentSlot::Body.id_str(), "slot_body");
        assert_eq!(EquipmentSlot::Cloak.id_str(), "slot_cloak");
        assert_eq!(EquipmentSlot::Back.id_str(), "slot_back");
        assert_eq!(EquipmentSlot::Focus.id_str(), "slot_focus");
        assert_eq!(EquipmentSlot::Relic.id_str(), "slot_relic");
    }

    #[test]
    fn all_7_slots_in_all_array() {
        // ALL 必须 7 个全覆盖
        assert_eq!(EquipmentSlot::ALL.len(), 7);
        // 必须唯一 (防 enum 改名导致重复)
        let mut sorted: Vec<_> = EquipmentSlot::ALL.iter().collect();
        sorted.sort_by_key(|s| s.id_str());
        sorted.dedup();
        assert_eq!(sorted.len(), 7);
    }

    #[test]
    fn label_zh_covers_all_7_slots() {
        for s in EquipmentSlot::ALL {
            let l = s.label_zh();
            assert!(!l.is_empty(), "label_zh 不能为空: {:?}", s);
            assert!(
                l.chars().any(|c| c as u32 > 127),
                "label_zh 应是中文: {:?}",
                s
            );
        }
    }

    #[test]
    fn quality_id_str_matches_doc() {
        assert_eq!(EquipmentQuality::Crude.id_str(), "q_crude");
        assert_eq!(EquipmentQuality::Crafted.id_str(), "q_crafted");
        assert_eq!(EquipmentQuality::Refined.id_str(), "q_refined");
        assert_eq!(EquipmentQuality::Arcane.id_str(), "q_arcane");
        assert_eq!(EquipmentQuality::Unique.id_str(), "q_unique");
    }

    #[test]
    fn arcane_stronger_than_crafted() {
        assert!(
            EquipmentQuality::Arcane.stat_multiplier()
                > EquipmentQuality::Crafted.stat_multiplier(),
            "Arcane 应比 Crafted 主属性强"
        );
        assert!(
            EquipmentQuality::Refined.stat_multiplier()
                > EquipmentQuality::Crafted.stat_multiplier(),
            "Refined 应比 Crafted 主属性强"
        );
    }

    #[test]
    fn crude_durability_lower_than_crafted() {
        assert!(
            EquipmentQuality::Crude.durability_multiplier()
                < EquipmentQuality::Crafted.durability_multiplier(),
            "Crude 耐久应比 Crafted 低 (-20%)"
        );
        assert!(
            EquipmentQuality::Refined.durability_multiplier()
                > EquipmentQuality::Crafted.durability_multiplier(),
            "Refined 耐久应比 Crafted 高 (+15%)"
        );
    }

    #[test]
    fn unique_skips_normal_durability() {
        assert!(!EquipmentQuality::Unique.uses_normal_durability());
        assert!(EquipmentQuality::Crude.uses_normal_durability());
        assert!(EquipmentQuality::Crafted.uses_normal_durability());
        assert!(EquipmentQuality::Refined.uses_normal_durability());
        assert!(EquipmentQuality::Arcane.uses_normal_durability());
    }

    #[test]
    fn durability_state_thresholds() {
        // 表格包 §18.2: 25% 进入受损, 0% 进入破裂
        assert_eq!(DurabilityState::from_percent(1.0), DurabilityState::Normal);
        assert_eq!(DurabilityState::from_percent(0.50), DurabilityState::Normal);
        assert_eq!(DurabilityState::from_percent(0.25), DurabilityState::Normal);
        assert_eq!(
            DurabilityState::from_percent(0.249),
            DurabilityState::Damaged
        );
        assert_eq!(
            DurabilityState::from_percent(0.10),
            DurabilityState::Damaged
        );
        assert_eq!(
            DurabilityState::from_percent(0.01),
            DurabilityState::Damaged
        );
        assert_eq!(DurabilityState::from_percent(0.0), DurabilityState::Broken);
    }

    #[test]
    fn broken_effectiveness_zero() {
        assert_eq!(DurabilityState::Broken.effectiveness(), 0.0);
        assert!(DurabilityState::Normal.effectiveness() > 0.0);
        assert!(DurabilityState::Damaged.effectiveness() > 0.0);
        // 受损应 < 正常
        assert!(DurabilityState::Damaged.effectiveness() < DurabilityState::Normal.effectiveness());
    }

    #[test]
    fn equipment_damage_clamp() {
        let mut e = EquipmentInstance::new(
            1,
            "blade_plain_duelist",
            EquipmentSlot::MainHand,
            EquipmentQuality::Crafted,
            160.0,
            4.0,
        );
        assert!((e.current_durability - 160.0).abs() < 0.01);

        // 扣 50 → 110
        e.damage(50.0);
        assert!((e.current_durability - 110.0).abs() < 0.01);
        // 还不是受损 (110/160 = 68.75% > 25%)
        assert_eq!(e.durability_state(), DurabilityState::Normal);

        // 扣到 30 → 80
        e.damage(30.0);
        assert!((e.current_durability - 80.0).abs() < 0.01);
        // 80/160 = 50% → Normal
        assert_eq!(e.durability_state(), DurabilityState::Normal);

        // 扣到破裂 → 0
        e.damage(200.0);
        assert_eq!(e.current_durability, 0.0);
        assert_eq!(e.durability_state(), DurabilityState::Broken);
        assert!(e.is_broken());
    }

    #[test]
    fn equipment_repair_clamp() {
        let mut e = EquipmentInstance::new(
            2,
            "shield_wood",
            EquipmentSlot::OffHand,
            EquipmentQuality::Crafted,
            100.0,
            3.0,
        );
        e.current_durability = 20.0;
        e.repair(30.0);
        assert!((e.current_durability - 50.0).abs() < 0.01);
        e.repair(999.0); // 不能超过 max
        assert_eq!(e.current_durability, 100.0);
    }

    #[test]
    fn unique_damage_is_noop() {
        let mut e = EquipmentInstance::new(
            3,
            "relic_crown_shard",
            EquipmentSlot::Relic,
            EquipmentQuality::Unique,
            100.0,
            0.0,
        );
        assert_eq!(e.current_durability, 100.0);
        e.damage(50.0);
        assert_eq!(e.current_durability, 100.0, "Unique 不走普通耐久");
        e.damage(9999.0);
        assert_eq!(e.current_durability, 100.0);
        assert_eq!(e.durability_state(), DurabilityState::Normal);
    }

    #[test]
    fn quality_durability_affects_max() {
        let crude = EquipmentInstance::new(
            10,
            "x",
            EquipmentSlot::MainHand,
            EquipmentQuality::Crude,
            100.0,
            1.0,
        );
        let crafted = EquipmentInstance::new(
            11,
            "x",
            EquipmentSlot::MainHand,
            EquipmentQuality::Crafted,
            100.0,
            1.0,
        );
        let refined = EquipmentInstance::new(
            12,
            "x",
            EquipmentSlot::MainHand,
            EquipmentQuality::Refined,
            100.0,
            1.0,
        );
        let arcane = EquipmentInstance::new(
            13,
            "x",
            EquipmentSlot::MainHand,
            EquipmentQuality::Arcane,
            100.0,
            1.0,
        );

        assert!(
            (crude.max_durability - 80.0).abs() < 0.01,
            "Crude 100*0.8=80"
        );
        assert!((crafted.max_durability - 100.0).abs() < 0.01);
        assert!(
            (refined.max_durability - 115.0).abs() < 0.01,
            "Refined 100*1.15=115"
        );
        assert!(
            (arcane.max_durability - 110.0).abs() < 0.01,
            "Arcane 100*1.10=110"
        );
    }

    #[test]
    fn effective_multiplier_combines_quality_and_durability() {
        // Crafted + Normal = 1.0 * 1.0 = 1.0
        let e = EquipmentInstance::new(
            20,
            "x",
            EquipmentSlot::Body,
            EquipmentQuality::Crafted,
            100.0,
            1.0,
        );
        assert!((e.effective_multiplier() - 1.0).abs() < 0.001);

        // Refined + Normal = 1.08 * 1.0 = 1.08
        let mut e2 = EquipmentInstance::new(
            21,
            "x",
            EquipmentSlot::Body,
            EquipmentQuality::Refined,
            100.0,
            1.0,
        );
        assert!((e2.effective_multiplier() - 1.08).abs() < 0.001);

        // Refined + Damaged = 1.08 * 0.75 = 0.81
        e2.current_durability = 20.0; // 20% < 25% → Damaged
        assert!((e2.effective_multiplier() - 0.81).abs() < 0.001);

        // Refined + Broken = 1.08 * 0.0 = 0.0
        e2.current_durability = 0.0;
        assert_eq!(e2.effective_multiplier(), 0.0);

        // Unique + 任何耐久 = 1.0
        let mut eu = EquipmentInstance::new(
            22,
            "x",
            EquipmentSlot::Relic,
            EquipmentQuality::Unique,
            100.0,
            0.0,
        );
        eu.current_durability = 0.0; // 试图扣
        assert_eq!(eu.effective_multiplier(), 1.0, "Unique 永远 1.0");
    }

    #[test]
    fn state_equip_replaces_same_slot() {
        let mut st = EquipmentState::new();
        let sword1 = EquipmentInstance::new(
            1,
            "blade_plain_duelist",
            EquipmentSlot::MainHand,
            EquipmentQuality::Crafted,
            160.0,
            4.0,
        );
        let sword2 = EquipmentInstance::new(
            2,
            "blade_moonsteel_rapier",
            EquipmentSlot::MainHand,
            EquipmentQuality::Arcane,
            140.0,
            3.0,
        );

        assert!(st.equip(sword1).is_none());
        assert_eq!(st.count(), 1);
        let replaced = st.equip(sword2);
        assert!(replaced.is_some());
        assert_eq!(replaced.unwrap().id, 1, "应返回被替换的 sword1");
        assert_eq!(st.count(), 1);
        // 当前是 sword2
        assert_eq!(st.get(EquipmentSlot::MainHand).unwrap().id, 2);
    }

    #[test]
    fn state_equip_same_id_is_noop() {
        let mut st = EquipmentState::new();
        let sword = EquipmentInstance::new(
            1,
            "blade_plain_duelist",
            EquipmentSlot::MainHand,
            EquipmentQuality::Crafted,
            160.0,
            4.0,
        );
        st.equip(sword.clone());
        // 再 equip 同一个 id → 应是 noop
        let result = st.equip(sword);
        assert!(result.is_none(), "同 id 重复装备应 noop");
    }

    #[test]
    fn state_unequip_removes() {
        let mut st = EquipmentState::new();
        let sword = EquipmentInstance::new(
            1,
            "blade_plain_duelist",
            EquipmentSlot::MainHand,
            EquipmentQuality::Crafted,
            160.0,
            4.0,
        );
        st.equip(sword);
        let removed = st.unequip(EquipmentSlot::MainHand);
        assert_eq!(removed.unwrap().id, 1);
        assert_eq!(st.count(), 0);

        // 重复 unequip → None
        assert!(st.unequip(EquipmentSlot::MainHand).is_none());
    }

    #[test]
    fn state_total_weight_sums_all() {
        let mut st = EquipmentState::new();
        st.equip(EquipmentInstance::new(
            1,
            "a",
            EquipmentSlot::MainHand,
            EquipmentQuality::Crafted,
            100.0,
            4.0,
        ));
        st.equip(EquipmentInstance::new(
            2,
            "b",
            EquipmentSlot::OffHand,
            EquipmentQuality::Crafted,
            100.0,
            3.0,
        ));
        st.equip(EquipmentInstance::new(
            3,
            "c",
            EquipmentSlot::Body,
            EquipmentQuality::Crafted,
            100.0,
            10.0,
        ));
        assert!((st.total_weight() - 17.0).abs() < 0.01);
    }

    #[test]
    fn state_broken_ids_lists_broken_only() {
        let mut st = EquipmentState::new();
        let mut sword = EquipmentInstance::new(
            1,
            "a",
            EquipmentSlot::MainHand,
            EquipmentQuality::Crafted,
            100.0,
            4.0,
        );
        sword.current_durability = 0.0; // 破裂
        st.equip(sword);

        let mut shield = EquipmentInstance::new(
            2,
            "b",
            EquipmentSlot::OffHand,
            EquipmentQuality::Crafted,
            100.0,
            3.0,
        );
        shield.current_durability = 50.0; // 正常
        st.equip(shield);

        let mut body = EquipmentInstance::new(
            3,
            "c",
            EquipmentSlot::Body,
            EquipmentQuality::Crafted,
            100.0,
            10.0,
        );
        body.current_durability = 20.0; // 受损 (不是破裂)
        st.equip(body);

        let broken = st.broken_ids();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0], 1, "只有 sword id=1 是破裂");
    }
}
