use std::collections::HashSet;

use bevy::prelude::*;
use lk2_core::eco_cycle::EcoCycle;
use lk2_core::ecology::{
    EcologyKind, ResourceDropKind, ResourceNodeKind, TreeKind, WildlifeKind, ecology_entry,
};
use lk2_core::player::PlayerState;
use lk2_core::world::{BlockType, World as GameWorld};

use crate::render::scalar_field::effective_ground_height;
use crate::render::{CameraAngles, CameraMode, stable_scene_baseline_enabled};

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

#[derive(Component, Clone, Default)]
pub struct PlayerAvatarModel;

#[derive(Component, Clone, Default)]
pub struct PlayerReadabilityMarker {
    pub part: PlayerReadabilityMarkerPart,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerReadabilityMarkerPart {
    #[default]
    Ring,
    Arrow,
}

#[derive(Component)]
pub struct WorldGroundFallback;

const PLAYER_AVATAR_SCALE: f32 = 0.90;
const PLAYER_AVATAR_FOOT_TO_ORIGIN: f32 = 0.41;
const PLAYER_MARKER_RING_RADIUS: f32 = 1.08;
const PLAYER_MARKER_RING_THICKNESS: f32 = 0.08;
const PLAYER_MARKER_ARROW_LENGTH: f32 = 1.15;
const PLAYER_MARKER_ARROW_WIDTH: f32 = 0.20;
const PLAYER_MARKER_Y_OFFSET: f32 = 0.06;

fn player_avatar_translation(x: f32, ground_top: f32, z: f32, bob: f32) -> Vec3 {
    Vec3::new(
        x,
        ground_top + PLAYER_AVATAR_FOOT_TO_ORIGIN * PLAYER_AVATAR_SCALE + bob,
        z,
    )
}

#[derive(Component)]
pub struct MonsterCube {
    pub base: Vec3,
}

#[allow(dead_code)]
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EcoVisual {
    Cloud { id: u32 },
    Rain { id: u32, index: u32 },
    Rabbit { id: u32 },
    Wildlife { id: u32 },
    BerryBush { id: u32 },
    BerryFruit { id: u32, index: u32 },
    PlantNode { id: u32 },
    Co2Bubble { index: u32 },
}

#[allow(dead_code)]
pub fn eco_fruit_marker_count(fruit: u32) -> u32 {
    fruit.clamp(1, 3)
}

#[allow(dead_code)]
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

#[derive(Component, Clone)]
pub struct CloudPuff {
    pub base: Vec3,

    pub phase: f32,
}

impl Default for CloudPuff {
    fn default() -> Self {
        Self { base: Vec3::ZERO, phase: 0.0 }
    }
}

fn cloud_puff_translation(base: Vec3, phase: f32, elapsed_secs: f32) -> Vec3 {
    let drift_x = (elapsed_secs * 0.22 + phase).sin() * 0.7;
    let drift_z = (elapsed_secs * 0.16 + phase * 0.7).cos() * 0.5;
    let bob_y = (elapsed_secs * 0.6 + phase).sin() * 0.22;
    base + Vec3::new(drift_x, bob_y, drift_z)
}

#[allow(unreachable_code, unused_variables)]
pub fn spawn_pretty(
    mut commands: Commands,
    game_world: Res<GameWorld>,
    player: Res<PlayerState>,
    cfg: Res<PrettyConfig>,
    asset_server: Res<AssetServer>,
    camera_mode: Res<CameraMode>,
    camera_angles: Res<CameraAngles>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // First-person hides only camera-obstructing presentation actors. Low
    // ground props stay visible so the map does not look empty.
    let first_person_mode = *camera_mode == CameraMode::FirstPerson;

    if stable_scene_baseline_enabled() {
        info!("stable-scene: skipped pretty startup decorations");
        return;
    }

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

    let auto_demo_mode = std::env::args().any(|a| a == "--auto-demo");
    let world_center = lk2_core::constant::WORLD_SIZE as f32 * 0.5 + 0.5;
    let world_anchor = Vec3::new(world_center, player.pos.y, world_center);

    spawn_fresh_grass_scene(
        &mut commands,
        &game_world,
        player.pos,
        &asset_server,
        camera_angles.yaw,
        first_person_mode,
        &mut meshes,
        &mut materials,
    );
    // The old dense village/ecology presentation is intentionally bypassed:
    // this startup scene is authored through Bevy 0.19 BSN and kept minimal.
    return;

    if cfg.show_water {
        let s = 14.0_f32;
        let water_y = lk2_core::constant::WATER_Y - 1.5;
        let cx = world_anchor.x;
        let cz = world_anchor.z;
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
        ));
        info!(
            "🌊 水面已 spawn (y={}, size={}, 跟随玩家 @ ({:.1}, {:.1}))",
            water_y, s, cx, cz
        );
    }

    spawn_ground_detail_layer(
        &mut commands,
        &game_world,
        world_anchor,
        &mut meshes,
        &mut materials,
    );

    spawn_playable_village_diorama(
        &mut commands,
        &game_world,
        world_anchor,
        &asset_server,
        &mut meshes,
        &mut materials,
    );

    if cfg.show_player_avatar && !first_person_mode {
        let ground_top =
            effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
        let avatar_translation =
            player_avatar_translation(player.pos.x, ground_top, player.pos.z, 0.0);
        commands.spawn((
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("procedural/pretty/sokpop_gatherer.glb"),
            )),
            Transform::from_translation(avatar_translation)
                .with_rotation(Quat::from_rotation_y(camera_angles.yaw))
                .with_scale(Vec3::splat(PLAYER_AVATAR_SCALE)),
            PlayerAvatarModel,
        ));
        spawn_player_readability_marker(
            &mut commands,
            &mut meshes,
            &mut materials,
            avatar_translation,
            camera_angles.yaw,
        );
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

    let show_clouds = first_person_mode || !auto_demo_mode;
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
        if !show_clouds {
            continue;
        }
        let cx = world_anchor.x + angle.cos() * r;
        let cz = world_anchor.z + angle.sin() * r;
        let cy = base_y;
        let base = Vec3::new(cx, cy, cz);
        let phase = angle * 1.3;

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
            Transform::from_translation(cloud_puff_translation(base, phase, 0.0)),
            CloudPuff { base, phase },
        ));

        let base = Vec3::new(cx - 1.5 * ex, cy + 0.2, cz);
        let phase = angle * 1.3 + 1.7;
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.1))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(cloud_puff_translation(base, phase, 0.0)),
            CloudPuff { base, phase },
        ));

        let base = Vec3::new(cx + 1.6 * ex, cy - 0.1, cz + 0.5 * ez);
        let phase = angle * 1.3 + 3.1;
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.2))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(cloud_puff_translation(base, phase, 0.0)),
            CloudPuff { base, phase },
        ));

        let base = Vec3::new(cx + 0.3, cy + 1.0, cz - 0.2 * ez);
        let phase = angle * 1.3 + 4.5;
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.9))),
            MeshMaterial3d(cloud_mat),
            Transform::from_translation(cloud_puff_translation(base, phase, 0.0)),
            CloudPuff { base, phase },
        ));
    }

    let ground_y = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);

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

    // is ~15 — that mismatch was the root cause of "kenney landmark Y=34.65, 19m
    // above player's head, first-person cannot see").
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

