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

/// 玩家脚下外圈深绿地盘（草地块）— 跟随玩家移动
#[derive(Component)]
pub struct GroundDiscOuter;

/// 玩家脚下内圈亮绿地盘（"小广场"）— 跟随玩家移动
#[derive(Component)]
pub struct GroundDiscInner;

/// 每帧把外圈+内圈圆盘跟到玩家 XZ 位置。
/// 玩家在 setup_world 之后移动时，ground 不会"留在原地"漂浮。
/// 用单个 query (Without<AvatarPart, MonsterCube>) 一次拿所有 ground disc entity,
/// 循环内按 marker 区分 y 偏移 — 避免双 query B0001 conflict。
pub fn follow_ground_discs(
    player: Res<PlayerState>,
    mut q: Query<
        (&mut Transform, Option<&GroundDiscOuter>, Option<&GroundDiscInner>),
        (Without<AvatarPart>, Without<MonsterCube>),
    >,
) {
    let ground_y = -0.5;
    for (mut t, is_outer, is_inner) in &mut q {
        let dy = if is_outer.is_some() {
            ground_y - 0.05
        } else if is_inner.is_some() {
            ground_y + 0.13
        } else {
            ground_y
        };
        t.translation = Vec3::new(player.pos.x, player.pos.y + dy, player.pos.z);
    }
}

/// 玩家 avatar 各部件 marker + 相对玩家的偏移
#[derive(Component)]
pub struct AvatarPart {
    pub offset: Vec3,
}

