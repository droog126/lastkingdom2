use bevy::prelude::*;
use lk2_core::eco_cycle::EcoCycle;
use lk2_core::ecology::{
    ecology_entry, EcologyKind, ResourceDropKind, ResourceNodeKind, TreeKind, WildlifeKind,
};
use lk2_core::player::PlayerState;
use lk2_core::world::World as GameWorld;

use crate::render::scalar_field::effective_ground_height;
use crate::render::CameraAngles;

#[cfg(feature = "audit-pretty-models")]
mod audit_pretty;

#[derive(Resource, Debug, Clone)]
pub struct PrettyConfig {
    pub show_water: bool,
    pub show_player_avatar: bool,
    pub show_monster_cubes: bool,
    pub show_legacy_debug_props: bool,
}

impl Default for PrettyConfig {
    fn default() -> Self {
        Self {
            show_water: false,
            show_player_avatar: true,
            show_monster_cubes: false,
            show_legacy_debug_props: false,
        }
    }
}

#[derive(Component)]
pub struct WaterMarker;

#[derive(Component)]
pub struct GrassPlatformMarker;

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

pub fn follow_grass_platform(
    player: Res<PlayerState>,
    game_world: Res<GameWorld>,
    mut q: Query<&mut Transform, With<GrassPlatformMarker>>,
) {
    let Ok(mut tf) = q.single_mut() else {
        return;
    };
    let ground_top = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
    tf.translation = Vec3::new(player.pos.x, ground_top + 0.035, player.pos.z);
}

#[derive(Component)]
pub struct AvatarPart {
    pub offset: Vec3,
    pub rest_scale: Vec3,
    pub sokpop: SokpopAnim,
}

