//! Snapshot-driven animal presentation helpers.

use bevy::prelude::Vec3;

#[must_use]
pub fn rabbit_scale(energy: f32) -> Vec3 {
    Vec3::splat(0.34 + energy.clamp(0.0, 10.0) * 0.008)
}

#[must_use]
pub fn wildlife_scale(kind: u8, energy: f32) -> Vec3 {
    let species_scale = match kind {
        1 => 0.82,
        2 => 0.68,
        3 => 1.05,
        4 => 0.78,
        _ => 0.48,
    };
    Vec3::splat(species_scale * (0.92 + energy.clamp(0.0, 10.0) * 0.008))
}
