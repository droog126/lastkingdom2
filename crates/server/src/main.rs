#![allow(dead_code)]
#![allow(unused_imports)]

use avian3d::prelude::{Collider, PhysicsPlugins, Position, RigidBody, Rotation};
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::LocalAddr;
use lightyear::prelude::server::ServerUdpIo;
use lightyear_avian3d::prelude::LightyearAvianPlugin;

use leafwing_input_manager::prelude::ActionState;
use lk2_core::protocol::components::{
    CartMounted, CartState, CreatureState, EcoSnapshot, GameplayHudState, PlayerPos, SwimmingState,
    VOXEL_CHUNK_SIZE_XZ, VoxelChunkSnapshot, VoxelDelta,
};
use lk2_core::protocol::messages::{
    BuildRecipe, ChatBroadcast, ChatMessage, GameplayCommand, GameplayCommandKind,
    GameplayFeedback, PingMessage, PongMessage, TerrainSnapshotRequest,
};
use lk2_core::protocol::{ControlChannel, PlayerAction, StateChannel};
use std::collections::{HashMap, HashSet};

use lightyear::prelude::PeerMetadata;

use std::time::Duration;

mod app;
mod authority;
mod los;
mod observation;
mod persistence;
mod replication;

use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::constant;
use lk2_core::ecology::EcoCycle;
use lk2_core::ecology::animals::{
    CREATURE_INITIAL_WANDER_SECS, CREATURE_TRAINING_ATTACK_RANGE_SQ, Creature, CreatureAI,
    CreatureKind, CreatureSpawnerDone, award_creature_drop, creature_attack_distance_sq,
};
use lk2_core::ecology::threats::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::{PlayerState, PlayerStateComponent, PlayerTag};
use lk2_core::pvp::{FixedTick, Health, Hitbox, PvpCombatant, SimpleWeapon};
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::scenario::{Scenario, ScenarioState};
use lk2_core::simulation::app_sets::SimSet;
use lk2_core::simulation::regions::{NatureRegionId, NatureRegionState, NatureRegionWorld};
use lk2_core::simulation::{SimRole, advance_fixed_authority_tick_report};
use lk2_core::transport::{
    DEFAULT_PORT, NETCODE_CLIENT_TIMEOUT_SECS, PRIVATE_KEY, PROTOCOL_ID,
    SERVER_POS_UPDATE_INTERVAL_TICKS,
};
use lk2_core::vehicle::{
    CartDriveInput, CartDriveState, cart_steering_from_right_input, step_cart_drive,
};
use lk2_core::world::{
    TERRAIN_CHUNK_SIZE, TerrainChunkCoord, World as GameWorld, WorldConfig, WorldGenerator,
    fallback_spawn_ring_offsets, generate_world, player_body_clear, player_position_is_safe,
    player_spawn_position_at, player_spawn_position_near, player_stand_position_at,
    resolve_player_stuck_near,
};

use crate::app::NatureServerProjectionPlugin;
use crate::authority::pvp::SimplePvpAuthorityPlugin;
use crate::authority::{
    LatestNatureRegionReports, LatestNatureReport, NatureAuthoritySet, report_snapshot_for_region,
};
use crate::persistence::LatestNatureRegionSaves;
use crate::persistence::LatestNatureSave;

const PLACE_WOOD_COST: i64 = 1;
const PLANK_PACK_WOOD_COST: i64 = 5;
const PLANK_PACK_OUTPUT: i64 = 1;
const ONLINE_MOVE_SPEED: f32 = 4.5;
const ONLINE_INPUT_MAX_MAGNITUDE: f32 = 1.5;
const RESPAWN_DELAY_TICKS: u32 = 90;
const ONLINE_STEP_THRESHOLD: f32 = 0.85;
const PLAYER_COLLISION_RADIUS: f32 = 0.34;
const JUMP_TAKEOFF_SPEED: f32 = 7.2;
const JUMP_GRAVITY: f32 = 28.0;
const JUMP_TERMINAL_SPEED: f32 = -18.0;
const SWIM_MIN_DEPTH: f32 = 0.85;
const SWIM_VERTICAL_SPEED: f32 = 3.0;
const SWIM_SURFACE_CLEARANCE: f32 = 0.05;
const ANTI_STUCK_SEARCH_RADIUS: i32 = 6;
const ANTI_STUCK_FAILED_MOVE_LIMIT: u8 = 12;
const TERRAIN_STREAM_RADIUS_CHUNKS: i32 = 3;

#[derive(Component, Debug, Clone, Copy)]
struct ServerJumpState {
    velocity_y: f32,
    grounded: bool,
}

#[derive(Component, Debug, Clone, Copy, Default)]
struct ServerCartDrive {
    state: CartDriveState,
}

#[derive(Component, Debug, Clone, Copy, Default)]
struct RespawnState {
    ticks_remaining: u32,
}

impl Default for ServerJumpState {
    fn default() -> Self {
        Self {
            velocity_y: 0.0,
            grounded: true,
        }
    }
}

#[derive(Component, Debug, Clone)]
struct AntiStuckState {
    last_safe_pos: Vec3,
    last_safe_block: [i32; 3],
    failed_move_ticks: u8,
}

impl Default for AntiStuckState {
    fn default() -> Self {
        Self {
            last_safe_pos: Vec3::ZERO,
            last_safe_block: [0, 0, 0],
            failed_move_ticks: 0,
        }
    }
}

#[derive(Resource, Default)]
struct WorldRevision(u64);

#[derive(Resource, Default)]
struct NextPlayerId(u32);

/// Commands issued by connection observers are deferred. Reserve a player
/// immediately so two ClientOf observers in the same command flush cannot
/// claim the same unowned startup player.
#[derive(Resource, Default)]
struct ReservedPlayerEntities(HashSet<Entity>);

#[derive(Resource, Default)]
struct DisconnectedPlayerSessions(HashMap<u64, DisconnectedPlayerSession>);

#[derive(Clone, Debug)]
struct DisconnectedPlayerSession {
    state: PlayerState,
    mounted: bool,
    cart: Option<CartState>,
}

/// Highest client command sequence accepted for one connection. The tick is
/// simulation metadata and is not unique: several legitimate actions can
/// share one tick. Sequence is the replay/deduplication identity.
#[derive(Component, Default, Debug, Clone, Copy)]
struct LastClientCommandSequence(u64);

fn accept_client_command_sequence(last: &mut u64, sequence: u64) -> bool {
    if sequence == 0 || sequence <= *last {
        return false;
    }
    *last = sequence;
    true
}

#[derive(Resource, Default)]
pub struct ServerCommandDiagnostics {
    pub receiver_entities: usize,
    pub server_entities: usize,
    pub started_servers: usize,
    pub link_of_entities: usize,
    pub move_world_received: u64,
    pub last_dx_milli: i16,
    pub last_dz_milli: i16,
    pub last_move_applied: bool,
}

#[derive(Resource, Default, Debug)]
pub struct ServerConnectionDiagnostics {
    pub active_connections: usize,
    pub connected_total: u64,
    pub disconnected_total: u64,
    pub last_disconnect_reason: Option<String>,
}

impl ServerConnectionDiagnostics {
    fn record_connected(&mut self) {
        self.active_connections = self.active_connections.saturating_add(1);
        self.connected_total = self.connected_total.saturating_add(1);
    }

    fn record_disconnected(&mut self, reason: Option<&str>) {
        if self.active_connections == 0 {
            return;
        }
        self.active_connections -= 1;
        self.disconnected_total = self.disconnected_total.saturating_add(1);
        self.last_disconnect_reason = reason.map(str::to_owned);
    }
}

#[derive(Resource, Clone)]
struct LastVoxelDeltaState {
    revision: u64,
    x: i32,
    y: i32,
    z: i32,
    block: lk2_core::world::BlockType,
}

#[derive(Resource, Default)]
struct VoxelChunkSnapshotCache {
    revision: u64,
    snapshots: HashMap<(i32, i32), VoxelChunkSnapshot>,
}

const SERVER_TERRAIN_RESYNC_COOLDOWN_TICKS: u64 = 15;

#[derive(Resource, Default)]
struct VoxelSnapshotResyncRequests {
    pending: HashMap<Entity, TerrainSnapshotRequest>,
    last_accepted_tick: HashMap<Entity, u64>,
}

impl VoxelSnapshotResyncRequests {
    fn accept(&mut self, owner: Entity, tick: u64, request: TerrainSnapshotRequest) -> bool {
        if request.request_id == 0 || self.pending.contains_key(&owner) {
            return false;
        }
        if self
            .last_accepted_tick
            .get(&owner)
            .is_some_and(|last| tick.saturating_sub(*last) < SERVER_TERRAIN_RESYNC_COOLDOWN_TICKS)
        {
            return false;
        }
        self.last_accepted_tick.insert(owner, tick);
        self.pending.insert(owner, request);
        true
    }
}

impl Default for LastVoxelDeltaState {
    fn default() -> Self {
        Self {
            revision: 0,
            x: 0,
            y: 0,
            z: 0,
            block: lk2_core::world::BlockType::Air,
        }
    }
}

fn block_type_to_u8(block: lk2_core::world::BlockType) -> u8 {
    block.to_wire_u8()
}

fn build_gameplay_hud_state(
    clock: &SimClock,
    player: &PlayerState,
    pool: &GlobalResourcePool,
    nations: &NationRegistry,
    monsters: &MonsterEcosystem,
    obs: &TickObserver,
) -> GameplayHudState {
    GameplayHudState {
        tick: clock.tick,
        player_block_pos: player.block_pos,
        player_pos: [player.pos.x, player.pos.y, player.pos.z],
        nation_id: player.nation_id.map(|id| id.0),
        monsters_killed: player.monsters_killed,
        blocks_gathered: player.blocks_gathered,
        nations_founded: player.nations_founded,
        inventory_wood: player
            .inventory
            .get(&ResourceKind::Wood)
            .copied()
            .unwrap_or(0),
        inventory_food: player
            .inventory
            .get(&ResourceKind::Food)
            .copied()
            .unwrap_or(0),
        inventory_apple: player
            .inventory
            .get(&ResourceKind::Apple)
            .copied()
            .unwrap_or(0),
        inventory_soul: player
            .inventory
            .get(&ResourceKind::Soul)
            .copied()
            .unwrap_or(0),
        pool_wood: pool.get(ResourceKind::Wood),
        pool_food: pool.get(ResourceKind::Food),
        pool_apple: pool.get(ResourceKind::Apple),
        pool_soul: pool.get(ResourceKind::Soul),
        flag_count: nations.flag_count,
        total_nations: nations.nations.len() as u32,
        monster_count: monsters.current_individuals,
        observer_anomalies: obs.anomalies.len() as u64,
        observer_invariant_violations: lk2_core::diagnostics::total_invariant_violations(obs),
        status_line: format!(
            "server tick={} pos={:?} flags={} monsters={}",
            clock.tick, player.block_pos, nations.flag_count, monsters.current_individuals
        ),
    }
}

fn empty_gameplay_hud_state() -> GameplayHudState {
    GameplayHudState {
        tick: 0,
        player_block_pos: [0, 0, 0],
        player_pos: [0.0, 0.0, 0.0],
        nation_id: None,
        monsters_killed: 0,
        blocks_gathered: 0,
        nations_founded: 0,
        inventory_wood: 0,
        inventory_food: 0,
        inventory_apple: 0,
        inventory_soul: 0,
        pool_wood: 0,
        pool_food: 0,
        pool_apple: 0,
        pool_soul: 0,
        flag_count: 0,
        total_nations: 0,
        monster_count: 0,
        observer_anomalies: 0,
        observer_invariant_violations: 0,
        status_line: String::new(),
    }
}

fn empty_voxel_delta() -> VoxelDelta {
    VoxelDelta {
        revision: 0,
        x: 0,
        y: 0,
        z: 0,
        block: block_type_to_u8(lk2_core::world::BlockType::Air),
    }
}

fn empty_voxel_chunk_snapshot() -> VoxelChunkSnapshot {
    VoxelChunkSnapshot {
        revision: 0,
        chunk_x: 0,
        chunk_z: 0,
        border: 0,
        y_min: 0,
        y_size: 0,
        blocks: Vec::new(),
    }
}

fn build_voxel_chunk_snapshot(
    world: &GameWorld,
    revision: u64,
    player_block_pos: [i32; 3],
) -> VoxelChunkSnapshot {
    const BORDER: i32 = 1;
    const SNAPSHOT_SIZE_XZ: i32 = VOXEL_CHUNK_SIZE_XZ + BORDER * 2;
    let chunk_x = player_block_pos[0].div_euclid(VOXEL_CHUNK_SIZE_XZ);
    let chunk_z = player_block_pos[2].div_euclid(VOXEL_CHUNK_SIZE_XZ);
    let x_min = chunk_x * VOXEL_CHUNK_SIZE_XZ - BORDER;
    let z_min = chunk_z * VOXEL_CHUNK_SIZE_XZ - BORDER;
    let y_min = 0;
    let y_size = world.size;
    let mut blocks = Vec::with_capacity((SNAPSHOT_SIZE_XZ * SNAPSHOT_SIZE_XZ * y_size) as usize);

    for y in y_min..(y_min + y_size) {
        for z in z_min..(z_min + SNAPSHOT_SIZE_XZ) {
            for x in x_min..(x_min + SNAPSHOT_SIZE_XZ) {
                blocks.push(block_type_to_u8(world.get(x, y, z)));
            }
        }
    }

    VoxelChunkSnapshot {
        revision,
        chunk_x,
        chunk_z,
        border: BORDER,
        y_min,
        y_size,
        blocks,
    }
}

