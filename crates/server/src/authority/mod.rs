//! Server-owned adapter around the shared deterministic world step.

pub mod pvp;

use bevy::prelude::*;
use lk2_core::ecology::EcoCycle;
use lk2_core::resource::GlobalResourcePool;
use lk2_core::simulation::{TickReport, WorldInput, step_world};

#[derive(Resource)]
pub struct NatureAuthority {
    ecology: EcoCycle,
    resources: GlobalResourcePool,
    last_tick: Option<u64>,
}

impl NatureAuthority {
    pub fn new(ecology: EcoCycle, resources: GlobalResourcePool) -> Self {
        Self {
            ecology,
            resources,
            last_tick: None,
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