#[derive(Component)]
pub struct PlayerAvatarModel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AvatarPartKind {
    Head,
    Hair,
    HeadDetail,
    Torso,
    Thigh,
    Shin,
    Hand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SquashAxis {
    None,
    Y,
}

#[derive(Clone, Copy, Debug)]
pub struct SokpopAnim {
    pub phase_offset: f32,
    pub bob_amp: Vec3,
    pub lean_factor: f32,
    pub squash_axis: SquashAxis,
}

fn avatar_sokpop(kind: AvatarPartKind) -> SokpopAnim {
    match kind {
        AvatarPartKind::Head => SokpopAnim {
            phase_offset: 0.0,
            bob_amp: Vec3::new(0.02, 0.05, 0.0),
            lean_factor: 0.85,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::Hair => SokpopAnim {
            phase_offset: 0.25,
            bob_amp: Vec3::new(0.02, 0.06, 0.0),
            lean_factor: 0.9,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::HeadDetail => SokpopAnim {
            phase_offset: 0.15,
            bob_amp: Vec3::new(0.02, 0.045, 0.0),
            lean_factor: 0.95,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::Torso => SokpopAnim {
            phase_offset: 0.0,
            bob_amp: Vec3::new(0.04, 0.06, 0.0),
            lean_factor: 1.0,
            squash_axis: SquashAxis::Y,
        },
        AvatarPartKind::Thigh => SokpopAnim {
            phase_offset: 0.0,
            bob_amp: Vec3::new(0.16, 0.09, 0.0),
            lean_factor: 0.25,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::Shin => SokpopAnim {
            phase_offset: std::f32::consts::PI,
            bob_amp: Vec3::new(0.18, 0.05, 0.0),
            lean_factor: 0.15,
            squash_axis: SquashAxis::None,
        },
        AvatarPartKind::Hand => SokpopAnim {
            phase_offset: std::f32::consts::PI,
            bob_amp: Vec3::new(0.22, 0.06, 0.0),
            lean_factor: 0.7,
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
    Rabbit { id: u32 },
    Wildlife { id: u32 },
    BerryBush { id: u32 },
    BerryFruit { id: u32, index: u32 },
    PlantNode { id: u32 },
    Co2Bubble { index: u32 },
}

pub fn eco_fruit_marker_count(fruit: u32) -> u32 {
    fruit.clamp(1, 3)
}

const ECO_CO2_BUBBLE_COUNT: u32 = 6;

// Animation-speed normalization baseline (PlayerState.pos delta m/s => smoothed_speed ~1.0).
// Matches the offline walk speed used in render::player_input's MANUAL_MOVE_SPEED.
const MANUAL_MOVE_ANIM_SPEED: f32 = 4.5;

#[derive(Resource, Default)]
pub struct PlayerAnimState {
    pub step_phase: f32,
    pub smoothed_speed: f32,
    pub smoothed_move_world: Vec2,
    pub last_pos_y: f32,
    pub last_horizontal_pos_xz: Option<Vec2>,
    pub vertical_vel: f32,
    pub initialized: bool,
}

pub fn update_player_anim_state(
    time: Res<Time>,
    player: Res<PlayerState>,
    mut state: ResMut<PlayerAnimState>,
) {
    let dt = time.delta_secs().max(0.0001);

    if !state.initialized {
        state.last_pos_y = player.pos.y;
        state.last_horizontal_pos_xz = Some(Vec2::new(player.pos.x, player.pos.z));
        state.initialized = true;
    }

    let smooth_k = 1.0 - (-dt * 8.0).exp();

    // Derive animation speed + direction from PlayerState.pos deltas.
    // In offline mode, PlayerState.pos is updated by try_player_move_continuous.
    // In online mode, PlayerState.pos mirrors the server-replicated PlayerPos
    // (wired via apply_networked_position), so we still get a moving avatar.
    let prev = state.last_horizontal_pos_xz.unwrap_or(Vec2::new(player.pos.x, player.pos.z));
    let curr = Vec2::new(player.pos.x, player.pos.z);
    let delta_h = (curr - prev) / dt;
    state.last_horizontal_pos_xz = Some(curr);
    state.last_pos_y = player.pos.y;

    // Normalize to a unit vector for direction; speed is the magnitude scaled
    // to roughly match the legacy 0..1 move_input range (manual walk ~4.5 m/s).
    let raw_h = delta_h.length();
    let direction = if raw_h > 0.0001 {
        delta_h / raw_h
    } else {
        Vec2::ZERO
    };
    // 4.5 m/s baseline maps to 1.0 (sprint ~6.75 above is fine, clamped below).
    let raw_speed = (raw_h / MANUAL_MOVE_ANIM_SPEED).min(1.5);
    state.smoothed_speed = state.smoothed_speed + (raw_speed - state.smoothed_speed) * smooth_k;

    // direction is in world (x,z); avatar facing tracks actual world movement.
    let target_move = direction;
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
pub struct KenneyLandmark {
    /// Local offset from the player: x = camera-right, y = vertical, z = camera-forward.
    pub rel: Vec3,
    pub y_offset: f32,
}

#[allow(unreachable_code)]
pub fn spawn_pretty(
    mut commands: Commands,
    game_world: Res<GameWorld>,
    player: Res<PlayerState>,
    cfg: Res<PrettyConfig>,
    asset_server: Res<AssetServer>,
    camera_mode: Res<crate::render::CameraMode>,
    camera_angles: Res<CameraAngles>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // First-person hides only camera-obstructing presentation actors. Low
    // ground props stay visible so the map does not look empty.
    let first_person_mode = *camera_mode == crate::render::CameraMode::FirstPerson;

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
    let auto_demo_mode = std::env::args().any(|a| a == "--auto-demo");

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

    if !first_person_mode {
        let grass_size = 180.0_f32;
        let ground_y =
            effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(grass_size, grass_size))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.62, 0.18),
                emissive: Color::srgb(0.03, 0.10, 0.02).into(),
                perceptual_roughness: 0.96,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_translation(Vec3::new(player.pos.x, ground_y + 0.035, player.pos.z)),
            GrassPlatformMarker,
        ));

        spawn_playable_village_diorama(
            &mut commands,
            &game_world,
            player.pos,
            &asset_server,
            &mut meshes,
            &mut materials,
        );
    }

    if !first_person_mode {
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

    if cfg.show_player_avatar && !first_person_mode {
        let ground_top =
            effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
        commands.spawn((
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("procedural/pretty/sokpop_gatherer.glb"),
            )),
            Transform::from_translation(Vec3::new(player.pos.x, ground_top + 0.24, player.pos.z))
                .with_rotation(Quat::from_rotation_y(camera_angles.yaw))
                .with_scale(Vec3::splat(0.62)),
            PlayerAvatarModel,
        ));
        info!("spawned sokpop gatherer avatar at {:?}", player.pos);
    }

    if cfg.show_legacy_debug_props && !first_person_mode {
        let tree_offset = local_offset_from_yaw(Vec3::new(4.8, 0.0, 9.0), camera_angles.yaw);
        let tree_x = player.pos.x + tree_offset.x;
        let tree_z = player.pos.z + tree_offset.z;
        let tree_ground =
            effective_ground_height(&game_world, tree_x.floor() as i32, tree_z.floor() as i32);
        commands.spawn((
            WorldAssetRoot(
                asset_server.load(
                    GltfAssetLabel::Scene(0)
                        .from_asset(ecology_entry(EcologyKind::Tree(TreeKind::Sokpop)).model_path),
                ),
            ),
            Transform::from_translation(Vec3::new(tree_x, tree_ground + 0.05, tree_z))
                .with_rotation(Quat::from_rotation_y(camera_angles.yaw + 0.35))
                .with_scale(ecology_entry(EcologyKind::Tree(TreeKind::Sokpop)).visual_scale),
        ));

        let stick_offset = local_offset_from_yaw(Vec3::new(1.1, 0.0, 2.1), camera_angles.yaw);
        let stick_x = player.pos.x + stick_offset.x;
        let stick_z = player.pos.z + stick_offset.z;
        let stick_ground =
            effective_ground_height(&game_world, stick_x.floor() as i32, stick_z.floor() as i32);
        commands.spawn((
            WorldAssetRoot(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(
                    ecology_entry(EcologyKind::Tree(TreeKind::FallenStick)).model_path,
                )),
            ),
            Transform::from_translation(Vec3::new(stick_x, stick_ground + 0.08, stick_z))
                .with_rotation(Quat::from_rotation_y(camera_angles.yaw - 0.7))
                .with_scale(ecology_entry(EcologyKind::Tree(TreeKind::FallenStick)).visual_scale),
        ));
        info!("spawned sokpop tree and fallen stick near player");
    }

    if false && cfg.show_player_avatar && !first_person_mode {
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

    if cfg.show_monster_cubes && !first_person_mode && !auto_demo_mode {
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

    let cloud_layouts: [(f32, f32, f32, f32, f32); 4] = if first_person_mode {
        [
            (0.7, 42.0, 33.0, 1.0, 0.7),
            (2.1, 48.0, 36.0, 0.8, 1.0),
            (3.8, 56.0, 39.0, 1.2, 0.8),
            (5.4, 46.0, 35.0, 0.7, 0.7),
        ]
    } else {
        [
            (0.7, 22.0, 18.0, 1.0, 0.7),
            (2.1, 18.0, 20.0, 0.8, 1.0),
            (3.8, 25.0, 22.0, 1.2, 0.8),
            (5.4, 16.0, 19.0, 0.7, 0.7),
        ]
    };
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

    if first_person_mode {
        if !auto_demo_mode {
            spawn_first_person_village(
                &mut commands,
                &asset_server,
                &game_world,
                player.pos,
                camera_angles.yaw,
            );
        }
        if kenney_enabled && !auto_demo_mode {
            spawn_first_person_camp_props(
                &mut commands,
                &asset_server,
                &game_world,
                player.pos,
                camera_angles.yaw,
            );
        }
        return;
    }

    if cfg.show_legacy_debug_props && !first_person_mode && !auto_demo_mode {
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
        if auto_demo_mode || !cfg.show_legacy_debug_props {
            break;
        }
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
        if auto_demo_mode || !cfg.show_legacy_debug_props {
            break;
        }
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

    for i in 0..28 {
        if auto_demo_mode || !cfg.show_legacy_debug_props {
            break;
        }
        let angle = i as f32 * 2.3999631;
        let r = 4.0 + (i % 7) as f32 * 2.15;
        let x = player.pos.x + angle.cos() * r;
        let z = player.pos.z + angle.sin() * r;
        let ground = effective_ground_height(&game_world, x as i32, z as i32);
        let color = match i % 4 {
            0 => Color::srgb(0.28, 0.55, 0.18),
            1 => Color::srgb(0.36, 0.62, 0.22),
            2 => Color::srgb(0.18, 0.45, 0.26),
            _ => Color::srgb(0.52, 0.42, 0.24),
        };
        spawn_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(x, ground + 0.18, z),
            Vec3::new(0.32, 0.36, 0.32),
            color,
        );
    }

    // Kenney landmarks + decorative markers stay visible in first-person so the
    // map does not look bare. Y is anchored to player.pos.y, NOT ground_y, so
    // the props sit at the player's eye level even when the surrounding terrain
    // has tall hills (ground_y at the offset (x, z) can be 35+ while player.pos.y
    // is ~15 — that mismatch was the root cause of "kenney landmark Y=34.65, 19m
    // above player's head, first-person cannot see").
    let landmark_anchor_y = player.pos.y;
    if cfg.show_legacy_debug_props && !auto_demo_mode {
        spawn_v2_crown_season_markers(
            &mut commands,
            &mut meshes,
            &mut materials,
            player.pos,
            landmark_anchor_y,
        );
    }
    if cfg.show_legacy_debug_props && kenney_enabled && !auto_demo_mode {
        spawn_kenney_landmarks(&mut commands, &asset_server, player.pos, landmark_anchor_y);
    }

    if !first_person_mode {
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

fn local_offset_from_yaw(offset: Vec3, yaw: f32) -> Vec3 {
    let forward = Vec3::new(yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, yaw.sin());
    right * offset.x + Vec3::Y * offset.y + forward * offset.z
}

fn spawn_scene_asset(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    _game_world: &GameWorld,
    player_pos: Vec3,
    yaw: f32,
    path: &'static str,
    offset: Vec3,
    scale: f32,
    asset_yaw: f32,
) {
    let world_offset = local_offset_from_yaw(offset, yaw);
    let x = player_pos.x + world_offset.x;
    let z = player_pos.z + world_offset.z;
    // Anchor Y to player_pos.y so first-person village buildings, camp props
    // and kenney markers sit at eye level instead of 19m+ above the player on
    // tall-hill terrain (root cause C of iter_200).
    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(path));
    commands.spawn((
        WorldAssetRoot(scene),
        Transform::from_translation(Vec3::new(x, player_pos.y + offset.y, z))
            .with_rotation(Quat::from_rotation_y(yaw + asset_yaw))
            .with_scale(Vec3::splat(scale)),
    ));
}

fn spawn_first_person_village(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    game_world: &GameWorld,
    player_pos: Vec3,
    yaw: f32,
) {
    const BUILDINGS: &[(&str, Vec3, f32, f32)] = &[
        (
            "procedural/pretty/house_small.glb",
            Vec3::new(-7.0, 0.0, 20.0),
            2.2,
            0.20,
        ),
        (
            "procedural/pretty/tavern.glb",
            Vec3::new(7.0, 0.0, 23.0),
            2.1,
            -0.25,
        ),
        (
            "procedural/pretty/well.glb",
            Vec3::new(0.0, 0.0, 18.0),
            1.7,
            0.0,
        ),
        (
            "procedural/pretty/barn.glb",
            Vec3::new(-12.0, 0.0, 30.0),
            2.3,
            0.35,
        ),
        (
            "procedural/pretty/windmill.glb",
            Vec3::new(13.0, 0.0, 33.0),
            2.2,
            -0.45,
        ),
        (
            "procedural/pretty/watchtower.glb",
            Vec3::new(-18.0, 0.0, 38.0),
            2.1,
            0.10,
        ),
        (
            "procedural/pretty/market_stall.glb",
            Vec3::new(5.0, 0.0, 15.0),
            1.8,
            0.65,
        ),
        (
            "procedural/pretty/fence.glb",
            Vec3::new(-4.0, 0.0, 13.0),
            2.0,
            1.57,
        ),
        (
            "procedural/pretty/fence.glb",
            Vec3::new(10.0, 0.0, 17.0),
            2.0,
            1.57,
        ),
        (
            "procedural/pretty/signpost.glb",
            Vec3::new(-1.8, 0.0, 12.0),
            1.4,
            -0.35,
        ),
        (
            "procedural/pretty/lantern_post.glb",
            Vec3::new(3.2, 0.0, 12.0),
            1.6,
            0.0,
        ),
        (
            "procedural/pretty/crate.glb",
            Vec3::new(-8.5, 0.0, 14.0),
            1.4,
            0.4,
        ),
        (
            "procedural/pretty/barrel.glb",
            Vec3::new(8.8, 0.0, 14.5),
            1.4,
            -0.2,
        ),
    ];

    for (path, offset, scale, asset_yaw) in BUILDINGS {
        spawn_scene_asset(
            commands,
            asset_server,
            game_world,
            player_pos,
            yaw,
            path,
            *offset,
            *scale,
            *asset_yaw,
        );
    }
}

fn spawn_first_person_camp_props(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    game_world: &GameWorld,
    player_pos: Vec3,
    yaw: f32,
) {
    const PROPS: &[(&str, Vec3, f32, f32)] = &[
        (
            "kenney/curated/survival_props/kenney_tent.glb",
            Vec3::new(-13.0, 0.0, 17.0),
            1.4,
            0.35,
        ),
        (
            "kenney/curated/survival_props/kenney_campfire_pit.glb",
            Vec3::new(-10.0, 0.0, 15.0),
            1.3,
            0.0,
        ),
        (
            "kenney/curated/survival_props/kenney_workbench.glb",
            Vec3::new(12.0, 0.0, 18.0),
            1.3,
            -0.45,
        ),
        (
            "kenney/curated/characters/kenney_villager_male_a.glb",
            Vec3::new(-2.8, 0.0, 16.0),
            1.0,
            0.15,
        ),
        (
            "kenney/curated/characters/kenney_villager_female_a.glb",
            Vec3::new(2.8, 0.0, 16.5),
            1.0,
            -0.15,
        ),
    ];

    for (path, offset, scale, asset_yaw) in PROPS {
        spawn_scene_asset(
            commands,
            asset_server,
            game_world,
            player_pos,
            yaw,
            path,
            *offset,
            *scale,
            *asset_yaw,
        );
    }
}

fn spawn_kenney_landmarks(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    player_pos: Vec3,
    _ground_y: f32,
) {
    const LANDMARKS: &[(&str, &str, Vec3, f32, f32)] = &[
        (
            "kenney_campfire_pit",
            "kenney/curated/survival_props/kenney_campfire_pit.glb",
            Vec3::new(-5.5, -0.35, -8.0),
            1.0,
            0.0,
        ),
        (
            "kenney_tent",
            "kenney/curated/survival_props/kenney_tent.glb",
            Vec3::new(-8.0, -0.35, -10.5),
            1.0,
            0.4,
        ),
        (
            "kenney_workbench",
            "kenney/curated/survival_props/kenney_workbench.glb",
            Vec3::new(-3.0, -0.35, -10.0),
            1.0,
            -0.2,
        ),
        (
            "kenney_resource_wood",
            "kenney/curated/survival_props/kenney_resource_wood.glb",
            Vec3::new(-2.0, -0.35, -6.0),
            1.0,
            0.2,
        ),
        (
            "kenney_resource_stone",
            "kenney/curated/survival_props/kenney_resource_stone.glb",
            Vec3::new(-8.5, -0.35, -6.5),
            1.0,
            -0.3,
        ),
        (
            "kenney_tool_axe",
            "kenney/curated/survival_props/kenney_tool_axe.glb",
            Vec3::new(-4.3, -0.20, -5.0),
            1.0,
            0.9,
        ),
        (
            "kenney_tool_pickaxe",
            "kenney/curated/survival_props/kenney_tool_pickaxe.glb",
            Vec3::new(-5.8, -0.20, -5.2),
            1.0,
            0.7,
        ),
        (
            "kenney_row_boat_small",
            "kenney/curated/coastal_and_pirate/kenney_row_boat_small.glb",
            Vec3::new(11.0, -0.30, 13.5),
            1.0,
            -0.8,
        ),
        (
            "kenney_dock_platform",
            "kenney/curated/coastal_and_pirate/kenney_dock_platform.glb",
            Vec3::new(8.0, -0.35, 12.0),
            1.0,
            0.0,
        ),
        (
            "kenney_palm_straight",
            "kenney/curated/coastal_and_pirate/kenney_palm_straight.glb",
            Vec3::new(14.0, -0.35, 10.0),
            1.0,
            0.2,
        ),
        (
            "kenney_ship_wreck",
            "kenney/curated/coastal_and_pirate/kenney_ship_wreck.glb",
            Vec3::new(17.0, -0.35, 15.0),
            1.0,
            -0.5,
        ),
        (
            "kenney_pirate_flag",
            "kenney/curated/coastal_and_pirate/kenney_pirate_flag.glb",
            Vec3::new(6.0, -0.35, 13.0),
            1.0,
            0.0,
        ),
        (
            "kenney_cannon",
            "kenney/curated/coastal_and_pirate/kenney_cannon.glb",
            Vec3::new(5.0, -0.35, 10.0),
            1.0,
            1.2,
        ),
        (
            "kenney_villager_male_a",
            "kenney/curated/characters/kenney_villager_male_a.glb",
            Vec3::new(3.0, -0.35, -7.0),
            1.0,
            -0.4,
        ),
        (
            "kenney_villager_female_a",
            "kenney/curated/characters/kenney_villager_female_a.glb",
            Vec3::new(5.0, -0.35, -9.0),
            1.0,
            0.6,
        ),
        (
            "kenney_bunny",
            "kenney/curated/animals/kenney_bunny.glb",
            Vec3::new(8.0, -0.35, -4.0),
            1.0,
            -0.7,
        ),
        (
            "kenney_deer",
            "kenney/curated/animals/kenney_deer.glb",
            Vec3::new(13.0, -0.35, -7.0),
            1.0,
            0.1,
        ),
        (
            "kenney_cow",
            "kenney/curated/animals/kenney_cow.glb",
            Vec3::new(11.0, -0.35, -12.5),
            1.0,
            -0.6,
        ),
        (
            "kenney_fox",
            "kenney/curated/animals/kenney_fox.glb",
            Vec3::new(15.5, -0.35, -2.5),
            1.0,
            0.8,
        ),
        (
            "kenney_bear",
            "kenney/curated/animals/kenney_bear.glb",
            Vec3::new(18.0, -0.35, -9.0),
            1.0,
            -0.9,
        ),
        (
            "kenney_block_grass_large",
            "kenney/curated/terrain_and_pickups/kenney_block_grass_large.glb",
            Vec3::new(0.0, -0.35, 8.0),
            1.0,
            0.0,
        ),
        (
            "kenney_block_snow_large",
            "kenney/curated/terrain_and_pickups/kenney_block_snow_large.glb",
            Vec3::new(2.8, -0.35, 8.0),
            1.0,
            0.0,
        ),
        (
            "kenney_coin_gold",
            "kenney/curated/terrain_and_pickups/kenney_coin_gold.glb",
            Vec3::new(1.2, 0.25, 5.5),
            1.0,
            0.0,
        ),
        (
            "kenney_heart",
            "kenney/curated/terrain_and_pickups/kenney_heart.glb",
            Vec3::new(2.6, 0.35, 5.8),
            1.0,
            0.0,
        ),
        (
            "kenney_door_open",
            "kenney/curated/terrain_and_pickups/kenney_door_open.glb",
            Vec3::new(-1.5, -0.35, 7.0),
            1.0,
            0.0,
        ),
    ];

    for (name, path, offset, scale, yaw) in LANDMARKS {
        let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(*path));
        // Anchor Y to player.pos.y so kenney props sit at eye level in
        // first-person view. The previous ground_y-based Y could be 19m+ above
        // the player's head (see iter_200 task root cause C).
        let pos = Vec3::new(
            player_pos.x + offset.x,
            player_pos.y + offset.y,
            player_pos.z + offset.z,
        );
        commands.spawn((
            WorldAssetRoot(scene),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(*yaw))
                .with_scale(Vec3::splat(*scale)),
            KenneyLandmark { rel: *offset, y_offset: offset.y },
        ));
        info!("[kenney] spawned landmark {} at {:?}", name, pos);
    }
}

pub fn follow_kenney_landmarks(
    player: Res<PlayerState>,
    _game_world: Res<GameWorld>,
    camera_angles: Res<CameraAngles>,
    mut q: Query<(&mut Transform, &KenneyLandmark)>,
) {
    let yaw = camera_angles.yaw;
    let forward = Vec3::new(yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, yaw.sin());
    for (mut transform, landmark) in &mut q {
        let rel = right * landmark.rel.x + forward * landmark.rel.z;
        let x = player.pos.x + rel.x;
        let z = player.pos.z + rel.z;
        // Track player.pos.y instead of ground_y so props stay anchored to the
        // player (which is what makes them visible in first-person).
        transform.translation = Vec3::new(x, player.pos.y + landmark.y_offset, z);
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

fn spawn_playable_village_diorama(
    commands: &mut Commands,
    game_world: &GameWorld,
    player_pos: Vec3,
    asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let grass_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.50, 0.16),
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    let road = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.58, 0.49),
        perceptual_roughness: 0.86,
        metallic: 0.0,
        ..default()
    });
    let field = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.66, 0.26),
        perceptual_roughness: 0.88,
        metallic: 0.0,
        ..default()
    });
    let soil = materials.add(StandardMaterial {
        base_color: Color::srgb(0.36, 0.24, 0.14),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    let rock = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.47, 0.42),
        perceptual_roughness: 0.84,
        metallic: 0.0,
        ..default()
    });

    let road_mesh = meshes.add(Cuboid::new(1.0, 0.05, 1.0));
    for (ox, oz, sx, sz) in [
        (0.0, -5.0, 2.0, 34.0),
        (-10.0, -5.0, 1.4, 28.0),
        (10.0, -5.0, 1.4, 28.0),
        (0.0, -13.0, 28.0, 1.7),
        (0.0, 4.0, 28.0, 1.7),
    ] {
        let pos = grounded_world_pos(game_world, player_pos, ox, oz, 0.03);
        commands.spawn((
            Mesh3d(road_mesh.clone()),
            MeshMaterial3d(road.clone()),
            Transform::from_translation(pos).with_scale(Vec3::new(sx, 1.0, sz)),
        ));
    }

    let patch_mesh = meshes.add(Cuboid::new(1.0, 0.03, 1.0));
    for i in 0..46 {
        let ox = -22.0 + pretty_hash01(i, 11) * 44.0;
        let oz = -24.0 + pretty_hash01(i, 17) * 38.0;
        if ox.abs() < 3.0 || (oz + 13.0).abs() < 2.0 || (oz - 4.0).abs() < 2.0 {
            continue;
        }
        let pos = grounded_world_pos(game_world, player_pos, ox, oz, 0.04);
        commands.spawn((
            Mesh3d(patch_mesh.clone()),
            MeshMaterial3d(grass_dark.clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(pretty_hash01(i, 23) * std::f32::consts::TAU))
                .with_scale(Vec3::new(
                    0.7 + pretty_hash01(i, 29) * 1.5,
                    1.0,
                    0.5 + pretty_hash01(i, 31) * 1.3,
                )),
        ));
    }

    let farm_mesh = meshes.add(Cuboid::new(1.0, 0.05, 1.0));
    let wheat_mesh = meshes.add(Cuboid::new(0.15, 0.58, 0.15));
    for (cx, cz, sx, sz) in [(-15.0, 8.0, 7.0, 6.0), (15.0, 8.0, 7.0, 6.0)] {
        let pos = grounded_world_pos(game_world, player_pos, cx, cz, 0.04);
        commands.spawn((
            Mesh3d(farm_mesh.clone()),
            MeshMaterial3d(soil.clone()),
            Transform::from_translation(pos).with_scale(Vec3::new(sx, 1.0, sz)),
        ));
        for ix in -4..=4 {
            for iz in -3..=3 {
                if (ix + iz) % 2 != 0 {
                    continue;
                }
                let wheat_pos = grounded_world_pos(
                    game_world,
                    player_pos,
                    cx + ix as f32 * 0.62,
                    cz + iz as f32 * 0.62,
                    0.30,
                );
                commands.spawn((
                    Mesh3d(wheat_mesh.clone()),
                    MeshMaterial3d(field.clone()),
                    Transform::from_translation(wheat_pos),
                ));
            }
        }
    }

    for (path, ox, oz, scale, yaw) in [
        ("procedural/pretty/house_small.glb", -6.0, -16.0, 0.62, 0.35),
        ("procedural/pretty/house_small.glb", 6.5, -16.0, 0.58, -0.2),
        ("procedural/pretty/house_small.glb", 15.5, -4.0, 0.56, 0.15),
        ("procedural/pretty/chapel.glb", -15.0, -3.0, 0.56, 0.1),
        ("procedural/pretty/well.glb", -8.0, -8.5, 0.44, 0.0),
        ("procedural/pretty/market_stall.glb", 7.0, 2.0, 0.56, 0.7),
    ] {
        spawn_grounded_scene(commands, game_world, player_pos, asset_server, path, ox, oz, 0.03, scale, yaw);
    }

    for (ox, oz, scale) in [
        (-23.0, -20.0, 0.52),
        (-20.0, -12.0, 0.48),
        (-22.0, -2.0, 0.50),
        (-18.0, 11.0, 0.46),
        (-12.0, 16.0, 0.44),
        (20.0, -19.0, 0.48),
        (22.0, -10.0, 0.46),
        (21.0, 2.0, 0.44),
        (20.0, 14.0, 0.42),
    ] {
        spawn_grounded_scene(
            commands,
            game_world,
            player_pos,
            asset_server,
            ecology_entry(EcologyKind::Tree(TreeKind::Sokpop)).model_path,
            ox,
            oz,
            0.03,
            scale,
            0.0,
        );
    }

    let rock_mesh = meshes.add(Cuboid::new(0.35, 0.20, 0.30));
    for i in 0..70 {
        let ox = -23.0 + pretty_hash01(i, 41) * 46.0;
        let oz = -22.0 + pretty_hash01(i, 43) * 42.0;
        let pos = grounded_world_pos(game_world, player_pos, ox, oz, 0.12);
        commands.spawn((
            Mesh3d(rock_mesh.clone()),
            MeshMaterial3d(rock.clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(pretty_hash01(i, 47) * std::f32::consts::TAU))
                .with_scale(Vec3::splat(0.55 + pretty_hash01(i, 53) * 0.75)),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_grounded_scene(
    commands: &mut Commands,
    game_world: &GameWorld,
    player_pos: Vec3,
    asset_server: &Res<AssetServer>,
    path: &'static str,
    ox: f32,
    oz: f32,
    y_offset: f32,
    scale: f32,
    yaw: f32,
) {
    commands.spawn((
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))),
        Transform::from_translation(grounded_world_pos(game_world, player_pos, ox, oz, y_offset))
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(scale)),
        Name::new(path),
    ));
}

