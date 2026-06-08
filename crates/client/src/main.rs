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

use bevy::prelude::*;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::window::{PresentMode, WindowResolution};
use std::path::PathBuf;

use avian3d::prelude::{Collider, Gravity, LinearVelocity, PhysicsPlugins, RigidBody};

// ---- 客户端 crate 内部模块（迁自 src/） ----
mod controller_systems;
mod pretty;
mod pvp_systems;
mod render;

// ---- lk2-core 共享 sim 逻辑 ----
use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::constant;
use lk2_core::creature::{player_attack_creatures, spawn_creatures, update_creatures, CreatureSpawnerDone};
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::pvp::{FixedTick, PositionHistory};
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::scenario::{Scenario, ScenarioState};
use lk2_core::world::{World as GameWorld, WorldGenerator};

// ---- 客户端 crate 内部模块的导出 ----
use crate::controller_systems::{
    auto_step_up, character_movement, collect_input, ground_detection, knockback_decay,
    ControllerPlugin,
};
use crate::pretty::{animate_avatar, spawn_pretty, PrettyConfig};
use crate::pvp_systems::{
    client_attack_predict, collect_local_input, on_damage_result, on_hit_confirm,
    on_knockback_event, trigger_visual_effects, HealthHudMarker,
};
use crate::render::{
    auto_demo, camera_mode_toggle, cycle_terrain_preset, emergency_teleport, first_person_camera,
    freefly_movement, freefly_toggle, held_weapon_follow, mouse_look_system, player_input,
    setup_atmosphere, setup_cursor_grab, spawn_terrain_around_player, update_animal_indicator,
    AnimalIndicatorText, CameraAngles, CameraMode, FreeFlyState, LastMoveDirection, Player,
    PlayerState, RenderConfig, SpawnedBlocks,
};

// ---- 重新导出 lk2-core PvP 数据（main.rs 里要直接用） ----
use lk2_core::pvp::{CombatState, Hitbox, WeaponStats};
use lk2_core::protocol::components::Health;

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
    let args: Vec<String> = std::env::args().collect();
    let offline_mode = args.iter().any(|a| a == "--offline");
    let auto_demo_mode = args.iter().any(|a| a == "--auto-demo");

    println!("[lk2-client] starting (offline={}, auto_demo={})", offline_mode, auto_demo_mode);

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

    // ----- App 装配 -----
    let mut app = App::new();

    // ===== 1. DefaultPlugins（含 winit / wgpu / WindowPlugin）=====
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("万国起源：最后一国 钻石版 — {}", scenario.name).into(),
                    resolution: WindowResolution::new(1280, 720),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::log::LogPlugin {
                level: bevy::log::Level::INFO,
                ..default()
            }),
    );

    // ===== 2. 物理（avian3d）=====
    app.add_plugins(PhysicsPlugins::default())
        .insert_resource(Gravity::default());

    // ===== 3. lightyear 0.26 ClientPlugins =====
    // 注意：当前（build-client）任务**还没接** UDP transport。`--offline` 模式
    // 下 Client 不会真去连 server；wire-network-and-loop task 会把
    // `ClientPlugins::net_config(...)` 加上 UDP transport。占位 default 配置
    // 启动起来 OK（Client entity 不 spawn 就啥也不发生）。
    //
    // 这里只 `add_plugins(ClientPlugins::default())`，并把协议 plugin 一起加，
    // 保证 register_message / register_component / InputPlugin 全部 init。
    app.add_plugins(lightyear::prelude::ClientPlugins::default());
    app.add_plugins(lk2_core::protocol::ProtocolPlugin);

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
    app.add_systems(
        Update,
        (
            // scenario / sim
            lk2_core::scenario::scenario_runner,
            lk2_core::scenario::simulate_player_actions,
            lk2_core::scenario::scenario_tick_recorder,
            // 客户端独有
            auto_demo,
            mouse_look_system,
            first_person_camera,
            held_weapon_follow,
            player_input,
            player_attack_creatures,
            animate_avatar,
            spawn_terrain_around_player,
            // F3 自由视角 / C 切 3rd person / F5 传送 / F8 切 preset
            freefly_toggle,
            camera_mode_toggle,
            emergency_teleport,
            cycle_terrain_preset,
            freefly_movement.before(first_person_camera),
            // PvP 客户端
            collect_local_input,
            client_attack_predict,
            on_hit_confirm,
            on_knockback_event,
            on_damage_result,
            trigger_visual_effects,
            // 控制器（地面 / WASD / 爬台阶 / 击退衰减）
            ground_detection,
            character_movement,
            auto_step_up,
            knockback_decay,
            collect_input,
        )
            .chain(),
    );

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
        use lk2_core::pvp::DamageEvent;
        use lk2_core::protocol::messages::{AttackInput, DamageResult, HitConfirm, KnockbackEvent};

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
        DirectionalLight {
            illuminance: 20000.0,
            shadows_enabled: false,
            ..default()
        },
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
    let sun_pos = Vec3::new(
        (t - 0.5) * 2.0 * dist,
        dayness * dist + 5.0,
        0.0,
    );
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

