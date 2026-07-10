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

