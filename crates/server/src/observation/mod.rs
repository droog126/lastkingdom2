//! Side-effect-free facts exported by the authority.

use lk2_core::simulation::NatureSnapshot;
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
    pub fn from_snapshot(snapshot: &NatureSnapshot) -> Self {
        Self {
            tick: snapshot.tick,
            cloud_count: snapshot.atmosphere.cloud_count,
            rainfall: snapshot.atmosphere.cumulative_rainfall,
            soil_moisture: snapshot.hydrology.available_water,
            plant_count: snapshot.ecology.plant_count,
            animal_count: snapshot.ecology.animal_count,
        }
    }

    pub fn is_finite(&self) -> bool {
        self.rainfall.is_finite() && self.soil_moisture.is_finite()
    }
}
