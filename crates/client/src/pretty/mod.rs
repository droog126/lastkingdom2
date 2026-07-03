use bevy::prelude::*;
use lk2_core::controller::components::PvPController;
use lk2_core::eco_cycle::EcoCycle;
use lk2_core::player::PlayerState;
use lk2_core::world::{Biome, World as GameWorld};

use crate::render::scalar_field::effective_ground_height;
use crate::render::CameraAngles;

#[cfg(feature = "audit-pretty-models")]
mod audit_pretty;

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

#[derive(Component)]
pub struct WaterMarker;

#[derive(Component)]
pub struct GroundDiscOuter;

#[derive(Component)]
pub struct GroundDiscInner;

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

pub fn follow_water(player: Res<PlayerState>, mut q: Query<&mut Transform, With<WaterMarker>>) {
    let Ok(mut tf) = q.single_mut() else {
        return;
    };
    tf.translation.x = player.pos.x;
    tf.translation.z = player.pos.z;
}

#[derive(Component)]
pub struct AvatarPart {
    pub offset: Vec3,
    pub rest_scale: Vec3,
    pub sokpop: SokpopAnim,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AvatarPartKind {
    Head,
    Hair,
    HeadDetail,
    Torso,
    Thigh,
    Shin,
    Hand,
    Foot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SquashAxis {
    None,
    Y,
}

#[derive(Clone, Copy, Debug)]
pub struct SokpopAnim {
    pub kind: AvatarPartKind,
    pub phase_offset: f32,
    pub bob_amp: Vec3,
    pub lean_factor: f32,
    pub squash_axis: SquashAxis,
}

fn avatar_sokpop(kind: AvatarPartKind) -> SokpopAnim {
    match kind {
        AvatarPartKind::Head => SokpopAnim {
            kind,
            phase_offset: 0.0,
            bob_amp: Vec3::new(0.02, 0.05, 0.0),
            lean_factor: 0.85,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::Hair => SokpopAnim {
            kind,
            phase_offset: 0.25,
            bob_amp: Vec3::new(0.02, 0.06, 0.0),
            lean_factor: 0.9,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::HeadDetail => SokpopAnim {
            kind,
            phase_offset: 0.15,
            bob_amp: Vec3::new(0.02, 0.045, 0.0),
            lean_factor: 0.95,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::Torso => SokpopAnim {
            kind,
            phase_offset: 0.0,
            bob_amp: Vec3::new(0.04, 0.06, 0.0),
            lean_factor: 1.0,
            squash_axis: SquashAxis::Y,
        },
        AvatarPartKind::Thigh => SokpopAnim {
            kind,
            phase_offset: 0.0,
            bob_amp: Vec3::new(0.16, 0.09, 0.0),
            lean_factor: 0.25,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::Shin => SokpopAnim {
            kind,
            phase_offset: std::f32::consts::PI,
            bob_amp: Vec3::new(0.18, 0.05, 0.0),
            lean_factor: 0.15,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::Hand => SokpopAnim {
            kind,
            phase_offset: std::f32::consts::PI,
            bob_amp: Vec3::new(0.22, 0.06, 0.0),
            lean_factor: 0.7,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::Foot => SokpopAnim {
            kind,
            phase_offset: std::f32::consts::PI,
            bob_amp: Vec3::new(0.10, 0.04, 0.0),
            lean_factor: 0.05,
            squash_axis: SquashAxis::None,
        },
    }
}

const AVATAR_VISUAL_SCALE: f32 = 0.55;

fn avatar_offset(offset: Vec3) -> Vec3 {
    offset * AVATAR_VISUAL_SCALE
}

impl AvatarPart {
    pub fn new(offset: Vec3, rest_scale: Vec3, kind: AvatarPartKind) -> Self {
        Self { offset, rest_scale, sokpop: avatar_sokpop(kind) }
    }
}

fn spawn_avatar_part(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    pos: Vec3,
    offset: Vec3,
    rest_scale: Vec3,
    kind: AvatarPartKind,
) {
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(pos).with_scale(rest_scale),
        AvatarPart::new(offset, rest_scale, kind),
    ));
}

#[derive(Component)]
pub struct MonsterCube {
    pub base: Vec3,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EcoVisual {
    Rabbit { id: u32, part: RabbitVisualPart },
    BerryBush { id: u32 },
    BerryFruit { id: u32, index: u32 },
    Co2Bubble { index: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RabbitVisualPart {
    Body,
    Head,
    EarLeft,
    EarRight,
    Tail,
}

pub fn eco_fruit_marker_count(fruit: u32) -> u32 {
    fruit.min(3)
}

const ECO_CO2_BUBBLE_COUNT: u32 = 6;

#[derive(Resource, Default)]
pub struct PlayerAnimState {
    pub step_phase: f32,
    pub smoothed_speed: f32,
    pub smoothed_move_world: Vec2,
    pub last_pos_y: f32,
    pub vertical_vel: f32,
    pub initialized: bool,
}

pub fn update_player_anim_state(
    time: Res<Time>,
    player: Res<PlayerState>,
    ctrl: Query<&PvPController>,
    camera_angles: Res<CameraAngles>,
    mut state: ResMut<PlayerAnimState>,
) {
    let dt = time.delta_secs().max(0.0001);
    let Ok(ctrl) = ctrl.single() else {
        return;
    };

    if !state.initialized {
        state.last_pos_y = player.pos.y;
        state.initialized = true;
    }

    let smooth_k = 1.0 - (-dt * 8.0).exp();

    let raw_speed = ctrl.move_input.length().min(1.0);
    state.smoothed_speed = state.smoothed_speed + (raw_speed - state.smoothed_speed) * smooth_k;

    let yaw = camera_angles.yaw;
    let (sy, cy) = yaw.sin_cos();
    let (mx, mz) = (ctrl.move_input.x, ctrl.move_input.y);
    let world_x = cy * mz + sy * mx;
    let world_z = -sy * mz + cy * mx;
    let target_move = Vec2::new(world_x, world_z);
    state.smoothed_move_world = state.smoothed_move_world.lerp(target_move, smooth_k);

    let step_freq = 1.6 + 2.4 * state.smoothed_speed;
    state.step_phase += step_freq * dt * std::f32::consts::TAU;

    let vy = (player.pos.y - state.last_pos_y) / dt;
    state.vertical_vel = state.vertical_vel + (vy - state.vertical_vel) * smooth_k;
    state.last_pos_y = player.pos.y;
}

#[derive(Component)]
pub struct CloudPuff {
    pub base: Vec3,

    pub phase: f32,
}

#[derive(Component)]
pub struct V2WorldMarker;

#[derive(Component)]
pub struct KenneyLandmark;

#[allow(unreachable_code)]
pub fn spawn_pretty(
    mut commands: Commands,
    game_world: Res<GameWorld>,
    player: Res<PlayerState>,
    cfg: Res<PrettyConfig>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    #[cfg(feature = "audit-pretty-models")]
    #[allow(unused_variables, unreachable_code)]
    {
        let _ = &cfg;
        let ground_top =
            effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
        audit_pretty::spawn_audit_ring(
            &mut commands,
            &mut meshes,
            &mut materials,
            asset_server,
            player.pos,
            ground_top,
        );
        return;
    }

    let kenney_enabled = std::env::var("LK2_DISABLE_KENNEY")
        .map(|value| !matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(true);

    if cfg.show_water {
        let s = 14.0_f32;
        let water_y = lk2_core::constant::WATER_Y - 1.5;
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

    {
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

    if cfg.show_player_avatar {
        let base = player.pos;

        // i=0 head
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.30)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.75),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.34, 0.30)) * 0.5,
                perceptual_roughness: 0.5,
                metallic: 0.0,
                ..default()
            }),
            base + avatar_offset(Vec3::new(0.0, 0.70, 0.0)),
            avatar_offset(Vec3::new(0.0, 0.70, 0.0)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.93, 0.93),
            AvatarPartKind::Head,
        );

        // i=1 hair
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.30)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.30, 0.20),
                emissive: LinearRgba::from(Color::srgb(0.18, 0.12, 0.08)) * 0.4,
                ..default()
            }),
            base + avatar_offset(Vec3::new(0.0, 0.80, 0.0)),
            avatar_offset(Vec3::new(0.0, 0.80, 0.0)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.60, 1.0),
            AvatarPartKind::Hair,
        );

        // i=2 left eye (black)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.09)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.02, 0.04),
                emissive: Color::BLACK.into(),
                ..default()
            }),
            base + avatar_offset(Vec3::new(-0.10, 0.72, 0.22)),
            avatar_offset(Vec3::new(-0.10, 0.72, 0.22)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.78, 1.0, 0.55),
            AvatarPartKind::HeadDetail,
        );

        // i=3 right eye (black)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.09)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.02, 0.04),
                emissive: Color::BLACK.into(),
                ..default()
            }),
            base + avatar_offset(Vec3::new(0.10, 0.72, 0.22)),
            avatar_offset(Vec3::new(0.10, 0.72, 0.22)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.78, 1.0, 0.55),
            AvatarPartKind::HeadDetail,
        );

        // i=4 left pupil (white)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.03)),
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE.to_linear() * 1.5,
                ..default()
            }),
            base + avatar_offset(Vec3::new(-0.085, 0.76, 0.27)),
            avatar_offset(Vec3::new(-0.085, 0.76, 0.27)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.6, 0.7, 0.3),
            AvatarPartKind::HeadDetail,
        );

        // i=5 right pupil (white)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.03)),
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE.to_linear() * 1.5,
                ..default()
            }),
            base + avatar_offset(Vec3::new(0.115, 0.76, 0.27)),
            avatar_offset(Vec3::new(0.115, 0.76, 0.27)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.6, 0.7, 0.3),
            AvatarPartKind::HeadDetail,
        );

        // i=6 left ear (pink)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.07)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.65, 0.70),
                emissive: LinearRgba::from(Color::srgb(0.50, 0.30, 0.30)) * 0.5,
                ..default()
            }),
            base + avatar_offset(Vec3::new(-0.22, 0.66, 0.18)),
            avatar_offset(Vec3::new(-0.22, 0.66, 0.18)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.7, 0.5),
            AvatarPartKind::HeadDetail,
        );

        // i=7 right ear (pink)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.07)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.65, 0.70),
                emissive: LinearRgba::from(Color::srgb(0.50, 0.30, 0.30)) * 0.5,
                ..default()
            }),
            base + avatar_offset(Vec3::new(0.22, 0.66, 0.18)),
            avatar_offset(Vec3::new(0.22, 0.66, 0.18)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.7, 0.5),
            AvatarPartKind::HeadDetail,
        );

        // i=8 mouth
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.04)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.30, 0.40),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.10, 0.15)) * 0.4,
                ..default()
            }),
            base + avatar_offset(Vec3::new(0.0, 0.62, 0.27)),
            avatar_offset(Vec3::new(0.0, 0.62, 0.27)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.4, 0.3, 0.3),
            AvatarPartKind::HeadDetail,
        );

        // i=9 torso (red)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.30)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.40, 0.40),
                emissive: LinearRgba::from(Color::srgb(0.50, 0.20, 0.20)) * 0.5,
                ..default()
            }),
            base + avatar_offset(Vec3::new(0.0, 0.40, 0.0)),
            avatar_offset(Vec3::new(0.0, 0.40, 0.0)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.93, 0.83),
            AvatarPartKind::Torso,
        );

        // i=10 left thigh (blue)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.10)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            }),
            base + avatar_offset(Vec3::new(-0.10, 0.10, 0.0)),
            avatar_offset(Vec3::new(-0.10, 0.10, 0.0)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0),
            AvatarPartKind::Thigh,
        );

        // i=11 right thigh (blue)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.10)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            }),
            base + avatar_offset(Vec3::new(0.10, 0.10, 0.0)),
            avatar_offset(Vec3::new(0.10, 0.10, 0.0)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0),
            AvatarPartKind::Thigh,
        );

        // i=12 left shin/foot
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.13)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            }),
            base + avatar_offset(Vec3::new(-0.10, 0.05, 0.05)),
            avatar_offset(Vec3::new(-0.10, 0.05, 0.05)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.6, 1.2),
            AvatarPartKind::Shin,
        );

        // i=13 right shin/foot
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.13)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            }),
            base + avatar_offset(Vec3::new(0.10, 0.05, 0.05)),
            avatar_offset(Vec3::new(0.10, 0.05, 0.05)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.6, 1.2),
            AvatarPartKind::Shin,
        );

        // i=14 left hand (skin)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.10)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.75),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.34, 0.30)) * 0.4,
                ..default()
            }),
            base + avatar_offset(Vec3::new(-0.30, 0.42, 0.0)),
            avatar_offset(Vec3::new(-0.30, 0.42, 0.0)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0),
            AvatarPartKind::Hand,
        );

        // i=15 right hand (skin)
        spawn_avatar_part(
            &mut commands,
            meshes.add(Sphere::new(0.10)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.75),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.34, 0.30)) * 0.4,
                ..default()
            }),
            base + avatar_offset(Vec3::new(0.30, 0.42, 0.0)),
            avatar_offset(Vec3::new(0.30, 0.42, 0.0)),
            Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0),
            AvatarPartKind::Hand,
        );

        info!(
            "🧍 玩家 avatar (sokpop-style Q 版) 已 spawn at {:?}",
            player.pos
        );
    }

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
            let r = 7.5 + (i as f32) * 1.8;
            let offset = Vec3::new(angle.cos() * r, 1.3, angle.sin() * r);
            let pos = player.pos + offset;

            let entity = commands
                .spawn((
                    Mesh3d(meshes.add(Sphere::new(1.3))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: *color,
                        emissive: LinearRgba::from(*color) * 0.4,
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

    let cloud_layouts: [(f32, f32, f32, f32, f32); 4] = [
        (0.7, 22.0, 18.0, 1.0, 0.7),
        (2.1, 18.0, 20.0, 0.8, 1.0),
        (3.8, 25.0, 22.0, 1.2, 0.8),
        (5.4, 16.0, 19.0, 0.7, 0.7),
    ];
    for (angle, r, base_y, ex, ez) in cloud_layouts.iter().copied() {
        let cx = player.pos.x + angle.cos() * r;
        let cz = player.pos.z + angle.sin() * r;
        let cy = base_y;

        let cloud_color = Color::srgba(0.92, 0.95, 1.0, 0.85);
        let cloud_mat = materials.add(StandardMaterial {
            base_color: cloud_color,
            emissive: (cloud_color.to_linear() * 0.18).into(),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.6))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(Vec3::new(cx, cy, cz)),
            CloudPuff { base: Vec3::new(cx, cy, cz), phase: angle * 1.3 },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.1))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(Vec3::new(cx - 1.5 * ex, cy + 0.2, cz)),
            CloudPuff { base: Vec3::new(cx - 1.5 * ex, cy + 0.2, cz), phase: angle * 1.3 + 1.7 },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.2))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(Vec3::new(cx + 1.6 * ex, cy - 0.1, cz + 0.5 * ez)),
            CloudPuff {
                base: Vec3::new(cx + 1.6 * ex, cy - 0.1, cz + 0.5 * ez),
                phase: angle * 1.3 + 3.1,
            },
        ));

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

    let ground_y = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);

    for i in 0..8 {
        let angle = (i as f32) * (std::f32::consts::TAU / 8.0);
        let r = 13.0;
        let t_x = player.pos.x + angle.cos() * r;
        let t_z = player.pos.z + angle.sin() * r;

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
        let rock_color = match i % 3 {
            0 => Color::srgb(0.42, 0.42, 0.45),
            1 => Color::srgb(0.58, 0.55, 0.50),
            _ => Color::srgb(0.50, 0.52, 0.48),
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
        Color::srgb(0.98, 0.30, 0.55),
        Color::srgb(1.0, 0.85, 0.20),
        Color::srgb(0.55, 0.30, 0.98),
        Color::srgb(1.0, 0.45, 0.20),
        Color::srgb(0.95, 0.30, 0.30),
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
    if kenney_enabled {
        spawn_kenney_landmarks(&mut commands, &asset_server, player.pos, ground_y);
    }

    #[cfg(feature = "audit-pretty-models")]
    audit_pretty::spawn_audit_ring(
        &mut commands,
        &mut meshes,
        &mut materials,
        asset_server,
        player.pos,
        ground_y,
    );
}

fn spawn_kenney_landmarks(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    player_pos: Vec3,
    ground_y: f32,
) {
    const LANDMARKS: &[(&str, &str, Vec3, f32)] = &[
        (
            "kenney_campfire_pit",
            "kenney_survival-kit/Models/GLB format/campfire-pit.glb",
            Vec3::new(4.8, -0.45, -3.2),
            1.0,
        ),
        (
            "kenney_tent",
            "kenney_survival-kit/Models/GLB format/tent.glb",
            Vec3::new(6.4, -0.45, -4.0),
            1.0,
        ),
        (
            "kenney_workbench",
            "kenney_survival-kit/Models/GLB format/workbench.glb",
            Vec3::new(3.3, -0.45, -5.0),
            1.0,
        ),
        (
            "kenney_row_boat_small",
            "kenney_pirate-kit/Models/GLB format/boat-row-small.glb",
            Vec3::new(-6.0, -0.45, 5.0),
            1.0,
        ),
        (
            "kenney_coin_gold",
            "kenney_platformer-kit/Models/GLB format/coin-gold.glb",
            Vec3::new(-2.7, 0.15, -3.2),
            1.0,
        ),
        (
            "kenney_heart",
            "kenney_platformer-kit/Models/GLB format/heart.glb",
            Vec3::new(-3.7, 0.25, -2.2),
            1.0,
        ),
    ];

    for (name, path, offset, scale) in LANDMARKS {
        let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", path));
        let pos = Vec3::new(
            player_pos.x + offset.x,
            ground_y + offset.y,
            player_pos.z + offset.z,
        );
        commands.spawn((
            SceneRoot(scene),
            Transform::from_translation(pos).with_scale(Vec3::splat(*scale)),
            KenneyLandmark,
        ));
        info!("[kenney] spawned landmark {} at {:?}", name, pos);
    }
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

                emissive: (color.to_linear() * 0.25).into(),
                perceptual_roughness: 0.6,
                metallic: 0.1,
                ..default()
            })),
            Transform::from_translation(pos),
        ))
        .id()
}

