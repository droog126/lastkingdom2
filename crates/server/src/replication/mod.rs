//! Read-only replication boundary. It owns no simulation state.

use bevy::prelude::*;
use lightyear::prelude::{ControlledBy, Replicate, ReplicationSender};
use lk2_core::protocol::components::{CartState, PlayerPos};
use lk2_core::simulation::{NatureEvent, NatureSnapshot, TickReport};
use lk2_core::world::region::WorldRegion;
use serde::{Deserialize, Serialize};

use super::authority::{LatestNatureReport, NatureAuthoritySet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NatureSnapshotDto<T> {
    pub tick: u64,
    pub value: T,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NatureEventDto<E> {
    Upsert { id: u64, value: E },
    Remove { id: u64 },
}

pub fn snapshot<T>(tick: u64, value: T) -> NatureSnapshotDto<T> {
    NatureSnapshotDto { tick, value }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplicationBatch<S, E> {
    pub snapshot: NatureSnapshotDto<S>,
    pub events: Vec<E>,
}

impl<S, E> ReplicationBatch<S, E> {
    pub fn new(tick: u64, value: S, events: Vec<E>) -> Self {
        Self {
            snapshot: snapshot(tick, value),
            events,
        }
    }

    pub fn tick(&self) -> u64 {
        self.snapshot.tick
    }
}

impl ReplicationBatch<NatureSnapshot, NatureEvent> {
    pub fn from_tick_report(report: TickReport) -> Self {
        Self::new(report.tick, report.snapshot, report.events)
    }

    pub fn from_tick_report_in_region(report: TickReport, center: [f32; 2], radius: f32) -> Self {
        Self::new(
            report.tick,
            report.snapshot.for_region(center, radius),
            report
                .events
                .iter()
                .filter_map(|event| event.for_region(center, radius))
                .collect(),
        )
    }
}

#[derive(Resource, Default)]
pub struct LatestNatureReplication(pub Option<ReplicationBatch<NatureSnapshot, NatureEvent>>);

pub struct NatureReplicationPlugin;

impl Plugin for NatureReplicationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LatestNatureReplication>().add_systems(
            FixedUpdate,
            build_replication_batch.in_set(NatureAuthoritySet::PublishReport),
        );
    }
}

/// Replicates player entities only to clients whose gameplay region overlaps
/// the player's region. This is the first AOI layer for the large world: it
/// keeps strategic entities region-scoped without pretending that every unit
/// needs a high-frequency physics stream.
pub struct PlayerInterestPlugin;

pub const DEFAULT_PLAYER_INTEREST_REGION_RADIUS: i32 = 2;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerInterestConfig {
    pub region_radius: i32,
}

impl Default for PlayerInterestConfig {
    fn default() -> Self {
        Self {
            region_radius: DEFAULT_PLAYER_INTEREST_REGION_RADIUS,
        }
    }
}

#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
struct InterestRecipients(Vec<Entity>);

impl Plugin for PlayerInterestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInterestConfig>()
            .add_systems(Update, update_player_replication_interest);
    }
}

fn update_player_replication_interest(
    mut commands: Commands,
    config: Res<PlayerInterestConfig>,
    connections: Query<Entity, With<ReplicationSender>>,
    owned_players: Query<(Entity, &PlayerPos, &ControlledBy), With<PlayerPos>>,
    replicated_players: Query<
        (
            Entity,
            &PlayerPos,
            Option<&ControlledBy>,
            &Replicate,
            Option<&InterestRecipients>,
        ),
        With<PlayerPos>,
    >,
    replicated_carts: Query<(Entity, &CartState, &Replicate, Option<&InterestRecipients>)>,
) {
    let observer_regions = owned_players
        .iter()
        .filter_map(|(_, position, owner)| {
            position
                .is_finite()
                .then(|| (owner.owner, WorldRegion::from_world_position(position.0)))
        })
        .collect::<std::collections::HashMap<_, _>>();

    if observer_regions.is_empty() {
        return;
    }

    let connections = connections.iter().collect::<Vec<_>>();
    for (entity, position, owner, _replicate, previous) in replicated_players.iter() {
        update_interest_target(
            &mut commands,
            entity,
            position.0,
            owner.map(|owner| owner.owner),
            previous,
            &connections,
            &observer_regions,
            config.region_radius,
        );
    }
    for (entity, cart, _replicate, previous) in replicated_carts.iter() {
        update_interest_target(
            &mut commands,
            entity,
            cart.position,
            None,
            previous,
            &connections,
            &observer_regions,
            config.region_radius,
        );
    }
}

fn update_interest_target(
    commands: &mut Commands,
    entity: Entity,
    position: Vec3,
    owner: Option<Entity>,
    previous: Option<&InterestRecipients>,
    connections: &[Entity],
    observer_regions: &std::collections::HashMap<Entity, WorldRegion>,
    region_radius: i32,
) {
    if !position.is_finite() {
        return;
    }
    let entity_region = WorldRegion::from_world_position(position);
    let mut recipients = connections
        .iter()
        .copied()
        .filter(|connection| {
            owner.is_some_and(|owner| owner == *connection)
                || observer_regions
                    .get(connection)
                    .is_some_and(|observer_region| {
                        entity_region.within(*observer_region, region_radius)
                    })
        })
        .collect::<Vec<_>>();
    recipients.sort_unstable_by_key(|entity| entity.to_bits());

    if previous.is_some_and(|previous| previous.0 == recipients) {
        return;
    }

    // ReplicationTarget uses component hooks to update Lightyear's visibility
    // bitset. Replacing the component is intentional; mutating its private
    // mode in place would not notify those hooks.
    commands
        .entity(entity)
        .remove::<Replicate>()
        .insert(Replicate::manual(recipients.clone()))
        .insert(InterestRecipients(recipients));
}

fn build_replication_batch(
    report: Res<LatestNatureReport>,
    mut replication: ResMut<LatestNatureReplication>,
) {
    let Some(report) = report.0.as_ref() else {
        return;
    };
    if replication
        .0
        .as_ref()
        .is_some_and(|batch| batch.tick() == report.tick)
    {
        return;
    }
    replication.0 = Some(ReplicationBatch::from_tick_report(report.clone()));
}