fn cached_voxel_chunk_snapshot<'a>(
    world: &GameWorld,
    revision: u64,
    player_block_pos: [i32; 3],
    cache: &'a mut VoxelChunkSnapshotCache,
) -> &'a VoxelChunkSnapshot {
    if cache.revision != revision {
        cache.revision = revision;
        cache.snapshots.clear();
    }
    let chunk = (
        player_block_pos[0].div_euclid(VOXEL_CHUNK_SIZE_XZ),
        player_block_pos[2].div_euclid(VOXEL_CHUNK_SIZE_XZ),
    );
    cache
        .snapshots
        .entry(chunk)
        .or_insert_with(|| build_voxel_chunk_snapshot(world, revision, player_block_pos))
}

fn record_voxel_delta(
    revision: &mut WorldRevision,
    last_delta: &mut LastVoxelDeltaState,
    x: i32,
    y: i32,
    z: i32,
    block: lk2_core::world::BlockType,
) {
    revision.0 = revision.0.wrapping_add(1).max(1);
    *last_delta = LastVoxelDeltaState {
        revision: revision.0,
        x,
        y,
        z,
        block,
    };
}

fn sanitize_online_move_direction(dx_milli: i16, dz_milli: i16) -> Option<Vec2> {
    let direction = Vec2::new(dx_milli as f32 / 1000.0, dz_milli as f32 / 1000.0);
    if direction.length_squared() <= 0.0001 {
        return None;
    }
    Some(direction.clamp_length_max(ONLINE_INPUT_MAX_MAGNITUDE))
}

fn sanitize_swim_vertical(dy_milli: i16) -> f32 {
    (dy_milli as f32 / 1000.0).clamp(-1.0, 1.0)
}

fn water_floor_foot_y(world: &GameWorld, x: i32, z: i32) -> Option<i32> {
    if world.get(x, constant::SEA_LEVEL, z) != lk2_core::world::BlockType::Water {
        return None;
    }
    (1..=constant::SEA_LEVEL).rev().find(|foot_y| {
        world.get(x, *foot_y - 1, z).is_solid() && player_body_clear(world, x, *foot_y, z)
    })
}

fn water_depth_at(world: &GameWorld, position: Vec3) -> Option<f32> {
    let x = position.x.floor() as i32;
    let z = position.z.floor() as i32;
    let floor_y = water_floor_foot_y(world, x, z)?;
    Some((constant::SEA_LEVEL + 1 - floor_y).max(0) as f32)
}

fn can_swim_at(world: &GameWorld, position: Vec3) -> bool {
    water_depth_at(world, position).is_some_and(|depth| {
        depth >= SWIM_MIN_DEPTH && position.y <= constant::SEA_LEVEL as f32 + 1.0 + 0.9
    })
}

fn player_inventory_can_receive(player: &PlayerState, kind: ResourceKind, amount: i64) -> bool {
    let current = player.inventory.get(&kind).copied().unwrap_or(0);
    amount >= 0 && amount <= kind.max() && current >= 0 && current <= kind.max() - amount
}

fn terrain_editable(world: &GameWorld, x: i32, y: i32, z: i32) -> bool {
    y >= 0
        && y < world.size
        && (world.procedural || (0..world.size).contains(&x) && (0..world.size).contains(&z))
}

fn apply_gameplay_command(
    player_id: u32,
    cmd: &GameplayCommand,
    world: &mut GameWorld,
    pool: &mut GlobalResourcePool,
    nations: &mut NationRegistry,
    player: &mut PlayerState,
    revision: &mut WorldRevision,
    last_delta: &mut LastVoxelDeltaState,
    nearest_creature: Option<CreatureKind>,
) -> GameplayFeedback {
    let mut ok = true;
    let summary = match cmd.kind {
        GameplayCommandKind::MoveWorld { .. } => {
            ok = false;
            "movement command was not routed to entity transform".to_string()
        }
        GameplayCommandKind::Jump => {
            ok = false;
            "jump command was not routed to entity transform".to_string()
        }
        GameplayCommandKind::MineTarget { target } => {
            let [x, y, z] = target;
            let [px, py, pz] = player.block_pos;
            // Keep client-supplied coordinates in a wide type before doing
            // reachability arithmetic. An i32 subtraction/multiplication here
            // would let a malformed remote command panic the server before
            // the bounds check runs.
            let reachable = lk2_core::world::mine_target_reachable([px, py, pz], [x, y, z]);
            if !terrain_editable(world, x, y, z) {
                ok = false;
                "mine target out of bounds".to_string()
            } else if !reachable {
                ok = false;
                "mine target is out of reach".to_string()
            } else if let Some((kind, amount)) = world.get(x, y, z).yields()
                && !player_inventory_can_receive(player, kind, amount)
            {
                ok = false;
                format!("inventory full for {:?}", kind)
            } else {
                match lk2_core::world::mine_block(world, pool, x, y, z, player_id) {
                    Ok(Some((block, drop))) => {
                        if let Some((kind, amount)) = drop {
                            *player.inventory.entry(kind).or_insert(0) += amount;
                        }
                        player.blocks_gathered += 1;
                        record_voxel_delta(
                            revision,
                            last_delta,
                            x,
                            y,
                            z,
                            lk2_core::world::BlockType::Air,
                        );
                        format!("mined {:?}", block)
                    }
                    Ok(None) => {
                        ok = false;
                        "target is not mineable".to_string()
                    }
                    Err(err) => {
                        ok = false;
                        err
                    }
                }
            }
        }
        GameplayCommandKind::MountCart => {
            ok = false;
            "cart mounting is handled by the authoritative player system".to_string()
        }
        GameplayCommandKind::GatherFootBlock => {
            let [x, y, z] = [
                player.block_pos[0],
                player.block_pos[1] - 1,
                player.block_pos[2],
            ];
            let gather_kind = world.get(x, y, z).yields();
            if let Some((kind, amount)) = gather_kind
                && !player_inventory_can_receive(player, kind, amount)
            {
                ok = false;
                "inventory full for gathered resource".to_string()
            } else {
                match lk2_core::world::gather_block(world, pool, x, y, z, player_id) {
                    Ok(Some((kind, amount))) => {
                        *player.inventory.entry(kind).or_insert(0) += amount;
                        player.blocks_gathered += 1;
                        record_voxel_delta(
                            revision,
                            last_delta,
                            x,
                            y,
                            z,
                            lk2_core::world::BlockType::Air,
                        );
                        format!("gathered {:?} x{}", kind, amount)
                    }
                    Ok(None) => {
                        ok = false;
                        "nothing to gather".to_string()
                    }
                    Err(err) => {
                        ok = false;
                        err
                    }
                }
            }
        }
        GameplayCommandKind::PlaceWoodFootBlock => {
            let [x, y, z] = [
                player.block_pos[0],
                player.block_pos[1] - 1,
                player.block_pos[2],
            ];
            if !world.in_bounds(x, y, z) {
                ok = false;
                "place target out of bounds".to_string()
            } else if world.get(x, y, z) != lk2_core::world::BlockType::Air {
                ok = false;
                "place target is occupied".to_string()
            } else if player
                .inventory
                .get(&ResourceKind::Wood)
                .copied()
                .unwrap_or(0)
                < PLACE_WOOD_COST
            {
                ok = false;
                "not enough wood in player inventory".to_string()
            } else {
                *player.inventory.entry(ResourceKind::Wood).or_insert(0) -= PLACE_WOOD_COST;
                world.set(x, y, z, lk2_core::world::BlockType::Wood);
                record_voxel_delta(
                    revision,
                    last_delta,
                    x,
                    y,
                    z,
                    lk2_core::world::BlockType::Wood,
                );
                "placed wood".to_string()
            }
        }
        GameplayCommandKind::Craft(BuildRecipe::PlankPack) => {
            let wood = player
                .inventory
                .get(&ResourceKind::Wood)
                .copied()
                .unwrap_or(0);
            let hardened_wood = player
                .inventory
                .get(&ResourceKind::HardenedWood)
                .copied()
                .unwrap_or(0);
            if wood < PLANK_PACK_WOOD_COST {
                ok = false;
                format!(
                    "not enough wood for plank pack: need {}, have {}",
                    PLANK_PACK_WOOD_COST, wood
                )
            } else if hardened_wood + PLANK_PACK_OUTPUT > ResourceKind::HardenedWood.max() {
                ok = false;
                "player inventory cannot hold another plank pack".to_string()
            } else {
                *player.inventory.entry(ResourceKind::Wood).or_insert(0) -= PLANK_PACK_WOOD_COST;
                *player
                    .inventory
                    .entry(ResourceKind::HardenedWood)
                    .or_insert(0) += PLANK_PACK_OUTPUT;
                "crafted plank pack".to_string()
            }
        }
        GameplayCommandKind::Craft(BuildRecipe::Campfire) => {
            ok = false;
            "campfire placement is not implemented on the online server".to_string()
        }
        GameplayCommandKind::FoundNation => {
            if player.nation_id.is_some() {
                ok = false;
                "already in nation".to_string()
            } else {
                match nations.found(
                    pool,
                    player_id,
                    format!("PlayerNation#{}", nations.flag_count + 1),
                    player.block_pos,
                    cmd.tick,
                ) {
                    Ok(id) => {
                        player.nation_id = Some(id);
                        player.nations_founded += 1;
                        format!("founded nation {}", id.0)
                    }
                    Err(err) => {
                        ok = false;
                        format!("found nation failed: {}", err)
                    }
                }
            }
        }
        GameplayCommandKind::KillNearestCreature => {
            if let Some(kind) = nearest_creature {
                match award_creature_drop(pool, player, kind) {
                    Ok(drop) => format!("killed {:?}, awarded {:?}", kind, drop),
                    Err(err) => {
                        ok = false;
                        format!("creature drop failed: {}", err)
                    }
                }
            } else {
                ok = false;
                "no creature in range".to_string()
            }
        }
    };

    GameplayFeedback { ok, summary }
}

