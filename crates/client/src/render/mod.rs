//! 体素渲染：把 World 的方块转成 PBR cube
//!
//! 策略：玩家周围 R 半径内的 solid 块 → spawn 一个 Mesh3d+MeshMaterial3d entity
//! 性能：3D scene 持 ~2000 个 entity 没问题；超过会卡

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::collections::{HashMap, HashSet};

use avian3d::prelude::{Collider, RigidBody};

use lk2_core::constant;
use lk2_core::creature::Creature;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::resource::ResourceKind;
use lk2_core::world::{BlockType, World as GameWorld};

mod greedy_mesh;
use greedy_mesh::build_all_terrain_meshes_aabb;

mod marching_cubes;
mod scalar_field;
mod smooth_mesh;

/// 体素渲染配置
#[derive(Resource, Debug, Clone)]
pub struct RenderConfig {
    pub radius: i32,       // 渲染半径（玩家 ±R）
    pub max_blocks: usize, // 一次性最多 spawn 多少个
    pub y_offset: f32,     // 玩家脚下贴图偏移（让 y=0 在地面）
    pub sky_color: Color,
    pub fog_color: Color,
    pub fog_start: f32,
    pub fog_end: f32,
    pub auto_orbit: bool,
    pub auto_orbit_speed: f32,
    pub auto_orbit_distance: f32,
    pub auto_walk: bool,
    pub auto_walk_interval_secs: f32,
    pub auto_keys: bool,      // --auto-demo 时自动按 F/J 测功能（不靠人按键）
    pub mouse_look: bool,     // 默认开：鼠标转视角；--auto-demo 关：用动物自动跟随
    pub smooth_terrain: bool, // 默认 true：标量场 + Marching Cubes 平滑地形（解决 cube 边角卡脚）
    pub smooth_passes: u32,   // Laplacian 平滑次数（0..=3），默认 0
    pub ground_step_threshold: f32, // 玩家移动"被卡"的软地表高度差阈值（默认 0.85）
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            radius: 20, // 之前 16，玩家只能看到 ±16 方块。升到 20 让世界看起来更辽阔
            max_blocks: 3000,
            y_offset: 0.0,
            sky_color: Color::srgb(0.45, 0.65, 0.95), // 亮天蓝
            fog_color: Color::srgb(0.75, 0.82, 0.95),
            fog_start: 18.0,
            fog_end: 48.0,
            auto_orbit: false,      // 默认玩家控制；--auto-demo 开启（loop.ps1 用）
            auto_orbit_speed: 0.30, // 0.22 太慢看不清全貌,0.30 12s 内能转接近半圈
            auto_orbit_distance: 6.5, // 15 太远,玩家在画面里就是个黑点;6.5 能看清 avatar + 周边
            auto_walk: false,       // 默认玩家控制；--auto-demo 开启
            auto_walk_interval_secs: 3.0, // 1.2 太频繁,玩家乱跑相机跟不住;3.0 让玩家多站一会儿
            auto_keys: false,       // --auto-demo 开启：自动按 F/J 验证
            mouse_look: true,       // 默认开：鼠标转视角（FPS 标准）
            smooth_terrain: true,   // 默认开：scalar field + MC
            smooth_passes: 0,       // v1 不平滑（先看效果）
            ground_step_threshold: 0.85, // 低矮起伏直接走，高墙才挡
        }
    }
}

/// 相机朝向（鼠标累积的 yaw + pitch）。mouse_look 系统读，first_person_camera 用
#[derive(Resource)]
pub struct CameraAngles {
    pub yaw: f32,   // 绕 +Y 轴，0 = 相机看 -Z；右转为负
    pub pitch: f32, // 绕相机右轴，0 = 水平；上视为正
}

impl Default for CameraAngles {
    fn default() -> Self {
        // 出生时明显俯视（约 -35°），让玩家第一眼看到脚下 + 远处地形
        Self { yaw: 0.0, pitch: -0.6 }
    }
}

/// 相机视角模式：C 键切换
#[derive(Resource, Default, PartialEq, Eq, Debug, Clone, Copy)]
pub enum CameraMode {
    /// 第一人称：相机在玩家眼睛位置
    #[default]
    FirstPerson,
    /// 第三人称：相机在玩家身后 3m，俯视玩家
    ThirdPerson,
}

/// 第三人称：相机到玩家的水平距离（m）+ 垂直抬高
const TP_DISTANCE: f32 = 4.0;
const TP_HEIGHT: f32 = 2.0;

/// 自由视角模式（F3 切换）：灵魂出窍，无视玩家位置和物理，自由飞
#[derive(Resource)]
pub struct FreeFlyState {
    pub enabled: bool,
    /// 世界坐标下的相机位置（独立于 PlayerState）
    pub position: Vec3,
    /// WASD 速度向量（用于平滑加减速）
    pub velocity: Vec3,
    /// 进入 freefly 时存的玩家格子位置，退出时还原。
    pub saved_player_pos: Option<[i32; 3]>,
    /// 进入 freefly 时存的玩家连续位置，退出时还原。
    pub saved_player_world_pos: Option<Vec3>,
}

impl Default for FreeFlyState {
    fn default() -> Self {
        Self {
            enabled: false,
            position: Vec3::new(48.5, 18.0, 48.5), // 默认从玩家出生点上方起
            velocity: Vec3::ZERO,
            saved_player_pos: None,
            saved_player_world_pos: None,
        }
    }
}

/// 自由视角移动速度（m/s）— 比步行快 20x，方便快速遍历
const FREEFLY_SPEED: f32 = 30.0;
/// Shift 加速倍率
const FREEFLY_BOOST: f32 = 3.0;

const MOUSE_SENS: f32 = 0.0022; // 弧度/像素（≈ 0.13°/像素）
const PITCH_LIMIT: f32 = 1.483; // ≈ 85°（防止翻转）
const YAW_QE_STEP: f32 = 22.5_f32.to_radians(); // Q/E 步进 22.5°（备胎）

