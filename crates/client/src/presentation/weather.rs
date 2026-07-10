//! Weather state and transient cues derived from authoritative nature output.

use bevy::prelude::Resource;
use lk2_core::simulation::{NatureEvent, NatureSnapshot};

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct WeatherPresentation {
    pub snapshot_tick: u64,
    pub rain_intensity: f32,
    pub accumulated_rainfall: f32,
}

impl WeatherPresentation {
    pub fn apply(&mut self, snapshot: &NatureSnapshot) {
        self.snapshot_tick = snapshot.tick;
        self.rain_intensity = if snapshot.atmosphere.cloud_count == 0 {
            0.0
        } else {
            snapshot.atmosphere.cloud_water / snapshot.atmosphere.cloud_count as f32
        };
        self.accumulated_rainfall = snapshot.atmosphere.cumulative_rainfall;
    }
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct NatureEventPresentation {
    pub rain_fell: f32,
    pub plants_grown: u32,
    pub rabbits_born: u32,
    pub wildlife_born: u32,
    pub fruit_eaten: u32,
    pub fruit_grown: u32,
}

impl NatureEventPresentation {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn apply(&mut self, event: NatureEvent) {
        match event {
            NatureEvent::RainFell { amount } => self.rain_fell += amount,
            NatureEvent::PlantsGrown { count } => self.plants_grown += count,
            NatureEvent::AnimalsBorn { rabbits, wildlife } => {
                self.rabbits_born += rabbits;
                self.wildlife_born += wildlife;
            }
            NatureEvent::FruitChanged { eaten, grown } => {
                self.fruit_eaten += eaten;
                self.fruit_grown += grown;
            }
        }
    }
}