fn spawn_player_readability_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    avatar_translation: Vec3,
    yaw: f32,
) {
    let marker_y = avatar_translation.y + PLAYER_MARKER_Y_OFFSET;
    let marker_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.28, 0.95),
        emissive: Color::srgb(0.08, 0.16, 0.55).into(),
        perceptual_roughness: 0.65,
        metallic: 0.0,
        ..default()
    });
    let arrow_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.92, 0.22),
        emissive: Color::srgb(0.55, 0.38, 0.04).into(),
        perceptual_roughness: 0.55,
        metallic: 0.0,
        ..default()
    });
    let ring_mesh = meshes.add(Torus {
        major_radius: PLAYER_MARKER_RING_RADIUS,
        minor_radius: PLAYER_MARKER_RING_THICKNESS,
    });
    commands.spawn_scene(bsn! {
        #FreshScenePlayerRing
        Mesh3d({ring_mesh})
        MeshMaterial3d::<StandardMaterial>({marker_mat})
        Transform {
            translation: Vec3::new(avatar_translation.x, marker_y, avatar_translation.z),
        }
        PlayerReadabilityMarker {
            part: PlayerReadabilityMarkerPart::Ring,
        }
        Visibility::Visible
    });
    let arrow_mesh = meshes.add(Cuboid::new(
        PLAYER_MARKER_ARROW_WIDTH,
        PLAYER_MARKER_RING_THICKNESS * 1.8,
        PLAYER_MARKER_ARROW_LENGTH,
    ));
    let forward = Quat::from_rotation_y(yaw).mul_vec3(Vec3::Z);
    let arrow_translation = Vec3::new(avatar_translation.x, marker_y + 0.03, avatar_translation.z)
        + forward * (PLAYER_MARKER_RING_RADIUS + PLAYER_MARKER_ARROW_LENGTH * 0.42);
    commands.spawn_scene(bsn! {
        #FreshScenePlayerArrow
        Mesh3d({arrow_mesh})
        MeshMaterial3d::<StandardMaterial>({arrow_mat})
        Transform {
            translation: arrow_translation,
            rotation: Quat::from_rotation_y(yaw),
        }
        PlayerReadabilityMarker {
            part: PlayerReadabilityMarkerPart::Arrow,
        }
        Visibility::Visible
    });
}

fn local_offset_from_yaw(offset: Vec3, yaw: f32) -> Vec3 {
    let forward = Vec3::new(yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, yaw.sin());
    right * offset.x + Vec3::Y * offset.y + forward * offset.z
}

fn supports_grass_detail(game_world: &GameWorld, x: i32, z: i32, ground_y: f32) -> bool {
    let surface_y = ground_y.floor() as i32 - 1;
    matches!(
        game_world.get(x, surface_y, z),
        BlockType::Grass | BlockType::Dirt | BlockType::Leaves | BlockType::BerryThicket
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_fresh_grass_scene(
    commands: &mut Commands,
    game_world: &GameWorld,
    player_pos: Vec3,
    asset_server: &Res<AssetServer>,
    yaw: f32,
    first_person_mode: bool,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let center = Vec3::new(player_pos.x, player_pos.y, player_pos.z);
    spawn_grass_meadow(commands, game_world, center, yaw, meshes, materials);

    if !first_person_mode {
        let ground_top = effective_ground_height(
            game_world,
            player_pos.x.floor() as i32,
            player_pos.z.floor() as i32,
        );
        let avatar_translation =
            player_avatar_translation(player_pos.x, ground_top, player_pos.z, 0.0);
        let gatherer_scene = asset_server
            .load(GltfAssetLabel::Scene(0).from_asset("procedural/pretty/sokpop_gatherer.glb"));
        commands.queue_spawn_scene(bsn! {
            #FreshScenePlayer
            Name("fresh_scene_player")
            WorldAssetRoot({gatherer_scene})
            Transform {
                translation: avatar_translation,
                rotation: Quat::from_rotation_y(yaw),
                scale: Vec3::splat(PLAYER_AVATAR_SCALE),
            }
            PlayerAvatarModel
        });
        spawn_player_readability_marker(commands, meshes, materials, avatar_translation, yaw);
    }

    spawn_simple_tree(
        commands,
        game_world,
        center,
        local_offset_from_yaw(Vec3::new(-8.8, 0.0, 8.2), yaw),
        1.0,
        true,
        yaw,
        meshes,
        materials,
    );
    spawn_simple_tree(
        commands,
        game_world,
        center,
        local_offset_from_yaw(Vec3::new(9.4, 0.0, 9.8), yaw),
        0.9,
        false,
        yaw,
        meshes,
        materials,
    );
    spawn_berry_bush(
        commands,
        game_world,
        center,
        local_offset_from_yaw(Vec3::new(3.6, 0.0, 4.8), yaw),
        meshes,
        materials,
    );
    spawn_scene_rabbit(
        commands,
        game_world,
        center,
        local_offset_from_yaw(Vec3::new(-3.4, 0.0, 4.4), yaw),
        yaw,
        asset_server,
    );
    spawn_scene_cloud(
        commands,
        center + local_offset_from_yaw(Vec3::new(0.0, 0.0, 14.0), yaw) + Vec3::Y * 11.5,
        meshes,
        materials,
    );
}

fn spawn_grass_meadow(
    commands: &mut Commands,
    game_world: &GameWorld,
    center: Vec3,
    yaw: f32,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let grass_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.55, 0.20),
        emissive: Color::srgb(0.020, 0.055, 0.014).into(),
        perceptual_roughness: 0.98,
        metallic: 0.0,
        ..default()
    });
    let flower_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.78, 0.24),
        emissive: Color::srgb(0.13, 0.08, 0.015).into(),
        perceptual_roughness: 0.82,
        metallic: 0.0,
        ..default()
    });
    let blade_mesh = meshes.add(Cuboid::new(0.12, 0.42, 0.10));

    for i in 0..58 {
        let angle = i as f32 * 2.3999631;
        let radius = 1.8 + (i % 13) as f32 * 0.86;
        let local = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius * 0.78);
        let offset = local_offset_from_yaw(local, yaw);
        let x = center.x + offset.x;
        let z = center.z + offset.z;
        let bx = x.floor() as i32;
        let bz = z.floor() as i32;
        let ground_y = effective_ground_height(game_world, bx, bz);
        if !supports_grass_detail(game_world, bx, bz, ground_y) {
            continue;
        }
        let mat = if i % 11 == 0 {
            flower_mat.clone()
        } else {
            grass_dark.clone()
        };
        let height = if i % 11 == 0 {
            0.30
        } else {
            0.28 + (i % 5) as f32 * 0.045
        };
        commands.spawn_scene(bsn! {
            #FreshSceneGrassBlade
            Name("fresh_scene_grass_blade")
            Mesh3d({blade_mesh.clone()})
            MeshMaterial3d::<StandardMaterial>({mat})
            Transform {
                translation: Vec3::new(x, ground_y + height * 0.5 + 0.04, z),
                rotation: Quat::from_rotation_y(yaw + angle),
                scale: Vec3::new(0.75, height / 0.42, 0.75),
            }
        });
    }
}

