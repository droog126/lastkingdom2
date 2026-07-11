use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::combat::Health;
use crate::resource::ResourceKind;
pub use crate::status::{StatusEffect, StatusKind, StatusSet};

pub const REAPER_DRAIN_FRACTION: f32 = 0.25;
pub const DRAGON_MAX_HEALTH: f32 = 160.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum LegendaryWeapon {
    ReaperScythe,
    DragonKatana,
}

impl LegendaryWeapon {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReaperScythe => "死神镰刀",
            Self::DragonKatana => "龙武士刀",
        }
    }

    pub const fn model_path(self) -> &'static str {
        match self {
            Self::ReaperScythe => "procedural/pretty/hoplite_reaper_scythe.glb",
            Self::DragonKatana => "procedural/pretty/hoplite_dragon_katana.glb",
        }
    }
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct LegendaryLoadout {
    pub owned: Vec<LegendaryWeapon>,
    pub equipped: Option<LegendaryWeapon>,
}

impl LegendaryLoadout {
    pub fn grant_and_equip(&mut self, weapon: LegendaryWeapon) {
        if !self.owned.contains(&weapon) {
            self.owned.push(weapon);
        }
        self.equipped = Some(weapon);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReaperStrikeOutcome {
    pub damage_dealt: f32,
    pub health_restored: f32,
    pub copied_statuses: Vec<StatusKind>,
}

pub fn resolve_reaper_strike(
    wielder_health: &mut Health,
    wielder_statuses: &mut StatusSet,
    target_health: &mut Health,
    target_statuses: &StatusSet,
    current_tick: u32,
) -> ReaperStrikeOutcome {
    let requested_damage = target_health.current.max(0.0) * REAPER_DRAIN_FRACTION;
    let damage_dealt = target_health.damage(requested_damage, current_tick, 0);
    let health_restored = wielder_health.heal(damage_dealt);
    let copied_statuses = wielder_statuses.copy_from(target_statuses);
    ReaperStrikeOutcome {
        damage_dealt,
        health_restored,
        copied_statuses,
    }
}

#[derive(Resource, Debug, Clone)]
pub struct DragonBossState {
    pub block_pos: [i32; 3],
    pub health: Health,
    pub statuses: StatusSet,
    pub loot_claimed: bool,
}

impl Default for DragonBossState {
    fn default() -> Self {
        Self::new([0, 0, 0])
    }
}

impl DragonBossState {
    pub fn new(block_pos: [i32; 3]) -> Self {
        let mut statuses = StatusSet::default();
        statuses.apply(StatusEffect::new(StatusKind::Strength, 2, 9_999.0));
        statuses.apply(StatusEffect::new(StatusKind::FireResistance, 1, 9_999.0));
        Self {
            block_pos,
            health: Health {
                current: DRAGON_MAX_HEALTH,
                max: DRAGON_MAX_HEALTH,
                invuln_until_tick: 0,
            },
            statuses,
            loot_claimed: false,
        }
    }

    pub fn is_alive(&self) -> bool {
        !self.health.is_dead()
    }

    pub fn damage(&mut self, amount: f32) -> DragonAttackOutcome {
        let was_alive = self.is_alive();
        let damage_dealt = self.health.damage(amount.max(0.0), 0, 0);
        DragonAttackOutcome {
            damage_dealt,
            remaining_health: self.health.current,
            defeated: was_alive && self.health.is_dead(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragonAttackOutcome {
    pub damage_dealt: f32,
    pub remaining_health: f32,
    pub defeated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragonLoot {
    pub dragon_hearts: i64,
    pub dragon_scales: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragonLootError {
    DragonAlive,
    AlreadyClaimed,
}

pub fn claim_dragon_loot(
    dragon: &mut DragonBossState,
    inventory: &mut HashMap<ResourceKind, i64>,
) -> Result<DragonLoot, DragonLootError> {
    if dragon.is_alive() {
        return Err(DragonLootError::DragonAlive);
    }
    if dragon.loot_claimed {
        return Err(DragonLootError::AlreadyClaimed);
    }
    let loot = DragonLoot {
        dragon_hearts: 1,
        dragon_scales: 6,
    };
    *inventory.entry(ResourceKind::DragonHeart).or_insert(0) += loot.dragon_hearts;
    *inventory.entry(ResourceKind::DragonScale).or_insert(0) += loot.dragon_scales;
    dragon.loot_claimed = true;
    Ok(loot)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CraftLegendaryError {
    Missing(ResourceKind, i64),
}

const DRAGON_KATANA_RECIPE: &[(ResourceKind, i64)] = &[
    (ResourceKind::DragonHeart, 1),
    (ResourceKind::DragonScale, 4),
];

pub fn legendary_recipe(weapon: LegendaryWeapon) -> &'static [(ResourceKind, i64)] {
    match weapon {
        LegendaryWeapon::DragonKatana => DRAGON_KATANA_RECIPE,
        LegendaryWeapon::ReaperScythe => &[],
    }
}

pub fn craft_legendary(
    weapon: LegendaryWeapon,
    inventory: &mut HashMap<ResourceKind, i64>,
    loadout: &mut LegendaryLoadout,
) -> Result<(), CraftLegendaryError> {
    for (kind, required) in legendary_recipe(weapon) {
        let current = inventory.get(kind).copied().unwrap_or(0);
        if current < *required {
            return Err(CraftLegendaryError::Missing(*kind, *required - current));
        }
    }
    for (kind, required) in legendary_recipe(weapon) {
        *inventory.entry(*kind).or_insert(0) -= *required;
    }
    loadout.grant_and_equip(weapon);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::Health;
    use crate::resource::ResourceKind;
    use std::collections::HashMap;

    fn health(current: f32, max: f32) -> Health {
        Health {
            current,
            max,
            invuln_until_tick: 0,
        }
    }

    #[test]
    fn reaper_copies_all_statuses_without_removing_them_from_target() {
        let mut wielder = StatusSet::default();
        wielder.apply(StatusEffect::new(StatusKind::Strength, 1, 5.0));
        let mut target = StatusSet::default();
        target.apply(StatusEffect::new(StatusKind::Strength, 2, 12.0));
        target.apply(StatusEffect::new(StatusKind::FireResistance, 1, 30.0));

        let copied = wielder.copy_from(&target);

        assert_eq!(
            copied,
            vec![StatusKind::Strength, StatusKind::FireResistance]
        );
        assert_eq!(
            target.len(),
            2,
            "Hoplite scythe copies effects; it does not cleanse the target"
        );
        assert_eq!(wielder.get(StatusKind::Strength).unwrap().potency, 2);
        assert_eq!(
            wielder.get(StatusKind::Strength).unwrap().remaining_secs,
            12.0
        );
        assert!(wielder.get(StatusKind::FireResistance).is_some());
    }

    #[test]
    fn reaper_strike_drains_quarter_current_health_and_heals_for_actual_damage() {
        let mut wielder_health = health(40.0, 100.0);
        let mut target_health = health(80.0, 80.0);
        let mut wielder_status = StatusSet::default();
        let mut target_status = StatusSet::default();
        target_status.apply(StatusEffect::new(StatusKind::Speed, 1, 18.0));

        let outcome = resolve_reaper_strike(
            &mut wielder_health,
            &mut wielder_status,
            &mut target_health,
            &target_status,
            10,
        );

        assert_eq!(outcome.damage_dealt, 20.0);
        assert_eq!(outcome.health_restored, 20.0);
        assert_eq!(target_health.current, 60.0);
        assert_eq!(wielder_health.current, 60.0);
        assert_eq!(outcome.copied_statuses, vec![StatusKind::Speed]);
    }

    #[test]
    fn dragon_loot_can_only_be_claimed_once() {
        let mut dragon = DragonBossState::new([64, 18, 64]);
        let mut inventory = HashMap::new();
        let damage = dragon.health.current;

        assert!(dragon.damage(damage).defeated);
        let loot = claim_dragon_loot(&mut dragon, &mut inventory).unwrap();

        assert_eq!(loot.dragon_hearts, 1);
        assert_eq!(loot.dragon_scales, 6);
        assert_eq!(inventory.get(&ResourceKind::DragonHeart), Some(&1));
        assert_eq!(inventory.get(&ResourceKind::DragonScale), Some(&6));
        assert_eq!(
            claim_dragon_loot(&mut dragon, &mut inventory),
            Err(DragonLootError::AlreadyClaimed)
        );
    }

    #[test]
    fn dragon_katana_requires_a_heart_and_never_partially_consumes_on_failure() {
        let mut inventory = HashMap::from([(ResourceKind::DragonScale, 6)]);
        let before = inventory.clone();
        let mut loadout = LegendaryLoadout::default();

        let err = craft_legendary(LegendaryWeapon::DragonKatana, &mut inventory, &mut loadout)
            .unwrap_err();

        assert_eq!(
            err,
            CraftLegendaryError::Missing(ResourceKind::DragonHeart, 1)
        );
        assert_eq!(inventory, before, "failed craft must be atomic");
        assert_eq!(loadout.equipped, None);
    }

    #[test]
    fn dragon_katana_consumes_exact_boss_materials_and_equips() {
        let mut inventory = HashMap::from([
            (ResourceKind::DragonHeart, 1),
            (ResourceKind::DragonScale, 6),
        ]);
        let mut loadout = LegendaryLoadout::default();

        craft_legendary(LegendaryWeapon::DragonKatana, &mut inventory, &mut loadout).unwrap();

        assert_eq!(inventory.get(&ResourceKind::DragonHeart), Some(&0));
        assert_eq!(inventory.get(&ResourceKind::DragonScale), Some(&2));
        assert_eq!(loadout.equipped, Some(LegendaryWeapon::DragonKatana));
    }
}
