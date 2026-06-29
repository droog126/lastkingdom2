

























#![allow(dead_code)]
#![allow(unused_imports)]

use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use std::path::PathBuf;

use avian3d::prelude::{Collider, Gravity, LinearVelocity, PhysicsPlugins, RigidBody};


mod controller_systems;
mod pretty;
mod pvp_systems;
mod render;
mod ui;


use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::constant;
use lk2_core::creature::{
    CreatureSpawnerDone, despawn_dead_creatures,
    player_attack_creatures as offline_player_attack_creatures, spawn_creatures, update_creatures,
};
use lk2_core::eco_cycle::EcoCycle;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::pvp::{FixedTick, PositionHistory};
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::scenario::{Scenario, ScenarioState};
use lk2_core::sim::{SimRole, advance_fixed_authority_tick};
use lk2_core::world::{World as GameWorld, WorldGenerator};


use crate::controller_systems::ControllerPlugin;
use crate::pretty::{
    PrettyConfig, animate_avatar, animate_monsters, follow_ground_discs, follow_monster_cubes,
    follow_player_avatar, spawn_pretty,
};
use crate::pvp_systems::{
    HealthHudMarker, client_attack_predict, collect_combat_input_offline, collect_local_input,
    offline_found_nation_input, on_damage_result, on_hit_confirm, on_knockback_event,
    trigger_visual_effects,
};
use crate::render::{
    AnimalIndicatorText, CameraAngles, CameraMode, FreeFlyState, LastMoveDirection,
    NestIndicatorText, NestMarkerCount, Player, RenderConfig, SpawnedBlocks, SwordSwing, auto_demo,
    camera_mode_toggle, cycle_terrain_preset, emergency_teleport, first_person_camera,
    freefly_movement, freefly_toggle, held_weapon_follow, mouse_look_system, player_input,
    player_spawn_position_at, setup_atmosphere, setup_cursor_grab, setup_terrain_underlay,
    spawn_nest_markers, spawn_terrain_around_player, toggle_cursor_grab_on_esc,
    underlay_follow_player, update_animal_indicator, update_nest_indicator,
    update_nest_marker_positions,
};
use crate::ui::{ClientRunMode, setup_fonts, setup_hud, update_hud, update_tutorial_overlay};


const AUTO_DEMO_WAIT_TICKS: u64 = 1_800;
const FIRST_SCREENSHOT_MIN_TICK: u64 = 500;

use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::Controlled;
use lk2_core::protocol::PlayerAction;
use lk2_core::protocol::components::{GameplayHudState, Health, VoxelDelta};
use lk2_core::protocol::messages::{BuildRecipe, GameplayCommand, GameplayCommandKind};
use lk2_core::pvp::{CombatState, Hitbox, WeaponStats};

#[derive(Resource, Default, Debug, Clone)]
struct ReplicatedSnapshot {
    has_data: bool,
    tick: u64,
    player_block_pos: [i32; 3],
    player_pos: [f32; 3],
    nation_id: Option<u32>,
    monsters_killed: u32,
    blocks_gathered: u32,
    nations_founded: u32,
    inventory_wood: i64,
    inventory_food: i64,
    inventory_apple: i64,
    inventory_soul: i64,
    pool_wood: i64,
    pool_food: i64,
    pool_apple: i64,
    pool_soul: i64,
    flag_count: u32,
    total_nations: u32,
    monster_count: u32,
    observer_anomalies: u64,
    observer_invariant_violations: u64,
    status_line: String,
    last_voxel_revision: u64,
}

#[derive(Resource, Debug, Clone)]
struct NetworkSmoothingState {
    initialized: bool,
    target_pos: Vec3,
    visual_pos: Vec3,
    target_block_pos: [i32; 3],
}

impl Default for NetworkSmoothingState {
    fn default() -> Self {
        Self {
            initialized: false,
            target_pos: Vec3::ZERO,
            visual_pos: Vec3::ZERO,
            target_block_pos: [0, 0, 0],
        }
    }
}

const ONLINE_INTERP_SPEED: f32 = 14.0;
const ONLINE_SNAP_DISTANCE: f32 = 8.0;






static PRESET_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
fn preset_name_static() -> &'static str {
    PRESET_NAME.get().map(|s| s.as_str()).unwrap_or("default")
}


static WALK_OVERRIDE: std::sync::OnceLock<Option<(i32, i32)>> = std::sync::OnceLock::new();
fn walk_override_static() -> Option<(i32, i32)> {
    WALK_OVERRIDE.get().copied().flatten()
}





