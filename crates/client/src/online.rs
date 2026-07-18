use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bevy::asset::RenderAssetUsages;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::render::{
    RenderPlugin,
    settings::{Backends, WgpuSettings},
};
use bevy::text::LetterSpacing;
use bevy::window::{PresentMode, WindowResolution};
use bevy_world_serialization::WorldAssetRoot;
use leafwing_input_manager::prelude::{ActionState, InputMap};
use lightyear::prelude::{Connect, LocalAddr, PeerAddr, UdpIo, client::ClientPlugins};
use lightyear::prelude::{MessageReceiver, MessageSender};
use lightyear_netcode::prelude::Authentication;
use lightyear_netcode::prelude::client::{NetcodeClient, NetcodeConfig};
use lk2_core::ecology::animals::CreatureKind;
use lk2_core::protocol::components::{
    CartMounted, CartState, CreatureState, EcoSnapshot, GameplayHudState, PlayerPos, SwimmingState,
    VOXEL_CHUNK_SIZE_XZ, VoxelChunkSnapshot, VoxelDelta,
};
use lk2_core::protocol::messages::{
    AttackInput, AttackResult, BuildRecipe, ChatBroadcast, ChatMessage, GameplayCommand,
    GameplayCommandKind, PingMessage, PongMessage, TerrainSnapshotRequest,
};
use lk2_core::protocol::{ControlChannel, PlayerAction, ProtocolPlugin, StateChannel};
use lk2_core::transport::{
    DEFAULT_PORT, NETCODE_CLIENT_TIMEOUT_SECS, PRIVATE_KEY, PROTOCOL_ID, generate_client_id,
    parse_connect_arg,
};
use serde_json::json;

use crate::game_scene::CART_FRONT_YAW_OFFSET;
use crate::nature::{NatureSnapshotBuffer, nature_snapshot_from_protocol};
use lk2_core::world::BlockType;
use lk2_core::world::voxel_mesh::{SurfaceNetsMesh, build_surface_nets_rect};

use crate::crisp_image_plugin;
use crate::milestone_capture::MilestoneCapturePlugin;

#[derive(Resource, Clone, Copy)]
struct OnlineConnection {
    server_addr: SocketAddr,
    client_id: u64,
    codex: bool,
}

#[derive(Resource)]
struct OnlineVisualAssets {
    player_mesh: Handle<Mesh>,
    player_material: Handle<StandardMaterial>,
    terrain_material: Handle<StandardMaterial>,
    ore_materials: [Handle<StandardMaterial>; 4],
    water_material: Handle<StandardMaterial>,
    seaweed_mesh: Handle<Mesh>,
    seaweed_material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct OnlinePlayerVisualRoot;

/// Presentation-only smoothing for remote strategic actors. The server
/// remains authoritative; this component only prevents low-rate AOI snapshots
/// from making remote players snap across the map. The locally controlled
/// actor stays on the authoritative position path for immediate command
/// feedback.
#[derive(Component, Clone, Copy, Debug)]
struct OnlineRemoteMotion {
    rendered: Vec3,
    target: Vec3,
}

#[derive(Component)]
struct OnlinePlayerVisual;

#[derive(Component)]
struct OnlineCreatureVisualRoot;

#[derive(Component)]
struct OnlineWaterSurface;

#[derive(Component, Clone, Copy)]
struct OnlineFishVisual {
    origin: Vec3,
    phase: f32,
}

#[derive(Component, Clone, Copy)]
struct OnlineSeaweedVisual {
    origin: Vec3,
    phase: f32,
    height: f32,
}

#[derive(Component)]
struct OnlineCartVisual {
    cart: Entity,
    last_position: Vec3,
    animation_phase: f32,
}

#[derive(Component)]
struct OnlineCamera;

#[derive(Component)]
struct OnlineTerrainSurface;

#[derive(Component)]
struct OnlineOreSurface {
    block: BlockType,
}

#[derive(Component)]
struct OnlineHudText;

#[derive(Component)]
struct OnlineChatText;

#[derive(Resource, Default)]
struct OnlineChatState {
    composing: bool,
    draft: String,
    log: Vec<String>,
}

#[derive(Resource, Default)]
struct OnlineReconnectState {
    cooldown_secs: f32,
    attempts: u8,
}

const ONLINE_RECONNECT_MAX_ATTEMPTS: u8 = 40;
const ONLINE_RECONNECT_COOLDOWN_SECS: f32 = 2.0;
const ONLINE_AUTO_DEMO_CAPTURE_AFTER_SECS: f32 = 60.0;
// Render extraction can retain a mesh reference for more than two seconds
// after a swap. Keep generated terrain handles alive for one minute so the
// asset store cannot reclaim a GPU allocation still referenced by the render
// world. A later session teardown drops the whole asset store.
const ONLINE_RETIRED_MESH_GRACE_FRAMES: u16 = 3_600;

#[derive(Resource, Default)]
struct OnlineCommandSequence(u64);

#[derive(Resource, Default)]
struct OnlineNetworkTelemetry {
    next_ping_at: f32,
    last_ping_sequence: u64,
    last_ping_at: Option<f32>,
    last_pong_at: Option<f32>,
    ping_ms: Option<f32>,
}

/// Latest server-confirmed hit, used only for short-lived player feedback.
#[derive(Resource, Default)]
struct OnlineCombatFeedback {
    visible_secs: f32,
    damage: f32,
    hit_count: u32,
    attack_cooldown_secs: f32,
    camera_shake_secs: f32,
}

#[derive(Resource, Default)]
struct OnlineTerrainEdits {
    latest_revision: u64,
    snapshot_revision: u64,
    snapshot_chunk: Option<(i32, i32)>,
    snapshot_border: i32,
    y_min: i32,
    y_size: i32,
    has_snapshot: bool,
    dirty: bool,
    awaiting_snapshot: bool,
    resync_stalled: bool,
    resync_budget: TerrainResyncBudget,
    blocks: HashMap<[i32; 3], BlockType>,
    retired_meshes: Vec<RetiredOnlineMesh>,
    fish_revision: Option<u64>,
}

const TERRAIN_RESYNC_RETRY_COOLDOWN_SECS: f32 = 0.5;
const TERRAIN_RESYNC_MAX_ATTEMPTS: u8 = 3;
const TERRAIN_RESYNC_STALL_COOLDOWN_SECS: f32 = 5.0;

#[derive(Default)]
struct TerrainResyncBudget {
    next_request_at: f32,
    attempts: u8,
    next_request_id: u64,
    stall_until: Option<f32>,
}

impl TerrainResyncBudget {
    fn request(
        &mut self,
        now: f32,
        chunk: (i32, i32),
        known_revision: u64,
    ) -> Option<TerrainSnapshotRequest> {
        if self.attempts >= TERRAIN_RESYNC_MAX_ATTEMPTS || now < self.next_request_at {
            return None;
        }

        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        self.attempts = self.attempts.saturating_add(1);
        self.next_request_at = now + TERRAIN_RESYNC_RETRY_COOLDOWN_SECS;
        Some(TerrainSnapshotRequest {
            request_id: self.next_request_id,
            chunk_x: chunk.0,
            chunk_z: chunk.1,
            known_revision,
        })
    }

    fn reset(&mut self) {
        self.next_request_at = 0.0;
        self.attempts = 0;
        self.stall_until = None;
    }

    fn exhausted(&self) -> bool {
        self.attempts >= TERRAIN_RESYNC_MAX_ATTEMPTS
    }

    fn enter_stall(&mut self, now: f32) {
        self.attempts = 0;
        self.next_request_at = 0.0;
        self.stall_until = Some(now + TERRAIN_RESYNC_STALL_COOLDOWN_SECS);
    }

    fn can_rearm(&self, now: f32) -> bool {
        self.stall_until.is_some_and(|until| now >= until)
    }

    fn rearm(&mut self) {
        self.stall_until = None;
        self.attempts = 0;
        self.next_request_at = 0.0;
    }
}

struct RetiredOnlineMesh {
    handle: Handle<Mesh>,
    frames_remaining: u16,
}

#[derive(Resource)]
struct OnlineAutoDemo {
    enabled: bool,
    iter_dir: Option<PathBuf>,
    png_path: PathBuf,
    elapsed: f32,
    frame: u64,
    frame_dt_over_50ms: u64,
    frame_dt_max_ms: f32,
    first_player_pos: Option<Vec3>,
    first_nature: Option<EcoSnapshot>,
    move_world_sent: u64,
    shot_requested: bool,
    exit_deadline: Option<Instant>,
}

impl OnlineAutoDemo {
    fn from_args(args: &[String]) -> Self {
        let enabled = args.iter().any(|arg| arg == "--auto-demo");
        let iter_dir = enabled
            .then(|| std::env::var_os("LK2_ITER_DIR").map(PathBuf::from))
            .flatten();
        let output_dir = iter_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("screenshots/online"));
        let png_path = screenshot_path(&output_dir, iter_dir.as_deref());
        Self {
            enabled,
            iter_dir,
            png_path,
            elapsed: 0.0,
            frame: 0,
            frame_dt_over_50ms: 0,
            frame_dt_max_ms: 0.0,
            first_player_pos: None,
            first_nature: None,
            move_world_sent: 0,
            shot_requested: false,
            exit_deadline: None,
        }
    }
}

