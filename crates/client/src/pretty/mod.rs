//! 视觉增强：水、玩家 avatar、怪物
//!
//! 全部在 startup 时 spawn 一次，运行期由 render 模块管理
//!
//! 包含：
//!   * 水面（半透明蓝平面，sea level）
//!   * 玩家 avatar（body + head + arm，像素人）
//!   * 怪物 cube（不同颜色代表不同类型）

use bevy::prelude::*;
use lk2_core::player::PlayerState;
use lk2_core::world::{Biome, World as GameWorld};

/// 视觉增强配置
#[derive(Resource, Debug, Clone)]
pub struct PrettyConfig {
    pub show_water: bool,
    pub show_player_avatar: bool,
    pub show_monster_cubes: bool,
}

impl Default for PrettyConfig {
    fn default() -> Self {
        Self { show_water: true, show_player_avatar: true, show_monster_cubes: true }
    }
}

/// 水面 entity（用于移动）
#[derive(Component)]
pub struct WaterMarker;

/// 玩家 avatar 各部件 marker
#[derive(Component)]
pub struct AvatarPart;

/// 怪物 marker
#[derive(Component)]
pub struct MonsterCube;

/// 启动时 spawn 水面 + 玩家 avatar
pub fn spawn_pretty(
    mut commands: Commands,
    game_world: Res<GameWorld>,
    player: Res<PlayerState>,
    cfg: Res<PrettyConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ---- 水面 ----
    if cfg.show_water {
        let s = (game_world.size as f32) * 1.5; // 比世界稍大，看着舒服
        let water_y = lk2_core::constant::SEA_LEVEL as f32 + 0.45; // 海平面 + 一点点浮空
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(s, s))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.45, 0.75, 0.7),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.2,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_translation(Vec3::new(s * 0.5, water_y, s * 0.5)),
            WaterMarker,
        ));
        info!("🌊 水面已 spawn（y={}）", water_y);
    }

    // ---- 玩家 avatar ----
    if cfg.show_player_avatar {
        let base = player.pos + Vec3::new(0.0, 0.0, 0.0);
        // 身体（红）
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            base + Vec3::new(0.0, 0.7, 0.0),
            Vec3::new(0.6, 0.9, 0.4),
            Color::srgb(0.85, 0.25, 0.25),
        );
        // 头（肤色）
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            base + Vec3::new(0.0, 1.45, 0.0),
            Vec3::new(0.55, 0.55, 0.55),
            Color::srgb(0.95, 0.78, 0.65),
        );
        // 头发（深棕）
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            base + Vec3::new(0.0, 1.78, 0.0),
            Vec3::new(0.6, 0.18, 0.6),
            Color::srgb(0.20, 0.12, 0.05),
        );
        // 眼睛
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            base + Vec3::new(-0.13, 1.5, -0.28),
            Vec3::new(0.10, 0.10, 0.05),
            Color::srgb(0.0, 0.0, 0.0),
        );
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            base + Vec3::new(0.13, 1.5, -0.28),
            Vec3::new(0.10, 0.10, 0.05),
            Color::srgb(0.0, 0.0, 0.0),
        );
        // 腿（深蓝）
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            base + Vec3::new(-0.13, 0.20, 0.0),
            Vec3::new(0.22, 0.45, 0.35),
            Color::srgb(0.15, 0.18, 0.55),
        );
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            base + Vec3::new(0.13, 0.20, 0.0),
            Vec3::new(0.22, 0.45, 0.35),
            Color::srgb(0.15, 0.18, 0.55),
        );
        // 旗杆（白色高杆）— 让玩家从远处也能看到
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            base + Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(0.08, 2.5, 0.08),
            Color::srgb(0.95, 0.95, 0.95),
        );
        // 旗面（鲜红色）
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            base + Vec3::new(0.4, 3.6, 0.0),
            Vec3::new(0.6, 0.4, 0.04),
            Color::srgb(0.95, 0.10, 0.10),
        );
        info!("🧍 玩家 avatar + 旗 已 spawn at {:?}", player.pos);
    }

    // ---- 怪物 cube（每只一种颜色） ----
    if cfg.show_monster_cubes {
        let monster_kinds = [
            (Color::srgb(0.5, 0.85, 0.2), "Snake"),
            (Color::srgb(0.3, 0.7, 0.95), "FrostElf"),
            (Color::srgb(0.95, 0.7, 0.2), "SandWurm"),
            (Color::srgb(0.4, 0.25, 0.1), "Treant"),
            (Color::srgb(0.7, 0.3, 0.85), "AetherWraith"),
        ];
        for (i, (color, _name)) in monster_kinds.iter().enumerate() {
            let angle = (i as f32) * 1.2566;
            let r = 8.0 + (i as f32) * 1.5;
            let x = player.pos.x + angle.cos() * r;
            let z = player.pos.z + angle.sin() * r;
            let y = player.pos.y + 0.5;
            spawn_cube(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(x, y, z),
                Vec3::new(0.8, 1.2, 0.8),
                *color,
            );
        }
        info!("👹 5 个怪物 cube 已 spawn");
    }

    // ---- 云朵（白色大方块漂在天上） ----
    for i in 0..6 {
        let cx = player.pos.x + ((i as f32) * 7.0 - 20.0);
        let cy = 24.0 + (i as f32) * 0.5;
        let cz = player.pos.z + ((i as f32) * 5.0 - 15.0);
        spawn_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(cx, cy, cz),
            Vec3::new(3.0 + (i as f32) * 0.4, 1.2, 2.0 + (i as f32) * 0.3),
            Color::srgba(1.0, 1.0, 1.0, 0.85),
        );
    }

    // ---- 树（深棕树干 + 绿色树冠） ----
    let tree_positions: [(i32, i32); 5] = [(5, 5), (-5, 3), (3, -7), (-4, -6), (8, -3)];
    for (_i, (tx, tz)) in tree_positions.iter().enumerate() {
        let t_x = (player.pos.x as i32 + tx).max(0) as f32;
        let t_z = (player.pos.z as i32 + tz).max(0) as f32;
        // 树干：3 格高
        for h in 0..3 {
            spawn_cube(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(t_x + 0.5, player.pos.y + 1.0 + h as f32, t_z + 0.5),
                Vec3::new(0.4, 1.0, 0.4),
                Color::srgb(0.45, 0.27, 0.10),
            );
        }
        // 树冠：2x2x2 绿色
        for dx in 0..2 {
            for dy in 0..2 {
                for dz in 0..2 {
                    spawn_cube(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        Vec3::new(
                            t_x - 0.5 + dx as f32,
                            player.pos.y + 4.0 + dy as f32,
                            t_z - 0.5 + dz as f32,
                        ),
                        Vec3::new(0.7, 0.7, 0.7),
                        Color::srgb(0.25, 0.55, 0.20),
                    );
                }
            }
        }
    }
}