fn main() {

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let offline_mode = args.iter().any(|a| a == "--offline");
    let auto_demo_mode = args.iter().any(|a| a == "--auto-demo");
    let first_person_mode = args.iter().any(|a| a == "--first-person");



    let connect_addr = lk2_core::transport::parse_connect_arg(&args);
    let network_mode = connect_addr.is_some() && !offline_mode;
    if network_mode {
        println!(
            "[lk2-client] network mode: connect to {}",
            connect_addr.unwrap()
        );
    } else {
        println!(
            "[lk2-client] starting (offline={}, auto_demo={})",
            offline_mode, auto_demo_mode
        );
    }


    let preset_name = args
        .iter()
        .find(|a| a.starts_with("--preset="))
        .map(|a| a.trim_start_matches("--preset=").to_string())
        .unwrap_or_else(|| "default".to_string());
    let _ = PRESET_NAME.set(preset_name.clone());
    println!("[terrain] preset = {}", preset_name);


    let smooth_terrain = !args.iter().any(|a| a == "--legacy-voxel");
    println!("[render] smooth_terrain = {}", smooth_terrain);


    let walk_pos = args
        .iter()
        .find(|a| a.starts_with("--walk="))
        .map(|a| a.trim_start_matches("--walk=").to_string());
    if let Some(s) = walk_pos {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() == 2 {
            if let (Ok(x), Ok(z)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                let _ = WALK_OVERRIDE.set(Some((x, z)));
                println!("[terrain] --walk override: spawn at ({}, ?, {})", x, z);
            }
        }
    }


    let scenario = if auto_demo_mode {
        Scenario {
            name: "idle".into(),
            record_window: None,
            steps: vec![
                lk2_core::scenario::ScenarioStep::Log {
                    msg: "=== idle: 玩家不动看动物 ===".into(),
                },
                lk2_core::scenario::ScenarioStep::WaitTicks { ticks: AUTO_DEMO_WAIT_TICKS },
            ],
        }
    } else {
        lk2_core::scenario::load_scenario_from_args_or_default(&args)
    };
    let scenario_state = ScenarioState::from_scenario(scenario.clone());

    let _ = std::fs::create_dir_all("screenshots");

    let mut app = App::new();


    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin { file_path: "../../assets".into(), ..default() })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("万国起源：最后一国 钻石版 — {}", scenario.name).into(),
                    resolution: WindowResolution::new(1280, 720),

                    present_mode: PresentMode::Immediate,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::log::LogPlugin { level: bevy::log::Level::INFO, ..default() }),
    );


    app.insert_resource(bevy::winit::WinitSettings::continuous());


    app.add_plugins(PhysicsPlugins::default()).insert_resource(Gravity::default());


    app.add_plugins(lightyear::prelude::client::ClientPlugins::default());
    app.add_plugins(lk2_core::protocol::ProtocolPlugin);



    app.init_resource::<lightyear::prelude::PeerMetadata>()
        .init_resource::<lk2_core::pvp::FixedTick>()
        .init_resource::<TimeOfDay>();


    app.add_message::<GameplayCommand>();

    if network_mode {
        let server_addr = connect_addr.expect("network_mode=true implies connect_addr is Some");
        app.add_systems(Startup, move |commands: Commands| {


            let client_id_seed: u64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64 & 0xFFFF_FFFF_FFFF_FFFF)
                .unwrap_or(0xC11E_71);
            spawn_networked_client(commands, server_addr, client_id_seed);
        });
    }


    app.init_resource::<RenderConfig>()
        .init_resource::<CameraAngles>()
        .init_resource::<SwordSwing>()
        .insert_resource(CameraMode::default())

        .add_systems(
            Startup,
            move |mut mode: ResMut<CameraMode>, mut cfg: ResMut<RenderConfig>| {
                if first_person_mode {
                    *mode = CameraMode::FirstPerson;
                    cfg.auto_orbit = false;
                    cfg.auto_walk = false;
                    cfg.auto_keys = false;
                    cfg.mouse_look = true;
                    tracing::info!("📷 --first-person: CameraMode=FirstPerson, mouse_look=true");
                }
            },
        )
        .add_systems(Startup, move |mut cfg: ResMut<RenderConfig>| {
            if auto_demo_mode {

                cfg.auto_walk = true;
                cfg.auto_orbit = true;
                cfg.auto_keys = true;
                cfg.mouse_look = false;
            }
            cfg.smooth_terrain = smooth_terrain;
        })
        .init_resource::<SpawnedBlocks>()
        .init_resource::<PrettyConfig>()
        .init_resource::<PlayerState>()
        .init_resource::<SimClock>()
        .init_resource::<GameWorld>()
        .init_resource::<GlobalResourcePool>()
        .init_resource::<NationRegistry>()
        .init_resource::<MonsterEcosystem>()
        .init_resource::<EcoCycle>()
        .init_resource::<TickObserver>()
        .init_resource::<TickRecorder>()
        .init_resource::<LastMoveDirection>()
        .init_resource::<FreeFlyState>()


        .init_resource::<CreatureSpawnerDone>()
        .init_resource::<FixedTick>()
        .init_resource::<ReplicatedSnapshot>()
        .init_resource::<NetworkSmoothingState>()
        .init_resource::<NestMarkerCount>()
        .insert_resource(if network_mode {
            ClientRunMode::Online
        } else {
            ClientRunMode::Offline
        })
        .insert_resource(scenario_state);



    app.add_plugins(lk2_core::match_state::MatchStatePlugin)
        .add_plugins(lk2_core::protection::ProtectionPlugin)
        .add_plugins(lk2_core::sovereign_spark::SovereignSparkPlugin)
        .add_plugins(lk2_core::mining_site::MiningSitePlugin)
        .add_plugins(lk2_core::terrain_overlay::TerrainOverlayPlugin)
        .add_plugins(lk2_core::equipment::EquipmentPlugin)
        .add_plugins(lk2_core::combat::CombatPlugin)
        .add_plugins(lk2_core::objectives::ObjectivesPlugin)
        .add_plugins(ClientPvPPlugin)
        .add_plugins(ControllerPlugin);


    app.add_systems(
        Startup,
        (
            setup_fonts,
            setup_camera,
            setup_light,
            setup_atmosphere,
            setup_cursor_grab,
            setup_world,
            spawn_nest_markers,
            spawn_pretty,
            spawn_creatures,
            setup_hud,
            self_check,
            setup_player_pvp,
            lk2_core::objectives::setup_default_objectives,
        )
            .chain(),
    );








    app.add_systems(
        Update,
        (

            lk2_core::scenario::scenario_runner,
            lk2_core::scenario::simulate_player_actions,
            lk2_core::scenario::scenario_tick_recorder,


            apply_networked_position,


            apply_server_pos_update,
            debug_dump_replicated_entities,
            apply_authoritative_snapshot,
            apply_voxel_delta,
            send_online_gameplay_commands,



            collect_keys_to_action_state,

            auto_demo,
            mouse_look_system,
            first_person_camera,
            held_weapon_follow,
            player_input,
            sync_player_combat_anchor,
            offline_player_attack_creatures,
            animate_avatar,
            spawn_terrain_around_player,
            toggle_cursor_grab_on_esc,
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (

            freefly_toggle,
            camera_mode_toggle,
            emergency_teleport,
            cycle_terrain_preset,
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (

            collect_local_input,
            client_attack_predict,
            on_hit_confirm,
            on_knockback_event,
            on_damage_result,
            trigger_visual_effects,



        )
            .chain(),
    );

    app.add_systems(Update, freefly_movement.before(first_person_camera));

    app.add_systems(Update, update_nest_marker_positions);


    app.add_systems(
        Update,
        (
            follow_player_avatar,
            follow_ground_discs,
            follow_monster_cubes,
            animate_monsters,
        )
            .chain(),
    );

    app.add_systems(Update, collect_combat_input_offline);

    app.add_systems(Update, offline_found_nation_input);






    app.add_systems(
        Update,
        (
            simulation_tick,
            end_tick_system,
            update_hud,
            update_tutorial_overlay,
            update_animal_indicator,
            update_nest_indicator,
            tick_recorder,
            periodic_screenshot,
            despawn_dead_creatures,
            update_creatures,
            day_night_cycle,
            exit_on_esc,
        )
            .chain(),
    );


    app.run();
}


fn spawn_networked_client(
    mut commands: Commands,
    server_addr: std::net::SocketAddr,
    client_id_seed: u64,
) {
    use lightyear::prelude::MessageReceiver;
    use lightyear::prelude::UdpIo;
    use lightyear::prelude::client::Connect;
    use lightyear::prelude::{LinkStart, LocalAddr, PeerAddr};
    use lightyear_netcode::client_plugin::{NetcodeClient, NetcodeConfig};
    use lightyear_netcode::prelude::Authentication;
    use lk2_core::protocol::messages::ServerPosUpdate;

    info!(
        "[net] spawning client entity with UdpIo + LocalAddr(0.0.0.0:0) + PeerAddr({})",
        server_addr
    );









    let private_key: lightyear_netcode::Key = [0xAA; lightyear_netcode::PRIVATE_KEY_BYTES];
    let protocol_id: u64 = 0x4C4B3256_4E455457;
    let netcode_client = NetcodeClient::new(
        Authentication::Manual { server_addr, client_id: client_id_seed, private_key, protocol_id },
        NetcodeConfig::default(),
    )
    .expect("NetcodeClient::new(Manual) failed");
    info!(
        "[net] NetcodeClient initialized for server={}, client_id={}, protocol_id=0x{:x}",
        server_addr, client_id_seed, protocol_id
    );

    let client_id = commands
        .spawn((
            Name::new("Client"),
            UdpIo::default(),
            LocalAddr(std::net::SocketAddr::from(([0, 0, 0, 0], 0))),
            PeerAddr(server_addr),
            netcode_client,



            MessageReceiver::<ServerPosUpdate>::default(),
        ))
        .id();



    info!(
        "[net] triggering LinkStart on client entity {:?}",
        client_id
    );
    commands.trigger(LinkStart { entity: client_id });





    info!("[net] triggering Connect on client entity {:?}", client_id);
    commands.trigger(Connect { entity: client_id });
}




fn apply_networked_position(
    mut q: Query<(&mut Transform, &lk2_core::protocol::components::PlayerPos)>,
) {




    let mut count = 0;
    let mut first_pos = bevy::math::Vec3::ZERO;
    for (mut tf, pos) in q.iter_mut() {
        tf.translation = pos.0;
        first_pos = pos.0;
        count += 1;
    }
    if count > 0 {


        tracing::info!(
            "[net] applied PlayerPos to {} player entity, pos={:?}",
            count,
            first_pos
        );
    }
}






fn debug_dump_replicated_entities(
    run_mode: Res<ClientRunMode>,
    time: Res<Time>,
    mut last_dump: Local<f32>,
    names: Query<(Entity, Option<&Name>)>,
    replicate_count: Query<Entity, With<lightyear::prelude::Replicate>>,
    replicated_count: Query<Entity, With<lightyear::prelude::Replicated>>,
    clientof_count: Query<Entity, With<lightyear_connection::client_of::ClientOf>>,
    linked_count: Query<Entity, With<lightyear::prelude::Linked>>,
    playerpos_count: Query<Entity, With<lk2_core::protocol::components::PlayerPos>>,
) {
    if *run_mode != ClientRunMode::Online {
        return;
    }
    let now = time.elapsed_secs();
    if now - *last_dump < 5.0 {
        return;
    }
    *last_dump = now;
    let total = names.iter().count();
    let rep = replicate_count.iter().count();
    let red = replicated_count.iter().count();
    let co = clientof_count.iter().count();
    let ln = linked_count.iter().count();
    let pp = playerpos_count.iter().count();

    let mut sample: Vec<String> = Vec::new();
    for (e, name) in names.iter().take(8) {
        let n = name.map(|n| n.as_str()).unwrap_or("?");
        sample.push(format!("{:?}={}", e, n));
    }
    tracing::info!(
        "[net-debug] total_entities={}, replicate={}, replicated={}, clientof={}, linked={}, playerpos={}, sample=[{}]",
        total,
        rep,
        red,
        co,
        ln,
        pp,
        sample.join(", ")
    );
}


















fn apply_server_pos_update(
    run_mode: Res<ClientRunMode>,
    mut receiver_q: Query<
        &mut lightyear::prelude::MessageReceiver<lk2_core::protocol::messages::ServerPosUpdate>,
    >,
    mut player: ResMut<PlayerState>,
) {


    if true {

        for mut receiver in receiver_q.iter_mut() {
            for _msg in receiver.receive() {}
        }
        return;
    }
    if *run_mode != ClientRunMode::Online {
        return;
    }
    let mut any = false;
    let mut last_pos = bevy::math::Vec3::ZERO;
    let mut last_tick: u32 = 0;
    for mut receiver in receiver_q.iter_mut() {
        for msg in receiver.receive() {
            last_pos = msg.pos;
            last_tick = msg.server_tick;
            any = true;
        }
    }
    if any {
        player.pos = last_pos;
        tracing::info!(
            "[net] applied ServerPosUpdate tick={} pos={:?} → PlayerState.pos",
            last_tick,
            last_pos
        );
    }
}

fn apply_authoritative_snapshot(
    run_mode: Res<ClientRunMode>,
    hud_q: Query<&GameplayHudState>,
    mut snapshot: ResMut<ReplicatedSnapshot>,
    mut clock: ResMut<SimClock>,
    mut player: ResMut<PlayerState>,
    mut pool: ResMut<GlobalResourcePool>,
    mut nations: ResMut<NationRegistry>,
    mut monsters: ResMut<MonsterEcosystem>,
) {
    if *run_mode != ClientRunMode::Online {
        return;
    }
    let Some(hud) = hud_q.iter().next() else {
        return;
    };

    snapshot.has_data = true;
    snapshot.tick = hud.tick;
    snapshot.player_block_pos = hud.player_block_pos;
    snapshot.player_pos = hud.player_pos;
    snapshot.nation_id = hud.nation_id;
    snapshot.monsters_killed = hud.monsters_killed;
    snapshot.blocks_gathered = hud.blocks_gathered;
    snapshot.nations_founded = hud.nations_founded;
    snapshot.inventory_wood = hud.inventory_wood;
    snapshot.inventory_food = hud.inventory_food;
    snapshot.inventory_apple = hud.inventory_apple;
    snapshot.inventory_soul = hud.inventory_soul;
    snapshot.pool_wood = hud.pool_wood;
    snapshot.pool_food = hud.pool_food;
    snapshot.pool_apple = hud.pool_apple;
    snapshot.pool_soul = hud.pool_soul;
    snapshot.flag_count = hud.flag_count;
    snapshot.total_nations = hud.total_nations;
    snapshot.monster_count = hud.monster_count;
    snapshot.observer_anomalies = hud.observer_anomalies;
    snapshot.observer_invariant_violations = hud.observer_invariant_violations;
    snapshot.status_line = hud.status_line.clone();

    clock.tick = hud.tick;
    player.block_pos = hud.player_block_pos;
    player.pos = Vec3::new(hud.player_pos[0], hud.player_pos[1], hud.player_pos[2]);
    player.monsters_killed = hud.monsters_killed;
    player.blocks_gathered = hud.blocks_gathered;
    player.nations_founded = hud.nations_founded;
    player.nation_id = hud.nation_id.map(lk2_core::nation::NationId);
    player.inventory.insert(ResourceKind::Wood, hud.inventory_wood);
    player.inventory.insert(ResourceKind::Food, hud.inventory_food);
    player.inventory.insert(ResourceKind::Apple, hud.inventory_apple);
    player.inventory.insert(ResourceKind::Soul, hud.inventory_soul);

    pool.current.insert(ResourceKind::Wood, hud.pool_wood);
    pool.current.insert(ResourceKind::Food, hud.pool_food);
    pool.current.insert(ResourceKind::Apple, hud.pool_apple);
    pool.current.insert(ResourceKind::Soul, hud.pool_soul);
    nations.flag_count = hud.flag_count;
    monsters.current_individuals = hud.monster_count;
}

fn block_type_from_u8(value: u8) -> lk2_core::world::BlockType {
    use lk2_core::world::BlockType;
    match value {
        1 => BlockType::Dirt,
        2 => BlockType::Stone,
        3 => BlockType::Sand,
        4 => BlockType::Snow,
        5 => BlockType::Leaves,
        6 => BlockType::Water,
        7 => BlockType::Wood,
        8 => BlockType::IronOre,
        9 => BlockType::SunstoneOre,
        10 => BlockType::FrostcoreOre,
        11 => BlockType::LivingRoot,
        12 => BlockType::BerryThicket,
        _ => BlockType::Air,
    }
}

fn apply_voxel_delta(
    run_mode: Res<ClientRunMode>,
    voxel_q: Query<&VoxelDelta>,
    mut snapshot: ResMut<ReplicatedSnapshot>,
    mut game_world: ResMut<GameWorld>,
    mut spawned: ResMut<SpawnedBlocks>,
) {
    if *run_mode != ClientRunMode::Online {
        return;
    }
    for delta in voxel_q.iter() {
        if delta.revision <= snapshot.last_voxel_revision {
            continue;
        }
        game_world.set(delta.x, delta.y, delta.z, block_type_from_u8(delta.block));
        snapshot.last_voxel_revision = delta.revision;
        spawned.last_player_block = [i32::MIN; 3];
    }
}

fn send_online_gameplay_commands(
    run_mode: Res<ClientRunMode>,
    keys: Res<ButtonInput<KeyCode>>,
    player: Res<PlayerState>,
    clock: Res<SimClock>,
    writer: Option<MessageWriter<GameplayCommand>>,
) {
    if *run_mode != ClientRunMode::Online {
        return;
    }
    let Some(mut writer) = writer else {
        return;
    };

    let mut send = |kind: GameplayCommandKind| {
        writer.write(GameplayCommand { tick: clock.tick, player_block: player.block_pos, kind });
    };

    if keys.just_pressed(KeyCode::KeyG) {
        send(GameplayCommandKind::GatherFootBlock);
    }
    if keys.just_pressed(KeyCode::KeyP) {
        send(GameplayCommandKind::PlaceWoodFootBlock);
    }
    if keys.just_pressed(KeyCode::KeyH) {
        send(GameplayCommandKind::Craft(BuildRecipe::PlankPack));
    }
    if keys.just_pressed(KeyCode::KeyF) {
        send(GameplayCommandKind::FoundNation);
    }
    if keys.just_pressed(KeyCode::KeyJ) || keys.just_pressed(KeyCode::KeyK) {
        send(GameplayCommandKind::KillNearestCreature);
    }
}


fn collect_keys_to_action_state(
    cfg: Res<RenderConfig>,
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut ActionState<PlayerAction>, With<Controlled>>,
) {
    use leafwing_input_manager::prelude::ActionState as _;

    let mut action_state = match q.single_mut() {
        Ok(s) => s,
        Err(_) => {


            return;
        }
    };

    let w_pressed = keys.pressed(KeyCode::KeyW);


    let w_active = w_pressed || cfg.auto_walk;


    *action_state = ActionState::<PlayerAction>::default();

    if w_active {
        action_state.press(&PlayerAction::MoveForward);
    }
    if keys.pressed(KeyCode::KeyS) {
        action_state.press(&PlayerAction::MoveBackward);
    }
    if keys.pressed(KeyCode::KeyA) {
        action_state.press(&PlayerAction::MoveLeft);
    }
    if keys.pressed(KeyCode::KeyD) {
        action_state.press(&PlayerAction::MoveRight);
    }
    if keys.pressed(KeyCode::Space) {
        action_state.press(&PlayerAction::Jump);
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        action_state.press(&PlayerAction::Sprint);
    }
}












pub struct ClientPvPPlugin;

impl Plugin for ClientPvPPlugin {
    fn build(&self, app: &mut App) {
        use lk2_core::protocol::messages::{AttackInput, DamageResult, HitConfirm, KnockbackEvent};
        use lk2_core::pvp::DamageEvent;

        app.add_message::<AttackInput>()
            .add_message::<HitConfirm>()
            .add_message::<KnockbackEvent>()
            .add_message::<DamageResult>()
            .add_message::<DamageEvent>()
            .add_message::<lk2_core::pvp::VisualEffectEvent>()
            .add_systems(FixedUpdate, (lk2_core::pvp::increment_fixed_tick,));
    }
}






#[derive(Component)]
pub struct Sun;


#[derive(Resource)]
pub struct TimeOfDay(pub f32);
impl Default for TimeOfDay {
    fn default() -> Self {
        Self(0.5)
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn setup_light(mut commands: Commands) {

    commands.spawn((
        DirectionalLight {
            illuminance: 22000.0,
            shadows_enabled: true,
            color: Color::srgb(1.0, 0.96, 0.88),
            ..default()
        },
        Transform::from_xyz(40.0, 80.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y),
        Sun,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 6000.0,
            shadows_enabled: false,
            color: Color::srgb(0.85, 0.88, 0.95),
            ..default()
        },
        Transform::from_xyz(-40.0, 50.0, -25.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.92, 0.90, 0.85),
        brightness: 0.6,
        affects_lightmapped_meshes: true,
    });
}


pub fn day_night_cycle(
    time: Res<Time>,
    mut tod: ResMut<TimeOfDay>,
    mut sun: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut fill: Query<&mut DirectionalLight, (Without<Sun>, With<DirectionalLight>)>,
    mut clear: ResMut<ClearColor>,
) {
    tod.0 = (tod.0 + time.delta_secs() / 60.0) % 1.0;
    let t = tod.0;
    let dayness = (std::f32::consts::PI * t).sin().max(0.0);
    let sunset_glow = (1.0 - (2.0 * t - 1.0).abs()).powi(3);

    let dist = 80.0;
    let sun_pos = Vec3::new((t - 0.5) * 2.0 * dist, dayness * dist + 5.0, 0.0);
    if let Ok((mut tf, mut l)) = sun.single_mut() {
        *tf = Transform::from_translation(sun_pos).looking_at(Vec3::ZERO, Vec3::Y);
        l.illuminance = 1500.0 + 30000.0 * dayness;
        l.color = Color::srgb(
            1.0 - 0.15 * sunset_glow,
            0.95 - 0.35 * sunset_glow,
            0.85 - 0.65 * sunset_glow,
        );
    }
    if let Ok(mut l) = fill.single_mut() {
        l.illuminance = 3000.0 * dayness + 150.0;
    }

    let day = (0.55, 0.78, 0.98);
    let dusk = (0.95, 0.55, 0.30);
    let night = (0.18, 0.25, 0.45);



    let w_dusk = sunset_glow * 0.35;
    let w_night = (1.0 - dayness).max(0.0) * (1.0 - sunset_glow * 0.5);
    let w_day = 1.0 - w_dusk - w_night;
    clear.0 = Color::srgb(
        day.0 * w_day + dusk.0 * w_dusk + night.0 * w_night,
        day.1 * w_day + dusk.1 * w_dusk + night.1 * w_night,
        day.2 * w_day + dusk.2 * w_dusk + night.2 * w_night,
    );
}





#[allow(clippy::too_many_arguments)]
fn setup_world(
    mut commands: Commands,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut player: ResMut<PlayerState>,
) {
    let pipeline = lk2_core::world::terrain::presets::by_name(preset_name_static());
    *game_world = lk2_core::world::World::with_pipeline(constant::WORLD_SIZE, pipeline);
    info!("[terrain] using preset '{}'", game_world.pipeline.name);

    for k in ResourceKind::ALL {
        let init = k.demo_initial_amount();
        let _ = pool.force_add(*k, init);
    }

    monsters.demo_init([
        constant::WORLD_SIZE / 2,
        constant::SEA_LEVEL + 1,
        constant::WORLD_SIZE / 2,
    ]);

    let (sx, sz) =
        walk_override_static().unwrap_or((constant::WORLD_SIZE / 2, constant::WORLD_SIZE / 2));
    let (spawn_pos, spawn) = player_spawn_position_at(&game_world, sx, sz).unwrap_or((
        Vec3::new(sx as f32 + 0.5, 28.0, sz as f32 + 0.5),
        [sx, 28, sz],
    ));
    player.block_pos = spawn;
    player.pos = spawn_pos;

    if std::env::args().any(|a| a == "--auto-demo") {
        player.block_pos = [48, 16, 48];
        player.pos = Vec3::new(48.5, 16.0, 48.5);
    }
    player.inventory.insert(ResourceKind::Wood, 0);
    player.inventory.insert(ResourceKind::Food, 5);

    commands.spawn((
        Player,
        Transform::from_translation(player.pos),
        GlobalTransform::default(),
    ));

    info!(
        "🌍 世界已生成: {}³, 玩家在 {:?}",
        constant::WORLD_SIZE,
        spawn
    );
}

fn sync_player_combat_anchor(mut q: Query<&mut Transform, With<Player>>, player: Res<PlayerState>) {
    for mut transform in q.iter_mut() {
        transform.translation = player.pos;
    }
}


fn self_check(
    game_world: Res<GameWorld>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    eco: Res<EcoCycle>,
    mut obs: ResMut<TickObserver>,
) {
    info!(">>> 启动自检 100 tick ...");
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
        info!(">>> 自检 ✅ 100 tick 全部通过");
    } else {
        error!(">>> 自检 ❌ {} 处违例", violations.len());
    }
    info!("{}", obs.report());
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
        SimRole::ClientOffline,
    );
}

