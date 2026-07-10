use serde::{Deserialize, Serialize};

use crate::eco_cycle::EcoCycle;

/// Read-only atmosphere data derived from the existing authoritative ecology cycle.
///
/// `EcoCycle` remains the owner during migration; this summary gives new server,
/// client, and loop adapters a stable boundary without duplicating cloud state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphereSnapshot {
    pub cloud_count: u32,
    pub cloud_water: f32,
    pub cumulative_rainfall: f32,
}

impl AtmosphereSnapshot {
    #[must_use]
    pub fn from_ecology(ecology: &EcoCycle) -> Self {
        Self {
            cloud_count: ecology.clouds.len() as u32,
            cloud_water: ecology.clouds.iter().map(|cloud| cloud.rain).sum(),
            cumulative_rainfall: ecology.rainfall,
        }
    }

    #[must_use]
    pub fn is_finite(self) -> bool {
        self.cloud_water.is_finite()
            && self.cloud_water >= 0.0
            && self.cumulative_rainfall.is_finite()
            && self.cumulative_rainfall >= 0.0
    }
}

