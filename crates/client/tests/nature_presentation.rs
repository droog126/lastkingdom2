#[path = "../src/app/mod.rs"]
mod client_app;
#[path = "../src/presentation/mod.rs"]
mod presentation;
#[path = "../src/synchronization/mod.rs"]
mod synchronization;

use bevy::prelude::*;
use client_app::{
    NatureClientPlugin, NatureRunMode, submit_authoritative_snapshot, submit_authoritative_tick,
};
use lk2_core::atmosphere::AtmosphereSnapshot;
use lk2_core::ecology::nature::NatureEcologySnapshot;
use lk2_core::hydrology::HydrologySnapshot;
use lk2_core::protocol::components::{
    EcoBerryNet, EcoCloudNet, EcoPlantNet, EcoRabbitNet, EcoSnapshot, EcoWildlifeNet,
};
use lk2_core::simulation::{NatureEvent, NatureSnapshot};
use presentation::weather::NatureEventPresentation;
use presentation::{NatureVisualKey, desired_visuals};
use synchronization::{NaturePresentationInput, NatureSnapshotBuffer, SnapshotAcceptance};

fn nature_snapshot(tick: u64) -> NatureSnapshot {
    NatureSnapshot {
        tick,
        atmosphere: AtmosphereSnapshot {
            cloud_count: 1,
            cloud_water: 0.8,
            cumulative_rainfall: 2.4,
        },
        hydrology: HydrologySnapshot { available_water: 0.7 },
        ecology: NatureEcologySnapshot { plant_count: 2, plant_units: 4, animal_count: 2 },
        detailed_ecology: EcoSnapshot {
            tick,
            co2: 1.0,
            rain: 0.7,
            rainfall: 2.4,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
            clouds: vec![EcoCloudNet { id: 10, x: 2.0, z: 3.0, rain: 0.8, phase: 0.4 }],
            rabbits: vec![EcoRabbitNet { id: 20, x: 4.0, z: 5.0, energy: 3.0 }],
            wildlife: vec![EcoWildlifeNet { id: 30, kind: 2, x: 6.0, z: 7.0, energy: 4.0 }],
            berries: vec![EcoBerryNet { id: 40, x: 8.0, z: 9.0, fruit: 2 }],
            plants: vec![EcoPlantNet { id: 50, kind: 3, x: 10.0, z: 11.0, stock: 2 }],
        },
    }
}

#[test]
fn one_snapshot_drives_every_natural_visual_category() {
    let visuals = desired_visuals(&nature_snapshot(7));
    let keys = visuals.iter().map(|(key, _)| *key).collect::<Vec<_>>();

    assert!(keys.contains(&NatureVisualKey::Cloud(10)));
    assert!(keys.contains(&NatureVisualKey::RainDrop { cloud_id: 10, index: 0 }));
    assert!(keys.contains(&NatureVisualKey::Plant(50)));
    assert!(keys.contains(&NatureVisualKey::BerryBush(40)));
    assert!(keys.contains(&NatureVisualKey::BerryFruit { bush_id: 40, index: 1 }));
    assert!(keys.contains(&NatureVisualKey::Rabbit(20)));
    assert!(keys.contains(&NatureVisualKey::Wildlife(30)));
}

#[test]
fn visual_population_is_deterministic_and_contains_no_ghosts() {
    let snapshot = nature_snapshot(7);
    let first = desired_visuals(&snapshot);
    let second = desired_visuals(&snapshot);
    assert_eq!(first, second);

    let empty = NatureSnapshot {
        atmosphere: AtmosphereSnapshot {
            cloud_count: 0,
            cloud_water: 0.0,
            cumulative_rainfall: 2.4,
        },
        ecology: NatureEcologySnapshot { plant_count: 0, plant_units: 0, animal_count: 0 },
        detailed_ecology: EcoSnapshot {
            clouds: vec![],
            rabbits: vec![],
            wildlife: vec![],
            berries: vec![],
            plants: vec![],
            ..nature_snapshot(8).detailed_ecology
        },
        ..nature_snapshot(8)
    };
    assert!(desired_visuals(&empty).is_empty());
}

#[test]
fn online_and_offline_inputs_share_monotonic_cache_contract() {
    let mut offline = NatureSnapshotBuffer::default();
    assert_eq!(
        offline.push(nature_snapshot(10)),
        SnapshotAcceptance::Accepted
    );
    assert_eq!(
        offline.push(nature_snapshot(9)),
        SnapshotAcceptance::DuplicateOrStale
    );
    assert_eq!(offline.latest().map(|snapshot| snapshot.tick), Some(10));

    let mut online = NatureSnapshotBuffer::default();
    assert_eq!(NatureRunMode::Online, NatureRunMode::Online);
    assert_eq!(
        submit_authoritative_snapshot(&mut online, nature_snapshot(10)),
        SnapshotAcceptance::Accepted
    );
    assert_eq!(online.latest(), offline.latest());
}

#[test]
fn plugin_reconciles_spawn_update_and_cleanup_from_snapshot_ids() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<StandardMaterial>::default())
        .add_plugins(NatureClientPlugin { mode: NatureRunMode::Offline });
    app.update();

    let snapshot = nature_snapshot(10);
    let input = NaturePresentationInput::from(&snapshot);
    assert_eq!(input.tick, 10);
    assert_eq!(input.clouds.len(), 1);
    assert_eq!(input.rabbits.len(), 1);
    assert_eq!(input.wildlife.len(), 1);
    assert_eq!(input.berries.len(), 1);
    assert_eq!(input.plants.len(), 1);
    assert_eq!(input.cloud_water, 0.8);
    assert_eq!(input.available_water, 0.7);
    assert_eq!(input.cumulative_rainfall, 2.4);

    assert_eq!(
        submit_authoritative_tick(
            &mut app.world_mut().resource_mut::<NatureSnapshotBuffer>(),
            snapshot,
            [
                NatureEvent::RainFell { amount: 0.3 },
                NatureEvent::PlantsGrown { count: 2 },
            ],
        ),
        SnapshotAcceptance::Accepted
    );
    app.update();

    let event_cues = *app.world().resource::<NatureEventPresentation>();
    assert_eq!(event_cues.rain_fell, 0.3);
    assert_eq!(event_cues.plants_grown, 2);

    let visible_count = {
        let world = app.world_mut();
        let mut query = world.query::<&NatureVisualKey>();
        query.iter(world).count()
    };
    assert_eq!(visible_count, 12);

    let empty = NatureSnapshot {
        atmosphere: AtmosphereSnapshot {
            cloud_count: 0,
            cloud_water: 0.0,
            cumulative_rainfall: 2.4,
        },
        ecology: NatureEcologySnapshot { plant_count: 0, plant_units: 0, animal_count: 0 },
        detailed_ecology: EcoSnapshot {
            clouds: vec![],
            rabbits: vec![],
            wildlife: vec![],
            berries: vec![],
            plants: vec![],
            ..nature_snapshot(11).detailed_ecology
        },
        ..nature_snapshot(11)
    };
    assert_eq!(
        app.world_mut().resource_mut::<NatureSnapshotBuffer>().push(empty),
        SnapshotAcceptance::Accepted
    );
    app.update();

    let visible_count = {
        let world = app.world_mut();
        let mut query = world.query::<&NatureVisualKey>();
        query.iter(world).count()
    };
    assert_eq!(visible_count, 0);
}