// ---- HUD ----
#[derive(Component)]
struct HudText;
#[derive(Component)]
struct HudFooter;

#[derive(Resource)]
struct UiFonts {
    cn: Handle<Font>,
}

fn setup_fonts(mut commands: Commands) {
    let cn: Handle<Font> = Handle::default();
    commands.insert_resource(UiFonts { cn });
}

fn setup_hud(mut commands: Commands, fonts: Res<UiFonts>) {
    commands.spawn((
        Text::new("WANGUO ORIGINS v0.4  loading..."),
        TextFont {
            font: fonts.cn.clone(),
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        TextShadow {
            offset: Vec2::new(2.0, 2.0),
            color: Color::srgba(0.0, 0.0, 0.0, 0.85),
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        HudText,
    ));

    // 屏幕中心：十字准星
    let cross_size = 16.0_f32;
    let cross_thickness = 2.0_f32;
    let cross_offset = -cross_size / 2.0;
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: px(cross_size),
            height: px(cross_thickness),
            margin: UiRect {
                left: Val::Px(cross_offset),
                top: Val::Px(cross_offset + (cross_size - cross_thickness) / 2.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
            },
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: px(cross_thickness),
            height: px(cross_size),
            margin: UiRect {
                left: Val::Px(cross_offset + (cross_size - cross_thickness) / 2.0),
                top: Val::Px(cross_offset),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
            },
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.95)),
    ));

    // 底部：操作 + 目标
    commands.spawn((
        Text::new(""),
        TextFont {
            font: fonts.cn.clone(),
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.7)),
        TextShadow {
            offset: Vec2::new(1.5, 1.5),
            color: Color::srgba(0.0, 0.0, 0.0, 0.85),
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
        HudFooter,
    ));

    // 顶部居中：动物方向指示器
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(56),
            left: px(0),
            right: px(0),
            height: px(36),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            Text::new("[scanning...]"),
            TextFont {
                font: fonts.cn.clone(),
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.9, 0.4)),
            TextShadow {
                offset: Vec2::new(1.5, 1.5),
                color: Color::srgba(0.0, 0.0, 0.0, 0.9),
            },
            AnimalIndicatorText,
        )],
    ));

    // 顶部右上：HP HUD（被 on_damage_result 刷新）
    commands.spawn((
        Text::new("❤ 20 / 20"),
        TextFont {
            font: fonts.cn.clone(),
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.4, 0.4)),
        TextShadow {
            offset: Vec2::new(1.5, 1.5),
            color: Color::srgba(0.0, 0.0, 0.0, 0.9),
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            right: px(12),
            ..default()
        },
        HealthHudMarker,
    ));
}

fn update_hud(
    mut q_top: Query<&mut Text, With<HudText>>,
    mut q_bot: Query<&mut Text, (With<HudFooter>, Without<HudText>)>,
    clock: Res<SimClock>,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    obs: Res<TickObserver>,
    time: Res<Time>,
) {
    let fps = (1.0 / time.delta_secs().max(0.001)).round() as i32;
    if let Ok(mut text) = q_top.single_mut() {
        **text = format!(
            "WANGUO ORIGINS v0.4  [{fps} fps]\n\
             tick {} ({:.1}s)\n\
             player @ {:?}\n\
             Wood={}  Food={}  Apple={}  Soul={}\n\
             flags={}/8  monsters={}\n\
             anomalies={}  invariants=ok",
            clock.tick,
            time.elapsed_secs(),
            player.block_pos,
            pool.get(ResourceKind::Wood),
            pool.get(ResourceKind::Food),
            pool.get(ResourceKind::Apple),
            pool.get(ResourceKind::Soul),
            nations.flag_count,
            monsters.current_individuals,
            obs.anomalies.len(),
        );
    }
    let wood = pool.get(ResourceKind::Wood);
    let goal = 10;
    let progress_bar = {
        let pct = (wood as f32 / goal as f32).clamp(0.0, 1.0);
        let filled = (pct * 16.0) as usize;
        format!("{}{}", "█".repeat(filled), "░".repeat(16 - filled))
    };
    let status = if wood >= goal {
        "*** WIN! 10 wood collected. Try Found Nation (F) ***"
    } else {
        ""
    };
    if let Ok(mut text) = q_bot.single_mut() {
        **text = format!(
            "[WASD] move  [Space] jump  [Shift] sneak  [G] gather  [K] sword  [Esc] quit\n\
             Goal: gather 10 wood    {wood}/{goal}  {progress_bar}\n\
             {status}",
        );
    }
}

