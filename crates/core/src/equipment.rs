

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {

    MainHand,

    OffHand,

    Body,

    Cloak,

    Back,

    Focus,

    Relic,
}

impl EquipmentSlot {

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentQuality {

    Crude,

    Crafted,

    Refined,

    Arcane,

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

    pub fn stat_multiplier(self) -> f32 {
        match self {
            Self::Crude => 1.0,
            Self::Crafted => 1.0,
            Self::Refined => 1.08,
            Self::Arcane => 1.12,
            Self::Unique => 1.0,
        }
    }

    pub fn durability_multiplier(self) -> f32 {
        match self {
            Self::Crude => 0.80,
            Self::Crafted => 1.0,
            Self::Refined => 1.15,
            Self::Arcane => 1.10,
            Self::Unique => 1.0,
        }
    }

    pub fn uses_normal_durability(self) -> bool {
        !matches!(self, Self::Unique)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurabilityState {

    Normal,

    Damaged,

    Broken,
}

impl DurabilityState {

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

    pub fn effectiveness(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Damaged => 0.75,
            Self::Broken => 0.0,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct EquipmentInstance {

    pub id: u32,

    pub item_id: String,

    pub slot: EquipmentSlot,

    pub quality: EquipmentQuality,

    pub current_durability: f32,

    pub max_durability: f32,

    pub weight: f32,
}

impl EquipmentInstance {

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

    pub fn durability_state(&self) -> DurabilityState {
        if !self.quality.uses_normal_durability() {

            return DurabilityState::Normal;
        }
        DurabilityState::from_percent(self.current_durability / self.max_durability.max(1.0))
    }

    pub fn damage(&mut self, amount: f32) {
        if !self.quality.uses_normal_durability() {
            return;
        }
        self.current_durability = (self.current_durability - amount).max(0.0);
    }

    pub fn repair(&mut self, amount: f32) {
        if !self.quality.uses_normal_durability() {
            return;
        }
        self.current_durability = (self.current_durability + amount).min(self.max_durability);
    }

    pub fn is_broken(&self) -> bool {
        self.durability_state() == DurabilityState::Broken
    }

    pub fn effective_multiplier(&self) -> f32 {
        if !self.quality.uses_normal_durability() {
            return 1.0;
        }
        self.quality.stat_multiplier() * self.durability_state().effectiveness()
    }
}

#[derive(Component, Debug, Default, Clone)]
pub struct EquipmentState {

    pub slots: std::collections::HashMap<EquipmentSlot, EquipmentInstance>,
}

impl EquipmentState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn equip(&mut self, item: EquipmentInstance) -> Option<EquipmentInstance> {

        if let Some(existing) = self.slots.get(&item.slot) {
            if existing.id == item.id {
                return None;
            }
        }
        self.slots.insert(item.slot, item)
    }

    pub fn unequip(&mut self, slot: EquipmentSlot) -> Option<EquipmentInstance> {
        self.slots.remove(&slot)
    }

    pub fn get(&self, slot: EquipmentSlot) -> Option<&EquipmentInstance> {
        self.slots.get(&slot)
    }

    pub fn get_mut(&mut self, slot: EquipmentSlot) -> Option<&mut EquipmentInstance> {
        self.slots.get_mut(&slot)
    }

    pub fn count(&self) -> usize {
        self.slots.len()
    }

    pub fn total_weight(&self) -> f32 {
        self.slots.values().map(|e| e.weight).sum()
    }

    pub fn broken_ids(&self) -> Vec<u32> {
        self.slots.values().filter(|e| e.is_broken()).map(|e| e.id).collect()
    }
}

#[derive(Message, Debug, Clone)]
pub struct EquipmentChangedMsg {
    pub player_id: u32,
    pub slot: EquipmentSlot,
    pub new_item_id: Option<u32>,
}

#[derive(Message, Debug, Clone)]
pub struct EquipmentDurabilityChangedMsg {
    pub player_id: u32,
    pub slot: EquipmentSlot,
    pub item_id: u32,
    pub new_durability: f32,
    pub new_state: DurabilityState,
}

pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EquipmentChangedMsg>().add_message::<EquipmentDurabilityChangedMsg>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_id_str_matches_doc() {

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

        assert_eq!(EquipmentSlot::ALL.len(), 7);

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

        e.damage(50.0);
        assert!((e.current_durability - 110.0).abs() < 0.01);

        assert_eq!(e.durability_state(), DurabilityState::Normal);

        e.damage(30.0);
        assert!((e.current_durability - 80.0).abs() < 0.01);

        assert_eq!(e.durability_state(), DurabilityState::Normal);

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
        e.repair(999.0);
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

        let e = EquipmentInstance::new(
            20,
            "x",
            EquipmentSlot::Body,
            EquipmentQuality::Crafted,
            100.0,
            1.0,
        );
        assert!((e.effective_multiplier() - 1.0).abs() < 0.001);

        let mut e2 = EquipmentInstance::new(
            21,
            "x",
            EquipmentSlot::Body,
            EquipmentQuality::Refined,
            100.0,
            1.0,
        );
        assert!((e2.effective_multiplier() - 1.08).abs() < 0.001);

        e2.current_durability = 20.0;
        assert!((e2.effective_multiplier() - 0.81).abs() < 0.001);

        e2.current_durability = 0.0;
        assert_eq!(e2.effective_multiplier(), 0.0);

        let mut eu = EquipmentInstance::new(
            22,
            "x",
            EquipmentSlot::Relic,
            EquipmentQuality::Unique,
            100.0,
            0.0,
        );
        eu.current_durability = 0.0;
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
        sword.current_durability = 0.0;
        st.equip(sword);

        let mut shield = EquipmentInstance::new(
            2,
            "b",
            EquipmentSlot::OffHand,
            EquipmentQuality::Crafted,
            100.0,
            3.0,
        );
        shield.current_durability = 50.0;
        st.equip(shield);

        let mut body = EquipmentInstance::new(
            3,
            "c",
            EquipmentSlot::Body,
            EquipmentQuality::Crafted,
            100.0,
            10.0,
        );
        body.current_durability = 20.0;
        st.equip(body);

        let broken = st.broken_ids();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0], 1, "只有 sword id=1 是破裂");
    }
}
