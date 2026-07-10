//! Snapshot-driven plant presentation helpers.

use bevy::prelude::Vec3;

#[must_use]
pub fn plant_scale(kind: u8, stock: u32) -> Vec3 {
    let base = match kind {
        1 | 2 => Vec3::new(0.38, 0.52, 0.38),
        3 => Vec3::new(0.24, 0.62, 0.24),
        4 | 5 => Vec3::new(0.62, 0.38, 0.58),
        6 | 7 => Vec3::new(0.46, 0.82, 0.46),
        _ => Vec3::splat(0.56),
    };
    base * if stock == 0 { 0.62 } else { 1.0 }
}

#[must_use]
pub fn berry_bush_scale(fruit: u32) -> Vec3 {
    Vec3::splat(0.62 + fruit.min(3) as f32 * 0.035)
}