/// 已 spawn 的 terrain entity 列表（用于 despawn 重生）
/// 视觉 + 碰撞 分开存：视觉走 greedy mesh 出 mesh3d 实体，碰撞走 trimesh 实体
#[derive(Resource, Default)]
pub struct SpawnedBlocks {
    pub visual_entities: Vec<Entity>,
    pub collider_entities: Vec<Entity>,
    /// 上次 spawn 时用的玩家位置（玩家移动 > 1 格才重新 spawn）
    pub last_player_block: [i32; 3],
}

/// 玩家 + 相机 marker
#[derive(Component)]
pub struct PlayerCube;

/// 启动时 spawn 玩家周围方块（Greedy Mesh + Trimesh 碰撞版）
///
/// 每个 renderable block type → 1 个 Mesh3d 实体（视觉）+ 1 个 Trimesh 实体（碰撞）。
/// 之前 naive 做法每方块一个 entity（3000+）→ 现在 ~12 个。
///
/// `cfg.smooth_terrain` = true 时走 scalar_field + Marching Cubes 路径：
///  - 一个 mesh + vertex color（grass/dirt/stone 分层）
///  - 解决 cube 边角卡脚问题
pub fn spawn_terrain_around_player(
    mut commands: Commands,
    game_world: Res<GameWorld>,
    cfg: Res<RenderConfig>,
    player: Res<PlayerState>,
    mut spawned: ResMut<SpawnedBlocks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    // 限流：同一次刷屏只 warn 一次（1 秒间隔，按真实时间）
    _last_warn_time: Local<f32>,
    // 节流：最近一次 re-mesh 用了多少 ms。60fps + Greedy Mesh ≈ 600-1200ms 一次，
    // 所以按帧调会被卡成 1fps。用真实时间节流，0.5s 内不重复 re-mesh。
    mut last_mesh_wall: Local<f32>,
) {
    // 1) 玩家没动 + 上次有 mesh → skip
    // 2) 玩家动了 + 距离上次 re-mesh < 0.5s → skip（防 auto-walk 每步卡顿）
    // 3) 否则 re-mesh
    let now = time.elapsed_secs();
    let moved = spawned.last_player_block != player.block_pos;
    if !moved && !spawned.visual_entities.is_empty() {
        return;
    }
    if moved && now - *last_mesh_wall < 1.5 && !spawned.visual_entities.is_empty() {
        return; // 0.5s → 1.5s 留时间给更大的 40³ re-mesh (12 type * 40³)
    }

    // 清掉上一次的（视觉 + 碰撞）
    for e in spawned.visual_entities.drain(..) {
        commands.entity(e).despawn();
    }
    for e in spawned.collider_entities.drain(..) {
        commands.entity(e).despawn();
    }

    // AABB 范围（统一：玩家周围 ±R，Y clamp 到 world 范围）
    let r = cfg.radius as i32;
    let py = player.block_pos[1];
    let y_min = (py - 20).max(0);
    let y_max = (py + 20).min(game_world.size as i32 - 1);
    let min = [player.block_pos[0] - r, y_min, player.block_pos[2] - r];
    let max = [player.block_pos[0] + r, y_max, player.block_pos[2] + r];

    // ─────────── 走 smooth path（默认）───────────
    if cfg.smooth_terrain {
        let started = time.elapsed_secs();
        let sm = smooth_mesh::build_smooth_mesh(&game_world, min, max, 0.5, cfg.smooth_passes);
        if let Some(sm) = sm {
            let total_tris = sm.collider_indices.len() / 3;
            // 单一 material：vertex color 模式 + 平滑 terrain
            let mat = materials.add(StandardMaterial {
                base_color: Color::WHITE, // vertex color 覆盖
                perceptual_roughness: 0.85,
                metallic: 0.0,
                ..default()
            });
            let mesh_handle = meshes.add(sm.mesh.clone());
            let visual = commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(mat),
                    Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                    TerrainChunk,
                ))
                .id();
            spawned.visual_entities.push(visual);
            // 碰撞：Trimesh（avian3d 0.6 要 Vec<Vec3> + Vec<[u32; 3]>）
            let collider_verts: Vec<Vec3> =
                sm.collider_trimesh.iter().map(|p| Vec3::new(p[0], p[1], p[2])).collect();
            let collider_indices: Vec<[u32; 3]> = sm
                .collider_indices
                .chunks(3)
                .filter(|c| c.len() == 3)
                .map(|c| [c[0], c[1], c[2]])
                .collect();
            let collider = Collider::trimesh(collider_verts, collider_indices);
            let collider_ent = commands
                .spawn((
                    RigidBody::Static,
                    collider,
                    Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                    TerrainChunk,
                ))
                .id();
            spawned.collider_entities.push(collider_ent);

            spawned.last_player_block = player.block_pos;
            let mesh_secs = time.elapsed_secs() - started;
            debug!(
                "🌊 smooth mesh: {} tris, passes={}, 耗时 {:.0}ms（玩家 {:?}）",
                total_tris,
                cfg.smooth_passes,
                mesh_secs * 1000.0,
                player.block_pos
            );
        } else {
            // 标量场全空（cave 都没有）→ 不 spawn 任何东西
            debug!("🌊 smooth mesh: 标量场全空（无 solid 在 AABB 内）");
            spawned.last_player_block = player.block_pos;
        }
        *last_mesh_wall = time.elapsed_secs();
        return;
    }

    // ─────────── 走 legacy greedy path（--legacy-voxel 启用）───────────
    // 1. 准备 12 种 BlockType 对应的材质（共享，减少 GPU 状态切换）
    let mut mats: HashMap<BlockType, Handle<StandardMaterial>> = HashMap::new();
    for bt in [
        BlockType::Dirt,
        BlockType::Stone,
        BlockType::Sand,
        BlockType::Snow,
        BlockType::Leaves,
        BlockType::Water,
        BlockType::Wood,
        BlockType::IronOre,
        BlockType::SunstoneOre,
        BlockType::FrostcoreOre,
        BlockType::LivingRoot,
        BlockType::BerryThicket,
    ] {
        let c = bt.debug_color_rgba();
        let emissive = if matches!(bt, BlockType::BerryThicket | BlockType::SunstoneOre) {
            Color::srgb(c[0] * 0.3, c[1] * 0.3, c[2] * 0.3)
        } else {
            Color::BLACK
        };
        // Water: 半透明 + 蓝绿色 alpha blend
        let is_water = matches!(bt, BlockType::Water);
        let material = if is_water {
            StandardMaterial {
                base_color: Color::srgba(c[0], c[1], c[2], 0.65),
                emissive: Color::srgb(0.05, 0.10, 0.18).into(),
                perceptual_roughness: 0.10,
                metallic: 0.30,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }
        } else {
            StandardMaterial {
                base_color: Color::srgba(c[0], c[1], c[2], c[3]),
                emissive: emissive.into(),
                perceptual_roughness: 0.85,
                metallic: 0.0,
                ..default()
            }
        };
        mats.insert(bt, materials.add(material));
    }

    // 2. Greedy Mesh：每个 block type 一个 mesh（玩家周围 AABB，41³ = ~70KB）
    let started = time.elapsed_secs();
    let block_meshes = build_all_terrain_meshes_aabb(&game_world, min, max);
    let mesh_count = block_meshes.len();
    let total_tris: usize = block_meshes.iter().map(|m| m.indices.len() / 3).sum();
    let mesh_secs = time.elapsed_secs() - started;

    // 3. Spawn 每个 mesh（视觉 + 碰撞）
    for bm in block_meshes {
        let mat = mats[&bm.block_type].clone();
        let bevy_mesh = bm.to_bevy_mesh();

        // 碰撞：跳过 water（玩家应该能穿过水；且 Trimesh 不适合双面薄面）
        let collider_opt = if matches!(bm.block_type, BlockType::Water) {
            None
        } else {
            Collider::trimesh_from_mesh(&bevy_mesh)
        };

        // 把 mesh 加进 assets（视觉用 handle；碰撞用 mesh 引用）
        let mesh_handle = meshes.add(bevy_mesh);

        // 视觉：Mesh3d + MeshMaterial3d，identity transform
        let visual = commands
            .spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat),
                Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                TerrainChunk,
            ))
            .id();
        spawned.visual_entities.push(visual);

        if let Some(collider) = collider_opt {
            let collider_ent = commands
                .spawn((
                    RigidBody::Static,
                    collider,
                    Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                    TerrainChunk,
                ))
                .id();
            spawned.collider_entities.push(collider_ent);
        }
    }

    spawned.last_player_block = player.block_pos;
    debug!(
        "🧱 greedy mesh: {} type(s), {} tris, 耗时 {:.0}ms（玩家 {:?}）",
        mesh_count,
        total_tris,
        mesh_secs * 1000.0,
        player.block_pos
    );
    *last_mesh_wall = time.elapsed_secs();
}

