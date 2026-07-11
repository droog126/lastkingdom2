use serde::{Deserialize, Serialize};

use crate::ecology::EcoCycle;

/// Transitional water-cycle summary over `EcoCycle::rain`.
///
/// The legacy cycle already transfers cloud rain into plant growth. Until a
/// spatial soil-water model exists, `available_water` names the unconsumed
/// rain budget instead of pretending that a full hydrology simulation exists.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HydrologySnapshot {
    pub available_water: f32,
}

impl HydrologySnapshot {
    #[must_use]
    pub fn from_ecology(ecology: &EcoCycle) -> Self {
        Self {
            available_water: ecology.rain,
        }
    }

    #[must_use]
    pub fn is_finite(self) -> bool {
        self.available_water.is_finite() && self.available_water >= 0.0
    }
}
