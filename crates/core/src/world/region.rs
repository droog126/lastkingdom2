use bevy::prelude::Vec3;
use serde::{Deserialize, Serialize};

/// Horizontal world partition used by streaming, simulation LOD, and network
/// interest management. Keep this independent from render chunks: a region is
/// a gameplay/replication boundary, not a mesh size.
pub const WORLD_REGION_SIZE: i32 = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldRegion {
    pub x: i32,
    pub z: i32,
}

impl WorldRegion {
    #[must_use]
    pub fn from_world_position(position: Vec3) -> Self {
        if !position.x.is_finite() || !position.z.is_finite() {
            return Self::default();
        }
        Self {
            x: (position.x / WORLD_REGION_SIZE as f32).floor() as i32,
            z: (position.z / WORLD_REGION_SIZE as f32).floor() as i32,
        }
    }

    #[must_use]
    pub const fn chebyshev_distance(self, other: Self) -> i32 {
        let dx = (self.x - other.x).abs();
        let dz = (self.z - other.z).abs();
        if dx > dz { dx } else { dz }
    }

    #[must_use]
    pub const fn within(self, other: Self, radius: i32) -> bool {
        // A negative radius is the explicit "no AOI limit" value used by
        // callers that want to disable regional filtering.
        radius < 0 || self.chebyshev_distance(other) <= radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_map_to_stable_regions() {
        assert_eq!(
            WorldRegion::from_world_position(Vec3::new(63.99, 0.0, -0.01)),
            WorldRegion { x: 0, z: -1 }
        );
        assert_eq!(
            WorldRegion::from_world_position(Vec3::new(64.0, 0.0, 128.0)),
            WorldRegion { x: 1, z: 2 }
        );
    }

    #[test]
    fn region_interest_uses_chebyshev_distance() {
        let center = WorldRegion { x: 0, z: 0 };
        assert!(center.within(WorldRegion { x: 2, z: -2 }, 2));
        assert!(!center.within(WorldRegion { x: 3, z: 0 }, 2));
        assert!(center.within(WorldRegion { x: 9, z: 9 }, -1));
    }

    #[test]
    fn non_finite_positions_fail_safe_to_origin_region() {
        assert_eq!(
            WorldRegion::from_world_position(Vec3::new(f32::NAN, 0.0, f32::INFINITY)),
            WorldRegion::default()
        );
    }
}
