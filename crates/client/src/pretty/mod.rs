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

use crate::render::scalar_field::effective_ground_height;

#[cfg(feature = "audit-pretty-models")]
mod audit_pretty;

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

/// 每帧把外圈+内圈圆盘贴到玩家脚下**地表**。
/// 玩家在 setup_world 之后移动时，ground 不会"留在原地"漂浮。
/// y 用 effective_ground_height(player.x, player.z) 而不是写死 player.y - 0.5,
/// 这样无论玩家站在山顶还是水边, 圆盘都贴 solid 块顶。
/// 用单个 query (Without<AvatarPart, MonsterCube>) 一次拿所有 ground disc entity,
/// 循环内按 marker 区分 y 偏移 — 避免双 query B0001 conflict。
pub fn follow_ground_discs(
    player: Res<PlayerState>,
    game_world: Res<GameWorld>,
    mut outer: Query<&mut Transform, (With<GroundDiscOuter>, Without<GroundDiscInner>)>,
    mut inner: Query<&mut Transform, (With<GroundDiscInner>, Without<GroundDiscOuter>)>,
) {
    let ground_top = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
    for mut t in &mut outer {
        t.translation = Vec3::new(player.pos.x, ground_top - 0.05, player.pos.z);
    }
    for mut t in &mut inner {
        t.translation = Vec3::new(player.pos.x, ground_top + 0.25, player.pos.z);
    }
}

/// 每帧把水面盘跟随玩家 XZ 移动, 保持 Y 不变。
///
/// 配合 spawn_pretty 里把水面 size 缩到 56m 并 spawn 在 player.pos.xz —
/// 玩家走到哪儿水面就跟到哪儿, 远处不再是一望无际的假水蓝。
pub fn follow_water(player: Res<PlayerState>, mut q: Query<&mut Transform, With<WaterMarker>>) {
    let Ok(mut tf) = q.single_mut() else {
        return;
    };
    tf.translation.x = player.pos.x;
    tf.translation.z = player.pos.z;
    // Y 不动 (spawn 时已是 WATER_Y = SEA_LEVEL = 12)
}

/// 玩家 avatar 各部件 marker + 相对玩家的偏移
#[derive(Component)]
pub struct AvatarPart {
    pub offset: Vec3,
}

const AVATAR_VISUAL_SCALE: f32 = 0.55;

fn avatar_offset(offset: Vec3) -> Vec3 {
    offset * AVATAR_VISUAL_SCALE
}

/// 怪物 marker + 相对玩家 offset（spawn 时记录，follow 时跟随 player 移动）
#[derive(Component)]
pub struct MonsterCube {
    pub base: Vec3,
}

/// 云朵 puff marker：4 朵云各 4 个 sphere 拼, follow 系统让云跟玩家平移 + 上下浮动
#[derive(Component)]
pub struct CloudPuff {
    /// puff 的"世界锚点" (相对世界, 不随玩家移动)
    pub base: Vec3,
    /// 浮动相位 (rad), 让不同 puff 异相 bob
    pub phase: f32,
}

#[derive(Component)]
pub struct V2WorldMarker;

