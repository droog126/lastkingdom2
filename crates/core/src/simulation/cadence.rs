//! Deterministic sharding helpers for low-frequency simulation rules.

#[must_use]
pub fn elapsed_scale(id: u32, tick: u64, skip: u64) -> f32 {
    assert!(skip > 0, "cadence skip must be positive");
    if tick == u64::MAX {
        1.0
    } else if (u64::from(id) + tick).is_multiple_of(skip) {
        skip as f32
    } else {
        0.0
    }
}

pub const MAX_REGION_CATCH_UP_UPDATES: u64 = 8;

/// Update frequency for a region based on how much detail players need from
/// it. The scheduler owns no world state; it only decides when a caller should
/// invoke the shared simulation entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RegionLod {
    Active,
    Nearby,
    Distant,
}

impl RegionLod {
    #[must_use]
    pub const fn max(self, other: Self) -> Self {
        if self.priority() >= other.priority() {
            self
        } else {
            other
        }
    }

    #[must_use]
    const fn priority(self) -> u8 {
        match self {
            Self::Active => 3,
            Self::Nearby => 2,
            Self::Distant => 1,
        }
    }

    #[must_use]
    pub fn for_distance(distance: f32, active_radius: f32, nearby_radius: f32) -> Self {
        if distance.is_finite() && distance <= active_radius.max(0.0) {
            Self::Active
        } else if distance.is_finite() && distance <= nearby_radius.max(active_radius) {
            Self::Nearby
        } else {
            Self::Distant
        }
    }

    #[must_use]
    pub const fn tick_interval(self) -> u64 {
        match self {
            Self::Active => 1,
            Self::Nearby => 3,
            Self::Distant => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionScheduler {
    lod: RegionLod,
    simulated_tick: u64,
}

impl RegionScheduler {
    #[must_use]
    pub const fn new(lod: RegionLod) -> Self {
        Self {
            lod,
            simulated_tick: 0,
        }
    }

    #[must_use]
    pub const fn lod(self) -> RegionLod {
        self.lod
    }

    pub fn set_lod(&mut self, lod: RegionLod) {
        self.lod = lod;
    }

    /// Synchronizes a compatibility region with an external fixed authority
    /// clock during migration from the single-region adapter.
    pub fn sync_to_tick(&mut self, tick: u64) {
        self.simulated_tick = self.simulated_tick.max(tick);
    }

    #[must_use]
    pub const fn simulated_tick(self) -> u64 {
        self.simulated_tick
    }

    /// Returns the number of world ticks now elapsed for this region and
    /// advances only by complete region intervals. Partial intervals remain
    /// pending. The returned value is the elapsed-time input for
    /// `step_world_elapsed`, not merely the number of region updates.
    pub fn due_ticks(&mut self, world_tick: u64) -> u32 {
        if world_tick <= self.simulated_tick {
            return 0;
        }
        let interval = self.lod.tick_interval();
        let due_updates =
            ((world_tick - self.simulated_tick) / interval).min(MAX_REGION_CATCH_UP_UPDATES);
        let due = due_updates
            .saturating_mul(interval)
            .min(u64::from(u32::MAX));
        let due_u32 = due as u32;
        // `due` is already expressed in world ticks. Multiplying it by the
        // interval again would make a nearby region jump from tick 0 to 9
        // when only three world ticks have elapsed.
        self.simulated_tick = self.simulated_tick.saturating_add(u64::from(due_u32));
        due_u32
    }
}

#[cfg(test)]
mod tests {
    use super::{RegionLod, RegionScheduler, elapsed_scale};

    #[test]
    fn cadence_covers_each_object_once_per_window() {
        let skip = 3;
        for id in 0..12 {
            let due_ticks = (1..=skip)
                .filter(|tick| elapsed_scale(id, *tick, skip) > 0.0)
                .count();
            assert_eq!(due_ticks, 1, "object {id} was not assigned one shard");
        }
    }

    #[test]
    fn cadence_returns_elapsed_scale_for_due_objects() {
        assert_eq!(elapsed_scale(2, 1, 3), 3.0);
        assert_eq!(elapsed_scale(1, 1, 3), 0.0);
        assert_eq!(elapsed_scale(7, u64::MAX, 3), 1.0);
    }

    #[test]
    fn region_scheduler_preserves_partial_intervals() {
        let mut scheduler = RegionScheduler::new(RegionLod::Nearby);
        assert_eq!(scheduler.due_ticks(2), 0);
        assert_eq!(scheduler.simulated_tick(), 0);
        assert_eq!(scheduler.due_ticks(3), 3);
        assert_eq!(scheduler.simulated_tick(), 3);
        assert_eq!(scheduler.due_ticks(8), 3);
        assert_eq!(scheduler.simulated_tick(), 6);
    }

    #[test]
    fn region_scheduler_has_explicit_lod_intervals() {
        assert_eq!(RegionLod::Active.tick_interval(), 1);
        assert_eq!(RegionLod::Nearby.tick_interval(), 3);
        assert_eq!(RegionLod::Distant.tick_interval(), 9);
    }

    #[test]
    fn region_lod_is_selected_from_distance() {
        assert_eq!(RegionLod::for_distance(4.0, 8.0, 24.0), RegionLod::Active);
        assert_eq!(RegionLod::for_distance(16.0, 8.0, 24.0), RegionLod::Nearby);
        assert_eq!(RegionLod::for_distance(40.0, 8.0, 24.0), RegionLod::Distant);
        assert_eq!(
            RegionLod::for_distance(f32::NAN, 8.0, 24.0),
            RegionLod::Distant
        );
    }

    #[test]
    fn region_lod_merge_keeps_the_highest_frequency_request() {
        assert_eq!(RegionLod::Distant.max(RegionLod::Nearby), RegionLod::Nearby);
        assert_eq!(RegionLod::Nearby.max(RegionLod::Active), RegionLod::Active);
        assert_eq!(RegionLod::Active.max(RegionLod::Distant), RegionLod::Active);
    }
}