// ---------------------------------------------------------------------------
// 客户端世界初始化：地形 + 玩家 + 资源 + 怪物
// ---------------------------------------------------------------------------

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
    info!(
        "[terrain] using preset '{}'",
        game_world.pipeline.name
    );

    for k in ResourceKind::ALL {
        let init = 50.min(k.max() / 2).max(10);
        let _ = pool.force_add(*k, init);
    }

    monsters.demo_init([
        constant::WORLD_SIZE / 2,
        constant::SEA_LEVEL + 1,
        constant::WORLD_SIZE / 2,
    ]);

    let (sx, sz) = walk_override_static().unwrap_or((constant::WORLD_SIZE / 2, constant::WORLD_SIZE / 2));
    let spawn = [sx, 30, sz];
    player.block_pos = spawn;
    player.pos = Vec3::new(
        spawn[0] as f32 + 0.5,
        spawn[1] as f32 + 0.5,
        spawn[2] as f32 + 0.5,
    );
    player.inventory.insert(ResourceKind::Wood, 0);
    player.inventory.insert(ResourceKind::Food, 5);

    info!(
        "🌍 世界已生成: {}³, 玩家在 {:?}",
        constant::WORLD_SIZE, spawn
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
    let mut pool = pool.clone();
    let mut monsters = MonsterEcosystem::clone(&*monsters);
    let mut violations: Vec<String> = Vec::new();
    for tick in 0..100 {
        obs.begin_tick();
        monsters.tick(&mut pool);
        let _ = pool.try_add(ResourceKind::Food, 2);
        if let Err(e) = obs.end_tick(
            tick,
            &game_world,
            &pool,
            &nations,
            &monsters,
            Some([
                constant::WORLD_SIZE / 2,
                constant::SEA_LEVEL + 2,
                constant::WORLD_SIZE / 2,
            ]),
        ) {
            violations.push(format!("tick {}: {}", tick, e.join("; ")));
        }
    }
    if violations.is_empty() {
        info!(">>> 自检 ✅ 100 tick 全部通过");
    } else {
        error!(">>> 自检 ❌ {} 处违例", violations.len());
    }
    info!("{}", obs.report());
}

// ---------------------------------------------------------------------------
// 客户端 simulation tick + tick 结束 invariants + 截图
// ---------------------------------------------------------------------------
//
// 在 `--offline` 模式下客户端**自己跑 sim**（loopback 给 demo 用）。联网模式
// 下 sim 应在 server 跑、client 只预测 — 那条路径在 wire-network-and-loop
// task 实现。

fn simulation_tick(
    time: Res<Time>,
    mut clock: ResMut<SimClock>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut obs: ResMut<TickObserver>,
) {
    let now = time.elapsed_secs();
    if now - clock.last_tick_wall < constant::SLOW_TICK_SECS {
        return;
    }
    clock.last_tick_wall = now;
    clock.tick += 1;
    let _ = pool.try_add(ResourceKind::Apple, 1);
    let _ = pool.try_add(ResourceKind::Food, 2);
    obs.begin_tick();
    monsters.tick(&mut pool);
    if clock.tick % 10 == 0 {
        info!(
            "⏱ tick {}: monsters={}, food={}",
            clock.tick,
            monsters.current_individuals,
            pool.get(ResourceKind::Food)
        );
    }
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
        &time, &clock, &player, &pool, &nations, &monsters, &obs, &game_world,
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
    mut clock: ResMut<SimClock>,
    player: Res<PlayerState>,
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
    let path = format!("screenshots/state_t{}.json", clock.tick);
    let state = build_state_json(
        &time, &clock, &player, &pool, &nations, &monsters, &obs, &game_world,
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
) -> serde_json::Value {
    serde_json::json!({
        "tick": clock.tick,
        "wall_secs": time.elapsed_secs(),
        "player": {
            "block_pos": player.block_pos,
            "pos": [player.pos.x, player.pos.y, player.pos.z],
            "nation_id": player.nation_id.map(|n| n.0),
            "monsters_killed": player.monsters_killed,
            "blocks_gathered": player.blocks_gathered,
            "nations_founded": player.nations_founded,
        },
        "pool": {
            "wood": pool.get(ResourceKind::Wood),
            "food": pool.get(ResourceKind::Food),
            "apple": pool.get(ResourceKind::Apple),
            "soul": pool.get(ResourceKind::Soul),
        },
        "nations": {
            "flag_count": nations.flag_count,
            "total_nations": nations.nations.len(),
        },
        "monsters": {
            "current": monsters.current_individuals,
            "kingdoms": monsters.kingdoms.len(),
            "nests": monsters.kingdoms.values().map(|k| k.nests.len() as u32).sum::<u32>(),
        },
        "observer": {
            "snapshots": obs.snapshots.len(),
            "decisions": obs.decisions.len(),
            "anomalies": obs.anomalies.len(),
            "invariant_violations": obs.invariants.values().map(|i| i.total_violations).sum::<u64>(),
        },
        "world": {
            "size": game_world.size,
        },
    })
}
