//! 万国起源：最后一国 钻石版 — 服务端 binary
//!
//! 启动一个 Bevy MinimalPlugins headless 服务器（无窗口/无渲染/无输入），
//! 加载 lk2-core 提供的 sim / 协议，监听 UDP 等待客户端接入。
//!
//! ## 运行模式
//!
//! - 默认：监听 UDP 0.0.0.0:5000 (端口可通过 `LK2_PORT` 环境变量覆盖)。
//!   启动后跑 100 个 headless tick 做 self_check, 通过后进入正式 sim。
//! - 单机会话测试：`cargo run -p lk2-client -- --offline` 走客户端内置的 in-process
//!   sim, 不需要 server。本 binary 仅供多客户端联机时使用。
//!
//! ## 依赖关系
//!
//! ```text
//! lk2-server (MinimalPlugins + 权威 sim + 物理 + 接收 client 输入)
//!     ├── lk2-core (sim 逻辑 / 协议 / 数据结构)
//!     │       └── bevy 0.18 + leafwing + lightyear 0.26
//!     ├── bevy 0.18 (MinimalPlugins: ECS + Time + ScheduleRunner, 无 wgpu/winit)
//!     ├── avian3d 0.6 (物理确定性 step)
//!     ├── lightyear 0.26 (ServerPlugins: NetcodeServer / Replication 权威)
//!     └── leafwing-input-manager (服务端解析 client 上行的 PlayerAction)
//! ```
//!
//! 详细设计见 `docs/plans/client-server-split.md` §4 + §9 步骤 4。

#![allow(dead_code)]
#![allow(unused_imports)]

use avian3d::prelude::PhysicsPlugins;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;
use lightyear::prelude::LocalAddr;
use lightyear::prelude::server::ServerUdpIo;
// 用 leafwing ActionState 读 client 上行的 PlayerAction (lightyear 0.26
// InputPlugin::finish() 在 server 端自动加 InputManagerPlugin::<A>::server()
// + 初始化 ActionState<PlayerAction> resource, 我们直接 Res<ActionState<...>> 读就行)
use leafwing_input_manager::prelude::ActionState;
use lk2_core::protocol::PlayerAction;
use lk2_core::protocol::components::{GameplayHudState, PlayerPos, VoxelDelta};
use lk2_core::protocol::messages::{BuildRecipe, GameplayCommand, GameplayCommandKind, GameplayFeedback};
// lightyear 0.26.4 bug 绕开: `ServerMultiMessageSender` (lightyear_messages
// server.rs:33 `metadata: Res<'w, PeerMetadata>`) 依赖 `Res<PeerMetadata>`,而
// `PeerMetadata` 只在 `lightyear_connection::client::ConnectionPlugin::build`
// (lightyear_connection-0.26.4/src/client.rs:184) 里 init_resource。
// 但 server binary 只 enable 'server' feature, **不**加 `client::ConnectionPlugin`,
// 所以 `PeerMetadata` 永远不存在 → 启 server 后 system `receive_input_message`
// 第一次跑立刻 panic "Parameter ServerMultiMessageSender::metadata failed
// validation: Resource does not exist"。
//
// 修法: server main.rs 手动 init `PeerMetadata` 资源,跟 client::ConnectionPlugin
// 行为对齐。`PeerMetadata` 通过 `lightyear-0.26.4/src/lib.rs:326
// 'pub use lightyear_connection::*;'` 在 `lightyear::prelude::` 顶层 re-export,
// 路径是 `lightyear::prelude::PeerMetadata`(不是 `prelude::client::PeerMetadata`)。
use lightyear::prelude::PeerMetadata;
// lightyear 0.26.4 文档 (lightyear-0.26.4/src/lib.rs:133):
// "You can trigger LinkStart to start the link" — 必须手动 trigger,
// 否则 ServerUdpIo 不会 bind socket
use lightyear::prelude::LinkStart;
// 绕开 avian3d 0.6.1 + MinimalPlugins: avian3d::init_collider_constructor_hierarchies
// 读 `Res<SceneSpawner>`, MinimalPlugins 没 ScenePlugin 不会 init 它。
// 手动 init 一个空 SceneSpawner — server 不加载任何 .scn / .gltf 资产, 这个
// resource 永远空着不影响行为。
use bevy::scene::SceneSpawner;

