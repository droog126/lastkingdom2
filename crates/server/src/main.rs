#![allow(dead_code)]
#![allow(unused_imports)]

use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;
use lightyear::prelude::LocalAddr;
use lightyear::prelude::server::ServerUdpIo;

use leafwing_input_manager::prelude::ActionState;
use lk2_core::protocol::PlayerAction;
use lk2_core::protocol::components::{
    EcoSnapshot, GameplayHudState, PlayerPos, VOXEL_CHUNK_SIZE_XZ, VoxelChunkSnapshot, VoxelDelta,
};
use lk2_core::protocol::messages::{
    BuildRecipe, GameplayCommand, GameplayCommandKind, GameplayFeedback, PingMessage, PongMessage,
};

use lightyear::prelude::PeerMetadata;

use std::time::Duration;

mod los;
mod pvp_systems;

use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::constant;
use lk2_core::creature::{
    CREATURE_TRAINING_ATTACK_RANGE_SQ, Creature, CreatureAI, CreatureKind, CreatureSpawnerDone,
    award_creature_drop, creature_attack_distance_sq,
};
use lk2_core::eco_cycle::EcoCycle;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::{PlayerState, PlayerTag};
use lk2_core::pvp::FixedTick;
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::scenario::{Scenario, ScenarioState};
use lk2_core::sim::{SimRole, advance_fixed_authority_tick};
use lk2_core::transport::{
    DEFAULT_PORT, NETCODE_CLIENT_TIMEOUT_SECS, PRIVATE_KEY, PROTOCOL_ID,
    SERVER_POS_UPDATE_INTERVAL_TICKS, gameplay_port_for,
};
use lk2_core::v2::app_sets::SimSet;
use lk2_core::world::{
    World as GameWorld, WorldConfig, WorldGenerator, fallback_spawn_ring_offsets, generate_world,
    player_body_clear, player_position_is_safe, player_spawn_position_at,
    player_spawn_position_near, player_stand_position_at, resolve_player_stuck_near,
};

use crate::pvp_systems::{
    ServerPvPPlugin, apply_damage_and_knockback, expire_knockback_immunity, melee_hit_registration,
    read_attack_inputs, record_position_history, tick_combat_cooldowns,
};

const PLACE_WOOD_COST: i64 = 1;
const ONLINE_MOVE_SPEED: f32 = 4.5;
const ONLINE_STEP_THRESHOLD: f32 = 0.85;
const PLAYER_COLLISION_RADIUS: f32 = 0.34;
const JUMP_TAKEOFF_SPEED: f32 = 7.2;
const JUMP_GRAVITY: f32 = 28.0;
const JUMP_TERMINAL_SPEED: f32 = -18.0;
const ANTI_STUCK_SEARCH_RADIUS: i32 = 6;
const ANTI_STUCK_FAILED_MOVE_LIMIT: u8 = 12;

#[derive(Resource, Debug, Clone, Copy)]
struct ServerJumpState {
    velocity_y: f32,
    grounded: bool,
}

impl Default for ServerJumpState {
    fn default() -> Self {
        Self { velocity_y: 0.0, grounded: true }
    }
}

#[derive(Resource, Debug, Clone)]
struct AntiStuckState {
    last_safe_pos: Vec3,
    last_safe_block: [i32; 3],
    failed_move_ticks: u8,
}

impl Default for AntiStuckState {
    fn default() -> Self {
        Self { last_safe_pos: Vec3::ZERO, last_safe_block: [0, 0, 0], failed_move_ticks: 0 }
    }
}

#[derive(Resource, Default)]
struct WorldRevision(u64);

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
    pub udp_commands_received: u64,
}

#[derive(Resource)]
struct GameplayCommandUdp {
    socket: std::net::UdpSocket,
}

#[derive(Resource, Clone)]
struct LastVoxelDeltaState {
    revision: u64,
    x: i32,
    y: i32,
    z: i32,
    block: lk2_core::world::BlockType,
}

impl Default for LastVoxelDeltaState {
    fn default() -> Self {
        Self { revision: 0, x: 0, y: 0, z: 0, block: lk2_core::world::BlockType::Air }
    }
}