fn end_tick_system(
    game_world: Res<GameWorld>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    clock: Res<SimClock>,
    mut obs: ResMut<TickObserver>,
    player: Res<PlayerState>,
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





fn periodic_screenshot(
    time: Res<Time>,
    mut clock: ResMut<SimClock>,
    mut commands: Commands,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    eco: Res<EcoCycle>,
    obs: Res<TickObserver>,
    game_world: Res<GameWorld>,
    run_mode: Res<ClientRunMode>,
) {



    let now = {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        let start = START.get_or_init(std::time::Instant::now);
        start.elapsed().as_secs_f32()
    };
    let _ = time;



    if clock.tick < FIRST_SCREENSHOT_MIN_TICK {
        return;
    }

    if now - clock.last_screenshot_wall < 4.0 {
        return;
    }
    clock.last_screenshot_wall = now;
    clock.screenshot_count += 1;

    let iter_id = clock.screenshot_count;
    let iter_dir = format!("screenshots/iter_{:02}", iter_id);
    let _ = std::fs::create_dir_all(&iter_dir);
    let png_path: PathBuf = format!("{}/iter_{:02}.png", iter_dir, iter_id).into();
    let state_path = format!("{}/final_state.json", iter_dir);
    let diff_path = format!("{}/diff.json", iter_dir);

    info!("📸 截图 #{} → {}", iter_id, png_path.display());
    commands
        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(png_path));

    let state = build_state_json(
        &time,
        &clock,
        &player,
        &pool,
        &nations,
        &monsters,
        &eco,
        &obs,
        &game_world,
        *run_mode,
    );
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        if let Err(e) = std::fs::write(&state_path, s) {
            warn!("写 final_state.json 失败: {}", e);
        } else {
            info!("📝 final_state dumped → {}", state_path);
        }
    }
    if let Some(diff) = build_state_diff_for_iter(iter_id, &state) {
        if let Ok(s) = serde_json::to_string_pretty(&diff) {
            if let Err(e) = std::fs::write(&diff_path, s) {
                warn!("write diff.json failed: {}", e);
            } else {
                info!("diff.json dumped -> {}", diff_path);
            }
        }
    }
}