pub fn spawn_eco_visuals(
    mut commands: Commands,
    eco: Res<EcoCycle>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let rabbit_body_mesh = meshes.add(Sphere::new(0.5));
    let rabbit_head_mesh = meshes.add(Sphere::new(0.5));
    let rabbit_ear_mesh = meshes.add(Cuboid::new(0.12, 0.48, 0.08));
    let rabbit_tail_mesh = meshes.add(Sphere::new(0.5));
    let bush_mesh = meshes.add(Sphere::new(0.5));
    let fruit_mesh = meshes.add(Sphere::new(0.5));
    let bubble_mesh = meshes.add(Sphere::new(0.5));

    let rabbit_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.92, 0.82),
        emissive: Color::srgb(0.35, 0.25, 0.20).into(),
        perceptual_roughness: 0.72,
        metallic: 0.0,
        ..default()
    });
    let rabbit_inner_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.55, 0.68),
        emissive: Color::srgb(0.35, 0.10, 0.16).into(),
        perceptual_roughness: 0.72,
        metallic: 0.0,
        ..default()
    });
    let bush_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.58, 0.20),
        emissive: Color::srgb(0.04, 0.20, 0.03).into(),
        perceptual_roughness: 0.88,
        metallic: 0.0,
        ..default()
    });
    let fruit_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.14, 0.28),
        emissive: Color::srgb(0.80, 0.05, 0.15).into(),
        perceptual_roughness: 0.42,
        metallic: 0.0,
        ..default()
    });
    let bubble_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.45, 0.88, 1.0, 0.48),
        emissive: Color::srgb(0.18, 0.55, 0.85).into(),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.18,
        metallic: 0.0,
        ..default()
    });

    for rabbit in &eco.rabbits {
        spawn_eco_visual_part(
            &mut commands,
            rabbit_body_mesh.clone(),
            rabbit_mat.clone(),
            EcoVisual::Rabbit { id: rabbit.id, part: RabbitVisualPart::Body },
            Vec3::new(rabbit.pos.x, 0.0, rabbit.pos.y),
            Vec3::new(0.70, 0.42, 0.95),
        );
        spawn_eco_visual_part(
            &mut commands,
            rabbit_head_mesh.clone(),
            rabbit_mat.clone(),
            EcoVisual::Rabbit { id: rabbit.id, part: RabbitVisualPart::Head },
            Vec3::new(rabbit.pos.x, 0.0, rabbit.pos.y),
            Vec3::new(0.42, 0.38, 0.42),
        );
        spawn_eco_visual_part(
            &mut commands,
            rabbit_ear_mesh.clone(),
            rabbit_inner_mat.clone(),
            EcoVisual::Rabbit { id: rabbit.id, part: RabbitVisualPart::EarLeft },
            Vec3::new(rabbit.pos.x, 0.0, rabbit.pos.y),
            Vec3::ONE,
        );
        spawn_eco_visual_part(
            &mut commands,
            rabbit_ear_mesh.clone(),
            rabbit_inner_mat.clone(),
            EcoVisual::Rabbit { id: rabbit.id, part: RabbitVisualPart::EarRight },
            Vec3::new(rabbit.pos.x, 0.0, rabbit.pos.y),
            Vec3::ONE,
        );
        spawn_eco_visual_part(
            &mut commands,
            rabbit_tail_mesh.clone(),
            rabbit_mat.clone(),
            EcoVisual::Rabbit { id: rabbit.id, part: RabbitVisualPart::Tail },
            Vec3::new(rabbit.pos.x, 0.0, rabbit.pos.y),
            Vec3::splat(0.26),
        );
    }

    for berry in &eco.berries {
        spawn_eco_visual_part(
            &mut commands,
            bush_mesh.clone(),
            bush_mat.clone(),
            EcoVisual::BerryBush { id: berry.id },
            Vec3::new(berry.pos.x, 0.0, berry.pos.y),
            Vec3::new(0.95, 0.58, 0.95),
        );
        for index in 0..3 {
            spawn_eco_visual_part(
                &mut commands,
                fruit_mesh.clone(),
                fruit_mat.clone(),
                EcoVisual::BerryFruit { id: berry.id, index },
                Vec3::new(berry.pos.x, 0.0, berry.pos.y),
                Vec3::splat(0.18),
            );
        }
    }

    for index in 0..ECO_CO2_BUBBLE_COUNT {
        spawn_eco_visual_part(
            &mut commands,
            bubble_mesh.clone(),
            bubble_mat.clone(),
            EcoVisual::Co2Bubble { index },
            Vec3::ZERO,
            Vec3::splat(0.28),
        );
    }

    info!(
        "Eco visuals spawned: {} rabbits, {} berry bushes, {} CO2 bubbles",
        eco.rabbits.len(),
        eco.berries.len(),
        ECO_CO2_BUBBLE_COUNT
    );
}