/// Terrain chunk marker（greedy mesh 出的视觉/碰撞实体）
#[derive(Component)]
pub struct TerrainChunk;

/// 天空颜色 + 雾 + 武器 spawn（spawn 时把剑挂到相机子节点上，跟着相机走）
pub fn setup_atmosphere(
    mut commands: Commands,
    cfg: Res<RenderConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Query<Entity, With<Camera3d>>,
) {
    // 雾
    // bevy 0.18: Fog 组件挂在 camera 上
    // 0.18 的 fog: use bevy::pbr::FogSettings 或者 scene::Fog
    // 这里用简化版 ClearColor
    commands.insert_resource(ClearColor(cfg.sky_color));

    // ── 武器：剑（handle 棕 + blade 银）— 小尺寸贴屏幕右下角，斜 15° ──
    let handle_mesh = meshes.add(Cuboid::new(0.18, 0.55, 0.18));
    let handle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.28, 0.12),
        perceptual_roughness: 0.6,
        emissive: Color::srgb(0.10, 0.06, 0.02).into(),
        ..default()
    });
    let blade_mesh = meshes.add(Cuboid::new(0.18, 1.20, 0.08));
    let blade_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.94, 1.00),
        perceptual_roughness: 0.20,
        metallic: 0.90,
        emissive: Color::srgb(0.25, 0.27, 0.40).into(),
        ..default()
    });
    let Ok(cam_entity) = camera.single() else {
        warn!("setup_atmosphere: 找不到 Camera3d 实体，剑不 spawn");
        return;
    };
    // 斜 15°（绕 Z 轴），让剑看起来"握着"
    let tilt = Quat::from_rotation_z(15_f32.to_radians());
    // 把手：相机本地，右下角
    let handle = commands
        .spawn((
            HeldWeaponPart,
            Mesh3d(handle_mesh),
            MeshMaterial3d(handle_mat),
            Transform::from_translation(Vec3::new(0.45, -0.55, -0.75)).with_rotation(tilt),
        ))
        .id();
    // 刀刃：把手正上方叠
    let blade = commands
        .spawn((
            HeldWeaponPart,
            Mesh3d(blade_mesh),
            MeshMaterial3d(blade_mat),
            Transform::from_translation(Vec3::new(0.45, 0.10, -0.75)).with_rotation(tilt),
        ))
        .id();
    commands.entity(cam_entity).add_child(handle);
    commands.entity(cam_entity).add_child(blade);
    info!("⚔ 剑已 spawn（缩小到右下角，斜 15°）");
}

/// 武器 marker：被 held_weapon_follow 系统认领
#[derive(Component)]
pub struct HeldWeaponPart;

/// 把剑贴在相机右前方，跟随相机 transform
/// 现在剑是相机的子节点，bevy 自动处理 transform 跟随；这个系统留作占位 / 以后做挥剑动画
pub fn held_weapon_follow() {
    // 剑现在是 Camera3d 的子节点，bevy 自动按相机本地坐标渲染
    // 后续如果要加挥剑动画：query 剑的 Transform + 计时器 + 修改 local Y rotation
}

