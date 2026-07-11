use serde::{Deserialize, Serialize};

use crate::ecology::EcoCycle;

/// Compact, presentation-independent ecology totals for replication and health checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatureEcologySnapshot {
    pub plant_count: u32,
    pub plant_units: u64,
    pub animal_count: u32,
}

impl NatureEcologySnapshot {
    #[must_use]
    pub fn from_ecology(ecology: &EcoCycle) -> Self {
        let plant_units = ecology
            .plants
            .iter()
            .map(|plant| u64::from(plant.stock))
            .chain(ecology.berries.iter().map(|berry| u64::from(berry.fruit)))
            .sum();
        Self {
            plant_count: (ecology.plants.len() + ecology.berries.len()) as u32,
            plant_units,
            animal_count: (ecology.rabbits.len() + ecology.wildlife.len()) as u32,
        }
    }
}
