use serde::{Deserialize, Serialize};

use crate::atmosphere::AtmosphereSnapshot;
use crate::ecology::nature::NatureEcologySnapshot;
use crate::ecology::{EcoCycle, EcoTickReport};
use crate::hydrology::HydrologySnapshot;
use crate::protocol::components::EcoSnapshot;
use crate::resource::GlobalResourcePool;

pub mod app_sets;
mod authority;

pub mod cadence;
pub mod regions;

pub use authority::{
    SimRole, advance_demo_tick, advance_fixed_authority_tick, advance_fixed_authority_tick_report,
};

pub type SimulationTick = u64;

/// Inputs shared by server authority, offline authority, scenarios, and tests.
///
/// Player interventions can be added as explicit variants later. The first
/// migration step freezes tick identity while preserving the existing ecology rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldInput {
    pub tick: SimulationTick,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NatureEvent {
    RainFell { amount: f32 },
    PlantsGrown { count: u32 },
    AnimalsBorn { rabbits: u32, wildlife: u32 },
    FruitChanged { eaten: u32, grown: u32 },
}

impl NatureEvent {
    /// Current ecology events are aggregate/global facts and therefore remain
    /// visible in every regional projection. Spatial events can later return
    /// `None` here without changing the replication API.
    #[must_use]
    pub fn for_region(&self, _center: [f32; 2], _radius: f32) -> Option<Self> {
        Some(self.clone())
    }
}

/// Stable observation envelope over the existing detailed `EcoSnapshot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NatureSnapshot {
    pub tick: SimulationTick,
    pub atmosphere: AtmosphereSnapshot,
    pub hydrology: HydrologySnapshot,
    pub ecology: NatureEcologySnapshot,
    pub detailed_ecology: EcoSnapshot,
}

impl NatureSnapshot {
    #[must_use]
    pub fn from_ecology(tick: SimulationTick, ecology: &EcoCycle) -> Self {
        Self {
            tick,
            atmosphere: AtmosphereSnapshot::from_ecology(ecology),
            hydrology: HydrologySnapshot::from_ecology(ecology),
            ecology: NatureEcologySnapshot::from_ecology(ecology),
            detailed_ecology: ecology.to_snapshot(tick),
        }
    }

    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.atmosphere.is_finite()
            && self.hydrology.is_finite()
            && self.detailed_ecology.is_finite()
            && self
                .detailed_ecology
                .clouds
                .iter()
                .all(|cloud| cloud.is_finite())
            && self
                .detailed_ecology
                .rabbits
                .iter()
                .all(|rabbit| rabbit.is_finite())
            && self
                .detailed_ecology
                .wildlife
                .iter()
                .all(|animal| animal.is_finite())
            && self
                .detailed_ecology
                .berries
                .iter()
                .all(|berry| berry.is_finite())
            && self
                .detailed_ecology
                .plants
                .iter()
                .all(|plant| plant.is_finite())
    }

    /// Creates a read-only spatial projection for replication or presentation.
    /// Cumulative counters remain global facts; spatial entities and regional
    /// ecology totals are filtered by the requested radius.
    #[must_use]
    pub fn for_region(&self, center: [f32; 2], radius: f32) -> Self {
        let radius_squared = radius.max(0.0).powi(2);
        let within = |x: f32, z: f32| {
            let dx = x - center[0];
            let dz = z - center[1];
            dx.mul_add(dx, dz * dz) <= radius_squared
        };
        let mut detailed = self.detailed_ecology.clone();
        detailed.clouds.retain(|cloud| within(cloud.x, cloud.z));
        detailed.rabbits.retain(|rabbit| within(rabbit.x, rabbit.z));
        detailed
            .wildlife
            .retain(|animal| within(animal.x, animal.z));
        detailed.berries.retain(|berry| within(berry.x, berry.z));
        detailed.plants.retain(|plant| within(plant.x, plant.z));

        let ecology = NatureEcologySnapshot {
            plant_count: (detailed.plants.len() + detailed.berries.len()) as u32,
            plant_units: detailed
                .plants
                .iter()
                .map(|plant| u64::from(plant.stock))
                .chain(detailed.berries.iter().map(|berry| u64::from(berry.fruit)))
                .sum(),
            animal_count: (detailed.rabbits.len() + detailed.wildlife.len()) as u32,
        };
        let atmosphere = AtmosphereSnapshot {
            cloud_count: detailed.clouds.len() as u32,
            cloud_water: detailed.clouds.iter().map(|cloud| cloud.rain).sum(),
            cumulative_rainfall: self.atmosphere.cumulative_rainfall,
        };

        Self {
            tick: self.tick,
            atmosphere,
            hydrology: self.hydrology,
            ecology,
            detailed_ecology: detailed,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TickReport {
    pub tick: SimulationTick,
    pub ecology: EcoTickReport,
    pub snapshot: NatureSnapshot,
    pub events: Vec<NatureEvent>,
}

/// Advance the existing authoritative natural-world cycle exactly once.
///
/// This is intentionally an adapter over `EcoCycle::tick`, not a replacement
/// implementation. All authorities and tests should converge on this entry point.
pub fn step_world(
    input: WorldInput,
    ecology: &mut EcoCycle,
    resources: &mut GlobalResourcePool,
) -> TickReport {
    step_world_elapsed(input, 1, ecology, resources)
}

/// Advances the ecology by several simulation ticks while publishing one
/// report for the caller's current world tick. This is the seam used by
/// coarse-grained/off-screen simulation: callers can catch an inactive region
/// up without creating a second set of ecology rules.
pub fn step_world_elapsed(
    input: WorldInput,
    elapsed_ticks: u32,
    ecology: &mut EcoCycle,
    resources: &mut GlobalResourcePool,
) -> TickReport {
    let mut report = EcoTickReport::default();
    let mut events = Vec::new();
    let step_count = elapsed_ticks.max(1);
    let first_tick = input
        .tick
        .saturating_sub(u64::from(step_count.saturating_sub(1)));
    for offset in 0..step_count {
        let tick_report = ecology.tick_at(first_tick + u64::from(offset), resources);
        events.extend(events_from_report(tick_report));
        report.rain_fell += tick_report.rain_fell;
        report.plants_grown += tick_report.plants_grown;
        report.rabbits_born += tick_report.rabbits_born;
        report.wildlife_born += tick_report.wildlife_born;
        report.fruit_eaten += tick_report.fruit_eaten;
        report.fruit_grown += tick_report.fruit_grown;
        report.food_produced += tick_report.food_produced;
        report.apples_reserved += tick_report.apples_reserved;
    }
    TickReport {
        tick: input.tick,
        ecology: report,
        snapshot: NatureSnapshot::from_ecology(input.tick, ecology),
        events,
    }
}

fn events_from_report(report: EcoTickReport) -> Vec<NatureEvent> {
    let mut events = Vec::with_capacity(4);
    if report.rain_fell > 0.0 {
        events.push(NatureEvent::RainFell {
            amount: report.rain_fell,
        });
    }
    if report.plants_grown > 0 {
        events.push(NatureEvent::PlantsGrown {
            count: report.plants_grown,
        });
    }
    if report.rabbits_born > 0 || report.wildlife_born > 0 {
        events.push(NatureEvent::AnimalsBorn {
            rabbits: report.rabbits_born,
            wildlife: report.wildlife_born,
        });
    }
    if report.fruit_eaten > 0 || report.fruit_grown > 0 {
        events.push(NatureEvent::FruitChanged {
            eaten: report.fruit_eaten,
            grown: report.fruit_grown,
        });
    }
    events
}