fn grounded_world_pos(
    game_world: &GameWorld,
    player_pos: Vec3,
    ox: f32,
    oz: f32,
    y_offset: f32,
) -> Vec3 {
    let x = player_pos.x + ox;
    let z = player_pos.z + oz;
    let y = effective_ground_height(game_world, x.floor() as i32, z.floor() as i32) + y_offset;
    Vec3::new(x, y, z)
}

fn pretty_hash01(i: usize, salt: usize) -> f32 {
    let mut x = (i as u32)
        .wrapping_mul(1_664_525)
        .wrapping_add((salt as u32).wrapping_mul(1_013_904_223));
    x ^= x >> 16;
    x = x.wrapping_mul(2_246_822_519);
    ((x >> 8) as f32) / ((u32::MAX >> 8) as f32)
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
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let bubble_mesh = meshes.add(Sphere::new(0.5));
    let bubble_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.45, 0.90, 1.0, 0.62),
        emissive: Color::srgb(0.25, 0.75, 1.0).into(),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.18,
        metallic: 0.0,
        ..default()
    });

    let rabbit_entry = ecology_entry(EcologyKind::Wildlife(WildlifeKind::Rabbit));
    let bush_entry = ecology_entry(EcologyKind::ResourceNode(ResourceNodeKind::BerryBush));
    let fruit_entry = ecology_entry(EcologyKind::ResourceDrop(ResourceDropKind::BerryFruit));

    for rabbit in &eco.rabbits {
        spawn_eco_scene_visual(
            &mut commands,
            &asset_server,
            rabbit_entry.model_path,
            EcoVisual::Rabbit { id: rabbit.id },
            Vec3::new(rabbit.pos.x, 0.0, rabbit.pos.y),
            rabbit_entry.visual_scale,
        );
    }

    for animal in &eco.wildlife {
        let entry = ecology_entry(EcologyKind::Wildlife(animal.kind));
        spawn_eco_scene_visual(
            &mut commands,
            &asset_server,
            entry.model_path,
            EcoVisual::Wildlife { id: animal.id },
            Vec3::new(animal.pos.x, 0.0, animal.pos.y),
            entry.visual_scale,
        );
    }

    for berry in &eco.berries {
        spawn_eco_scene_visual(
            &mut commands,
            &asset_server,
            bush_entry.model_path,
            EcoVisual::BerryBush { id: berry.id },
            Vec3::new(berry.pos.x, 0.0, berry.pos.y),
            bush_entry.visual_scale,
        );
        for index in 0..3 {
            spawn_eco_scene_visual(
                &mut commands,
                &asset_server,
                fruit_entry.model_path,
                EcoVisual::BerryFruit { id: berry.id, index },
                Vec3::new(berry.pos.x, 0.0, berry.pos.y),
                fruit_entry.visual_scale,
            );
        }
    }

    for plant in &eco.plants {
        let entry = ecology_entry(EcologyKind::ResourceNode(plant.kind));
        spawn_eco_scene_visual(
            &mut commands,
            &asset_server,
            entry.model_path,
            EcoVisual::PlantNode { id: plant.id },
            Vec3::new(plant.pos.x, 0.0, plant.pos.y),
            entry.visual_scale,
        );
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

fn spawn_eco_scene_visual(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    path: &'static str,
    visual: EcoVisual,
    pos: Vec3,
    scale: Vec3,
) {
    commands.spawn((
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))),
        Transform::from_translation(pos).with_scale(scale),
        visual,
    ));
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
            EcoVisual::Rabbit { id } => {
                let Some(rabbit) = eco.rabbits.iter().find(|rabbit| rabbit.id == id) else {
                    transform.scale = Vec3::ZERO;
                    continue;
                };
                let x = rabbit.pos.x;
                let z = rabbit.pos.y;
                let ground_y = effective_ground_height(&game_world, x as i32, z as i32);
                let phase = t * 5.0 + id as f32 * 0.9;
                let hop = phase.sin().max(0.0) * 0.28;
                let breath = (t * 2.2 + id as f32).sin() * 0.03;
                transform.translation = Vec3::new(x, ground_y + 0.08 + hop + breath, z);
                transform.scale =
                    ecology_entry(EcologyKind::Wildlife(WildlifeKind::Rabbit)).visual_scale;
                transform.rotation = Quat::from_rotation_y((t * 0.4 + id as f32).sin() * 0.35);
            }
            EcoVisual::Wildlife { id } => {
                let Some(animal) = eco.wildlife.iter().find(|animal| animal.id == id) else {
                    transform.scale = Vec3::ZERO;
                    continue;
                };
                let x = animal.pos.x;
                let z = animal.pos.y;
                let ground_y = effective_ground_height(&game_world, x as i32, z as i32);
                let entry = ecology_entry(EcologyKind::Wildlife(animal.kind));
                let bob = (t * 1.8 + id as f32).sin() * 0.045;
                transform.translation = Vec3::new(x, ground_y + 0.08 + bob, z);
                transform.scale = entry.visual_scale;
                transform.rotation = Quat::from_rotation_y((t * 0.22 + id as f32).sin() * 0.45);
            }
            EcoVisual::BerryBush { id } => {
                let Some(berry) = eco.berries.iter().find(|berry| berry.id == id) else {
                    transform.scale = Vec3::ZERO;
                    continue;
                };
                let x = berry.pos.x;
                let z = berry.pos.y;
                let ground_y = effective_ground_height(&game_world, x as i32, z as i32);
                let sway = (t * 1.4 + id as f32).sin() * 0.015;
                transform.translation = Vec3::new(x, ground_y + 0.08 + sway, z);
                transform.scale =
                    ecology_entry(EcologyKind::ResourceNode(ResourceNodeKind::BerryBush))
                        .visual_scale
                        + Vec3::Y * sway.abs();
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
                    x + angle.cos() * 0.24,
                    ground_y + 0.42 + bob,
                    z + angle.sin() * 0.24,
                );
                transform.scale =
                    ecology_entry(EcologyKind::ResourceDrop(ResourceDropKind::BerryFruit))
                        .visual_scale;
            }
            EcoVisual::PlantNode { id } => {
                let Some(plant) = eco.plants.iter().find(|plant| plant.id == id) else {
                    transform.scale = Vec3::ZERO;
                    continue;
                };
                let x = plant.pos.x;
                let z = plant.pos.y;
                let ground_y = effective_ground_height(&game_world, x as i32, z as i32);
                let entry = ecology_entry(EcologyKind::ResourceNode(plant.kind));
                let sway = (t * 1.1 + id as f32 * 0.7).sin() * 0.025;
                transform.translation = Vec3::new(x, ground_y + 0.06 + sway, z);
                transform.scale = entry.visual_scale * if plant.stock == 0 { 0.65 } else { 1.0 };
                transform.rotation = Quat::from_rotation_y(id as f32 * 0.71);
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
                let radius = 0.55 + index as f32 * 0.10;
                let alpha_scale = (eco.co2 / 2.0).clamp(0.55, 1.55);
                transform.translation = Vec3::new(
                    x + phase.cos() * radius,
                    ground_y + 1.35 + phase.sin().abs() * 1.05,
                    z + phase.sin() * radius,
                );
                transform.scale = Vec3::splat((0.34 + index as f32 * 0.04) * alpha_scale);
            }
        }
    }
}

pub fn animate_avatar(
    mut q: Query<(&mut Transform, &AvatarPart)>,
    mut model_q: Query<&mut Transform, (With<PlayerAvatarModel>, Without<AvatarPart>)>,
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

    for mut transform in &mut model_q {
        let bob = (phase * 0.5).sin() * 0.035 * speed.max(0.15);
        let yaw = if state.smoothed_move_world.length_squared() > 0.001 {
            state.smoothed_move_world.x.atan2(state.smoothed_move_world.y)
        } else {
            transform.rotation.to_euler(EulerRot::YXZ).0
        };
        transform.translation = Vec3::new(base_x, ground_top + 0.24 + bob, base_z);
        transform.rotation = Quat::from_rotation_y(yaw);
        transform.scale = Vec3::splat(0.62);
    }

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
    fn eco_fruit_marker_count_keeps_empty_bushes_readable() {
        assert_eq!(eco_fruit_marker_count(0), 1);
        assert_eq!(eco_fruit_marker_count(1), 1);
        assert_eq!(eco_fruit_marker_count(3), 3);
        assert_eq!(eco_fruit_marker_count(9), 3);
    }
}