fn apply_gameplay_commands(
    clock: Res<SimClock>,
    mut world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut nations: ResMut<NationRegistry>,
    mut revision: ResMut<WorldRevision>,
    mut last_delta: ResMut<LastVoxelDeltaState>,
    mut feedback: MessageWriter<GameplayFeedback>,
    mut commands: Commands,
    creatures: Query<(Entity, &Creature, &CreatureAI)>,
    mut carts: Query<(&mut CartState, &mut ServerCartDrive)>,
    mut diagnostics: ResMut<ServerCommandDiagnostics>,
    mut snapshot_requests: ResMut<VoxelSnapshotResyncRequests>,
    mut player_q: Query<
        (
            &mut Transform,
            &mut PlayerPos,
            &mut PlayerStateComponent,
            &mut ServerJumpState,
            &mut SwimmingState,
            &mut RespawnState,
            &mut CartMounted,
            &lightyear::prelude::ControlledBy,
            &PlayerTag,
        ),
        With<PlayerPos>,
    >,
    mut receivers: Query<
        (
            Entity,
            &mut lightyear::prelude::MessageReceiver<GameplayCommand>,
            &mut lightyear::prelude::MessageReceiver<TerrainSnapshotRequest>,
            &mut LastClientCommandSequence,
        ),
        With<lightyear_connection::client_of::ClientOf>,
    >,
) {
    diagnostics.receiver_entities = receivers.iter().len();
    let mut consumed_creatures = HashSet::new();
    for (connection_entity, mut receiver, mut terrain_receiver, mut last_command_sequence) in
        receivers.iter_mut()
    {
        for request in terrain_receiver.receive() {
            if snapshot_requests.accept(connection_entity, clock.tick, request) {
                debug!(
                    connection = ?connection_entity,
                    "accepted terrain snapshot resync request"
                );
            } else {
                debug!(
                    connection = ?connection_entity,
                    "dropped terrain snapshot resync request over budget"
                );
            }
        }
        // Drain the receiver once. We deliberately keep ONLY the latest MoveWorld
        // per tick — clients send at their frame rate (often 144Hz) while we
        // tick at 60Hz, so a backlog of 2-3 MoveWorld commands per tick is
        // normal. Applying every queued command teleports the player; applying
        // only the latest gives us a clean per-tick movement step whose speed
        // is decoupled from the client's frame rate.
        let mut latest_move: Option<(i16, i16, i16)> = None;
        let mut jump_requested = false;
        let mut other_cmds: Vec<GameplayCommand> = Vec::new();
        for cmd in receiver.receive() {
            if !accept_client_command_sequence(&mut last_command_sequence.0, cmd.sequence) {
                debug!(
                    connection = ?connection_entity,
                    command_sequence = cmd.sequence,
                    last_accepted_sequence = last_command_sequence.0,
                    "dropping replayed gameplay command"
                );
                continue;
            }
            match cmd.kind {
                GameplayCommandKind::MoveWorld {
                    dx_milli,
                    dz_milli,
                    dy_milli,
                } => {
                    latest_move = Some((dx_milli, dz_milli, dy_milli));
                }
                GameplayCommandKind::Jump => {
                    jump_requested = true;
                }
                _ => other_cmds.push(cmd),
            }
        }

        let Some((
            mut transform,
            mut player_pos,
            mut player_state,
            mut jump,
            mut swimming,
            respawn,
            mut cart_mounted,
            _,
            player_tag,
        )) = player_q
            .iter_mut()
            .find(|(_, _, _, _, _, _, _, owner, _)| owner.owner == connection_entity)
        else {
            continue;
        };

        if respawn.ticks_remaining > 0 {
            continue;
        }

        if jump_requested && jump.grounded && !swimming.0 && !cart_mounted.0 {
            jump.velocity_y = JUMP_TAKEOFF_SPEED;
            jump.grounded = false;
        }

        let requested_move = if let Some((dx_milli, dz_milli, dy_milli)) = latest_move {
            diagnostics.move_world_received = diagnostics.move_world_received.saturating_add(1);
            diagnostics.last_dx_milli = dx_milli;
            diagnostics.last_dz_milli = dz_milli;
            let horizontal = sanitize_online_move_direction(dx_milli, dz_milli);
            let vertical = sanitize_swim_vertical(dy_milli);
            (horizontal.is_some() || vertical.abs() > 0.0001)
                .then(|| (horizontal.unwrap_or(Vec2::ZERO), vertical))
        } else {
            None
        };
        let moved = if cart_mounted.0 {
            if let Some((_, mut cart_drive)) = carts
                .iter_mut()
                .find(|(cart, _)| cart.player_id == player_tag.id())
            {
                let input = requested_move.map_or_else(CartDriveInput::default, |(dir, _)| {
                    cart_input_from_world_direction(cart_drive.state.yaw, dir)
                });
                apply_cart_move(
                    &mut transform,
                    &mut player_pos,
                    &mut player_state.0,
                    &world,
                    &mut cart_drive,
                    input,
                    jump.grounded,
                    1.0 / 60.0,
                )
            } else {
                false
            }
        } else {
            requested_move.map_or(false, |(dir, vertical)| {
                let grounded = jump.grounded;
                apply_player_move(
                    &mut transform,
                    &mut player_pos,
                    &mut player_state.0,
                    &world,
                    &mut jump,
                    &mut swimming,
                    dir,
                    vertical,
                    1.0 / 60.0,
                    grounded,
                    cart_mounted.0,
                )
            })
        };
        diagnostics.last_move_applied = moved;
        if moved {
            feedback.write(GameplayFeedback {
                ok: true,
                summary: "moved".to_string(),
            });
        }

        if !swimming.0 {
            let _ = step_authoritative_jump(
                &mut transform,
                &mut player_pos,
                &mut player_state.0,
                &world,
                &mut jump,
                1.0 / 60.0,
            );
        }

        for cmd in other_cmds {
            if matches!(cmd.kind, GameplayCommandKind::MountCart) {
                let Some((mut cart, mut cart_drive)) = carts
                    .iter_mut()
                    .find(|(cart, _)| cart.player_id == player_tag.id())
                else {
                    feedback.write(GameplayFeedback {
                        ok: false,
                        summary: "cart is unavailable".into(),
                    });
                    continue;
                };
                if !cart_mounted.0 && transform.translation.distance(cart.position) > 3.5 {
                    feedback.write(GameplayFeedback {
                        ok: false,
                        summary: "move closer to your cart".into(),
                    });
                    continue;
                }
                cart_mounted.0 = !cart_mounted.0;
                cart.occupied = cart_mounted.0;
                cart_drive.state = CartDriveState {
                    yaw: cart.yaw,
                    ..default()
                };
                feedback.write(GameplayFeedback {
                    ok: true,
                    summary: if cart_mounted.0 {
                        "mounted cart"
                    } else {
                        "dismounted cart"
                    }
                    .into(),
                });
                continue;
            }
            let nearest_creature = if matches!(cmd.kind, GameplayCommandKind::KillNearestCreature) {
                creatures
                    .iter()
                    .filter_map(|(entity, creature, _)| {
                        if consumed_creatures.contains(&entity) {
                            return None;
                        }
                        let d2 = creature_attack_distance_sq(
                            player_state.0.block_pos,
                            creature.block_pos,
                        );
                        (d2 <= CREATURE_TRAINING_ATTACK_RANGE_SQ).then_some((
                            entity,
                            d2,
                            creature.kind,
                        ))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))
            } else {
                None
            };
            let nearest_kind = nearest_creature.map(|(_, _, kind)| kind);
            let result = apply_gameplay_command(
                player_tag.id(),
                &cmd,
                &mut world,
                &mut pool,
                &mut nations,
                &mut player_state.0,
                &mut revision,
                &mut last_delta,
                nearest_kind,
            );
            if result.ok {
                if let Some((entity, _, _)) = nearest_creature {
                    consumed_creatures.insert(entity);
                    commands.entity(entity).despawn();
                }
            }
            if result.ok {
                info!("[gameplay] {}", result.summary);
            } else {
                warn!("[gameplay] {}", result.summary);
            }
            feedback.write(result);
        }

        if let Some((mut cart, mut cart_drive)) = carts
            .iter_mut()
            .find(|(cart, _)| cart.player_id == player_tag.id())
        {
            cart.occupied = cart_mounted.0;
            if cart_mounted.0 {
                cart.position = transform.translation - Vec3::Y * 0.55;
                cart.yaw = cart_drive.state.yaw;
            }
        }
    }
}

fn apply_leafwing_input(
    world: Res<GameWorld>,
    mut diagnostics: ResMut<ServerCommandDiagnostics>,
    mut carts: Query<(&mut CartState, &mut ServerCartDrive)>,
    mut player_q: Query<
        (
            &mut Transform,
            &mut PlayerPos,
            &mut PlayerStateComponent,
            &ActionState<PlayerAction>,
            &mut ServerJumpState,
            &mut SwimmingState,
            &RespawnState,
            &mut AntiStuckState,
            &CartMounted,
            &PlayerTag,
            &lightyear::prelude::ControlledBy,
        ),
        With<PlayerPos>,
    >,
) {
    for (
        mut transform,
        mut player_pos,
        mut player_state,
        actions,
        mut jump,
        mut swimming,
        respawn,
        mut anti_stuck,
        cart_mounted,
        player_tag,
        _owner,
    ) in &mut player_q
    {
        if respawn.ticks_remaining > 0 {
            continue;
        }
        if actions.just_pressed(&PlayerAction::Jump)
            && jump.grounded
            && !swimming.0
            && !cart_mounted.0
        {
            jump.velocity_y = JUMP_TAKEOFF_SPEED;
            jump.grounded = false;
        }

        let mut dir = Vec2::ZERO;
        if actions.pressed(&PlayerAction::MoveForward) {
            dir.y -= 1.0;
        }
        if actions.pressed(&PlayerAction::MoveBackward) {
            dir.y += 1.0;
        }
        if actions.pressed(&PlayerAction::MoveLeft) {
            dir.x -= 1.0;
        }
        if actions.pressed(&PlayerAction::MoveRight) {
            dir.x += 1.0;
        }
        let vertical: f32 = if actions.pressed(&PlayerAction::Jump) {
            1.0
        } else if actions.pressed(&PlayerAction::Crouch) {
            -1.0
        } else {
            0.0
        };
        let moved = if cart_mounted.0 {
            if let Some((_, mut cart_drive)) = carts
                .iter_mut()
                .find(|(cart, _)| cart.player_id == player_tag.id())
            {
                let input = CartDriveInput {
                    throttle: (-dir.y).clamp(-1.0, 1.0),
                    steering: cart_steering_from_right_input(dir.x),
                };
                let moved = apply_cart_move(
                    &mut transform,
                    &mut player_pos,
                    &mut player_state.0,
                    &world,
                    &mut cart_drive,
                    input,
                    jump.grounded,
                    1.0 / 60.0,
                );
                let input_dir = Vec2::new(input.steering, -input.throttle);
                if input_dir.length_squared() > 0.0001 {
                    diagnostics.move_world_received =
                        diagnostics.move_world_received.saturating_add(1);
                    diagnostics.last_dx_milli = (input_dir.x * 1000.0).round() as i16;
                    diagnostics.last_dz_milli = (input_dir.y * 1000.0).round() as i16;
                }
                moved
            } else {
                false
            }
        } else if dir.length_squared() > 0.0001 || vertical.abs() > 0.0001 {
            if actions.pressed(&PlayerAction::Sprint) {
                dir *= 1.5;
            }
            dir = dir.clamp_length_max(ONLINE_INPUT_MAX_MAGNITUDE);
            let dx_milli = (dir.x * 1000.0).round() as i16;
            let dz_milli = (dir.y * 1000.0).round() as i16;
            diagnostics.move_world_received = diagnostics.move_world_received.saturating_add(1);
            diagnostics.last_dx_milli = dx_milli;
            diagnostics.last_dz_milli = dz_milli;
            let grounded = jump.grounded;
            apply_player_move(
                &mut transform,
                &mut player_pos,
                &mut player_state.0,
                &world,
                &mut jump,
                &mut swimming,
                dir,
                vertical,
                1.0 / 60.0,
                grounded,
                cart_mounted.0,
            )
        } else {
            false
        };
        diagnostics.last_move_applied = moved;
        if cart_mounted.0 || dir.length_squared() > 0.0001 || vertical.abs() > 0.0001 {
            maintain_anti_stuck(
                &world,
                &mut player_state.0,
                &mut transform,
                &mut player_pos,
                &mut anti_stuck,
                moved,
            );
        }

        if !swimming.0 {
            let _ = step_authoritative_jump(
                &mut transform,
                &mut player_pos,
                &mut player_state.0,
                &world,
                &mut jump,
                1.0 / 60.0,
            );
        }

        if cart_mounted.0 {
            if let Some((mut cart, cart_drive)) = carts
                .iter_mut()
                .find(|(cart, _)| cart.player_id == player_tag.id())
            {
                cart.position = transform.translation - Vec3::Y * 0.55;
                cart.yaw = cart_drive.state.yaw;
                cart.occupied = true;
            }
        }
    }
}

fn recover_dead_players(
    world: Res<GameWorld>,
    tick: Res<FixedTick>,
    mut carts: Query<(&mut CartState, &mut ServerCartDrive)>,
    mut players: Query<(
        &PlayerTag,
        &mut Health,
        &mut PvpCombatant,
        &mut Transform,
        &mut PlayerPos,
        &mut PlayerStateComponent,
        &mut ServerJumpState,
        &mut SwimmingState,
        &mut RespawnState,
        &mut AntiStuckState,
        &mut CartMounted,
    )>,
) {
    for (
        player_tag,
        mut health,
        mut combatant,
        mut transform,
        mut player_pos,
        mut player_state,
        mut jump,
        mut swimming,
        mut respawn,
        mut anti_stuck,
        mut cart_mounted,
    ) in &mut players
    {
        if !health.is_dead() && respawn.ticks_remaining == 0 {
            continue;
        }

        if respawn.ticks_remaining == 0 {
            respawn.ticks_remaining = RESPAWN_DELAY_TICKS;
            combatant.cooldown_remaining = 0.0;
            continue;
        }

        respawn.ticks_remaining -= 1;
        if respawn.ticks_remaining > 0 {
            continue;
        }

        let center = [world.size / 2, 0, world.size / 2];
        let (position, block_pos) = connected_player_spawn(
            &world,
            center,
            Vec3::new(center[0] as f32 + 0.5, 1.0, center[2] as f32 + 0.5),
            player_tag.id(),
        );
        transform.translation = position;
        player_pos.0 = position;
        player_state.0.pos = position;
        player_state.0.block_pos = block_pos;
        *jump = ServerJumpState::default();
        swimming.0 = false;
        cart_mounted.0 = false;
        if let Some((mut cart, mut cart_drive)) = carts
            .iter_mut()
            .find(|(cart, _)| cart.player_id == player_tag.id())
        {
            cart.position = position - Vec3::Y * 0.55;
            cart.yaw = 0.0;
            cart.occupied = false;
            cart_drive.state = CartDriveState::default();
        }
        anti_stuck.last_safe_pos = position;
        anti_stuck.last_safe_block = block_pos;
        anti_stuck.failed_move_ticks = 0;
        health.current = health.max.max(1.0);
        health.invuln_until_tick = tick.0.saturating_add(RESPAWN_DELAY_TICKS);
        combatant.cooldown_remaining = 0.0;
        info!(
            "[pvp] respawned player {} at {:?}",
            player_tag.id(),
            block_pos
        );
    }
}

fn echo_ping_messages(
    clock: Res<SimClock>,
    mut receivers: Query<
        (
            &mut lightyear::prelude::MessageReceiver<PingMessage>,
            &mut lightyear::prelude::MessageSender<PongMessage>,
        ),
        With<lightyear_connection::client_of::ClientOf>,
    >,
) {
    for (mut receiver, mut sender) in receivers.iter_mut() {
        for ping in receiver.receive() {
            if !ping.is_finite() {
                warn!(
                    "[net] ignoring non-finite PingMessage from client {}",
                    ping.client_id
                );
                continue;
            }
            let pong = PongMessage {
                client_id: ping.client_id,
                sequence: ping.sequence,
                client_time_secs: ping.client_time_secs,
                server_tick: clock.tick as u32,
            };
            sender.send::<StateChannel>(pong);
        }
    }
}

fn broadcast_gameplay_feedback(
    mut feedback: MessageReader<GameplayFeedback>,
    server_q: Query<&lightyear::prelude::Server, With<lightyear_connection::server::Started>>,
    mut sender: lightyear::prelude::ServerMultiMessageSender<()>,
) {
    let Ok(server) = server_q.single() else {
        return;
    };

    for msg in feedback.read() {
        if msg.summary == "moved" {
            continue;
        }
        let _ =
            sender.send::<_, ControlChannel>(msg, server, &lightyear::prelude::NetworkTarget::All);
    }
}

