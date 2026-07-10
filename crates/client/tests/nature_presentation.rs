#[path = "../src/presentation/mod.rs"]
mod presentation;
#[path = "../src/synchronization/mod.rs"]
mod synchronization;

use lk2_core::protocol::components::{
    EcoBerryNet, EcoCloudNet, EcoPlantNet, EcoRabbitNet, EcoSnapshot, EcoWildlifeNet,
};
use presentation::{NatureVisualKey, desired_visuals};
use synchronization::{NatureSnapshotBuffer, SnapshotAcceptance};

fn nature_snapshot(tick: u64) -> EcoSnapshot {
    EcoSnapshot {
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

    let empty = EcoSnapshot {
        clouds: vec![],
        rabbits: vec![],
        wildlife: vec![],
        berries: vec![],
        plants: vec![],
        ..nature_snapshot(8)
    };
    assert!(desired_visuals(&empty).is_empty());
}

#[test]
fn online_and_offline_inputs_share_monotonic_cache_contract() {
    let mut buffer = NatureSnapshotBuffer::default();
    assert_eq!(
        buffer.push(nature_snapshot(10)),
        SnapshotAcceptance::Accepted
    );
    assert_eq!(
        buffer.push(nature_snapshot(9)),
        SnapshotAcceptance::DuplicateOrStale
    );
    assert_eq!(buffer.latest().map(|snapshot| snapshot.tick), Some(10));
}