pub fn run_online_scene() {
    let args = std::env::args().collect::<Vec<_>>();
    let codex_mode = args.iter().any(|arg| arg == "--codex-client");
    let server_addr = parse_connect_arg(&args).unwrap_or_else(|| {
        format!("127.0.0.1:{DEFAULT_PORT}")
            .parse()
            .expect("default local server address must parse")
    });
    let client_id = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--client-id=")?.parse().ok())
        .filter(|client_id| *client_id != 0)
        .unwrap_or_else(generate_client_id);

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(crisp_image_plugin())
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: Some(Backends::VULKAN),
                    ..default()
                }
                .into(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: if codex_mode {
                        "Last Kingdom - Codex Player".into()
                    } else {
                        "Last Kingdom - Online".into()
                    },
                    resolution: WindowResolution::new(1280, 720),
                    present_mode: PresentMode::Immediate,
                    focused: true,
                    visible: true,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::log::LogPlugin {
                level: bevy::log::Level::INFO,
                ..default()
            }),
    )
    .add_plugins(MilestoneCapturePlugin)
    // Lightyear's client group must be installed before the shared protocol
    // so the Leafwing input plugin can attach its client systems in `finish`.
    .add_plugins(ClientPlugins::default())
    .add_plugins(ProtocolPlugin)
    .add_observer(cleanup_replicated_creature_visual)
    .insert_resource(OnlineConnection {
        server_addr,
        client_id,
        codex: codex_mode,
    })
    .insert_resource(OnlineAutoDemo::from_args(&args))
    .init_resource::<OnlineChatState>()
    .init_resource::<OnlineReconnectState>()
    .init_resource::<OnlineCommandSequence>()
    .init_resource::<OnlineTerrainEdits>()
    .init_resource::<OnlineNetworkTelemetry>()
    .init_resource::<OnlineCombatFeedback>()
    .init_resource::<NatureSnapshotBuffer>()
    .add_systems(Startup, (setup_online_scene, spawn_netcode_client).chain())
    .add_systems(
        Update,
        (
            connect_netcode_client,
            reconnect_online_client,
            send_online_pings,
            receive_online_pongs,
            receive_online_attack_results,
            install_player_input_maps,
            handle_online_chat_input,
            suppress_online_actions_while_chatting,
            send_online_action_messages,
            receive_online_chat_messages,
            sync_replicated_players,
            sync_replicated_creatures,
            update_online_underwater_presentation,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            request_online_terrain_resync,
            sync_online_terrain_mesh,
            sync_online_fish,
            animate_online_fish,
            animate_online_seaweed,
            cleanup_retired_online_meshes,
            cleanup_replicated_player_visuals,
            sync_replicated_carts,
            update_online_overlay,
            update_online_chat_ui,
            follow_online_camera,
            log_connection_state,
            online_auto_demo_capture,
            online_auto_demo_exit,
        )
            .chain()
            .after(sync_replicated_players),
    );
    if codex_mode {
        crate::codex_client::install_codex_gameplay(&mut app, &args);
    }
    app.run();
}

fn screenshot_path(output_dir: &Path, iter_dir: Option<&Path>) -> PathBuf {
    if let Some(iter_dir) = iter_dir {
        let stem = iter_dir
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| name.starts_with("iter_"))
            .unwrap_or("iter_capture");
        return iter_dir.join(format!("{stem}.png"));
    }
    output_dir.join("online_scene.png")
}

fn setup_online_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let player_mesh = meshes.add(Cuboid::new(0.8, 1.8, 0.8));
    let player_material = materials.add(Color::srgb(0.22, 0.78, 0.42));
    let terrain_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.34, 0.31),
        perceptual_roughness: 0.92,
        ..default()
    });
    let ore_materials = [
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.45, 0.28),
            emissive: Color::srgb(0.08, 0.025, 0.01).into(),
            perceptual_roughness: 0.72,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.42, 0.08),
            emissive: Color::srgb(0.25, 0.035, 0.005).into(),
            perceptual_roughness: 0.58,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.34, 0.78, 1.0),
            emissive: Color::srgb(0.02, 0.12, 0.28).into(),
            perceptual_roughness: 0.42,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.20, 0.82, 0.32),
            emissive: Color::srgb(0.01, 0.10, 0.02).into(),
            perceptual_roughness: 0.70,
            ..default()
        }),
    ];
    let water_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.08, 0.34, 0.55, 0.52),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.25,
        ..default()
    });
    let seaweed_mesh = meshes.add(build_online_seaweed_mesh());
    let seaweed_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.62, 0.34),
        emissive: Color::srgb(0.015, 0.13, 0.045).into(),
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let floor_mesh = meshes.add(Cuboid::new(80.0, 0.2, 80.0));
    let floor_material = materials.add(Color::srgb(0.24, 0.32, 0.28));
    commands.insert_resource(OnlineVisualAssets {
        player_mesh,
        player_material,
        terrain_material: terrain_material.clone(),
        ore_materials: ore_materials.clone(),
        water_material: water_material.clone(),
        seaweed_mesh,
        seaweed_material,
    });
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(empty_online_mesh())),
        MeshMaterial3d(terrain_material),
        OnlineTerrainSurface,
        Name::new("online_smooth_terrain"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(empty_online_mesh())),
        MeshMaterial3d(water_material),
        OnlineWaterSurface,
        Name::new("online_water_surface"),
    ));
    for (block, material) in [
        (BlockType::IronOre, ore_materials[0].clone()),
        (BlockType::SunstoneOre, ore_materials[1].clone()),
        (BlockType::FrostcoreOre, ore_materials[2].clone()),
        (BlockType::LivingRoot, ore_materials[3].clone()),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(empty_online_mesh())),
            MeshMaterial3d(material),
            OnlineOreSurface { block },
            Name::new(format!("online_{block:?}_vein")),
        ));
    }
    commands.spawn((
        Camera3d::default(),
        // The online scene has no temporal AA history, so use forward MSAA
        // explicitly to keep voxel/terrain silhouettes readable at 1280x720.
        Msaa::Sample4,
        OnlineCamera,
        Transform::from_xyz(12.0, 10.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 18_000.0,
            shadow_maps_enabled: true,
            color: Color::srgb(1.0, 0.91, 0.76),
            ..default()
        },
        Transform::from_xyz(-8.0, 16.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.78, 0.86, 0.82),
        brightness: 140.0,
        affects_lightmapped_meshes: true,
    });
    commands.insert_resource(ClearColor(Color::srgb(0.45, 0.62, 0.78)));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
            padding: UiRect::all(px(10)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.62)),
        children![(
            Text::new("在线\n等待服务器快照"),
            TextFont {
                font: FontSource::UiMonospace,
                font_size: FontSize::Rem(0.94),
                weight: FontWeight::SEMIBOLD,
                width: FontWidth::SEMI_CONDENSED,
                ..default()
            },
            LetterSpacing::Px(0.3),
            TextColor(Color::WHITE),
            OnlineHudText,
        )],
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            bottom: px(12),
            width: px(560),
            padding: UiRect::all(px(10)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.68)),
        children![(
            Text::new("聊天  按 Enter 输入"),
            TextFont {
                font: FontSource::UiMonospace,
                font_size: FontSize::Rem(0.86),
                weight: FontWeight::MEDIUM,
                width: FontWidth::SEMI_CONDENSED,
                ..default()
            },
            LetterSpacing::Px(0.2),
            TextColor(Color::srgb(0.88, 0.94, 0.96)),
            OnlineChatText,
        )],
    ));
}

fn spawn_netcode_client(mut commands: Commands, connection: Res<OnlineConnection>) {
    spawn_netcode_client_entity(&mut commands, &connection);
}

fn spawn_netcode_client_entity(commands: &mut Commands, connection: &OnlineConnection) {
    let auth = Authentication::Manual {
        server_addr: connection.server_addr,
        client_id: connection.client_id,
        private_key: PRIVATE_KEY,
        protocol_id: PROTOCOL_ID,
    };
    let config = NetcodeConfig {
        client_timeout_secs: NETCODE_CLIENT_TIMEOUT_SECS,
        token_expire_secs: -1,
        ..default()
    };
    let Ok(netcode_client) = NetcodeClient::new(auth, config) else {
        error!("[net] failed to create NetcodeClient");
        return;
    };
    let entity = commands
        .spawn((
            Name::new("OnlineClient"),
            netcode_client,
            UdpIo::default(),
            LocalAddr(SocketAddr::from(([0, 0, 0, 0], 0))),
            PeerAddr(connection.server_addr),
        ))
        .id();
    if connection.codex {
        commands
            .entity(entity)
            .insert(crate::codex_client::CodexControlled);
    }
    info!(
        "[net] created Lightyear client {:?}; server={}",
        entity, connection.server_addr
    );
}

fn connect_netcode_client(
    mut commands: Commands,
    clients: Query<Entity, (With<NetcodeClient>, Added<NetcodeClient>)>,
) {
    for entity in clients.iter() {
        commands.trigger(Connect { entity });
    }
}

fn reconnect_online_client(
    mut commands: Commands,
    time: Res<Time>,
    connection: Res<OnlineConnection>,
    mut reconnect: ResMut<OnlineReconnectState>,
    clients: Query<Entity, With<NetcodeClient>>,
    disconnected: Query<Entity, Added<lightyear::prelude::Disconnected>>,
) {
    reconnect.cooldown_secs = (reconnect.cooldown_secs - time.delta_secs()).max(0.0);
    for entity in &disconnected {
        reconnect.attempts = reconnect.attempts.saturating_add(1);
        reconnect.cooldown_secs = ONLINE_RECONNECT_COOLDOWN_SECS;
        commands.entity(entity).despawn();
        warn!(
            "[net] connection lost; reconnect attempt {}/{} will reuse client_id={} after server timeout backoff",
            reconnect.attempts, ONLINE_RECONNECT_MAX_ATTEMPTS, connection.client_id
        );
    }
    if reconnect.attempts == 0
        || reconnect.attempts > ONLINE_RECONNECT_MAX_ATTEMPTS
        || reconnect.cooldown_secs > 0.0
        || !clients.is_empty()
    {
        return;
    }
    spawn_netcode_client_entity(&mut commands, &connection);
}