/// 怪物 marker + 相对玩家 offset（spawn 时记录，follow 时跟随 player 移动）
#[derive(Component)]
pub struct MonsterCube {
    pub base: Vec3,
}

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
        let water_y = lk2_core::constant::SEA_LEVEL as f32 + 1.5; // 海平面 + 1.5m，orbit 相机 14m 高能瞥见
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(s, s))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.20, 0.38, 0.58, 0.75),
                alpha_mode: AlphaMode::Blend,
                // 提高 roughness 0.2→0.85: 减少阳光直射水面时的爆光亮点, 看着更像湖水而非镜子
                perceptual_roughness: 0.85,
                metallic: 0.0,
                // 加 reflectiveness 衰减: 让水更像哑光, 不出现太阳的圆点
                reflectance: 0.15, // 默认 0.5 → 0.15 减反射强度
                ..default()
            })),
            Transform::from_translation(Vec3::new(s * 0.5, water_y, s * 0.5)),
            WaterMarker,
        ));
        info!("🌊 水面已 spawn（y={}, 哑光版）", water_y);
    }

    // ---- 玩家脚下"基地盘"（给画面一个明确的"地面"感，避免漂浮） ----
    // 双层圆盘: 外圈深绿当草地, 内圈亮绿当"小广场"
    // 加大: 外圈半径 4m (直径 8m), 内圈 1.8m, 高差 0.18m 让"台阶"明显
    {
        let ground_y = -0.5; // 玩家脚下 0.5m（相对玩家）

        // 外圈大圆盘 (直径 8.0)
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(4.0, 0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.32, 0.48, 0.20),
                emissive: Color::srgb(0.02, 0.04, 0.015).into(),
                perceptual_roughness: 0.95,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_translation(Vec3::new(
                player.pos.x,
                player.pos.y + ground_y - 0.05,
                player.pos.z,
            )),
            GroundDiscOuter,
        ));

        // 内圈小圆盘 (直径 3.6) — 高 0.18m 让台阶明显
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(1.8, 0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.62, 0.28),
                emissive: Color::srgb(0.03, 0.05, 0.02).into(),
                perceptual_roughness: 0.92,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_translation(Vec3::new(
                player.pos.x,
                player.pos.y + ground_y + 0.13,
                player.pos.z,
            )),
            GroundDiscInner,
        ));
    }

    // ---- 玩家 avatar ----
    // 简化: 只留身体 + 头, 砍头发/眼睛/旗杆/旗面/旗面条/腿
    // 之前 9 块拼出来远看是"一坨", 现在 2 块清晰, 比例正常
    if cfg.show_player_avatar {
        // 身体（红）
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            player.pos + Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 2.0, 0.7),
            Color::srgb(0.95, 0.30, 0.30),
            Vec3::new(0.0, 1.0, 0.0),
        );
        // 头（肤色, 顶上 0.15m 让衔接自然）
        spawn_avatar_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            player.pos + Vec3::new(0.0, 2.55, 0.0),
            Vec3::new(0.7, 0.7, 0.7),
            Color::srgb(0.98, 0.82, 0.68),
            Vec3::new(0.0, 2.55, 0.0),
        );
        info!("🧍 玩家 avatar (简版 body+head) 已 spawn at {:?}", player.pos);
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
            // 从 8-15m 缩到 3-5m，让 demo 怪物一定在玩家视野内（画面更满）
            let r = 10.0 + (i as f32) * 1.8;
            let offset = Vec3::new(angle.cos() * r, 0.5, angle.sin() * r);
            let pos = player.pos + offset;
            let entity = spawn_cube(
                &mut commands,
                &mut meshes,
                &mut materials,
                pos,
                Vec3::new(0.8, 1.2, 0.8),
                *color,
            );
            commands.entity(entity).insert(MonsterCube { base: pos });
        }
        info!("👹 5 个怪物 cube 已 spawn");
    }

    // ---- 云朵（白色大方块漂在天上） ----
    for i in 0..6 {
        let cx = player.pos.x + ((i as f32) * 9.0 - 26.0);
        let cy = player.pos.y + 11.0 + (i as f32) * 0.4;
        let cz = player.pos.z + ((i as f32) * 6.0 - 20.0);
        spawn_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(cx, cy, cz),
            Vec3::new(2.8 + (i as f32) * 0.3, 0.35, 1.1 + (i as f32) * 0.2),
            Color::srgb(0.82, 0.88, 0.95),
        );
    }

    // ---- 树（深棕树干 + 绿色树冠）— 8 棵绕玩家圆周分布 ----
    let ground_y = player.pos.y - 2.0;
    // 8 棵树, 半径 20m (从 13m 拉远避免 14m 俯瞰相机时树挡在玩家面前成一坨)
    // 树干砍到 2 格, 树冠 1x2x1 缩小, 让树不再喧宾夺主
    for i in 0..8 {
        let angle = (i as f32) * (std::f32::consts::TAU / 8.0);
        let r = 20.0;
        let t_x = player.pos.x + angle.cos() * r;
        let t_z = player.pos.z + angle.sin() * r;
        // 树干：2 格高 (砍掉 1 格)
        for h in 0..2 {
            spawn_cube(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(t_x, ground_y + 1.0 + h as f32, t_z),
                Vec3::new(0.4, 1.0, 0.4),
                Color::srgb(0.45, 0.27, 0.10),
            );
        }
        // 树冠：1x2x1 绿色 (砍 2x2x2→1x2x1, 体积减半)
        for dy in 0..2 {
            spawn_cube(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(t_x, ground_y + 3.0 + dy as f32, t_z),
                Vec3::new(0.9, 0.9, 0.9),
                Color::srgb(0.25, 0.55, 0.20),
            );
        }
    }

    // ---- 石头（小灰块散布在 spawn 周围 4-6m 外圈，10 块） ----
    // 给画面增加"野外"质感，不再只有树和怪物
    let rock_positions: [(f32, f32, f32); 10] = [
        (4.5, 4.5, 0.7),
        (-5.2, 4.8, 0.55),
        (3.5, -5.5, 0.65),
        (-3.2, -6.5, 0.45),
        (5.8, -1.5, 0.85),
        (-6.0, -1.0, 0.5),
        (6.0, 5.5, 0.6),
        (-5.5, -3.5, 0.75),
        (1.5, 6.5, 0.4),
        (-1.5, -7.0, 0.5),
    ];
    for (i, (rx, rz, scale)) in rock_positions.iter().enumerate() {
        // 三种灰混搭, 让石头有变化
        let rock_color = match i % 3 {
            0 => Color::srgb(0.42, 0.42, 0.45), // 深灰
            1 => Color::srgb(0.58, 0.55, 0.50), // 中灰
            _ => Color::srgb(0.50, 0.52, 0.48), // 灰绿
        };
        let r_x = player.pos.x + rx * 1.8;
        let r_z = player.pos.z + rz * 1.8;
        spawn_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(r_x, ground_y + 0.4 * scale, r_z),
            Vec3::new(*scale, *scale * 0.7, *scale),
            rock_color,
        );
    }

    // ---- 花朵（小彩色斑点缀在 spawn 周围 2-6m，10 朵） ----
    let flower_positions: [(f32, f32); 10] = [
        (2.5, 2.5),
        (-2.8, 3.2),
        (3.2, -1.5),
        (-3.5, -2.5),
        (4.2, -0.8),
        (-4.8, 1.5),
        (1.2, -4.5),
        (5.0, 3.5),
        (-4.2, -5.0),
        (3.8, 5.2),
    ];
    let flower_colors = [
        Color::srgb(0.98, 0.30, 0.55), // 粉红
        Color::srgb(1.0, 0.85, 0.20),  // 黄
        Color::srgb(0.55, 0.30, 0.98), // 紫
        Color::srgb(1.0, 0.45, 0.20),  // 橙
        Color::srgb(0.95, 0.30, 0.30), // 红
    ];
    for (i, (fx, fz)) in flower_positions.iter().enumerate() {
        let f_color = flower_colors[i % flower_colors.len()];
        let f_x = player.pos.x + fx * 2.0;
        let f_z = player.pos.z + fz * 2.0;
        spawn_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(f_x, ground_y + 0.4, f_z),
            Vec3::new(0.4, 0.55, 0.4), // 加大 0.3→0.4 让花更显眼
            f_color,
        );
    }

    // ---- 远景山丘（大绿块在 15m 外, 给画面深度感） ----
    // 4 个方向各放一块, 距离 15m, 高度 3.0m, 提到玩家头顶高度
    let hill_distance = 28.0;
    let hill_offsets: [(f32, f32); 4] = [
        (hill_distance, hill_distance),
        (-hill_distance, hill_distance),
        (hill_distance, -hill_distance),
        (-hill_distance, -hill_distance),
    ];
    for (hx, hz) in hill_offsets.iter() {
        let h_x = player.pos.x + hx;
        let h_z = player.pos.z + hz;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(5.0, 3.0, 5.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.20, 0.42, 0.18),
                emissive: Color::srgb(0.04, 0.08, 0.03).into(),
                perceptual_roughness: 0.95,
                metallic: 0.0,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_translation(Vec3::new(h_x, ground_y + 2.5, h_z)),
        ));
    }
}

