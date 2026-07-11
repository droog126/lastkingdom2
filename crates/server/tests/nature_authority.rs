#[path = "../src/app/mod.rs"]
mod app;
#[path = "../src/authority/mod.rs"]
mod authority;
#[path = "../src/networking/mod.rs"]
mod networking;
#[path = "../src/observation/mod.rs"]
mod observation;
#[path = "../src/persistence/mod.rs"]
mod persistence;
#[path = "../src/replication/mod.rs"]
mod replication;

use app::{NatureServerPlugin, NatureServerProjectionPlugin};
use authority::{LatestNatureReport, NatureAuthority, NatureAuthorityFault, NatureAuthorityPlugin};
use bevy::prelude::*;
use lk2_core::ecology::EcoCycle;
use lk2_core::resource::GlobalResourcePool;
use lk2_core::simulation::WorldInput;

#[test]
fn concrete_authority_calls_shared_step_and_rejects_duplicate_tick() {
    let mut authority = NatureAuthority::new(EcoCycle::default(), GlobalResourcePool::new());

    let report = authority.advance(WorldInput { tick: 1 }).unwrap();

    assert_eq!(report.tick, 1);
    assert!(report.snapshot.is_finite());
    assert_eq!(
        authority.ecology().to_snapshot(1),
        report.snapshot.detailed_ecology
    );
    assert!(
        authority
            .resources()
            .current
            .values()
            .all(|amount| *amount >= 0)
    );
    assert!(authority.advance(WorldInput { tick: 1 }).is_err());
}

#[test]
fn networking_rejects_duplicate_or_reversed_ticks() {
    assert_eq!(
        networking::decode_world_input(5, Some(4)).unwrap(),
        WorldInput { tick: 5 }
    );
    assert!(networking::decode_world_input(4, Some(4)).is_err());
    assert!(networking::decode_world_input(3, Some(4)).is_err());
}

#[test]
fn fixed_schedule_advances_and_publishes_one_shared_report_per_run() {
    let mut app = App::new();
    app.add_plugins(NatureAuthorityPlugin);

    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        app.world()
            .resource::<LatestNatureReport>()
            .0
            .as_ref()
            .unwrap()
            .tick,
        1
    );
    assert_eq!(app.world().resource::<NatureAuthorityFault>().0, None);

    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        app.world()
            .resource::<LatestNatureReport>()
            .0
            .as_ref()
            .unwrap()
            .tick,
        2
    );
}

#[test]
fn report_projects_to_replication_observation_and_persistence_without_new_rules() {
    let mut authority = NatureAuthority::default();
    let report = authority.advance(WorldInput { tick: 9 }).unwrap();
    let batch = replication::ReplicationBatch::from_tick_report(report);
    let observation = observation::NatureObservation::from_snapshot(&batch.snapshot.value);
    let save = persistence::NatureSave::new(batch.tick(), batch.snapshot.value.clone());
    let removal = replication::NatureEventDto::<&str>::Remove { id: 7 };

    assert_eq!(batch.tick(), 9);
    assert_eq!(observation.tick, 9);
    assert!(observation.is_finite());
    assert_eq!(save.state, batch.snapshot.value);
    assert!(save.validate().is_ok());
    assert_eq!(removal, replication::NatureEventDto::Remove { id: 7 });
}

#[test]
fn aggregate_plugin_publishes_all_read_only_boundaries_for_the_same_tick() {
    assert!(app::NATURE_SERVER_MODULES.authority);
    assert!(app::NATURE_SERVER_MODULES.replication);
    assert!(app::NATURE_SERVER_MODULES.observation);
    assert!(app::NATURE_SERVER_MODULES.persistence);

    let mut app = App::new();
    app.add_plugins(NatureServerPlugin);

    app.world_mut().run_schedule(FixedUpdate);

    let report_tick = app
        .world()
        .resource::<LatestNatureReport>()
        .0
        .as_ref()
        .unwrap()
        .tick;
    let replication_tick = app
        .world()
        .resource::<replication::LatestNatureReplication>()
        .0
        .as_ref()
        .unwrap()
        .tick();
    let observation_tick = app
        .world()
        .resource::<observation::LatestNatureObservation>()
        .0
        .as_ref()
        .unwrap()
        .tick;
    let save_tick = app
        .world()
        .resource::<persistence::LatestNatureSave>()
        .0
        .as_ref()
        .unwrap()
        .tick;

    assert_eq!(
        (report_tick, replication_tick, observation_tick, save_tick),
        (1, 1, 1, 1)
    );
}

#[test]
fn projection_plugin_does_not_start_a_second_authoritative_world() {
    let mut app = App::new();
    app.add_plugins(NatureServerProjectionPlugin);

    app.world_mut().run_schedule(FixedUpdate);

    assert!(app.world().resource::<LatestNatureReport>().0.is_none());
    assert!(
        app.world()
            .resource::<replication::LatestNatureReplication>()
            .0
            .is_none()
    );
    assert!(
        app.world()
            .resource::<observation::LatestNatureObservation>()
            .0
            .is_none()
    );
    assert!(
        app.world()
            .resource::<persistence::LatestNatureSave>()
            .0
            .is_none()
    );
}