fn relay_chat_messages(
    clock: Res<SimClock>,
    mut receivers: Query<(
        Option<&lightyear::prelude::RemoteId>,
        &mut lightyear::prelude::MessageReceiver<ChatMessage>,
    )>,
    server_q: Query<&lightyear::prelude::Server, With<lightyear_connection::server::Started>>,
    mut sender: lightyear::prelude::ServerMultiMessageSender<()>,
) {
    let Ok(server) = server_q.single() else {
        return;
    };
    for (remote_id, mut receiver) in receivers.iter_mut() {
        let sender_name =
            remote_id.map_or_else(|| "Player".to_string(), |id| format!("{:?}", id.0));
        for message in receiver.receive() {
            let text = sanitize_chat_text(&message.text);
            if text.is_empty() {
                continue;
            }
            info!("[chat] {}: {}", sender_name, text);
            let broadcast = ChatBroadcast {
                server_tick: clock.tick,
                sender: sender_name.clone(),
                text,
            };
            let _ = sender.send::<_, ControlChannel>(
                &broadcast,
                server,
                &lightyear::prelude::NetworkTarget::All,
            );
        }
    }
}

fn sanitize_chat_text(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(120)
        .collect()
}

fn maintain_anti_stuck(
    world: &GameWorld,
    player: &mut PlayerState,
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    anti_stuck: &mut AntiStuckState,
    moved: bool,
) {
    if player_position_is_safe(world, player.pos) {
        anti_stuck.last_safe_pos = player.pos;
        anti_stuck.last_safe_block = player.block_pos;
        anti_stuck.failed_move_ticks = 0;
        return;
    }

    anti_stuck.failed_move_ticks = if moved {
        0
    } else {
        anti_stuck.failed_move_ticks.saturating_add(1)
    };

    if anti_stuck.failed_move_ticks < ANTI_STUCK_FAILED_MOVE_LIMIT {
        return;
    }

    let resolved =
        resolve_player_stuck_near(world, player.pos, ANTI_STUCK_SEARCH_RADIUS).or_else(|| {
            resolve_player_stuck_near(world, anti_stuck.last_safe_pos, ANTI_STUCK_SEARCH_RADIUS)
        });
    let Some((pos, block_pos)) = resolved else {
        return;
    };

    player.pos = pos;
    player.block_pos = block_pos;
    transform.translation = pos;
    player_pos.0 = pos;
    anti_stuck.last_safe_pos = pos;
    anti_stuck.last_safe_block = block_pos;
    anti_stuck.failed_move_ticks = 0;
    warn!("[anti-stuck] resolved player to {:?}", block_pos);
}

fn apply_cart_move(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    world: &GameWorld,
    drive: &mut ServerCartDrive,
    input: CartDriveInput,
    grounded: bool,
    dt: f32,
) -> bool {
    let displacement = step_cart_drive(&mut drive.state, input, dt);
    if displacement.length_squared() <= 0.0001 {
        transform.rotation = Quat::from_rotation_y(drive.state.yaw);
        return false;
    }

    let moved = if grounded {
        apply_world_displacement(
            transform,
            player_pos,
            player,
            world,
            Vec3::new(displacement.x, 0.0, displacement.y),
            true,
        )
    } else {
        apply_air_world_displacement(
            transform,
            player_pos,
            player,
            world,
            Vec3::new(displacement.x, 0.0, displacement.y),
            true,
        )
    };
    transform.rotation = Quat::from_rotation_y(drive.state.yaw);
    if !moved {
        // Grid collision is the vehicle's equivalent of STK's wheel/ground
        // impulse limit: hitting a solid cell should not keep the car's full
        // momentum alive for several ticks.
        drive.state.speed = 0.0;
    }
    moved
}

fn cart_input_from_world_direction(yaw: f32, direction: Vec2) -> CartDriveInput {
    let forward = Vec2::new(yaw.sin(), yaw.cos());
    let reverse = direction.dot(forward) < -0.35;
    let desired_yaw =
        direction.x.atan2(direction.y) + if reverse { std::f32::consts::PI } else { 0.0 };
    let heading_error = normalize_angle(desired_yaw - yaw);
    CartDriveInput {
        throttle: (if reverse { -1.0 } else { 1.0 }) * direction.length().clamp(0.0, 1.0),
        steering: (heading_error / 0.9).clamp(-1.0, 1.0),
    }
}

fn normalize_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn apply_world_move(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    world: &GameWorld,
    dir: Vec2,
    dt: f32,
    cart_mounted: bool,
) -> bool {
    if dir.length_squared() <= 0.0001 {
        return false;
    }
    // Magnitude is meaningful: the client sends 1.0 for walk and 1.5 for sprint,
    // so we do NOT re-normalize here. Per-tick movement = dir * speed * dt, and
    // we drop exactly one MoveWorld per tick on the caller side, so speed stays
    // at ONLINE_MOVE_SPEED (× sprint factor) regardless of client frame rate.
    let step = Vec3::new(dir.x, 0.0, dir.y) * ONLINE_MOVE_SPEED * dt;
    apply_world_displacement(transform, player_pos, player, world, step, cart_mounted)
}

fn apply_world_displacement(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    world: &GameWorld,
    step: Vec3,
    cart_mounted: bool,
) -> bool {
    if step.length_squared() <= 0.0001 {
        return false;
    }
    let next = player.pos + step;
    let next_x = next.x.floor() as i32;
    let next_z = next.z.floor() as i32;
    let Some((stand_pos, block_pos)) =
        player_stand_position_at(world, next_x, next_z, player.pos.y, ONLINE_STEP_THRESHOLD)
    else {
        return false;
    };
    if stand_pos.y - player.pos.y > ONLINE_STEP_THRESHOLD {
        return false;
    }

    let next_pos = Vec3::new(next.x, stand_pos.y, next.z);
    if !vehicle_volume_clear_at(world, next_pos, cart_mounted) {
        return false;
    }

    transform.translation = next_pos;
    player_pos.0 = next_pos;
    player.pos = next_pos;
    player.block_pos = block_pos;
    true
}

fn apply_player_move(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    world: &GameWorld,
    jump: &mut ServerJumpState,
    swimming: &mut SwimmingState,
    dir: Vec2,
    vertical: f32,
    dt: f32,
    grounded: bool,
    cart_mounted: bool,
) -> bool {
    let horizontal_step = Vec3::new(dir.x, 0.0, dir.y) * ONLINE_MOVE_SPEED * dt;
    let swim_probe = player.pos + horizontal_step;
    let was_swimming = swimming.0;
    if swimming.0 || can_swim_at(world, player.pos) || can_swim_at(world, swim_probe) {
        let next = player.pos + horizontal_step + Vec3::Y * vertical * SWIM_VERTICAL_SPEED * dt;
        if can_swim_at(world, next) {
            let x = next.x.floor() as i32;
            let z = next.z.floor() as i32;
            let Some(floor_y) = water_floor_foot_y(world, x, z) else {
                swimming.0 = false;
                return false;
            };
            let min_y = floor_y as f32 + 0.05;
            let max_y = constant::SEA_LEVEL as f32 + 1.0 - SWIM_SURFACE_CLEARANCE;
            if min_y <= max_y {
                let next_pos = Vec3::new(next.x, next.y.clamp(min_y, max_y), next.z);
                if player_volume_clear_at(world, next_pos) {
                    transform.translation = next_pos;
                    player_pos.0 = next_pos;
                    player.pos = next_pos;
                    player.block_pos = [x, next_pos.y.floor() as i32, z];
                    swimming.0 = true;
                    jump.grounded = false;
                    jump.velocity_y = 0.0;
                    return true;
                }
            }
        }
        // Moving out of the water falls back to the ordinary step solver.
        swimming.0 = false;
        if was_swimming {
            return apply_world_move(transform, player_pos, player, world, dir, dt, cart_mounted);
        }
    }

    if grounded {
        apply_world_move(transform, player_pos, player, world, dir, dt, cart_mounted)
    } else {
        apply_air_world_move(transform, player_pos, player, world, dir, dt, cart_mounted)
    }
}

fn apply_air_world_move(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    world: &GameWorld,
    dir: Vec2,
    dt: f32,
    cart_mounted: bool,
) -> bool {
    if dir.length_squared() <= 0.0001 {
        return false;
    }
    let step = Vec3::new(dir.x, 0.0, dir.y) * ONLINE_MOVE_SPEED * dt;
    apply_air_world_displacement(transform, player_pos, player, world, step, cart_mounted)
}

fn apply_air_world_displacement(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    world: &GameWorld,
    step: Vec3,
    cart_mounted: bool,
) -> bool {
    if step.length_squared() <= 0.0001 {
        return false;
    }
    let next_pos = player.pos + step;
    if !vehicle_volume_clear_at(world, next_pos, cart_mounted) {
        return false;
    }

    transform.translation = next_pos;
    player_pos.0 = next_pos;
    player.pos = next_pos;
    player.block_pos = [
        next_pos.x.floor() as i32,
        player.block_pos[1],
        next_pos.z.floor() as i32,
    ];
    true
}

fn vehicle_volume_clear_at(world: &GameWorld, position: Vec3, cart_mounted: bool) -> bool {
    if !player_volume_clear_at(world, position) {
        return false;
    }
    if !cart_mounted {
        return true;
    }

    const CART_HALF_WIDTH: f32 = 0.82;
    const CART_HALF_LENGTH: f32 = 0.62;
    [
        Vec3::new(CART_HALF_WIDTH, 0.0, 0.0),
        Vec3::new(-CART_HALF_WIDTH, 0.0, 0.0),
        Vec3::new(0.0, 0.0, CART_HALF_LENGTH),
        Vec3::new(0.0, 0.0, -CART_HALF_LENGTH),
    ]
    .into_iter()
    .all(|offset| player_volume_clear_at(world, position + offset))
}

fn step_authoritative_jump(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    world: &GameWorld,
    jump: &mut ServerJumpState,
    dt: f32,
) -> (bool, &'static str) {
    let dt = dt.clamp(0.0, 0.05);
    let Some((stand_pos, stand_block)) = player_stand_position_at(
        world,
        player.pos.x.floor() as i32,
        player.pos.z.floor() as i32,
        player.pos.y,
        ONLINE_STEP_THRESHOLD,
    ) else {
        jump.grounded = false;
        jump.velocity_y = (jump.velocity_y - JUMP_GRAVITY * dt).max(JUMP_TERMINAL_SPEED);
        return (false, "jump_no_floor");
    };

    let ground_y = stand_pos.y;
    if player.pos.y <= ground_y + 0.02 && jump.velocity_y <= 0.0 {
        if (player.pos.y - ground_y).abs() > 0.001 || player.block_pos != stand_block {
            let pos = Vec3::new(player.pos.x, ground_y, player.pos.z);
            transform.translation = pos;
            player_pos.0 = pos;
            player.pos = pos;
            player.block_pos = stand_block;
        }
        jump.velocity_y = 0.0;
        jump.grounded = true;
        return (false, "grounded");
    }

    jump.grounded = false;
    jump.velocity_y = (jump.velocity_y - JUMP_GRAVITY * dt).max(JUMP_TERMINAL_SPEED);
    let next_y = player.pos.y + jump.velocity_y * dt;

    if jump.velocity_y > 0.0 {
        let next_pos = Vec3::new(player.pos.x, next_y, player.pos.z);
        if player_volume_clear_at(world, next_pos) {
            transform.translation = next_pos;
            player_pos.0 = next_pos;
            player.pos = next_pos;
            player.block_pos = stand_block;
            return (true, "jump_rise");
        }
        jump.velocity_y = 0.0;
        return (false, "jump_head_blocked");
    }

    let next_pos = if next_y <= ground_y {
        jump.velocity_y = 0.0;
        jump.grounded = true;
        Vec3::new(player.pos.x, ground_y, player.pos.z)
    } else {
        Vec3::new(player.pos.x, next_y, player.pos.z)
    };
    transform.translation = next_pos;
    player_pos.0 = next_pos;
    player.pos = next_pos;
    player.block_pos = stand_block;
    (
        true,
        if jump.grounded {
            "jump_landed"
        } else {
            "jump_fall"
        },
    )
}

fn player_volume_clear_at(world: &GameWorld, pos: Vec3) -> bool {
    let foot_y = pos.y.floor() as i32;
    if !player_body_clear(world, pos.x.floor() as i32, foot_y, pos.z.floor() as i32) {
        return false;
    }
    for y in foot_y..=(foot_y + 1) {
        for ox in [-PLAYER_COLLISION_RADIUS, PLAYER_COLLISION_RADIUS] {
            for oz in [-PLAYER_COLLISION_RADIUS, PLAYER_COLLISION_RADIUS] {
                let x = (pos.x + ox).floor() as i32;
                let z = (pos.z + oz).floor() as i32;
                if (!world.procedural && (x < 0 || x >= world.size || z < 0 || z >= world.size))
                    || y < 0
                    || y >= world.size
                {
                    return false;
                }
                if world.get(x, y, z).is_solid() {
                    return false;
                }
            }
        }
    }
    true
}