fn block_type_to_u8(block: lk2_core::world::BlockType) -> u8 {
    use lk2_core::world::BlockType;
    match block {
        BlockType::Air => 0,
        BlockType::Dirt => 1,
        BlockType::Stone => 2,
        BlockType::Sand => 3,
        BlockType::Snow => 4,
        BlockType::Leaves => 5,
        BlockType::Water => 6,
        BlockType::Wood => 7,
        BlockType::IronOre => 8,
        BlockType::SunstoneOre => 9,
        BlockType::FrostcoreOre => 10,
        BlockType::LivingRoot => 11,
        BlockType::BerryThicket => 12,
        BlockType::Grass => 13,
    }
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
        inventory_wood: player.inventory.get(&ResourceKind::Wood).copied().unwrap_or(0),
        inventory_food: player.inventory.get(&ResourceKind::Food).copied().unwrap_or(0),
        inventory_apple: player.inventory.get(&ResourceKind::Apple).copied().unwrap_or(0),
        inventory_soul: player.inventory.get(&ResourceKind::Soul).copied().unwrap_or(0),
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
    let chunk_x = player_block_pos[0].div_euclid(VOXEL_CHUNK_SIZE_XZ);
    let chunk_z = player_block_pos[2].div_euclid(VOXEL_CHUNK_SIZE_XZ);
    let x_min = chunk_x * VOXEL_CHUNK_SIZE_XZ;
    let z_min = chunk_z * VOXEL_CHUNK_SIZE_XZ;
    let y_min = 0;
    let y_size = world.size;
    let mut blocks =
        Vec::with_capacity((VOXEL_CHUNK_SIZE_XZ * VOXEL_CHUNK_SIZE_XZ * y_size) as usize);

    for y in y_min..(y_min + y_size) {
        for z in z_min..(z_min + VOXEL_CHUNK_SIZE_XZ) {
            for x in x_min..(x_min + VOXEL_CHUNK_SIZE_XZ) {
                blocks.push(block_type_to_u8(world.get(x, y, z)));
            }
        }
    }

    VoxelChunkSnapshot { revision, chunk_x, chunk_z, y_min, y_size, blocks }
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
    *last_delta = LastVoxelDeltaState { revision: revision.0, x, y, z, block };
}

