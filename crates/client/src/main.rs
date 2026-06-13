//! 万国起源：最后一国 钻石版 — 客户端 binary
//!
//! 启动一个 Bevy 窗口（DefaultPlugins），加载 lk2-core 提供的 sim / 协议，
//! 运行渲染 / 输入 / HUD / 客户端 PvP 预测 / 怪物同步 等**只**在客户端跑的逻辑。
//!
//! ## 运行模式
//!
//! - 默认：尝试连接一个已启动的 lk2-server（UDP 5000）。当前任务（build-client）
//!   还没接 UDP 传输，所以默认启动会 hang（待 wire-network-and-loop task 修）。
//! - `--offline`：客户端启动一个**进程内**的 in-process sim（无 transport、无
//!   server），行为与原先的单 binary demo 完全一致。loop.ps1 默认走 `--offline`。
//!
//! ## 依赖关系
//!
//! ```text
//! lk2-client (DefaultPlugins + 渲染 + 输入 + 客户端 PvP)
//!     ├── lk2-core (sim 逻辑 / 协议 / 数据结构)
//!     │       └── bevy 0.18 + leafwing + lightyear 0.26
//!     ├── bevy 0.18 (DefaultPlugins: winit / wgpu / window / ...)
//!     ├── avian3d 0.6 (物理 — 客户端视觉插值用)
//!     ├── lightyear 0.26 (ClientPlugins: NetworkMessages / Replication 资源)
//!     └── ...
//! ```
//!
//! 详细设计见 `docs/plans/client-server-split.md` §5 + §9 步骤 5。

#![allow(dead_code)]
#![allow(unused_imports)]

use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use std::path::PathBuf;

use avian3d::prelude::{Collider, Gravity, LinearVelocity, PhysicsPlugins, RigidBody};

// ---- 客户端 crate 内部模块（迁自 src/） ----
mod controller_systems;
mod pretty;
mod pvp_systems;
mod render;
mod ui;

// ---- lk2-core 共享 sim 逻辑 ----
use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::constant;
use lk2_core::creature::{
    CreatureSpawnerDone, player_attack_creatures as offline_player_attack_creatures,
    spawn_creatures, update_creatures,
};
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::pvp::{FixedTick, PositionHistory};
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::scenario::{Scenario, ScenarioState};
use lk2_core::sim::{SimRole, advance_demo_tick};
use lk2_core::world::{World as GameWorld, WorldGenerator};

// ---- 客户端 crate 内部模块的导出 ----
use crate::controller_systems::{
    ControllerPlugin, auto_step_up, character_movement, collect_input, ground_detection,
    knockback_decay,
};
use crate::pretty::{PrettyConfig, animate_avatar, spawn_pretty};
use crate::pvp_systems::{
    HealthHudMarker, client_attack_predict, collect_local_input, on_damage_result, on_hit_confirm,
    on_knockback_event, trigger_visual_effects,
};
use crate::render::{
    AnimalIndicatorText, CameraAngles, CameraMode, FreeFlyState, LastMoveDirection, Player,
    RenderConfig, SpawnedBlocks, auto_demo, camera_mode_toggle, cycle_terrain_preset,
    emergency_teleport, first_person_camera, freefly_movement, freefly_toggle, held_weapon_follow,
    mouse_look_system, player_input, player_spawn_position_at, setup_atmosphere,
    setup_cursor_grab, setup_terrain_underlay, spawn_terrain_around_player, underlay_follow_player,
    update_animal_indicator,
};
use crate::ui::{ClientRunMode, setup_fonts, setup_hud, update_hud};

// ---- 重新导出 lk2-core PvP 数据（main.rs 里要直接用） ----
use leafwing_input_manager::prelude::ActionState;
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

// ---------------------------------------------------------------------------
// CLI 解析
// ---------------------------------------------------------------------------

/// 启动时解析的 preset 名
static PRESET_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
fn preset_name_static() -> &'static str {
    PRESET_NAME.get().map(|s| s.as_str()).unwrap_or("default")
}

