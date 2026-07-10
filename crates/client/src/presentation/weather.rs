//! Weather state derived from authoritative rain values.

use bevy::prelude::Resource;
use lk2_core::protocol::components::EcoSnapshot;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct WeatherPresentation {
    pub snapshot_tick: u64,
    pub rain_intensity: f32,
    pub accumulated_rainfall: f32,
}

impl WeatherPresentation {
    pub fn apply(&mut self, snapshot: &EcoSnapshot) {
        self.snapshot_tick = snapshot.tick;
        self.rain_intensity = snapshot.rain.max(0.0);
        self.accumulated_rainfall = snapshot.rainfall.max(0.0);
    }
}
