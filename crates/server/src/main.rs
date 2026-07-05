#![allow(dead_code)]
#![allow(unused_imports)]

use avian3d::prelude::PhysicsPlugins;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;
use lightyear::prelude::server::ServerUdpIo;
use lightyear::prelude::LocalAddr;

use leafwing_input_manager::prelude::ActionState;
use lk2_core::protocol::components::{GameplayHudState, PlayerPos, VoxelDelta};
use lk2_core::protocol::messages::{
    BuildRecipe, GameplayCommand, GameplayCommandKind, GameplayFeedback,
};
use lk2_core::protocol::PlayerAction;

use lightyear::prelude::PeerMetadata;

use std::time::Duration;

mod los;
mod pvp_systems;

use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::constant;
use lk2_core::creature::{
    award_creature_drop, creature_attack_distance_sq, Creature, CreatureAI, CreatureKind,
    CreatureSpawnerDone, CREATURE_TRAINING_ATTACK_RANGE_SQ,
};
use lk2_core::eco_cycle::EcoCycle;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::{PlayerState, PlayerTag};
use lk2_core::pvp::FixedTick;
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::scenario::{Scenario, ScenarioState};
use lk2_core::sim::{advance_fixed_authority_tick, SimRole};
use lk2_core::v2::app_sets::SimSet;
use lk2_core::world::{
    install_huge_spawn_platform, player_spawn_position_near, World as GameWorld, WorldGenerator,
};

use crate::pvp_systems::{
    apply_damage_and_knockback, expire_knockback_immunity, melee_hit_registration,
    read_attack_inputs, record_position_history, tick_combat_cooldowns, ServerPvPPlugin,
};

const PLACE_WOOD_COST: i64 = 1;