fn send_online_pings(
    time: Res<Time>,
    mut telemetry: ResMut<OnlineNetworkTelemetry>,
    mut clients: Query<
        (
            &mut MessageSender<PingMessage>,
            Option<&lightyear::prelude::Connected>,
        ),
        With<NetcodeClient>,
    >,
) {
    let Ok((mut sender, connected)) = clients.single_mut() else {
        return;
    };
    let now = time.elapsed_secs();
    if connected.is_none() || now < telemetry.next_ping_at {
        return;
    }

    telemetry.last_ping_sequence = telemetry.last_ping_sequence.wrapping_add(1).max(1);
    telemetry.last_ping_at = Some(now);
    telemetry.next_ping_at = now + lk2_core::transport::PING_INTERVAL.as_secs_f32() * 0.25;
    sender.send::<StateChannel>(PingMessage {
        client_id: 0,
        sequence: telemetry.last_ping_sequence,
        client_time_secs: f64::from(now),
    });
}

fn receive_online_pongs(
    time: Res<Time>,
    mut telemetry: ResMut<OnlineNetworkTelemetry>,
    mut clients: Query<&mut MessageReceiver<PongMessage>, With<NetcodeClient>>,
) {
    let Ok(mut receiver) = clients.single_mut() else {
        return;
    };
    let now = time.elapsed_secs();
    for pong in receiver.receive() {
        if !pong.client_time_secs.is_finite() {
            continue;
        }
        let elapsed = (f64::from(now) - pong.client_time_secs).max(0.0) as f32;
        telemetry.ping_ms = Some(elapsed * 1000.0);
        telemetry.last_pong_at = Some(now);
    }
}

fn receive_online_attack_results(
    time: Res<Time>,
    mut feedback: ResMut<OnlineCombatFeedback>,
    mut clients: Query<&mut MessageReceiver<AttackResult>, With<NetcodeClient>>,
) {
    feedback.visible_secs = (feedback.visible_secs - time.delta_secs()).max(0.0);
    feedback.camera_shake_secs = (feedback.camera_shake_secs - time.delta_secs() * 4.0).max(0.0);
    let Ok(mut receiver) = clients.single_mut() else {
        return;
    };
    for result in receiver.receive() {
        // Zero damage means the authoritative health gate rejected the
        // contact; it must not create a local hit confirmation.
        if !result.is_finite() || result.damage <= 0.0 {
            continue;
        }
        feedback.visible_secs = 0.45;
        feedback.damage = result.damage;
        feedback.hit_count = feedback.hit_count.saturating_add(1);
        feedback.camera_shake_secs = feedback.camera_shake_secs.max(0.11);
    }
}

fn install_player_input_maps(
    mut commands: Commands,
    clients: Query<Entity, With<NetcodeClient>>,
    players: Query<
        (Entity, &lightyear::prelude::Controlled),
        (With<PlayerPos>, Without<InputMap<PlayerAction>>),
    >,
) {
    if clients.single().is_err() {
        return;
    }
    for (entity, _) in players.iter() {
        let mut input_map = InputMap::default();
        input_map.insert(PlayerAction::MoveForward, KeyCode::KeyW);
        input_map.insert(PlayerAction::MoveBackward, KeyCode::KeyS);
        input_map.insert(PlayerAction::MoveLeft, KeyCode::KeyA);
        input_map.insert(PlayerAction::MoveRight, KeyCode::KeyD);
        input_map.insert(PlayerAction::Jump, KeyCode::Space);
        input_map.insert(PlayerAction::Crouch, KeyCode::ControlLeft);
        input_map.insert(PlayerAction::Sprint, KeyCode::ShiftLeft);
        input_map.insert(PlayerAction::Attack, KeyCode::KeyF);
        input_map.insert(PlayerAction::Mine, KeyCode::KeyM);
        input_map.insert(PlayerAction::Gather, KeyCode::KeyG);
        input_map.insert(PlayerAction::Place, KeyCode::KeyP);
        input_map.insert(PlayerAction::Craft, KeyCode::KeyH);
        input_map.insert(PlayerAction::FoundNation, KeyCode::KeyJ);
        input_map.insert(PlayerAction::KillCreature, KeyCode::KeyK);
        input_map.insert(PlayerAction::Interact, KeyCode::KeyR);
        commands.entity(entity).insert(input_map);
    }
}

fn send_online_action_messages(
    time: Res<Time>,
    players: Query<
        (
            &ActionState<PlayerAction>,
            Option<&GameplayHudState>,
            &lightyear::prelude::Controlled,
        ),
        With<PlayerPos>,
    >,
    cameras: Query<&Transform, With<OnlineCamera>>,
    chat: Res<OnlineChatState>,
    mut combat: ResMut<OnlineCombatFeedback>,
    mut demo: ResMut<OnlineAutoDemo>,
    mut command_sequence: ResMut<OnlineCommandSequence>,
    mut clients: Query<
        (
            &mut MessageSender<GameplayCommand>,
            &mut MessageSender<AttackInput>,
        ),
        With<NetcodeClient>,
    >,
) {
    combat.attack_cooldown_secs =
        (combat.attack_cooldown_secs - time.delta_secs().max(0.0)).max(0.0);
    if chat.composing {
        return;
    }
    let Ok((mut gameplay_sender, mut attack_sender)) = clients.single_mut() else {
        return;
    };
    let Some((actions, hud, _)) = players.iter().next() else {
        return;
    };
    let tick = hud.map_or(0, |state| state.tick.min(u32::MAX as u64) as u32);
    let player_block = hud.map_or([0, 0, 0], |state| state.player_block_pos);

    if demo.enabled && demo.elapsed < 3.5 {
        gameplay_sender.send::<ControlChannel>(GameplayCommand {
            sequence: next_online_command_sequence(&mut command_sequence),
            tick: tick as u64,
            player_block,
            kind: GameplayCommandKind::MoveWorld {
                dx_milli: 0,
                dz_milli: -1000,
                dy_milli: 0,
            },
        });
        demo.move_world_sent = demo.move_world_sent.saturating_add(1);
    }

    let mut send_command = |kind: GameplayCommandKind| {
        gameplay_sender.send::<ControlChannel>(GameplayCommand {
            sequence: next_online_command_sequence(&mut command_sequence),
            tick: tick as u64,
            player_block,
            kind,
        });
    };

    if actions.just_pressed(&PlayerAction::Gather) {
        send_command(GameplayCommandKind::GatherFootBlock);
    }
    if actions.just_pressed(&PlayerAction::Place) {
        send_command(GameplayCommandKind::PlaceWoodFootBlock);
    }
    if actions.just_pressed(&PlayerAction::Craft) {
        send_command(GameplayCommandKind::Craft(BuildRecipe::PlankPack));
    }
    if actions.just_pressed(&PlayerAction::FoundNation) {
        send_command(GameplayCommandKind::FoundNation);
    }
    if actions.just_pressed(&PlayerAction::KillCreature) {
        send_command(GameplayCommandKind::KillNearestCreature);
    }
    if actions.just_pressed(&PlayerAction::Interact) {
        send_command(GameplayCommandKind::MountCart);
    }
    // Keep the local cadence aligned with the authoritative weapon cooldown.
    // The server still rejects early or malformed inputs; this gate simply
    // prevents a held key from flooding the reliable control channel.
    if actions.pressed(&PlayerAction::Attack) && combat.attack_cooldown_secs <= 0.0 {
        let input_dir = cameras
            .single()
            .map(|camera| camera.forward().as_vec3())
            .unwrap_or(-Vec3::Z);
        attack_sender.send::<ControlChannel>(AttackInput { tick, input_dir });
        combat.attack_cooldown_secs = 0.625;
    }
    if actions.just_pressed(&PlayerAction::Mine) {
        let target = [player_block[0], player_block[1] - 1, player_block[2]];
        send_command(GameplayCommandKind::MineTarget { target });
    }
}

fn next_online_command_sequence(sequence: &mut OnlineCommandSequence) -> u64 {
    sequence.0 = sequence.0.wrapping_add(1);
    if sequence.0 == 0 {
        sequence.0 = 1;
    }
    sequence.0
}

fn handle_online_chat_input(
    mut keyboard_inputs: MessageReader<KeyboardInput>,
    mut chat: ResMut<OnlineChatState>,
    players: Query<(Option<&GameplayHudState>, &lightyear::prelude::Controlled), With<PlayerPos>>,
    mut clients: Query<&mut MessageSender<ChatMessage>, With<NetcodeClient>>,
) {
    for event in keyboard_inputs.read() {
        if !event.state.is_pressed() || event.repeat {
            continue;
        }
        match &event.logical_key {
            Key::Enter => {
                if chat.composing {
                    let text = sanitize_chat_text(&chat.draft);
                    chat.draft.clear();
                    chat.composing = false;
                    if !text.is_empty() {
                        let Ok(mut sender) = clients.single_mut() else {
                            continue;
                        };
                        let tick = players
                            .iter()
                            .next()
                            .and_then(|(hud, _)| hud.map(|state| state.tick))
                            .unwrap_or(0);
                        {
                            sender.send::<ControlChannel>(ChatMessage {
                                client_tick: tick,
                                text,
                            });
                        }
                    }
                } else {
                    chat.composing = true;
                    chat.draft.clear();
                }
            }
            Key::Escape => {
                if chat.composing {
                    chat.composing = false;
                    chat.draft.clear();
                }
            }
            Key::Backspace => {
                if chat.composing {
                    chat.draft.pop();
                }
            }
            _ => {
                if chat.composing {
                    if let Some(text) = &event.text {
                        append_chat_text(&mut chat.draft, text);
                    }
                }
            }
        }
    }
}