fn spawn_eco_visual_part(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    visual: EcoVisual,
    pos: Vec3,
    scale: Vec3,
) {
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(pos).with_scale(scale),
        visual,
    ));
}

pub fn update_eco_visuals(
    time: Res<Time>,
    eco: Res<EcoCycle>,
    game_world: Res<GameWorld>,
    mut q: Query<(&EcoVisual, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (visual, mut transform) in &mut q {
        match *visual {
            EcoVisual::Rabbit { id, part } => {
                let Some(rabbit) = eco.rabbits.iter().find(|rabbit| rabbit.id == id) else {
                    transform.scale = Vec3::ZERO;
                    continue;
                };
                let x = rabbit.pos.x;
                let z = rabbit.pos.y;
                let ground_y = effective_ground_height(&game_world, x as i32, z as i32);
                let phase = t * 5.0 + id as f32 * 0.9;
                let hop = phase.sin().max(0.0) * 0.14;
                let breath = (t * 2.2 + id as f32).sin() * 0.025;
                let (offset, scale) = rabbit_part_pose(part, breath);
                transform.translation = Vec3::new(x, ground_y, z) + offset + Vec3::Y * hop;
                transform.scale = scale;
                transform.rotation = Quat::from_rotation_y((t * 0.4 + id as f32).sin() * 0.35);
            }
            EcoVisual::BerryBush { id } => {
                let Some(berry) = eco.berries.iter().find(|berry| berry.id == id) else {
                    transform.scale = Vec3::ZERO;
                    continue;
                };
                let x = berry.pos.x;
                let z = berry.pos.y;
                let ground_y = effective_ground_height(&game_world, x as i32, z as i32);
                let sway = (t * 1.4 + id as f32).sin() * 0.035;
                transform.translation = Vec3::new(x, ground_y + 0.36 + sway, z);
                transform.scale = Vec3::new(0.95, 0.58 + sway.abs(), 0.95);
            }
            EcoVisual::BerryFruit { id, index } => {
                let Some(berry) = eco.berries.iter().find(|berry| berry.id == id) else {
                    transform.scale = Vec3::ZERO;
                    continue;
                };
                if index >= eco_fruit_marker_count(berry.fruit) {
                    transform.scale = Vec3::ZERO;
                    continue;
                }
                let x = berry.pos.x;
                let z = berry.pos.y;
                let ground_y = effective_ground_height(&game_world, x as i32, z as i32);
                let angle = index as f32 * std::f32::consts::TAU / 3.0 + id as f32 * 0.35;
                let bob = (t * 3.0 + index as f32).sin() * 0.035;
                transform.translation = Vec3::new(
                    x + angle.cos() * 0.35,
                    ground_y + 0.68 + bob,
                    z + angle.sin() * 0.35,
                );
                transform.scale = Vec3::splat(0.18);
            }
            EcoVisual::Co2Bubble { index } => {
                if eco.rabbits.is_empty() {
                    transform.scale = Vec3::ZERO;
                    continue;
                }
                let rabbit = &eco.rabbits[index as usize % eco.rabbits.len()];
                let x = rabbit.pos.x;
                let z = rabbit.pos.y;
                let ground_y = effective_ground_height(&game_world, x as i32, z as i32);
                let phase = t * 1.8 + index as f32 * 1.13;
                let radius = 0.35 + index as f32 * 0.08;
                let alpha_scale = (eco.co2 / 2.0).clamp(0.35, 1.35);
                transform.translation = Vec3::new(
                    x + phase.cos() * radius,
                    ground_y + 0.95 + phase.sin().abs() * 0.75,
                    z + phase.sin() * radius,
                );
                transform.scale = Vec3::splat((0.18 + index as f32 * 0.025) * alpha_scale);
            }
        }
    }
}

fn rabbit_part_pose(part: RabbitVisualPart, breath: f32) -> (Vec3, Vec3) {
    match part {
        RabbitVisualPart::Body => (
            Vec3::new(0.0, 0.34 + breath, 0.0),
            Vec3::new(0.70, 0.42, 0.95),
        ),
        RabbitVisualPart::Head => (
            Vec3::new(0.0, 0.58 + breath, -0.42),
            Vec3::new(0.42, 0.38, 0.42),
        ),
        RabbitVisualPart::EarLeft => (
            Vec3::new(-0.13, 0.95 + breath, -0.47),
            Vec3::new(1.0, 1.0, 1.0),
        ),
        RabbitVisualPart::EarRight => (
            Vec3::new(0.13, 0.95 + breath, -0.47),
            Vec3::new(1.0, 1.0, 1.0),
        ),
        RabbitVisualPart::Tail => (Vec3::new(0.0, 0.43 + breath, 0.48), Vec3::splat(0.26)),
    }
}

pub fn animate_avatar(
    mut q: Query<(&mut Transform, &AvatarPart)>,
    player: Res<PlayerState>,
    game_world: Res<GameWorld>,
    state: Res<PlayerAnimState>,
) {
    let ground_top = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
    let base_x = player.pos.x;
    let base_z = player.pos.z;

    let phase = state.step_phase;
    let speed = state.smoothed_speed;
    let idle = (1.0 - speed).max(0.0);
    let breath = (phase * 0.18).sin() * 0.03 * idle;

    let move_world = state.smoothed_move_world;
    let lean_strength = 0.18 * speed.max(0.1);
    let lean_pitch = -move_world.y * lean_strength;
    let lean_roll = move_world.x * lean_strength;

    let vy = state.vertical_vel;
    let stretch = (1.0 + vy * 0.06).clamp(0.85, 1.15);

    for (mut transform, part) in q.iter_mut() {
        let p = phase + part.sokpop.phase_offset;
        let amp = part.sokpop.bob_amp;

        let sway_x = p.sin() * amp.x;
        let bob_y = p.sin() * amp.y + breath;
        let sway_z = p.cos() * amp.z * 0.5;

        let rest = part.offset;
        transform.translation = Vec3::new(
            base_x + rest.x + sway_x,
            ground_top + rest.y + bob_y,
            base_z + rest.z + sway_z,
        );

        let lf = part.sokpop.lean_factor;
        let pitch = lean_pitch * lf;
        let roll = lean_roll * lf;
        transform.rotation = Quat::from_euler(EulerRot::YXZ, 0.0, pitch, roll);

        let scale = match part.sokpop.squash_axis {
            SquashAxis::Y => Vec3::new(
                part.rest_scale.x / stretch,
                part.rest_scale.y * stretch,
                part.rest_scale.z / stretch,
            ),
            SquashAxis::None => part.rest_scale,
        };
        transform.scale = scale;
    }
}

pub fn follow_monster_cubes(
    mut q: Query<(&mut Transform, &mut MonsterCube)>,
    game_world: Res<GameWorld>,
    player: Res<PlayerState>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (mut t, mut mc) in q.iter_mut() {
        let to_player = player.pos - mc.base;
        let dist = (to_player.x * to_player.x + to_player.z * to_player.z).sqrt();
        let dir = if dist > 0.1 {
            Vec3::new(to_player.x / dist, 0.0, to_player.z / dist)
        } else {
            Vec3::ZERO
        };

        let speed = if dist < 12.0 { 0.4 } else { 0.0 };
        let new_base = mc.base + dir * speed * dt;

        let new_dist =
            ((new_base.x - player.pos.x).powi(2) + (new_base.z - player.pos.z).powi(2)).sqrt();
        mc.base = if new_dist < 2.5 { mc.base } else { new_base };
        let ground_top = effective_ground_height(&game_world, mc.base.x as i32, mc.base.z as i32);
        t.translation = Vec3::new(mc.base.x, ground_top + 0.5, mc.base.z);
    }
}

pub fn animate_monsters(time: Res<Time>, mut q: Query<(&mut Transform, &MonsterCube)>) {
    let t = time.elapsed_secs();
    for (i, (mut transform, monster)) in q.iter_mut().enumerate() {
        let phase = (i as f32) * 0.7;

        let bob = (t * 1.5 + phase).sin() * 0.12;
        transform.translation = monster.base + Vec3::Y * bob;

        transform.rotate_y(0.4 * time.delta_secs());
    }
}

pub fn animate_cloud_puffs(time: Res<Time>, mut q: Query<(&mut Transform, &CloudPuff)>) {
    let t = time.elapsed_secs();
    for (mut tf, puff) in q.iter_mut() {
        let bob = (t * 0.6 + puff.phase).sin() * 0.15;
        tf.translation = puff.base + Vec3::Y * bob;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eco_fruit_marker_count_is_capped_for_readability() {
        assert_eq!(eco_fruit_marker_count(0), 0);
        assert_eq!(eco_fruit_marker_count(1), 1);
        assert_eq!(eco_fruit_marker_count(3), 3);
        assert_eq!(eco_fruit_marker_count(9), 3);
    }
}
