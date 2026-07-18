//! Server-owned adapter around the shared deterministic world step.

pub mod pvp;

use std::collections::BTreeMap;

use bevy::prelude::*;
use lk2_core::ecology::EcoCycle;
use lk2_core::resource::GlobalResourcePool;
use lk2_core::simulation::regions::{NatureRegionId, NatureRegionState, NatureRegionWorld};
use lk2_core::simulation::{
    NatureSnapshot, TickReport, WorldInput, cadence::RegionLod, cadence::RegionScheduler,
    step_world, step_world_elapsed,
};

#[derive(Resource)]
pub struct NatureAuthority {
    ecology: EcoCycle,
    resources: GlobalResourcePool,
    last_tick: Option<u64>,
    scheduler: RegionScheduler,
}

/// Server adapter for multiple independently scheduled natural regions.
/// Each region owns its simulation state in `lk2_core`; this adapter only
/// coordinates insertion, removal, and report publication.
#[derive(Default)]
pub struct RegionalNatureAuthority {
    regions: NatureRegionWorld,
}

#[derive(Resource, Default)]
pub struct LatestNatureRegionReports(pub BTreeMap<NatureRegionId, TickReport>);

/// Select the last authoritative snapshot published for a region.
///
/// Projection consumers must read this report cache instead of rebuilding a
/// snapshot from mutable region state after the authority step. The primary
/// report is the compatibility fallback while a player's region is still
/// waiting for its first scheduled update.
#[must_use]
pub fn report_snapshot_for_region<'a>(
    reports: &'a LatestNatureRegionReports,
    primary_id: NatureRegionId,
    region_id: NatureRegionId,
) -> Option<&'a NatureSnapshot> {
    reports
        .0
        .get(&region_id)
        .or_else(|| reports.0.get(&primary_id))
        .map(|report| &report.snapshot)
}

impl RegionalNatureAuthority {
    #[must_use]
    pub fn single(
        id: NatureRegionId,
        ecology: EcoCycle,
        resources: GlobalResourcePool,
        lod: RegionLod,
    ) -> Self {
        Self {
            regions: NatureRegionWorld::single(id, ecology, resources, lod),
        }
    }

    pub fn insert_region(&mut self, region: NatureRegionState) -> Option<NatureRegionState> {
        self.regions.insert(region)
    }

    pub fn remove_region(&mut self, id: NatureRegionId) -> Option<NatureRegionState> {
        self.regions.remove(id)
    }

    pub fn advance(&mut self, world_tick: u64) -> Vec<(NatureRegionId, TickReport)> {
        self.regions.advance_all(world_tick)
    }

    pub fn advance_primary(&mut self, world_tick: u64) -> Option<(NatureRegionId, TickReport)> {
        self.regions.advance_primary(world_tick)
    }

    #[must_use]
    pub fn region(&self, id: NatureRegionId) -> Option<&NatureRegionState> {
        self.regions.get(id)
    }

    #[must_use]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }
}

impl NatureAuthority {
    pub fn new(ecology: EcoCycle, resources: GlobalResourcePool) -> Self {
        Self {
            ecology,
            resources,
            last_tick: None,
            scheduler: RegionScheduler::new(RegionLod::Active),
        }
    }

    pub fn with_lod(ecology: EcoCycle, resources: GlobalResourcePool, lod: RegionLod) -> Self {
        Self {
            ecology,
            resources,
            last_tick: None,
            scheduler: RegionScheduler::new(lod),
        }
    }

    pub fn advance(&mut self, input: WorldInput) -> Result<TickReport, &'static str> {
        if self.last_tick.is_some_and(|last| input.tick <= last) {
            return Err("authority tick must increase monotonically");
        }
        let report = step_world(input, &mut self.ecology, &mut self.resources);
        if !report.snapshot.is_finite() {
            return Err("shared world step produced a non-finite snapshot");
        }
        self.last_tick = Some(report.tick);
        Ok(report)
    }

    /// Advances a region only when its LOD interval is complete. The returned
    /// report still comes from the shared world step and covers the complete
    /// elapsed world time since the previous region update.
    pub fn advance_scheduled(
        &mut self,
        input: WorldInput,
    ) -> Result<Option<TickReport>, &'static str> {
        if self.last_tick.is_some_and(|last| input.tick <= last) {
            return Err("authority tick must increase monotonically");
        }
        let previous_scheduler = self.scheduler;
        let elapsed_ticks = self.scheduler.due_ticks(input.tick);
        if elapsed_ticks == 0 {
            self.last_tick = Some(input.tick);
            return Ok(None);
        }
        let report = step_world_elapsed(
            WorldInput {
                tick: self.scheduler.simulated_tick(),
            },
            elapsed_ticks,
            &mut self.ecology,
            &mut self.resources,
        );
        if !report.snapshot.is_finite() {
            self.scheduler = previous_scheduler;
            return Err("shared world step produced a non-finite snapshot");
        }
        self.last_tick = Some(input.tick);
        Ok(Some(report))
    }

    pub fn lod(&self) -> RegionLod {
        self.scheduler.lod()
    }

    pub fn update_lod_for_distance(
        &mut self,
        distance: f32,
        active_radius: f32,
        nearby_radius: f32,
    ) -> RegionLod {
        let lod = RegionLod::for_distance(distance, active_radius, nearby_radius);
        self.scheduler.set_lod(lod);
        lod
    }

    pub fn advance_scheduled_for_distance(
        &mut self,
        input: WorldInput,
        distance: f32,
        active_radius: f32,
        nearby_radius: f32,
    ) -> Result<Option<TickReport>, &'static str> {
        self.update_lod_for_distance(distance, active_radius, nearby_radius);
        self.advance_scheduled(input)
    }

    pub fn ecology(&self) -> &EcoCycle {
        &self.ecology
    }

    pub fn resources(&self) -> &GlobalResourcePool {
        &self.resources
    }
}

impl Default for NatureAuthority {
    fn default() -> Self {
        Self::new(EcoCycle::default(), GlobalResourcePool::new())
    }
}

#[derive(Resource, Default)]
pub struct LatestNatureReport(pub Option<TickReport>);

#[derive(Resource, Default, Debug, PartialEq, Eq)]
pub struct NatureAuthorityFault(pub Option<&'static str>);

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NatureAuthoritySet {
    ValidateInput,
    StepWorld,
    PublishReport,
}

pub struct NatureAuthorityPlugin;

impl Plugin for NatureAuthorityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NatureAuthority>()
            .init_resource::<LatestNatureReport>()
            .init_resource::<NatureAuthorityFault>()
            .configure_sets(
                FixedUpdate,
                (
                    NatureAuthoritySet::ValidateInput,
                    NatureAuthoritySet::StepWorld,
                    NatureAuthoritySet::PublishReport,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                advance_nature_authority.in_set(NatureAuthoritySet::StepWorld),
            );
    }
}

fn advance_nature_authority(
    mut authority: ResMut<NatureAuthority>,
    mut latest: ResMut<LatestNatureReport>,
    mut fault: ResMut<NatureAuthorityFault>,
) {
    let tick = latest
        .0
        .as_ref()
        .map_or(1, |report| report.tick.saturating_add(1));
    match authority.advance(WorldInput { tick }) {
        Ok(report) => {
            latest.0 = Some(report);
            fault.0 = None;
        }
        Err(error) => fault.0 = Some(error),
    }
}