fn sync_authoritative_snapshot_components(
    clock: Res<SimClock>,
    world: Res<GameWorld>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    regions: Res<NatureRegionWorld>,
    reports: Res<LatestNatureRegionReports>,
    obs: Res<TickObserver>,
    revision: Res<WorldRevision>,
    last_delta: Res<LastVoxelDeltaState>,
    mut snapshot_cache: ResMut<VoxelChunkSnapshotCache>,
    mut snapshot_requests: ResMut<VoxelSnapshotResyncRequests>,
    mut q: Query<
        (
            &mut GameplayHudState,
            &mut EcoSnapshot,
            &mut VoxelDelta,
            &mut VoxelChunkSnapshot,
            &PlayerStateComponent,
            Option<&lightyear::prelude::ControlledBy>,
        ),
        With<PlayerPos>,
    >,
) {
    let Some(primary) = regions.primary() else {
        return;
    };
    let primary_snapshot = report_snapshot_for_region(&reports, primary.id, primary.id);
    let Some(primary_snapshot) = primary_snapshot else {
        return;
    };
    if !primary_snapshot.is_finite() {
        warn!(
            "[nature] retaining previous replicated snapshot after non-finite state at tick {}",
            clock.tick
        );
        return;
    }
    for (mut hud_state, mut eco_state, mut delta, mut chunk, player, controlled_by) in q.iter_mut()
    {
        let region_id = NatureRegionId::from_position(
            [player.0.block_pos[0] as f32, player.0.block_pos[2] as f32],
            lk2_core::simulation::regions::DEFAULT_REGION_SIZE,
        );
        let nature_snapshot =
            report_snapshot_for_region(&reports, primary.id, region_id).unwrap_or(primary_snapshot);
        let eco_snapshot = nature_snapshot
            .for_region(
                [player.0.block_pos[0] as f32, player.0.block_pos[2] as f32],
                (constant::PLAYER_VISION_RADIUS * 2) as f32,
            )
            .detailed_ecology;
        let hud = build_gameplay_hud_state(&clock, &player.0, &pool, &nations, &monsters, &obs);
        let chunk_snapshot = cached_voxel_chunk_snapshot(
            &world,
            revision.0,
            player.0.block_pos,
            &mut snapshot_cache,
        );
        let requested = controlled_by
            .and_then(|controlled| snapshot_requests.pending.remove(&controlled.owner));
        let requested_current_chunk = requested.as_ref().is_some_and(|request| {
            request.chunk_x == chunk_snapshot.chunk_x && request.chunk_z == chunk_snapshot.chunk_z
        });
        *hud_state = hud.clone();
        *eco_state = eco_snapshot.clone();
        if requested_current_chunk || *chunk != *chunk_snapshot {
            *chunk = chunk_snapshot.clone();
        }
        if last_delta.revision > 0 {
            *delta = VoxelDelta {
                revision: last_delta.revision,
                x: last_delta.x,
                y: last_delta.y,
                z: last_delta.z,
                block: block_type_to_u8(last_delta.block),
            };
        }
    }
}

#[derive(Resource)]
pub struct TimeOfDay(pub f32);

impl Default for TimeOfDay {
    fn default() -> Self {
        Self(0.5)
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let port: u16 = std::env::var("LK2_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    if !server_ports_available(port) {
        error!(
            "[server] UDP port {} is already in use. Stop the existing lk2-server process or set LK2_PORT to a free port.",
            port
        );
        return;
    }
    info!("[server] listening on UDP 0.0.0.0:{}", port);

    let args: Vec<String> = std::env::args().collect();
    let auto_demo_mode = args.iter().any(|a| a == "--auto-demo");
    let scenario_file_requested = args
        .iter()
        .skip(1)
        .any(|arg| !arg.starts_with("--") && arg.ends_with(".json"));
    let scenario = if auto_demo_mode {
        Scenario {
            name: "idle".into(),
            record_window: None,
            world: Default::default(),
            player: Default::default(),
            steps: vec![
                lk2_core::scenario::ScenarioStep::Log {
                    msg: "=== idle: server stand-by ===".into(),
                },
                lk2_core::scenario::ScenarioStep::WaitTicks { ticks: 1000 },
            ],
        }
    } else {
        lk2_core::scenario::load_scenario_from_args_or_default(&args)
    };
    let scenario_state = if auto_demo_mode || scenario_file_requested {
        ScenarioState::from_scenario(scenario.clone())
    } else {
        ScenarioState::default()
    };

    let _ = std::fs::create_dir_all("screenshots");
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(lightyear::prelude::server::ServerPlugins::default())
        .add_plugins(LightyearAvianPlugin::default())
        .add_plugins(lk2_core::protocol::ProtocolPlugin)
        .add_plugins(lk2_core::match_state::MatchStatePlugin)
        .add_plugins(lk2_core::objectives::ObjectivesPlugin)
        .add_plugins(lk2_core::protection::ProtectionPlugin)
        .add_plugins(lk2_core::combat::CombatPlugin)
        .add_plugins(SimplePvpAuthorityPlugin)
        .add_plugins(NatureServerProjectionPlugin)
        .add_message::<ChatBroadcast>()
        .add_message::<ChatMessage>()
        .add_message::<GameplayCommand>()
        .add_message::<TerrainSnapshotRequest>()
        .add_message::<GameplayFeedback>()
        .add_message::<PingMessage>()
        .add_message::<PongMessage>()
        .init_resource::<PeerMetadata>()
        .init_resource::<SimClock>()
        .init_resource::<TimeOfDay>()
        .init_resource::<GameWorld>()
        .init_resource::<GlobalResourcePool>()
        .init_resource::<NationRegistry>()
        .init_resource::<MonsterEcosystem>()
        .init_resource::<NatureRegionWorld>()
        .init_resource::<LatestNatureRegionReports>()
        .init_resource::<NatureRestoreStatus>()
        .init_resource::<TickObserver>()
        .init_resource::<TickRecorder>()
        .init_resource::<CreatureSpawnerDone>()
        .init_resource::<PlayerState>()
        .init_resource::<NextPlayerId>()
        .init_resource::<ReservedPlayerEntities>()
        .init_resource::<DisconnectedPlayerSessions>()
        .init_resource::<ServerTickCounter>()
        .init_resource::<WorldRevision>()
        .init_resource::<LastVoxelDeltaState>()
        .init_resource::<VoxelChunkSnapshotCache>()
        .init_resource::<VoxelSnapshotResyncRequests>()
        .init_resource::<ServerCommandDiagnostics>()
        .init_resource::<ServerConnectionDiagnostics>()
        .insert_resource(scenario_state)
        .add_systems(
            Startup,
            (
                setup_world.after(persistence::load_nature_region_saves),
                spawn_server_creatures,
                dump_world_resources,
                spawn_server,
                spawn_player,
                lk2_core::objectives::setup_default_objectives,
                // self_check was here previously; it ran 100 sim ticks inside
                // Startup, blocking the App::run loop and starving UDP packet
                // processing for ~9s — which exactly matched the default
                // client_timeout_secs and caused connection_request timeouts.
                // Moved to first Update tick instead (see below).
            )
                .chain(),
        )
        .add_observer(replicate_player_for_connected)
        .add_observer(record_disconnected_client)
        .configure_sets(
            FixedUpdate,
            (SimSet::Interaction, SimSet::ScoreAndAudit, SimSet::Snapshot).chain(),
        )
        .add_systems(
            Update,
            // Run startup self-check on first Update so it doesn't starve
            // the UDP receiver. TickObserver invariants are still checked on
            // every sim tick via end_tick_system, so this is just for the
            // early "100 ticks all green" diagnostic.
            run_startup_self_check_once,
        )
        .add_systems(
            Update,
            (
                lk2_core::ecology::animals::update_creatures,
                sync_server_creature_states.after(lk2_core::ecology::animals::update_creatures),
            ),
        )
        .add_systems(
            FixedUpdate,
            (
                sync_nature_regions.in_set(SimSet::Interaction),
                simulation_tick
                    .after(sync_nature_regions)
                    .in_set(SimSet::Interaction)
                    .in_set(NatureAuthoritySet::StepWorld),
                lk2_core::scenario::scenario_runner
                    .after(simulation_tick)
                    .in_set(SimSet::Interaction),
                // iter_199 Phase A: nation 自动 upkeep — 每个 nation 周期消耗
                // Wood/Food 维持 flag，缺资源时扣 flag_hp 并可能 dissolve。
                // 放在 simulation_tick 之后才能看到 sim tick 加的 Apple/Food。
                lk2_core::nation::tick_nations_upkeep_system
                    .after(simulation_tick)
                    .in_set(SimSet::Interaction),
                end_tick_system.in_set(SimSet::ScoreAndAudit),
                tick_recorder
                    .after(NatureAuthoritySet::PublishReport)
                    .after(sync_authoritative_snapshot_components)
                    .in_set(SimSet::Snapshot),
                lk2_core::scenario::scenario_tick_recorder
                    .after(lk2_core::scenario::scenario_runner)
                    .in_set(SimSet::Snapshot),
            ),
        )
        .add_systems(
            FixedUpdate,
            (
                stream_terrain_chunks,
                apply_gameplay_commands.before(sync_nature_regions),
                recover_dead_players.before(sync_nature_regions),
                apply_leafwing_input.before(sync_nature_regions),
                sync_authoritative_avian_transforms,
                echo_ping_messages,
                broadcast_gameplay_feedback,
                relay_chat_messages,
                sync_authoritative_snapshot_components
                    .after(simulation_tick)
                    .after(NatureAuthoritySet::PublishReport),
                broadcast_player_pos,
            )
                .chain(),
        )
        .run();
}

fn server_ports_available(port: u16) -> bool {
    std::net::UdpSocket::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port))).is_ok()
}

/// Keep Avian's networked state aligned with the existing authoritative
/// movement system. The server remains the sole writer; Lightyear Avian then
/// projects these components to online clients.
fn sync_authoritative_avian_transforms(
    mut players: Query<(&Transform, &mut Position, &mut Rotation), With<PlayerPos>>,
) {
    for (transform, mut position, mut rotation) in &mut players {
        if transform.translation.is_finite() {
            position.0 = transform.translation;
            rotation.0 = transform.rotation;
        }
    }
}

fn dump_world_resources(world: &World) {
    let has_peer_metadata = world.get_resource::<PeerMetadata>().is_some();
    info!("[debug] PeerMetadata in world? {}", has_peer_metadata);
}

fn spawn_server(mut commands: Commands) {
    use lightyear::prelude::server::Start;
    use lightyear_netcode::server_plugin::{NetcodeConfig, NetcodeServer};

    let server_addr = lk2_core::transport::server_listen_addr();
    info!(
        "[net] spawning server entity with ServerUdpIo + NetcodeServer + LocalAddr({})",
        server_addr
    );

    let netcode_server = NetcodeServer::new(
        NetcodeConfig::default()
            .with_protocol_id(PROTOCOL_ID)
            .with_key(PRIVATE_KEY)
            .with_client_timeout_secs(NETCODE_CLIENT_TIMEOUT_SECS),
    );
    info!(
        "[net] NetcodeServer initialized: protocol_id=0x{:x}, key=<fixed-dev>",
        PROTOCOL_ID
    );

    let server_id = commands
        .spawn((
            Name::new("Server"),
            lightyear::prelude::Server::default(),
            ServerUdpIo::default(),
            LocalAddr(server_addr),
            netcode_server,
        ))
        .id();

    info!("[net] triggering Start on server entity {:?}", server_id);
    commands.trigger(Start { entity: server_id });
}

fn spawn_server_player(
    commands: &mut Commands,
    name: String,
    player_id: u32,
    state: PlayerState,
    mounted: bool,
    saved_cart: Option<CartState>,
    jump: ServerJumpState,
    anti_stuck: AntiStuckState,
) -> Entity {
    let position = state.pos;
    let mut entity = commands.spawn_empty();
    entity
        .insert(Name::new(name.clone()))
        .insert(PlayerTag(player_id))
        .insert(Transform::from_translation(position))
        .insert(Position(position))
        .insert(Rotation::default())
        // The world-grid movement rules remain the authority for terrain
        // clearance. This kinematic body gives Lightyear Avian a concrete
        // networked player body without letting the local solver own motion.
        .insert(RigidBody::Kinematic)
        .insert(Collider::capsule(0.38, 1.0))
        .insert(PlayerPos(position))
        .insert(CartMounted(mounted))
        .insert(SwimmingState::default())
        .insert(PlayerStateComponent(state))
        .insert(jump)
        .insert(RespawnState::default())
        .insert(anti_stuck)
        .insert(PvpCombatant::default())
        .insert(SimpleWeapon::default())
        .insert(Hitbox::default())
        .insert(Health::default())
        .insert(ActionState::<PlayerAction>::default())
        .insert(empty_gameplay_hud_state())
        .insert(EcoCycle::default().to_snapshot(0))
        .insert(empty_voxel_delta())
        .insert(empty_voxel_chunk_snapshot());
    let entity_id = entity.id();
    drop(entity);
    let mut cart = saved_cart.unwrap_or(CartState {
        player_id,
        position: position - Vec3::Y * 0.55,
        yaw: 0.0,
        occupied: mounted,
    });
    // Player ids are server-local and can change after a reconnect. The
    // session restores cart pose/state, but ownership must follow the new id.
    cart.player_id = player_id;
    cart.occupied = mounted;
    commands.spawn((
        Name::new(format!("{name}-Cart")),
        cart,
        ServerCartDrive {
            state: CartDriveState {
                yaw: cart.yaw,
                ..default()
            },
        },
        Transform::from_translation(cart.position).with_rotation(Quat::from_rotation_y(cart.yaw)),
        lightyear::prelude::Replicate::to_clients(lightyear::prelude::NetworkTarget::All),
    ));
    entity_id
}

