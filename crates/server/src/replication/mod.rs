//! Read-only replication boundary. It owns no simulation state.

use serde::{Deserialize, Serialize};

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
