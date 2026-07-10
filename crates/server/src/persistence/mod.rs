//! Versioned, simulation-only persistence DTOs.

use bevy::prelude::*;
use lk2_core::simulation::NatureSnapshot;
use serde::{Deserialize, Serialize};

use super::authority::{LatestNatureReport, NatureAuthoritySet};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NatureSave<S> {
    pub schema_version: u32,
    pub tick: u64,
    pub state: S,
}

impl<S> NatureSave<S> {
    pub fn new(tick: u64, state: S) -> Self {
        Self { schema_version: CURRENT_SCHEMA_VERSION, tick, state }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err("unsupported nature save schema");
        }
        Ok(())
    }
}

#[derive(Resource, Default)]
pub struct LatestNatureSave(pub Option<NatureSave<NatureSnapshot>>);

pub struct NaturePersistencePlugin;

impl Plugin for NaturePersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LatestNatureSave>().add_systems(
            FixedUpdate,
            stage_save_snapshot.in_set(NatureAuthoritySet::PublishReport),
        );
    }
}

fn stage_save_snapshot(report: Res<LatestNatureReport>, mut save: ResMut<LatestNatureSave>) {
    let Some(report) = report.0.as_ref() else {
        return;
    };
    if save.0.as_ref().is_some_and(|value| value.tick == report.tick) {
        return;
    }
    save.0 = Some(NatureSave::new(report.tick, report.snapshot.clone()));
}