/// --walk=x,z 解析的 spawn 位置
static WALK_OVERRIDE: std::sync::OnceLock<Option<(i32, i32)>> = std::sync::OnceLock::new();
fn walk_override_static() -> Option<(i32, i32)> {
    WALK_OVERRIDE.get().copied().flatten()
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    // init tracing subscriber, 跟 server 一样让 info/warn 能看到
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let offline_mode = args.iter().any(|a| a == "--offline");
    let auto_demo_mode = args.iter().any(|a| a == "--auto-demo");

    // 解析 --connect=<ip:port>（wire-network-and-loop 任务, 2026-06-10）
    // --offline 时强制 offline 模式（即使用户写了 --connect）
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

    // 解析 --preset
    let preset_name = args
        .iter()
        .find(|a| a.starts_with("--preset="))
        .map(|a| a.trim_start_matches("--preset=").to_string())
        .unwrap_or_else(|| "default".to_string());
    let _ = PRESET_NAME.set(preset_name.clone());
    println!("[terrain] preset = {}", preset_name);

    // --smooth-terrain / --legacy-voxel
    let smooth_terrain = !args.iter().any(|a| a == "--legacy-voxel");
    println!("[render] smooth_terrain = {}", smooth_terrain);

    // --walk=x,z
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

    // 加载 scenario
    let scenario = if auto_demo_mode {
        Scenario {
            name: "idle".into(),
            record_window: None,
            steps: vec![
                lk2_core::scenario::ScenarioStep::Log {
                    msg: "=== idle: 玩家不动看动物 ===".into(),
                },
                lk2_core::scenario::ScenarioStep::WaitTicks { ticks: 1000 },
            ],
        }
    } else {
        lk2_core::scenario::load_scenario_from_args_or_default(&args)
    };
    let scenario_state = ScenarioState::from_scenario(scenario.clone());

    let _ = std::fs::create_dir_all("screenshots");

    let mut app = App::new();

    // ===== 1. DefaultPlugins（含 winit / wgpu / WindowPlugin）=====
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin { file_path: "../../assets".into(), ..default() })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("万国起源：最后一国 钻石版 — {}", scenario.name).into(),
                    resolution: WindowResolution::new(1280, 720),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::log::LogPlugin { level: bevy::log::Level::INFO, ..default() }),
    );

    // ===== 2. 物理（avian3d）=====
    app.add_plugins(PhysicsPlugins::default()).insert_resource(Gravity::default());

    // Always add ClientPlugins; offline mode simply skips spawning the network client entity.
    app.add_plugins(lightyear::prelude::client::ClientPlugins::default());
    app.add_plugins(lk2_core::protocol::ProtocolPlugin);

    // Required by lightyear 0.26 + workspace feature unification; without these,
    // client startup can panic in network mode.
    app.init_resource::<lightyear::prelude::PeerMetadata>()
        .init_resource::<lk2_core::pvp::FixedTick>()
        .init_resource::<TimeOfDay>();
    // Local Bevy message bus used by send_online_gameplay_commands before
    // Lightyear forwards GameplayCommand to the server.
    app.add_message::<GameplayCommand>();

    if network_mode {
        let server_addr = connect_addr.expect("network_mode=true implies connect_addr is Some");
        app.add_systems(Startup, move |commands: Commands| {
            // client_id 用启动时 unix timestamp ms 末 16 位, dev 模式不需要
            // 全局唯一, 1 个 client 就够。如果同机起多个 client 再用 counter。
            let client_id_seed: u64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64 & 0xFFFF_FFFF_FFFF_FFFF)
                .unwrap_or(0xC11E_71);
            spawn_networked_client(commands, server_addr, client_id_seed);
        });
    }

    // ===== 4. 资源初始化 =====
    app.init_resource::<RenderConfig>()
        .init_resource::<CameraAngles>()
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
        .init_resource::<TickObserver>()
        .init_resource::<TickRecorder>()
        .init_resource::<LastMoveDirection>()
        .init_resource::<FreeFlyState>()
        .init_resource::<CameraMode>()
        .init_resource::<CreatureSpawnerDone>()
        .init_resource::<FixedTick>()
        .init_resource::<ReplicatedSnapshot>()
        .init_resource::<NetworkSmoothingState>()
        .insert_resource(if network_mode {
            ClientRunMode::Online
        } else {
            ClientRunMode::Offline
        })
        .insert_resource(scenario_state);

    // ===== 5. PvP / 控制器 plugins（合并 lk2-core 的协议 + 客户端实现）=====
    app.add_plugins(ClientPvPPlugin).add_plugins(ControllerPlugin);

    // ===== 6. 启动系统（一次性 setup）=====
    app.add_systems(
        Startup,
        (
            setup_fonts,
            setup_camera,
            setup_light,
            setup_atmosphere,
            setup_cursor_grab,
            setup_terrain_underlay,    // ← 兜底盖板（修缝 B：marching_cubes 漏底面）
            setup_world,
            spawn_pretty,
            spawn_creatures,
            setup_hud,
            self_check,
            setup_player_pvp,
        )
            .chain(),
    );

    // ===== 7. 每帧 Update 系统（核心循环）=====
    //
    // 注意: bevy 0.18 的 `IntoScheduleConfigs` tuple impl 只到 20 元 (见
    // bevy_ecs-0.18.1/src/schedule/config.rs:613 `all_tuples!(..., 1, 20, ...)`),
    // 且 `.chain()` 不能在 tuple 里混 `.before(X)` (产生 boxed ScheduleConfigs,
    // 不是 tuple 形式)。所以分多块 add_systems, 每块 tuple ≤ 20。
    // 22 systems 拆 2 块: 第一块 scenario + network, 第二块 input/camera/HUD。
    app.add_systems(
        Update,
        (
            // scenario / sim (3)
            lk2_core::scenario::scenario_runner,
            lk2_core::scenario::simulate_player_actions,
            lk2_core::scenario::scenario_tick_recorder,
            // network mode (1): apply server-replicated PlayerPos to PlayerState
            // (offline 模式下 PlayerPos 没注册组件, 这个系统是 no-op)
            apply_networked_position,
            // 应用层 PlayerPos sync 接收 (走 ServerPosUpdate message, 绕开 lightyear
            // 0.26 自动 UpdatesMessage 那 1%)
            apply_server_pos_update,
            debug_dump_replicated_entities,
            apply_authoritative_snapshot,
            apply_voxel_delta,
            send_online_gameplay_commands,
            // 键盘 → leafwing ActionState(lightyear 会自动 serialize 上行)
            // client 端必须用 ActionState 标记 pressed, lightyear_inputs_leafwing
            // 的 ClientInputPlugin 才会把它打包成 InputMessage 发到 server。
            collect_keys_to_action_state,
            // 客户端独有 (8) — split 一部分到第二个 chain 避免 tuple 超 20
            auto_demo,
            mouse_look_system,
            first_person_camera,
            held_weapon_follow,
            player_input,
            offline_player_attack_creatures,
            animate_avatar,
            underlay_follow_player,    // ← 兜底盖板跟玩家（marching_cubes 漏底面 → 兜底 plane 跟到脚下 -5m）
            spawn_terrain_around_player,
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (
            // F3 自由视角 / C 切 3rd person / F5 传送 / F8 切 preset (4)
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
            // PvP 客户端 (6)
            collect_local_input,
            client_attack_predict,
            on_hit_confirm,
            on_knockback_event,
            on_damage_result,
            trigger_visual_effects,
            // 控制器（地面 / WASD / 爬台阶 / 击退衰减） (5)
            ground_detection,
            character_movement,
            auto_step_up,
            knockback_decay,
            collect_input,
        )
            .chain(),
    );
    // freefly_movement 必须先于 first_person_camera (否则镜头不动)
    app.add_systems(Update, freefly_movement.before(first_person_camera));
    // interpolate_online_player / apply_authoritative_snapshot 之前被加
    // 但函数没定义(都是 baseline 不稳定)。apply_networked_position
    // 已经够用 (server 复制 PlayerPos → 写本机玩家 Transform)。
    // 留着 hook 注释以备后续 per-client prediction / interpolation 加进来。

    // ===== 8. 辅助系统（截图 / HUD / 退出 / tick 录制）=====
    app.add_systems(
        Update,
        (
            simulation_tick,
            end_tick_system,
            update_hud,
            update_animal_indicator,
            tick_recorder,
            periodic_screenshot,
            update_creatures,
            day_night_cycle,
            exit_on_esc,
        )
            .chain(),
    );

    // ===== 9. 启动！ =====
    app.run();
}

