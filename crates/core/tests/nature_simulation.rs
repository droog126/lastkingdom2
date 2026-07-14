use bevy::prelude::Vec2;
use lk2_core::ecology::EcoCycle;
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::simulation::{
    NatureEvent, NatureSnapshot, WorldInput, step_world, step_world_elapsed,
};
use lk2_core::world::generation::{NatureProfile, WorldRecipe};

#[test]
fn shared_step_reuses_existing_ecology_and_reports_the_causal_chain() {
    let mut ecology = EcoCycle::seeded_weather_at(Vec2::new(24.0, 24.0));
    let mut resources = GlobalResourcePool::default();
    let mut saw_plant_growth = false;
    let mut saw_animal_birth = false;
    let mut last_report = None;

    for tick in 1..=24 {
        let report = step_world(WorldInput { tick }, &mut ecology, &mut resources);
        saw_plant_growth |= report
            .events
            .iter()
            .any(|event| matches!(event, NatureEvent::PlantsGrown { count } if *count > 0));
        saw_animal_birth |= report.events.iter().any(|event| {
            matches!(event, NatureEvent::AnimalsBorn { rabbits, wildlife } if *rabbits + *wildlife > 0)
        });
        last_report = Some(report);
    }

    let report = last_report.expect("the simulation produced a report");
    assert!(report.snapshot.is_finite());
    assert!(report.snapshot.atmosphere.cumulative_rainfall > 0.0);
    assert!(report.snapshot.hydrology.available_water >= 0.0);
    assert!(report.snapshot.ecology.plant_units > 0);
    assert!(report.snapshot.ecology.animal_count > 0);
    assert!(saw_plant_growth);
    assert!(saw_animal_birth);
}

#[test]
fn shared_step_is_deterministic_for_equal_state_and_inputs() {
    let initial = EcoCycle::default();
    let mut left_ecology = initial.clone();
    let mut right_ecology = initial;
    let mut left_resources = GlobalResourcePool::default();
    let mut right_resources = GlobalResourcePool::default();

    for tick in 1..=12 {
        let left = step_world(WorldInput { tick }, &mut left_ecology, &mut left_resources);
        let right = step_world(
            WorldInput { tick },
            &mut right_ecology,
            &mut right_resources,
        );
        assert_eq!(left, right);
    }
}

#[test]
fn world_recipe_reuses_existing_ecology_initializers_deterministically() {
    let recipe = WorldRecipe {
        seed: 42,
        center: [32.0, 48.0],
        nature: NatureProfile::LivingBasin,
    };

    let left = recipe.generate_ecology();
    let right = recipe.generate_ecology();
    assert_eq!(left, right);
    assert!(!left.clouds.is_empty());
    assert!(!left.plants.is_empty());
    assert!(!left.rabbits.is_empty());
}

#[test]
fn snapshot_rejects_non_finite_existing_cloud_state() {
    let mut ecology = EcoCycle::default();
    ecology.clouds[0].phase = f32::NAN;
    let mut resources = GlobalResourcePool::default();

    let report = step_world(WorldInput { tick: 1 }, &mut ecology, &mut resources);
    assert!(!report.snapshot.is_finite());
}

#[test]
fn elapsed_step_catches_up_without_bypassing_shared_rules() {
    let mut ecology = EcoCycle::seeded_weather_at(Vec2::new(24.0, 24.0));
    let mut resources = GlobalResourcePool::default();
    let report = step_world_elapsed(WorldInput { tick: 9 }, 4, &mut ecology, &mut resources);

    assert_eq!(report.tick, 9);
    assert!(report.ecology.rain_fell >= 0.0);
    assert!(report.snapshot.is_finite());
    assert!(!report.events.is_empty());
}

#[test]
fn zero_elapsed_is_a_safe_single_tick() {
    let mut left = EcoCycle::default();
    let mut right = left.clone();
    let mut left_resources = GlobalResourcePool::default();
    let mut right_resources = GlobalResourcePool::default();

    let left_report = step_world_elapsed(WorldInput { tick: 1 }, 0, &mut left, &mut left_resources);
    let right_report = step_world(WorldInput { tick: 1 }, &mut right, &mut right_resources);
    assert_eq!(left_report, right_report);
}

