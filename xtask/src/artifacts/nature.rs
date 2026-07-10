//! Versioned, deterministic artifacts for the natural-world loop.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const NATURE_ARTIFACT_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NatureObservation {
    pub schema: u32,
    pub tick: u64,
    pub clouds: u64,
    pub rainfall: f64,
    pub soil_moisture: f64,
    pub plants: u64,
    pub animals: u64,
    pub events: Vec<String>,
}

impl NatureObservation {
    pub fn finite_and_bounded(&self) -> bool {
        self.rainfall.is_finite() && self.soil_moisture.is_finite()
            && (0.0..=1.0).contains(&self.soil_moisture)
    }

    pub fn to_artifact(&self) -> Value {
        serde_json::to_value(self).expect("NatureObservation is serializable")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NatureArtifacts {
    pub schema: u32,
    pub observation: NatureObservation,
    pub assertions: Vec<String>,
}

impl NatureArtifacts {
    pub fn new(observation: NatureObservation, assertions: Vec<String>) -> Self {
        Self { schema: NATURE_ARTIFACT_SCHEMA, observation, assertions }
    }
}