use std::time::Duration;

// ---- 服务端 crate 内部模块（迁自 src/pvp/） ----
mod los;
mod pvp_systems;

// ---- lk2-core 共享 sim 逻辑 ----
use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::constant;
use lk2_core::creature::{CreatureSpawnerDone, update_creatures};
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::pvp::FixedTick;
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::scenario::{Scenario, ScenarioState};
use lk2_core::sim::{SimRole, advance_demo_tick};
use lk2_core::world::{World as GameWorld, WorldGenerator};

// ---- 服务端 crate 内部模块的导出 ----
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

// ============================================================================
// SimClock (备用，self_check / tick_recorder 用)
// ============================================================================
//
// SimClock 已经从 src/main.rs 迁到 lk2_core::clock::SimClock（task-1 干的）。
// 这里直接 use, 不重新定义。

// ============================================================================
// TimeOfDay
// ============================================================================

#[derive(Resource)]
pub struct TimeOfDay(pub f32);

impl Default for TimeOfDay {
    fn default() -> Self {
        Self(0.5)
    } // 正午
}

// ============================================================================
// main
// ============================================================================

fn main() {
    // init tracing subscriber (info 级别). server main.rs 之前没初始化,
    // info!() 调用全被吞掉, 看 server_run.out.txt 是空文件。
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 监听端口 (env LK2_PORT 覆盖, 默认 5000)
    let port: u16 = std::env::var("LK2_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(5000);
    info!("[server] listening on UDP 0.0.0.0:{}", port);

    // 读取 scenario
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
        // ====== Plugins ======
        // MinimalPlugins: ECS + Time + ScheduleRunner, 无 winit/wgpu/asset/audio/ui
        .add_plugins(MinimalPlugins)
        // 物理确定性 (server 跑 step, 不插值)
        .add_plugins(PhysicsPlugins::default())
        // 兼容补丁: avian3d 0.6.1 的 ColliderCachePlugin 默认包含在
        // PhysicsPlugins 里, 它在 PreUpdate 跑 clear_unused_colliders 读
        // `MessageReader<AssetEvent<Mesh>>`。MinimalPlugins 没启 AssetPlugin,
        // 这个 message buffer 没初始化 → panic "Message not initialized"。
        // 修复: 显式 `init_asset` + `add_message::<AssetEvent<Mesh>>` 注册。
        // server 完全不用 mesh asset, 只是为了让 system 找到 buffer 不 panic。
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<bevy::prelude::Mesh>()
        .add_message::<bevy::asset::AssetEvent<bevy::prelude::Mesh>>()
        // 兼容补丁: avian3d 0.6.1 `init_collider_constructor_hierarchies` 系统
        // (avian3d-0.6.1/src/collision/collider/backend.rs:324-329) 需要
        // `Res<SceneSpawner>` (在 bevy_scene feature 编译进来后)。MinimalPlugins
        // 不含 ScenePlugin, SceneSpawner resource 没被 init →
        // panic "Resource does not exist: SceneSpawner"。修复: 显式
        // `init_resource::<SceneSpawner>()` 注一个空的(用 bevy_scene::SceneSpawner::default())。
        // server 完全不实际 spawn scene, 仅为让 system 找到 resource 不 panic。
        .init_resource::<bevy::scene::SceneSpawner>()
        // lightyear 0.26 服务端权威
        // ⚠️ 顺序 (lightyear-0.26.4/src/lib.rs:96 强制约束):
        //   1) ServerPlugins 先 (装 netcode / link / sync / replication 系统)
        //   2) ProtocolPlugin 后 (register_message / register_component / InputPlugin)
        //   3) 之后才 spawn Server entity (后续 wire-network-and-loop task 做)
        // 缺步骤 1 时, 编译能过 (register_message lazy init MessageRegistry),
        // 但运行时 server 缺 link/sync/netcode, netcode 起不来。
        .add_plugins(lightyear::prelude::server::ServerPlugins::default())
        .add_plugins(lk2_core::protocol::ProtocolPlugin)
        .add_plugins(ServerPvPPlugin)
        // wire-network-and-loop 任务（2026-06-10）补: bevy 0.18 的 Message
        // 总线（本地 event，区别于 lightyear register 的网络 message）需要
        // 显式 add_message，read_attack_inputs 读 MessageReader<AttackInput>。
        .add_message::<lk2_core::protocol::messages::AttackInput>()
        .add_message::<lk2_core::protocol::messages::HitConfirm>()
        .add_message::<lk2_core::protocol::messages::KnockbackEvent>()
        .add_message::<lk2_core::protocol::messages::DamageResult>()
        .add_message::<lk2_core::pvp::DamageEvent>()
        .add_message::<lk2_core::pvp::VisualEffectEvent>()
        // ====== Resources ======
        // 绕开 lightyear 0.26.4 bug: PeerMetadata 必须 init,否则 receive_input_message panic
        .init_resource::<PeerMetadata>()
        // 绕开 avian3d 0.6.1 + MinimalPlugins: SceneSpawner 必须存在,
        // 否则 init_collider_constructor_hierarchies panic
        .init_resource::<SceneSpawner>()
        .init_resource::<SimClock>()
        .init_resource::<TimeOfDay>()
        .init_resource::<GameWorld>()
        .init_resource::<GlobalResourcePool>()
        .init_resource::<NationRegistry>()
        .init_resource::<MonsterEcosystem>()
        .init_resource::<TickObserver>()
        .init_resource::<TickRecorder>()
        .init_resource::<CreatureSpawnerDone>()
        .init_resource::<PlayerState>()
        .init_resource::<FixedTick>()
        .insert_resource(scenario_state)
        // ====== Startup ======
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
        // 备选: 监听 client connect (On<Add, Connected>) 时再 spawn,
        // 但启动就 spawn 也行 (server-side 复制的 entity 会在 client connect
        // 时被自动 broadcast)。当前走 Startup 启动即 spawn 路径, 简单点。
        // .add_observer(spawn_player_for_connected)
        // ====== Update ======
        .add_systems(
            Update,
            (
                simulation_tick,
                end_tick_system,
                tick_recorder,
                update_creatures,
            )
                .chain(),
        )
        // ====== FixedUpdate (server PvP) ======
        .add_systems(
            FixedUpdate,
            (
                // record_position_history 已在 ServerPvPPlugin 内
                apply_input_to_player,
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

// ============================================================================
// dump_world_resources — debug: 确认 PeerMetadata 在 world 里
// ============================================================================
fn dump_world_resources(world: &bevy::prelude::World) {
    let has_peer_metadata = world.get_resource::<PeerMetadata>().is_some();
    info!("[debug] PeerMetadata in world? {}", has_peer_metadata);
    let has_scene_spawner = world.get_resource::<SceneSpawner>().is_some();
    info!("[debug] SceneSpawner in world? {}", has_scene_spawner);
}

// ============================================================================
// spawn_server — wire lightyear UDP transport (subtask 1 of wire-network-and-loop)
// ============================================================================
//
// lightyear 0.26 用 reactive 模式启 transport: 不是 add_plugins 启,而是 spawn
// 一个 entity 挂 `ServerUdpIo` + `LocalAddr(server_addr)`,然后
// `LinkStart` observer 触发,系统自动 `UdpSocket::bind(local_addr)`。
//
// 参考:
// - lightyear_udp-0.26.4/src/server.rs:30-50 (`ServerUdpIo` 定义 + `#[require(Server)]`)
// - lightyear_udp-0.26.4/src/server.rs:71-95 (LinkStart observer 真正 bind socket 的 system)
//
// ServerUdpIo 的 `#[require(Server)]` 会自动加 Server marker, 所以不用手写。
fn spawn_server(mut commands: Commands) {
    use lightyear::prelude::server::Start;
    use lightyear_netcode::server_plugin::{NetcodeConfig, NetcodeServer};

    let server_addr = lk2_core::transport::server_listen_addr();
    info!(
        "[net] spawning server entity with ServerUdpIo + NetcodeServer + LocalAddr({})",
        server_addr
    );

    // 构造 NetcodeServer: 这是 lightyear 0.26 server 端接 client UDP connect
    // request 的核心组件, NetcodeServerPlugin 的 start observer 跑时
    // `Query<(), With<NetcodeServer>>` 必须能 match 到, 否则 Started marker
    // 不会 insert, client connect 永远停在 Connecting。
    // private_key + protocol_id 必须跟 client 端的 NetcodeClient 一致:
    // - 同一对 server/client 通信: shared private_key (从固定文件读 / 启动时
    //   随机生成并写文件) + 同一 protocol_id
    // - dev 模式: 启动时 generate_key() 并写 .lk2_server_key, client 端从
    //   --server-arg 读。这里先简化为固定 32 字节全 0xAA 占位 (注意: 必须跟
    //   client 端 用的 key 一致, 后续再调成从文件读)。
    // - protocol_id 0x4C4B3256_4E455457 = "LK2VNETW" dev id
    //   (lightyear netcode 要求两端的 protocol_id 完全相同, 校验失败 server
    //    直接 reject client 的 connect token)
    let private_key: lightyear_netcode::Key = [0xAA; lightyear_netcode::PRIVATE_KEY_BYTES];
    let protocol_id: u64 = 0x4C4B3256_4E455457;
    let netcode_server = NetcodeServer::new(
        NetcodeConfig::default()
            .with_protocol_id(protocol_id)
            .with_key(private_key),
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
    // 手动 trigger LinkStart: lightyear 0.26.4 文档明示
    // "You can trigger LinkStart to start the link"
    // (lightyear-0.26.4/src/lib.rs:133)。不 trigger 的话 ServerUdpIo 的
    // LinkStart observer 永远不跑, UDP socket 永远不 bind, server 等于
    // 没 listen。
    info!(
        "[net] triggering LinkStart on server entity {:?}",
        server_id
    );
    commands.trigger(LinkStart { entity: server_id });
    // 手动 trigger Start: lightyear_netcode 0.26 NetcodeServerPlugin
    // 加了 On<Start> observer (lightyear_netcode-0.26.4/src/server_plugin.rs:295),
    // 不 trigger 的话 Started marker 不会 insert, server-side netcode 不真
    // 接受 client connect request。Start 来自 lightyear_connection::server::Start。
    info!("[net] triggering Start on server entity {:?}", server_id);
    commands.trigger(Start { entity: server_id });
}

// ============================================================================
// spawn_player — authoritative player entity, 挂 PlayerPos + Replicate
// ============================================================================
//
// wire-network-and-loop 任务 (2026-06-11): B 粒度闭环补完。
// server 启动即 spawn 一个 authoritative player entity, lightyear 默认
// server→client 复制会把 PlayerPos 自动发给 client, client 端
// apply_networked_position 就能 apply。
//
// 简化: 只支持 1 个 client (单进程 demo)。后面做 per-client player 实体
// 再细化。
fn spawn_player(mut commands: Commands) {
    let spawn = bevy::math::Vec3::new(
        constant::WORLD_SIZE as f32 / 2.0 + 0.5,
        (constant::SEA_LEVEL + 2) as f32 + 0.5,
        constant::WORLD_SIZE as f32 / 2.0 + 0.5,
    );
    info!("[player] spawning authoritative player entity at {:?}", spawn);
    commands.spawn((
        Name::new("Player"),
        bevy::prelude::Transform::from_translation(spawn),
        lk2_core::protocol::components::PlayerPos(spawn),
        // docs: lightyear-0.26.4/src/lib.rs:195 "To replicate an entity from the
        // local world to the remote world, you can just add the Replicate
        // component to the entity"
        lightyear::prelude::Replicate::to_clients(
            lightyear::prelude::NetworkTarget::All,
        ),
    ));
}

// 备选 observer 版本 (没启,见 Startup chain 注释)
// fn spawn_player_for_connected(...)

// ============================================================================
// apply_input_to_player — server 端读 client ActionState, 改 Player Transform
// ============================================================================
//
// 读 `Res<ActionState<PlayerAction>>` (lightyear_inputs_leafwing 的
// InputManagerPlugin::server() 自动 init, 通过 Res 拿全局 input) 算 Vec2
// 方向, 应用到玩家 entity 的 Transform.translation, 同时写回 PlayerPos
// (lightyear 复制的是 PlayerPos), 然后 lightyear 自动把 PlayerPos 同步给 client。
//
// 这是 authoritative server sim 的最小闭环: client 按 W → server
// 收到 ActionState.pressed(MoveForward)=true → 玩家 entity 向 +Z 移动
// → PlayerPos 复制 → client apply_networked_position 更新本机
// Transform。
fn apply_input_to_player(
    actions: Option<Res<ActionState<PlayerAction>>>,
    mut q: Query<
        (
            &mut bevy::prelude::Transform,
            &mut lk2_core::protocol::components::PlayerPos,
        ),
        With<Name>,
    >,
) {
    let Some(actions) = actions else {
        return;
    };
    let mut dir = bevy::math::Vec2::ZERO;
    if actions.pressed(&PlayerAction::MoveForward) {
        dir.y += 1.0;
    }
    if actions.pressed(&PlayerAction::MoveBackward) {
        dir.y -= 1.0;
    }
    if actions.pressed(&PlayerAction::MoveLeft) {
        dir.x -= 1.0;
    }
    if actions.pressed(&PlayerAction::MoveRight) {
        dir.x += 1.0;
    }
    if dir.length() > 1.0 {
        dir = dir.normalize();
    }
    let speed = 4.0; // m/s, 跟 client 的 PvPController 默认 speed 对齐
    let dt = 1.0 / 30.0; // 30 TPS fixed update
    let delta = bevy::math::Vec3::new(dir.x, 0.0, -dir.y) * speed * dt; // bevy camera: -Z = forward
    for (mut transform, mut player_pos) in q.iter_mut() {
        if transform.translation == bevy::math::Vec3::ZERO && transform.translation.length() < 0.001
        {
            // 跳过默认 Transform (origin 0,0,0) — spawn 的玩家有真实位置
            continue;
        }
        transform.translation += delta;
        // 同步 PlayerPos: lightyear 复制的是 PlayerPos, 不是 Transform
        player_pos.0 = transform.translation;
    }
    if dir.length() > 0.01 {
        info!("[player] input dir={:?} applied delta={:?}", dir, delta);
    }
}

// ============================================================================
// Setup
// ============================================================================

fn setup_world(
    _commands: Commands,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
) {
    // 默认 preset (跟原来一致)
    let pipeline = lk2_core::world::terrain::presets::by_name("default");
    *game_world = GameWorld::with_pipeline(constant::WORLD_SIZE, pipeline);
    info!("[terrain] using preset '{}'", game_world.pipeline.name);

    use lk2_core::resource::ResourceKind;
    for k in ResourceKind::ALL {
        let init = 50.min(k.max() / 2).max(10);
        let _ = pool.force_add(*k, init);
    }

    monsters.demo_init([
        constant::WORLD_SIZE / 2,
        constant::SEA_LEVEL + 1,
        constant::WORLD_SIZE / 2,
    ]);

    info!("🌍 世界已生成: {}³ (server)", constant::WORLD_SIZE);
}

fn self_check(
    game_world: Res<GameWorld>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    mut obs: ResMut<TickObserver>,
) {
    info!(">>> 服务端启动自检 100 tick ...");
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

// ============================================================================
// Simulation
// ============================================================================

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

// ============================================================================
// Tick 录制（每 5 tick dump 一次 state JSON, 跟原 src/main.rs::tick_recorder 一致）
// ============================================================================

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
        &obs,
        &game_world,
        SimRole::ServerAuthority.into(),
    );
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, s);
    }
}