#[test]
fn elapsed_step_matches_repeated_single_steps_and_keeps_all_events() {
    let initial = EcoCycle::seeded_weather_at(Vec2::new(24.0, 24.0));
    let mut batched_ecology = initial.clone();
    let mut repeated_ecology = initial;
    let mut batched_resources = GlobalResourcePool::default();
    let mut repeated_resources = GlobalResourcePool::default();

    let batched = step_world_elapsed(
        WorldInput { tick: 4 },
        4,
        &mut batched_ecology,
        &mut batched_resources,
    );
    let mut repeated_events = Vec::new();
    for tick in 1..=4 {
        repeated_events.extend(
            step_world(
                WorldInput { tick },
                &mut repeated_ecology,
                &mut repeated_resources,
            )
            .events,
        );
    }

    assert_eq!(
        batched.snapshot,
        NatureSnapshot::from_ecology(4, &repeated_ecology)
    );
    for kind in ResourceKind::ALL {
        assert_eq!(
            batched_resources.get(*kind),
            repeated_resources.get(*kind),
            "resource state diverged for {kind:?}"
        );
    }
    assert_eq!(batched.events, repeated_events);
}

#[test]
fn wildlife_updates_are_spread_across_deterministic_tick_shards() {
    let mut ecology = EcoCycle::default();
    let before = ecology.wildlife.clone();
    let mut resources = GlobalResourcePool::default();

    ecology.tick_at(1, &mut resources);

    let changed = before
        .iter()
        .zip(&ecology.wildlife)
        .filter(|(before, after)| before.pos != after.pos || before.energy != after.energy)
        .count();
    assert!(changed > 0);
    assert!(changed < before.len());
}

#[test]
fn rabbit_updates_are_spread_across_deterministic_tick_shards() {
    let mut ecology = EcoCycle::default();
    let before = ecology.rabbits.clone();
    let mut resources = GlobalResourcePool::default();

    ecology.tick_at(1, &mut resources);

    let changed = before
        .iter()
        .zip(&ecology.rabbits)
        .filter(|(before, after)| before.pos != after.pos || before.energy != after.energy)
        .count();
    assert!(changed > 0);
    assert!(changed < before.len());
}

#[test]
fn plant_regrowth_reaches_every_shard_within_the_cadence_window() {
    let mut ecology = EcoCycle::default();
    for plant in &mut ecology.plants {
        plant.stock = 0;
    }
    ecology.rain = 100.0;
    let mut resources = GlobalResourcePool::default();

    ecology.tick_at(1, &mut resources);
    let after_first = ecology
        .plants
        .iter()
        .filter(|plant| plant.stock > 0)
        .count();
    assert!(after_first > 0);
    assert!(after_first < ecology.plants.len());

    ecology.tick_at(2, &mut resources);
    ecology.tick_at(3, &mut resources);
    assert!(ecology.plants.iter().all(|plant| plant.stock > 0));
}

#[test]
fn regional_snapshot_filters_entities_without_mutating_global_facts() {
    let mut ecology = EcoCycle::seeded_weather_at(Vec2::new(10.0, 10.0));
    let mut resources = GlobalResourcePool::default();
    let report = step_world(WorldInput { tick: 1 }, &mut ecology, &mut resources);

    let regional = report.snapshot.for_region([10.0, 10.0], 3.0);
    assert!(regional.is_finite());
    assert!(regional.ecology.animal_count <= report.snapshot.ecology.animal_count);
    assert!(regional.ecology.plant_count <= report.snapshot.ecology.plant_count);
    assert_eq!(
        regional.atmosphere.cumulative_rainfall,
        report.snapshot.atmosphere.cumulative_rainfall
    );
}

#[test]
fn aggregate_nature_events_remain_visible_in_regional_projection() {
    let event = NatureEvent::RainFell { amount: 1.0 };
    assert_eq!(event.for_region([0.0, 0.0], 1.0), Some(event));
}
