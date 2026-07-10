//! Versioned, simulation-only persistence DTOs.

use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NatureSave<S> {
    pub schema_version: u32,
    pub tick: u64,
    pub state: S,
}

impl<S> NatureSave<S> {
    pub fn new(tick: u64, state: S) -> Self {
        Self { schema_version: CURRENT_SCHEMA_VERSION, tick, state }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err("unsupported nature save schema");
        }
        Ok(())
    }
}