fn spawn_simple_tree(
    commands: &mut Commands,
    game_world: &GameWorld,
    center: Vec3,
    offset: Vec3,
    scale: f32,
    drops_branches: bool,
    yaw: f32,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let trunk = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.29, 0.12),
        perceptual_roughness: 0.86,
        metallic: 0.0,
        ..default()
    });
    let leaf = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.60, 0.22),
        emissive: Color::srgb(0.020, 0.070, 0.016).into(),
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    let trunk_mesh = meshes.add(Cuboid::new(0.72, 2.6, 0.72));
    let leaf_mesh = meshes.add(Sphere::new(1.35));
    let x = center.x + offset.x;
    let z = center.z + offset.z;
    let ground_y = effective_ground_height(game_world, x.floor() as i32, z.floor() as i32);

    commands.spawn_scene(bsn! {
        #FreshSceneTreeTrunk
        Name("fresh_scene_tree_trunk")
        Mesh3d({trunk_mesh})
        MeshMaterial3d::<StandardMaterial>({trunk})
        Transform {
            translation: Vec3::new(x, ground_y + 1.30 * scale, z),
            scale: Vec3::new(scale, scale, scale),
        }
    });
    for (lx, ly, lz, s) in [
        (0.0, 2.85, 0.0, 1.15),
        (-0.72, 2.55, 0.10, 0.82),
        (0.72, 2.48, -0.05, 0.78),
        (0.08, 3.35, 0.10, 0.72),
    ] {
        commands.spawn_scene(bsn! {
            #FreshSceneTreeLeaf
            Name("fresh_scene_tree_leaf")
            Mesh3d({leaf_mesh.clone()})
            MeshMaterial3d::<StandardMaterial>({leaf.clone()})
            Transform {
                translation: Vec3::new(x + lx * scale, ground_y + ly * scale, z + lz * scale),
                scale: Vec3::splat(s * scale),
            }
        });
    }
    if drops_branches {
        spawn_fallen_branches(
            commands,
            game_world,
            Vec3::new(x, ground_y, z),
            yaw,
            scale,
            meshes,
            materials,
        );
    }
}

fn spawn_fallen_branches(
    commands: &mut Commands,
    game_world: &GameWorld,
    tree_base: Vec3,
    yaw: f32,
    tree_scale: f32,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let branch_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.44, 0.27, 0.12),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    let branch_mesh = meshes.add(Cuboid::new(1.0, 0.12, 0.16));
    for (local_x, local_z, length, angle) in [
        (-1.10, 0.92, 1.65, 0.35),
        (0.88, -0.76, 1.18, -0.55),
        (1.38, 0.62, 0.82, 0.95),
    ] {
        let offset = local_offset_from_yaw(
            Vec3::new(local_x * tree_scale, 0.0, local_z * tree_scale),
            yaw,
        );
        let x = tree_base.x + offset.x;
        let z = tree_base.z + offset.z;
        let ground_y = effective_ground_height(game_world, x.floor() as i32, z.floor() as i32);
        commands.spawn_scene(bsn! {
            #FreshSceneFallenBranch
            Name("fresh_scene_fallen_branch")
            Mesh3d({branch_mesh.clone()})
            MeshMaterial3d::<StandardMaterial>({branch_mat.clone()})
            Transform {
                translation: Vec3::new(x, ground_y + 0.08, z),
                rotation: Quat::from_rotation_y(yaw + angle),
                scale: Vec3::new(length * tree_scale, 1.0, 1.0),
            }
        });
    }
}