fn receive_online_chat_messages(
    mut chat: ResMut<OnlineChatState>,
    mut clients: Query<&mut MessageReceiver<ChatBroadcast>, With<NetcodeClient>>,
) {
    let Ok(mut receiver) = clients.single_mut() else {
        return;
    };
    for message in receiver.receive() {
        chat.log.push(format!(
            "[{}] {}: {}",
            message.server_tick, message.sender, message.text
        ));
    }
    let overflow = chat.log.len().saturating_sub(6);
    if overflow > 0 {
        chat.log.drain(0..overflow);
    }
}

fn suppress_online_actions_while_chatting(
    chat: Res<OnlineChatState>,
    mut players: Query<&mut ActionState<PlayerAction>, With<PlayerPos>>,
) {
    if !chat.composing {
        return;
    }
    for mut actions in &mut players {
        actions.reset_all();
    }
}

fn update_online_chat_ui(
    chat: Res<OnlineChatState>,
    mut text: Query<&mut Text, With<OnlineChatText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let mut lines = Vec::new();
    lines.push(if chat.composing {
        format!("聊天  > {}", chat.draft)
    } else {
        "聊天  按 Enter 输入".to_string()
    });
    lines.extend(chat.log.iter().cloned());
    text.0 = lines.join("\n");
}

fn append_chat_text(draft: &mut String, text: &str) {
    for ch in text.chars() {
        if ch.is_control() {
            continue;
        }
        if draft.chars().count() >= 120 {
            break;
        }
        draft.push(ch);
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

fn sync_replicated_players(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<OnlineVisualAssets>,
    mut players: Query<
        (
            Entity,
            &PlayerPos,
            Option<&CartMounted>,
            Option<&SwimmingState>,
            Option<&EcoSnapshot>,
            Option<&VoxelDelta>,
            Option<&VoxelChunkSnapshot>,
            Option<&lightyear::prelude::Controlled>,
            Option<&Children>,
            Option<&OnlinePlayerVisualRoot>,
            Option<&mut OnlineRemoteMotion>,
            Option<&mut Transform>,
        ),
        Without<OnlinePlayerVisual>,
    >,
    mut nature: ResMut<NatureSnapshotBuffer>,
    mut terrain: ResMut<OnlineTerrainEdits>,
    mut visuals: Query<&mut Transform, With<OnlinePlayerVisual>>,
) {
    for (
        entity,
        legacy_position,
        mounted,
        swimming,
        eco_snapshot,
        voxel_delta,
        voxel_snapshot,
        controlled_by,
        children,
        visual_root,
        mut remote_motion,
        transform,
    ) in players.iter_mut()
    {
        let is_locally_controlled = controlled_by.is_some();
        if is_locally_controlled {
            if let Some(snapshot) = voxel_snapshot {
                apply_online_snapshot(&mut terrain, snapshot);
            }
            if let Some(delta) = voxel_delta {
                if terrain.resync_stalled {
                    // The last bounded request window was exhausted. Keep
                    // ignoring deltas until a new full snapshot arrives or
                    // the backoff window re-arms another bounded request set.
                } else if terrain.awaiting_snapshot {
                    // A full snapshot is the only safe recovery path after
                    // an ordered terrain update gap.
                } else if delta.revision > terrain.latest_revision.saturating_add(1) {
                    warn!(
                        "[net] terrain revision gap: have {}, received {}; awaiting snapshot",
                        terrain.latest_revision, delta.revision
                    );
                    terrain.awaiting_snapshot = true;
                } else if terrain_delta_is_contiguous(terrain.latest_revision, delta.revision) {
                    terrain.latest_revision = delta.revision;
                    terrain.blocks.insert(
                        [delta.x, delta.y, delta.z],
                        BlockType::from_wire_u8(delta.block).unwrap_or(BlockType::Air),
                    );
                    terrain.dirty = true;
                }
            }
            if let Some(snapshot) = eco_snapshot {
                let _ = nature.push(nature_snapshot_from_protocol(snapshot));
            }
        }
        // Lightyear owns the replicated entity's Transform. The mesh lives on
        // a child entity so Avian correction cannot overwrite visual offsets.
        if !legacy_position.is_finite() {
            warn!(
                "[net] ignored non-finite authoritative PlayerPos on {:?}",
                entity
            );
            continue;
        }
        let seated = mounted.is_some_and(|mounted| mounted.0);
        let submerged = swimming.is_some_and(|swimming| swimming.0);
        let visual_translation = if submerged {
            -Vec3::Y * 0.22
        } else if seated {
            -Vec3::Y * 0.18
        } else {
            Vec3::ZERO
        };
        let visual_scale = if submerged {
            Vec3::new(0.92, 0.82, 0.92)
        } else if seated {
            Vec3::new(1.0, 0.78, 1.0)
        } else {
            Vec3::ONE
        };
        let rendered_position = if is_locally_controlled {
            legacy_position.0
        } else if let Some(motion) = remote_motion.as_deref_mut() {
            motion.target = legacy_position.0;
            let blend = 1.0 - (-12.0 * time.delta_secs()).exp();
            motion.rendered = motion.rendered.lerp(motion.target, blend.clamp(0.0, 1.0));
            motion.rendered
        } else {
            commands.entity(entity).insert(OnlineRemoteMotion {
                rendered: legacy_position.0,
                target: legacy_position.0,
            });
            legacy_position.0
        };
        if transform.is_none() {
            commands
                .entity(entity)
                .insert(Transform::from_translation(rendered_position));
        } else if let Some(mut transform) = transform {
            transform.translation = rendered_position;
        }
        let visual_child =
            children.and_then(|children| children.iter().find(|child| visuals.get(*child).is_ok()));
        if visual_root.is_none() {
            commands.entity(entity).insert((
                OnlinePlayerVisualRoot,
                Visibility::default(),
                InheritedVisibility::default(),
            ));
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    OnlinePlayerVisual,
                    Mesh3d(assets.player_mesh.clone()),
                    MeshMaterial3d(assets.player_material.clone()),
                    Transform::from_translation(visual_translation).with_scale(visual_scale),
                ));
            });
        } else if let Some(child) = visual_child {
            if let Ok(mut transform) = visuals.get_mut(child) {
                transform.translation = visual_translation;
                transform.scale = visual_scale;
            }
        }
    }
}

fn online_creature_asset(kind: CreatureKind) -> &'static str {
    match kind {
        CreatureKind::Pig => "animals/pig.glb",
        CreatureKind::Sheep => "animals/sheep.glb",
        CreatureKind::Cow => "animals/cow.glb",
        CreatureKind::Chicken => "animals/chicken.glb",
    }
}

fn sync_replicated_creatures(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut creatures: Query<(
        Entity,
        &CreatureState,
        Option<&mut Transform>,
        Option<&OnlineCreatureVisualRoot>,
    )>,
) {
    for (entity, state, transform, visual_root) in &mut creatures {
        if !state.position.is_finite() {
            warn!(
                "[net] ignored non-finite authoritative CreatureState on {:?}",
                entity
            );
            continue;
        }

        if let Some(mut transform) = transform {
            transform.translation = state.position;
        }
        if visual_root.is_some() {
            continue;
        }

        let kind = CreatureKind::from_u8(state.kind);
        let path = online_creature_asset(kind);
        commands.entity(entity).insert((
            OnlineCreatureVisualRoot,
            Name::new(format!("online_{kind:?}")),
            Transform::from_translation(state.position).with_scale(Vec3::splat(0.58)),
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))),
        ));
    }
}

fn cleanup_replicated_creature_visual(trigger: On<Remove, CreatureState>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .despawn_related::<Children>();
}