// Spawn the lightyear UDP client link in online mode.
fn spawn_networked_client(
    mut commands: Commands,
    server_addr: std::net::SocketAddr,
    client_id_seed: u64,
) {
    use lightyear::prelude::UdpIo;
    use lightyear::prelude::client::Connect;
    use lightyear::prelude::{LinkStart, LocalAddr, PeerAddr};
    use lightyear::prelude::MessageReceiver;
    use lk2_core::protocol::messages::ServerPosUpdate;
    use lightyear_netcode::client_plugin::{NetcodeClient, NetcodeConfig};
    use lightyear_netcode::prelude::Authentication;

    info!(
        "[net] spawning client entity with UdpIo + LocalAddr(0.0.0.0:0) + PeerAddr({})",
        server_addr
    );
    // 构造 NetcodeClient: 这是 lightyear 0.26 client 端的 "连接凭证" 组件,
    // NetcodeClientPlugin::connect observer (`On<Connect>` at
    // lightyear_netcode-0.26.4/src/client_plugin.rs:193) 跑时
    // `Query<&mut NetcodeClient, Without<Connected>>` 必须能 match 到这个组件,
    // 否则 `client.inner.connect()` 永远不调用。Authentication::Manual 是
    // 开发模式 (生产环境应该走 backend HTTPS 拿 ConnectToken, 但本地单局域网
    // 联机不需要这层安全, dev 模式直接 Manual 配 private_key 即可)。
    // dev 模式: 跟 server 端 spawn_server 一致用固定 0xAA key + 固定 protocol_id。
    // (生产环境应该 server 把 key 写 .lk2_server_key, client 从 argv / env 读)
    let private_key: lightyear_netcode::Key = [0xAA; lightyear_netcode::PRIVATE_KEY_BYTES];
    let protocol_id: u64 = 0x4C4B3256_4E455457; // "LK2VNETW" → dev protocol id
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
            // 应用层 PlayerPos sync — client 端 receiver buffer (绕开 lightyear 0.26
            // 自动 UpdatesMessage 那条 1% 卡死的路径, server 用 ServerMultiMessageSender
            // 推 ServerPosUpdate, 走 MetadataChannel, 100% 可靠 + 不乱序)
            MessageReceiver::<ServerPosUpdate>::default(),
        ))
        .id();
    // 手动 trigger LinkStart, 跟 server 端同理 (lightyear 0.26 文档明示
    // "You can trigger LinkStart to start the link" — lightyear-0.26.4/src/lib.rs:133)。
    // 不 trigger 的话 UdpIo 的 LinkStart observer 永远不跑, UDP socket 永远不 bind。
    info!(
        "[net] triggering LinkStart on client entity {:?}",
        client_id
    );
    commands.trigger(LinkStart { entity: client_id });
    // 手动 trigger Connect: lightyear_netcode 0.26 NetcodeClientPlugin
    // 加了 On<Connect> observer (lightyear_netcode-0.26.4/src/client_plugin.rs:193),
    // 不 trigger 的话 client.inner.connect() 永远不跑, UDP 不真发 connect
    // request 到 server, server 永远不会收到 client。必须 LinkStart 之后
    // 立刻 trigger Connect (同 frame / 同 command batch)。
    info!("[net] triggering Connect on client entity {:?}", client_id);
    commands.trigger(Connect { entity: client_id });
}

