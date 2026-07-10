//! Side-effect-free facts exported by the authority.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NatureObservation {
    pub tick: u64,
    pub cloud_count: u32,
    pub rainfall: f32,
    pub soil_moisture: f32,
    pub plant_count: u32,
    pub animal_count: u32,
}

impl NatureObservation {
    pub fn is_finite(&self) -> bool {
        self.rainfall.is_finite() && self.soil_moisture.is_finite()
    }
}