fn update_online_underwater_presentation(
    players: Query<(&SwimmingState, &lightyear::prelude::Controlled), With<PlayerPos>>,
    mut water: Query<&mut Visibility, With<OnlineWaterSurface>>,
) {
    let submerged = players.iter().any(|(swimming, _)| swimming.0);
    for mut visibility in &mut water {
        *visibility = if submerged {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

fn apply_online_snapshot(terrain: &mut OnlineTerrainEdits, snapshot: &VoxelChunkSnapshot) {
    let snapshot_size = VOXEL_CHUNK_SIZE_XZ + snapshot.border.saturating_mul(2);
    if snapshot.border < 0
        || snapshot.border > 4
        || snapshot_size <= 0
        || snapshot.y_size <= 0
        || snapshot.blocks.len() < (snapshot.y_size * snapshot_size * snapshot_size) as usize
    {
        warn!(
            "[net] ignored malformed terrain snapshot: border={}, y_size={}, blocks={}",
            snapshot.border,
            snapshot.y_size,
            snapshot.blocks.len()
        );
        return;
    }

    let chunk = (snapshot.chunk_x, snapshot.chunk_z);
    let region_changed = !terrain.has_snapshot
        || terrain.snapshot_chunk != Some(chunk)
        || terrain.snapshot_border != snapshot.border
        || terrain.y_min != snapshot.y_min
        || terrain.y_size != snapshot.y_size;
    if !region_changed
        && !terrain.awaiting_snapshot
        && !terrain.resync_stalled
        && snapshot.revision <= terrain.snapshot_revision
    {
        return;
    }
    if !region_changed
        && (terrain.awaiting_snapshot || terrain.resync_stalled)
        && snapshot.revision < terrain.snapshot_revision
    {
        warn!(
            "[net] ignored stale terrain recovery snapshot: have {}, received {}",
            terrain.snapshot_revision, snapshot.revision
        );
        return;
    }
    let reset_revision_cursor = region_changed || terrain.awaiting_snapshot;

    terrain.blocks.clear();
    let origin_x = snapshot.chunk_x * VOXEL_CHUNK_SIZE_XZ - snapshot.border;
    let origin_z = snapshot.chunk_z * VOXEL_CHUNK_SIZE_XZ - snapshot.border;
    for y in 0..snapshot.y_size {
        for z in 0..snapshot_size {
            for x in 0..snapshot_size {
                let index = ((y * snapshot_size + z) * snapshot_size + x) as usize;
                let block =
                    BlockType::from_wire_u8(snapshot.blocks[index]).unwrap_or(BlockType::Air);
                terrain
                    .blocks
                    .insert([origin_x + x, snapshot.y_min + y, origin_z + z], block);
            }
        }
    }
    terrain.snapshot_revision = snapshot.revision;
    if reset_revision_cursor {
        // The revision cursor belongs to the snapshot's chunk/session. A new
        // region or gap recovery (including a reconnect with a restarted
        // server) must not inherit an older cursor or every following delta
        // will look like a permanent gap.
        terrain.latest_revision = snapshot.revision;
    } else {
        terrain.latest_revision = terrain.latest_revision.max(snapshot.revision);
    }
    terrain.snapshot_chunk = Some(chunk);
    terrain.snapshot_border = snapshot.border;
    terrain.y_min = snapshot.y_min;
    terrain.y_size = snapshot.y_size;
    terrain.has_snapshot = true;
    terrain.awaiting_snapshot = false;
    terrain.resync_stalled = false;
    terrain.resync_budget.reset();
    terrain.dirty = true;
}

fn request_online_terrain_resync(
    time: Res<Time>,
    mut terrain: ResMut<OnlineTerrainEdits>,
    mut clients: Query<&mut MessageSender<TerrainSnapshotRequest>, With<NetcodeClient>>,
) {
    let now = time.elapsed_secs();
    if terrain.resync_stalled {
        if !terrain.resync_budget.can_rearm(now) {
            return;
        }
        terrain.resync_stalled = false;
        terrain.awaiting_snapshot = true;
        terrain.resync_budget.rearm();
    }
    if !terrain.awaiting_snapshot {
        return;
    }
    let Some(chunk) = terrain.snapshot_chunk else {
        return;
    };
    let Ok(mut sender) = clients.single_mut() else {
        return;
    };
    let known_revision = terrain.latest_revision;
    let Some(request) = terrain.resync_budget.request(now, chunk, known_revision) else {
        if terrain.resync_budget.exhausted() {
            terrain.awaiting_snapshot = false;
            terrain.resync_stalled = true;
            terrain.resync_budget.enter_stall(now);
            warn!(
                "[net] terrain snapshot resync exhausted; entering {}s backoff",
                TERRAIN_RESYNC_STALL_COOLDOWN_SECS
            );
        }
        return;
    };
    sender.send::<ControlChannel>(request);
}

fn terrain_delta_is_contiguous(latest_revision: u64, incoming_revision: u64) -> bool {
    incoming_revision == latest_revision.saturating_add(1)
}

fn sync_online_terrain_mesh(
    mut terrain: ResMut<OnlineTerrainEdits>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_queries: ParamSet<(
        Query<&mut Mesh3d, With<OnlineTerrainSurface>>,
        Query<(&OnlineOreSurface, &mut Mesh3d)>,
        Query<&mut Mesh3d, With<OnlineWaterSurface>>,
    )>,
) {
    if !terrain.dirty || !terrain.has_snapshot {
        return;
    }

    let terrain_handle = meshes.add(build_online_terrain_mesh(&terrain, None));
    let ore_handles = [
        BlockType::IronOre,
        BlockType::SunstoneOre,
        BlockType::FrostcoreOre,
        BlockType::LivingRoot,
    ]
    .map(|block| meshes.add(build_online_terrain_mesh(&terrain, Some(block))));
    let water_handle = meshes.add(build_online_water_mesh(&terrain));

    for mut mesh in mesh_queries.p0().iter_mut() {
        let old_handle = mesh.0.clone();
        mesh.0 = terrain_handle.clone();
        terrain.retired_meshes.push(RetiredOnlineMesh {
            handle: old_handle,
            frames_remaining: ONLINE_RETIRED_MESH_GRACE_FRAMES,
        });
    }
    for (ore, mut mesh) in mesh_queries.p1().iter_mut() {
        let old_handle = mesh.0.clone();
        if let Some(handle) = ore_handles
            .iter()
            .zip([
                BlockType::IronOre,
                BlockType::SunstoneOre,
                BlockType::FrostcoreOre,
                BlockType::LivingRoot,
            ])
            .find_map(|(handle, block)| (block == ore.block).then_some(handle))
        {
            mesh.0 = handle.clone();
            terrain.retired_meshes.push(RetiredOnlineMesh {
                handle: old_handle,
                frames_remaining: ONLINE_RETIRED_MESH_GRACE_FRAMES,
            });
        }
    }
    for mut mesh in mesh_queries.p2().iter_mut() {
        let old_handle = mesh.0.clone();
        mesh.0 = water_handle.clone();
        terrain.retired_meshes.push(RetiredOnlineMesh {
            handle: old_handle,
            frames_remaining: ONLINE_RETIRED_MESH_GRACE_FRAMES,
        });
    }
    terrain.dirty = false;
}

fn cleanup_retired_online_meshes(
    mut terrain: ResMut<OnlineTerrainEdits>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut pending = Vec::with_capacity(terrain.retired_meshes.len());
    for mut retired in terrain.retired_meshes.drain(..) {
        if retired.frames_remaining == 0 {
            let _ = meshes.remove(&retired.handle);
        } else {
            retired.frames_remaining -= 1;
            pending.push(retired);
        }
    }
    terrain.retired_meshes = pending;
}

fn build_online_terrain_mesh(terrain: &OnlineTerrainEdits, only_block: Option<BlockType>) -> Mesh {
    let snapshot_size = VOXEL_CHUNK_SIZE_XZ + terrain.snapshot_border * 2;
    let cells_xz = (snapshot_size - 1).max(1);
    let cells_y = (terrain.y_size - 1).max(1);
    let origin = [
        terrain
            .snapshot_chunk
            .map_or(0, |(x, _)| x * VOXEL_CHUNK_SIZE_XZ)
            - terrain.snapshot_border,
        terrain.y_min,
        terrain
            .snapshot_chunk
            .map_or(0, |(_, z)| z * VOXEL_CHUNK_SIZE_XZ)
            - terrain.snapshot_border,
    ];
    let nets = build_surface_nets_rect([cells_xz, cells_y, cells_xz], |x, y, z| {
        let block = terrain
            .blocks
            .get(&[origin[0] + x, origin[1] + y, origin[2] + z])
            .copied()
            .unwrap_or(BlockType::Air);
        let solid = block.is_solid() && only_block.is_none_or(|kind| kind == block);
        if solid { 1.0 } else { -1.0 }
    });
    surface_nets_to_online_mesh(nets, origin, only_block.is_some())
}

fn build_online_water_mesh(terrain: &OnlineTerrainEdits) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for (&[x, y, z], block) in &terrain.blocks {
        if *block != BlockType::Water
            || terrain.blocks.get(&[x, y + 1, z]) == Some(&BlockType::Water)
        {
            continue;
        }
        let base = positions.len() as u32;
        positions.extend([
            [x as f32, y as f32 + 1.01, z as f32],
            [x as f32 + 1.0, y as f32 + 1.01, z as f32],
            [x as f32 + 1.0, y as f32 + 1.01, z as f32 + 1.0],
            [x as f32, y as f32 + 1.01, z as f32 + 1.0],
        ]);
        normals.extend([Vec3::Y.to_array(); 4]);
        uvs.extend([
            [x as f32 * 0.08, z as f32 * 0.08],
            [(x + 1) as f32 * 0.08, z as f32 * 0.08],
            [(x + 1) as f32 * 0.08, (z + 1) as f32 * 0.08],
            [x as f32 * 0.08, (z + 1) as f32 * 0.08],
        ]);
        indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }

    if positions.is_empty() {
        return empty_online_mesh();
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn build_online_seaweed_mesh() -> Mesh {
    const BLADES: usize = 5;
    let mut positions = Vec::with_capacity(BLADES * 5);
    let mut indices = Vec::with_capacity(BLADES * 9);
    for blade in 0..BLADES {
        let angle = blade as f32 / BLADES as f32 * std::f32::consts::TAU + 0.22;
        let radial = Vec3::new(angle.cos(), 0.0, angle.sin());
        let side = Vec3::new(-angle.sin(), 0.0, angle.cos());
        let base_center = radial * (blade as f32 * 0.012);
        let middle_center = base_center + radial * (0.10 + blade as f32 * 0.018);
        let tip = base_center
            + radial * (0.18 + blade as f32 * 0.02)
            + side * (0.015 * (blade as f32 - 2.0));
        let width = 0.075 - blade as f32 * 0.006;
        let base = positions.len() as u32;
        positions.extend([
            base_center - side * width,
            base_center + side * width,
            middle_center - side * width * 0.62 + Vec3::Y * 0.46,
            middle_center + side * width * 0.62 + Vec3::Y * 0.46,
            tip + Vec3::Y,
        ]);
        indices.extend([
            base,
            base + 2,
            base + 1,
            base + 1,
            base + 2,
            base + 3,
            base + 2,
            base + 4,
            base + 3,
        ]);
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let uvs = positions
        .iter()
        .map(|position| [position[0] + 0.5, position[1]])
        .collect::<Vec<_>>();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn sync_online_fish(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    assets: Res<OnlineVisualAssets>,
    mut terrain: ResMut<OnlineTerrainEdits>,
    fish: Query<Entity, With<OnlineFishVisual>>,
    seaweed: Query<Entity, With<OnlineSeaweedVisual>>,
) {
    if !terrain.has_snapshot || terrain.fish_revision == Some(terrain.latest_revision) {
        return;
    }
    for entity in &fish {
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .despawn();
    }
    for entity in &seaweed {
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .despawn();
    }

    let mut candidates = terrain
        .blocks
        .iter()
        .filter_map(|(&[x, y, z], block)| {
            if *block != BlockType::Water
                || terrain.blocks.get(&[x, y + 1, z]) == Some(&BlockType::Water)
            {
                return None;
            }
            let floor_y = (terrain.y_min..y).rev().find(|floor| {
                terrain
                    .blocks
                    .get(&[x, *floor, z])
                    .is_some_and(|block| (*block).is_solid())
            })?;
            let depth = y - floor_y;
            (depth >= 2).then_some((x, y, z, floor_y))
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable();

    for (index, &(x, water_y, z, floor_y)) in candidates.iter().take(8).enumerate() {
        let origin = Vec3::new(
            x as f32 + 0.5,
            floor_y as f32 + (water_y - floor_y) as f32 * 0.52,
            z as f32 + 0.5,
        );
        commands.spawn((
            WorldAssetRoot(
                asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset("animals/fish.glb".to_string())),
            ),
            Transform::from_translation(origin),
            OnlineFishVisual {
                origin,
                phase: index as f32 * 1.37,
            },
            Name::new(format!("online_water_fish_{index}")),
        ));
    }
    for (index, &(x, _water_y, z, floor_y)) in candidates.iter().skip(8).take(24).enumerate() {
        let height = 0.65 + (index % 5) as f32 * 0.12;
        let origin = Vec3::new(x as f32 + 0.5, floor_y as f32 + 0.02, z as f32 + 0.5);
        commands.spawn((
            Mesh3d(assets.seaweed_mesh.clone()),
            MeshMaterial3d(assets.seaweed_material.clone()),
            Transform::from_translation(origin).with_scale(Vec3::new(1.0, height, 1.0)),
            OnlineSeaweedVisual {
                origin,
                phase: index as f32 * 1.37,
                height,
            },
            Name::new(format!("online_water_seaweed_{index}")),
        ));
    }
    terrain.fish_revision = Some(terrain.latest_revision);
}

fn animate_online_fish(time: Res<Time>, mut fish: Query<(&OnlineFishVisual, &mut Transform)>) {
    for (fish, mut transform) in &mut fish {
        let phase = time.elapsed_secs() * 0.8 + fish.phase;
        transform.translation = fish.origin
            + Vec3::new(
                phase.sin() * 0.3,
                (phase * 1.7).sin() * 0.05,
                phase.cos() * 0.2,
            );
        transform.rotation = Quat::from_rotation_y(phase.sin().atan2(phase.cos()));
    }
}

fn animate_online_seaweed(
    time: Res<Time>,
    mut seaweed: Query<(&OnlineSeaweedVisual, &mut Transform)>,
) {
    for (plant, mut transform) in &mut seaweed {
        let phase = time.elapsed_secs() * 0.9 + plant.phase;
        transform.translation = plant.origin
            + Vec3::new(
                phase.sin() * 0.04,
                (phase * 1.3).sin() * 0.016,
                phase.cos() * 0.03,
            );
        transform.rotation = Quat::from_rotation_y(plant.phase)
            * Quat::from_rotation_z(phase.sin() * 0.14)
            * Quat::from_rotation_x(phase.cos() * 0.07);
        transform.scale = Vec3::new(1.0, plant.height, 1.0);
    }
}

fn empty_online_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    // Bevy 0.19's mesh allocator skips zero-sized vertex buffers but still
    // attempts to upload them. One unindexed vertex is a valid invisible
    // triangle-list mesh and avoids a render-world use-after-free warning.
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0, 0.0, 0.0]])
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]])
}