fn spawn_avatar_cube(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    size: Vec3,
    color: Color,
) {
    let entity = spawn_cube(commands, meshes, materials, pos, size, color);
    commands.entity(entity).insert(AvatarPart);
}

fn spawn_cube(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    size: Vec3,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.6,
                metallic: 0.1,
                ..default()
            })),
            Transform::from_translation(pos),
        ))
        .id()
}

/// Update 玩家 avatar 位置（跟随 PlayerState）
pub fn follow_player_avatar(
    mut q: Query<&mut Transform, With<AvatarPart>>,
    player: Res<PlayerState>,
) {
    if !player.is_changed() {
        return;
    }
    for mut t in q.iter_mut() {
        t.translation = player.pos + (t.translation - player.pos).normalize_or_zero() * 0.0;
    }
}

/// 玩家上下浮动 + 旋转动画（更生动）
pub fn animate_avatar(
    time: Res<Time>,
    mut q: Query<&mut Transform, With<AvatarPart>>,
    player: Res<PlayerState>,
) {
    let t = time.elapsed_secs();
    let bob = (t * 2.0).sin() * 0.05;
    let base = player.pos;
    for (i, mut transform) in q.iter_mut().enumerate() {
        let offset = match i {
            0 => Vec3::new(0.0, 0.7 + bob, 0.0),     // body
            1 => Vec3::new(0.0, 1.45 + bob, 0.0),    // head
            2 => Vec3::new(0.0, 1.78 + bob, 0.0),    // hair
            3 => Vec3::new(-0.13, 1.5 + bob, -0.28), // L eye
            4 => Vec3::new(0.13, 1.5 + bob, -0.28),  // R eye
            5 => Vec3::new(-0.13, 0.20, 0.0),        // L leg
            6 => Vec3::new(0.13, 0.20, 0.0),         // R leg
            7 => Vec3::new(0.0, 3.0 + bob, 0.0),     // flag pole
            8 => Vec3::new(0.4, 3.6 + bob, 0.0),     // flag
            _ => Vec3::ZERO,
        };
        transform.translation = base + offset;
    }
}