fn spawn_berry_bush(
    commands: &mut Commands,
    game_world: &GameWorld,
    center: Vec3,
    offset: Vec3,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let bush_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.50, 0.20),
        emissive: Color::srgb(0.018, 0.060, 0.015).into(),
        perceptual_roughness: 0.94,
        metallic: 0.0,
        ..default()
    });
    let berry_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.08, 0.18),
        emissive: Color::srgb(0.16, 0.015, 0.025).into(),
        perceptual_roughness: 0.62,
        metallic: 0.0,
        ..default()
    });
    let bush_mesh = meshes.add(Sphere::new(0.68));
    let berry_mesh = meshes.add(Sphere::new(0.13));
    let x = center.x + offset.x;
    let z = center.z + offset.z;
    let ground_y = effective_ground_height(game_world, x.floor() as i32, z.floor() as i32);

    for (bx, bz, s) in [(0.0, 0.0, 1.0), (-0.38, 0.12, 0.72), (0.42, -0.10, 0.68)] {
        commands.spawn_scene(bsn! {
            #FreshSceneBerryBush
            Name("fresh_scene_berry_bush")
            Mesh3d({bush_mesh.clone()})
            MeshMaterial3d::<StandardMaterial>({bush_mat.clone()})
            Transform {
                translation: Vec3::new(x + bx, ground_y + 0.52 * s, z + bz),
                scale: Vec3::new(1.0 * s, 0.72 * s, 1.0 * s),
            }
        });
    }
    for (bx, by, bz) in [
        (-0.32, 0.78, 0.18),
        (0.22, 0.88, -0.28),
        (0.48, 0.62, 0.16),
        (-0.06, 1.02, 0.34),
        (-0.50, 0.58, -0.20),
    ] {
        commands.spawn_scene(bsn! {
            #FreshSceneBerry
            Name("fresh_scene_berry")
            Mesh3d({berry_mesh.clone()})
            MeshMaterial3d::<StandardMaterial>({berry_mat.clone()})
            Transform {
                translation: Vec3::new(x + bx, ground_y + by, z + bz),
            }
        });
    }
}

fn spawn_scene_rabbit(
    commands: &mut Commands,
    game_world: &GameWorld,
    center: Vec3,
    offset: Vec3,
    yaw: f32,
    asset_server: &Res<AssetServer>,
) {
    let x = center.x + offset.x;
    let z = center.z + offset.z;
    let ground_y = effective_ground_height(game_world, x.floor() as i32, z.floor() as i32);
    let rabbit_entry = ecology_entry(EcologyKind::Wildlife(WildlifeKind::Rabbit));
    let rabbit_scale = rabbit_entry.visual_scale;
    let rabbit_scene =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(rabbit_entry.model_path));
    commands.queue_spawn_scene(bsn! {
        #FreshSceneRabbit
        Name("fresh_scene_rabbit")
        WorldAssetRoot({rabbit_scene})
        Transform {
            translation: Vec3::new(x, ground_y + 0.03, z),
            rotation: Quat::from_rotation_y(yaw - 0.45),
            scale: rabbit_scale,
        }
    });
}

