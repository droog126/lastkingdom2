#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::Health;
    use crate::resource::ResourceKind;
    use std::collections::HashMap;

    fn health(current: f32, max: f32) -> Health {
        Health { current, max, invuln_until_tick: 0 }
    }

    #[test]
    fn reaper_copies_all_statuses_without_removing_them_from_target() {
        let mut wielder = StatusSet::default();
        wielder.apply(StatusEffect::new(StatusKind::Strength, 1, 5.0));
        let mut target = StatusSet::default();
        target.apply(StatusEffect::new(StatusKind::Strength, 2, 12.0));
        target.apply(StatusEffect::new(StatusKind::FireResistance, 1, 30.0));

        let copied = wielder.copy_from(&target);

        assert_eq!(copied, vec![StatusKind::Strength, StatusKind::FireResistance]);
        assert_eq!(target.len(), 2, "Hoplite scythe copies effects; it does not cleanse the target");
        assert_eq!(wielder.get(StatusKind::Strength).unwrap().potency, 2);
        assert_eq!(wielder.get(StatusKind::Strength).unwrap().remaining_secs, 12.0);
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
        assert_eq!(claim_dragon_loot(&mut dragon, &mut inventory), Err(DragonLootError::AlreadyClaimed));
    }

    #[test]
    fn dragon_katana_requires_a_heart_and_never_partially_consumes_on_failure() {
        let mut inventory = HashMap::from([(ResourceKind::DragonScale, 6)]);
        let before = inventory.clone();
        let mut loadout = LegendaryLoadout::default();

        let err = craft_legendary(
            LegendaryWeapon::DragonKatana,
            &mut inventory,
            &mut loadout,
        )
        .unwrap_err();

        assert_eq!(err, CraftLegendaryError::Missing(ResourceKind::DragonHeart, 1));
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

        craft_legendary(
            LegendaryWeapon::DragonKatana,
            &mut inventory,
            &mut loadout,
        )
        .unwrap();

        assert_eq!(inventory.get(&ResourceKind::DragonHeart), Some(&0));
        assert_eq!(inventory.get(&ResourceKind::DragonScale), Some(&2));
        assert_eq!(loadout.equipped, Some(LegendaryWeapon::DragonKatana));
    }
}
