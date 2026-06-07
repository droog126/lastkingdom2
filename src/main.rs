//! 万国起源：最后一国 钻石版 — Demo
//!
//! bevy 0.18.1 + 自动 demo + 截图
//! 运行：cargo run → 窗口自动转、玩家自动走、每 5 秒截图
//! 操作：ESC 退出；其他可选 WASD/G/F/U/J/T

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::WindowResolution;
use bevy::ecs::schedule::IntoScheduleConfigs;
use std::collections::HashMap;
use std::path::PathBuf;

mod constant;
mod resource;
mod world;
mod nation;
mod monster;
mod ai;
mod render;
mod creature;
mod pretty;
mod scenario;
mod network;
mod pvp;
mod controller;

use crate::ai::{AiDecision, AiDecisionKind, TickObserver};
use crate::scenario::{Scenario, ScenarioState};
use crate::monster::MonsterEcosystem;
use crate::nation::{NationId, NationRegistry};
use crate::resource::{GlobalResourcePool, ResourceKind};
use crate::world::{BlockType, World as GameWorld, WorldGenerator};
use crate::render::{
    auto_demo, first_person_camera, held_weapon_follow, mouse_look_system, player_input,
    setup_atmosphere, setup_cursor_grab, spawn_terrain_around_player, update_animal_indicator,
    CameraAngles, Player, PlayerState, RenderConfig, SpawnedBlocks,
};
use crate::pretty::{spawn_pretty, animate_avatar, PrettyConfig};
use crate::creature::{player_attack_creatures, spawn_creatures, update_creatures, CreatureSpawnerDone};
use crate::pvp::{PvPPlugin, WeaponId, WeaponStats, Hitbox, CombatState, Ping, PositionHistory, FixedTick};
use crate::controller::{ControllerPlugin, PvPController, PlayerCollider};
use avian3d::prelude::{LinearVelocity, RigidBody, Collider, PhysicsPlugins, Gravity};

// ---------------------------------------------------------------------------
// SimClock
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct SimClock {
    pub tick: u64,
    pub last_tick_wall: f32,
    pub last_hud_wall: f32,
    pub last_screenshot_wall: f32,
    pub screenshot_count: u32,
}

impl Default for SimClock {
    fn default() -> Self {
        // 扫 screenshots/iter_NN/ 找最大编号, 让 screenshot_count 接着涨 (避免覆盖老 iter)
        // loop.ps1 期望每轮一个独立目录, 但 SimClock 是 in-process 重置, 所以这里手动接力
        let mut max_iter: u32 = 0;
        if let Ok(entries) = std::fs::read_dir("screenshots") {
            for e in entries.flatten() {
                if let Some(name) = e.file_name().to_str() {
                    if let Some(rest) = name.strip_prefix("iter_") {
                        if let Ok(n) = rest.parse::<u32>() {
                            if n > max_iter { max_iter = n; }
                        }
                    }
                }
            }
        }
        Self {
            tick: 0,
            last_tick_wall: 0.0,
            last_hud_wall: 0.0,
            last_screenshot_wall: 0.0,
            screenshot_count: max_iter,  // 接力，避免覆盖
        }
    }
}

// ---------------------------------------------------------------------------
// CameraOrbit
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct CameraOrbit {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
}

