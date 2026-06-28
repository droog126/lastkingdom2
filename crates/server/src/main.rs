

























#![allow(dead_code)]
#![allow(unused_imports)]

use avian3d::prelude::PhysicsPlugins;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;
use lightyear::prelude::LocalAddr;
use lightyear::prelude::server::ServerUdpIo;



use leafwing_input_manager::prelude::ActionState;
use lk2_core::protocol::PlayerAction;
use lk2_core::protocol::components::{GameplayHudState, PlayerPos, VoxelDelta};
use lk2_core::protocol::messages::{
    BuildRecipe, GameplayCommand, GameplayCommandKind, GameplayFeedback,
};













use lightyear::prelude::PeerMetadata;



use lightyear::prelude::LinkStart;




use bevy::scene::SceneSpawner;

use std::time::Duration;


mod los;
mod pvp_systems;


use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::constant;
use lk2_core::creature::{CreatureSpawnerDone, update_creatures};
use lk2_core::eco_cycle::EcoCycle;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::pvp::FixedTick;
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::scenario::{Scenario, ScenarioState};
use lk2_core::sim::{SimRole, advance_fixed_authority_tick};
use lk2_core::v2::app_sets::SimSet;
use lk2_core::world::{World as GameWorld, WorldGenerator};


use crate::pvp_systems::{
    ServerPvPPlugin, apply_damage_and_knockback, expire_knockback_immunity, melee_hit_registration,
    read_attack_inputs, record_position_history, tick_combat_cooldowns,
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

        .add_plugins(PhysicsPlugins::default())






        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<bevy::prelude::Mesh>()
        .add_message::<bevy::asset::AssetEvent<bevy::prelude::Mesh>>()







        .init_resource::<bevy::scene::SceneSpawner>()







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
        .add_message::<lk2_core::protocol::messages::HitConfirm>()
        .add_message::<lk2_core::protocol::messages::KnockbackEvent>()
        .add_message::<lk2_core::protocol::messages::DamageResult>()
        .add_message::<lk2_core::pvp::DamageEvent>()
        .add_message::<lk2_core::pvp::VisualEffectEvent>()


        .init_resource::<PeerMetadata>()


        .init_resource::<SceneSpawner>()
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

        .add_systems(Update, update_creatures)

        .configure_sets(
            FixedUpdate,
            (SimSet::Interaction, SimSet::ScoreAndAudit, SimSet::Snapshot).chain(),
        )
        .add_systems(
            FixedUpdate,
            (
                simulation_tick.in_set(SimSet::Interaction),
                end_tick_system.in_set(SimSet::ScoreAndAudit),
                tick_recorder.in_set(SimSet::Snapshot),
            ),
        )

        .add_systems(
            FixedUpdate,
            (

                apply_input_to_player,
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
    let has_scene_spawner = world.get_resource::<SceneSpawner>().is_some();
    info!("[debug] SceneSpawner in world? {}", has_scene_spawner);
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





    info!(
        "[net] triggering LinkStart on server entity {:?}",
        server_id
    );
    commands.trigger(LinkStart { entity: server_id });




    info!("[net] triggering Start on server entity {:?}", server_id);
    commands.trigger(Start { entity: server_id });















    info!(
        "[net] manually inserting Started marker to server entity {:?}",
        server_id
    );
    commands.entity(server_id).insert(lightyear_connection::server::Started);
}



















fn spawn_player(mut commands: Commands) {
    let spawn = bevy::math::Vec3::new(
        constant::WORLD_SIZE as f32 / 2.0 + 0.5,
        (constant::SEA_LEVEL + 2) as f32 + 0.5,
        constant::WORLD_SIZE as f32 / 2.0 + 0.5,
    );
    info!(
        "[player] spawning authoritative player entity at {:?}",
        spawn
    );
    commands.spawn((
        Name::new("Player"),
        bevy::prelude::Transform::from_translation(spawn),
        lk2_core::protocol::components::PlayerPos(spawn),
    ));
}




















fn replicate_player_for_connected(
    trigger: On<Add, lightyear_connection::client_of::ClientOf>,
    mut commands: Commands,
    player_q: Query<Entity, With<Name>>,
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
        let delta = bevy::math::Vec3::new(local_dir.x, 0.0, -local_dir.y) * speed * dt;
        transform.translation += delta;

        player_pos.0 = transform.translation;
        if local_dir.length() > 0.01 {
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


        let _ = sender.send::<_, lightyear::prelude::MetadataChannel>(
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
