use bevy::prelude::Vec2;
use serde::{Deserialize, Serialize};

use crate::eco_cycle::EcoCycle;

/// Discrete natural-world starting profiles backed by existing ecology constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NatureProfile {
    LivingBasin,
    RainRecovery,
}

/// Minimal deterministic recipe for the natural simulation slice.
///
/// Terrain and rendering are deliberately absent. They can consume the same
/// recipe later without becoming the owner of ecology state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldRecipe {
    pub seed: u64,
    pub center: [f32; 2],
    pub nature: NatureProfile,
}

impl WorldRecipe {
    #[must_use]
    pub fn generate_ecology(self) -> EcoCycle {
        let center = Vec2::from_array(self.center) + seeded_offset(self.seed);
        match self.nature {
            NatureProfile::LivingBasin => EcoCycle::demo_at(center),
            NatureProfile::RainRecovery => EcoCycle::seeded_weather_at(center),
        }
    }
}

fn seeded_offset(seed: u64) -> Vec2 {
    let x = hash_unit(seed ^ 0x9E37_79B9_7F4A_7C15);
    let z = hash_unit(seed ^ 0xC2B2_AE3D_27D4_EB4F);
    Vec2::new((x - 0.5) * 4.0, (z - 0.5) * 4.0)
}

fn hash_unit(mut value: u64) -> f32 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value as u32) as f32 / u32::MAX as f32
}