fn exit_on_esc(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }
}





fn setup_player_pvp(mut commands: Commands, player: Query<Entity, With<Player>>) {
    use lk2_core::combat::{
        AttackState as CombatAttackState, BlockState as CombatBlockState, Downed as CombatDowned,
        Health as CombatHealth, InputBuffer as CombatInputBuffer, Knockback as CombatKnockback,
        ParryWindow as CombatParryWindow, Stamina as CombatStamina, StunState as CombatStunState,
    };
    use lk2_core::pvp::WeaponId;
    let iron = WeaponId::IronSword.stats();
    for entity in player.iter() {
        let mut cmd = commands.entity(entity);

        cmd.insert((
            RigidBody::Kinematic,
            Collider::capsule(0.3, 0.9),
            LinearVelocity::default(),
            lk2_core::controller::PvPController::new()
                .with_speed(5.0)
                .with_jump(8.0)
                .with_knockback_resistance(0.1),
            lk2_core::controller::PlayerCollider::default(),
            CombatState::default(),

            ActionState::<PlayerAction>::default(),
            WeaponStats {
                reach: iron.reach,
                damage: iron.damage,
                knockback: iron.knockback,
                attack_speed: iron.attack_speed,
                sweep_angle_deg: iron.sweep_deg,
                sweep_range: iron.reach,
            },
            Hitbox::default(),
            lk2_core::pvp::Ping(0.0),
            PositionHistory::new(60),
            Health(100.0),
        ));


        cmd.insert((
            CombatHealth::default(),
            CombatStamina::default(),
            CombatBlockState::default(),
            CombatParryWindow::default(),
            CombatStunState::default(),
            CombatKnockback::default(),
            CombatAttackState::default(),
            CombatDowned::default(),
            CombatInputBuffer::new(0.16, 30),
        ));
        info!(
            "⚔ PvP 组件已挂载（铁剑 reach={}, dmg={}）+ V2 战斗组件（HP/STA/Block/Parry/Stun/Knockback/Attack/Downed/InputBuffer）",
            iron.reach, iron.damage
        );
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
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    eco: Res<EcoCycle>,
    obs: Res<TickObserver>,
    game_world: Res<GameWorld>,
    run_mode: Res<ClientRunMode>,
) {
    if clock.tick == 0 || clock.tick % 5 != 0 || clock.tick == rec.last_dump_tick {
        return;
    }
    rec.last_dump_tick = clock.tick;
    rec.current_iter = clock.tick as u32;
    let path = format!("screenshots/state_t{}.json", clock.tick);
    let state = build_state_json(
        &time,
        &clock,
        &player,
        &pool,
        &nations,
        &monsters,
        &eco,
        &obs,
        &game_world,
        *run_mode,
    );
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, s);
        info!("📝 tick state dumped → {}", path);
    }
}