/// 启动时 spawn 水面 + 玩家 avatar
pub fn spawn_pretty(
    mut commands: Commands,
    game_world: Res<GameWorld>,
    player: Res<PlayerState>,
    cfg: Res<PrettyConfig>,
    _asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ---- 水面 ----
    // 诊断发现: superflat preset 实际没填水 (WaterFillModule weight=0), 但水面盘仍 spawn 在 Y=12
    // → 跟 smooth mesh surface (Y=12-13) 同一高度, Z-fight + alpha 0.72 把 smooth mesh 整片盖住
    // 解决: 水面只跟随玩家, 但尺寸缩到 36m (够看 + 不会盖整个 smooth mesh 36m 半径)
    // 干脆先彻底关掉 superflat 的水: superflat 是测试地图, 没水也合理
    if cfg.show_water {
        // 关键: 之前 s = world.size * 1.5 = 144m 的水面盘在世界中心, superflat preset
        // 实际上没有填水 (WaterFillModule weight=0), 但视觉上仍有这块假水 —
        // 一片纯蓝铺满 36m smooth mesh 之外的全部视野, 把"纯色"地形掩盖掉了。
        //
        // 现在: 水面缩小到 56m (够看 + 不喧宾夺主), 跟随玩家移动 — 玩家走到哪儿
        // 水面就在哪儿, 远处不再是一望无际的假水。
        let s = 56.0_f32;
        let water_y = lk2_core::constant::WATER_Y;
        let cx = player.pos.x;
        let cz = player.pos.z;
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(s, s))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.22, 0.42, 0.62, 0.72),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.85,
                metallic: 0.0,
                reflectance: 0.18,
                ..default()
            })),
            Transform::from_translation(Vec3::new(cx, water_y, cz)),
            WaterMarker,
        ));
        info!(
            "🌊 水面已 spawn (y={}, size={}, 跟随玩家 @ ({:.1}, {:.1}))",
            water_y, s, cx, cz
        );
    }

    // ---- 玩家脚下"基地盘"（v4c 2026-06-21: 加大到比 avatar 略大, 让玩家能看见自己位置） ----
    // 玩家 avatar ~0.6m 宽, 外圈 1.6m 直径 (半径 0.8), 内圈 0.6m 直径 (半径 0.3)
    // 圆盘作用: 脚下"光圈" 提示玩家位置 (v4b 0.30m 太小被 avatar 0.6m 遮住 → 玩家找不到自己)
    {
        // 外圈大圆盘 (直径 1.6, 0.05 厚)
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.8, 0.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.32, 0.48, 0.20, 0.65),
                emissive: Color::srgb(0.20, 0.40, 0.10).into(),
                perceptual_roughness: 0.95,
                metallic: 0.0,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_translation(Vec3::new(player.pos.x, player.pos.y - 0.05, player.pos.z)),
            GroundDiscOuter,
        ));

        // 内圈"光点" (直径 0.6)
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.3, 0.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.55, 0.75, 0.30, 0.85),
                emissive: Color::srgb(0.30, 0.50, 0.15).into(),
                perceptual_roughness: 0.92,
                metallic: 0.0,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_translation(Vec3::new(
                player.pos.x,
                player.pos.y + 0.005,
                player.pos.z,
            )),
            GroundDiscInner,
        ));
    }

    // ---- 玩家 avatar (v5-cute: 球+大眼睛+红脸蛋+微笑, 0.8m 高) ----
    // Q 版比例, smooth 球体, 糖果色 + 强自发光, 用 bevy Sphere mesh
    if cfg.show_player_avatar {
        let base = player.pos;
        // 头 (大圆球, Q 版头占身 1/2)
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.30))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.75), // 肤色
                emissive: LinearRgba::from(Color::srgb(0.40, 0.34, 0.30)) * 0.5, // 暖发光
                perceptual_roughness: 0.5,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.0, 0.70, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.93, 0.93)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.0, 0.70, 0.0)) },
        ));
        // 头发 (棕色一片)
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.30))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.30, 0.20),
                emissive: LinearRgba::from(Color::srgb(0.18, 0.12, 0.08)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.0, 0.80, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.60, 1.0)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.0, 0.80, 0.0)) },
        ));
        // 大眼睛 L
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.09))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.02, 0.04),
                emissive: Color::BLACK.into(),
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.10, 0.72, 0.22)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.78, 1.0, 0.55)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.10, 0.72, 0.22)) },
        ));
        // 大眼睛 R
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.09))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.02, 0.04),
                emissive: Color::BLACK.into(),
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.10, 0.72, 0.22)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.78, 1.0, 0.55)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.10, 0.72, 0.22)) },
        ));
        // 眼睛高光 L (小白点, 强发光)
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.03))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE.to_linear() * 1.5,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.085, 0.76, 0.27)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.6, 0.7, 0.3)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.085, 0.76, 0.27)) },
        ));
        // 眼睛高光 R
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.03))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE.to_linear() * 1.5,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.115, 0.76, 0.27)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.6, 0.7, 0.3)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.115, 0.76, 0.27)) },
        ));
        // 红脸蛋 L (腮红)
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.07))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.65, 0.70),
                emissive: LinearRgba::from(Color::srgb(0.50, 0.30, 0.30)) * 0.5,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.22, 0.66, 0.18)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.7, 0.5)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.22, 0.66, 0.18)) },
        ));
        // 红脸蛋 R
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.07))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.65, 0.70),
                emissive: LinearRgba::from(Color::srgb(0.50, 0.30, 0.30)) * 0.5,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.22, 0.66, 0.18)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.7, 0.5)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.22, 0.66, 0.18)) },
        ));
        // 微笑嘴 (1 个小弧, 用小紫红球)
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.04))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.30, 0.40),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.10, 0.15)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.0, 0.62, 0.27)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.4, 0.3, 0.3)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.0, 0.62, 0.27)) },
        ));
        // 球身 (红衣)
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.30))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.40, 0.40),
                emissive: LinearRgba::from(Color::srgb(0.50, 0.20, 0.20)) * 0.5,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.0, 0.40, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.93, 0.83)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.0, 0.40, 0.0)) },
        ));
        // 球腿 L (蓝裤)
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.10, 0.10, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.10, 0.10, 0.0)) },
        ));
        // 球腿 R
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.10, 0.10, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.10, 0.10, 0.0)) },
        ));
        // 球脚 L (大圆球当脚)
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.13))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.10, 0.05, 0.05)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.6, 1.2)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.10, 0.05, 0.05)) },
        ));
        // 球脚 R
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.13))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.10, 0.05, 0.05)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.6, 1.2)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.10, 0.05, 0.05)) },
        ));
        // 球手 L (肤色)
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.75),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.34, 0.30)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.30, 0.42, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.30, 0.42, 0.0)) },
        ));
        // 球手 R
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.75),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.34, 0.30)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.30, 0.42, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.30, 0.42, 0.0)) },
        ));
        info!(
            "🧍 玩家 avatar (v5-cute Q 版球体) 已 spawn at {:?}",
            player.pos
        );
    }

    // ---- 怪物（球体 + 颜色，5 种，5-15m 圆周，落地） ----
    // 改 cube→sphere 让它看起来像生物. 半径 0.72 → 1.3 让怪物体量跟旁边的树/石头匹配。
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
            let r = 7.5 + (i as f32) * 1.8; // 7.5-14.7m 圆周（之前 6-12m 太挤，跟玩家 avatar 叠）
            let offset = Vec3::new(angle.cos() * r, 1.3, angle.sin() * r); // y 也从 0.5→1.3, 球心对齐新半径
            let pos = player.pos + offset;
            // 球体 + 自发光, 半径 1.3m（之前 0.72m, 几乎是个小球）
            let entity = commands
                .spawn((
                    Mesh3d(meshes.add(Sphere::new(1.3))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: *color,
                        emissive: LinearRgba::from(*color) * 0.4, // 自发光 0.4x 颜色, 远处也能看见
                        perceptual_roughness: 0.6,
                        metallic: 0.0,
                        ..default()
                    })),
                    Transform::from_translation(pos),
                    MonsterCube { base: pos },
                ))
                .id();
            let _ = entity;
        }
        info!("👹 5 个怪物球体已 spawn (7.5-14.7m 圆周, 半径 1.3m, 朝 player 走)");
    }

    // ---- 云朵（软球拼接，4 朵绕玩家 30m 上空） ----
    // 之前 6 块方块云"硬且扁", 现在每朵 = 3-4 个 sphere 拼, 看起来像水汽
    // 配 soft 白色 (0.92, 0.95, 1.0) + 高 roughness 模拟水汽散射
    // 4 朵环绕, 每朵在玩家头顶 18-22m, 半径 14-22m
    let cloud_layouts: [(f32, f32, f32, f32, f32); 4] = [
        // (angle_rad, radius, base_y, x_extent, z_extent)
        (0.7, 22.0, 18.0, 1.0, 0.7),
        (2.1, 18.0, 20.0, 0.8, 1.0),
        (3.8, 25.0, 22.0, 1.2, 0.8),
        (5.4, 16.0, 19.0, 0.7, 0.7),
    ];
    for (angle, r, base_y, ex, ez) in cloud_layouts.iter().copied() {
        let cx = player.pos.x + angle.cos() * r;
        let cz = player.pos.z + angle.sin() * r;
        let cy = base_y;
        // 朵中心
        let cloud_color = Color::srgba(0.92, 0.95, 1.0, 0.85);
        let cloud_mat = materials.add(StandardMaterial {
            base_color: cloud_color,
            emissive: (cloud_color.to_linear() * 0.18).into(),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        // 中心大球
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.6))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(Vec3::new(cx, cy, cz)),
            CloudPuff { base: Vec3::new(cx, cy, cz), phase: angle * 1.3 },
        ));
        // 左小球
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.1))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(Vec3::new(cx - 1.5 * ex, cy + 0.2, cz)),
            CloudPuff { base: Vec3::new(cx - 1.5 * ex, cy + 0.2, cz), phase: angle * 1.3 + 1.7 },
        ));
        // 右小球
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.2))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(Vec3::new(cx + 1.6 * ex, cy - 0.1, cz + 0.5 * ez)),
            CloudPuff {
                base: Vec3::new(cx + 1.6 * ex, cy - 0.1, cz + 0.5 * ez),
                phase: angle * 1.3 + 3.1,
            },
        ));
        // 顶小帽
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.9))),
            MeshMaterial3d(cloud_mat),
            Transform::from_translation(Vec3::new(cx + 0.3, cy + 1.0, cz - 0.2 * ez)),
            CloudPuff {
                base: Vec3::new(cx + 0.3, cy + 1.0, cz - 0.2 * ez),
                phase: angle * 1.3 + 4.5,
            },
        ));
    }

    // ---- 树（深棕树干 + 绿色树冠）— 8 棵绕玩家圆周分布 ----
    // ground_y 用 effective_ground_height(player.x, player.z) 而不是 player.pos.y - 2.0,
    // 玩家站在山顶或水边时, 装饰物都能贴地表而不是浮在空中 (参见 iter_1284 截图 bug).
    let ground_y = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
    // 8 棵树, 半径 13m (从 20m 拉近, 22m 相机下能看清树冠) + 树干/树冠再 +30%
    // (之前 1.85+1.70 ≈ 7.1m 总高, 20m 远在 22m 相机下只占几像素)
    for i in 0..8 {
        let angle = (i as f32) * (std::f32::consts::TAU / 8.0);
        let r = 13.0;
        let t_x = player.pos.x + angle.cos() * r;
        let t_z = player.pos.z + angle.sin() * r;
        // 树干：3 格高, 截面 +30%
        for h in 0..3 {
            spawn_cube(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(t_x, ground_y + 1.10 + h as f32, t_z),
                Vec3::new(1.10, 2.10, 1.10),
                Color::srgb(0.45, 0.27, 0.10),
            );
        }
        // 树冠：3 层, 1.85→2.40 放大
        for dy in 0..3 {
            spawn_cube(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(t_x, ground_y + 3.55 + dy as f32, t_z),
                Vec3::new(2.40, 2.20, 2.40),
                Color::srgb(0.25, 0.55, 0.20),
            );
        }
    }

    // ---- 石头（中灰块散布在 spawn 周围 3-5m 外圈，10 块） ----
    // 给画面增加"野外"质感。scale 0.4-0.85 太瘦小, 全部 *1.7 让石头看着有体积。
    // 半径 *1.4 → 1.8 让石头更靠近玩家 (之前 4.5-7m 在 22m 相机下太小)
    let rock_positions: [(f32, f32, f32); 10] = [
        (3.0, 3.0, 1.2),
        (-3.5, 3.2, 0.95),
        (2.5, -3.8, 1.1),
        (-2.2, -4.5, 0.75),
        (4.0, -1.0, 1.45),
        (-4.2, -0.7, 0.85),
        (4.2, 3.8, 1.05),
        (-3.8, -2.4, 1.3),
        (1.0, 4.5, 0.7),
        (-1.0, -4.8, 0.85),
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
            Vec3::new(*scale * 1.55, *scale * 1.20, *scale * 1.55),
            rock_color,
        );
    }

    // ---- 花朵（中彩色斑点缀在 spawn 周围 1.5-4m，10 朵） ----
    // 0.55→0.95, 0.75→1.30, 体量约 +75%, 像真花不像贴片。
    // 半径 *1.4 → *1.0 让花更靠近玩家 (之前 2.5-5.2m 在 22m 相机下太小)
    let flower_positions: [(f32, f32); 10] = [
        (1.8, 1.8),
        (-2.0, 2.3),
        (2.3, -1.0),
        (-2.5, -1.8),
        (3.0, -0.5),
        (-3.4, 1.0),
        (0.8, -3.2),
        (3.6, 2.5),
        (-3.0, -3.6),
        (2.7, 3.7),
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
            Vec3::new(f_x, ground_y + 0.6, f_z),
            Vec3::new(0.95, 1.30, 0.95),
            f_color,
        );
    }

    // ---- 远景山丘（大绿块在 28m 外, 给画面深度感） ----
    // 之前 5×3×5 太小, 提到 8×4.5×8, 像远处的山头而不是桌子。
    // v2 (2026-06-20): 中心 y 从 ground_y + 3.2 改 ground_y + 2.25 (半高),
    // 让 Cuboid 底面贴在 ground 上. 之前 center=ground_y+3.2, 半高 2.25 →
    // bottom = ground_y + 0.95, 漂在地面 0.95m 上方. 修后 bottom = ground_y 贴地.
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
            Mesh3d(meshes.add(Cuboid::new(8.0, 4.5, 8.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.20, 0.42, 0.18),
                emissive: Color::srgb(0.04, 0.08, 0.03).into(),
                perceptual_roughness: 0.95,
                metallic: 0.0,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_translation(Vec3::new(h_x, ground_y + 2.25, h_z)),
        ));
    }
    spawn_v2_crown_season_markers(
        &mut commands,
        &mut meshes,
        &mut materials,
        player.pos,
        ground_y,
    );

    // ---- [dev-only] pretty/ 资产可视化审计 ----
    // 仅在 --features audit-pretty-models 时编译. 把所有 23 个 .glb 摆成
    // 一圈, 用来确认 Blender 资产在引擎里能渲染 + 看大致外观, 不用替换原
    // 任何 spawn 代码.
    #[cfg(feature = "audit-pretty-models")]
    audit_pretty::spawn_audit_ring(
        &mut commands,
        &mut meshes,
        &mut materials,
        _asset_server,
        player.pos,
        ground_y,
    );
}

