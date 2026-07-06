use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::schedule::{IntoScheduleConfigs, common_conditions::resource_equals};
use bevy::light::{AtmosphereEnvironmentMapLight, VolumetricFog, VolumetricLight};
use bevy::pbr::{AtmosphereSettings, ScreenSpaceReflections};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};

use avian3d::prelude::{Collider, Gravity, LinearVelocity, PhysicsPlugins, RigidBody};
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;

mod capture;
mod model_preview;
mod pretty;
mod pvp_systems;
mod render;
mod terrain_preview;
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
use lk2_core::player::{PlayerState, PlayerTag};
use lk2_core::pvp::{FixedTick, PositionHistory};
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::scenario::{Scenario, ScenarioState};
use lk2_core::sim::{SimRole, advance_fixed_authority_tick};
use lk2_core::world::{World as GameWorld, WorldConfig, generate_world, player_spawn_position_at};

use crate::capture::{TickRecorder, periodic_screenshot, tick_recorder};
use crate::pretty::{
    PlayerAnimState, PrettyConfig, animate_avatar, animate_cloud_puffs, animate_monsters,
    follow_grass_platform, follow_ground_details, follow_ground_discs, follow_monster_cubes,
    follow_water, spawn_eco_visuals, spawn_pretty, update_eco_visuals, update_player_anim_state,
};
use crate::pvp_systems::{
    HealthHudMarker, client_attack_predict, collect_combat_input_offline, collect_local_input,
    offline_found_nation_input, on_damage_result, on_hit_confirm, on_knockback_event,
    trigger_visual_effects,
};
use crate::render::{
    AntiStuckState, CameraAngles, CameraMode, FreeFlyState, JumpState, LastMoveDirection,
    NestMarkerCount, Player, RenderConfig, SpawnedBlocks, SwordSwing, auto_demo,
    camera_mode_toggle, cycle_terrain_preset, emergency_teleport, first_person_camera,
    freefly_movement, freefly_toggle, held_weapon_follow, maintain_cursor_grab, mouse_look_system,
    offline_anti_stuck, player_input, setup_atmosphere, setup_cursor_grab, setup_terrain_underlay,
    spawn_nest_markers, spawn_terrain_around_player, toggle_cursor_grab_on_esc,
    underlay_follow_player, update_animal_indicator, update_nest_indicator,
    update_nest_marker_positions,
};
use crate::ui::{
    ClientRunMode, setup_fonts, setup_hud, update_hud, update_nest_radar, update_tutorial_overlay,
};

const AUTO_DEMO_WAIT_TICKS: u64 = 5_500;

fn workspace_asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets")
}

use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::Controlled;
use lk2_core::protocol::PlayerAction;
use lk2_core::protocol::components::{
    EcoSnapshot, GameplayHudState, Health, VOXEL_CHUNK_SIZE_XZ, VoxelChunkSnapshot, VoxelDelta,
};
use lk2_core::protocol::messages::{BuildRecipe, GameplayCommand, GameplayCommandKind};
use lk2_core::pvp::{CombatState, Hitbox, WeaponStats};

#[derive(Resource, Default, Debug, Clone)]
pub struct OnlineCommandDiagnostics {
    pub sender_entities: usize,
    pub move_world_sent: u64,
    pub last_dx_milli: i16,
    pub last_dz_milli: i16,
}

#[derive(Resource)]
pub(crate) struct OnlineMotionTrace {
    pub(crate) enabled: bool,
    file: Option<std::fs::File>,
    sample_index: u64,
    last_player_pos: Option<Vec3>,
    last_camera_pos: Option<Vec3>,
    last_server_pos: Option<Vec3>,
    pub(crate) last_local_moved: bool,
    pub(crate) last_local_attempted: bool,
    pub(crate) last_local_reason: &'static str,
    pub(crate) last_local_dir: [f32; 3],
    pub(crate) last_local_distance: f32,
    last_server_correction: f32,
}

impl Default for OnlineMotionTrace {
    fn default() -> Self {
        Self {
            enabled: false,
            file: None,
            sample_index: 0,
            last_player_pos: None,
            last_camera_pos: None,
            last_server_pos: None,
            last_local_moved: false,
            last_local_attempted: false,
            last_local_reason: "none",
            last_local_dir: [0.0, 0.0, 0.0],
            last_local_distance: 0.0,
            last_server_correction: 0.0,
        }
    }
}