fn apply_gameplay_command(
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
        GameplayCommandKind::GatherFootBlock => {
            let [x, y, z] = [
                player.block_pos[0],
                player.block_pos[1] - 1,
                player.block_pos[2],
            ];
            match lk2_core::world::gather_block(world, pool, x, y, z, 0) {
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
            } else {
                match pool.try_sub(ResourceKind::Wood, PLACE_WOOD_COST) {
                    Ok(_) => {
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
                    Err(err) => {
                        ok = false;
                        format!("not enough wood: {}", err)
                    }
                }
            }
        }
        GameplayCommandKind::Craft(recipe) => {
            ok = false;
            format!("craft {:?} is not implemented on server", recipe)
        }
        GameplayCommandKind::FoundNation => {
            if player.nation_id.is_some() {
                ok = false;
                "already in nation".to_string()
            } else {
                match nations.found(
                    pool,
                    0,
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
    mut world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut nations: ResMut<NationRegistry>,
    mut player: ResMut<PlayerState>,
    mut revision: ResMut<WorldRevision>,
    mut last_delta: ResMut<LastVoxelDeltaState>,
    mut feedback: MessageWriter<GameplayFeedback>,
    mut commands: Commands,
    creatures: Query<(Entity, &Creature, &CreatureAI)>,
    mut diagnostics: ResMut<ServerCommandDiagnostics>,
    mut jump: ResMut<ServerJumpState>,
    mut player_q: Query<
        (
            &mut bevy::prelude::Transform,
            &mut lk2_core::protocol::components::PlayerPos,
        ),
        With<lk2_core::protocol::components::PlayerPos>,
    >,
    mut receivers: Query<
        &mut lightyear::prelude::MessageReceiver<GameplayCommand>,
        With<lightyear_connection::client_of::ClientOf>,
    >,
) {
    diagnostics.receiver_entities = receivers.iter().len();
    for mut receiver in receivers.iter_mut() {
        // Drain the receiver once. We deliberately keep ONLY the latest MoveWorld
        // per tick — clients send at their frame rate (often 144Hz) while we
        // tick at 60Hz, so a backlog of 2-3 MoveWorld commands per tick is
        // normal. Applying every queued command teleports the player; applying
        // only the latest gives us a clean per-tick movement step whose speed
        // is decoupled from the client's frame rate.
        let mut latest_move: Option<(i16, i16)> = None;
        let mut jump_requested = false;
        let mut other_cmds: Vec<GameplayCommand> = Vec::new();
        for cmd in receiver.receive() {
            match cmd.kind {
                GameplayCommandKind::MoveWorld { dx_milli, dz_milli } => {
                    latest_move = Some((dx_milli, dz_milli));
                }
                GameplayCommandKind::Jump => {
                    jump_requested = true;
                }
                _ => other_cmds.push(cmd),
            }
        }

        if jump_requested && jump.grounded {
            jump.velocity_y = JUMP_TAKEOFF_SPEED;
            jump.grounded = false;
        }

        if let Some((dx_milli, dz_milli)) = latest_move {
            diagnostics.move_world_received = diagnostics.move_world_received.saturating_add(1);
            diagnostics.last_dx_milli = dx_milli;
            diagnostics.last_dz_milli = dz_milli;
            // Magnitude is encoded by the client (1.0 walk / 1.5 sprint), so we
            // pass the raw vector to apply_world_move without re-normalizing.
            let dir = Vec2::new(dx_milli as f32 / 1000.0, dz_milli as f32 / 1000.0);
            let moved = if let Ok((mut transform, mut player_pos)) = player_q.single_mut() {
                if jump.grounded {
                    apply_world_move(
                        &mut transform,
                        &mut player_pos,
                        &mut player,
                        &world,
                        dir,
                        1.0 / 60.0,
                    )
                } else {
                    apply_air_world_move(
                        &mut transform,
                        &mut player_pos,
                        &mut player,
                        &world,
                        dir,
                        1.0 / 60.0,
                    )
                }
            } else {
                false
            };
            diagnostics.last_move_applied = moved;
            if moved {
                feedback.write(GameplayFeedback { ok: true, summary: "moved".to_string() });
            }
        }

        if let Ok((mut transform, mut player_pos)) = player_q.single_mut() {
            let _ = step_authoritative_jump(
                &mut transform,
                &mut player_pos,
                &mut player,
                &world,
                &mut jump,
                1.0 / 60.0,
            );
        }

        for cmd in other_cmds {
            let nearest_creature = if matches!(cmd.kind, GameplayCommandKind::KillNearestCreature) {
                creatures
                    .iter()
                    .filter_map(|(entity, creature, _)| {
                        let d2 = creature_attack_distance_sq(player.block_pos, creature.block_pos);
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
                &cmd,
                &mut world,
                &mut pool,
                &mut nations,
                &mut player,
                &mut revision,
                &mut last_delta,
                nearest_kind,
            );
            if result.ok {
                if let Some((entity, _, _)) = nearest_creature {
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
    }
}

fn apply_udp_gameplay_commands(
    udp: Option<Res<GameplayCommandUdp>>,
    mut player: ResMut<PlayerState>,
    world: Res<GameWorld>,
    mut diagnostics: ResMut<ServerCommandDiagnostics>,
    jump: Res<ServerJumpState>,
    mut anti_stuck: ResMut<AntiStuckState>,
    mut player_q: Query<
        (
            &mut bevy::prelude::Transform,
            &mut lk2_core::protocol::components::PlayerPos,
        ),
        With<lk2_core::protocol::components::PlayerPos>,
    >,
) {
    let Some(udp) = udp else {
        return;
    };
    let mut buf = [0u8; 64];
    let mut latest_move: Option<(i16, i16)> = None;
    loop {
        match udp.socket.recv_from(&mut buf) {
            Ok((len, _)) => {
                let Ok(line) = std::str::from_utf8(&buf[..len]) else {
                    continue;
                };
                let mut parts = line.split_whitespace();
                if parts.next() != Some("MOVE") {
                    continue;
                }
                let Some(dx_milli) = parts.next().and_then(|s| s.parse::<i16>().ok()) else {
                    continue;
                };
                let Some(dz_milli) = parts.next().and_then(|s| s.parse::<i16>().ok()) else {
                    continue;
                };
                diagnostics.udp_commands_received =
                    diagnostics.udp_commands_received.saturating_add(1);
                latest_move = Some((dx_milli, dz_milli));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
    let Some((dx_milli, dz_milli)) = latest_move else {
        return;
    };
    diagnostics.last_dx_milli = dx_milli;
    diagnostics.last_dz_milli = dz_milli;
    let dir = Vec2::new(dx_milli as f32 / 1000.0, dz_milli as f32 / 1000.0);
    diagnostics.last_move_applied =
        if let Ok((mut transform, mut player_pos)) = player_q.single_mut() {
            let moved = if jump.grounded {
                apply_world_move(
                    &mut transform,
                    &mut player_pos,
                    &mut player,
                    &world,
                    dir,
                    1.0 / 60.0,
                )
            } else {
                apply_air_world_move(
                    &mut transform,
                    &mut player_pos,
                    &mut player,
                    &world,
                    dir,
                    1.0 / 60.0,
                )
            };
            maintain_anti_stuck(
                &world,
                &mut player,
                &mut transform,
                &mut player_pos,
                &mut anti_stuck,
                moved,
            );
            moved
        } else {
            false
        };
}

fn echo_ping_messages(
    clock: Res<SimClock>,
    mut receivers: Query<
        &mut lightyear::prelude::MessageReceiver<PingMessage>,
        With<lightyear_connection::client_of::ClientOf>,
    >,
    server_q: Query<&lightyear::prelude::Server, With<lightyear_connection::server::Started>>,
    mut sender: lightyear::prelude::ServerMultiMessageSender<()>,
) {
    let Ok(server) = server_q.single() else {
        return;
    };

    for mut receiver in receivers.iter_mut() {
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
            let _ = sender.send::<_, lightyear_replication::metadata::MetadataChannel>(
                &pong,
                server,
                &lightyear::prelude::NetworkTarget::All,
            );
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
        let _ = sender.send::<_, lightyear_replication::metadata::MetadataChannel>(
            msg,
            server,
            &lightyear::prelude::NetworkTarget::All,
        );
    }
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

fn apply_world_move(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    world: &GameWorld,
    dir: Vec2,
    dt: f32,
) -> bool {
    if dir.length_squared() <= 0.0001 {
        return false;
    }
    // Magnitude is meaningful: the client sends 1.0 for walk and 1.5 for sprint,
    // so we do NOT re-normalize here. Per-tick movement = dir * speed * dt, and
    // we drop exactly one MoveWorld per tick on the caller side, so speed stays
    // at ONLINE_MOVE_SPEED (× sprint factor) regardless of client frame rate.
    let step = Vec3::new(dir.x, 0.0, dir.y) * ONLINE_MOVE_SPEED * dt;
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
    if !player_volume_clear_at(world, next_pos) {
        return false;
    }

    transform.translation = next_pos;
    player_pos.0 = next_pos;
    player.pos = next_pos;
    player.block_pos = block_pos;
    true
}

fn apply_air_world_move(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    world: &GameWorld,
    dir: Vec2,
    dt: f32,
) -> bool {
    if dir.length_squared() <= 0.0001 {
        return false;
    }
    let step = Vec3::new(dir.x, 0.0, dir.y) * ONLINE_MOVE_SPEED * dt;
    let next_pos = player.pos + step;
    if !player_volume_clear_at(world, next_pos) {
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
                if x < 0 || x >= world.size || y < 0 || y >= world.size || z < 0 || z >= world.size
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
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    eco: Res<EcoCycle>,
    obs: Res<TickObserver>,
    revision: Res<WorldRevision>,
    last_delta: Res<LastVoxelDeltaState>,
    mut q: Query<
        (
            &mut GameplayHudState,
            &mut EcoSnapshot,
            &mut VoxelDelta,
            &mut VoxelChunkSnapshot,
        ),
        With<PlayerPos>,
    >,
) {
    let hud = build_gameplay_hud_state(&clock, &player, &pool, &nations, &monsters, &obs);
    let eco_snapshot = eco.to_snapshot(clock.tick);
    let chunk_snapshot = build_voxel_chunk_snapshot(&world, revision.0, player.block_pos);
    for (mut hud_state, mut eco_state, mut delta, mut chunk) in q.iter_mut() {
        *hud_state = hud.clone();
        *eco_state = eco_snapshot.clone();
        *chunk = chunk_snapshot.clone();
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

    let port: u16 =
        std::env::var("LK2_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_PORT);
    if !server_ports_available(port) {
        error!(
            "[server] UDP port {} or {} is already in use. Stop the existing lk2-server process or set LK2_PORT to a free port.",
            port,
            gameplay_port_for(port)
        );
        return;
    }
    info!("[server] listening on UDP 0.0.0.0:{}", port);

    let args: Vec<String> = std::env::args().collect();
    let auto_demo_mode = args.iter().any(|a| a == "--auto-demo");
    let scenario = if auto_demo_mode {
        Scenario {
            name: "idle".into(),
            record_window: None,
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
    let scenario_state = ScenarioState::from_scenario(scenario.clone());

    let _ = std::fs::create_dir_all("screenshots");
    let gameplay_udp = {
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], gameplay_port_for(port)));
        std::net::UdpSocket::bind(addr).ok().and_then(|socket| {
            socket.set_nonblocking(true).ok()?;
            Some(GameplayCommandUdp { socket })
        })
    };

    let mut app = App::new();
    if let Some(gameplay_udp) = gameplay_udp {
        app.insert_resource(gameplay_udp);
    }
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(lightyear::prelude::server::ServerPlugins::default())
        .add_plugins(lk2_core::protocol::ProtocolPlugin)
        .add_plugins(lk2_core::match_state::MatchStatePlugin)
        .add_plugins(lk2_core::protection::ProtectionPlugin)
        .add_plugins(lk2_core::sovereign_spark::SovereignSparkPlugin)
        .add_plugins(lk2_core::mining_site::MiningSitePlugin)
        .add_plugins(lk2_core::terrain_overlay::TerrainOverlayPlugin)
        .add_plugins(lk2_core::equipment::EquipmentPlugin)
        .add_plugins(lk2_core::combat::CombatPlugin)
        .add_plugins(ServerPvPPlugin)
        .add_message::<lk2_core::protocol::messages::AttackInput>()
        .add_message::<lk2_core::protocol::messages::GameplayCommand>()
        .add_message::<lk2_core::protocol::messages::GameplayFeedback>()
        .add_message::<PingMessage>()
        .add_message::<PongMessage>()
        .add_message::<lk2_core::protocol::messages::HitConfirm>()
        .add_message::<lk2_core::protocol::messages::KnockbackEvent>()
        .add_message::<lk2_core::protocol::messages::DamageResult>()
        .add_message::<lk2_core::pvp::DamageEvent>()
        .add_message::<lk2_core::pvp::VisualEffectEvent>()
        .init_resource::<PeerMetadata>()
        .init_resource::<SimClock>()
        .init_resource::<TimeOfDay>()
        .init_resource::<GameWorld>()
        .init_resource::<GlobalResourcePool>()
        .init_resource::<NationRegistry>()
        .init_resource::<MonsterEcosystem>()
        .init_resource::<EcoCycle>()
        .init_resource::<TickObserver>()
        .init_resource::<TickRecorder>()
        .init_resource::<CreatureSpawnerDone>()
        .init_resource::<PlayerState>()
        .init_resource::<FixedTick>()
        .init_resource::<ServerTickCounter>()
        .init_resource::<WorldRevision>()
        .init_resource::<LastVoxelDeltaState>()
        .init_resource::<ServerCommandDiagnostics>()
        .init_resource::<ServerJumpState>()
        .init_resource::<AntiStuckState>()
        .insert_resource(scenario_state)
        .add_systems(
            Startup,
            (
                setup_world,
                dump_world_resources,
                spawn_server,
                spawn_player,
                // self_check was here previously; it ran 100 sim ticks inside
                // Startup, blocking the App::run loop and starving UDP packet
                // processing for ~9s — which exactly matched the default
                // client_timeout_secs and caused connection_request timeouts.
                // Moved to first Update tick instead (see below).
            )
                .chain(),
        )
        .add_observer(replicate_player_for_connected)
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
            FixedUpdate,
            (
                simulation_tick.in_set(SimSet::Interaction),
                // iter_199 Phase A: nation 自动 upkeep — 每个 nation 周期消耗
                // Wood/Food 维持 flag，缺资源时扣 flag_hp 并可能 dissolve。
                // 放在 simulation_tick 之后才能看到 sim tick 加的 Apple/Food。
                lk2_core::nation::tick_nations_upkeep_system
                    .after(simulation_tick)
                    .in_set(SimSet::Interaction),
                end_tick_system.in_set(SimSet::ScoreAndAudit),
                tick_recorder.in_set(SimSet::Snapshot),
            ),
        )
        .add_systems(
            FixedUpdate,
            (
                apply_gameplay_commands,
                apply_udp_gameplay_commands,
                echo_ping_messages,
                broadcast_gameplay_feedback,
                sync_authoritative_snapshot_components,
                broadcast_player_pos,
                read_attack_inputs,
                melee_hit_registration,
                apply_damage_and_knockback,
                expire_knockback_immunity,
                tick_combat_cooldowns,
            )
                .chain(),
        )
        .run();
}

fn server_ports_available(port: u16) -> bool {
    let main = std::net::UdpSocket::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)));
    let gameplay = std::net::UdpSocket::bind(std::net::SocketAddr::from((
        [0, 0, 0, 0],
        gameplay_port_for(port),
    )));
    main.is_ok() && gameplay.is_ok()
}

fn dump_world_resources(world: &bevy::prelude::World) {
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

fn spawn_player(
    mut commands: Commands,
    mut player: ResMut<PlayerState>,
    mut anti_stuck: ResMut<AntiStuckState>,
    game_world: Res<GameWorld>,
) {
    let sx = constant::WORLD_SIZE / 2;
    let sz = constant::WORLD_SIZE / 2;
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
            let fallback = bevy::math::Vec3::new(
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
    player.pos = spawn;
    player.block_pos = spawn_block;
    anti_stuck.last_safe_pos = spawn;
    anti_stuck.last_safe_block = spawn_block;
    anti_stuck.failed_move_ticks = 0;
    info!(
        "[player] spawning authoritative player entity at {:?}, block={:?}",
        spawn, spawn_block
    );
    commands.spawn((
        Name::new("Player"),
        PlayerTag(0),
        bevy::prelude::Transform::from_translation(spawn),
        lk2_core::protocol::components::PlayerPos(spawn),
        ActionState::<PlayerAction>::default(),
        empty_gameplay_hud_state(),
        EcoCycle::default().to_snapshot(0),
        empty_voxel_delta(),
        empty_voxel_chunk_snapshot(),
    ));
}

fn replicate_player_for_connected(
    trigger: On<Add, lightyear_connection::client_of::ClientOf>,
    mut commands: Commands,
    player_q: Query<Entity, With<PlayerPos>>,
) {
    let client_of_entity = trigger.entity;
    info!(
        "[net] ClientOf entity {:?} connected, attaching ReplicationSender to it + Replicate to local Player entity",
        client_of_entity
    );

    commands.entity(client_of_entity).insert((
        lightyear::prelude::ReplicationSender::default(),
        lightyear::prelude::MessageReceiver::<PingMessage>::default(),
    ));
    info!(
        "[net] ReplicationSender attached to ClientOf entity {:?} — now this client can receive replicated entities",
        client_of_entity
    );

    if let Some(entity) = player_q.iter().next() {
        commands.entity(entity).insert((
            lightyear::prelude::Replicate::manual(vec![client_of_entity]),
            lightyear::prelude::ControlledBy {
                owner: client_of_entity,
                lifetime: lightyear::prelude::Lifetime::Persistent,
            },
        ));
        info!(
            "[net] Replicate + ControlledBy attached to Player entity {:?} — owner={:?}, senders=[{:?}]",
            entity, client_of_entity, client_of_entity
        );
    }
}

#[derive(bevy::prelude::Resource, Default)]
pub struct ServerTickCounter(pub u32);

fn broadcast_player_pos(
    mut tick: ResMut<ServerTickCounter>,
    q: Query<&bevy::prelude::Transform, With<lk2_core::protocol::components::PlayerPos>>,
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

        let _ = sender.send::<_, lightyear_replication::metadata::MetadataChannel>(
            &msg,
            server,
            &lightyear::prelude::NetworkTarget::All,
        );
    }
}

fn setup_world(
    _commands: Commands,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut eco: ResMut<EcoCycle>,
) {
    *game_world = generate_world(&WorldConfig::default());
    info!("[terrain] using preset '{}'", game_world.pipeline.name);

    use lk2_core::resource::ResourceKind;
    for k in ResourceKind::ALL {
        let init = k.demo_initial_amount();
        let _ = pool.force_add(*k, init);
    }

    monsters.demo_init([
        constant::WORLD_SIZE / 2,
        constant::SEA_LEVEL + 1,
        constant::WORLD_SIZE / 2,
    ]);
    *eco = EcoCycle::demo_at(Vec2::new(
        constant::WORLD_SIZE as f32 * 0.5,
        constant::WORLD_SIZE as f32 * 0.5,
    ));

    info!("🌍 世界已生成: {}³ (server)", constant::WORLD_SIZE);
}

fn self_check(
    game_world: Res<GameWorld>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    eco: Res<EcoCycle>,
    mut obs: ResMut<TickObserver>,
) {
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
    eco: Res<EcoCycle>,
    mut obs: ResMut<TickObserver>,
) {
    if *fired {
        return;
    }
    *fired = true;
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
    std::env::var("LK2_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(5000)
}

fn simulation_tick(
    fixed_time: Res<Time<Fixed>>,
    mut clock: ResMut<SimClock>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut eco: ResMut<EcoCycle>,
    mut obs: ResMut<TickObserver>,
) {
    let _ = advance_fixed_authority_tick(
        fixed_time.delta_secs(),
        &mut clock,
        &mut pool,
        &mut monsters,
        &mut eco,
        &mut obs,
        SimRole::ServerAuthority,
    );
}

fn end_tick_system(
    game_world: Res<GameWorld>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    clock: Res<SimClock>,
    mut obs: ResMut<TickObserver>,
    player: Res<lk2_core::player::PlayerState>,
) {
    if !clock.last_sim_step_ran {
        return;
    }

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

fn tick_recorder(
    time: Res<Time>,
    mut rec: ResMut<TickRecorder>,
    clock: Res<SimClock>,
    player: Res<lk2_core::player::PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    eco: Res<EcoCycle>,
    obs: Res<TickObserver>,
    game_world: Res<GameWorld>,
    diagnostics: Res<ServerCommandDiagnostics>,
    server_q: Query<Entity, With<lightyear::prelude::Server>>,
    started_q: Query<Entity, With<lightyear_connection::server::Started>>,
    link_of_q: Query<Entity, With<lightyear::prelude::server::LinkOf>>,
) {
    if std::env::var("LK2_CAPTURE").is_err() {
        return;
    }
    if clock.tick == 0 || clock.tick % 5 != 0 || clock.tick == rec.last_dump_tick {
        return;
    }
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
            "network_command".to_string(),
            serde_json::json!({
                "receiver_entities": diagnostics.receiver_entities,
                "server_entities": server_q.iter().count(),
                "started_servers": started_q.iter().count(),
                "link_of_entities": link_of_q.iter().count(),
                "move_world_received": diagnostics.move_world_received,
                "udp_commands_received": diagnostics.udp_commands_received,
                "last_dx_milli": diagnostics.last_dx_milli,
                "last_dz_milli": diagnostics.last_dz_milli,
                "last_move_applied": diagnostics.last_move_applied,
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
    fn block_type_wire_codes_keep_existing_values_and_append_grass() {
        assert_eq!(block_type_to_u8(BlockType::Air), 0);
        assert_eq!(block_type_to_u8(BlockType::Dirt), 1);
        assert_eq!(block_type_to_u8(BlockType::Stone), 2);
        assert_eq!(block_type_to_u8(BlockType::BerryThicket), 12);
        assert_eq!(block_type_to_u8(BlockType::Grass), 13);
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
}