fn spawn_scene_cloud(
    commands: &mut Commands,
    base: Vec3,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let cloud_color = Color::srgb(0.94, 0.97, 1.0);
    let cloud_mat = materials.add(StandardMaterial {
        base_color: cloud_color,
        emissive: (cloud_color.to_linear() * 0.20).into(),
        perceptual_roughness: 0.96,
        metallic: 0.0,
        ..default()
    });
    let puff_mesh = meshes.add(Sphere::new(1.0));
    for (i, (x, y, z, sx, sy, sz)) in [
        (0.0, 0.0, 0.0, 1.65, 0.58, 0.92),
        (-1.15, -0.05, 0.05, 1.05, 0.50, 0.74),
        (1.20, -0.08, -0.08, 1.20, 0.48, 0.80),
        (0.20, 0.52, 0.08, 0.88, 0.58, 0.66),
    ]
    .into_iter()
    .enumerate()
    {
        let puff_base = base + Vec3::new(x, y, z);
        let phase = i as f32 * 1.1;
        commands.spawn_scene(bsn! {
            #FreshSceneCloud
            Name("fresh_scene_cloud")
            Mesh3d({puff_mesh.clone()})
            MeshMaterial3d::<StandardMaterial>({cloud_mat.clone()})
            Transform {
                translation: puff_base,
                scale: Vec3::new(sx, sy, sz),
            }
            CloudPuff {
                base: puff_base,
                phase,
            }
        });
    }
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
        base_color: Color::srgb(0.23, 0.46, 0.18),
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    let road = materials.add(StandardMaterial {
        base_color: Color::srgb(0.70, 0.61, 0.43),
        perceptual_roughness: 0.86,
        metallic: 0.0,
        ..default()
    });
    let field = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.64, 0.24),
        perceptual_roughness: 0.88,
        metallic: 0.0,
        ..default()
    });
    let soil = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.27, 0.16),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    let rock = materials.add(StandardMaterial {
        base_color: Color::srgb(0.52, 0.50, 0.43),
        perceptual_roughness: 0.84,
        metallic: 0.0,
        ..default()
    });
    let ruin_stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.61, 0.58, 0.49),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    let ruin_shadow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.37, 0.32),
        perceptual_roughness: 0.94,
        metallic: 0.0,
        ..default()
    });
    let moss = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.56, 0.20),
        emissive: Color::srgb(0.025, 0.075, 0.015).into(),
        perceptual_roughness: 0.96,
        metallic: 0.0,
        ..default()
    });
    let blossom = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.47, 0.42),
        emissive: Color::srgb(0.14, 0.035, 0.025).into(),
        perceptual_roughness: 0.82,
        metallic: 0.0,
        ..default()
    });

    spawn_living_ruins(
        commands,
        game_world,
        player_pos,
        meshes,
        &ruin_stone,
        &ruin_shadow,
        &moss,
        &blossom,
    );

    let road_mesh = meshes.add(Cuboid::new(1.0, 0.05, 1.0));
    for (ox, oz, sx, sz) in [
        (-22.0, -4.0, 1.45, 48.0),
        (-10.0, -4.0, 1.60, 52.0),
        (2.0, -4.0, 1.80, 54.0),
        (14.0, -4.0, 1.55, 50.0),
        (26.0, -4.0, 1.40, 44.0),
        (2.0, -24.0, 56.0, 1.55),
        (2.0, -12.0, 58.0, 1.70),
        (2.0, 0.0, 60.0, 1.85),
        (2.0, 12.0, 58.0, 1.70),
        (2.0, 24.0, 54.0, 1.55),
    ] {
        let pos = grounded_world_pos(game_world, player_pos, ox, oz, 0.03);
        commands.spawn((
            Mesh3d(road_mesh.clone()),
            MeshMaterial3d(road.clone()),
            Transform::from_translation(pos).with_scale(Vec3::new(sx, 1.0, sz)),
        ));
    }

    let patch_mesh = meshes.add(Cuboid::new(1.0, 0.03, 1.0));
    for i in 0..86 {
        let ox = -31.0 + pretty_hash01(i, 11) * 62.0;
        let oz = -30.0 + pretty_hash01(i, 17) * 58.0;
        if [-22.0, -10.0, 2.0, 14.0, 26.0].iter().any(|road_x| (ox - road_x).abs() < 1.8)
            || [-24.0, -12.0, 0.0, 12.0, 24.0].iter().any(|road_z| (oz - road_z).abs() < 1.8)
        {
            continue;
        }
        let pos = grounded_world_pos(game_world, player_pos, ox, oz, 0.04);
        commands.spawn((
            Mesh3d(patch_mesh.clone()),
            MeshMaterial3d(grass_dark.clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(
                    pretty_hash01(i, 23) * std::f32::consts::TAU,
                ))
                .with_scale(Vec3::new(
                    0.7 + pretty_hash01(i, 29) * 1.5,
                    1.0,
                    0.5 + pretty_hash01(i, 31) * 1.3,
                )),
        ));
    }

    let farm_mesh = meshes.add(Cuboid::new(1.0, 0.05, 1.0));
    let wheat_mesh = meshes.add(Cuboid::new(0.15, 0.58, 0.15));
    for (cx, cz, sx, sz) in [
        (-16.0, -18.0, 8.5, 7.5),
        (8.0, -18.0, 8.5, 7.5),
        (-16.0, 6.0, 8.5, 7.5),
        (8.0, 6.0, 8.5, 7.5),
        (20.0, 18.0, 7.0, 6.0),
    ] {
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
        ("procedural/pretty/well.glb", -4.0, -6.0, 0.38, 0.0),
        ("procedural/pretty/well.glb", 20.0, -6.0, 0.34, 0.4),
        ("procedural/pretty/market_stall.glb", -4.0, 18.0, 0.44, 0.7),
        ("procedural/pretty/market_stall.glb", 20.0, 6.0, 0.40, -0.5),
        ("procedural/pretty/house_small.glb", -16.0, -6.0, 0.54, 0.15),
        (
            "procedural/pretty/house_small.glb",
            -16.0,
            18.0,
            0.50,
            -0.45,
        ),
        ("procedural/pretty/house_small.glb", 20.0, -18.0, 0.52, 0.85),
        ("procedural/pretty/tavern.glb", 8.0, 18.0, 0.48, -0.25),
        ("procedural/pretty/barn.glb", -28.0, 6.0, 0.46, 0.45),
        ("procedural/pretty/chapel.glb", 8.0, -6.0, 0.46, -0.2),
        ("procedural/pretty/watchtower.glb", -28.0, -18.0, 0.46, 0.4),
        ("procedural/pretty/watchtower.glb", 32.0, 18.0, 0.42, -0.5),
        ("procedural/pretty/windmill.glb", 32.0, -6.0, 0.46, 0.1),
        ("procedural/pretty/forge.glb", -4.0, -18.0, 0.42, 0.7),
        ("procedural/pretty/fountain.glb", 8.0, 6.0, 0.34, 0.0),
        ("procedural/pretty/barrel.glb", -16.0, -6.0, 0.34, 0.4),
        ("procedural/pretty/barrel.glb", 8.0, 18.0, 0.32, -0.2),
        ("procedural/pretty/cart.glb", 20.0, 18.0, 0.34, 0.5),
        ("procedural/pretty/haystack.glb", -28.0, 18.0, 0.38, -0.2),
    ] {
        spawn_grounded_scene(
            commands,
            game_world,
            player_pos,
            asset_server,
            path,
            ox,
            oz,
            0.03,
            scale,
            yaw,
        );
    }

    for (ox, oz, scale) in [
        (-31.0, -29.0, 0.62),
        (-28.0, -16.0, 0.58),
        (-30.0, -3.0, 0.64),
        (-28.0, 10.0, 0.58),
        (-31.0, 25.0, 0.56),
        (-4.0, -30.0, 0.62),
        (24.0, -28.0, 0.58),
        (31.0, -14.0, 0.56),
        (30.0, 2.0, 0.58),
        (29.0, 16.0, 0.56),
        (26.0, 29.0, 0.56),
        (-8.0, 29.0, 0.58),
        (4.0, 13.5, 0.52),
    ] {
        spawn_grounded_scene(
            commands,
            game_world,
            player_pos,
            asset_server,
            ecology_entry(EcologyKind::Tree(TreeKind::Sokpop)).model_path,
            ox,
            oz,
            0.01,
            scale,
            0.0,
        );
    }

    for (i, (ox, oz, yaw)) in [
        (-7.5, -3.5, 0.1),
        (-5.2, 1.5, -0.2),
        (-2.8, 5.4, 0.4),
        (2.5, -2.8, -0.5),
        (4.6, 2.6, 0.25),
        (8.4, 5.6, -0.15),
        (-12.2, 4.8, 0.55),
        (12.4, -8.0, -0.35),
    ]
    .into_iter()
    .enumerate()
    {
        spawn_grounded_scene(
            commands,
            game_world,
            player_pos,
            asset_server,
            ecology_entry(EcologyKind::ResourceNode(ResourceNodeKind::BerryBush)).model_path,
            ox,
            oz,
            0.01,
            0.46,
            yaw,
        );
        for fruit_index in 0..3 {
            let angle = fruit_index as f32 * std::f32::consts::TAU / 3.0 + i as f32 * 0.43;
            spawn_grounded_scene(
                commands,
                game_world,
                player_pos,
                asset_server,
                ecology_entry(EcologyKind::ResourceDrop(ResourceDropKind::BerryFruit)).model_path,
                ox + angle.cos() * 0.24,
                oz + angle.sin() * 0.24,
                0.34,
                0.22,
                angle,
            );
        }
    }

    for (ox, oz, yaw) in [
        (-9.0, -0.5, 0.7),
        (-3.8, 3.6, -0.5),
        (3.4, 4.8, 1.1),
        (8.2, -2.2, -1.0),
        (13.5, 2.7, 0.3),
        (-15.4, -5.5, -0.8),
    ] {
        spawn_grounded_scene(
            commands,
            game_world,
            player_pos,
            asset_server,
            ecology_entry(EcologyKind::Wildlife(WildlifeKind::Rabbit)).model_path,
            ox,
            oz,
            0.02,
            0.42,
            yaw,
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
                .with_rotation(Quat::from_rotation_y(
                    pretty_hash01(i, 47) * std::f32::consts::TAU,
                ))
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

#[allow(clippy::too_many_arguments)]
fn spawn_living_ruins(
    commands: &mut Commands,
    game_world: &GameWorld,
    player_pos: Vec3,
    meshes: &mut ResMut<Assets<Mesh>>,
    ruin_stone: &Handle<StandardMaterial>,
    ruin_shadow: &Handle<StandardMaterial>,
    moss: &Handle<StandardMaterial>,
    blossom: &Handle<StandardMaterial>,
) {
    let block_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cap_mesh = meshes.add(Cuboid::new(1.0, 0.08, 1.0));
    let sprout_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    for (ox, oz, sx, sy, sz, yaw, broken) in [
        (-6.0, -1.6, 2.8, 0.95, 0.42, 0.10, false),
        (-3.0, -1.3, 1.8, 0.62, 0.40, -0.08, true),
        (1.0, -1.8, 2.4, 0.72, 0.42, 0.05, true),
        (4.3, -1.4, 1.6, 1.05, 0.44, -0.12, false),
        (-5.6, 4.2, 0.45, 1.30, 1.0, 0.18, false),
        (5.2, 3.8, 0.42, 1.05, 0.9, -0.22, true),
        (-0.8, 4.9, 0.50, 0.76, 1.1, 0.03, true),
    ] {
        let pos = grounded_world_pos(game_world, player_pos, ox, oz, sy * 0.5 + 0.04);
        commands.spawn((
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(if broken {
                ruin_shadow.clone()
            } else {
                ruin_stone.clone()
            }),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(yaw))
                .with_scale(Vec3::new(sx, sy, sz)),
        ));

        let moss_pos = grounded_world_pos(game_world, player_pos, ox, oz, sy + 0.10);
        commands.spawn((
            Mesh3d(cap_mesh.clone()),
            MeshMaterial3d(moss.clone()),
            Transform::from_translation(moss_pos)
                .with_rotation(Quat::from_rotation_y(yaw + 0.04))
                .with_scale(Vec3::new(sx * 0.82, 1.0, sz * 1.55)),
        ));
    }

    for (i, (ox, oz)) in [
        (-7.2, -0.1),
        (-4.6, 0.8),
        (-1.8, -0.6),
        (2.2, 0.6),
        (5.8, 0.0),
        (-5.4, 3.0),
        (-2.2, 3.7),
        (2.8, 3.2),
        (5.9, 2.4),
        (0.2, 5.8),
    ]
    .into_iter()
    .enumerate()
    {
        let is_blossom = i % 3 == 0;
        let height = if is_blossom { 0.34 } else { 0.42 };
        let pos = grounded_world_pos(game_world, player_pos, ox, oz, height * 0.5 + 0.06);
        commands.spawn((
            Mesh3d(sprout_mesh.clone()),
            MeshMaterial3d(if is_blossom {
                blossom.clone()
            } else {
                moss.clone()
            }),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(i as f32 * 0.73))
                .with_scale(if is_blossom {
                    Vec3::new(0.16, height, 0.16)
                } else {
                    Vec3::new(0.18, height, 0.14)
                }),
        ));
    }
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
    let mut x =
        (i as u32).wrapping_mul(1_664_525).wrapping_add((salt as u32).wrapping_mul(1_013_904_223));
    x ^= x >> 16;
    x = x.wrapping_mul(2_246_822_519);
    ((x >> 8) as f32) / ((u32::MAX >> 8) as f32)
}