/// 鼠标视角系统：读 AccumulatedMouseMotion 资源（bevy 0.18 每帧自动累加并清零）→ 累积到 CameraAngles
pub fn mouse_look_system(
    motion: Res<AccumulatedMouseMotion>,
    mut angles: ResMut<CameraAngles>,
    cfg: Res<RenderConfig>,
    freefly: Res<FreeFlyState>,
) {
    // 自由视角下强制开（脱离玩家也想用鼠标看）
    if !cfg.mouse_look && !freefly.enabled {
        return;
    }
    if motion.delta == Vec2::ZERO {
        return;
    }
    // FPS 标准：鼠标右滑 → 视角右转（yaw+）；鼠标上滑 → 抬头（pitch+）
    angles.yaw += motion.delta.x * MOUSE_SENS;
    angles.pitch -= motion.delta.y * MOUSE_SENS;
    angles.pitch = angles.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
}

/// F3 切换自由视角（灵魂出窍）。切换时把相机放到玩家头顶上方 18m
///
/// 重要：进入时**快照玩家位置**到 freefly.saved_player_pos，scenario / 玩家输入
/// 等其他系统继续运行可能挪动玩家；退出时**还原**到快照，保证身体没漂。
pub fn freefly_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut freefly: ResMut<FreeFlyState>,
    mut player: ResMut<PlayerState>,
) {
    if !keys.just_pressed(KeyCode::F3) {
        return;
    }
    freefly.enabled = !freefly.enabled;
    if freefly.enabled {
        // 进入：快照玩家位置，相机从玩家头顶 18m 起飞
        freefly.saved_player_pos = Some(player.block_pos);
        freefly.saved_player_world_pos = Some(player.pos);
        freefly.position = player.pos + Vec3::Y * 18.0;
        freefly.velocity = Vec3::ZERO;
        info!(
            "🕊 FreeFly ON — 玩家身体冻结在 {:?}，WASD 飞 / Space↑ / Shift↓ / 鼠标视角 / F3 回本体",
            player.block_pos
        );
    } else {
        // 退出：还原玩家位置（block_pos 和 pos 同步）
        if let Some(saved) = freefly.saved_player_pos.take() {
            let saved_pos = freefly.saved_player_world_pos.take().unwrap_or(Vec3::new(
                saved[0] as f32 + 0.5,
                saved[1] as f32,
                saved[2] as f32 + 0.5,
            ));
            info!(
                "🕊 FreeFly OFF — 还原玩家 {:?} -> {:?}",
                player.block_pos, saved
            );
            set_player_position(&mut player, saved_pos, saved);
        } else {
            info!("🕊 FreeFly OFF");
        }
        freefly.velocity = Vec3::ZERO;
    }
}

/// C 键切换 1st / 3rd person 视角
pub fn camera_mode_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<CameraMode>,
    freefly: Res<FreeFlyState>,
) {
    // FreeFly 模式下禁用 C 切换（避免模式冲突）
    if freefly.enabled {
        return;
    }
    if !keys.just_pressed(KeyCode::KeyC) {
        return;
    }
    *mode = match *mode {
        CameraMode::FirstPerson => {
            info!("📷 CameraMode → 3rd person");
            CameraMode::ThirdPerson
        }
        CameraMode::ThirdPerson => {
            info!("📷 CameraMode → 1st person");
            CameraMode::FirstPerson
        }
    };
}

/// F5 紧急传送：把玩家传回出生点的可站地面（卡在山里/找不到自己时救命用）
pub fn emergency_teleport(
    keys: Res<ButtonInput<KeyCode>>,
    game_world: Res<GameWorld>,
    mut player: ResMut<PlayerState>,
) {
    if !keys.just_pressed(KeyCode::F5) {
        return;
    }
    let x = lk2_core::constant::WORLD_SIZE / 2;
    let z = lk2_core::constant::WORLD_SIZE / 2;
    let Some((pos, block_pos)) = player_spawn_position_at(&game_world, x, z) else {
        warn!("🚨 F5 紧急传送失败：出生列没有可站位置");
        return;
    };
    warn!("🚨 F5 紧急传送： {:?} -> {:?}", player.block_pos, block_pos);
    set_player_position(&mut player, pos, block_pos);
}

/// F8 循环切换地形 preset
pub fn cycle_terrain_preset(keys: Res<ButtonInput<KeyCode>>, mut game_world: ResMut<GameWorld>) {
    if !keys.just_pressed(KeyCode::F8) {
        return;
    }
    let names = lk2_core::world::terrain::presets::preset_names();
    let current = game_world.pipeline.name.clone();
    let next_idx =
        names.iter().position(|n| *n == current).map(|i| (i + 1) % names.len()).unwrap_or(0);
    let next_name = names[next_idx];
    let new_pipeline = lk2_core::world::terrain::presets::by_name(next_name);
    let new_name = new_pipeline.name.clone();
    game_world.pipeline = std::sync::Arc::new(new_pipeline);
    info!("🌍 F8 切 preset: {} -> {}", current, new_name);
}

/// 自由视角下的移动：WASD + Space/Shift，按住持续移动（不像 player_input 那种按一下走一格）
pub fn freefly_movement(
    keys: Res<ButtonInput<KeyCode>>,
    mut freefly: ResMut<FreeFlyState>,
    angles: Res<CameraAngles>,
    time: Res<Time>,
) {
    if !freefly.enabled {
        return;
    }

    // 视野方向（完整 3D，包括 pitch — freefly 应该能飞高飞低）
    let (sy, cy) = angles.yaw.sin_cos();
    let (sp, cp) = angles.pitch.sin_cos();
    let forward = Vec3::new(sy * cp, sp, -cy * cp);
    // right = forward × Y（Y 是世界 up，freefly 也遵守世界 up 不翻滚）
    let right = forward.cross(Vec3::Y);
    // up = Y（不要 roll）
    let up = Vec3::Y;

    // 累加意图（按住 = 持续）
    let mut wish = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        wish += forward;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        wish -= forward;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        wish += right;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        wish -= right;
    }
    if keys.pressed(KeyCode::Space) {
        wish += up;
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        wish -= up;
    }
    // Q/E = 加速
    let speed = if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::KeyE) {
        FREEFLY_SPEED * FREEFLY_BOOST
    } else {
        FREEFLY_SPEED
    };

    // 平滑加减速：往 wish 方向 lerp
    let target = if wish.length() > 0.01 {
        wish.normalize() * speed
    } else {
        Vec3::ZERO
    };
    let dt = time.delta_secs();
    // 简化：直接 = target（无 lerp，避免复杂；玩家想要"立刻响应"）
    let v = target;
    freefly.velocity = v;
    let pos = freefly.position + v * dt;
    freefly.position = pos;
}

