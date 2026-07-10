use bevy::prelude::Vec2;
use lk2_core::eco_cycle::EcoCycle;
use lk2_core::resource::GlobalResourcePool;
use lk2_core::simulation::{NatureEvent, WorldInput, step_world};

#[test]
fn shared_step_reuses_existing_ecology_and_reports_the_causal_chain() {
    let mut ecology = EcoCycle::seeded_weather_at(Vec2::new(24.0, 24.0));
    let mut resources = GlobalResourcePool::default();
    let mut saw_plant_growth = false;
    let mut saw_animal_birth = false;
    let mut last_report = None;

    for tick in 1..=24 {
        let report = step_world(
            WorldInput { tick },
            &mut ecology,
            &mut resources,
        );
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
        let left = step_world(
            WorldInput { tick },
            &mut left_ecology,
            &mut left_resources,
        );
        let right = step_world(
            WorldInput { tick },
            &mut right_ecology,
            &mut right_resources,
        );
        assert_eq!(left, right);
    }
}