fn spawn_server_creatures(
    mut commands: Commands,
    world: Res<GameWorld>,
    mut done: ResMut<CreatureSpawnerDone>,
) {
    if done.0 {
        return;
    }
    done.0 = true;

    let center_x = world.size / 2;
    let center_z = world.size / 2;
    let placements = [
        (3, -3, CreatureKind::Cow),
        (3, 3, CreatureKind::Pig),
        (-3, 3, CreatureKind::Sheep),
        (-3, -3, CreatureKind::Chicken),
        (5, 0, CreatureKind::Cow),
        (0, 5, CreatureKind::Pig),
        (-5, 0, CreatureKind::Sheep),
        (0, -5, CreatureKind::Chicken),
    ];

    let mut spawned = 0;
    for (dx, dz, kind) in placements {
        let x = center_x + dx;
        let z = center_z + dz;
        let Some((position, block_pos)) = player_spawn_position_at(&world, x, z) else {
            continue;
        };
        commands.spawn((
            Creature { kind, block_pos },
            server_creature_ai(),
            Transform::from_translation(position),
            CreatureState {
                kind: kind.to_u8(),
                position,
            },
            lightyear::prelude::Replicate::to_clients(lightyear::prelude::NetworkTarget::All),
        ));
        spawned += 1;
    }
    info!(
        "[creature] spawned {} authoritative online creatures",
        spawned
    );
}

fn server_creature_ai() -> CreatureAI {
    CreatureAI {
        wander_timer: 0.0,
        next_wander_secs: CREATURE_INITIAL_WANDER_SECS,
        bob_phase: 0.0,
    }
}

fn sync_server_creature_states(creatures: Query<(&Creature, &Transform, &mut CreatureState)>) {
    for (creature, transform, mut state) in creatures {
        if !transform.translation.is_finite() {
            continue;
        }
        let kind = creature.kind.to_u8();
        if state.kind != kind || state.position != transform.translation {
            state.kind = kind;
            state.position = transform.translation;
        }
    }
}

fn spawn_player(
    mut commands: Commands,
    mut player: ResMut<PlayerState>,
    mut next_player_id: ResMut<NextPlayerId>,
    game_world: Res<GameWorld>,
    scenario_state: Res<ScenarioState>,
) {
    let scenario_spawn = scenario_state
        .scenario
        .as_ref()
        .and_then(|scenario| scenario.player.spawn);
    let sx = scenario_spawn.map_or(game_world.size / 2, |spawn| spawn[0]);
    let sz = scenario_spawn.map_or(game_world.size / 2, |spawn| spawn[2]);
    let (spawn, spawn_block) = player_spawn_position_near(&game_world, sx, sz, 14, 2)
        .unwrap_or_else(|| {
            // Primary search failed (e.g. procedural terrain has no
            // standable block in radius). Walk the golab-style deterministic
            // ring of [dx, dz] offsets and snap to the first one that stands
            // safely. Only if the entire ring is unstandable do we drop the
            // player onto a hard-coded column above sea level.
            for offset in fallback_spawn_ring_offsets(0) {
                if let Some((pos, block)) =
                    player_spawn_position_at(&game_world, sx + offset[0], sz + offset[1])
                {
                    return (pos, block);
                }
            }
            let fallback = Vec3::new(
                sx as f32 + 0.5,
                (constant::SEA_LEVEL + 2) as f32 + 0.5,
                sz as f32 + 0.5,
            );
            (
                fallback,
                [
                    fallback.x.floor() as i32,
                    fallback.y.floor() as i32,
                    fallback.z.floor() as i32,
                ],
            )
        });
    let (spawn, spawn_block) = scenario_spawn.map_or((spawn, spawn_block), |block| {
        (
            Vec3::new(
                block[0] as f32 + 0.5,
                block[1] as f32,
                block[2] as f32 + 0.5,
            ),
            block,
        )
    });
    player.pos = spawn;
    player.block_pos = spawn_block;
    next_player_id.0 = 1;
    info!(
        "[player] spawning authoritative player entity at {:?}, block={:?}",
        spawn, spawn_block
    );
    spawn_server_player(
        &mut commands,
        "Player".to_string(),
        0,
        PlayerState {
            pos: spawn,
            block_pos: spawn_block,
            ..default()
        },
        false,
        None,
        ServerJumpState::default(),
        AntiStuckState {
            last_safe_pos: spawn,
            last_safe_block: spawn_block,
            failed_move_ticks: 0,
        },
    );
}

fn connected_player_spawn(
    world: &GameWorld,
    base_block: [i32; 3],
    base_pos: Vec3,
    player_id: u32,
) -> (Vec3, [i32; 3]) {
    for [dx, dz] in fallback_spawn_ring_offsets((player_id % 8) as u8) {
        let x = base_block[0] + dx;
        let z = base_block[2] + dz;
        let Some((position, block_pos)) = player_spawn_position_at(world, x, z) else {
            continue;
        };
        if player_volume_clear_at(world, position) {
            return (position, block_pos);
        }
    }

    player_spawn_position_near(world, base_block[0], base_block[2], 14, 2)
        .unwrap_or((base_pos, base_block))
}

fn replicate_player_for_connected(
    trigger: On<Add, lightyear_connection::client_of::ClientOf>,
    mut commands: Commands,
    players: Query<(Entity, Option<&lightyear::prelude::ControlledBy>), With<PlayerPos>>,
    game_world: Res<GameWorld>,
    player_resource: Res<PlayerState>,
    mut next_player_id: ResMut<NextPlayerId>,
    remote_ids: Query<&lightyear::prelude::RemoteId>,
    mut diagnostics: ResMut<ServerConnectionDiagnostics>,
    mut sessions: ResMut<DisconnectedPlayerSessions>,
    mut reserved_players: ResMut<ReservedPlayerEntities>,
) {
    let client_of_entity = trigger.entity;
    diagnostics.record_connected();
    let remote_id = remote_ids
        .get(client_of_entity)
        .ok()
        .map(|id| id.0.to_bits());
    info!(
        "[net] client {:?} connected on {:?}; active={}, connected_total={}",
        remote_id, client_of_entity, diagnostics.active_connections, diagnostics.connected_total,
    );

    commands.entity(client_of_entity).insert((
        lightyear::prelude::ReplicationSender::default(),
        lightyear::prelude::MessageReceiver::<PingMessage>::default(),
        lightyear::prelude::MessageReceiver::<TerrainSnapshotRequest>::default(),
        LastClientCommandSequence::default(),
    ));
    info!(
        "[net] ReplicationSender attached to ClientOf entity {:?} — now this client can receive replicated entities",
        client_of_entity
    );

    let entity = if let Some((entity, _)) = players
        .iter()
        .find(|(entity, owner)| owner.is_none() && !reserved_players.0.contains(entity))
    {
        entity
    } else {
        let player_id = next_player_id.0;
        next_player_id.0 = next_player_id.0.saturating_add(1);
        let (spawn, spawn_block) = connected_player_spawn(
            &game_world,
            player_resource.block_pos,
            player_resource.pos,
            player_id,
        );
        let session = remote_id.and_then(|key| sessions.0.remove(&key));
        let (state, mounted, saved_cart) = session.map_or_else(
            || {
                (
                    PlayerState {
                        pos: spawn,
                        block_pos: spawn_block,
                        ..default()
                    },
                    false,
                    None,
                )
            },
            |session| (session.state, session.mounted, session.cart),
        );
        let restored_pos = state.pos;
        let restored_block = state.block_pos;
        spawn_server_player(
            &mut commands,
            format!("Player-{player_id}"),
            player_id,
            state,
            mounted,
            saved_cart,
            ServerJumpState::default(),
            AntiStuckState {
                last_safe_pos: restored_pos,
                last_safe_block: restored_block,
                failed_move_ticks: 0,
            },
        )
    };
    reserved_players.0.insert(entity);
    commands.entity(entity).insert((
        lightyear::prelude::Replicate::to_clients(lightyear::prelude::NetworkTarget::All),
        lightyear::prelude::ControlledBy {
            owner: client_of_entity,
            lifetime: lightyear::prelude::Lifetime::Persistent,
        },
    ));
    info!(
        "[net] Replicate + ControlledBy attached to Player entity {:?} — owner={:?}, target=all",
        entity, client_of_entity
    );
}

fn record_disconnected_client(
    trigger: On<Add, lightyear_connection::client::Disconnected>,
    mut commands: Commands,
    connections: Query<(
        &lightyear_connection::client::Disconnected,
        Option<&lightyear::prelude::RemoteId>,
    )>,
    mut diagnostics: ResMut<ServerConnectionDiagnostics>,
    mut sessions: ResMut<DisconnectedPlayerSessions>,
    mut reserved_players: ResMut<ReservedPlayerEntities>,
    mut snapshot_requests: ResMut<VoxelSnapshotResyncRequests>,
    players: Query<
        (
            Entity,
            &lightyear::prelude::ControlledBy,
            &PlayerTag,
            &PlayerStateComponent,
            &CartMounted,
        ),
        With<PlayerPos>,
    >,
    carts: Query<(Entity, &CartState)>,
) {
    let Ok((disconnected, remote_id)) = connections.get(trigger.entity) else {
        return;
    };
    diagnostics.record_disconnected(disconnected.reason.as_deref());
    snapshot_requests.pending.remove(&trigger.entity);
    snapshot_requests.last_accepted_tick.remove(&trigger.entity);
    let session_key = remote_id.map(|id| id.0.to_bits());
    for (player, owner, player_tag, player_state, mounted) in &players {
        if owner.owner == trigger.entity {
            reserved_players.0.remove(&player);
            if let Some(key) = session_key.as_ref() {
                let cart = carts
                    .iter()
                    .find(|(_, cart)| cart.player_id == player_tag.id())
                    .map(|(_, cart)| *cart);
                sessions.0.insert(
                    *key,
                    DisconnectedPlayerSession {
                        state: player_state.0.clone(),
                        mounted: mounted.0,
                        cart,
                    },
                );
            }
            commands
                .entity(player)
                .despawn_related::<Children>()
                .despawn();
            if let Some((cart, _)) = carts
                .iter()
                .find(|(_, cart)| cart.player_id == player_tag.id())
            {
                commands.entity(cart).despawn();
            }
        }
    }
    info!(
        "[net] client {:?} disconnected from {:?}: {}; active={}, disconnected_total={}",
        remote_id.map(|id| id.0),
        trigger.entity,
        disconnected
            .reason
            .as_deref()
            .unwrap_or("no reason provided"),
        diagnostics.active_connections,
        diagnostics.disconnected_total,
    );
}

#[derive(bevy::prelude::Resource, Default)]
pub struct ServerTickCounter(pub u32);

fn broadcast_player_pos(
    mut tick: ResMut<ServerTickCounter>,
    q: Query<&Transform, With<PlayerPos>>,
    server_q: Query<&lightyear::prelude::Server, With<lightyear_connection::server::Started>>,
    mut sender: lightyear::prelude::ServerMultiMessageSender<()>,
) {
    tick.0 = tick.0.wrapping_add(1);

    if tick.0 % SERVER_POS_UPDATE_INTERVAL_TICKS != 0 {
        return;
    }
    let Ok(server) = server_q.single() else {
        return;
    };

    for transform in q.iter() {
        // golab's lesson: a single NaN sneaking into the wire corrupts every
        // downstream Bevy transform on every client. Build the message
        // through the finite-checked helper so corrupted engine state never
        // reaches the wire; the next tick will repopulate a sane value.
        let Some(msg) =
            lk2_core::protocol::wire_format::try_make_server_pos_update(&transform, tick.0)
        else {
            error!(
                "[net] dropping non-finite ServerPosUpdate from broadcast: pos={:?}, tick={}",
                transform.translation, tick.0
            );
            continue;
        };

        let _ =
            sender.send::<_, StateChannel>(&msg, server, &lightyear::prelude::NetworkTarget::All);
    }
}

fn setup_world(
    _commands: Commands,
    mut game_world: ResMut<GameWorld>,
    scenario_state: Res<ScenarioState>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut regions: ResMut<NatureRegionWorld>,
    saves: Res<LatestNatureRegionSaves>,
    mut restore_status: ResMut<NatureRestoreStatus>,
) {
    let world_config = scenario_state
        .scenario
        .as_ref()
        .map(|scenario| scenario.world_config("default"))
        .unwrap_or_default();
    *game_world = generate_world(&world_config);
    if let Some(path) = persistence::TerrainSavePath::from_env().0 {
        match persistence::read_terrain_save(&path) {
            Ok(save) => match save.apply_to_world(&mut game_world) {
                Ok(()) => info!("[terrain] restored edits from {:?}", path),
                Err(error) => error!("[terrain] rejected save {:?}: {}", path, error),
            },
            Err(error) => warn!("[terrain] no usable save {:?}: {}", path, error),
        }
    }
    info!("[terrain] using preset '{}'", game_world.pipeline.name);
    if let Some(content) = game_world.content.as_ref() {
        info!(
            "[content] 3D volume {:?}, settlement {:?}, monster territory {:?}, dungeon entrance {:?}, treasure {:?}",
            content.volume.dimensions,
            content.settlement_position,
            content.monster_position,
            content.entrance_position,
            content.treasure_position
        );
    }

    use lk2_core::resource::ResourceKind;
    for k in ResourceKind::ALL {
        let init = k.demo_initial_amount();
        let _ = pool.force_add(*k, init);
    }

    let monster_anchor = game_world.content.as_ref().map_or(
        [
            constant::WORLD_SIZE / 2,
            constant::SEA_LEVEL + 1,
            constant::WORLD_SIZE / 2 - 8,
        ],
        |content| content.monster_position,
    );
    monsters.demo_init_at(monster_anchor);
    let ecology = EcoCycle::demo_at(Vec2::new(
        constant::WORLD_SIZE as f32 * 0.5,
        constant::WORLD_SIZE as f32 * 0.5,
    ));
    let primary_id = NatureRegionId::from_position(
        [
            constant::WORLD_SIZE as f32 * 0.5,
            constant::WORLD_SIZE as f32 * 0.5,
        ],
        lk2_core::simulation::regions::DEFAULT_REGION_SIZE,
    );
    let primary_restored = saves
        .0
        .get(&primary_id)
        .is_some_and(|save| save.validate_persisted().is_ok());
    restore_status.primary_restored = primary_restored;
    let primary_region = if primary_restored {
        restored_or_generated_region(
            primary_id,
            lk2_core::simulation::cadence::RegionLod::Active,
            &saves,
        )
    } else {
        NatureRegionState::new(
            primary_id,
            ecology,
            pool.clone(),
            lk2_core::simulation::cadence::RegionLod::Active,
        )
    };
    regions.insert(primary_region);
    regions.sync_external_pool_from_primary(&mut pool);

    info!("🌍 世界已生成: {}³ (server)", constant::WORLD_SIZE);
}