/// 锁光标到窗口中央 + 隐藏（FPS 标准）。mouse_look 关时不锁
pub fn setup_cursor_grab(
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    cfg: Res<RenderConfig>,
) {
    if !cfg.mouse_look {
        return;
    }
    if let Ok(mut cursor) = cursors.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// 动物方向指示器 marker（被 `update_animal_indicator` 系统刷新）
/// 原来在 `src/main.rs` 里定义，迁到 render 模块更近
#[derive(Component)]
pub struct AnimalIndicatorText;

/// 动物方向指示器系统：每帧找最近的动物 + 算相对相机的屏幕方向 → 更新顶部 HUD 文字
pub fn update_animal_indicator(
    mut q_text: Query<&mut Text, With<AnimalIndicatorText>>,
    player: Res<PlayerState>,
    camera: Query<&Transform, With<Camera3d>>,
    creatures: Query<&Creature>,
) {
    let Ok(mut text) = q_text.single_mut() else {
        return;
    };
    let px = player.block_pos[0] as f32 + 0.5;
    let pz = player.block_pos[2] as f32 + 0.5;

    // 找最近的动物（水平距离，限 30 格）
    let mut best: Option<(&Creature, f32)> = None;
    for c in creatures.iter() {
        let dx = c.block_pos[0] as f32 + 0.5 - px;
        let dz = c.block_pos[2] as f32 + 0.5 - pz;
        let d2 = dx * dx + dz * dz;
        if d2 < 900.0 && (best.is_none() || d2 < best.unwrap().1) {
            best = Some((c, d2));
        }
    }

    let Some((c, d2)) = best else {
        text.0 = "🔍 附近无动物（>30 格）".to_string();
        return;
    };
    let dist = d2.sqrt();
    let animal_v = Vec2::new(
        c.block_pos[0] as f32 + 0.5 - px,
        c.block_pos[2] as f32 + 0.5 - pz,
    );
    if animal_v.length() < 0.01 {
        text.0 = format!("· {} 就在脚下", c.kind.label_zh());
        return;
    }

    // 算相对相机的方向（→ 屏幕箭头）
    let arrow = if let Ok(tf) = camera.single() {
        let f = tf.forward();
        let cam = Vec2::new(f.x, f.z);
        let cam_n = if cam.length() > 0.01 {
            cam.normalize()
        } else {
            Vec2::new(1.0, 0.0)
        };
        let dot = animal_v.dot(cam_n);
        let cross = animal_v.x * cam_n.y - animal_v.y * cam_n.x;
        // cross < 0 = 动物在右；angle 量化到 8 方向
        let angle = cross.atan2(dot);
        let oct = ((-angle).to_degrees() / 45.0).round() as i32;
        match oct.rem_euclid(8) {
            0 => "↑",
            1 => "↗",
            2 => "→",
            3 => "↘",
            4 => "↓",
            5 => "↙",
            6 => "←",
            7 => "↖",
            _ => "·",
        }
    } else {
        "·"
    };

    // 用英文标签（默认字体没 CJK，全显示成 ↑ 难看）
    let label = match c.kind {
        lk2_core::creature::CreatureKind::Pig => "Pig",
        lk2_core::creature::CreatureKind::Sheep => "Sheep",
        lk2_core::creature::CreatureKind::Cow => "Cow",
        lk2_core::creature::CreatureKind::Chicken => "Chicken",
    };
    text.0 = format!("{}  {}  {:.1}m", arrow, label, dist);
}

/// 玩家最后移动的方向（用于第一人称相机看向方向）
#[derive(Resource, Default)]
pub struct LastMoveDirection(pub Vec3);

/// 玩家键盘输入：WASD 移动（相对相机方向）/ Space 跳 / Shift 下降 / Q E 转向 / G 采集 / K 杀动物 / F 造国 / J 杀怪 / Esc 退出
pub fn player_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut player: ResMut<PlayerState>,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<lk2_core::resource::GlobalResourcePool>,
    mut angles: ResMut<CameraAngles>,
    mut nations: ResMut<NationRegistry>,
    mut monsters: ResMut<MonsterEcosystem>,
    camera: Query<&Transform, With<Camera3d>>,
    time: Res<Time>,
    freefly: Res<FreeFlyState>,
    cfg: Res<RenderConfig>,
) {
    // FreeFly 模式下，WASD/Space/Shift/QE 全部交给 freefly_movement
    // 这里只保留鼠标视角外的「功能键」（G/F/J/K）
    let freefly_active = freefly.enabled;

    // 读相机当前朝向 → 算 forward / right（水平）
    // 读相机当前朝向 → 算 forward / right（水平）
    let cam_tf = camera.single().ok();
    let (forward, right) = if let Some(tf) = cam_tf {
        // bevy 0.18: Transform::forward() 返回 local -Z 在 world 中的方向（相机看哪里）
        let f = tf.forward();
        let f_h = Vec3::new(f.x, 0.0, f.z);
        let f_n = if f_h.length() > 0.01 {
            f_h.normalize()
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
        // right = fwd × Y：看着 -Z 时 right = +X（D 往右移，符合 FPS 习惯）
        let r = f_n.cross(Vec3::Y);
        (f_n, r)
    } else {
        (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0))
    };

    // 移动：W = +forward, S = -forward, A = -right, D = +right；相对相机方向
    // FreeFly 模式下 WASD/Space/Shift/QE 全部跳过（由 freefly_movement 处理）
    let mut d = Vec3::ZERO;
    if !freefly_active {
        if keys.just_pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::ArrowUp) {
            d += forward;
        }
        if keys.just_pressed(KeyCode::KeyS) || keys.just_pressed(KeyCode::ArrowDown) {
            d -= forward;
        }
        if keys.just_pressed(KeyCode::KeyA) || keys.just_pressed(KeyCode::ArrowLeft) {
            d -= right;
        }
        if keys.just_pressed(KeyCode::KeyD) || keys.just_pressed(KeyCode::ArrowRight) {
            d += right;
        }
        if keys.just_pressed(KeyCode::Space) {
            d += Vec3::Y;
        }
        if keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight) {
            d -= Vec3::Y;
        }
    }

    // 转向：Q 左转 22.5°，E 右转 22.5°（改 CameraAngles.yaw，相机跟）
    // FreeFly 下 Q/E 是加速键（见 freefly_movement），跳过这里
    if !freefly_active {
        if keys.just_pressed(KeyCode::KeyQ) {
            angles.yaw -= YAW_QE_STEP;
        } else if keys.just_pressed(KeyCode::KeyE) {
            angles.yaw += YAW_QE_STEP;
        }
    }

    // 把 d 量化成 [i32; 3] 1-格移动（选主轴）
    if d.length() > 0.01 {
        let mut di: [i32; 3] = [0, 0, 0];
        let ad = d.abs();
        if ad.x >= ad.y && ad.x >= ad.z {
            di[0] = d.x.signum() as i32;
        } else if ad.z >= ad.y {
            di[2] = d.z.signum() as i32;
        } else {
            di[1] = d.y.signum() as i32;
        }
        try_player_move(&mut player, &mut game_world, di, cfg.ground_step_threshold);
        // 玩家输入不再改 LastMoveDirection（让相机自动转动物 / Q E 改朝向）
    }

    // 采集：G 键 = 挖当前脚下方块
    if keys.just_pressed(KeyCode::KeyG) {
        let (x, y, z) = (
            player.block_pos[0],
            player.block_pos[1] - 1,
            player.block_pos[2],
        );
        let b = game_world.get(x, y, z);
        if b.is_solid() {
            if let Some((res, _)) = b.yields() {
                game_world.set(x, y, z, BlockType::Air);
                let _ = pool.try_add(res, 1);
                *player.inventory.entry(res).or_insert(0) += 1;
                player.blocks_gathered += 1;
                info!(
                    "⛏ 你挖了 {:?} (库存 {:?})",
                    res,
                    player.inventory.get(&res).copied().unwrap_or(0)
                );
            } else {
                info!("这个方块挖不出东西");
            }
        } else {
            info!("脚下没方块");
        }
    }

    // 杀动物：K 键（creature 系统自己处理）
    // （不在这写 — 由 creature::player_attack_creatures 系统响应）

    // 造国：F 键 = 在当前坐标立旗
    if keys.just_pressed(KeyCode::KeyF) {
        if player.nation_id.is_some() {
            info!("🚩 你已经是一个国家的王了，不能再立旗");
        } else {
            let tick_now = time.elapsed_secs() as u64;
            let cost = nations.next_flag_cost();
            match nations.found(
                &mut pool,
                0u32, // single-player：玩家固定 id=0
                format!("玩家之国@{:?}", player.block_pos),
                player.block_pos,
                tick_now,
            ) {
                Ok(id) => {
                    player.nation_id = Some(id);
                    player.nations_founded += 1;
                    info!("🚩 你建立了国家 {:?}（消耗 {} 灵魂）", id, cost);
                }
                Err(e) => {
                    info!("🚩 造国失败：{:?}", e);
                }
            }
        }
    }

    // 杀怪：J 键 = 攻击 2 格内最近怪物个体
    if keys.just_pressed(KeyCode::KeyJ) {
        let p = player.block_pos;
        let mut best: Option<(f32, u32, u32, u32)> = None; // (dist, kid, nid, iid)
        for (kid, k) in monsters.kingdoms.iter() {
            for (nid, n) in k.nests.iter() {
                for (iid, ind) in n.individuals.iter() {
                    let dx = (ind.position[0] - p[0]) as f32;
                    let dy = (ind.position[1] - p[1]) as f32;
                    let dz = (ind.position[2] - p[2]) as f32;
                    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                    if dist <= 2.0 {
                        match best {
                            Some((bd, _, _, _)) if dist >= bd => {}
                            _ => best = Some((dist, *kid, *nid, *iid)),
                        }
                    }
                }
            }
        }
        if let Some((dist, kid, nid, iid)) = best {
            if monsters.kill_individual(kid, nid, iid, &mut pool) {
                player.monsters_killed += 1;
                info!(
                    "⚔ 你击杀了怪物 (距离 {:.1} 格) kid={} nid={} iid={}",
                    dist, kid, nid, iid
                );
            }
        } else {
            info!("⚔ 2 格内没有怪物");
        }
    }
}