fn spawn_ground_detail_layer(
    commands: &mut Commands,
    game_world: &GameWorld,
    player_pos: Vec3,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let patch_mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    let pebble_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let tuft_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    let patch_mats: Vec<Handle<StandardMaterial>> = [
        Color::srgba(0.26, 0.46, 0.20, 0.56),
        Color::srgba(0.52, 0.54, 0.28, 0.44),
        Color::srgba(0.32, 0.40, 0.24, 0.48),
        Color::srgba(0.55, 0.42, 0.24, 0.38),
        Color::srgba(0.42, 0.48, 0.38, 0.42),
    ]
    .into_iter()
    .map(|color| {
        materials.add(StandardMaterial {
            base_color: color,
            emissive: (color.to_linear() * 0.06).into(),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.98,
            metallic: 0.0,
            ..default()
        })
    })
    .collect();

    let pebble_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.54, 0.53, 0.45),
        emissive: Color::srgb(0.035, 0.035, 0.030).into(),
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    let tuft_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.19, 0.43, 0.16),
        emissive: Color::srgb(0.025, 0.065, 0.020).into(),
        perceptual_roughness: 0.96,
        metallic: 0.0,
        ..default()
    });
    let flower_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.94, 0.67, 0.34),
        emissive: Color::srgb(0.12, 0.07, 0.03).into(),
        perceptual_roughness: 0.86,
        metallic: 0.0,
        ..default()
    });

    for i in 0..42 {
        let radius = 5.0 + pretty_hash01(i, 1) * 34.0;
        let angle = i as f32 * 2.3999631 + pretty_hash01(i, 2) * 0.55;
        let offset = Vec2::new(angle.cos() * radius, angle.sin() * radius * 0.72);
        let x = player_pos.x + offset.x;
        let z = player_pos.z + offset.y;
        let ground_y = effective_ground_height(game_world, x.floor() as i32, z.floor() as i32);
        let sx = 2.8 + pretty_hash01(i, 3) * 5.6;
        let sz = 1.4 + pretty_hash01(i, 4) * 3.6;
        commands.spawn((
            Mesh3d(patch_mesh.clone()),
            MeshMaterial3d(patch_mats[i % patch_mats.len()].clone()),
            Transform::from_translation(Vec3::new(x, ground_y + 0.052, z))
                .with_rotation(Quat::from_rotation_y(angle * 0.37))
                .with_scale(Vec3::new(sx, 1.0, sz)),
        ));
    }

    for i in 0..46 {
        let radius = 4.0 + pretty_hash01(i, 11) * 32.0;
        let angle = i as f32 * 1.671 + pretty_hash01(i, 12);
        let offset = Vec2::new(angle.cos() * radius, angle.sin() * radius * 0.70);
        let x = player_pos.x + offset.x;
        let z = player_pos.z + offset.y;
        let ground_y = effective_ground_height(game_world, x.floor() as i32, z.floor() as i32);
        let is_flower = i % 9 == 0;
        let scale = if is_flower {
            Vec3::new(0.10, 0.24, 0.10)
        } else if i % 3 == 0 {
            Vec3::new(0.22, 0.10, 0.18)
        } else {
            Vec3::new(0.10, 0.24 + pretty_hash01(i, 13) * 0.18, 0.10)
        };
        let y_offset = if is_flower { 0.16 } else { scale.y * 0.5 };
        let mesh = if i % 3 == 0 {
            pebble_mesh.clone()
        } else {
            tuft_mesh.clone()
        };
        let material = if is_flower {
            flower_mat.clone()
        } else if i % 3 == 0 {
            pebble_mat.clone()
        } else {
            tuft_mat.clone()
        };
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(Vec3::new(x, ground_y + y_offset, z))
                .with_rotation(Quat::from_rotation_y(angle))
                .with_scale(scale),
        ));
    }
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

