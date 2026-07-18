//! Offline authority: holds the live `EcoCycle` and steps it via the shared
//! `step_world` entry point. The presentation layer reads the resulting
//! `NatureSnapshot` to drive visuals.

use bevy::prelude::*;
use lk2_core::constant;
use lk2_core::ecology::EcoCycle;
use lk2_core::farming::FarmingState;
use lk2_core::resource::GlobalResourcePool;
use lk2_core::settlement::{CAMP_STONE_COST, CAMP_WOOD_COST, SettlementPhase, SettlementState};
use lk2_core::simulation::{NatureSnapshot, WorldInput, step_world};
use lk2_core::world::{World as GameWorld, terrain};

use crate::nature::NatureSnapshotBuffer;

const LIVING_ECOLOGY_CENTER: Vec2 = Vec2::new(9.0, -4.0);

/// The offline scene's deliberately small, finishable player loop.
///
/// This is presentation-facing progression state; gathering, settlement,
/// combat, and ecology remain owned by their existing authoritative state.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OfflineQuestPhase {
    #[default]
    Explore,
    CampReady,
    Establishing,
    HuntGuardian,
    Victory,
}

impl OfflineQuestPhase {
    pub fn advance(
        &mut self,
        wood: i64,
        stone: i64,
        settlement: SettlementPhase,
        guardian_defeated: bool,
    ) {
        if *self == Self::Victory {
            return;
        }
        if settlement == SettlementPhase::Failed {
            *self = Self::Explore;
            return;
        }
        if guardian_defeated && settlement == SettlementPhase::Established {
            *self = Self::Victory;
        } else if guardian_defeated {
            *self = Self::Establishing;
        } else if matches!(
            settlement,
            SettlementPhase::Camp | SettlementPhase::Established
        ) {
            *self = Self::HuntGuardian;
        } else if wood >= CAMP_WOOD_COST && stone >= CAMP_STONE_COST {
            *self = Self::CampReady;
        } else {
            *self = Self::Explore;
        }
    }
}

#[derive(Resource)]
pub struct OfflineNature {
    pub tick: u64,
    pub accumulated_secs: f32,
    pub ecology: EcoCycle,
    pub resources: GlobalResourcePool,
    pub farming: FarmingState,
    pub settlement: SettlementState,
    pub snapshot: NatureSnapshot,
    pub initial_snapshot: NatureSnapshot,
    pub buffer: NatureSnapshotBuffer,
    pub terrain_world: GameWorld,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_quest_moves_from_gathering_to_victory() {
        let mut phase = OfflineQuestPhase::default();
        phase.advance(12, 6, SettlementPhase::Wilderness, false);
        assert_eq!(phase, OfflineQuestPhase::CampReady);

        phase.advance(0, 0, SettlementPhase::Camp, false);
        assert_eq!(phase, OfflineQuestPhase::HuntGuardian);

        phase.advance(0, 0, SettlementPhase::Camp, true);
        assert_eq!(phase, OfflineQuestPhase::Establishing);

        phase.advance(0, 0, SettlementPhase::Established, false);
        assert_eq!(phase, OfflineQuestPhase::HuntGuardian);

        phase.advance(0, 0, SettlementPhase::Established, true);
        assert_eq!(phase, OfflineQuestPhase::Victory);
    }

    #[test]
    fn failed_camp_restarts_the_gathering_goal() {
        let mut phase = OfflineQuestPhase::HuntGuardian;
        phase.advance(0, 0, SettlementPhase::Failed, false);
        assert_eq!(phase, OfflineQuestPhase::Explore);
    }

    #[test]
    fn playable_settlement_can_reach_established_without_manual_food_injection() {
        let mut nature = OfflineNature::playable();
        nature
            .resources
            .force_add(lk2_core::resource::ResourceKind::Wood, CAMP_WOOD_COST);
        nature
            .resources
            .force_add(lk2_core::resource::ResourceKind::Stone, CAMP_STONE_COST);
        nature
            .settlement
            .build_camp(&mut nature.resources)
            .expect("playable resources should allow the camp to start");

        for _ in 0..lk2_core::settlement::SETTLEMENT_GOAL_TICKS {
            nature.advance(constant::SLOW_TICK_SECS);
        }

        assert_eq!(nature.settlement.phase, SettlementPhase::Established);
    }
}

fn initial_terrain_world() -> GameWorld {
    GameWorld::with_pipeline(constant::WORLD_SIZE, terrain::presets::default_preset())
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
            settlement: SettlementState::default(),
            initial_snapshot: snapshot.clone(),
            snapshot,
            buffer: NatureSnapshotBuffer::default(),
            terrain_world: initial_terrain_world(),
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
            settlement: SettlementState::default(),
            initial_snapshot: snapshot.clone(),
            snapshot,
            buffer: NatureSnapshotBuffer::default(),
            terrain_world: initial_terrain_world(),
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
            let _ = self.settlement.advance_tick(&mut self.resources);
            steps += 1;
        }
        steps
    }
}

/// Bevy system that steps the offline authority each frame.
pub fn advance_offline_nature(time: Res<Time>, mut nature: ResMut<OfflineNature>) {
    nature.advance(time.delta_secs());
}

/// Projects existing offline authority state into the player's finishable
/// quest. This runs after combat and camp input so the HUD reflects the same
/// frame's successful action.
pub fn update_offline_quest(
    nature: Res<OfflineNature>,
    mut quest: ResMut<OfflineQuestPhase>,
    guardians: Query<&lk2_core::pvp::Health, With<super::state::BossActor>>,
) {
    let guardian_defeated = guardians
        .iter()
        .next()
        .is_some_and(|health| health.is_dead());
    quest.advance(
        nature.resources.get(lk2_core::resource::ResourceKind::Wood),
        nature
            .resources
            .get(lk2_core::resource::ResourceKind::Stone),
        nature.settlement.phase,
        guardian_defeated,
    );
}