impl Default for CameraOrbit {
    fn default() -> Self {
        Self {
            yaw: 0.5,
            pitch: 0.6,
            distance: 24.0,
            target: Vec3::new(
                constant::WORLD_SIZE as f32 * 0.5,
                constant::WORLD_SIZE as f32 * 0.4,
                constant::WORLD_SIZE as f32 * 0.5,
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

/// 启动时解析的 preset 名（setup_world 系统要读）
static PRESET_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

fn preset_name_static() -> &'static str {
    PRESET_NAME.get().map(|s| s.as_str()).unwrap_or("default")
}

/// --walk=x,z 解析的 spawn 位置
static WALK_OVERRIDE: std::sync::OnceLock<Option<(i32, i32)>> = std::sync::OnceLock::new();

fn walk_override_static() -> Option<(i32, i32)> {
    WALK_OVERRIDE.get().copied().flatten()
}

fn main() {
    // 读取剧本（从 argv[1] 加载，否则用默认）
    let args: Vec<String> = std::env::args().collect();
    let auto_demo_mode = args.iter().any(|a| a == "--auto-demo");
    // 地形 preset：--preset=default | flat | mountainous | lold_arena | random
    let preset_name = args.iter()
        .find(|a| a.starts_with("--preset="))
        .map(|a| a.trim_start_matches("--preset=").to_string())
        .unwrap_or_else(|| "default".to_string());
    let _ = PRESET_NAME.set(preset_name.clone());
    println!("[terrain] preset = {}", preset_name);

    // --walk=x,z：玩家在指定 XZ 出生（验证无限世界）
    let walk_pos = args.iter()
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
        // --auto-demo：用原地待命剧本（不 MoveTo），玩家就留在出生点看动物
        scenario::Scenario {
            name: "idle".into(),
            record_window: None,
            steps: vec![
                scenario::ScenarioStep::Log { msg: "=== idle: 玩家不动看动物 ===".into() },
                scenario::ScenarioStep::WaitTicks { ticks: 1000 },
            ],
        }
    } else {
        scenario::load_scenario_from_args_or_default(&args)
    };
    let has_scenario_file = args.iter().skip(1).any(|a| a.ends_with(".json"));
    let scenario_state = scenario::ScenarioState::from_scenario(scenario.clone());

    let _ = std::fs::create_dir_all("screenshots");

    // CLI: --auto-demo  开启自动走 + 自动 orbit（loop.ps1 用来做 AI 迭代）

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("万国起源：最后一国 钻石版 — {}", scenario.name).into(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<RenderConfig>()
        .init_resource::<CameraAngles>()
        .add_systems(Startup, move |mut cfg: ResMut<RenderConfig>| {
            if auto_demo_mode {
                // --auto-demo：玩家到处走走看地形（不按键），让截图能看到不同 preset/区域
                cfg.auto_walk = true;    // 让玩家自己随机走
                cfg.auto_orbit = true;   // 相机跟随（已被 auto_walk 接管时几乎不动）
                cfg.auto_keys = true;    // 自动按 F/J 测造国 + 杀怪
                cfg.mouse_look = false;  // 关掉鼠标视角
            }
        })
        .init_resource::<SpawnedBlocks>()
        .init_resource::<PrettyConfig>()
        .init_resource::<PlayerState>()
        .init_resource::<SimClock>()
        .init_resource::<CameraOrbit>()
        .init_resource::<GameWorld>()
        .init_resource::<GlobalResourcePool>()
        .init_resource::<NationRegistry>()
        .init_resource::<MonsterEcosystem>()
        .init_resource::<TickObserver>()
        .init_resource::<TickRecorder>()
        .init_resource::<TimeOfDay>()
        .init_resource::<crate::render::LastMoveDirection>()
        .init_resource::<crate::render::FreeFlyState>()
        .init_resource::<crate::render::CameraMode>()
        .init_resource::<CreatureSpawnerDone>()
        .insert_resource(scenario_state)
        // avian3d 物理引擎
        .add_plugins(PhysicsPlugins::default())
        .insert_resource(Gravity::default())
        // PvP 系统（服务端权威 + 客户端预测）
        .add_plugins(PvPPlugin)
        // 角色控制器（体素友好 + PvP）
        .add_plugins(ControllerPlugin)
        .add_systems(
            Startup,
            (
                setup_fonts,
                setup_camera,
                setup_light,
                setup_atmosphere,
                setup_cursor_grab,        // ← mouse_look 开时锁光标
                setup_world,
                spawn_pretty,
                spawn_creatures,
                setup_hud,
                self_check,
                setup_player_pvp,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                scenario::scenario_runner,
                scenario::simulate_player_actions,
                scenario::scenario_tick_recorder,
                auto_demo,
                mouse_look_system,           // ← 鼠标累积到 yaw/pitch（先于 camera）
                first_person_camera,
                held_weapon_follow,
                player_input,
                player_attack_creatures,
                animate_avatar,
                spawn_terrain_around_player,
            )
                .chain(),
        )
        .add_systems(
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
        )
        // F3 自由视角 + C 切 3rd person + F5 紧急传送 + F8 切地形 preset
        .add_systems(
            Update,
            (
                render::freefly_toggle,
                render::camera_mode_toggle,
                render::emergency_teleport,
                render::cycle_terrain_preset,
                render::freefly_movement.before(first_person_camera),
            ),
        )
        .run();
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        render::Player, // PvP 系统的玩家 entity 标记
    ));
}

/// 太阳 marker：让 day_night_cycle 系统找得到
#[derive(Component)]
pub struct Sun;

/// TimeOfDay：0.0 = 子夜, 0.5 = 正午, 1.0 = 子夜（循环）
#[derive(Resource)]
pub struct TimeOfDay(pub f32);

impl Default for TimeOfDay {
    fn default() -> Self { Self(0.5) }  // 初始：正午（最亮）
}

fn setup_light(mut commands: Commands) {
    // 一盏太阳光，方向由 day_night_cycle 实时更新
    commands.spawn((
        DirectionalLight {
            illuminance: 20000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        Sun,
    ));
    // 蓝色补光（夜里降低到接近 0，模拟月光/无光）
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
/// - 太阳 DirectionalLight 位置 + 颜色随时间变
/// - 天空 ClearColor 从蓝 → 黄昏橙 → 夜黑
/// - 蓝色补光在夜里变暗
pub fn day_night_cycle(
    time: Res<Time>,
    mut tod: ResMut<TimeOfDay>,
    mut sun: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut fill: Query<&mut DirectionalLight, (Without<Sun>, With<DirectionalLight>)>,
    mut clear: ResMut<ClearColor>,
) {
    // 60 真实秒 = 1 游戏日
    tod.0 = (tod.0 + time.delta_secs() / 60.0) % 1.0;
    let t = tod.0;

    // dayness: 0=子夜, 1=正午, 平滑曲线；t=0.25 / t=0.75 是日出/日落
    let dayness = (std::f32::consts::PI * t).sin().max(0.0);
    // sunset glow: 在日出/日落 (t≈0.25, 0.75) 时最强
    let sunset_glow = (1.0 - (2.0 * t - 1.0).abs()).powi(3);

    // 太阳位置：x 横扫，y 随 dayness 升降
    let dist = 80.0;
    let sun_pos = Vec3::new(
        (t - 0.5) * 2.0 * dist,           // 东→西扫
        dayness * dist + 5.0,            // 升到天上
        0.0,
    );
    if let Ok((mut tf, mut l)) = sun.single_mut() {
        *tf = Transform::from_translation(sun_pos).looking_at(Vec3::ZERO, Vec3::Y);
        l.illuminance = 1500.0 + 30000.0 * dayness;
        // 太阳颜色：日落偏橙，正午偏白
        l.color = Color::srgb(
            1.0 - 0.15 * sunset_glow,
            0.95 - 0.35 * sunset_glow,
            0.85 - 0.65 * sunset_glow,
        );
    }
    // 蓝色补光：白天弱，夜里几乎 0
    if let Ok(mut l) = fill.single_mut() {
        l.illuminance = 3000.0 * dayness + 150.0;
    }

    // 天空颜色：白天天蓝，黎明/黄昏偏橙，夜里暗蓝
    clear.0 = Color::srgb(
        0.04 + 0.41 * dayness + 0.60 * sunset_glow,
        0.06 + 0.59 * dayness + 0.30 * sunset_glow,
        0.16 + 0.79 * dayness + 0.10 * sunset_glow,
    );
}

/// 屏幕 HUD 文字（左上角）
#[derive(Component)]
struct HudText;

/// 屏幕 HUD 文字（左下角：操作 + 目标）
#[derive(Component)]
struct HudFooter;

/// 字体资源：HUD 全部走思源黑体（含完整 CJK 字形）。默认 FiraSans/Mono 不含中文字形 → 豆腐块。
#[derive(Resource)]
struct UiFonts { cn: Handle<Font> }

fn setup_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    // 用 bevy 内置默认字体 (Fira Mono)，不再依赖外部 NotoSansCJKsc 字体文件 —
    // 之前找不到 fonts/NotoSansCJKsc-Regular.otf 导致 HUD 全部渲染成豆腐方块
    let cn: Handle<Font> = Handle::default();
    let _ = asset_server;  // 保留 import 以防别处用
    commands.insert_resource(UiFonts { cn });
}

fn setup_hud(mut commands: Commands, fonts: Res<UiFonts>) {
    // 左上：状态 HUD
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
    // 屏幕中心：十字准星（瞄准提示）
    // 用 Val::Percent(50.0) + 负 margin 居中（之前 px(50.0) 是离左上 50px 的位置）
    // 两条十字线（横+竖），用 Node + 高对比颜色，浅色方块上也能看见
    let cross_size = 16.0_f32;
    let cross_thickness = 2.0_f32;
    let cross_offset = -cross_size / 2.0;  // 居中
    // 水平横线
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
    // 垂直竖线
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
    // 顶部居中：动物方向指示器（被 update_animal_indicator 刷新）
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
}

/// 动物方向指示器 marker
#[derive(Component)]
pub struct AnimalIndicatorText;

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
    let fps = (1.0 / time.delta_secs()).round() as i32;
    if let Ok(mut text) = q_top.single_mut() {
        **text = format!(
            "WANGUO ORIGINS v0.4  [{fps} fps]\n\
             tick {} ({:.1}s)\n\
             player @ {:?}\n\
             Wood={}  Food={}  Apple={}  Soul={}\n\
             flags={}/8  monsters={}\n\
             anomalies={}  invariants=ok",
            clock.tick, time.elapsed_secs(), player.block_pos,
            pool.get(ResourceKind::Wood), pool.get(ResourceKind::Food),
            pool.get(ResourceKind::Apple), pool.get(ResourceKind::Soul),
            nations.flag_count, monsters.current_individuals,
            obs.anomalies.len(),
        );
    }
    // 底部：操作 + 目标
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

#[allow(clippy::too_many_arguments)]
fn setup_world(
    mut commands: Commands,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<GlobalResourcePool>,
    mut monsters: ResMut<MonsterEcosystem>,
    mut player: ResMut<PlayerState>,
) {
    // 用 CLI 选择的 preset 生成地形（如果 preset=lold_arena 每次会随机）
    let pipeline = crate::world::terrain::presets::by_name(preset_name_static());
    *game_world = crate::world::World::with_pipeline(constant::WORLD_SIZE, pipeline);
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

    // 出生点：地图正中心 + Y=15（接近地面，让玩家直接看到起始牧场动物和方块）
    // 之前是 Y=80 太高（看不到细节），然后 Y=25 还是偏上（牧场动物在 Y=12 被相机边缘切掉）
    // 如果用户传了 --walk=x,z，则在指定 XZ 出生（验证无限世界）
    let (sx, sz) = walk_override_static().unwrap_or((constant::WORLD_SIZE / 2, constant::WORLD_SIZE / 2));
    let spawn = [
        sx,
        30,  // 之前是 15，太低看不到山；30 起步 + 相机略俯视，出生第一眼就看到地形
        sz,
    ];
    player.block_pos = spawn;
    player.pos = Vec3::new(
        spawn[0] as f32 + 0.5,
        spawn[1] as f32 + 0.5,
        spawn[2] as f32 + 0.5,
    );
    player.inventory.insert(ResourceKind::Wood, 0);
    player.inventory.insert(ResourceKind::Food, 5);

    info!("🌍 世界已生成: {}³, 玩家在 {:?}", constant::WORLD_SIZE, spawn);
}

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
            Some([constant::WORLD_SIZE / 2, constant::SEA_LEVEL + 2, constant::WORLD_SIZE / 2]),
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
// Simulation
// ---------------------------------------------------------------------------

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
// 截图：每 5 秒一张
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

    // Sprint 1: 改为 iter_NN/ 子目录（每轮一个独立目录）
    // 用 :02 与 loop.ps1 对齐, 排序稳定。
    let iter_id = clock.screenshot_count;
    let iter_dir = format!("screenshots/iter_{:02}", iter_id);
    let _ = std::fs::create_dir_all(&iter_dir);
    let png_path: PathBuf = format!("{}/iter_{:02}.png", iter_dir, iter_id).into();
    let state_path = format!("{}/final_state.json", iter_dir);

    // 1) 截图
    info!("📸 截图 #{} → {}", iter_id, png_path.display());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(png_path));