fn stream_terrain_chunks(
    mut world: ResMut<GameWorld>,
    players: Query<&PlayerStateComponent, With<PlayerTag>>,
) {
    let mut wanted = HashSet::new();
    let vertical_chunks = (world.size + TERRAIN_CHUNK_SIZE - 1) / TERRAIN_CHUNK_SIZE;

    for player in &players {
        let [px, _, pz] = player.0.block_pos;
        let center = TerrainChunkCoord::from_block([px, 0, pz]);
        for z in
            (center.z - TERRAIN_STREAM_RADIUS_CHUNKS)..=(center.z + TERRAIN_STREAM_RADIUS_CHUNKS)
        {
            for x in (center.x - TERRAIN_STREAM_RADIUS_CHUNKS)
                ..=(center.x + TERRAIN_STREAM_RADIUS_CHUNKS)
            {
                for y in 0..vertical_chunks {
                    let coord = TerrainChunkCoord::new(x, y, z);
                    let origin = coord.origin();
                    if !world.procedural
                        && (origin[0] < 0
                            || origin[2] < 0
                            || origin[0] >= world.size
                            || origin[2] >= world.size)
                    {
                        continue;
                    }
                    wanted.insert(coord);
                }
            }
        }
    }

    for coord in wanted.iter().copied() {
        world.materialize_chunk(coord);
    }

    let loaded = world.loaded_chunks.loaded_coords().collect::<Vec<_>>();
    for coord in loaded {
        if !wanted.contains(&coord) {
            let _ = world.unload_chunk(coord);
        }
    }
}

fn sync_nature_regions(
    mut regions: ResMut<NatureRegionWorld>,
    saves: Res<LatestNatureRegionSaves>,
    players: Query<&PlayerStateComponent, With<PlayerTag>>,
) {
    let mut wanted = Vec::new();
    let mut region_lods: HashMap<NatureRegionId, lk2_core::simulation::cadence::RegionLod> =
        HashMap::new();
    let active_radius = constant::PLAYER_VISION_RADIUS as f32;
    let nearby_radius = active_radius * 3.0;

    for player in &players {
        let [px, _, pz] = player.0.block_pos;
        let player_position = [px as f32, pz as f32];
        let base = NatureRegionId::from_position(
            player_position,
            lk2_core::simulation::regions::DEFAULT_REGION_SIZE,
        );
        if regions.get(base).is_none() {
            regions.insert(restored_or_generated_region(
                base,
                lk2_core::simulation::cadence::RegionLod::Active,
                &saves,
            ));
        }
        for dz in -1..=1 {
            for dx in -1..=1 {
                let id = NatureRegionId {
                    x: base.x + dx,
                    z: base.z + dz,
                };
                let center = id.center(lk2_core::simulation::regions::DEFAULT_REGION_SIZE);
                let distance = Vec2::new(
                    center[0] - player_position[0],
                    center[1] - player_position[1],
                )
                .length();
                let lod = lk2_core::simulation::cadence::RegionLod::for_distance(
                    distance,
                    active_radius,
                    nearby_radius,
                );
                region_lods
                    .entry(id)
                    .and_modify(|current| *current = current.max(lod))
                    .or_insert(lod);
                if regions.get(id).is_none() {
                    regions.insert(restored_or_generated_region(id, lod, &saves));
                }
                wanted.push(id);
            }
        }
    }

    for (id, lod) in region_lods {
        if let Some(region) = regions.get_mut(id) {
            region.scheduler.set_lod(lod);
        }
    }
    regions.retain_except_primary(&wanted);
}

fn restored_or_generated_region(
    id: NatureRegionId,
    lod: lk2_core::simulation::cadence::RegionLod,
    saves: &LatestNatureRegionSaves,
) -> NatureRegionState {
    if let Some(save) = saves
        .0
        .get(&id)
        .filter(|save| save.validate_persisted().is_ok())
    {
        let mut ecology = EcoCycle::default();
        ecology.apply_snapshot(&save.state.snapshot.detailed_ecology);
        let mut region = NatureRegionState::new(id, ecology, save.state.resources.clone(), lod);
        region.scheduler.sync_to_tick(save.tick);
        return region;
    }
    let center = id.center(lk2_core::simulation::regions::DEFAULT_REGION_SIZE);
    NatureRegionState::new(
        id,
        EcoCycle::demo_at(Vec2::new(center[0], center[1])),
        GlobalResourcePool::new(),
        lod,
    )
}

fn self_check(
    game_world: Res<GameWorld>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    regions: Res<NatureRegionWorld>,
    mut obs: ResMut<TickObserver>,
) {
    let Some(eco) = regions.primary().map(|region| &region.ecology) else {
        return;
    };
    info!(">>> 服务端启动自检 100 tick ...");
    let report = lk2_core::diagnostics::run_self_check(
        &game_world,
        &pool,
        &nations,
        &monsters,
        &eco,
        &mut obs,
        [
            constant::WORLD_SIZE / 2,
            constant::SEA_LEVEL + 2,
            constant::WORLD_SIZE / 2,
        ],
        100,
    );
    let violations = report.violations;
    if violations.is_empty() {
        info!(">>> 自检 ✅ 100 tick 全部通过 (server)");
        info!(
            ">>> Listening on UDP 0.0.0.0:{} - ready for client connections",
            port_from_env()
        );
    } else {
        error!(">>> 自检 ❌ {} 处违例", violations.len());
    }
    info!("{}", obs.report());
}

fn run_startup_self_check_once(
    mut fired: Local<bool>,
    game_world: Res<GameWorld>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    regions: Res<NatureRegionWorld>,
    mut obs: ResMut<TickObserver>,
) {
    if *fired {
        return;
    }
    *fired = true;
    let Some(eco) = regions.primary().map(|region| &region.ecology) else {
        return;
    };
    // Run the same body as `self_check` but lazily on first Update tick — that
    // way Startup completes immediately and the UDP loop can drain packets from
    // the moment we accept connections.
    info!(">>> 服务端延迟自检 100 tick (deferred from Startup)");
    let report = lk2_core::diagnostics::run_self_check(
        &game_world,
        &pool,
        &nations,
        &monsters,
        &eco,
        &mut obs,
        [
            constant::WORLD_SIZE / 2,
            constant::SEA_LEVEL + 2,
            constant::WORLD_SIZE / 2,
        ],
        100,
    );
    let violations = report.violations;
    if violations.is_empty() {
        info!(">>> 延迟自检 ✅ 100 tick 全部通过 (server)");
    } else {
        error!(">>> 延迟自检 ❌ {} 处违例", violations.len());
    }
}

fn port_from_env() -> u16 {
    std::env::var("LK2_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5000)
}

fn simulation_tick(
    fixed_time: Res<Time<Fixed>>,
    mut clock: ResMut<SimClock>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut regions: ResMut<NatureRegionWorld>,
    mut obs: ResMut<TickObserver>,
    mut latest_nature: ResMut<LatestNatureReport>,
    mut latest_regions: ResMut<LatestNatureRegionReports>,
) {
    // The regional pool is authoritative. The legacy global resource is a
    // compatibility mirror because gameplay systems still query it.
    // Synchronize it through NatureRegionWorld so every migration caller uses
    // the same ownership boundary.
    regions.sync_primary_resources_from_external(&pool);
    let (primary_id, report) = {
        let Some(primary) = regions.primary_mut() else {
            return;
        };
        let id = primary.id;
        let report = advance_fixed_authority_tick_report(
            fixed_time.delta_secs(),
            &mut clock,
            &mut primary.resources,
            &mut monsters,
            &mut primary.ecology,
            &mut obs,
            SimRole::ServerAuthority,
        );
        if let Some(report) = report.as_ref() {
            primary.scheduler.sync_to_tick(report.tick);
        }
        (id, report)
    };
    regions.sync_external_pool_from_primary(&mut pool);
    if let Some(report) = report {
        if report.snapshot.is_finite() {
            latest_regions.0.insert(primary_id, report.clone());
            latest_nature.0 = Some(report);
        } else {
            warn!(
                "[nature] dropping non-finite authoritative snapshot at tick {}",
                report.tick
            );
        }
    }
    for (id, report) in regions.advance_all(clock.tick) {
        if report.snapshot.is_finite() {
            latest_regions.0.insert(id, report);
        } else {
            warn!(
                "[nature] dropping non-finite background region snapshot at tick {}",
                report.tick
            );
        }
    }
    latest_regions.0.retain(|id, _| regions.get(*id).is_some());
}

fn end_tick_system(
    game_world: Res<GameWorld>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    clock: Res<SimClock>,
    mut obs: ResMut<TickObserver>,
    players: Query<&PlayerStateComponent, With<PlayerTag>>,
) {
    if !clock.last_sim_step_ran {
        return;
    }

    let Some(player) = players.iter().next().map(|state| &state.0) else {
        return;
    };
    if let Err(e) = obs.end_tick(
        clock.tick,
        &game_world,
        &pool,
        &nations,
        &monsters,
        Some(player.block_pos),
    ) {
        for line in e {
            error!("invariant: {}", line);
        }
    }
}

#[derive(Resource, Default)]
pub struct TickRecorder {
    pub last_dump_tick: u64,
    pub current_iter: u32,
}

#[derive(Resource, Default)]
struct NatureRestoreStatus {
    primary_restored: bool,
}

#[derive(SystemParam)]
struct TickRecorderNature<'w> {
    regions: Res<'w, NatureRegionWorld>,
    reports: Res<'w, LatestNatureRegionReports>,
    save: Res<'w, LatestNatureSave>,
    restore_status: Res<'w, NatureRestoreStatus>,
}

fn nature_capture_evidence(
    world_tick: u64,
    region_tick: u64,
    event_count: usize,
    save_restored: bool,
) -> serde_json::Value {
    serde_json::json!({
        "tick": world_tick,
        "event_count": event_count,
        "region_tick": region_tick,
        "catch_up_remaining": world_tick.saturating_sub(region_tick),
        "save_restored": save_restored,
    })
}