fn build_state_json(
    time: &Time,
    clock: &SimClock,
    player: &PlayerState,
    pool: &GlobalResourcePool,
    nations: &NationRegistry,
    monsters: &MonsterEcosystem,
    eco: &EcoCycle,
    obs: &TickObserver,
    game_world: &GameWorld,
    run_mode: ClientRunMode,
) -> serde_json::Value {
    lk2_core::diagnostics::build_state_json(
        time,
        clock,
        player,
        pool,
        nations,
        monsters,
        eco,
        obs,
        game_world,
        run_mode.snapshot_role(),
    )
}

fn build_state_diff_for_iter(iter_id: u32, state: &serde_json::Value) -> Option<serde_json::Value> {
    if iter_id <= 1 {
        return None;
    }
    let prev_path = format!("screenshots/iter_{:02}/final_state.json", iter_id - 1);
    let prev_raw = std::fs::read_to_string(prev_path).ok()?;
    let prev_state: serde_json::Value = serde_json::from_str(&prev_raw).ok()?;
    Some(build_state_diff(&prev_state, state))
}

fn build_state_diff(prev: &serde_json::Value, current: &serde_json::Value) -> serde_json::Value {
    let mut deltas = Vec::new();
    for path in [
        "tick",
        "player.monsters_killed",
        "player.blocks_gathered",
        "player.nations_founded",
        "pool.wood",
        "pool.food",
        "pool.apple",
        "pool.soul",
        "nations.flag_count",
        "nations.total_nations",
        "monsters.current",
        "monsters.kingdoms",
        "monsters.nests",
        "creatures.passive_current",
        "eco_cycle.rabbits",
        "eco_cycle.berry_bushes",
        "eco_cycle.fruit",
        "eco_cycle.fruit_eaten",
        "eco_cycle.fruit_grown",
        "observer.anomalies",
        "observer.invariant_violations",
    ] {
        if let Some(delta) = diff_numeric_path(prev, current, path) {
            deltas.push(delta);
        }
    }

    deltas.sort_by(|a, b| {
        let a_abs = a["delta_abs"].as_f64().unwrap_or(0.0);
        let b_abs = b["delta_abs"].as_f64().unwrap_or(0.0);
        b_abs.partial_cmp(&a_abs).unwrap_or(std::cmp::Ordering::Equal).then_with(|| {
            a["path"].as_str().unwrap_or_default().cmp(b["path"].as_str().unwrap_or_default())
        })
    });

    serde_json::json!({
        "prev_tick": value_at_path(prev, "tick").and_then(|v| v.as_u64()),
        "tick": value_at_path(current, "tick").and_then(|v| v.as_u64()),
        "resource_deltas": deltas,
    })
}

