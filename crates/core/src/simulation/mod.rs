use serde::{Deserialize, Serialize};

use crate::atmosphere::AtmosphereSnapshot;
use crate::ecology::nature::NatureEcologySnapshot;
use crate::ecology::{EcoCycle, EcoTickReport};
use crate::hydrology::HydrologySnapshot;
use crate::protocol::components::EcoSnapshot;
use crate::resource::GlobalResourcePool;

pub mod app_sets;
mod authority;

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
    let report = ecology.tick(resources);
    let events = events_from_report(report);
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
