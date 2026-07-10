//! Side-effect-free facts exported by the authority.

use bevy::prelude::*;
use lk2_core::simulation::NatureSnapshot;
use serde::{Deserialize, Serialize};

use super::authority::{LatestNatureReport, NatureAuthoritySet};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NatureObservation {
    pub tick: u64,
    pub cloud_count: u32,
    pub rainfall: f32,
    pub soil_moisture: f32,
    pub plant_count: u32,
    pub animal_count: u32,
}

impl NatureObservation {
    pub fn from_snapshot(snapshot: &NatureSnapshot) -> Self {
        Self {
            tick: snapshot.tick,
            cloud_count: snapshot.atmosphere.cloud_count,
            rainfall: snapshot.atmosphere.cumulative_rainfall,
            soil_moisture: snapshot.hydrology.available_water,
            plant_count: snapshot.ecology.plant_count,
            animal_count: snapshot.ecology.animal_count,
        }
    }

    pub fn is_finite(&self) -> bool {
        self.rainfall.is_finite() && self.soil_moisture.is_finite()
    }
}

#[derive(Resource, Default)]
pub struct LatestNatureObservation(pub Option<NatureObservation>);

pub struct NatureObservationPlugin;

impl Plugin for NatureObservationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LatestNatureObservation>().add_systems(
            FixedUpdate,
            export_observation.in_set(NatureAuthoritySet::PublishReport),
        );
    }
}

fn export_observation(
    report: Res<LatestNatureReport>,
    mut observation: ResMut<LatestNatureObservation>,
) {
    let Some(report) = report.0.as_ref() else {
        return;
    };
    if observation.0.as_ref().is_some_and(|value| value.tick == report.tick) {
        return;
    }
    observation.0 = Some(NatureObservation::from_snapshot(&report.snapshot));
}