#[derive(Resource)]
struct OnlineGameplayUdp {
    socket: std::net::UdpSocket,
}

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
    eco_rabbits: usize,
    eco_wildlife: usize,
    eco_berries: usize,
    eco_plants: usize,
    last_voxel_revision: u64,
    last_chunk_revision: u64,
    last_chunk_x: i32,
    last_chunk_z: i32,
}

#[derive(Resource, Debug, Clone)]
#[allow(dead_code)]
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

#[allow(dead_code)]
const ONLINE_INTERP_SPEED: f32 = 14.0;
#[allow(dead_code)]
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
    let args: Vec<String> = std::env::args().collect();
    let offline_mode = args.iter().any(|a| a == "--offline");
    let auto_demo_mode = args.iter().any(|a| a == "--auto-demo");
    let no_scenario_mode = args.iter().any(|a| a == "--no-scenario");
    let first_person_mode = args.iter().any(|a| a == "--first-person");
    let hold_forward_test = args.iter().any(|a| a == "--hold-forward-test");
    let no_focus_window = args.iter().any(|a| a == "--no-focus-window");
    let hidden_window = args.iter().any(|a| a == "--hidden-window");
    let model_preview_mode = args.iter().any(|a| a == "--model-preview");
    let terrain_preview_mode = args.iter().any(|a| a == "--terrain-preview");

    if model_preview_mode {
        // --model-preview takes a separate code path: no physics, no network,
        // no terrain / creatures / HUD. Just lay every GLB in assets/ out on
        // a grid, render one screenshot through the real atmosphere + bloom +
        // SSR + TAA pipeline, then exit. See crates/client/src/model_preview.rs.
        model_preview::run_model_preview();
        return;
    }
    if terrain_preview_mode {
        terrain_preview::run_terrain_preview();
        return;
    }

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
    let effective_preset_name = if network_mode {
        if preset_name != "default" {
            println!(
                "[terrain] online mode ignores local --preset={} and uses server preset default",
                preset_name
            );
        }
        "default".to_string()
    } else {
        preset_name
    };
    let _ = PRESET_NAME.set(effective_preset_name.clone());
    println!("[terrain] preset = {}", effective_preset_name);

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

    let scenario = if no_scenario_mode && !auto_demo_mode {
        None
    } else if auto_demo_mode {
        Some(Scenario {
            name: "idle".into(),
            record_window: None,
            steps: vec![
                lk2_core::scenario::ScenarioStep::Log {
                    msg: "=== idle: 玩家不动看动物 ===".into(),
                },
                lk2_core::scenario::ScenarioStep::WaitTicks { ticks: AUTO_DEMO_WAIT_TICKS },
            ],
        })
    } else {
        Some(lk2_core::scenario::load_scenario_from_args_or_default(
            &args,
        ))
    };
    let scenario_name =
        scenario.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "manual".to_string());
    let scenario_state =
        scenario.map(ScenarioState::from_scenario).unwrap_or_else(ScenarioState::default);

    let _ = std::fs::create_dir_all("screenshots");

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: workspace_asset_root().to_string_lossy().into_owned(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("万国起源：最后一国 钻石版 — {}", scenario_name).into(),
                    resolution: WindowResolution::new(1280, 720),

                    present_mode: PresentMode::Immediate,
                    focused: !no_focus_window && !hidden_window,
                    visible: !hidden_window,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::log::LogPlugin { level: bevy::log::Level::INFO, ..default() }),
    );

    app.insert_resource(bevy::winit::WinitSettings::continuous());

    // iter_456: switch to the deferred opaque renderer. The deferred pipeline is
    // required for bevy 0.19's volumetric fog raymarch to sample the G-buffer
    // properly. The forward path renders fog as a flat color. Atmosphere
    // scattering itself works either way; we just want volumetric god rays to
    // work too.
    app.insert_resource(bevy::pbr::DefaultOpaqueRendererMethod::deferred());

    app.add_plugins(PhysicsPlugins::default()).insert_resource(Gravity::default());

    app.add_plugins(lightyear::prelude::client::ClientPlugins::default());
    app.add_plugins(lk2_core::protocol::ProtocolPlugin);

    app.init_resource::<lightyear::prelude::PeerMetadata>()
        .init_resource::<lk2_core::pvp::FixedTick>()
        .init_resource::<TimeOfDay>();

    app.add_message::<GameplayCommand>();

    if network_mode {
        let server_addr = connect_addr.expect("network_mode=true implies connect_addr is Some");
        if let Ok(socket) = std::net::UdpSocket::bind(std::net::SocketAddr::from(([0, 0, 0, 0], 0)))
        {
            let mut gameplay_addr = server_addr;
            gameplay_addr.set_port(gameplay_addr.port().saturating_add(1));
            if socket.connect(gameplay_addr).is_ok() {
                let _ = socket.set_nonblocking(true);
                app.insert_resource(OnlineGameplayUdp { socket });
            }
        }
        app.add_systems(Startup, move |commands: Commands| {
            let client_id_seed: u64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64 & 0xFFFF_FFFF_FFFF_FFFF)
                .unwrap_or(0xC11E_71);
            spawn_networked_client(commands, server_addr, client_id_seed);
        });
    }

    let mut render_config = RenderConfig::default();
    render_config.smooth_terrain = smooth_terrain;
    if auto_demo_mode {
        render_config.auto_walk = true;
        render_config.auto_keys = true;
        render_config.mouse_look = false;
    }
    if first_person_mode {
        render_config.auto_orbit = false;
        render_config.auto_walk = false;
        render_config.auto_keys = false;
        render_config.mouse_look = true;
        tracing::info!("--first-person: CameraMode=FirstPerson, mouse_look=true");
    }
    if hold_forward_test {
        render_config.auto_walk = true;
        render_config.auto_keys = false;
        tracing::info!("--hold-forward-test: simulating held W without OS input");
    }

    let camera_angles = if auto_demo_mode && !first_person_mode {
        CameraAngles { yaw: std::f32::consts::FRAC_PI_2, pitch: -0.22 }
    } else {
        CameraAngles::default()
    };

    app.insert_resource(render_config)
        .insert_resource(camera_angles)
        .init_resource::<SwordSwing>()
        .insert_resource(if first_person_mode {
            CameraMode::FirstPerson
        } else {
            CameraMode::default()
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
        .init_resource::<JumpState>()
        .init_resource::<AntiStuckState>()
        .init_resource::<CreatureSpawnerDone>()
        .init_resource::<FixedTick>()
        .init_resource::<ReplicatedSnapshot>()
        .init_resource::<NetworkSmoothingState>()
        .init_resource::<OnlineMotionTrace>()
        .init_resource::<NestMarkerCount>()
        .init_resource::<PlayerAnimState>()
        .init_resource::<OnlineCommandDiagnostics>()
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
        .add_plugins(ClientPvPPlugin);
    // ControllerPlugin used to write PvPController.move_input. That is now done
    // directly in player_input so online first-person movement has one owner.

    app.add_systems(
        Startup,
        (
            setup_fonts,
            setup_camera,
            setup_light,
            setup_atmosphere,
            setup_terrain_underlay,
            setup_cursor_grab,
            setup_world,
            spawn_nest_markers,
            spawn_pretty,
            spawn_eco_visuals,
            spawn_creatures,
            setup_hud,
            setup_player_pvp,
            lk2_core::objectives::setup_default_objectives,
        )
            .chain(),
    );
    app.add_systems(Update, run_startup_self_check_once);

    app.add_systems(
        Update,
        (
            lk2_core::scenario::scenario_runner.run_if(resource_equals(ClientRunMode::Offline)),
            lk2_core::scenario::simulate_player_actions
                .run_if(resource_equals(ClientRunMode::Offline)),
            lk2_core::scenario::scenario_tick_recorder
                .run_if(resource_equals(ClientRunMode::Offline)),
            apply_networked_position,
            apply_server_pos_update,
            debug_dump_replicated_entities,
            apply_authoritative_snapshot,
            apply_voxel_chunk_snapshot,
            apply_voxel_delta,
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (
            collect_keys_to_action_state,
            auto_demo.run_if(resource_equals(ClientRunMode::Offline)),
            maintain_cursor_grab,
            mouse_look_system,
            player_input,
            offline_anti_stuck.run_if(resource_equals(ClientRunMode::Offline)),
            send_online_gameplay_commands,
            first_person_camera,
            record_online_motion_trace,
            held_weapon_follow,
            sync_player_combat_anchor,
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (
            offline_player_attack_creatures.run_if(resource_equals(ClientRunMode::Offline)),
            animate_avatar,
            spawn_terrain_around_player,
            toggle_cursor_grab_on_esc,
        )
            .chain(),
    );

    app.add_systems(Update, update_player_anim_state.before(animate_avatar));
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

    app.add_systems(
        Update,
        update_nest_marker_positions.run_if(resource_equals(CameraMode::ThirdPerson)),
    );

    app.add_systems(
        Update,
        (
            follow_grass_platform,
            follow_ground_discs,
            follow_ground_details,
            underlay_follow_player,
            follow_water,
            follow_monster_cubes,
            animate_monsters,
            animate_cloud_puffs,
        )
            .chain(),
    );

    app.add_systems(Update, collect_combat_input_offline);

    app.add_systems(Update, offline_found_nation_input);

    app.add_systems(
        Update,
        (
            simulation_tick.run_if(resource_equals(ClientRunMode::Offline)),
            update_eco_visuals,
            end_tick_system.run_if(resource_equals(ClientRunMode::Offline)),
            update_hud,
            update_nest_radar,
            update_tutorial_overlay,
            update_animal_indicator,
            update_nest_indicator,
            tick_recorder,
            periodic_screenshot,
            despawn_dead_creatures.run_if(resource_equals(ClientRunMode::Offline)),
            update_creatures.run_if(resource_equals(ClientRunMode::Offline)),
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
            lightyear_connection::client::Client::default(),
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
    run_mode: Res<ClientRunMode>,
    mut q: Query<(&mut Transform, &lk2_core::protocol::components::PlayerPos), Without<Player>>,
    mut player: ResMut<PlayerState>,
) {
    let mut count = 0;
    let mut last_pos = bevy::math::Vec3::ZERO;
    for (mut tf, pos) in q.iter_mut() {
        tf.translation = pos.0;
        last_pos = pos.0;
        count += 1;
    }
    if count > 0 && *run_mode == ClientRunMode::Online {
        // First-person camera feel is driven by local prediction in
        // player_input. Keep replicated transforms current for debugging and
        // remote entities, but do not snap PlayerState.pos back to the network
        // echo every frame.
        let _ = (&mut player, last_pos);
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
    mut trace: ResMut<OnlineMotionTrace>,
) {
    if *run_mode != ClientRunMode::Online {
        for mut receiver in receiver_q.iter_mut() {
            for _msg in receiver.receive() {}
        }
        return;
    }
    let mut last_pos = bevy::math::Vec3::ZERO;
    let mut last_tick: u32 = 0;
    let mut count: u32 = 0;
    for mut receiver in receiver_q.iter_mut() {
        for msg in receiver.receive() {
            last_pos = msg.pos;
            last_tick = msg.server_tick;
            count = count.saturating_add(1);
        }
    }
    if count > 0 {
        // Keep the first-person camera on local prediction for normal movement.
        // Packet echoes can be a few frames old; applying every one feels like a
        // periodic hitch even when FPS is high. Only snap on a real desync.
        let drift = player.pos.distance(last_pos);
        trace.last_server_pos = Some(last_pos);
        trace.last_server_correction = 0.0;
        if drift > 1.5 {
            player.pos = last_pos;
            player.block_pos = [
                last_pos.x.floor() as i32,
                last_pos.y.floor() as i32,
                last_pos.z.floor() as i32,
            ];
            trace.last_server_correction = drift;
        }
        tracing::debug!(
            "[net] drained {} ServerPosUpdate messages; tick={} pos={:?}",
            count,
            last_tick,
            last_pos
        );
    }
}

fn apply_authoritative_snapshot(
    run_mode: Res<ClientRunMode>,
    hud_q: Query<&GameplayHudState>,
    eco_q: Query<&EcoSnapshot>,
    mut snapshot: ResMut<ReplicatedSnapshot>,
    mut clock: ResMut<SimClock>,
    mut player: ResMut<PlayerState>,
    mut pool: ResMut<GlobalResourcePool>,
    mut nations: ResMut<NationRegistry>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut eco: ResMut<EcoCycle>,
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
    // The FPS camera follows client-side prediction. Server snapshots update
    // replicated gameplay state, but stale snapshots must not snap the view back.
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

    if let Some(eco_state) = eco_q.iter().next() {
        eco.apply_snapshot(eco_state);
        snapshot.eco_rabbits = eco_state.rabbits.len();
        snapshot.eco_wildlife = eco_state.wildlife.len();
        snapshot.eco_berries = eco_state.berries.len();
        snapshot.eco_plants = eco_state.plants.len();
    }
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

fn apply_voxel_chunk_snapshot(
    run_mode: Res<ClientRunMode>,
    chunk_q: Query<&VoxelChunkSnapshot>,
    mut snapshot: ResMut<ReplicatedSnapshot>,
    mut game_world: ResMut<GameWorld>,
    mut spawned: ResMut<SpawnedBlocks>,
) {
    if *run_mode != ClientRunMode::Online {
        return;
    }
    for chunk in chunk_q.iter() {
        if chunk.revision < snapshot.last_chunk_revision {
            continue;
        }
        if chunk.revision == snapshot.last_chunk_revision
            && chunk.chunk_x == snapshot.last_chunk_x
            && chunk.chunk_z == snapshot.last_chunk_z
        {
            continue;
        }
        if chunk.y_size <= 0 {
            continue;
        }

        let expected_len = (VOXEL_CHUNK_SIZE_XZ * VOXEL_CHUNK_SIZE_XZ * chunk.y_size) as usize;
        if chunk.blocks.len() != expected_len {
            tracing::warn!(
                "[net] ignoring malformed chunk snapshot ({},{}): len={} expected={}",
                chunk.chunk_x,
                chunk.chunk_z,
                chunk.blocks.len(),
                expected_len
            );
            continue;
        }

        let x_min = chunk.chunk_x * VOXEL_CHUNK_SIZE_XZ;
        let z_min = chunk.chunk_z * VOXEL_CHUNK_SIZE_XZ;
        let mut i = 0usize;
        for y in chunk.y_min..(chunk.y_min + chunk.y_size) {
            for z in z_min..(z_min + VOXEL_CHUNK_SIZE_XZ) {
                for x in x_min..(x_min + VOXEL_CHUNK_SIZE_XZ) {
                    let block = block_type_from_u8(chunk.blocks[i]);
                    game_world.set(x, y, z, block);
                    i += 1;
                }
            }
        }

        snapshot.last_chunk_revision = chunk.revision;
        snapshot.last_chunk_x = chunk.chunk_x;
        snapshot.last_chunk_z = chunk.chunk_z;
        spawned.last_player_block = [i32::MIN; 3];
    }
}

fn record_online_motion_trace(
    run_mode: Res<ClientRunMode>,
    time: Res<Time>,
    player: Res<PlayerState>,
    camera_q: Query<&Transform, With<Camera3d>>,
    mut trace: ResMut<OnlineMotionTrace>,
) {
    if !trace.enabled {
        trace.enabled =
            std::env::var("LK2_MOTION_TRACE").is_ok() || std::env::var("LK2_ONLINE_PROBE").is_ok();
        if trace.enabled {
            let _ = std::fs::create_dir_all("screenshots");
            let path = if *run_mode == ClientRunMode::Online {
                "screenshots/online_motion_trace.jsonl"
            } else {
                "screenshots/offline_motion_trace.jsonl"
            };
            match std::fs::File::create(path) {
                Ok(file) => trace.file = Some(file),
                Err(err) => {
                    warn!("[motion-trace] failed to create trace file: {err}");
                    trace.enabled = false;
                }
            }
        }
    }
    if !trace.enabled {
        return;
    }

    let camera_pos = camera_q.single().ok().map(|tf| tf.translation);
    let player_delta = trace.last_player_pos.map(|p| player.pos - p).unwrap_or(Vec3::ZERO);
    let camera_delta = match (trace.last_camera_pos, camera_pos) {
        (Some(prev), Some(curr)) => curr - prev,
        _ => Vec3::ZERO,
    };
    let server_drift = trace.last_server_pos.map(|p| player.pos.distance(p)).unwrap_or(0.0);

    let sample = json!({
        "sample": trace.sample_index,
        "wall_secs": time.elapsed_secs(),
        "dt": time.delta_secs(),
        "player_pos": [player.pos.x, player.pos.y, player.pos.z],
        "player_block_pos": player.block_pos,
        "player_delta": [player_delta.x, player_delta.y, player_delta.z],
        "player_step_len": player_delta.length(),
        "camera_pos": camera_pos.map(|p| [p.x, p.y, p.z]),
        "camera_delta": [camera_delta.x, camera_delta.y, camera_delta.z],
        "camera_step_len": camera_delta.length(),
        "local_move_attempted": trace.last_local_attempted,
        "local_move_moved": trace.last_local_moved,
        "local_move_reason": trace.last_local_reason,
        "local_move_dir": trace.last_local_dir,
        "local_move_distance": trace.last_local_distance,
        "server_pos": trace.last_server_pos.map(|p| [p.x, p.y, p.z]),
        "server_drift": server_drift,
        "server_correction": trace.last_server_correction,
    });

    if let Some(file) = trace.file.as_mut() {
        let _ = writeln!(file, "{sample}");
    }
    trace.sample_index = trace.sample_index.saturating_add(1);
    trace.last_player_pos = Some(player.pos);
    trace.last_camera_pos = camera_pos;
    trace.last_server_correction = 0.0;
}

fn send_online_gameplay_commands(
    run_mode: Res<ClientRunMode>,
    cfg: Res<RenderConfig>,
    keys: Res<ButtonInput<KeyCode>>,
    player: Res<PlayerState>,
    clock: Res<SimClock>,
    angles: Res<CameraAngles>,
    mut diagnostics: ResMut<OnlineCommandDiagnostics>,
    gameplay_udp: Option<Res<OnlineGameplayUdp>>,
    writer: Option<MessageWriter<GameplayCommand>>,
) {
    if *run_mode != ClientRunMode::Online {
        return;
    }
    diagnostics.sender_entities = usize::from(writer.is_some());
    let mut writer = writer;

    let mut send = |kind: GameplayCommandKind| {
        if let Some(writer) = writer.as_mut() {
            writer.write(GameplayCommand {
                tick: clock.tick,
                player_block: player.block_pos,
                kind,
            });
        }
    };

    let mut move_input = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) || cfg.auto_walk {
        move_input.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        move_input.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        move_input.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        move_input.x += 1.0;
    }
    if move_input.length_squared() > 0.0001 {
        let move_input = move_input.normalize_or_zero();
        let (sy, cy) = angles.yaw.sin_cos();
        let forward = Vec3::new(sy, 0.0, -cy).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let sprint = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        let sprint_factor = if sprint { 1.5 } else { 1.0 };
        let world_dir =
            (forward * move_input.y + right * move_input.x).normalize_or_zero() * sprint_factor;
        let dx_milli = (world_dir.x * 1000.0).round() as i16;
        let dz_milli = (world_dir.z * 1000.0).round() as i16;
        diagnostics.move_world_sent = diagnostics.move_world_sent.saturating_add(1);
        diagnostics.last_dx_milli = dx_milli;
        diagnostics.last_dz_milli = dz_milli;
        if let Some(udp) = gameplay_udp.as_deref() {
            let _ = udp.socket.send(format!("MOVE {} {}\n", dx_milli, dz_milli).as_bytes());
        } else {
            send(GameplayCommandKind::MoveWorld { dx_milli, dz_milli });
        }
    }

    if keys.just_pressed(KeyCode::Space) {
        send(GameplayCommandKind::Jump);
    }
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
        Self(0.42)
    }
}

fn setup_camera(mut commands: Commands) {
    // iter_456: wire the bevy 0.19 atmosphere + post-process stack onto the camera
    // so the rendered scene actually uses atmospheric scattering, volumetric fog,
    // bloom, screen-space reflections, and TAA instead of a flat clear color.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        // AtmosphereSettings is required so the renderer actually reads the nearest
        // Atmosphere entity. Default uses the lookup-texture path (cheap).
        AtmosphereSettings::default(),
        // Lets the atmosphere drive ambient lighting + IBL for this view.
        AtmosphereEnvironmentMapLight::default(),
        // RAW_SUNLIGHT + Atmospheric scattering requires a higher exposure than the
        // standard 13.0 EV recommended by the official atmosphere example.
        Exposure { ev100: 13.0 },
        Tonemapping::AcesFitted,
        Bloom::NATURAL,
        // ambient_intensity 0 so the fog isn't tinted by ambient light; the scene
        // already has a global ambient light setup, we just want the god-ray tint.
        VolumetricFog { ambient_intensity: 0.0, ..default() },
        Msaa::Off,
        TemporalAntiAliasing::default(),
        ScreenSpaceReflections { min_perceptual_roughness: 0.0..0.0, ..default() },
    ));
}

fn setup_light(mut commands: Commands) {
    // iter_456: Sun must use `lux::RAW_SUNLIGHT` + the `VolumetricLight` marker for
    // bevy 0.19's atmospheric scattering to compute the sun's contribution to the
    // sky and to cast volumetric god-rays through the volumetric fog.
    commands.spawn((
        DirectionalLight {
            illuminance: bevy::light::light_consts::lux::RAW_SUNLIGHT,
            shadow_maps_enabled: true,
            color: Color::srgb(1.0, 0.96, 0.88),
            ..default()
        },
        Transform::from_xyz(40.0, 80.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y),
        Sun,
        VolumetricLight,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 6000.0,
            shadow_maps_enabled: false,
            color: Color::srgb(0.85, 0.88, 0.95),
            ..default()
        },
        Transform::from_xyz(-40.0, 50.0, -25.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.98, 0.96, 0.88),
        brightness: 1.15,
        affects_lightmapped_meshes: true,
    });
}

pub fn day_night_cycle(
    time: Res<Time>,
    mut tod: ResMut<TimeOfDay>,
    mut sun: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut fill: Query<&mut DirectionalLight, (Without<Sun>, With<DirectionalLight>)>,
) {
    // iter_456: the legacy `mut clear: ResMut<ClearColor>` is gone. Bevy 0.19's
    // `Atmosphere` entity computes the sky color procedurally from the sun
    // direction, so the clear color is no longer the sky. We still drive the
    // sun's transform + intensity, which lets the atmosphere produce sunset /
    // dawn colors automatically (Rayleigh + Mie scattering).
    if std::env::args().any(|a| a == "--auto-demo") {
        tod.0 = 0.42;
    } else {
        tod.0 = (tod.0 + time.delta_secs() / 60.0) % 1.0;
    }
    let t = tod.0;
    let dayness = (std::f32::consts::PI * t).sin().max(0.0);
    let sunset_glow = (1.0 - (2.0 * t - 1.0).abs()).powi(3);

    let dist = 80.0;
    let sun_pos = Vec3::new((t - 0.5) * 2.0 * dist, dayness * dist + 5.0, 0.0);
    if let Ok((mut tf, mut l)) = sun.single_mut() {
        *tf = Transform::from_translation(sun_pos).looking_at(Vec3::ZERO, Vec3::Y);
        // RAW_SUNLIGHT at full day; ramp down to near-moonless at night so the
        // Atmosphere's Mie phase renders the sun correctly without blowing out
        // the camera exposure.
        l.illuminance = bevy::light::light_consts::lux::RAW_SUNLIGHT * (0.05 + 0.95 * dayness);
        l.color = Color::srgb(
            1.0 - 0.15 * sunset_glow,
            0.95 - 0.35 * sunset_glow,
            0.85 - 0.65 * sunset_glow,
        );
    }
    if let Ok(mut l) = fill.single_mut() {
        l.illuminance = 3000.0 * dayness + 150.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn setup_world(
    mut commands: Commands,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut player: ResMut<PlayerState>,
    mut anti_stuck: ResMut<AntiStuckState>,
) {
    *game_world = generate_world(&WorldConfig {
        preset: preset_name_static().to_string(),
        ..WorldConfig::default()
    });
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
    // iter_210: fallback to SEA_LEVEL + 2 instead of an arbitrary 28m so that
    // first-person view never spawns the player floating in the sky if
    // standable_foot_y can't find a column (server's authoritative spawn uses
    // the same SEA_LEVEL + 2 fallback at crates/server/src/main.rs:770).
    let fallback_y = (constant::SEA_LEVEL + 2) as f32;
    let (spawn_pos, spawn) = player_spawn_position_at(&game_world, sx, sz).unwrap_or((
        Vec3::new(sx as f32 + 0.5, fallback_y, sz as f32 + 0.5),
        [sx, constant::SEA_LEVEL + 2, sz],
    ));
    player.block_pos = spawn;
    player.pos = spawn_pos;

    // iter_210: --auto-demo previously hard-overrode player pos to (48.5, 16,
    // 48.5) regardless of where the world actually has solid ground. With the
    // default preset's SpawnHill plus install_huge_spawn_platform the real
    // ground at (48, 48) is around y=15 (top of the platform leaves), so the
    // legacy override was close but the spawn position computed from the
    // world is now the authoritative answer. Keep the legacy override only
    // when the auto-demo flag is set AND first-person is NOT active.
    if std::env::args().any(|a| a == "--auto-demo")
        && !std::env::args().any(|a| a == "--first-person")
        && (spawn_pos - Vec3::new(48.5, 16.0, 48.5)).length() < 2.0
    {
        player.block_pos = [48, 16, 48];
        player.pos = Vec3::new(48.5, 16.0, 48.5);
    }
    anti_stuck.last_safe_pos = player.pos;
    anti_stuck.last_safe_block = player.block_pos;
    anti_stuck.unsafe_ticks = 0;
    player.inventory.insert(ResourceKind::Wood, 0);
    player.inventory.insert(ResourceKind::Food, 5);

    commands.spawn((
        Player,
        PlayerTag(0),
        Transform::from_translation(player.pos),
        GlobalTransform::default(),
    ));

    info!(
        "🌍 世界已生成: {}³, 玩家在 {:?}",
        constant::WORLD_SIZE,
        spawn
    );
}

fn sync_player_combat_anchor(
    mut q: Query<&mut Transform, With<Player>>,
    player: Res<PlayerState>,
    run_mode: Res<ClientRunMode>,
    cfg: Res<RenderConfig>,
    diagnostics: Res<OnlineCommandDiagnostics>,
    trace: Res<OnlineMotionTrace>,
    gameplay_udp: Option<Res<OnlineGameplayUdp>>,
    time: Res<Time>,
    mut last_probe: Local<f32>,
) {
    for mut transform in q.iter_mut() {
        transform.translation = player.pos;
    }
    if *run_mode == ClientRunMode::Online
        && std::env::var("LK2_ONLINE_PROBE").is_ok()
        && time.elapsed_secs() - *last_probe >= 1.0
    {
        *last_probe = time.elapsed_secs();
        let _ = std::fs::write(
            "screenshots/online_local_probe.json",
            serde_json::json!({
                "wall_secs": time.elapsed_secs(),
                "run_mode": format!("{:?}", *run_mode),
                "auto_walk": cfg.auto_walk,
                "mouse_look": cfg.mouse_look,
                "online_udp": gameplay_udp.is_some(),
                "sender_entities": diagnostics.sender_entities,
                "move_world_sent": diagnostics.move_world_sent,
                "last_dx_milli": diagnostics.last_dx_milli,
                "last_dz_milli": diagnostics.last_dz_milli,
                "local_move_attempted": trace.last_local_attempted,
                "local_move_moved": trace.last_local_moved,
                "local_move_reason": trace.last_local_reason,
                "local_move_dir": trace.last_local_dir,
                "local_move_distance": trace.last_local_distance,
                "motion_trace_enabled": trace.enabled,
                "motion_trace_samples": trace.sample_index,
                "player_pos": [player.pos.x, player.pos.y, player.pos.z],
                "player_block_pos": player.block_pos,
            })
            .to_string(),
        );
    }
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
    info!(">>> running deferred startup self-check for 100 ticks");
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
    if report.violations.is_empty() {
        info!(">>> deferred startup self-check passed");
    } else {
        error!(
            ">>> deferred startup self-check failed: {} violations",
            report.violations.len()
        );
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

fn exit_on_esc(keys: Res<ButtonInput<KeyCode>>, cfg: Res<RenderConfig>) {
    if cfg.mouse_look {
        return;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::FIRST_SCREENSHOT_MIN_FRAME;

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
        assert!(FIRST_SCREENSHOT_MIN_FRAME >= 500);
    }
}