// Mirror replicated server position onto PlayerState resource.
// All client systems (terrain, avatar, camera) already read PlayerState, so this
// is the only bridge needed between network and rendering.
fn apply_networked_position(
    mut q: Query<(&mut Transform, &lk2_core::protocol::components::PlayerPos)>,
) {
    // 注: 不带 With<Player> filter — server 端 spawn 的 authoritative player
    // entity 复制到 client 时, 不会带 client 本地 crate::render::Player marker
    // (server 端没有这个 type), 只有一个 Name("Player") 跟 Replicate 组件。
    // 所以 query 不加 With<Player> filter, 不管哪个 entity 复制了 PlayerPos 都 apply。
    let mut count = 0;
    let mut first_pos = bevy::math::Vec3::ZERO;
    for (mut tf, pos) in q.iter_mut() {
        tf.translation = pos.0;
        first_pos = pos.0;
        count += 1;
    }
    if count > 0 {
        // 复制确实到了:本地玩家 entity 上有 PlayerPos 组件
        // (offline 模式没 server 复制 → count = 0, 不 log 噪音)
        tracing::info!(
            "[net] applied PlayerPos to {} player entity, pos={:?}",
            count,
            first_pos
        );
    }
}

// Debug: 5 秒一次 dump client world 里所有 entity 的 component name + lightyear
// 关键 marker (Replicate / Linked / ClientOf / Connect token) 是否出现。
// 给 wire-network-and-loop 任务排查 PlayerPos 复制是否真到 client world 用。
// (Query<EntityRef> + TypedReflect / TypeRegistry 在 bevy 0.18 的接口
//  仍不稳定, 走简化的 Query<Option<&Name>> + 多个 marker Query 计数)
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
    // 列出所有带 Name 的 entity + component 概要
    let mut sample: Vec<String> = Vec::new();
    for (e, name) in names.iter().take(8) {
        let n = name.map(|n| n.as_str()).unwrap_or("?");
        sample.push(format!("{:?}={}", e, n));
    }
    tracing::info!(
        "[net-debug] total_entities={}, replicate={}, replicated={}, clientof={}, linked={}, playerpos={}, sample=[{}]",
        total, rep, red, co, ln, pp, sample.join(", ")
    );
}

