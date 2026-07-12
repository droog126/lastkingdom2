//! Offline authority: holds the live `EcoCycle` and steps it via the shared
//! `step_world` entry point. The presentation layer reads the resulting
//! `NatureSnapshot` to drive visuals.

use bevy::prelude::*;
use lk2_core::constant;
use lk2_core::ecology::EcoCycle;
use lk2_core::farming::FarmingState;
use lk2_core::resource::GlobalResourcePool;
use lk2_core::simulation::{step_world, NatureSnapshot, WorldInput};

use crate::nature::NatureSnapshotBuffer;

const LIVING_ECOLOGY_CENTER: Vec2 = Vec2::new(9.0, -4.0);

#[derive(Resource)]
pub struct OfflineNature {
    pub tick: u64,
    pub accumulated_secs: f32,
    pub ecology: EcoCycle,
    pub resources: GlobalResourcePool,
    pub farming: FarmingState,
    pub snapshot: NatureSnapshot,
    pub initial_snapshot: NatureSnapshot,
    pub buffer: NatureSnapshotBuffer,
}

impl Default for OfflineNature {
    fn default() -> Self {
        let ecology = EcoCycle::seeded_weather_at(LIVING_ECOLOGY_CENTER);
        let snapshot = NatureSnapshot::from_ecology(0, &ecology);
        Self {
            tick: 0,
            accumulated_secs: 0.0,
            ecology,
            resources: GlobalResourcePool::new(),
            farming: FarmingState::new(3),
            initial_snapshot: snapshot.clone(),
            snapshot,
            buffer: NatureSnapshotBuffer::default(),
        }
    }
}

impl OfflineNature {
    #[must_use]
    pub fn playable() -> Self {
        let ecology = EcoCycle::demo_at(LIVING_ECOLOGY_CENTER);
        let snapshot = NatureSnapshot::from_ecology(0, &ecology);
        let mut resources = GlobalResourcePool::new();
        resources.force_add(lk2_core::resource::ResourceKind::WheatSeeds, 3);
        resources.force_add(lk2_core::resource::ResourceKind::Carrot, 2);
        resources.force_add(lk2_core::resource::ResourceKind::Potato, 2);
        Self {
            tick: 0,
            accumulated_secs: 0.0,
            ecology,
            resources,
            farming: FarmingState::new(3),
            initial_snapshot: snapshot.clone(),
            snapshot,
            buffer: NatureSnapshotBuffer::default(),
        }
    }

    pub fn advance(&mut self, delta_secs: f32) -> u32 {
        self.accumulated_secs += delta_secs.max(0.0);
        let mut steps = 0;
        while self.accumulated_secs + f32::EPSILON >= constant::SLOW_TICK_SECS {
            self.accumulated_secs -= constant::SLOW_TICK_SECS;
            self.tick += 1;
            let report = step_world(
                WorldInput { tick: self.tick },
                &mut self.ecology,
                &mut self.resources,
            );
            let _ = self
                .buffer
                .push_with_events(report.snapshot.clone(), report.events);
            self.snapshot = self.buffer.latest().cloned().unwrap_or(report.snapshot);
            self.farming.advance_tick();
            steps += 1;
        }
        steps
    }
}

/// Bevy system that steps the offline authority each frame.
pub fn advance_offline_nature(time: Res<Time>, mut nature: ResMut<OfflineNature>) {
    nature.advance(time.delta_secs());
}