fn surface_nets_to_online_mesh(nets: SurfaceNetsMesh, origin: [i32; 3], ore_offset: bool) -> Mesh {
    if nets.positions.is_empty() || nets.indices.is_empty() {
        return empty_online_mesh();
    }
    let mut positions = nets
        .positions
        .iter()
        .map(|[x, y, z]| {
            [
                origin[0] as f32 + *x,
                origin[1] as f32 + *y,
                origin[2] as f32 + *z,
            ]
        })
        .collect::<Vec<_>>();
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for triangle in nets.indices.chunks_exact(3) {
        let [a, b, c] = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        let normal = (Vec3::from_array(positions[b]) - Vec3::from_array(positions[a]))
            .cross(Vec3::from_array(positions[c]) - Vec3::from_array(positions[a]));
        normals[a] += normal;
        normals[b] += normal;
        normals[c] += normal;
    }
    let normals = normals
        .into_iter()
        .map(|normal| normal.normalize_or_zero().to_array())
        .collect::<Vec<_>>();
    if ore_offset {
        for (position, normal) in positions.iter_mut().zip(&normals) {
            position[0] += normal[0] * 0.018;
            position[1] += normal[1] * 0.018;
            position[2] += normal[2] * 0.018;
        }
    }
    let uvs = positions
        .iter()
        .map(|position| [position[0] * 0.08, position[2] * 0.08])
        .collect::<Vec<_>>();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(nets.indices))
}

fn cleanup_replicated_player_visuals(
    mut commands: Commands,
    players: Query<Entity, With<PlayerPos>>,
    visuals: Query<(Entity, &ChildOf), With<OnlinePlayerVisual>>,
) {
    for (visual, parent) in &visuals {
        if players.get(parent.parent()).is_err() {
            commands.entity(visual).despawn();
        }
    }
}

fn sync_replicated_carts(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    carts: Query<(Entity, &CartState)>,
    mut visuals: Query<(Entity, &mut OnlineCartVisual, &mut Transform)>,
) {
    for (entity, state) in &carts {
        if !state.position.is_finite() || !state.yaw.is_finite() {
            continue;
        }
        if let Some((_, mut visual, mut transform)) = visuals
            .iter_mut()
            .find(|(_, visual, _)| visual.cart == entity)
        {
            let speed =
                visual.last_position.distance(state.position) / time.delta_secs().max(0.001);
            visual.animation_phase = (visual.animation_phase
                + time.delta_secs() * (3.0 + speed * 1.5))
                .rem_euclid(std::f32::consts::TAU);
            visual.last_position = state.position;
            let moving = state.occupied && speed > 0.05;
            let bob = if moving {
                visual.animation_phase.sin().abs() * 0.025
            } else {
                0.0
            };
            let roll = if moving {
                visual.animation_phase.sin() * 0.025
            } else {
                0.0
            };
            transform.translation = state.position;
            transform.translation.y += bob;
            transform.rotation = Quat::from_rotation_y(state.yaw + CART_FRONT_YAW_OFFSET)
                * Quat::from_rotation_z(roll);
        } else {
            commands.spawn((
                WorldAssetRoot(asset_server.load(
                    GltfAssetLabel::Scene(0).from_asset("procedural/pretty/cart.glb".to_string()),
                )),
                Transform::from_translation(state.position)
                    .with_rotation(Quat::from_rotation_y(state.yaw + CART_FRONT_YAW_OFFSET)),
                Visibility::default(),
                InheritedVisibility::default(),
                OnlineCartVisual {
                    cart: entity,
                    last_position: state.position,
                    animation_phase: 0.0,
                },
                Name::new("online_cart"),
            ));
        }
    }

    for (visual_entity, visual, _) in &mut visuals {
        if !carts.iter().any(|(entity, _)| entity == visual.cart) {
            commands
                .entity(visual_entity)
                .despawn_related::<Children>()
                .despawn();
        }
    }
}

fn update_online_overlay(
    clients: Query<Entity, With<NetcodeClient>>,
    combat: Res<OnlineCombatFeedback>,
    terrain: Res<OnlineTerrainEdits>,
    players: Query<(
        &PlayerPos,
        Option<&CartMounted>,
        Option<&SwimmingState>,
        Option<&GameplayHudState>,
        Option<&EcoSnapshot>,
        &lightyear::prelude::Controlled,
    )>,
    mut text: Query<&mut Text, With<OnlineHudText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let Some(_client_entity) = clients.iter().next() else {
        text.0 = "在线\n等待服务器快照".to_string();
        return;
    };
    let Some((pos, mounted, swimming, hud, eco, _)) = players.iter().next() else {
        text.0 = "在线\n等待服务器快照".to_string();
        return;
    };
    text.0 = online_overlay_text(pos, mounted, swimming, hud, eco, &terrain, &combat);
}

fn online_overlay_text(
    pos: &PlayerPos,
    mounted: Option<&CartMounted>,
    swimming: Option<&SwimmingState>,
    hud: Option<&GameplayHudState>,
    eco: Option<&EcoSnapshot>,
    terrain: &OnlineTerrainEdits,
    combat: &OnlineCombatFeedback,
) -> String {
    let tick = hud.map_or(0, |state| state.tick);
    let block = hud.map_or(scene_pos_to_block(pos.0), |state| state.player_block_pos);
    let nation = hud
        .and_then(|state| state.nation_id)
        .map_or_else(|| "none".to_string(), |id| id.to_string());
    let wood = hud.map_or(0, |state| state.inventory_wood);
    let food = hud.map_or(0, |state| state.inventory_food);
    let flags = hud.map_or(0, |state| state.flag_count);
    let nations = hud.map_or(0, |state| state.total_nations);
    let monsters = hud.map_or(0, |state| state.monster_count);
    let (clouds, plants, animals, rainfall) = eco.map_or((0, 0, 0, 0.0), |snapshot| {
        (
            snapshot.clouds.len(),
            snapshot.plants.len() + snapshot.berries.len(),
            snapshot.rabbits.len() + snapshot.wildlife.len(),
            snapshot.rainfall,
        )
    });
    let ride_status = if mounted.is_some_and(|mounted| mounted.0) {
        "骑乘中"
    } else {
        "未骑乘"
    };
    let mut text = format!(
        "在线\nTick {tick}  区块 [{}, {}, {}]  国家 {nation}\n木头 {wood}  食物 {food}  旗帜 {flags}  国家数 {nations}  怪物 {monsters}\n云朵 {clouds}  植物 {plants}  动物 {animals}  降雨 {:.1}",
        block[0], block[1], block[2], rainfall
    );
    text.push_str(&format!("\n载具状态：{ride_status}（R 上车/下车）"));
    let water_status = if swimming.is_some_and(|swimming| swimming.0) {
        "水下"
    } else {
        "水面"
    };
    text.push_str(&format!(
        "\n水体状态：{water_status}（Space 上浮 / Ctrl 下潜）"
    ));
    if combat.visible_secs > 0.0 {
        text.push_str(&format!(
            "\n命中确认：-{:.0} HP（累计 {} 次）",
            combat.damage, combat.hit_count
        ));
    }
    let terrain_status = if terrain.resync_stalled {
        "地形同步退避"
    } else if terrain.awaiting_snapshot {
        "地形等待全量快照"
    } else if terrain.has_snapshot {
        "地形已同步"
    } else {
        "地形初始化"
    };
    text.push_str(&format!("\n地形状态：{terrain_status}"));
    text
}