fn diff_numeric_path(
    prev: &serde_json::Value,
    current: &serde_json::Value,
    path: &str,
) -> Option<serde_json::Value> {
    let prev_num = value_at_path(prev, path)?.as_f64()?;
    let current_num = value_at_path(current, path)?.as_f64()?;
    let delta = current_num - prev_num;
    if delta.abs() < f64::EPSILON {
        return None;
    }
    Some(serde_json::json!({
        "path": path,
        "previous": prev_num,
        "current": current_num,
        "delta": delta,
        "delta_abs": delta.abs(),
    }))
}

fn value_at_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_self_check_resources_are_registered() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GameWorld>()
            .init_resource::<GlobalResourcePool>()
            .init_resource::<NationRegistry>()
            .init_resource::<MonsterEcosystem>()
            .init_resource::<EcoCycle>()
            .init_resource::<TickObserver>();

        assert!(app.world().contains_resource::<GameWorld>());
        assert!(app.world().contains_resource::<GlobalResourcePool>());
        assert!(app.world().contains_resource::<NationRegistry>());
        assert!(app.world().contains_resource::<MonsterEcosystem>());
        assert!(app.world().contains_resource::<EcoCycle>());
        assert!(app.world().contains_resource::<TickObserver>());
    }

    #[test]
    fn auto_demo_waits_long_enough_for_first_screenshot() {
        assert!(AUTO_DEMO_WAIT_TICKS >= 1_500);
    }

    #[test]
    fn first_screenshot_waits_for_meaningful_sim_progress() {
        assert!(FIRST_SCREENSHOT_MIN_TICK >= 500);
    }

    #[test]
    fn state_diff_reports_sorted_numeric_deltas() {
        let prev = serde_json::json!({
            "tick": 100,
            "player": {
                "monsters_killed": 1,
                "blocks_gathered": 4,
                "nations_founded": 1
            },
            "pool": {
                "wood": 50,
                "food": 90,
                "apple": 70,
                "soul": 60
            },
            "nations": {
                "flag_count": 7,
                "total_nations": 7
            },
            "monsters": {
                "current": 60,
                "kingdoms": 1,
                "nests": 3
            },
            "creatures": {
                "passive_current": 5
            },
            "eco_cycle": {
                "rabbits": 5,
                "berry_bushes": 10,
                "fruit": 1,
                "fruit_eaten": 20,
                "fruit_grown": 11
            },
            "observer": {
                "anomalies": 0,
                "invariant_violations": 0
            }
        });
        let current = serde_json::json!({
            "tick": 125,
            "player": {
                "monsters_killed": 2,
                "blocks_gathered": 6,
                "nations_founded": 1
            },
            "pool": {
                "wood": 45,
                "food": 93,
                "apple": 70,
                "soul": 60
            },
            "nations": {
                "flag_count": 8,
                "total_nations": 8
            },
            "monsters": {
                "current": 58,
                "kingdoms": 1,
                "nests": 3
            },
            "creatures": {
                "passive_current": 4
            },
            "eco_cycle": {
                "rabbits": 5,
                "berry_bushes": 10,
                "fruit": 0,
                "fruit_eaten": 23,
                "fruit_grown": 13
            },
            "observer": {
                "anomalies": 0,
                "invariant_violations": 0
            }
        });

        let diff = build_state_diff(&prev, &current);
        assert_eq!(diff["prev_tick"], 100);
        assert_eq!(diff["tick"], 125);

        let deltas = diff["resource_deltas"].as_array().unwrap();
        let paths = deltas
            .iter()
            .map(|delta| delta["path"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![
                "tick",
                "pool.wood",
                "eco_cycle.fruit_eaten",
                "pool.food",
                "eco_cycle.fruit_grown",
                "monsters.current",
                "player.blocks_gathered",
                "creatures.passive_current",
                "eco_cycle.fruit",
                "nations.flag_count",
                "nations.total_nations",
                "player.monsters_killed"
            ]
        );

        let wood = deltas
            .iter()
            .find(|delta| delta["path"] == "pool.wood")
            .unwrap();
        assert_eq!(wood["previous"], 50.0);
        assert_eq!(wood["current"], 45.0);
        assert_eq!(wood["delta"], -5.0);
    }
}
