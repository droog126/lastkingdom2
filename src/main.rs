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

use crate::ai::{AiDecision, AiDecisionKind, TickObserver};
use crate::scenario::{Scenario, ScenarioState};
use crate::monster::MonsterEcosystem;
use crate::nation::{NationId, NationRegistry};
use crate::resource::{GlobalResourcePool, ResourceKind};
use crate::world::{BlockType, World as GameWorld, WorldGenerator};
use crate::render::{
    auto_demo, first_person_camera, held_weapon_follow, mouse_look_system, player_input,
    setup_atmosphere, setup_cursor_grab, spawn_terrain_around_player, update_animal_indicator,
    CameraAngles, PlayerState, RenderConfig, SpawnedBlocks,
};
use crate::pretty::{spawn_pretty, animate_avatar, PrettyConfig};
use crate::creature::{player_attack_creatures, spawn_creatures, update_creatures, CreatureSpawnerDone};

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
        Self {
            tick: 0,
            last_tick_wall: 0.0,
            last_hud_wall: 0.0,
            last_screenshot_wall: 0.0,
            screenshot_count: 0,
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

fn main() {
    // 读取剧本（从 argv[1] 加载，否则用默认）
    let args: Vec<String> = std::env::args().collect();
    let auto_demo_mode = args.iter().any(|a| a == "--auto-demo");
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
                // --auto-demo：玩家不动（保留在出生点附近，能看见起始牧场），
                // 相机用动物自动跟随（不要 mouse-look，无人按键）。loop 用来 AI 迭代。
                cfg.auto_walk = false;
                cfg.auto_orbit = false;
                cfg.auto_keys = true;  // 自动按 F/J 测造国 + 杀怪
                cfg.mouse_look = false; // 关掉鼠标视角，用自动动物跟随
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
        .init_resource::<CreatureSpawnerDone>()
        .insert_resource(scenario_state)
        .add_systems(
            Startup,
            (
                setup_camera,
                setup_light,
                setup_atmosphere,
                setup_cursor_grab,        // ← mouse_look 开时锁光标
                setup_world,
                spawn_pretty,
                spawn_creatures,
                setup_hud,
                self_check,
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
        .run();
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
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

fn setup_hud(mut commands: Commands) {
    // 左上：状态 HUD
    commands.spawn((
        Text::new("WANGUO ORIGINS v0.4  loading..."),
        TextFont {
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
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(50.0),
            top: px(50.0),
            width: px(12.0),
            height: px(12.0),
            margin: UiRect::all(Val::Px(-6.0)),  // 居中
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
    ));
    // 底部：操作 + 目标
    commands.spawn((
        Text::new(""),
        TextFont {
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
            Text::new("🔍 搜索中…"),
            TextFont {
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
        "🎉 胜利！采集满 10 块木头。试试造国（F）？"
    } else {
        ""
    };
    if let Ok(mut text) = q_bot.single_mut() {
        **text = format!(
            "WASD/方向键 移动  ·  Space 跳  ·  Shift 下  ·  G 采集  ·  K 挥剑  ·  Esc 退出\n\
             🎯 目标: 采集 10 块木头    {wood}/{goal}  {progress_bar}\n\
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
    *game_world = WorldGenerator::default().generate(constant::WORLD_SIZE);

    for k in ResourceKind::ALL {
        let init = 50.min(k.max() / 2).max(10);
        let _ = pool.force_add(*k, init);
    }

    monsters.demo_init([
        constant::WORLD_SIZE / 2,
        constant::SEA_LEVEL + 1,
        constant::WORLD_SIZE / 2,
    ]);

    let spawn = [
        constant::WORLD_SIZE / 2,
        constant::SEA_LEVEL + 1,  // 直接站在平地上
        constant::WORLD_SIZE / 2,
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
    let mut monsters = monsters.clone();
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
) {
    let now = time.elapsed_secs();
    if now - clock.last_screenshot_wall < 5.0 {
        return;
    }
    clock.last_screenshot_wall = now;
    clock.screenshot_count += 1;
    let path: PathBuf = format!("screenshots/iter_{:02}.png", clock.screenshot_count).into();
    info!("📸 截图 #{} → {}", clock.screenshot_count, path.display());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

fn exit_on_esc(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Escape) {
        std::process::exit(0);
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
    // 每 5 tick dump 一次
    if clock.tick % 5 != 0 || clock.tick == 0 {
        return;
    }
    rec.current_iter = clock.tick as u32;
    let path = format!("screenshots/state_t{}.json", clock.tick);
    let state = serde_json::json!({
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
    });
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, s);
        info!("📝 tick state dumped → {}", path);
    }
}