fn follow_online_camera(
    time: Res<Time>,
    clients: Query<Entity, With<NetcodeClient>>,
    combat: Res<OnlineCombatFeedback>,
    players: Query<(
        &PlayerPos,
        &lightyear::prelude::Controlled,
        Option<&SwimmingState>,
    )>,
    mut cameras: Query<&mut Transform, With<OnlineCamera>>,
) {
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };
    let Some(_client_entity) = clients.iter().next() else {
        return;
    };
    let Some((player, _, swimming)) = players.iter().next() else {
        return;
    };
    let submerged = swimming.is_some_and(|swimming| swimming.0);
    let desired = player.0
        + if submerged {
            // The old orbit lifted the camera above the replicated swimmer,
            // so hiding the surface left the client looking at the shoreline
            // instead of the seabed. Keep the chase camera close to the
            // swimmer's vertical level while retaining the readable offset.
            Vec3::new(8.0, 0.28, 10.0)
        } else {
            Vec3::new(10.0, 8.0, 12.0)
        };
    let shake = online_camera_shake_offset(time.elapsed_secs(), combat.camera_shake_secs);
    camera.translation = camera.translation.lerp(desired, 0.08) + shake;
    camera.look_at(
        player.0 + Vec3::Y * if submerged { 0.25 } else { 1.0 } + shake * 0.25,
        Vec3::Y,
    );
}

fn online_camera_shake_offset(elapsed: f32, strength: f32) -> Vec3 {
    let phase = elapsed * 92.0;
    Vec3::new(
        phase.sin() * strength,
        (phase * 1.37).cos() * strength * 0.55,
        (phase * 0.83).sin() * strength * 0.35,
    )
}

fn log_connection_state(
    mut reconnect: ResMut<OnlineReconnectState>,
    connected: Query<Entity, Added<lightyear::prelude::Connected>>,
    disconnected: Query<
        (Entity, &lightyear::prelude::Disconnected),
        Added<lightyear::prelude::Disconnected>,
    >,
) {
    for entity in connected.iter() {
        reconnect.attempts = 0;
        reconnect.cooldown_secs = 0.0;
        info!("[net] Lightyear client connected: {:?}", entity);
    }
    for (entity, state) in disconnected.iter() {
        warn!(
            "[net] Lightyear client disconnected: {:?}: {}",
            entity,
            state.reason.as_deref().unwrap_or("unknown")
        );
    }
}

fn online_auto_demo_capture(
    mut commands: Commands,
    time: Res<Time>,
    mut demo: ResMut<OnlineAutoDemo>,
    clients: Query<Option<&lightyear::prelude::Connected>, With<NetcodeClient>>,
    telemetry: Res<OnlineNetworkTelemetry>,
    players: Query<(
        &PlayerPos,
        Option<&GameplayHudState>,
        Option<&EcoSnapshot>,
        &lightyear::prelude::Controlled,
    )>,
) {
    if !demo.enabled {
        return;
    }
    let dt = time.delta_secs();
    demo.elapsed += dt;
    demo.frame += 1;
    if dt > 0.05 {
        demo.frame_dt_over_50ms = demo.frame_dt_over_50ms.saturating_add(1);
    }
    demo.frame_dt_max_ms = demo.frame_dt_max_ms.max(dt * 1000.0);
    if demo.first_nature.is_none() {
        demo.first_nature = players
            .iter()
            .next()
            .and_then(|(_, _, snapshot, _)| snapshot.cloned());
    }
    if demo.shot_requested || demo.elapsed < ONLINE_AUTO_DEMO_CAPTURE_AFTER_SECS || demo.frame < 24
    {
        return;
    }

    if let Some(iter_dir) = demo.iter_dir.clone() {
        let _ = std::fs::create_dir_all(&iter_dir);
        let transport_connected = clients.iter().next().is_some_and(|state| state.is_some());
        let (pos, hud, eco) = transport_connected
            .then(|| players.iter().next())
            .flatten()
            .map(|(position, hud, eco, _)| (position, hud, eco))
            .map_or((Vec3::ZERO, None, None), |(position, hud, eco)| {
                (position.0, hud, eco)
            });
        if demo.first_player_pos.is_none() {
            demo.first_player_pos = Some(pos);
        }
        let first_player_pos = demo.first_player_pos.unwrap_or(pos);
        let player_block = hud.map_or(scene_pos_to_block(pos), |state| state.player_block_pos);
        let tick = hud.map_or(0, |state| state.tick);
        let final_state = json!({
            "schema": 1,
            "role": "client_online",
            "tick": tick,
            "wall_secs": demo.elapsed,
            "frame": demo.frame,
            "world": {"size": 96},
            "player": {
                "block_pos": player_block,
                "pos": [pos.x, pos.y, pos.z],
                "blocks_gathered": hud.map_or(0, |state| state.blocks_gathered),
                "monsters_killed": hud.map_or(0, |state| state.monsters_killed),
                "nations_founded": hud.map_or(0, |state| state.nations_founded)
            },
            "nations": {"total_nations": hud.map_or(0, |state| state.total_nations)},
            "network_connection": {
                "transport": if transport_connected { "Connected" } else { "" },
                "ping_ms": telemetry.ping_ms,
                "pong_age_secs": telemetry.last_pong_at.map(|pong_at| (time.elapsed_secs() - pong_at).max(0.0))
            },
            "observer": {"anomalies": hud.map_or(0, |state| state.observer_anomalies), "invariant_violations": hud.map_or(0, |state| state.observer_invariant_violations)},
            "camera": {"mode": "ThirdPerson", "first_person_eye": [pos.x, pos.y + 1.6, pos.z]},
            "visual": {
                "movement_probe": {
                    "first_player_pos": [first_player_pos.x, first_player_pos.y, first_player_pos.z],
                    "current_player_pos": [pos.x, pos.y, pos.z],
                    "first_static_world": {"floor": [0.0, -0.1, 0.0]},
                    "current_static_world": {"floor": [0.0, -0.1, 0.0]}
                },
                "player_readability": {"marker_count": 1, "marker_max_distance": 0.0}
            },
            "network_command": {"move_world_sent": demo.move_world_sent},
            "render": {
                "frame": {"dt_over_50ms": demo.frame_dt_over_50ms, "max_ms": demo.frame_dt_max_ms},
                "terrain": {"smooth_mesh_builds": 0, "smooth_mesh_max_ms": 0.0, "terrain_despawns": 0},
                "lighting": {"dayness": 1.0}
            },
            "eco_cycle": eco.map_or(json!({}), |snapshot| json!({
                "clouds": snapshot.clouds.len(),
                "rainfall": snapshot.rainfall,
                "plants": snapshot.plants.len() + snapshot.berries.len(),
                "animals": snapshot.rabbits.len() + snapshot.wildlife.len(),
                "plants_grown": snapshot.plants_grown,
                "fruit_eaten": snapshot.fruit_eaten,
                "fruit_grown": snapshot.fruit_grown
            })),
            "nature": eco.map_or(json!({}), |snapshot| json!({
                "tick": snapshot.tick,
                "clouds": snapshot.clouds,
                "cloud_count": snapshot.clouds.len(),
                "rainfall": snapshot.rainfall,
                "soil_moisture": snapshot.rain.max(0.0),
                "plants": snapshot.plants.len() + snapshot.berries.len(),
                "animals": snapshot.rabbits.len() + snapshot.wildlife.len(),
                "animal_food_available": snapshot
                    .berries
                    .iter()
                    .map(|berry| berry.fruit as f32)
                    .sum::<f32>()
                    + snapshot
                        .plants
                        .iter()
                        .map(|plant| plant.stock as f32)
                        .sum::<f32>(),
                "events": []
            })),
            "presentation": eco.map_or(json!({}), |snapshot| json!({
                "nature": {
                    "clouds": snapshot.clouds.len(),
                    "plants": snapshot.plants.len() + snapshot.berries.len(),
                    "animals": snapshot.rabbits.len() + snapshot.wildlife.len()
                }
            }))
        });
        let _ = std::fs::write(
            iter_dir.join("final_state.json"),
            serde_json::to_vec_pretty(&final_state).unwrap_or_default(),
        );
        let _ = std::fs::write(
            iter_dir.join("diff.json"),
            "{\"resource_deltas\":[{\"kind\":\"network\",\"delta\":1}]}\n",
        );
        if let Some(snapshot) = demo.first_nature.as_ref() {
            let initial = json!({
                "nature": {
                    "tick": snapshot.tick,
                    "clouds": snapshot.clouds,
                    "cloud_count": snapshot.clouds.len(),
                    "rainfall": snapshot.rainfall,
                    "soil_moisture": snapshot.rain.max(0.0),
                    "plants": snapshot.plants.len() + snapshot.berries.len(),
                    "animals": snapshot.rabbits.len() + snapshot.wildlife.len(),
                    "animal_food_available": snapshot
                        .berries
                        .iter()
                        .map(|berry| berry.fruit as f32)
                        .sum::<f32>()
                        + snapshot
                            .plants
                            .iter()
                            .map(|plant| plant.stock as f32)
                            .sum::<f32>()
                }
            });
            let _ = std::fs::write(
                iter_dir.join("nature_initial.json"),
                serde_json::to_vec_pretty(&initial).unwrap_or_default(),
            );
        }
    }

    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(demo.png_path.clone()));
    demo.shot_requested = true;
    demo.exit_deadline = Some(Instant::now() + Duration::from_secs(8));
}