const PLAYER_BODY_CLEARANCE_BLOCKS: i32 = 2;
const MAX_SMOOTH_DROP: f32 = 6.0;

fn player_body_clear(world: &GameWorld, x: i32, foot_y: i32, z: i32) -> bool {
    if foot_y < 0 || foot_y + PLAYER_BODY_CLEARANCE_BLOCKS > world.size {
        return false;
    }
    for y in foot_y..(foot_y + PLAYER_BODY_CLEARANCE_BLOCKS) {
        if world.get(x, y, z).is_solid() {
            return false;
        }
    }
    true
}

fn standable_foot_y(
    world: &GameWorld,
    x: i32,
    z: i32,
    near_y: f32,
    max_step_up: f32,
) -> Option<i32> {
    let min_y = ((near_y - MAX_SMOOTH_DROP).floor() as i32).max(1);
    let max_y = ((near_y + max_step_up).ceil() as i32).min(world.size - 2);
    (min_y..=max_y)
        .filter(|foot_y| {
            world.get(x, *foot_y - 1, z).is_solid() && player_body_clear(world, x, *foot_y, z)
        })
        .min_by(|a, b| {
            let da = (*a as f32 - near_y).abs();
            let db = (*b as f32 - near_y).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn standable_foot_y_any_height(world: &GameWorld, x: i32, z: i32) -> Option<i32> {
    (1..(world.size - 2)).rev().find(|foot_y| {
        world.get(x, *foot_y - 1, z).is_solid() && player_body_clear(world, x, *foot_y, z)
    })
}

pub fn player_stand_position_at(
    world: &GameWorld,
    x: i32,
    z: i32,
    near_y: f32,
    max_step_up: f32,
) -> Option<(Vec3, [i32; 3])> {
    let foot_y = standable_foot_y(world, x, z, near_y, max_step_up)?;
    Some((
        Vec3::new(x as f32 + 0.5, foot_y as f32, z as f32 + 0.5),
        [x, foot_y, z],
    ))
}

pub fn player_spawn_position_at(world: &GameWorld, x: i32, z: i32) -> Option<(Vec3, [i32; 3])> {
    let foot_y = standable_foot_y_any_height(world, x, z)?;
    Some((
        Vec3::new(x as f32 + 0.5, foot_y as f32, z as f32 + 0.5),
        [x, foot_y, z],
    ))
}

fn set_player_position(player: &mut PlayerState, pos: Vec3, block_pos: [i32; 3]) {
    player.pos = pos;
    player.block_pos = block_pos;
}

/// 玩家移动：水平移动按软地表找落脚点，竖直移动只检查身体空间。
///
/// 这不是完整刚体物理；`PlayerState` 仍是 demo 的权威位置。关键是移动判定不再把
/// “目标脚下格是 solid” 当成卡住，而是找目标 XZ 附近可站的地面，所以方块边缘不会卡脚。
fn try_player_move(
    player: &mut PlayerState,
    game_world: &mut GameWorld,
    d: [i32; 3],
    threshold: f32,
) -> bool {
    if d[1] != 0 && d[0] == 0 && d[2] == 0 {
        let next_y = player.block_pos[1] + d[1];
        if !game_world.in_bounds(player.block_pos[0], next_y, player.block_pos[2]) {
            return false;
        }
        if !player_body_clear(game_world, player.block_pos[0], next_y, player.block_pos[2]) {
            return false;
        }
        let pos = Vec3::new(player.pos.x, next_y as f32, player.pos.z);
        set_player_position(
            player,
            pos,
            [player.block_pos[0], next_y, player.block_pos[2]],
        );
        return true;
    }

    let next_x = player.block_pos[0] + d[0];
    let next_z = player.block_pos[2] + d[2];
    let Some((pos, block_pos)) =
        player_stand_position_at(game_world, next_x, next_z, player.pos.y, threshold)
    else {
        return false;
    };

    if pos.y - player.pos.y > threshold {
        return false;
    }

    set_player_position(player, pos, block_pos);
    true
}

// ---------------------------------------------------------------------------
// 相机：auto_orbit 时绕玩家慢转；玩家控制时停在固定俯瞰角跟随玩家
// ---------------------------------------------------------------------------

/// 玩家 entity 的标记 component（和 PlayerState Resource 配合用）
#[derive(Component)]
pub struct Player;

/// 自动 demo 模式：每 N 秒随机移动玩家，让相机跟着转。
/// 移动沿用手动输入的软地表落脚逻辑，避免演示路径贴墙或卡边。
pub fn auto_demo(
    time: Res<Time>,
    mut player: ResMut<PlayerState>,
    mut game_world: ResMut<GameWorld>,
    cfg: Res<RenderConfig>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut last: ResMut<LastMoveDirection>,
    mut walk_timer: Local<f32>,
    mut walk_step: Local<u32>,
    mut auto_frame: Local<u32>,
) {
    // ── auto-demo 自动测试 F / J（不靠人按键，loop 也能验证）────────────
    // 注意：放在 auto_walk 检查之前 — auto-demo 模式下 auto_walk=false，
    // 但我们仍想跑 keypress 模拟来验证造国/杀怪逻辑。
    if cfg.auto_keys {
        *auto_frame += 1;
        // t=1.0s: 按 F（第一次造国，应成功 +20 souls）
        if *auto_frame == 60 {
            keys.press(KeyCode::KeyF);
        }
        if *auto_frame == 62 {
            keys.release(KeyCode::KeyF);
        }
        // t=4.0s: 按 J（杀怪 — 玩家身边可能没怪，info 一下即可）
        if *auto_frame == 240 {
            keys.press(KeyCode::KeyJ);
        }
        if *auto_frame == 242 {
            keys.release(KeyCode::KeyJ);
        }
        // t=8.0s: 再按 F（应失败：已有国家）
        if *auto_frame == 480 {
            keys.press(KeyCode::KeyF);
        }
        if *auto_frame == 482 {
            keys.release(KeyCode::KeyF);
        }
    }

    if !cfg.auto_walk {
        return;
    }

    // 玩家按了任何移动键 → 让位给真实输入
    if keys.pressed(KeyCode::KeyW)
        || keys.pressed(KeyCode::KeyA)
        || keys.pressed(KeyCode::KeyS)
        || keys.pressed(KeyCode::KeyD)
        || keys.pressed(KeyCode::Space)
        || keys.pressed(KeyCode::ShiftLeft)
    {
        return;
    }
    *walk_timer += time.delta_secs();
    if *walk_timer < cfg.auto_walk_interval_secs {
        return;
    }
    *walk_timer = 0.0;
    *walk_step += 1;

    // 8 水平方向 + 偶尔 Y 方向；移动函数负责找可站地面和身体空间。
    let all_dirs: [[i32; 3]; 9] = [
        [1, 0, 0],
        [-1, 0, 0],
        [0, 0, 1],
        [0, 0, -1],
        [1, 0, 1],
        [1, 0, -1],
        [-1, 0, 1],
        [-1, 0, -1],
        [0, 1, 0],
    ];
    // 过滤：前方 2 格都能按软地表落脚（避免被挡住后第一视角贴着墙看）
    let good_dirs: Vec<[i32; 3]> = all_dirs
        .iter()
        .filter(|d| {
            let nx1 = player.block_pos[0] + d[0];
            let nz1 = player.block_pos[2] + d[2];
            let nx2 = nx1 + d[0];
            let nz2 = nz1 + d[2];
            player_stand_position_at(
                &game_world,
                nx1,
                nz1,
                player.pos.y,
                cfg.ground_step_threshold,
            )
            .and_then(|(pos, _)| {
                player_stand_position_at(&game_world, nx2, nz2, pos.y, cfg.ground_step_threshold)
            })
            .is_some()
        })
        .copied()
        .collect();
    if good_dirs.is_empty() {
        return;
    }
    let d = good_dirs[(*walk_step as usize) % good_dirs.len()];

    let before = player.block_pos;
    if try_player_move(&mut player, &mut game_world, d, cfg.ground_step_threshold) {
        // 记下最后水平移动方向
        let dx = (player.block_pos[0] - before[0]) as f32;
        let dz = (player.block_pos[2] - before[2]) as f32;
        if dx != 0.0 || dz != 0.0 {
            let v = Vec3::new(dx, 0.0, dz);
            last.0 = v.normalize();
        }
    }
    // 周期性地采集脚下块（如果可采集）
    let cur = game_world.get(
        player.block_pos[0],
        player.block_pos[1] - 1,
        player.block_pos[2],
    );
    if let Some((res, _)) = cur.yields() {
        if cur.is_solid() {
            game_world.set(
                player.block_pos[0],
                player.block_pos[1] - 1,
                player.block_pos[2],
                BlockType::Air,
            );
            *player.inventory.entry(res).or_insert(0) += 1;
            player.blocks_gathered += 1;
        }
    }
}

/// 第一人称相机：
/// - mouse_look=true  → 用 CameraAngles（鼠标控制 yaw/pitch）
/// - mouse_look=false → 自动跟最近的可见动物（auto-demo 模式）
pub fn first_person_camera(
    mut q: Query<&mut Transform, With<Camera3d>>,
    time: Res<Time>,
    player: Res<PlayerState>,
    angles: Res<CameraAngles>,
    cfg: Res<RenderConfig>,
    last: Res<LastMoveDirection>,
    creatures: Query<&Creature>,
    world: Res<GameWorld>,
    freefly: Res<FreeFlyState>,
    mode: Res<CameraMode>,
    mut orbit_angle: Local<f32>,
) {
    let Ok(mut tf) = q.single_mut() else {
        return;
    };

    // F3 自由視点：相机从 freefly.position 起飞，完全脱离玩家
    if freefly.enabled {
        let (sy, cy) = angles.yaw.sin_cos();
        let (sp, cp) = angles.pitch.sin_cos();
        let dir = Vec3::new(sy * cp, sp, -cy * cp);
        let look_target = freefly.position + dir * 5.0;
        tf.translation = freefly.position;
        tf.look_at(look_target, Vec3::Y);
        return;
    }

    // auto-demo + auto-orbit：俯瞰 orbit 模式（之前这个分支不存在，玩家被第一人称贴脸，
    // 根本看不到自己的 avatar，所以 iter_85/90/96 的 player 维度都只拿 2-3 分）
    // 相机绕玩家在水平面上慢转，抬高 3m 俯视，dist 默认 6.5m
    if cfg.auto_orbit && !cfg.mouse_look {
        *orbit_angle += time.delta_secs() * cfg.auto_orbit_speed;
        let a = *orbit_angle;
        let target = player.pos + Vec3::Y * 1.0; // 看向玩家胸口/头部高度
        let cam_pos = target
            + Vec3::new(
                a.cos() * cfg.auto_orbit_distance,
                3.0,
                a.sin() * cfg.auto_orbit_distance,
            );
        tf.translation = cam_pos;
        tf.look_at(target, Vec3::Y);
        return;
    }

    // C 切 3rd person：相机放玩家身后 4m + 高 2m，俯视玩家
    if *mode == CameraMode::ThirdPerson {
        let (sy, cy) = angles.yaw.sin_cos();
        // yaw 对应水平 forward = (sy, 0, -cy)；相机在玩家身后 = -forward
        let back = Vec3::new(-sy, 0.0, cy);
        let target = player.pos + Vec3::Y * 1.4;
        let cam_pos = target + back * TP_DISTANCE + Vec3::new(0.0, TP_HEIGHT, 0.0);
        tf.translation = cam_pos;
        tf.look_at(target, Vec3::Y);
        return;
    }

    let eye = player.pos + Vec3::Y * 1.7;

    let dir = if cfg.mouse_look {
        // 鼠标视角：forward = (sin(yaw)cos(pitch), sin(pitch), -cos(yaw)cos(pitch))
        let (sy, cy) = angles.yaw.sin_cos();
        let (sp, cp) = angles.pitch.sin_cos();
        Vec3::new(sy * cp, sp, -cy * cp)
    } else {
        // 自动跟动物（auto-demo 模式）：找最近可见动物
        let mut candidates: Vec<(f32, [i32; 3])> = Vec::new();
        for c in creatures.iter() {
            let dx = (c.block_pos[0] as f32 + 0.5) - eye.x;
            let dz = (c.block_pos[2] as f32 + 0.5) - eye.z;
            let d2 = dx * dx + dz * dz;
            if d2 < 900.0 {
                candidates.push((d2, c.block_pos));
            }
        }
        candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut best: Option<[i32; 3]> = None;
        for (_d2, c) in candidates.iter().take(5) {
            let tx = c[0] as f32 + 0.5;
            let tz = c[2] as f32 + 0.5;
            let mut clear = true;
            for i in 1..=5 {
                let t = i as f32 / 5.0;
                let x = eye.x + (tx - eye.x) * t;
                let z = eye.z + (tz - eye.z) * t;
                let bx = x.floor() as i32;
                let bz = z.floor() as i32;
                let by = eye.y.floor() as i32;
                if world.in_bounds(bx, by, bz) && world.get(bx, by, bz).is_solid() {
                    clear = false;
                    break;
                }
            }
            if clear {
                best = Some(*c);
                break;
            }
        }
        if let Some(c) = best {
            let v = Vec3::new(
                (c[0] as f32 + 0.5) - eye.x,
                0.0,
                (c[2] as f32 + 0.5) - eye.z,
            );
            if v.length() > 0.01 {
                v.normalize()
            } else {
                Vec3::new(1.0, 0.0, 0.0)
            }
        } else if last.0.length() < 0.01 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            last.0.normalize()
        }
    };
    let look_target = eye + dir * 5.0 - Vec3::new(0.0, 1.0, 0.0);
    tf.translation = eye;
    tf.look_at(look_target, Vec3::Y);
}