    // 2) 顺便 dump 本轮 final_state.json（AI 直接读这个就知道"这轮游戏死没死"）
    let state = build_state_json(&time, &clock, &player, &pool, &nations, &monsters, &obs, &game_world);
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

fn setup_player_pvp(
    mut commands: Commands,
    player: Query<Entity, With<Player>>,
) {
    // 玩家已经在 Startup 时 spawn 了（通过 render 模块）
    // 这里给它们加上 PvP 组件 + 物理组件
    let iron = WeaponId::IronSword.stats();
    for entity in player.iter() {
        commands.entity(entity).insert((
            // avian3d 物理组件
            RigidBody::Kinematic,  // Kinematic 角色控制器（不受物理力影响）
            Collider::capsule(0.3, 0.9), // 胶囊体：半径 0.3m，半高 0.9m
            LinearVelocity::default(),
            // 角色控制器组件
            PvPController::new()
                .with_speed(5.0)
                .with_jump(8.0)
                .with_knockback_resistance(0.1),
            PlayerCollider::default(),
            // PvP 战斗组件
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
            Ping(0.0),
            PositionHistory::new(60),
            crate::network::protocols::components::Health(20.0),
        ));
        info!("⚔ PvP 组件已挂载（铁剑 reach={}, dmg={}）+ 角色控制器", iron.reach, iron.damage);
    }
}

// ---------------------------------------------------------------------------
// Tick-level 录制：每 5 tick dump 一次 JSON 到 screenshots/iter_NN_state.json
// 截图系统结束后，agent 可以读这个 JSON 知道 sim 的精确状态
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
    // 每 5 tick dump 一次（按 clock.tick 变化触发，不能按帧判，否则 clock.tick=10 时
    // 每帧都满足 % 5==0，会每帧写文件）
    if clock.tick == 0 || clock.tick % 5 != 0 || clock.tick == rec.last_dump_tick {
        return;
    }
    rec.last_dump_tick = clock.tick;
    rec.current_iter = clock.tick as u32;
    let path = format!("screenshots/state_t{}.json", clock.tick);
    let state = build_state_json(&time, &clock, &player, &pool, &nations, &monsters, &obs, &game_world);
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, s);
        info!("📝 tick state dumped → {}", path);
    }
}

/// 构造完整 sim state JSON（被 tick_recorder + periodic_screenshot 复用）
/// schema: { tick, wall_secs, player, pool, nations, monsters, observer, world }
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