// ============================================================================
// apply_server_pos_update — 应用层 PlayerPos sync 接收 (绕开 lightyear 0.26
// 自动 UpdatesMessage 那 1%)
//
// server 端用 ServerMultiMessageSender 每 2 tick 推一个 ServerPosUpdate
// (走 MetadataChannel, UnorderedReliable), client 端的 MessageReceiver buffer
// 自动接收 (on_add hook 注册到 MessageManager.receive_messages, 跟新 protocol
// `ServerPosUpdate` type bind 起来)。这里 drain 出来写 PlayerState.pos, 本机
// render 系统的所有 reader 都会用新 pos。
//
// 注意: server 端推的是 server 端 sim player entity 的位置, client 端没有
// "replicated server player entity" (因为 lightyear 0.26 UpdatesMessage 不来),
// 所以这里**直接拿最新 ServerPosUpdate.pos 写 PlayerState.pos**, 反正
// 单 client demo 我们就是要把 server 推过来的权威位置渲染到本机玩家身上。
//
// offline 模式不跑 (run_mode != Online, 走 early return)。
// ============================================================================
fn apply_server_pos_update(
    run_mode: Res<ClientRunMode>,
    mut receiver_q: Query<
        &mut lightyear::prelude::MessageReceiver<
            lk2_core::protocol::messages::ServerPosUpdate,
        >,
    >,
    mut player: ResMut<PlayerState>,
) {
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

// Map local keyboard state into the replicated ActionState used by lightyear.
fn collect_keys_to_action_state(
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut ActionState<PlayerAction>, With<Player>>,
) {
    use leafwing_input_manager::prelude::ActionState as _;

    let mut action_state = match q.single_mut() {
        Ok(s) => s,
        Err(_) => return, // 玩家 entity 不存在 (offline 模式 client 端 spawn 的本地玩家是另一个 entity, 没 ActionState)
    };

    // Rebuild the action state each tick to avoid stale presses.
    *action_state = ActionState::<PlayerAction>::default();

    if keys.pressed(KeyCode::KeyW) {
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

// ---------------------------------------------------------------------------
// ClientPvPPlugin — 客户端 PvP 系统打包
// ---------------------------------------------------------------------------
//
// 原 umbrella 的 `src/pvp/mod.rs::PvPPlugin` 同时 add 了 server 系统和 client
// 系统。本 crate 是客户端，所以只 add client 那批。
//
// `FixedTick` / `DamageEvent` / `VisualEffectEvent` 是 message，必须显式
// `add_message`（`register_message` 是给网络用的，message 总线是 bevy 自己
// 的 `add_message`）。

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

// ---------------------------------------------------------------------------
// 客户端独有：场景光、HUD、字体、相机
// ---------------------------------------------------------------------------

/// 太阳 marker
#[derive(Component)]
pub struct Sun;

/// 昼夜循环用的时间（0..1）
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
        Player,
    ));
}

fn setup_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight { illuminance: 20000.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        Sun,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: false,
            color: Color::srgb(0.8, 0.85, 1.0),
            ..default()
        },
        Transform::from_xyz(-30.0, 40.0, -20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 1.2,
        affects_lightmapped_meshes: true,
    });
}

/// 昼夜循环：60 真实秒 = 1 游戏日（0..1）
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
    clear.0 = Color::srgb(
        0.04 + 0.41 * dayness + 0.60 * sunset_glow,
        0.06 + 0.59 * dayness + 0.30 * sunset_glow,
        0.16 + 0.79 * dayness + 0.10 * sunset_glow,
    );
}