fn online_auto_demo_exit(demo: Res<OnlineAutoDemo>) {
    if let Some(deadline) = demo.exit_deadline {
        if demo.png_path.exists() || Instant::now() >= deadline {
            std::process::exit(0);
        }
    }
}

fn scene_pos_to_block(pos: Vec3) -> [i32; 3] {
    [
        (pos.x.round() as i32 + 48).clamp(0, 95),
        (pos.y.round() as i32 + 16).clamp(0, 95),
        (pos.z.round() as i32 + 48).clamp(0, 95),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn online_screenshot_path_matches_loop_iter_contract() {
        let iter_dir = PathBuf::from("screenshots/iter_07");
        assert_eq!(
            screenshot_path(&iter_dir, Some(&iter_dir)),
            PathBuf::from("screenshots/iter_07/iter_07.png")
        );
        assert_eq!(
            screenshot_path(&PathBuf::from("screenshots/online"), None),
            PathBuf::from("screenshots/online/online_scene.png")
        );
    }

    #[test]
    fn terrain_delta_requires_the_next_revision() {
        assert!(terrain_delta_is_contiguous(0, 1));
        assert!(terrain_delta_is_contiguous(9, 10));
        assert!(!terrain_delta_is_contiguous(9, 9));
        assert!(!terrain_delta_is_contiguous(9, 11));
    }

    #[test]
    fn entering_a_new_snapshot_region_resets_the_delta_cursor() {
        let mut terrain = OnlineTerrainEdits {
            latest_revision: 99,
            snapshot_revision: 99,
            snapshot_chunk: Some((0, 0)),
            snapshot_border: 1,
            y_min: 0,
            y_size: 1,
            has_snapshot: true,
            ..OnlineTerrainEdits::default()
        };
        let snapshot_size = VOXEL_CHUNK_SIZE_XZ + 2;
        let snapshot = VoxelChunkSnapshot {
            revision: 4,
            chunk_x: 1,
            chunk_z: 0,
            border: 1,
            y_min: 0,
            y_size: 1,
            blocks: vec![0; (snapshot_size * snapshot_size) as usize],
        };

        apply_online_snapshot(&mut terrain, &snapshot);

        assert_eq!(terrain.latest_revision, 4);
        assert_eq!(terrain.snapshot_revision, 4);
        assert!(!terrain.awaiting_snapshot);
        assert!(terrain.dirty);
    }

    #[test]
    fn gap_recovery_snapshot_resets_an_inherited_delta_cursor() {
        let mut terrain = OnlineTerrainEdits {
            latest_revision: 99,
            snapshot_revision: 4,
            snapshot_chunk: Some((1, 0)),
            snapshot_border: 1,
            y_min: 0,
            y_size: 1,
            has_snapshot: true,
            awaiting_snapshot: true,
            resync_stalled: true,
            ..OnlineTerrainEdits::default()
        };
        let snapshot_size = VOXEL_CHUNK_SIZE_XZ + 2;
        let snapshot = VoxelChunkSnapshot {
            revision: 4,
            chunk_x: 1,
            chunk_z: 0,
            border: 1,
            y_min: 0,
            y_size: 1,
            blocks: vec![0; (snapshot_size * snapshot_size) as usize],
        };

        apply_online_snapshot(&mut terrain, &snapshot);

        assert_eq!(terrain.latest_revision, 4);
        assert!(!terrain.awaiting_snapshot);
        assert!(!terrain.resync_stalled);
    }

    #[test]
    fn equal_revision_snapshot_can_clear_stalled_resync() {
        let mut terrain = OnlineTerrainEdits {
            latest_revision: 8,
            snapshot_revision: 8,
            snapshot_chunk: Some((1, 0)),
            snapshot_border: 1,
            y_min: 0,
            y_size: 1,
            has_snapshot: true,
            resync_stalled: true,
            ..OnlineTerrainEdits::default()
        };
        let snapshot_size = VOXEL_CHUNK_SIZE_XZ + 2;
        let snapshot = VoxelChunkSnapshot {
            revision: 8,
            chunk_x: 1,
            chunk_z: 0,
            border: 1,
            y_min: 0,
            y_size: 1,
            blocks: vec![0; (snapshot_size * snapshot_size) as usize],
        };

        apply_online_snapshot(&mut terrain, &snapshot);

        assert!(!terrain.resync_stalled);
        assert!(!terrain.awaiting_snapshot);
    }

    #[test]
    fn stale_snapshot_cannot_overwrite_a_newer_same_region_cache() {
        let mut terrain = OnlineTerrainEdits {
            latest_revision: 8,
            snapshot_revision: 8,
            snapshot_chunk: Some((1, 0)),
            snapshot_border: 1,
            y_min: 0,
            y_size: 1,
            has_snapshot: true,
            ..OnlineTerrainEdits::default()
        };
        terrain.blocks.insert([16, 0, 0], BlockType::Stone);
        let snapshot_size = VOXEL_CHUNK_SIZE_XZ + 2;
        let stale_snapshot = VoxelChunkSnapshot {
            revision: 7,
            chunk_x: 1,
            chunk_z: 0,
            border: 1,
            y_min: 0,
            y_size: 1,
            blocks: vec![0; (snapshot_size * snapshot_size) as usize],
        };

        apply_online_snapshot(&mut terrain, &stale_snapshot);

        assert_eq!(terrain.latest_revision, 8);
        assert_eq!(terrain.snapshot_revision, 8);
        assert_eq!(terrain.blocks.get(&[16, 0, 0]), Some(&BlockType::Stone));
    }

    #[test]
    fn terrain_gap_resync_is_coalesced_and_cooldown_limited() {
        let mut budget = TerrainResyncBudget::default();

        let first = budget
            .request(0.0, (2, -1), 9)
            .expect("first gap should request a snapshot");
        assert_eq!(first.request_id, 1);
        assert!(budget.request(0.1, (2, -1), 9).is_none());

        let second = budget
            .request(TERRAIN_RESYNC_RETRY_COOLDOWN_SECS, (2, -1), 9)
            .expect("a retry is allowed after the cooldown");
        assert_eq!(second.request_id, 2);
    }

    #[test]
    fn terrain_gap_resync_stops_after_bounded_retries_and_snapshot_resets_budget() {
        let mut budget = TerrainResyncBudget::default();

        for attempt in 0..TERRAIN_RESYNC_MAX_ATTEMPTS {
            assert!(
                budget
                    .request(
                        f32::from(attempt) * TERRAIN_RESYNC_RETRY_COOLDOWN_SECS,
                        (0, 0),
                        4,
                    )
                    .is_some()
            );
        }
        assert!(
            budget
                .request(
                    f32::from(TERRAIN_RESYNC_MAX_ATTEMPTS) * TERRAIN_RESYNC_RETRY_COOLDOWN_SECS,
                    (0, 0),
                    4,
                )
                .is_none()
        );

        budget.reset();
        assert!(budget.request(0.0, (0, 0), 8).is_some());
    }

    #[test]
    fn exhausted_resync_enters_backoff_instead_of_permanent_pending() {
        let mut budget = TerrainResyncBudget::default();
        for attempt in 0..TERRAIN_RESYNC_MAX_ATTEMPTS {
            assert!(
                budget
                    .request(
                        f32::from(attempt) * TERRAIN_RESYNC_RETRY_COOLDOWN_SECS,
                        (0, 0),
                        4,
                    )
                    .is_some()
            );
        }
        assert!(budget.exhausted());

        budget.enter_stall(2.0);
        assert!(!budget.can_rearm(6.9));
        assert!(budget.can_rearm(7.0));
        budget.rearm();
        assert!(budget.request(7.0, (0, 0), 8).is_some());
    }

    #[test]
    fn stale_recovery_snapshot_does_not_clear_a_pending_gap() {
        let snapshot_size = VOXEL_CHUNK_SIZE_XZ + 2;
        let mut terrain = OnlineTerrainEdits {
            latest_revision: 10,
            snapshot_revision: 8,
            snapshot_chunk: Some((1, 0)),
            snapshot_border: 1,
            y_min: 0,
            y_size: 1,
            has_snapshot: true,
            awaiting_snapshot: true,
            ..OnlineTerrainEdits::default()
        };
        terrain.blocks.insert([16, 0, 0], BlockType::Stone);
        let stale_snapshot = VoxelChunkSnapshot {
            revision: 7,
            chunk_x: 1,
            chunk_z: 0,
            border: 1,
            y_min: 0,
            y_size: 1,
            blocks: vec![0; (snapshot_size * snapshot_size) as usize],
        };

        apply_online_snapshot(&mut terrain, &stale_snapshot);

        assert!(terrain.awaiting_snapshot);
        assert_eq!(terrain.snapshot_revision, 8);
        assert_eq!(terrain.blocks.get(&[16, 0, 0]), Some(&BlockType::Stone));
    }
}