fn spawn_avatar_cube(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    size: Vec3,
    color: Color,
    offset: Vec3, // 相对 player.pos 的偏移，follow 时用
) {
    let entity = spawn_cube(commands, meshes, materials, pos, size, color);
    commands.entity(entity).insert(AvatarPart { offset });
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
                // 弱 emissive 防止 fog 把它吃掉；旗面在 spawn 处再强盖
                emissive: (color.to_linear() * 0.25).into(),
                perceptual_roughness: 0.6,
                metallic: 0.1,
                ..default()
            })),
            Transform::from_translation(pos),
        ))
        .id()
}

/// Update 玩家 avatar 位置（跟随 PlayerState）
/// avatar 在 spawn 时存了相对 player.pos 的 offset，每帧 t.translation = player.pos + offset
pub fn follow_player_avatar(mut q: Query<(&mut Transform, &AvatarPart)>, player: Res<PlayerState>) {
    for (mut t, part) in q.iter_mut() {
        t.translation = player.pos + part.offset;
    }
}

/// Update 怪物 cube 位置（跟随 PlayerState），让怪物永远在玩家周围画圆
pub fn follow_monster_cubes(mut q: Query<(&mut Transform, &MonsterCube)>) {
    for (mut t, mc) in q.iter_mut() {
        t.translation.x = mc.base.x;
        t.translation.z = mc.base.z;
    }
}

/// 怪物 Idle 动画：上下浮动 + 慢速旋转，看起来像活的
pub fn animate_monsters(time: Res<Time>, mut q: Query<(&mut Transform, &MonsterCube)>) {
    let t = time.elapsed_secs();
    for (i, (mut transform, monster)) in q.iter_mut().enumerate() {
        let phase = (i as f32) * 0.7;
        // 上下浮动（每只怪不同 phase）
        let bob = (t * 1.5 + phase).sin() * 0.12;
        transform.translation = monster.base + Vec3::Y * bob;
        // 慢速 yaw 旋转
        transform.rotate_y(0.4 * time.delta_secs());
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
        // 偏移必须跟 spawn_pretty 里的 "pos + offset" 完全一致, 否则位置错乱
        // spawn_pretty 新尺寸 (1.4x): body y=1.25, head y=3.0, hair y=3.7, eyes y=3.05, legs y=0.42
        // flag pole y=4.5 (3.8m 高), flag y=6.5 (1.4x0.9x0.06), red strip y=6.0
        let offset = match i {
            0 => Vec3::new(0.0, 1.25 + bob, 0.0),     // body (1.9m 高)
            1 => Vec3::new(0.0, 3.0 + bob, 0.0),      // head
            2 => Vec3::new(0.0, 3.7 + bob, 0.0),      // hair
            3 => Vec3::new(-0.28, 3.05 + bob, -0.58), // L eye
            4 => Vec3::new(0.28, 3.05 + bob, -0.58),  // R eye
            5 => Vec3::new(-0.28, 0.42, 0.0),         // L leg
            6 => Vec3::new(0.28, 0.42, 0.0),          // R leg
            7 => Vec3::new(0.0, 4.5 + bob, 0.0),      // flag pole (3.8m 高, 中心 y=4.5)
            8 => Vec3::new(0.7, 6.5 + bob, 0.0),      // flag (orange, 加大后中心 y=6.5)
            9 => Vec3::new(0.7, 6.0 + bob, 0.0),      // flag red strip (旗面下方)
            _ => Vec3::ZERO,
        };
        transform.translation = base + offset;
    }
}