#[allow(dead_code)]
pub fn spawn_eco_visuals(
    mut commands: Commands,
    eco: Res<EcoCycle>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let bubble_mesh = meshes.add(Sphere::new(0.5));
    let cloud_mesh = meshes.add(Sphere::new(1.35));
    let rain_mesh = meshes.add(Cuboid::new(0.035, 0.72, 0.035));
    let bubble_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.45, 0.90, 1.0, 0.62),
        emissive: Color::srgb(0.25, 0.75, 1.0).into(),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.18,
        metallic: 0.0,
        ..default()
    });
    let cloud_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.86, 0.90, 0.92, 0.78),
        emissive: Color::srgb(0.30, 0.34, 0.36).into(),
        unlit: false,
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });
    let rain_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.34, 0.58, 0.95, 0.72),
        emissive: Color::srgb(0.14, 0.30, 0.70).into(),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.25,
        metallic: 0.0,
        ..default()
    });

    let rabbit_entry = ecology_entry(EcologyKind::Wildlife(WildlifeKind::Rabbit));
    let bush_entry = ecology_entry(EcologyKind::ResourceNode(ResourceNodeKind::BerryBush));
    let fruit_entry = ecology_entry(EcologyKind::ResourceDrop(ResourceDropKind::BerryFruit));

    for cloud in &eco.clouds {
        spawn_eco_visual_part(
            &mut commands,
            cloud_mesh.clone(),
            cloud_mat.clone(),
            EcoVisual::Cloud { id: cloud.id },
            Vec3::ZERO,
            Vec3::splat(1.0),
        );
        for index in 0..4 {
            spawn_eco_visual_part(
                &mut commands,
                rain_mesh.clone(),
                rain_mat.clone(),
                EcoVisual::Rain { id: cloud.id, index },
                Vec3::ZERO,
                Vec3::splat(1.0),
            );
        }
    }

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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn update_eco_visuals(
    time: Res<Time>,
    eco: Res<EcoCycle>,
    game_world: Res<GameWorld>,
    mut q: Query<(&EcoVisual, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (visual, mut transform) in &mut q {
        match *visual {
            EcoVisual::Cloud { id } => {
                let Some(cloud) = eco.clouds.iter().find(|cloud| cloud.id == id) else {
                    transform.scale = Vec3::ZERO;
                    continue;
                };
                let phase = t * 0.45 + cloud.phase;
                transform.translation = Vec3::new(
                    cloud.pos.x + phase.sin() * 0.45,
                    8.5 + cloud.rain * 2.2 + phase.cos() * 0.18,
                    cloud.pos.y + phase.cos() * 0.35,
                );
                let scale = 1.1 + cloud.rain.clamp(0.0, 1.0) * 0.45;
                transform.scale = Vec3::new(scale * 1.65, scale * 0.52, scale);
            }
            EcoVisual::Rain { id, index } => {
                let Some(cloud) = eco.clouds.iter().find(|cloud| cloud.id == id) else {
                    transform.scale = Vec3::ZERO;
                    continue;
                };
                if cloud.rain <= 0.05 {
                    transform.scale = Vec3::ZERO;
                    continue;
                }
                let phase = t * 2.8 + cloud.phase + index as f32 * 0.73;
                let spread = 0.55 + index as f32 * 0.18;
                transform.translation = Vec3::new(
                    cloud.pos.x + phase.sin() * spread,
                    7.8 - (phase.fract() * 2.4),
                    cloud.pos.y + phase.cos() * spread * 0.7,
                );
                transform.scale = Vec3::new(0.06, 0.52 + cloud.rain * 0.25, 0.06);
                transform.rotation = Quat::from_rotation_z(0.16);
            }
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
                transform.translation = Vec3::new(x, ground_y + 0.025 + hop + breath, z);
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
                transform.translation = Vec3::new(x, ground_y + bob.max(0.0), z);
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
                transform.translation = Vec3::new(x, ground_y + sway.max(0.0), z);
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
                transform.translation = Vec3::new(x, ground_y + sway.max(0.0), z);
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

#[allow(dead_code)]
pub fn sync_eco_visual_spawns(
    mut commands: Commands,
    eco: Res<EcoCycle>,
    existing: Query<&EcoVisual>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let existing = existing.iter().copied().collect::<HashSet<_>>();
    let rabbit_entry = ecology_entry(EcologyKind::Wildlife(WildlifeKind::Rabbit));
    let bush_entry = ecology_entry(EcologyKind::ResourceNode(ResourceNodeKind::BerryBush));
    let fruit_entry = ecology_entry(EcologyKind::ResourceDrop(ResourceDropKind::BerryFruit));

    for cloud in &eco.clouds {
        let visual = EcoVisual::Cloud { id: cloud.id };
        if !existing.contains(&visual) {
            let cloud_mesh = meshes.add(Sphere::new(1.35));
            let cloud_mat = materials.add(StandardMaterial {
                base_color: Color::srgba(0.86, 0.90, 0.92, 0.78),
                emissive: Color::srgb(0.30, 0.34, 0.36).into(),
                unlit: false,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.95,
                metallic: 0.0,
                ..default()
            });
            spawn_eco_visual_part(
                &mut commands,
                cloud_mesh,
                cloud_mat,
                visual,
                Vec3::ZERO,
                Vec3::splat(1.0),
            );
        }
        for index in 0..4 {
            let visual = EcoVisual::Rain { id: cloud.id, index };
            if existing.contains(&visual) {
                continue;
            }
            let rain_mesh = meshes.add(Cuboid::new(0.035, 0.72, 0.035));
            let rain_mat = materials.add(StandardMaterial {
                base_color: Color::srgba(0.34, 0.58, 0.95, 0.72),
                emissive: Color::srgb(0.14, 0.30, 0.70).into(),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.25,
                metallic: 0.0,
                ..default()
            });
            spawn_eco_visual_part(
                &mut commands,
                rain_mesh,
                rain_mat,
                visual,
                Vec3::ZERO,
                Vec3::splat(1.0),
            );
        }
    }

    for rabbit in &eco.rabbits {
        let visual = EcoVisual::Rabbit { id: rabbit.id };
        if !existing.contains(&visual) {
            spawn_eco_scene_visual(
                &mut commands,
                &asset_server,
                rabbit_entry.model_path,
                visual,
                Vec3::new(rabbit.pos.x, 0.0, rabbit.pos.y),
                rabbit_entry.visual_scale,
            );
        }
    }

    for animal in &eco.wildlife {
        let visual = EcoVisual::Wildlife { id: animal.id };
        if !existing.contains(&visual) {
            let entry = ecology_entry(EcologyKind::Wildlife(animal.kind));
            spawn_eco_scene_visual(
                &mut commands,
                &asset_server,
                entry.model_path,
                visual,
                Vec3::new(animal.pos.x, 0.0, animal.pos.y),
                entry.visual_scale,
            );
        }
    }

    for berry in &eco.berries {
        let visual = EcoVisual::BerryBush { id: berry.id };
        if !existing.contains(&visual) {
            spawn_eco_scene_visual(
                &mut commands,
                &asset_server,
                bush_entry.model_path,
                visual,
                Vec3::new(berry.pos.x, 0.0, berry.pos.y),
                bush_entry.visual_scale,
            );
        }
        for index in 0..3 {
            let visual = EcoVisual::BerryFruit { id: berry.id, index };
            if existing.contains(&visual) {
                continue;
            }
            spawn_eco_scene_visual(
                &mut commands,
                &asset_server,
                fruit_entry.model_path,
                visual,
                Vec3::new(berry.pos.x, 0.0, berry.pos.y),
                fruit_entry.visual_scale,
            );
        }
    }

    for plant in &eco.plants {
        let visual = EcoVisual::PlantNode { id: plant.id };
        if !existing.contains(&visual) {
            let entry = ecology_entry(EcologyKind::ResourceNode(plant.kind));
            spawn_eco_scene_visual(
                &mut commands,
                &asset_server,
                entry.model_path,
                visual,
                Vec3::new(plant.pos.x, 0.0, plant.pos.y),
                entry.visual_scale,
            );
        }
    }
}

pub fn animate_avatar(
    mut model_q: Query<&mut Transform, With<PlayerAvatarModel>>,
    mut marker_q: Query<
        (&mut Transform, &mut Visibility, &PlayerReadabilityMarker),
        Without<PlayerAvatarModel>,
    >,
    player: Res<PlayerState>,
    game_world: Res<GameWorld>,
    state: Res<PlayerAnimState>,
    camera_mode: Res<CameraMode>,
) {
    let ground_top = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
    let base_x = player.pos.x;
    let base_z = player.pos.z;

    let phase = state.step_phase;
    let speed = state.smoothed_speed;

    let mut marker_yaw = 0.0;
    let mut saw_avatar = false;
    for mut transform in &mut model_q {
        saw_avatar = true;
        let bob = (phase * 0.5).sin() * 0.035 * speed.max(0.15);
        let yaw = if state.smoothed_move_world.length_squared() > 0.001 {
            state.smoothed_move_world.x.atan2(state.smoothed_move_world.y)
        } else {
            transform.rotation.to_euler(EulerRot::YXZ).0
        };
        marker_yaw = yaw;
        transform.translation = player_avatar_translation(base_x, ground_top, base_z, bob);
        transform.rotation = Quat::from_rotation_y(yaw);
        transform.scale = Vec3::splat(PLAYER_AVATAR_SCALE);
    }
    if !saw_avatar && *camera_mode != CameraMode::TopDown {
        for (_, mut visibility, _) in &mut marker_q {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    if *camera_mode != CameraMode::TopDown {
        for (_, mut visibility, _) in &mut marker_q {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let marker_y =
        ground_top + PLAYER_AVATAR_FOOT_TO_ORIGIN * PLAYER_AVATAR_SCALE + PLAYER_MARKER_Y_OFFSET;
    let forward = Quat::from_rotation_y(marker_yaw).mul_vec3(Vec3::Z);
    for (mut transform, mut visibility, marker) in &mut marker_q {
        *visibility = Visibility::Visible;
        match marker.part {
            PlayerReadabilityMarkerPart::Ring => {
                transform.translation = Vec3::new(base_x, marker_y, base_z);
                transform.rotation = Quat::IDENTITY;
            }
            PlayerReadabilityMarkerPart::Arrow => {
                transform.translation = Vec3::new(base_x, marker_y + 0.03, base_z)
                    + forward * (PLAYER_MARKER_RING_RADIUS + PLAYER_MARKER_ARROW_LENGTH * 0.42);
                transform.rotation = Quat::from_rotation_y(marker_yaw);
            }
        }
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
        tf.translation = cloud_puff_translation(puff.base, puff.phase, t);
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

    #[test]
    fn cloud_puff_translation_moves_horizontally() {
        let base = Vec3::new(10.0, 20.0, 30.0);
        let a = cloud_puff_translation(base, 0.7, 0.0);
        let b = cloud_puff_translation(base, 0.7, 12.0);

        assert!((a.x - b.x).abs() > 0.5);
        assert!((a.z - b.z).abs() > 0.3);
        assert!((a.y - base.y).abs() <= 0.23);
        assert!((b.y - base.y).abs() <= 0.23);
    }
}