fn spawn_v2_crown_season_markers(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    player_pos: Vec3,
    ground_y: f32,
) {
    spawn_v2_cube(
        commands,
        meshes,
        materials,
        Vec3::new(player_pos.x, ground_y + 0.28, player_pos.z),
        Vec3::new(7.5, 0.26, 7.5),
        Color::srgb(0.28, 0.34, 0.24),
        Color::srgb(0.16, 0.26, 0.12),
    );

    // POI pillars moved out of the player's immediate view (radius ~16m) so they no
    // longer block the forward camera. Spread evenly on a ring; keep y in 1.2-1.9 range.
    // POI pillars sit on a ring at radius ~16m around the player, well outside the
    // player's immediate surroundings, so they stop blocking the camera's forward view.
    let pois = [
        (
            Vec3::new(12.5, 0.85, -12.5),
            Vec3::new(0.65, 1.70, 0.65),
            Color::srgb(0.70, 0.30, 0.18),
            Color::srgb(0.95, 0.50, 0.24),
        ),
        (
            Vec3::new(-12.5, 0.75, -12.5),
            Vec3::new(0.70, 1.50, 0.70),
            Color::srgb(0.15, 0.46, 0.42),
            Color::srgb(0.28, 0.82, 0.72),
        ),
        (
            Vec3::new(12.5, 0.70, 12.5),
            Vec3::new(0.80, 1.40, 0.80),
            Color::srgb(0.55, 0.16, 0.28),
            Color::srgb(0.90, 0.28, 0.44),
        ),
        (
            Vec3::new(-12.5, 0.65, 12.5),
            Vec3::new(0.70, 1.30, 0.70),
            Color::srgb(0.62, 0.48, 0.20),
            Color::srgb(0.95, 0.76, 0.26),
        ),
    ];

    for (offset, size, base, glow) in pois {
        spawn_v2_cube(
            commands,
            meshes,
            materials,
            Vec3::new(
                player_pos.x + offset.x,
                ground_y + offset.y,
                player_pos.z + offset.z,
            ),
            size,
            base,
            glow,
        );
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.62))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: glow,
                emissive: (glow.to_linear() * 1.15).into(),
                perceptual_roughness: 0.38,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_translation(Vec3::new(
                player_pos.x + offset.x,
                ground_y + offset.y + size.y * 0.58,
                player_pos.z + offset.z,
            )),
            V2WorldMarker,
        ));
    }
}