#[derive(Resource, Default)]
struct WorldRevision(u64);

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
    mut reader: MessageReader<GameplayCommand>,
    mut world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut nations: ResMut<NationRegistry>,
    mut player: ResMut<PlayerState>,
    mut revision: ResMut<WorldRevision>,
    mut last_delta: ResMut<LastVoxelDeltaState>,
    mut feedback: MessageWriter<GameplayFeedback>,
    mut commands: Commands,
    creatures: Query<(Entity, &Creature, &CreatureAI)>,
    mut player_q: Query<
        (
            &mut bevy::prelude::Transform,
            &mut lk2_core::protocol::components::PlayerPos,
        ),
        With<lightyear::prelude::ControlledBy>,
    >,
) {
    for cmd in reader.read() {
        if let GameplayCommandKind::MoveWorld { dx_milli, dz_milli } = cmd.kind {
            let dir = Vec2::new(dx_milli as f32 / 1000.0, dz_milli as f32 / 1000.0);
            let moved = if let Ok((mut transform, mut player_pos)) = player_q.single_mut() {
                apply_world_move(&mut transform, &mut player_pos, &mut player, dir, 1.0 / 60.0)
            } else {
                false
            };
            if moved {
                feedback.write(GameplayFeedback { ok: true, summary: "moved".to_string() });
            }
            continue;
        }
        let nearest_creature = if matches!(cmd.kind, GameplayCommandKind::KillNearestCreature) {
            creatures
                .iter()
                .filter_map(|(entity, creature, _)| {
                    let d2 = creature_attack_distance_sq(player.block_pos, creature.block_pos);
                    (d2 <= CREATURE_TRAINING_ATTACK_RANGE_SQ).then_some((entity, d2, creature.kind))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
        } else {
            None
        };
        let nearest_kind = nearest_creature.map(|(_, _, kind)| kind);
        let result = apply_gameplay_command(
            cmd,
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

fn apply_world_move(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    dir: Vec2,
    dt: f32,
) -> bool {
    if dir.length_squared() <= 0.0001 {
        return false;
    }
    let dir = dir.normalize_or_zero();
    let speed = 4.0;
    let delta = Vec3::new(dir.x, 0.0, dir.y) * speed * dt;
    transform.translation += delta;
    player_pos.0 = transform.translation;
    player.pos = transform.translation;
    player.block_pos = [
        transform.translation.x.floor() as i32,
        transform.translation.y.floor() as i32,
        transform.translation.z.floor() as i32,
    ];
    true
}

fn sync_authoritative_snapshot_components(
    clock: Res<SimClock>,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    obs: Res<TickObserver>,
    last_delta: Res<LastVoxelDeltaState>,
    mut q: Query<(&mut GameplayHudState, &mut VoxelDelta), With<PlayerPos>>,
) {
    let hud = build_gameplay_hud_state(&clock, &player, &pool, &nations, &monsters, &obs);
    for (mut hud_state, mut delta) in q.iter_mut() {
        *hud_state = hud.clone();
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

    let port: u16 = std::env::var("LK2_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(5000);
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

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(PhysicsPlugins::default())
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
        .insert_resource(scenario_state)
        .add_systems(
            Startup,
            (
                setup_world,
                self_check,
                dump_world_resources,
                spawn_server,
                spawn_player,
            )
                .chain(),
        )
        .add_observer(replicate_player_for_connected)
        .configure_sets(
            FixedUpdate,
            (SimSet::Interaction, SimSet::ScoreAndAudit, SimSet::Snapshot).chain(),
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
                apply_input_to_player,
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

    let private_key: lightyear_netcode::Key = [0xAA; lightyear_netcode::PRIVATE_KEY_BYTES];
    let protocol_id: u64 = 0x4C4B3256_4E455457;
    let netcode_server = NetcodeServer::new(
        NetcodeConfig::default().with_protocol_id(protocol_id).with_key(private_key),
    );
    info!(
        "[net] NetcodeServer initialized: protocol_id=0x{:x}, key=<fixed-dev>",
        protocol_id
    );

    let server_id = commands
        .spawn((
            Name::new("Server"),
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
    game_world: Res<GameWorld>,
) {
    let sx = constant::WORLD_SIZE / 2;
    let sz = constant::WORLD_SIZE / 2;
    let (spawn, spawn_block) = player_spawn_position_near(&game_world, sx, sz, 14, 2)
        .unwrap_or_else(|| {
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
        empty_voxel_delta(),
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

    commands.entity(client_of_entity).insert(lightyear::prelude::ReplicationSender::default());
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

fn apply_input_to_player(
    mut q: Query<
        (
            &ActionState<PlayerAction>,
            &mut bevy::prelude::Transform,
            &mut lk2_core::protocol::components::PlayerPos,
        ),
        (With<Name>, With<lightyear::prelude::ControlledBy>),
    >,
    mut player: ResMut<PlayerState>,
) {
    let mut dir = bevy::math::Vec2::ZERO;
    let speed = 4.0;
    let dt = 1.0 / 30.0;
    let mut applied_count = 0;
    for (actions, mut transform, mut player_pos) in q.iter_mut() {
        let mut local_dir = bevy::math::Vec2::ZERO;
        if actions.pressed(&PlayerAction::MoveForward) {
            local_dir.y += 1.0;
        }
        if actions.pressed(&PlayerAction::MoveBackward) {
            local_dir.y -= 1.0;
        }
        if actions.pressed(&PlayerAction::MoveLeft) {
            local_dir.x -= 1.0;
        }
        if actions.pressed(&PlayerAction::MoveRight) {
            local_dir.x += 1.0;
        }
        if local_dir.length() > 1.0 {
            local_dir = local_dir.normalize();
        }
        if apply_world_move(&mut transform, &mut player_pos, &mut player, Vec2::new(local_dir.x, -local_dir.y), dt) {
            dir = local_dir;
            applied_count += 1;
        }
    }
    if applied_count > 0 {
        info!(
            "[player] apply_input_to_player: applied_count={}, dir={:?}, speed={}",
            applied_count, dir, speed
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

    if tick.0 % 2 != 0 {
        return;
    }
    let Ok(server) = server_q.single() else {
        return;
    };

    for transform in q.iter() {
        let msg = lk2_core::protocol::messages::ServerPosUpdate {
            server_tick: tick.0,
            pos: transform.translation,
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
    let pipeline = lk2_core::world::terrain::presets::by_name("default");
    *game_world = GameWorld::with_pipeline(constant::WORLD_SIZE, pipeline);
    install_huge_spawn_platform(&mut game_world);
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
) {
    if clock.tick == 0 || clock.tick % 5 != 0 || clock.tick == rec.last_dump_tick {
        return;
    }
    rec.last_dump_tick = clock.tick;
    rec.current_iter = clock.tick as u32;
    let path = format!("screenshots/server_state_t{}.json", clock.tick);
    let state = lk2_core::diagnostics::build_state_json(
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
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, s);
    }
}