fn tick_recorder(
    time: Res<Time>,
    mut rec: ResMut<TickRecorder>,
    clock: Res<SimClock>,
    players: Query<(&PlayerStateComponent, &VoxelChunkSnapshot), With<PlayerTag>>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    nature: TickRecorderNature,
    obs: Res<TickObserver>,
    game_world: Res<GameWorld>,
    diagnostics: Res<ServerCommandDiagnostics>,
    network_entities: Query<(
        Option<&lightyear::prelude::Server>,
        Option<&lightyear_connection::server::Started>,
        Option<&lightyear::prelude::server::LinkOf>,
    )>,
) {
    if std::env::var("LK2_CAPTURE").is_err() {
        return;
    }
    if clock.tick == 0 || clock.tick % 5 != 0 || clock.tick == rec.last_dump_tick {
        return;
    }
    let Some((player_state, chunk)) = players.iter().next() else {
        return;
    };
    let Some(eco) = nature.regions.primary().map(|region| &region.ecology) else {
        return;
    };
    let Some(primary) = nature.regions.primary() else {
        return;
    };
    let primary_tick = primary.scheduler.simulated_tick();
    let primary_event_count = nature
        .reports
        .0
        .get(&primary.id)
        .map_or(0, |report| report.events.len());
    let player = &player_state.0;
    rec.last_dump_tick = clock.tick;
    rec.current_iter = clock.tick as u32;
    let path = format!("screenshots/server_state_t{}.json", clock.tick);
    let mut state = lk2_core::diagnostics::build_state_json(
        &time,
        &clock,
        &player,
        &pool,
        &nations,
        &monsters,
        &eco,
        &obs,
        &game_world,
        SimRole::ServerAuthority.into(),
    );
    if let Some(obj) = state.as_object_mut() {
        obj.insert(
            "nature".to_string(),
            nature_capture_evidence(
                clock.tick,
                primary_tick,
                primary_event_count,
                nature.restore_status.primary_restored,
            ),
        );
        obj.insert(
            "network_command".to_string(),
            serde_json::json!({
                "receiver_entities": diagnostics.receiver_entities,
                "server_entities": network_entities
                    .iter()
                    .filter(|(server, _, _)| server.is_some())
                    .count(),
                "started_servers": network_entities
                    .iter()
                    .filter(|(_, started, _)| started.is_some())
                    .count(),
                "link_of_entities": network_entities
                    .iter()
                    .filter(|(_, _, link_of)| link_of.is_some())
                    .count(),
                "move_world_received": diagnostics.move_world_received,
                "last_dx_milli": diagnostics.last_dx_milli,
                "last_dz_milli": diagnostics.last_dz_milli,
                "last_move_applied": diagnostics.last_move_applied,
            }),
        );
        obj.insert(
            "persistence".to_string(),
            serde_json::json!({
                "schema_version": nature.save.0.as_ref().map(|value| value.schema_version),
                "tick": nature.save.0.as_ref().map(|value| value.tick),
                "valid": nature.save.0.as_ref().is_some_and(|value| value.validate().is_ok()),
            }),
        );
        obj.insert(
            "voxel_chunk".to_string(),
            serde_json::json!({
                "revision": chunk.revision,
                "chunk_x": chunk.chunk_x,
                "chunk_z": chunk.chunk_z,
                "border": chunk.border,
                "y_min": chunk.y_min,
                "y_size": chunk.y_size,
                "block_count": chunk.blocks.len(),
                "valid_block_count": chunk.blocks.iter().filter(|block| **block <= 13).count(),
            }),
        );
    }
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lk2_core::world::BlockType;

    #[test]
    fn server_creatures_start_wandering_after_a_short_delay() {
        let ai = server_creature_ai();

        assert_eq!(ai.wander_timer, 0.0);
        assert_eq!(ai.next_wander_secs, CREATURE_INITIAL_WANDER_SECS);
        assert!(ai.next_wander_secs <= 3.0);
    }

    #[test]
    fn block_type_wire_codes_keep_existing_values_and_append_grass() {
        assert_eq!(block_type_to_u8(BlockType::Air), 0);
        assert_eq!(block_type_to_u8(BlockType::Dirt), 1);
        assert_eq!(block_type_to_u8(BlockType::Stone), 2);
        assert_eq!(block_type_to_u8(BlockType::BerryThicket), 12);
        assert_eq!(block_type_to_u8(BlockType::Grass), 13);
    }

    #[test]
    fn water_column_exposes_a_swimmable_floor_and_bounded_vertical_input() {
        // The normal server fixture installs a large spawn platform that
        // intentionally covers the finite test window. Disable that authored
        // platform here so this test exercises the procedural water columns.
        let world = generate_world(&WorldConfig {
            install_spawn_platform: false,
            generate_content: false,
            ..WorldConfig::default()
        });
        let water = (0..world.size)
            .flat_map(|x| (0..world.size).map(move |z| (x, z)))
            .find_map(|(x, z)| {
                let position =
                    Vec3::new(x as f32 + 0.5, constant::SEA_LEVEL as f32, z as f32 + 0.5);
                can_swim_at(&world, position).then_some((x, z))
            })
            .expect("generated world should contain water");
        let position = Vec3::new(
            water.0 as f32 + 0.5,
            constant::SEA_LEVEL as f32,
            water.1 as f32 + 0.5,
        );
        let depth = water_depth_at(&world, position).expect("water should expose a floor");

        assert!(depth >= 0.0);
        assert!(can_swim_at(&world, position));
        assert_eq!(sanitize_swim_vertical(i16::MAX), 1.0);
        assert_eq!(sanitize_swim_vertical(i16::MIN), -1.0);
    }

    #[test]
    fn initial_chunk_snapshot_contains_materialized_3d_content() {
        let world = generate_world(&WorldConfig::default());
        let content = world.content.as_ref().expect("default content");
        let snapshot = build_voxel_chunk_snapshot(&world, 0, content.settlement_position);
        let treasure = content.treasure_position;
        let snapshot_size = VOXEL_CHUNK_SIZE_XZ + snapshot.border * 2;
        let x_min = snapshot.chunk_x * VOXEL_CHUNK_SIZE_XZ - snapshot.border;
        let z_min = snapshot.chunk_z * VOXEL_CHUNK_SIZE_XZ - snapshot.border;
        let index = (((treasure[1] - 1 - snapshot.y_min) * snapshot_size + (treasure[2] - z_min))
            * snapshot_size
            + (treasure[0] - x_min)) as usize;

        assert_eq!(
            snapshot.blocks.get(index).copied(),
            Some(block_type_to_u8(BlockType::SunstoneOre))
        );
        assert_eq!(snapshot.border, 1);
        assert_eq!(snapshot.blocks.len(), (18 * 18 * snapshot.y_size) as usize);
    }

    #[test]
    fn chunk_snapshot_includes_adjacent_chunk_border_content() {
        let mut world =
            GameWorld::with_pipeline(32, lk2_core::world::terrain::presets::default_preset());
        world.set(16, 1, 8, BlockType::Stone);

        let snapshot = build_voxel_chunk_snapshot(&world, 3, [8, 1, 8]);
        let snapshot_size = VOXEL_CHUNK_SIZE_XZ + snapshot.border * 2;
        let x_min = snapshot.chunk_x * VOXEL_CHUNK_SIZE_XZ - snapshot.border;
        let z_min = snapshot.chunk_z * VOXEL_CHUNK_SIZE_XZ - snapshot.border;
        let index = (((1 - snapshot.y_min) * snapshot_size + (8 - z_min)) * snapshot_size
            + (16 - x_min)) as usize;

        assert_eq!(snapshot.border, 1);
        assert_eq!(snapshot.blocks[index], block_type_to_u8(BlockType::Stone));
    }

    #[test]
    fn chunk_snapshot_cache_reuses_revision_and_chunk() {
        let world = generate_world(&WorldConfig::default());
        let position = [48, 20, 48];
        let mut cache = VoxelChunkSnapshotCache::default();

        let first = cached_voxel_chunk_snapshot(&world, 7, position, &mut cache).clone();
        let second = cached_voxel_chunk_snapshot(&world, 7, [49, 20, 49], &mut cache);
        assert_eq!(first, *second);
        assert_eq!(cache.snapshots.len(), 1);

        let _ = cached_voxel_chunk_snapshot(&world, 8, position, &mut cache);
        assert_eq!(cache.revision, 8);
        assert_eq!(cache.snapshots.len(), 1);
    }

    #[test]
    fn player_volume_clear_rejects_world_edge_escape() {
        let mut world = GameWorld::new(8);
        for x in 0..world.size {
            for z in 0..world.size {
                world.set(x, 0, z, BlockType::Stone);
            }
        }

        assert!(!player_volume_clear_at(&world, Vec3::new(8.1, 1.0, 3.5)));
        assert!(!player_volume_clear_at(&world, Vec3::new(-0.1, 1.0, 3.5)));
        assert!(player_volume_clear_at(&world, Vec3::new(3.5, 1.0, 3.5)));
    }

    #[test]
    fn connection_diagnostics_do_not_underflow_on_duplicate_disconnect() {
        let mut diagnostics = ServerConnectionDiagnostics::default();
        diagnostics.record_connected();
        diagnostics.record_disconnected(Some("timeout"));
        diagnostics.record_disconnected(None);

        assert_eq!(diagnostics.active_connections, 0);
        assert_eq!(diagnostics.connected_total, 1);
        assert_eq!(diagnostics.disconnected_total, 1);
        assert_eq!(
            diagnostics.last_disconnect_reason.as_deref(),
            Some("timeout")
        );
    }

    #[test]
    fn terrain_resync_budget_coalesces_pending_requests_per_connection() {
        let mut world = World::new();
        let owner = world.spawn_empty().id();
        let request = |request_id| TerrainSnapshotRequest {
            request_id,
            chunk_x: 1,
            chunk_z: -2,
            known_revision: 7,
        };
        let mut budget = VoxelSnapshotResyncRequests::default();

        assert!(budget.accept(owner, 1, request(1)));
        assert!(!budget.accept(owner, 1, request(2)));
        assert_eq!(budget.pending.len(), 1);

        budget.pending.remove(&owner);
        assert!(!budget.accept(owner, 15, request(3)));
        assert!(budget.accept(owner, 16, request(4)));
    }

    #[test]
    fn nature_capture_evidence_reports_bounded_catch_up_and_restore_status() {
        let evidence = nature_capture_evidence(12, 9, 3, true);

        assert_eq!(evidence["tick"], 12);
        assert_eq!(evidence["event_count"], 3);
        assert_eq!(evidence["region_tick"], 9);
        assert_eq!(evidence["catch_up_remaining"], 3);
        assert_eq!(evidence["save_restored"], true);
    }

    #[test]
    fn gameplay_command_sequences_accept_only_new_nonzero_values() {
        let mut last = 0;
        assert!(!accept_client_command_sequence(&mut last, 0));
        assert!(accept_client_command_sequence(&mut last, 1));
        assert_eq!(last, 1);
        assert!(!accept_client_command_sequence(&mut last, 1));
        assert!(!accept_client_command_sequence(&mut last, 0));
        assert!(!accept_client_command_sequence(&mut last, 0));
        assert!(accept_client_command_sequence(&mut last, 2));
        assert!(!accept_client_command_sequence(&mut last, 1));
        assert_eq!(last, 2);
    }

    #[test]
    fn online_move_input_is_bounded_to_walk_or_sprint_magnitude() {
        let direction = sanitize_online_move_direction(i16::MAX, i16::MAX)
            .expect("non-zero movement should be accepted");

        assert!((direction.length() - ONLINE_INPUT_MAX_MAGNITUDE).abs() < 0.001);
        assert!(sanitize_online_move_direction(0, 0).is_none());
    }

    #[test]
    fn mine_command_requires_reach_and_updates_authoritative_world() {
        let mut world = GameWorld::new(8);
        world.set(4, 1, 4, BlockType::Stone);
        let mut pool = GlobalResourcePool::new();
        let mut nations = NationRegistry::default();
        let mut player = PlayerState {
            block_pos: [4, 2, 4],
            ..default()
        };
        let mut revision = WorldRevision::default();
        let mut delta = LastVoxelDeltaState::default();

        let result = apply_gameplay_command(
            11,
            &GameplayCommand {
                sequence: 1,
                tick: 1,
                player_block: [4, 2, 4],
                kind: GameplayCommandKind::MineTarget { target: [4, 1, 4] },
            },
            &mut world,
            &mut pool,
            &mut nations,
            &mut player,
            &mut revision,
            &mut delta,
            None,
        );

        assert!(result.ok);
        assert_eq!(world.get(4, 1, 4), BlockType::Air);
        assert_eq!(player.inventory.get(&ResourceKind::Stone), Some(&1));
        assert!(revision.0 > 0);

        let rejected = apply_gameplay_command(
            11,
            &GameplayCommand {
                sequence: 2,
                tick: 2,
                player_block: [4, 2, 4],
                kind: GameplayCommandKind::MineTarget { target: [0, 0, 0] },
            },
            &mut world,
            &mut pool,
            &mut nations,
            &mut player,
            &mut revision,
            &mut delta,
            None,
        );

        assert!(!rejected.ok);
    }

    #[test]
    fn online_nation_founder_id_is_connection_specific() {
        let mut pool = GlobalResourcePool::new();
        pool.force_add(ResourceKind::Soul, 100);
        let mut nations = NationRegistry::default();
        let mut first = PlayerState::default();
        let mut second = PlayerState::default();
        let mut world = GameWorld::new(8);
        let mut revision = WorldRevision::default();
        let mut delta = LastVoxelDeltaState::default();

        let first_result = apply_gameplay_command(
            11,
            &GameplayCommand {
                sequence: 3,
                tick: 1,
                player_block: [0, 0, 0],
                kind: GameplayCommandKind::FoundNation,
            },
            &mut world,
            &mut pool,
            &mut nations,
            &mut first,
            &mut revision,
            &mut delta,
            None,
        );
        let second_result = apply_gameplay_command(
            22,
            &GameplayCommand {
                sequence: 4,
                tick: 2,
                player_block: [1, 0, 1],
                kind: GameplayCommandKind::FoundNation,
            },
            &mut world,
            &mut pool,
            &mut nations,
            &mut second,
            &mut revision,
            &mut delta,
            None,
        );

        assert!(first_result.ok);
        assert!(second_result.ok);
        assert_ne!(first.nation_id, second.nation_id);
    }

    #[test]
    fn connected_players_do_not_share_the_initial_spawn_tile() {
        let world = generate_world(&WorldConfig::default());
        let base = [world.size / 2, 0, world.size / 2];
        let first = connected_player_spawn(
            &world,
            base,
            Vec3::new(base[0] as f32 + 0.5, 1.0, base[2] as f32 + 0.5),
            1,
        );
        let second = connected_player_spawn(
            &world,
            base,
            Vec3::new(base[0] as f32 + 0.5, 1.0, base[2] as f32 + 0.5),
            2,
        );

        assert_ne!(first.1, second.1);
    }

    #[test]
    fn plank_pack_craft_uses_player_inventory_atomically() {
        let mut player = PlayerState::default();
        player
            .inventory
            .insert(ResourceKind::Wood, PLANK_PACK_WOOD_COST);

        let result = apply_gameplay_command(
            0,
            &GameplayCommand {
                sequence: 5,
                tick: 1,
                player_block: [0, 0, 0],
                kind: GameplayCommandKind::Craft(BuildRecipe::PlankPack),
            },
            &mut GameWorld::new(8),
            &mut GlobalResourcePool::new(),
            &mut NationRegistry::default(),
            &mut player,
            &mut WorldRevision::default(),
            &mut LastVoxelDeltaState::default(),
            None,
        );

        assert!(result.ok);
        assert_eq!(player.inventory.get(&ResourceKind::Wood), Some(&0));
        assert_eq!(
            player.inventory.get(&ResourceKind::HardenedWood),
            Some(&PLANK_PACK_OUTPUT)
        );
    }
}