// ---------------------------------------------------------------------------
// 客户端世界初始化：地形 + 玩家 + 资源 + 怪物
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn setup_world(
    _commands: Commands,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut player: ResMut<PlayerState>,
) {
    let pipeline = lk2_core::world::terrain::presets::by_name(preset_name_static());
    *game_world = lk2_core::world::World::with_pipeline(constant::WORLD_SIZE, pipeline);
    info!("[terrain] using preset '{}'", game_world.pipeline.name);

    for k in ResourceKind::ALL {
        let init = 50.min(k.max() / 2).max(10);
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
        Vec3::new(sx as f32 + 0.5, 50.0, sz as f32 + 0.5),
        [sx, 50, sz],
    ));
    player.block_pos = spawn;
    player.pos = spawn_pos;
    player.inventory.insert(ResourceKind::Wood, 0);
    player.inventory.insert(ResourceKind::Food, 5);

    info!(
        "🌍 世界已生成: {}³, 玩家在 {:?}",
        constant::WORLD_SIZE,
        spawn
    );
}

/// 启动自检：跑 100 tick headless sim invariants
fn self_check(
    game_world: Res<GameWorld>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    mut obs: ResMut<TickObserver>,
) {
    info!(">>> 启动自检 100 tick ...");
    let report = lk2_core::diagnostics::run_self_check(
        &game_world,
        &pool,
        &nations,
        &monsters,
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

// In offline mode the client advances the shared sim locally for demo/self-loop use.

fn simulation_tick(
    time: Res<Time>,
    mut clock: ResMut<SimClock>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut obs: ResMut<TickObserver>,
) {
    let _ = advance_demo_tick(
        &time,
        &mut clock,
        &mut pool,
        &mut monsters,
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

// ---------------------------------------------------------------------------
// 截图：每 5 秒一张（loop.ps1 用）
// ---------------------------------------------------------------------------

fn periodic_screenshot(
    time: Res<Time>,
    mut clock: ResMut<SimClock>,
    mut commands: Commands,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    obs: Res<TickObserver>,
    game_world: Res<GameWorld>,
    run_mode: Res<ClientRunMode>,
) {
    let now = time.elapsed_secs();
    if now - clock.last_screenshot_wall < 5.0 {
        return;
    }
    clock.last_screenshot_wall = now;
    clock.screenshot_count += 1;

    let iter_id = clock.screenshot_count;
    let iter_dir = format!("screenshots/iter_{:02}", iter_id);
    let _ = std::fs::create_dir_all(&iter_dir);
    let png_path: PathBuf = format!("{}/iter_{:02}.png", iter_dir, iter_id).into();
    let state_path = format!("{}/final_state.json", iter_dir);

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
}

fn exit_on_esc(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }
}

// ---------------------------------------------------------------------------
// PvP 初始化：给玩家 entity 挂上战斗组件
// ---------------------------------------------------------------------------

fn setup_player_pvp(mut commands: Commands, player: Query<Entity, With<Player>>) {
    use lk2_core::pvp::WeaponId;
    let iron = WeaponId::IronSword.stats();
    for entity in player.iter() {
        commands.entity(entity).insert((
            RigidBody::Kinematic,
            Collider::capsule(0.3, 0.9),
            LinearVelocity::default(),
            lk2_core::controller::PvPController::new()
                .with_speed(5.0)
                .with_jump(8.0)
                .with_knockback_resistance(0.1),
            lk2_core::controller::PlayerCollider::default(),
            CombatState::default(),
            // Online mode reads and replicates input from this component.
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
            Health(20.0),
        ));
        info!(
            "⚔ PvP 组件已挂载（铁剑 reach={}, dmg={}）+ 角色控制器",
            iron.reach, iron.damage
        );
    }
}

// ---------------------------------------------------------------------------
// Tick-level 录制
// ---------------------------------------------------------------------------

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
        &obs,
        &game_world,
        *run_mode,
    );
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, s);
        info!("📝 tick state dumped → {}", path);
    }
}

/// 构造完整 sim state JSON
fn build_state_json(
    time: &Time,
    clock: &SimClock,
    player: &PlayerState,
    pool: &GlobalResourcePool,
    nations: &NationRegistry,
    monsters: &MonsterEcosystem,
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
        obs,
        game_world,
        run_mode.snapshot_role(),
    )
}
