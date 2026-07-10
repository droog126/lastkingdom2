//! Read-only replication boundary. It owns no simulation state.

use bevy::prelude::*;
use lk2_core::simulation::{NatureEvent, NatureSnapshot, TickReport};
use serde::{Deserialize, Serialize};

use super::authority::{LatestNatureReport, NatureAuthoritySet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NatureSnapshotDto<T> {
    pub tick: u64,
    pub value: T,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NatureEventDto<E> {
    Upsert { id: u64, value: E },
    Remove { id: u64 },
}

pub fn snapshot<T>(tick: u64, value: T) -> NatureSnapshotDto<T> {
    NatureSnapshotDto { tick, value }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplicationBatch<S, E> {
    pub snapshot: NatureSnapshotDto<S>,
    pub events: Vec<E>,
}

impl<S, E> ReplicationBatch<S, E> {
    pub fn new(tick: u64, value: S, events: Vec<E>) -> Self {
        Self { snapshot: snapshot(tick, value), events }
    }

    pub fn tick(&self) -> u64 {
        self.snapshot.tick
    }
}

impl ReplicationBatch<NatureSnapshot, NatureEvent> {
    pub fn from_tick_report(report: TickReport) -> Self {
        Self::new(report.tick, report.snapshot, report.events)
    }
}

#[derive(Resource, Default)]
pub struct LatestNatureReplication(pub Option<ReplicationBatch<NatureSnapshot, NatureEvent>>);

pub struct NatureReplicationPlugin;

impl Plugin for NatureReplicationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LatestNatureReplication>().add_systems(
            FixedUpdate,
            build_replication_batch.in_set(NatureAuthoritySet::PublishReport),
        );
    }
}

fn build_replication_batch(
    report: Res<LatestNatureReport>,
    mut replication: ResMut<LatestNatureReplication>,
) {
    let Some(report) = report.0.as_ref() else {
        return;
    };
    if replication.0.as_ref().is_some_and(|batch| batch.tick() == report.tick) {
        return;
    }
    replication.0 = Some(ReplicationBatch::from_tick_report(report.clone()));
}