fn spawn_v2_cube(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    size: Vec3,
    base: Color,
    glow: Color,
) -> Entity {
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: base,
                emissive: (glow.to_linear() * 0.65).into(),
                perceptual_roughness: 0.62,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_translation(pos),
            V2WorldMarker,
        ))
        .id()
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
/// 改用 effective_ground_height 贴地表, 避免 avatar 跟圆盘一起飘水
pub fn follow_player_avatar(
    mut q: Query<(&mut Transform, &AvatarPart)>,
    player: Res<PlayerState>,
    game_world: Res<GameWorld>,
) {
    let ground_top = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
    // offset.y 是相对 player.pos 的偏移, 但 player.pos.y 跟地表不贴, 所以用 ground_top 替换基准 y
    for (mut t, part) in q.iter_mut() {
        t.translation = Vec3::new(player.pos.x, ground_top + part.offset.y, player.pos.z)
            + Vec3::new(part.offset.x, 0.0, part.offset.z);
    }
}

/// Update 怪物位置 — 朝玩家慢慢走 (P5 intent/commit AI 雏形)
/// 距离 < 12m 时朝 player 方向走 0.6 m/s, 距离 > 12m 时回 base
/// y 用 effective_ground_height 贴地
/// 注意: 这里 *mut* MonsterCube 才能持续更新 base 字段 (不然每帧都从老 base 算, 怪物会瞬移回原位)
pub fn follow_monster_cubes(
    mut q: Query<(&mut Transform, &mut MonsterCube)>,
    game_world: Res<GameWorld>,
    player: Res<PlayerState>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (mut t, mut mc) in q.iter_mut() {
        // 朝玩家方向 (xz)
        let to_player = player.pos - mc.base;
        let dist = (to_player.x * to_player.x + to_player.z * to_player.z).sqrt();
        let dir = if dist > 0.1 {
            Vec3::new(to_player.x / dist, 0.0, to_player.z / dist)
        } else {
            Vec3::ZERO
        };
        // 12m 内朝 player 走, 12m 外不动 (保持原 base 圆周)
        let speed = if dist < 12.0 { 0.4 } else { 0.0 }; // 0.6→0.4 慢一点
        let new_base = mc.base + dir * speed * dt;
        // 不要走进玩家 2.5m 内 (避免穿模 + 不挤压玩家)
        let new_dist =
            ((new_base.x - player.pos.x).powi(2) + (new_base.z - player.pos.z).powi(2)).sqrt();
        mc.base = if new_dist < 2.5 { mc.base } else { new_base };
        let ground_top = effective_ground_height(&game_world, mc.base.x as i32, mc.base.z as i32);
        t.translation = Vec3::new(mc.base.x, ground_top + 0.5, mc.base.z);
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

/// 云朵 bob 动画：每 puff 上下浮动 0.15m, 4-5s 周期
pub fn animate_cloud_puffs(time: Res<Time>, mut q: Query<(&mut Transform, &CloudPuff)>) {
    let t = time.elapsed_secs();
    for (mut tf, puff) in q.iter_mut() {
        let bob = (t * 0.6 + puff.phase).sin() * 0.15;
        tf.translation = puff.base + Vec3::Y * bob;
    }
}

/// 玩家上下浮动 + 旋转动画（v3 回退: 10 件套 cube）
///
/// 10 件套 avatar 顺序: head(0) helmet(1) torso(2) shoulderL(3) shoulderR(4)
///                     armL(5) armR(6) belt(7) legL(8) legR(9)
/// 偏移必须跟 spawn_pretty 里的 "pos + offset" 完全一致, 否则位置错乱
pub fn animate_avatar(
    time: Res<Time>,
    mut q: Query<&mut Transform, With<AvatarPart>>,
    player: Res<PlayerState>,
) {
    let t = time.elapsed_secs();
    let bob = (t * 2.0).sin() * 0.05;
    let upper_bob = bob;
    let base = player.pos;
    for (i, mut transform) in q.iter_mut().enumerate() {
        let offset = match i {
            0 => avatar_offset(Vec3::new(0.0, 0.70 + upper_bob, 0.0)),
            1 => avatar_offset(Vec3::new(0.0, 0.80 + upper_bob, 0.0)),
            2 => avatar_offset(Vec3::new(-0.10, 0.72 + upper_bob, 0.22)),
            3 => avatar_offset(Vec3::new(0.10, 0.72 + upper_bob, 0.22)),
            4 => avatar_offset(Vec3::new(-0.085, 0.76 + upper_bob, 0.27)),
            5 => avatar_offset(Vec3::new(0.115, 0.76 + upper_bob, 0.27)),
            6 => avatar_offset(Vec3::new(-0.22, 0.66 + upper_bob, 0.18)),
            7 => avatar_offset(Vec3::new(0.22, 0.66 + upper_bob, 0.18)),
            8 => avatar_offset(Vec3::new(0.0, 0.62 + upper_bob, 0.27)),
            9 => avatar_offset(Vec3::new(0.0, 0.40 + upper_bob, 0.0)),
            10 => avatar_offset(Vec3::new(-0.10, 0.10, 0.0)),
            11 => avatar_offset(Vec3::new(0.10, 0.10, 0.0)),
            12 => avatar_offset(Vec3::new(-0.10, 0.05, 0.05)),
            13 => avatar_offset(Vec3::new(0.10, 0.05, 0.05)),
            14 => avatar_offset(Vec3::new(-0.30, 0.42 + upper_bob, 0.0)),
            15 => avatar_offset(Vec3::new(0.30, 0.42 + upper_bob, 0.0)),
            _ => Vec3::ZERO,
        };
        transform.translation = base + offset;
    }
}
