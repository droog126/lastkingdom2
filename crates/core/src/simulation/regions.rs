//! Region-owned natural simulation state.
//!
//! Each region owns its ecology, resources, and scheduler. Rules still enter
//! only through the shared `step_world` / `step_world_elapsed` functions.

use std::collections::BTreeMap;

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::ecology::EcoCycle;
use crate::resource::GlobalResourcePool;

use super::cadence::{RegionLod, RegionScheduler};
use super::{NatureSnapshot, TickReport, WorldInput, step_world_elapsed};

pub const DEFAULT_REGION_SIZE: f32 = 32.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NatureRegionId {
    pub x: i32,
    pub z: i32,
}

impl NatureRegionId {
    #[must_use]
    pub fn from_position(position: [f32; 2], region_size: f32) -> Self {
        assert!(region_size.is_finite() && region_size > 0.0);
        Self {
            x: (position[0] / region_size).floor() as i32,
            z: (position[1] / region_size).floor() as i32,
        }
    }

    #[must_use]
    pub fn center(self, region_size: f32) -> [f32; 2] {
        [
            (self.x as f32 + 0.5) * region_size,
            (self.z as f32 + 0.5) * region_size,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct NatureRegionState {
    pub id: NatureRegionId,
    pub ecology: EcoCycle,
    pub resources: GlobalResourcePool,
    pub scheduler: RegionScheduler,
}

impl NatureRegionState {
    #[must_use]
    pub fn new(
        id: NatureRegionId,
        ecology: EcoCycle,
        resources: GlobalResourcePool,
        lod: RegionLod,
    ) -> Self {
        Self {
            id,
            ecology,
            resources,
            scheduler: RegionScheduler::new(lod),
        }
    }

    pub fn advance_to(&mut self, world_tick: u64) -> Option<TickReport> {
        let elapsed_ticks = self.scheduler.due_ticks(world_tick);
        if elapsed_ticks == 0 {
            return None;
        }
        Some(step_world_elapsed(
            WorldInput {
                tick: self.scheduler.simulated_tick(),
            },
            elapsed_ticks,
            &mut self.ecology,
            &mut self.resources,
        ))
    }

    #[must_use]
    pub fn latest_snapshot(&self) -> NatureSnapshot {
        NatureSnapshot::from_ecology(self.scheduler.simulated_tick(), &self.ecology)
    }
}

#[derive(Debug, Default, Resource)]
pub struct NatureRegionWorld {
    regions: BTreeMap<NatureRegionId, NatureRegionState>,
    primary_id: Option<NatureRegionId>,
}

impl NatureRegionWorld {
    #[must_use]
    pub fn single(
        id: NatureRegionId,
        ecology: EcoCycle,
        resources: GlobalResourcePool,
        lod: RegionLod,
    ) -> Self {
        let mut world = Self::default();
        world.insert(NatureRegionState::new(id, ecology, resources, lod));
        world
    }

    pub fn insert(&mut self, region: NatureRegionState) -> Option<NatureRegionState> {
        let id = region.id;
        let previous = self.regions.insert(id, region);
        if self.primary_id.is_none() {
            self.primary_id = Some(id);
        }
        previous
    }

    pub fn remove(&mut self, id: NatureRegionId) -> Option<NatureRegionState> {
        let removed = self.regions.remove(&id);
        if self.primary_id == Some(id) {
            self.primary_id = self.regions.keys().next().copied();
        }
        removed
    }

    #[must_use]
    pub fn get(&self, id: NatureRegionId) -> Option<&NatureRegionState> {
        self.regions.get(&id)
    }

    #[must_use]
    pub fn get_mut(&mut self, id: NatureRegionId) -> Option<&mut NatureRegionState> {
        self.regions.get_mut(&id)
    }

    #[must_use]
    pub fn primary(&self) -> Option<&NatureRegionState> {
        self.primary_id.and_then(|id| self.regions.get(&id))
    }

    #[must_use]
    pub fn primary_mut(&mut self) -> Option<&mut NatureRegionState> {
        let id = self.primary_id?;
        self.regions.get_mut(&id)
    }

    pub fn advance_primary(&mut self, world_tick: u64) -> Option<(NatureRegionId, TickReport)> {
        let region = self.primary_mut()?;
        let id = region.id;
        region.advance_to(world_tick).map(|report| (id, report))
    }

    /// Transitional bridge for Bevy systems that still expose a standalone
    /// `GlobalResourcePool` resource. The region remains the owner; the
    /// external pool is a compatibility mirror for gameplay systems.
    pub fn advance_primary_with_external_pool(
        &mut self,
        world_tick: u64,
        external_pool: &mut GlobalResourcePool,
    ) -> Option<(NatureRegionId, TickReport)> {
        let region = self.primary_mut()?;
        region.resources.clone_from(external_pool);
        let result = region.advance_to(world_tick);
        external_pool.clone_from(&region.resources);
        result.map(|report| (region.id, report))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&NatureRegionId, &NatureRegionState)> {
        self.regions.iter()
    }

    pub fn advance_all(&mut self, world_tick: u64) -> Vec<(NatureRegionId, TickReport)> {
        self.regions
            .iter_mut()
            .filter_map(|(id, region)| region.advance_to(world_tick).map(|report| (*id, report)))
            .collect()
    }

    pub fn retain_except_primary(&mut self, wanted: &[NatureRegionId]) {
        let primary_id = self.primary_id;
        self.regions
            .retain(|id, _| Some(*id) == primary_id || wanted.contains(id));
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use super::*;

    #[test]
    fn region_id_rounds_positions_into_stable_cells() {
        assert_eq!(
            NatureRegionId::from_position([0.0, 31.9], 32.0),
            NatureRegionId { x: 0, z: 0 }
        );
        assert_eq!(
            NatureRegionId::from_position([-0.1, 32.0], 32.0),
            NatureRegionId { x: -1, z: 1 }
        );
    }

    #[test]
    fn region_scheduler_isolates_state_and_lod() {
        let left_id = NatureRegionId { x: 0, z: 0 };
        let right_id = NatureRegionId { x: 1, z: 0 };
        let mut world = NatureRegionWorld::default();
        world.insert(NatureRegionState::new(
            left_id,
            EcoCycle::seeded_weather_at(Vec2::new(8.0, 8.0)),
            GlobalResourcePool::new(),
            RegionLod::Active,
        ));
        world.insert(NatureRegionState::new(
            right_id,
            EcoCycle::seeded_weather_at(Vec2::new(40.0, 8.0)),
            GlobalResourcePool::new(),
            RegionLod::Nearby,
        ));

        let reports = world.advance_all(1);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].0, left_id);
        assert_eq!(world.get(left_id).unwrap().scheduler.simulated_tick(), 1);
        assert_eq!(world.get(right_id).unwrap().scheduler.simulated_tick(), 0);

        let reports = world.advance_all(3);
        assert_eq!(reports.len(), 2);
        assert_eq!(world.get(right_id).unwrap().scheduler.simulated_tick(), 3);
    }

    #[test]
    fn region_removal_drops_owned_state_without_touching_other_regions() {
        let left_id = NatureRegionId { x: 0, z: 0 };
        let right_id = NatureRegionId { x: 1, z: 0 };
        let mut world = NatureRegionWorld::default();
        world.insert(NatureRegionState::new(
            left_id,
            EcoCycle::default(),
            GlobalResourcePool::new(),
            RegionLod::Active,
        ));
        world.insert(NatureRegionState::new(
            right_id,
            EcoCycle::default(),
            GlobalResourcePool::new(),
            RegionLod::Active,
        ));
        assert!(world.remove(left_id).is_some());
        assert!(world.get(left_id).is_none());
        assert!(world.get(right_id).is_some());
    }

    #[test]
    fn single_region_compatibility_path_advances_primary_region() {
        let id = NatureRegionId { x: 0, z: 0 };
        let mut world = NatureRegionWorld::single(
            id,
            EcoCycle::default(),
            GlobalResourcePool::new(),
            RegionLod::Active,
        );
        let (report_id, report) = world.advance_primary(1).expect("primary region is due");
        assert_eq!(report_id, id);
        assert_eq!(report.tick, 1);
        assert_eq!(world.primary().unwrap().scheduler.simulated_tick(), 1);
    }

    #[test]
    fn external_pool_bridge_returns_pool_and_keeps_region_owner() {
        let id = NatureRegionId { x: 0, z: 0 };
        let mut world = NatureRegionWorld::single(
            id,
            EcoCycle::seeded_weather_at(Vec2::new(8.0, 8.0)),
            GlobalResourcePool::new(),
            RegionLod::Active,
        );
        let mut external = GlobalResourcePool::new();
        external.force_add(crate::resource::ResourceKind::Food, 3);
        let report = world
            .advance_primary_with_external_pool(1, &mut external)
            .expect("primary region is due");
        assert_eq!(report.0, id);
        assert_eq!(
            world
                .primary()
                .unwrap()
                .resources
                .get(crate::resource::ResourceKind::Food),
            external.get(crate::resource::ResourceKind::Food)
        );
        assert_eq!(world.primary().unwrap().scheduler.simulated_tick(), 1);
    }

    #[test]
    fn primary_advance_does_not_follow_btree_order() {
        let primary = NatureRegionId { x: 5, z: 5 };
        let earlier = NatureRegionId { x: -5, z: -5 };
        let mut world = NatureRegionWorld::single(
            primary,
            EcoCycle::default(),
            GlobalResourcePool::new(),
            RegionLod::Active,
        );
        world.insert(NatureRegionState::new(
            earlier,
            EcoCycle::default(),
            GlobalResourcePool::new(),
            RegionLod::Active,
        ));
        assert_eq!(world.advance_primary(1).unwrap().0, primary);
    }
}
