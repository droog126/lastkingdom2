#[path = "../src/authority/mod.rs"]
mod authority;
#[path = "../src/observation/mod.rs"]
mod observation;
#[path = "../src/persistence/mod.rs"]
mod persistence;
#[path = "../src/replication/mod.rs"]
mod replication;

use authority::{AuthorityDriver, AuthorityInput, WorldStepper};

#[derive(Default)]
struct FakeWorld {
    calls: Vec<AuthorityInput>,
}

impl WorldStepper for FakeWorld {
    type Snapshot = u32;
    type Event = &'static str;

    fn step(&mut self, input: AuthorityInput) -> (Self::Snapshot, Vec<Self::Event>) {
        self.calls.push(input);
        (self.calls.len() as u32, vec!["rain"])
    }
}

#[test]
fn authority_validates_then_steps_and_publishes_one_tick() {
    let mut driver = AuthorityDriver::new(FakeWorld::default());

    let output = driver.advance(AuthorityInput { tick: 7, rainfall: 4.0 }).unwrap();

    assert_eq!(
        driver.world().calls,
        vec![AuthorityInput { tick: 7, rainfall: 1.0 }]
    );
    assert_eq!(output.snapshot, 1);
    assert_eq!(output.events, vec!["rain"]);
    assert_eq!(output.tick.tick, 7);
    assert!(output.tick.accepted);
}

#[test]
fn invalid_input_never_reaches_shared_step() {
    let mut driver = AuthorityDriver::new(FakeWorld::default());

    assert!(driver.advance(AuthorityInput { tick: 1, rainfall: f32::NAN }).is_err());
    assert!(driver.world().calls.is_empty());
}

#[test]
fn duplicate_or_reversed_tick_never_reaches_shared_step() {
    let mut driver = AuthorityDriver::new(FakeWorld::default());
    driver.advance(AuthorityInput { tick: 3, rainfall: 0.2 }).unwrap();

    assert!(driver.advance(AuthorityInput { tick: 3, rainfall: 0.2 }).is_err());
    assert!(driver.advance(AuthorityInput { tick: 2, rainfall: 0.2 }).is_err());
    assert_eq!(driver.world().calls.len(), 1);
}

#[test]
fn replication_persistence_and_observation_are_read_only_projections() {
    let batch = replication::ReplicationBatch::new(9, 42_u32, vec!["grew"]);
    let save = persistence::NatureSave::new(batch.tick(), batch.snapshot.value);
    let observation = observation::NatureObservation {
        tick: batch.tick(),
        cloud_count: 2,
        rainfall: 0.4,
        soil_moisture: 0.6,
        plant_count: 3,
        animal_count: 1,
    };

    assert_eq!(batch.snapshot.tick, 9);
    assert_eq!(batch.events, vec!["grew"]);
    assert_eq!(save.state, 42);
    assert!(save.validate().is_ok());
    assert!(observation.is_finite());
}
