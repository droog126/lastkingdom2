//! Sky state derived from authoritative cloud snapshots.

use bevy::prelude::Resource;
use lk2_core::protocol::components::EcoSnapshot;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct SkyPresentation {
    pub snapshot_tick: u64,
    pub cloud_count: usize,
    pub cloud_cover: f32,
}

impl SkyPresentation {
    pub fn apply(&mut self, snapshot: &EcoSnapshot) {
        self.snapshot_tick = snapshot.tick;
        self.cloud_count = snapshot.clouds.len();
        self.cloud_cover = (snapshot.clouds.len() as f32 / 8.0).clamp(0.0, 1.0);
    }
}
