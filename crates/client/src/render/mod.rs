use bevy::camera::ScalingMode;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::{FogVolume, VolumetricFog, VolumetricLight};
use bevy::pbr::{
    AtmosphereSettings, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
    ScreenSpaceReflections,
};
use bevy::prelude::*;
use bevy::render::camera::{MipBias, TemporalJitter};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use avian3d::prelude::{Collider, RigidBody};

use crate::pretty::PlayerAnimState;
use crate::render::camera_math::compute_third_person_camera;
use crate::render::scalar_field::effective_ground_height;
use crate::ui::ClientRunMode;
use lk2_core::constant;
use lk2_core::creature::Creature;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::world::{
    Biome, BlockType, World as GameWorld, player_body_clear, player_position_is_safe,
    player_spawn_position_near, player_stand_position_at, resolve_player_stuck_near,
};

mod greedy_mesh;
use greedy_mesh::build_all_terrain_meshes_aabb;

pub mod camera_math;
mod marching_cubes;
pub mod scalar_field;
mod smooth_mesh;

pub fn stable_scene_baseline_enabled() -> bool {
    stable_scene_baseline_enabled_from(
        std::env::args().map(|arg| arg == "--stable-scene"),
        std::env::var_os("LK2_STABLE_SCENE").is_some(),
    )
}

fn stable_scene_baseline_enabled_from(
    mut arg_matches: impl Iterator<Item = bool>,
    env_enabled: bool,
) -> bool {
    env_enabled || arg_matches.any(|matched| matched)
}

#[derive(Resource, Debug, Clone)]
#[allow(dead_code)]
pub struct RenderConfig {
    pub radius: i32,
    pub max_blocks: usize,
    pub y_offset: f32,
    pub sky_color: Color,
    pub fog_color: Color,
    pub fog_start: f32,
    pub fog_end: f32,
    pub auto_orbit: bool,
    pub auto_orbit_speed: f32,
    pub auto_orbit_distance: f32,
    pub auto_walk: bool,
    pub auto_walk_interval_secs: f32,
    pub auto_keys: bool,
    pub mouse_look: bool,
    pub smooth_terrain: bool,
    pub smooth_passes: u32,
    pub ground_step_threshold: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            radius: 48,
            max_blocks: 5000,
            y_offset: 0.0,
            sky_color: Color::srgb(0.64, 0.78, 0.96),
            fog_color: Color::srgb(0.82, 0.90, 0.94),
            fog_start: 130.0,
            fog_end: 360.0,
            auto_orbit: false,

            auto_orbit_speed: 0.30,

            auto_orbit_distance: 22.0,
            auto_walk: false,
            auto_walk_interval_secs: 0.1,
            auto_keys: false,
            mouse_look: false,

            smooth_terrain: true,
            smooth_passes: 2,
            ground_step_threshold: 0.85,
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderFeatureSettings {
    pub atmosphere: bool,
    pub taa: bool,
    pub ssr: bool,
    pub ssao: bool,
    pub volumetric_fog: bool,
}

impl Default for RenderFeatureSettings {
    fn default() -> Self {
        Self { atmosphere: true, taa: false, ssr: false, ssao: false, volumetric_fog: false }
    }
}

#[derive(Resource)]
pub struct CameraAngles {
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for CameraAngles {
    fn default() -> Self {
        // iter_210: pitch=0 puts the first-person camera on a horizontal
        // gaze on entry, like a normal FPS game. Looking straight ahead
        // lets the player immediately see trees, animals, and the horizon,
        // instead of staring straight down at their own feet (legacy pitch
        // = -1.05 rad ≈ -60° down).
        Self { yaw: std::f32::consts::FRAC_PI_2, pitch: 0.0 }
    }
}

#[derive(Resource, PartialEq, Eq, Debug, Clone, Copy)]
pub enum CameraMode {
    FirstPerson,

    ThirdPerson,

    TopDown,
}

impl Default for CameraMode {
    fn default() -> Self {
        Self::ThirdPerson
    }
}

const MANUAL_MOVE_SPEED: f32 = 4.5;
const PLAYER_COLLISION_RADIUS: f32 = 0.34;
const JUMP_TAKEOFF_SPEED: f32 = 7.2;
const JUMP_GRAVITY: f32 = 28.0;
const JUMP_TERMINAL_SPEED: f32 = -18.0;

#[derive(Resource, Debug, Clone, Copy)]
pub struct JumpState {
    pub velocity_y: f32,
    pub grounded: bool,
}

impl Default for JumpState {
    fn default() -> Self {
        Self { velocity_y: 0.0, grounded: true }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct AntiStuckState {
    pub last_safe_pos: Vec3,
    pub last_safe_block: [i32; 3],
    pub unsafe_ticks: u8,
}

impl Default for AntiStuckState {
    fn default() -> Self {
        Self { last_safe_pos: Vec3::ZERO, last_safe_block: [0, 0, 0], unsafe_ticks: 0 }
    }
}

#[derive(Resource)]
pub struct FreeFlyState {
    pub enabled: bool,

    pub position: Vec3,

    pub velocity: Vec3,
}

impl Default for FreeFlyState {
    fn default() -> Self {
        Self { enabled: false, position: Vec3::new(48.5, 18.0, 48.5), velocity: Vec3::ZERO }
    }
}

const FREEFLY_SPEED: f32 = 30.0;

const FREEFLY_BOOST: f32 = 3.0;
const TOP_DOWN_ORTHO_HEIGHT: f32 = 72.0;
const TERRAIN_REBUILD_MIN_SECS: f32 = 4.0;
const TERRAIN_REBUILD_RADIUS_FRACTION: f32 = 0.60;
const SMOOTH_TERRAIN_CHUNK_SIZE: i32 = 16;
const SMOOTH_TERRAIN_CHUNKS_PER_FRAME: usize = 1;

const MOUSE_SENS: f32 = 0.0022;
const PITCH_LIMIT: f32 = 1.483;
const YAW_QE_STEP: f32 = 22.5_f32.to_radians();

#[derive(Resource, Default)]
pub struct SpawnedBlocks {
    pub visual_entities: Vec<Entity>,
    pub collider_entities: Vec<Entity>,
    pub generated_meshes: Vec<Handle<Mesh>>,
    pub generated_materials: Vec<Handle<StandardMaterial>>,
    pub smooth_chunks: HashMap<SmoothTerrainChunkKey, SmoothTerrainChunk>,
    pub smooth_empty_chunks: HashSet<SmoothTerrainChunkKey>,
    pub smooth_material: Option<Handle<StandardMaterial>>,

    pub last_player_block: [i32; 3],
    pub last_mesh_center: Option<Vec3>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SmoothTerrainChunkKey {
    pub x: i32,
    pub z: i32,
    pub y_min: i32,
    pub y_max: i32,
}

pub struct SmoothTerrainChunk {
    pub visual_entity: Entity,
    pub collider_entity: Entity,
    pub mesh_handle: Handle<Mesh>,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct RenderTelemetry {
    pub smooth_mesh_builds: u32,
    pub smooth_mesh_total_ms: f32,
    pub smooth_mesh_max_ms: f32,
    pub smooth_mesh_total_tris: u64,
    pub greedy_mesh_builds: u32,
    pub greedy_mesh_total_ms: f32,
    pub greedy_mesh_max_ms: f32,
    pub terrain_despawns: u32,
    pub collider_rebuilds: u32,
    pub frame_samples: u32,
    pub frame_dt_max_ms: f32,
    pub frame_dt_over_50ms: u32,
}

#[derive(Resource, Debug, Clone)]
pub struct RenderLightingTelemetry {
    pub time_of_day: f32,
    pub dayness: f32,
    pub atmosphere_time_of_day: f32,
    pub atmosphere_dayness: f32,
    pub readable_dayness: f32,
    pub sunset_glow: f32,
    pub sun_illuminance: f32,
    pub fill_illuminance: f32,
    pub camera_mode: &'static str,
}

impl Default for RenderLightingTelemetry {
    fn default() -> Self {
        Self {
            time_of_day: 0.42,
            dayness: 0.0,
            atmosphere_time_of_day: 0.42,
            atmosphere_dayness: 0.0,
            readable_dayness: 0.0,
            sunset_glow: 0.0,
            sun_illuminance: 0.0,
            fill_illuminance: 0.0,
            camera_mode: "Unknown",
        }
    }
}

pub fn record_render_frame_time(time: Res<Time>, mut telemetry: ResMut<RenderTelemetry>) {
    let dt_ms = time.delta_secs() * 1000.0;
    telemetry.frame_samples = telemetry.frame_samples.saturating_add(1);
    if telemetry.frame_samples < 120 {
        return;
    }
    telemetry.frame_dt_max_ms = telemetry.frame_dt_max_ms.max(dt_ms);
    if dt_ms > 50.0 {
        telemetry.frame_dt_over_50ms = telemetry.frame_dt_over_50ms.saturating_add(1);
    }
}

fn despawn_and_release_generated_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    telemetry: &mut RenderTelemetry,
    visual_entities: Vec<Entity>,
    collider_entities: Vec<Entity>,
    mesh_handles: Vec<Handle<Mesh>>,
    material_handles: Vec<Handle<StandardMaterial>>,
) {
    for e in visual_entities {
        commands.entity(e).despawn();
        telemetry.terrain_despawns = telemetry.terrain_despawns.saturating_add(1);
    }
    for e in collider_entities {
        commands.entity(e).despawn();
        telemetry.terrain_despawns = telemetry.terrain_despawns.saturating_add(1);
    }
    for handle in mesh_handles {
        meshes.remove(&handle);
    }
    for handle in material_handles {
        materials.remove(&handle);
    }
}

fn despawn_and_release_smooth_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    telemetry: &mut RenderTelemetry,
    chunk: SmoothTerrainChunk,
) {
    commands.entity(chunk.visual_entity).despawn();
    telemetry.terrain_despawns = telemetry.terrain_despawns.saturating_add(1);
    commands.entity(chunk.collider_entity).despawn();
    telemetry.terrain_despawns = telemetry.terrain_despawns.saturating_add(1);
    meshes.remove(&chunk.mesh_handle);
}

fn clear_smooth_terrain_chunks(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    telemetry: &mut RenderTelemetry,
    spawned: &mut SpawnedBlocks,
) {
    let chunks: Vec<SmoothTerrainChunk> =
        spawned.smooth_chunks.drain().map(|(_, chunk)| chunk).collect();
    for chunk in chunks {
        despawn_and_release_smooth_chunk(commands, meshes, telemetry, chunk);
    }
    spawned.smooth_empty_chunks.clear();
    if let Some(material) = spawned.smooth_material.take() {
        materials.remove(&material);
    }
}

fn ensure_smooth_terrain_material(
    materials: &mut Assets<StandardMaterial>,
    spawned: &mut SpawnedBlocks,
) -> Handle<StandardMaterial> {
    if let Some(material) = spawned.smooth_material.clone() {
        return material;
    }
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.56, 0.78, 0.34),
        emissive: Color::srgb(0.08, 0.13, 0.04).into(),
        unlit: true,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        cull_mode: Some(bevy::render::render_resource::Face::Back),
        double_sided: false,
        ..default()
    });
    spawned.smooth_material = Some(material.clone());
    material
}

fn smooth_chunk_keys_for_aabb(
    min: [i32; 3],
    max: [i32; 3],
    mesh_center: Vec3,
) -> Vec<SmoothTerrainChunkKey> {
    let x0 = min[0].div_euclid(SMOOTH_TERRAIN_CHUNK_SIZE);
    let x1 = (max[0] - 1).div_euclid(SMOOTH_TERRAIN_CHUNK_SIZE);
    let z0 = min[2].div_euclid(SMOOTH_TERRAIN_CHUNK_SIZE);
    let z1 = (max[2] - 1).div_euclid(SMOOTH_TERRAIN_CHUNK_SIZE);
    let y_min = min[1];
    let y_max = max[1];
    let mut keys = Vec::new();
    for z in z0..=z1 {
        for x in x0..=x1 {
            keys.push(SmoothTerrainChunkKey { x, z, y_min, y_max });
        }
    }
    keys.sort_by(|a, b| {
        let ac = Vec2::new(
            (a.x * SMOOTH_TERRAIN_CHUNK_SIZE) as f32 + SMOOTH_TERRAIN_CHUNK_SIZE as f32 * 0.5,
            (a.z * SMOOTH_TERRAIN_CHUNK_SIZE) as f32 + SMOOTH_TERRAIN_CHUNK_SIZE as f32 * 0.5,
        );
        let bc = Vec2::new(
            (b.x * SMOOTH_TERRAIN_CHUNK_SIZE) as f32 + SMOOTH_TERRAIN_CHUNK_SIZE as f32 * 0.5,
            (b.z * SMOOTH_TERRAIN_CHUNK_SIZE) as f32 + SMOOTH_TERRAIN_CHUNK_SIZE as f32 * 0.5,
        );
        let pc = Vec2::new(mesh_center.x, mesh_center.z);
        ac.distance_squared(pc).total_cmp(&bc.distance_squared(pc))
    });
    keys
}

#[derive(Component)]
#[allow(dead_code)]
pub struct PlayerCube;

pub fn spawn_terrain_around_player(
    mut commands: Commands,
    game_world: Res<GameWorld>,
    cfg: Res<RenderConfig>,
    player: Res<PlayerState>,
    mut spawned: ResMut<SpawnedBlocks>,
    mode: Res<CameraMode>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut telemetry: ResMut<RenderTelemetry>,

    _last_warn_time: Local<f32>,

    mut last_mesh_wall: Local<f32>,
) {
    let now = time.elapsed_secs();
    let show_terrain_visuals = true;
    let mesh_center = if *mode == CameraMode::TopDown {
        let center = constant::WORLD_SIZE as f32 * 0.5;
        Vec3::new(center, player.block_pos[1] as f32, center)
    } else {
        Vec3::new(
            player.block_pos[0] as f32,
            player.block_pos[1] as f32,
            player.block_pos[2] as f32,
        )
    };
    if !show_terrain_visuals {
        spawned.last_mesh_center = Some(mesh_center);
        return;
    }

    let invalidated = spawned.last_player_block == [i32::MIN; 3];
    let has_mesh_record = spawned.last_mesh_center.is_some();
    if *mode == CameraMode::FirstPerson && !cfg.smooth_terrain && has_mesh_record {
        return;
    }
    let moved_far = if let Some(last) = spawned.last_mesh_center {
        let dx = mesh_center.x - last.x;
        let dz = mesh_center.z - last.z;
        invalidated
            || Vec2::new(dx, dz).length()
                > (cfg.radius as f32 * TERRAIN_REBUILD_RADIUS_FRACTION).max(24.0)
    } else {
        true
    };
    if cfg.smooth_terrain {
        if moved_far && now - *last_mesh_wall < TERRAIN_REBUILD_MIN_SECS && has_mesh_record {
            return;
        }
    } else {
        if !moved_far && has_mesh_record {
            return;
        }
        if now - *last_mesh_wall < TERRAIN_REBUILD_MIN_SECS && has_mesh_record {
            return;
        }
    }

    let first_person_mode = *mode == CameraMode::FirstPerson;
    let r = if first_person_mode {
        cfg.radius.min(28)
    } else {
        cfg.radius
    } as i32;
    let py = player.block_pos[1];
    let y_span = if first_person_mode { 24 } else { 40 };
    let y_min = (py - y_span).max(0);
    let y_max = (py + y_span).min(game_world.size as i32 - 1);
    let center_x = mesh_center.x.round() as i32;
    let center_z = mesh_center.z.round() as i32;
    let min = [center_x - r, y_min, center_z - r];
    let max = [center_x + r, y_max, center_z + r];

    if cfg.smooth_terrain {
        if invalidated {
            clear_smooth_terrain_chunks(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut telemetry,
                &mut spawned,
            );
        }

        let desired_keys = smooth_chunk_keys_for_aabb(min, max, mesh_center);

        let material = ensure_smooth_terrain_material(&mut materials, &mut spawned);
        let mut built_this_frame = 0usize;
        let mut built_tris = 0usize;
        let mut built_ms = 0.0_f32;
        for key in desired_keys {
            if spawned.smooth_chunks.contains_key(&key)
                || spawned.smooth_empty_chunks.contains(&key)
            {
                continue;
            }
            if built_this_frame >= SMOOTH_TERRAIN_CHUNKS_PER_FRAME {
                break;
            }

            let chunk_min = [
                key.x * SMOOTH_TERRAIN_CHUNK_SIZE,
                key.y_min,
                key.z * SMOOTH_TERRAIN_CHUNK_SIZE,
            ];
            let chunk_max = [
                chunk_min[0] + SMOOTH_TERRAIN_CHUNK_SIZE,
                key.y_max,
                chunk_min[2] + SMOOTH_TERRAIN_CHUNK_SIZE,
            ];
            let started = Instant::now();
            let Some(sm) = smooth_mesh::build_smooth_mesh(
                &game_world,
                chunk_min,
                chunk_max,
                0.5,
                cfg.smooth_passes,
            ) else {
                spawned.smooth_empty_chunks.insert(key);
                continue;
            };
            let mesh_ms = started.elapsed().as_secs_f32() * 1000.0;
            let total_tris = sm.collider_indices.len() / 3;

            let mesh_handle = meshes.add(sm.mesh);
            let visual = commands
                .spawn((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                    TerrainChunk,
                ))
                .id();

            let collider_verts: Vec<Vec3> =
                sm.collider_trimesh.iter().map(|p| Vec3::new(p[0], p[1], p[2])).collect();
            let collider_indices: Vec<[u32; 3]> = sm
                .collider_indices
                .chunks(3)
                .filter(|c| c.len() == 3)
                .map(|c| [c[0], c[1], c[2]])
                .collect();
            let collider = Collider::trimesh(collider_verts, collider_indices);
            let collider_entity = commands
                .spawn((
                    RigidBody::Static,
                    collider,
                    Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                    TerrainChunk,
                ))
                .id();

            spawned.smooth_chunks.insert(
                key,
                SmoothTerrainChunk { visual_entity: visual, collider_entity, mesh_handle },
            );

            built_this_frame += 1;
            built_tris += total_tris;
            built_ms += mesh_ms;
            telemetry.smooth_mesh_total_ms += mesh_ms;
            telemetry.smooth_mesh_max_ms = telemetry.smooth_mesh_max_ms.max(mesh_ms);
            telemetry.smooth_mesh_total_tris =
                telemetry.smooth_mesh_total_tris.saturating_add(total_tris as u64);
            telemetry.collider_rebuilds = telemetry.collider_rebuilds.saturating_add(1);
        }

        if built_this_frame > 0 {
            if moved_far || !has_mesh_record {
                telemetry.smooth_mesh_builds = telemetry.smooth_mesh_builds.saturating_add(1);
            }
            debug!(
                "smooth chunks: built {} chunk(s), {} tris, {:.0}ms total, {} cached around {:?}",
                built_this_frame,
                built_tris,
                built_ms,
                spawned.smooth_chunks.len(),
                player.block_pos
            );
        }
        spawned.last_player_block = player.block_pos;
        spawned.last_mesh_center = Some(mesh_center);
        *last_mesh_wall = time.elapsed_secs();
        return;
    }

    if !spawned.smooth_chunks.is_empty() || spawned.smooth_material.is_some() {
        clear_smooth_terrain_chunks(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut telemetry,
            &mut spawned,
        );
    }

    if cfg.smooth_terrain {
        let started = Instant::now();
        let sm = smooth_mesh::build_smooth_mesh(&game_world, min, max, 0.5, cfg.smooth_passes);
        if let Some(sm) = sm {
            let total_tris = sm.collider_indices.len() / 3;
            let old_visual_entities = std::mem::take(&mut spawned.visual_entities);
            let old_collider_entities = std::mem::take(&mut spawned.collider_entities);
            let old_mesh_handles = std::mem::take(&mut spawned.generated_meshes);
            let old_material_handles = std::mem::take(&mut spawned.generated_materials);

            let mat = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::srgb(0.04, 0.05, 0.03).into(),
                unlit: false,
                perceptual_roughness: 0.95,
                metallic: 0.0,
                cull_mode: Some(bevy::render::render_resource::Face::Back),
                double_sided: false,
                ..default()
            });
            spawned.generated_materials.push(mat.clone());
            if show_terrain_visuals {
                let mesh_handle = meshes.add(sm.mesh.clone());
                spawned.generated_meshes.push(mesh_handle.clone());
                let visual = commands
                    .spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat),
                        Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                        TerrainChunk,
                    ))
                    .id();
                spawned.visual_entities.push(visual);
            }

            if show_terrain_visuals {
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
            }

            despawn_and_release_generated_terrain(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut telemetry,
                old_visual_entities,
                old_collider_entities,
                old_mesh_handles,
                old_material_handles,
            );

            spawned.last_player_block = player.block_pos;
            spawned.last_mesh_center = Some(mesh_center);
            let mesh_secs = started.elapsed().as_secs_f32();
            let mesh_ms = mesh_secs * 1000.0;
            telemetry.smooth_mesh_builds = telemetry.smooth_mesh_builds.saturating_add(1);
            telemetry.smooth_mesh_total_ms += mesh_ms;
            telemetry.smooth_mesh_max_ms = telemetry.smooth_mesh_max_ms.max(mesh_ms);
            telemetry.smooth_mesh_total_tris =
                telemetry.smooth_mesh_total_tris.saturating_add(total_tris as u64);
            telemetry.collider_rebuilds = telemetry.collider_rebuilds.saturating_add(1);
            info!(
                "🌊 smooth mesh: {} tris, passes={}, 耗时 {:.0}ms（玩家 {:?}）",
                total_tris, cfg.smooth_passes, mesh_ms, player.block_pos
            );
        } else {
            debug!("🌊 smooth mesh: 标量场全空（无 solid 在 AABB 内）");
            spawned.last_player_block = player.block_pos;
        }
        *last_mesh_wall = time.elapsed_secs();
        return;
    }

    let started = Instant::now();
    let block_meshes = build_all_terrain_meshes_aabb(&game_world, min, max);
    let mesh_count = block_meshes.len();
    let total_tris: usize = block_meshes.iter().map(|m| m.indices.len() / 3).sum();
    let mesh_secs = started.elapsed().as_secs_f32();
    let mesh_ms = mesh_secs * 1000.0;
    if block_meshes.is_empty() {
        debug!(
            "terrain mesh rebuild produced no meshes; keeping previous terrain visible (player {:?})",
            player.block_pos
        );
        *last_mesh_wall = time.elapsed_secs();
        return;
    }
    let old_visual_entities = std::mem::take(&mut spawned.visual_entities);
    let old_collider_entities = std::mem::take(&mut spawned.collider_entities);
    let old_mesh_handles = std::mem::take(&mut spawned.generated_meshes);
    let old_material_handles = std::mem::take(&mut spawned.generated_materials);

    let mut mats: HashMap<BlockType, Handle<StandardMaterial>> = HashMap::new();
    for bt in [
        BlockType::Dirt,
        BlockType::Grass,
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
                unlit: false,
                perceptual_roughness: 0.85,
                metallic: 0.0,
                ..default()
            }
        };
        let handle = materials.add(material);
        spawned.generated_materials.push(handle.clone());
        mats.insert(bt, handle);
    }

    for bm in block_meshes {
        let bevy_mesh = bm.to_bevy_mesh();

        let collider_opt = if matches!(bm.block_type, BlockType::Water) {
            None
        } else {
            Collider::trimesh_from_mesh(&bevy_mesh)
        };

        if show_terrain_visuals {
            let mat = mats[&bm.block_type].clone();
            let mesh_handle = meshes.add(bevy_mesh.clone());
            spawned.generated_meshes.push(mesh_handle.clone());
            let visual = commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(mat),
                    Transform::from_translation(Vec3::new(0.0, cfg.y_offset, 0.0)),
                    TerrainChunk,
                ))
                .id();
            spawned.visual_entities.push(visual);
        }

        if show_terrain_visuals && let Some(collider) = collider_opt {
            telemetry.collider_rebuilds = telemetry.collider_rebuilds.saturating_add(1);
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

    despawn_and_release_generated_terrain(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut telemetry,
        old_visual_entities,
        old_collider_entities,
        old_mesh_handles,
        old_material_handles,
    );

    spawned.last_player_block = player.block_pos;
    telemetry.greedy_mesh_builds = telemetry.greedy_mesh_builds.saturating_add(1);
    telemetry.greedy_mesh_total_ms += mesh_ms;
    telemetry.greedy_mesh_max_ms = telemetry.greedy_mesh_max_ms.max(mesh_ms);
    debug!(
        "🧱 greedy mesh: {} type(s), {} tris, 耗时 {:.0}ms（玩家 {:?}）",
        mesh_count, total_tris, mesh_ms, player.block_pos
    );
    *last_mesh_wall = time.elapsed_secs();
}

#[derive(Component)]
pub struct TerrainChunk;

#[derive(Component)]
pub struct GameplayFogVolume;

pub fn setup_atmosphere(
    mut commands: Commands,
    cfg: Res<RenderConfig>,
    mut scattering_mediums: ResMut<Assets<bevy::light::atmosphere::ScatteringMedium>>,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    _camera: Query<Entity, With<Camera3d>>,
) {
    use bevy::light::Atmosphere;
    use bevy::light::atmosphere::ScatteringMedium;

    if stable_scene_baseline_enabled() {
        commands.insert_resource(ClearColor(Color::srgb(0.54, 0.73, 0.92)));
        info!("stable-scene: skipped Atmosphere and FogVolume setup");
        let _ = cfg;
        return;
    }

    // iter_456: clear color is now black-ish because the Atmosphere entity
    // occupies the entire sky as a procedural scattering sphere. The
    // `Atmosphere::earth` helper installs the right earth-radius defaults.
    let earth_medium = scattering_mediums.add(ScatteringMedium::earth(192, 64));
    let atm_entity = commands.spawn(Atmosphere::earth(earth_medium.clone())).id();
    info!(
        "🌌 iter_456: spawned Atmosphere entity {:?} (medium handle {:?})",
        atm_entity, earth_medium
    );

    // iter_456: a thin, low-density FogVolume box anchored above the spawn hill
    // so the VolumetricFog raymarch actually accumulates fog density. Default
    // density_factor (0.1) makes the entire screen opaque; we scale it down
    // so the player can see the sky + buildings clearly, with only a faint
    // atmospheric haze on the horizon.
    commands.spawn((
        FogVolume { density_factor: 0.0011, ..default() },
        Transform::from_scale(Vec3::new(360.0, 60.0, 360.0))
            .with_translation(Vec3::new(48.5, 20.0, 48.5)),
        GameplayFogVolume,
    ));

    // Keep a dark fallback clear color so any uncovered pixels stay neutral
    // (Atmosphere covers the sky, but having a sane default helps debug).
    commands.insert_resource(ClearColor(Color::srgb(0.56, 0.76, 0.98)));

    // `cfg` reserved: the legacy DistanceFog settings are no longer used, but
    // we leave them in RenderConfig for save-game / scenario compatibility.
    let _ = cfg;
}

pub fn sync_render_feature_settings(
    mut commands: Commands,
    settings: Res<RenderFeatureSettings>,
    cameras: Query<
        (
            Entity,
            Has<AtmosphereSettings>,
            Has<bevy::anti_alias::taa::TemporalAntiAliasing>,
            Has<ScreenSpaceReflections>,
            Has<ScreenSpaceAmbientOcclusion>,
            Has<VolumetricFog>,
        ),
        With<Camera3d>,
    >,
    fog_volumes: Query<Entity, With<GameplayFogVolume>>,
    volumetric_lights: Query<Entity, With<VolumetricLight>>,
    mut initialized: Local<bool>,
) {
    if !settings.is_changed() && *initialized {
        return;
    }
    *initialized = true;
    let stable_scene = stable_scene_baseline_enabled();

    for (camera, has_atmosphere_settings, has_taa, has_ssr, has_ssao, has_volumetric_fog) in
        cameras.iter()
    {
        let mut entity = commands.entity(camera);
        if settings.atmosphere && !stable_scene {
            if !has_atmosphere_settings {
                entity.insert(AtmosphereSettings::default());
            }
        } else {
            entity.remove::<AtmosphereSettings>();
        }

        if settings.taa && !stable_scene {
            if !has_taa {
                entity.insert(bevy::anti_alias::taa::TemporalAntiAliasing::default());
            }
        } else if has_taa {
            entity.remove::<(
                bevy::anti_alias::taa::TemporalAntiAliasing,
                TemporalJitter,
                MipBias,
                DepthPrepass,
                MotionVectorPrepass,
            )>();
        }

        if settings.ssr && !stable_scene {
            if !has_ssr {
                entity.insert(ScreenSpaceReflections {
                    min_perceptual_roughness: 0.0..0.0,
                    ..default()
                });
            }
        } else if has_ssr {
            entity.remove::<(ScreenSpaceReflections, DepthPrepass)>();
        }

        if settings.ssao && !stable_scene {
            if !has_ssao {
                entity.insert(ScreenSpaceAmbientOcclusion {
                    quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
                    ..default()
                });
            }
        } else if has_ssao {
            entity.remove::<(ScreenSpaceAmbientOcclusion, NormalPrepass, DepthPrepass)>();
        }

        if settings.volumetric_fog && !stable_scene {
            if !has_volumetric_fog {
                entity.insert(VolumetricFog { step_count: 48, ..default() });
            }
        } else if has_volumetric_fog {
            entity.remove::<VolumetricFog>();
        }
    }

    for entity in fog_volumes.iter() {
        commands.entity(entity).insert(if settings.volumetric_fog && !stable_scene {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }

    for entity in volumetric_lights.iter() {
        if settings.volumetric_fog && !stable_scene {
            commands.entity(entity).insert(VolumetricLight);
        } else {
            commands.entity(entity).remove::<VolumetricLight>();
        }
    }
}

#[derive(Component)]
pub struct HeldWeaponPart;

#[derive(Resource, Default)]
pub struct SwordSwing {
    pub start_t: f32,
    pub swinging: bool,
}

const SWING_DURATION: f32 = 0.18;

pub fn held_weapon_follow(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut swing: ResMut<SwordSwing>,
    mut q: Query<&mut Transform, With<HeldWeaponPart>>,
) {
    if keys.just_pressed(KeyCode::KeyK) && !swing.swinging {
        swing.swinging = true;
        swing.start_t = time.elapsed_secs();
    }

    if !swing.swinging {
        return;
    }

    let elapsed = time.elapsed_secs() - swing.start_t;
    if elapsed >= SWING_DURATION {
        swing.swinging = false;

        for mut tf in &mut q {
            tf.rotation = Quat::from_rotation_z(15_f32.to_radians());
        }
        return;
    }
    let t = elapsed / SWING_DURATION;

    let swing_phase = (t * std::f32::consts::PI).sin();

    let total_pitch = 15_f32.to_radians() + swing_phase * 90_f32.to_radians();
    for mut tf in &mut q {
        tf.rotation = Quat::from_euler(EulerRot::XYZ, total_pitch, 0.0, 15_f32.to_radians());
    }
}

pub fn mouse_look_system(
    motion: Res<AccumulatedMouseMotion>,
    mut angles: ResMut<CameraAngles>,
    cfg: Res<RenderConfig>,
    freefly: Res<FreeFlyState>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if !cfg.mouse_look && !freefly.enabled {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    if !window.visible {
        return;
    }
    if motion.delta == Vec2::ZERO {
        return;
    }

    angles.yaw += motion.delta.x * MOUSE_SENS;
    angles.pitch -= motion.delta.y * MOUSE_SENS;
    angles.pitch = angles.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
}

pub fn top_down_camera_toggle(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<CameraMode>) {
    if !keys.just_pressed(KeyCode::F3) {
        return;
    }
    *mode = match *mode {
        CameraMode::TopDown => {
            info!("CameraMode -> 3rd person");
            CameraMode::ThirdPerson
        }
        _ => {
            info!("CameraMode -> top-down");
            CameraMode::TopDown
        }
    };
}

pub fn toggle_freefly(
    keys: Res<ButtonInput<KeyCode>>,
    mut freefly: ResMut<FreeFlyState>,
    mut mode: ResMut<CameraMode>,
    camera: Query<&Transform, With<Camera3d>>,
) {
    if !keys.just_pressed(KeyCode::F4) {
        return;
    }
    freefly.enabled = !freefly.enabled;
    if freefly.enabled {
        if let Ok(tf) = camera.single() {
            freefly.position = tf.translation;
        }
        *mode = CameraMode::FirstPerson;
        info!("freefly enabled");
    } else {
        info!("freefly disabled");
    }
}

pub fn sync_camera_projection(
    mode: Res<CameraMode>,
    mut q: Query<&mut Projection, With<Camera3d>>,
) {
    if !mode.is_changed() {
        return;
    }
    let Ok(mut projection) = q.single_mut() else {
        return;
    };
    match *mode {
        CameraMode::TopDown => {
            *projection = Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical { viewport_height: TOP_DOWN_ORTHO_HEIGHT },
                ..OrthographicProjection::default_3d()
            });
        }
        CameraMode::FirstPerson | CameraMode::ThirdPerson => {
            *projection = Projection::Perspective(PerspectiveProjection::default());
        }
    }
}

pub fn camera_mode_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<CameraMode>,
    freefly: Res<FreeFlyState>,
) {
    if freefly.enabled {
        return;
    }
    if !keys.just_pressed(KeyCode::KeyC) {
        return;
    }
    *mode = match *mode {
        CameraMode::FirstPerson => {
            info!("CameraMode -> 3rd person");
            CameraMode::ThirdPerson
        }
        CameraMode::ThirdPerson => {
            info!("CameraMode -> 1st person");
            CameraMode::FirstPerson
        }
        CameraMode::TopDown => {
            info!("CameraMode -> 1st person");
            CameraMode::FirstPerson
        }
    };
}

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
    let Some((pos, block_pos)) = player_spawn_position_near(&game_world, x, z, 14, 2) else {
        warn!("🚨 F5 紧急传送失败：出生列没有可站位置");
        return;
    };
    warn!("🚨 F5 紧急传送： {:?} -> {:?}", player.block_pos, block_pos);
    set_player_position(&mut player, pos, block_pos);
}

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

pub fn freefly_movement(
    keys: Res<ButtonInput<KeyCode>>,
    mut freefly: ResMut<FreeFlyState>,
    angles: Res<CameraAngles>,
    time: Res<Time>,
) {
    if !freefly.enabled {
        return;
    }

    let (sy, cy) = angles.yaw.sin_cos();
    let (sp, cp) = angles.pitch.sin_cos();
    let forward = Vec3::new(sy * cp, sp, -cy * cp);

    let right = forward.cross(Vec3::Y);

    let up = Vec3::Y;

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

    let speed = if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::KeyE) {
        FREEFLY_SPEED * FREEFLY_BOOST
    } else {
        FREEFLY_SPEED
    };

    let target = if wish.length() > 0.01 {
        wish.normalize() * speed
    } else {
        Vec3::ZERO
    };
    let dt = time.delta_secs();

    let v = target;
    freefly.velocity = v;
    let pos = freefly.position + v * dt;
    freefly.position = pos;
}

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

pub fn maintain_cursor_grab(
    motion: Res<AccumulatedMouseMotion>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    cfg: Res<RenderConfig>,
    freefly: Res<FreeFlyState>,
) {
    if !cfg.mouse_look && !freefly.enabled {
        return;
    }
    if motion.delta == Vec2::ZERO {
        return;
    }
    let Ok(mut cursor) = cursors.single_mut() else {
        return;
    };
    if !matches!(cursor.grab_mode, CursorGrabMode::Locked) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

pub fn toggle_cursor_grab_on_esc(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    cfg: Res<RenderConfig>,
) {
    if !cfg.mouse_look {
        return;
    }
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    let Ok(mut cursor) = cursors.single_mut() else {
        return;
    };

    let is_locked = matches!(cursor.grab_mode, CursorGrabMode::Locked);
    if is_locked {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
        info!("🖱 ESC：释放光标（按 ESC 再抓回）");
    } else {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;

        info!("🖱 ESC：抓回光标");
    }
}

#[derive(Component)]
pub struct AnimalIndicatorText;

pub fn update_animal_indicator(
    mut q_text: Query<&mut Text, With<AnimalIndicatorText>>,
    player: Res<PlayerState>,
    mode: Res<CameraMode>,
    camera: Query<&Transform, With<Camera3d>>,
    creatures: Query<&Creature>,
) {
    let Ok(mut text) = q_text.single_mut() else {
        return;
    };
    if *mode == CameraMode::TopDown {
        text.0.clear();
        return;
    }
    let px = player.block_pos[0] as f32 + 0.5;
    let pz = player.block_pos[2] as f32 + 0.5;

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
        text.0 = "[no animal within 30m]".to_string();
        return;
    };
    let dist = d2.sqrt();
    let animal_v = Vec2::new(
        c.block_pos[0] as f32 + 0.5 - px,
        c.block_pos[2] as f32 + 0.5 - pz,
    );
    if animal_v.length() < 0.01 {
        text.0 = format!("* {} underfoot", c.kind.label_zh());
        return;
    }

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

    let label = match c.kind {
        lk2_core::creature::CreatureKind::Pig => "Pig",
        lk2_core::creature::CreatureKind::Sheep => "Sheep",
        lk2_core::creature::CreatureKind::Cow => "Cow",
        lk2_core::creature::CreatureKind::Chicken => "Chicken",
    };
    text.0 = format!("{}  {}  {:.1}m", arrow, label, dist);
}

#[allow(dead_code)]
const NEST_MARKER_SIZE: (f32, f32, f32) = (0.4, 4.0, 0.4);
#[allow(dead_code)]
const THIRD_PERSON_NEST_MARKER_SIZE: (f32, f32, f32) = (0.45, 0.55, 0.45);

#[derive(Component)]
pub struct NestMarker;

#[derive(Resource, Default)]
#[allow(dead_code)]
pub struct NestMarkerCount(pub u32);

#[allow(dead_code)]
pub fn spawn_nest_markers(
    mut commands: Commands,
    monsters: Res<MonsterEcosystem>,
    _player: Res<PlayerState>,
    mode: Res<CameraMode>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut count: ResMut<NestMarkerCount>,
) {
    if *mode == CameraMode::FirstPerson {
        count.0 = 0;
        return;
    }

    let mut spawned = 0_u32;

    for (_kid, kingdom) in monsters.kingdoms.iter() {
        if kingdom.destroyed {
            continue;
        }
        for (_nid, nest) in kingdom.nests.iter() {
            let (base_r, base_g, base_b) = match nest.biome {
                Biome::Desert => (1.0_f32, 0.85_f32, 0.5_f32),
                Biome::Jungle => (0.4_f32, 0.7_f32, 0.3_f32),
                Biome::Tundra => (0.7_f32, 0.85_f32, 1.0_f32),
            };
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(base_r, base_g, base_b),
                emissive: Color::srgb(1.0, 0.25, 0.15).into(),
                perceptual_roughness: 0.7,
                metallic: 0.0,
                ..default()
            });

            let marker_size = if *mode == CameraMode::TopDown {
                NEST_MARKER_SIZE
            } else {
                THIRD_PERSON_NEST_MARKER_SIZE
            };
            let mesh = meshes.add(Cuboid::new(marker_size.0, marker_size.1, marker_size.2));

            let nx = nest.center[0] as f32 + 0.5;
            let nz = nest.center[2] as f32 + 0.5;
            let ny = if *mode == CameraMode::TopDown {
                nest.center[1] as f32 + 5.0
            } else {
                nest.center[1] as f32 + marker_size.1 * 0.5
            };
            commands.spawn((
                NestMarker,
                Mesh3d(mesh),
                MeshMaterial3d(mat),
                Transform::from_translation(Vec3::new(nx, ny, nz)),
            ));
            spawned += 1;
        }
    }

    count.0 = spawned;
    info!(
        "🏳 MonsterNest 3D 旗杆已 spawn {} 个（biome 配色 + RedStrong emissive）",
        spawned
    );
}

#[derive(Component)]
pub struct NestIndicatorText;

pub fn update_nest_indicator(
    mut q_text: Query<&mut Text, With<NestIndicatorText>>,
    player: Res<PlayerState>,
    mode: Res<CameraMode>,
    camera: Query<&Transform, With<Camera3d>>,
    monsters: Res<MonsterEcosystem>,
) {
    let Ok(mut text) = q_text.single_mut() else {
        return;
    };
    if *mode == CameraMode::TopDown {
        text.0.clear();
        return;
    }
    let px = player.block_pos[0] as f32 + 0.5;
    let pz = player.block_pos[2] as f32 + 0.5;

    let mut best: Option<([i32; 3], u32, f32)> = None;
    for (_kid, k) in monsters.kingdoms.iter() {
        if k.destroyed {
            continue;
        }
        for (_nid, n) in k.nests.iter() {
            let dx = n.center[0] as f32 + 0.5 - px;
            let dz = n.center[2] as f32 + 0.5 - pz;
            let d2 = dx * dx + dz * dz;
            if d2 < 900.0 && (best.is_none() || d2 < best.unwrap().2) {
                best = Some((n.center, n.individuals.len() as u32, d2));
            }
        }
    }

    let Some((center, count, d2)) = best else {
        text.0 = "[no nest within 30m]".to_string();
        return;
    };
    let dist = d2.sqrt();
    let nest_v = Vec2::new(center[0] as f32 + 0.5 - px, center[2] as f32 + 0.5 - pz);
    if nest_v.length() < 0.01 {
        text.0 = format!("* Nest underfoot / {} mobs", count);
        return;
    }

    let arrow = if let Ok(tf) = camera.single() {
        let f = tf.forward();
        let cam = Vec2::new(f.x, f.z);
        let cam_n = if cam.length() > 0.01 {
            cam.normalize()
        } else {
            Vec2::new(1.0, 0.0)
        };
        let dot = nest_v.dot(cam_n);
        let cross = nest_v.x * cam_n.y - nest_v.y * cam_n.x;

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

    text.0 = format!("{} Nest {:.0}m / {} mobs", arrow, dist, count);
}

#[derive(Resource, Default)]
pub struct LastMoveDirection(pub Vec3);

pub fn player_input(
    run_mode: Res<ClientRunMode>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player: ResMut<PlayerState>,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<lk2_core::resource::GlobalResourcePool>,
    mut angles: ResMut<CameraAngles>,
    mut nations: ResMut<NationRegistry>,
    mut monsters: ResMut<MonsterEcosystem>,
    _camera: Query<&Transform, With<Camera3d>>,
    time: Res<Time>,
    freefly: Res<FreeFlyState>,
    cfg: Res<RenderConfig>,
    mut motion_trace: ResMut<crate::OnlineMotionTrace>,
    mut jump: ResMut<JumpState>,
) {
    let is_offline = *run_mode == ClientRunMode::Offline;

    let freefly_active = freefly.enabled;

    let (sy, cy) = angles.yaw.sin_cos();
    let forward = Vec3::new(sy, 0.0, -cy).normalize_or_zero();
    let right = forward.cross(Vec3::Y).normalize_or_zero();

    let mut d = Vec3::ZERO;
    if !freefly_active {
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) || cfg.auto_walk {
            d += forward;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            d -= forward;
        }
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            d -= right;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            d += right;
        }
        if is_offline
            && (keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight))
        {
            d -= Vec3::Y;
        }
    }

    if !freefly_active {
        if keys.just_pressed(KeyCode::KeyQ) {
            angles.yaw -= YAW_QE_STEP;
        } else if keys.just_pressed(KeyCode::KeyE) {
            angles.yaw += YAW_QE_STEP;
        }
        let pitch_step = 45.0_f32.to_radians() * time.delta_secs();
        if keys.pressed(KeyCode::KeyR) || keys.pressed(KeyCode::PageUp) {
            angles.pitch += pitch_step;
        }
        if keys.pressed(KeyCode::KeyT) || keys.pressed(KeyCode::PageDown) {
            angles.pitch -= pitch_step;
        }
        angles.pitch = angles.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    // First-person controls must move immediately in online mode too. The
    // server still receives the same input and can correct large desyncs, but
    // the camera is driven by local prediction instead of waiting for packets.
    motion_trace.last_local_attempted = d.length() > 0.01;
    motion_trace.last_local_moved = false;
    motion_trace.last_local_reason = "idle";
    motion_trace.last_local_dir = [d.x, d.y, d.z];
    motion_trace.last_local_distance = 0.0;
    if d.length() > 0.01 {
        if d.y.abs() > 0.01 && d.x.abs() < 0.01 && d.z.abs() < 0.01 {
            motion_trace.last_local_moved = try_player_move(
                &mut player,
                &mut game_world,
                [0, d.y.signum() as i32, 0],
                cfg.ground_step_threshold,
            );
            motion_trace.last_local_reason = if motion_trace.last_local_moved {
                "moved_discrete"
            } else {
                "blocked_discrete"
            };
        } else {
            let sprint = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
            let speed = if sprint {
                MANUAL_MOVE_SPEED * 1.5
            } else {
                MANUAL_MOVE_SPEED
            };
            let distance = speed * time.delta_secs();
            motion_trace.last_local_distance = distance;
            let (moved, reason) = if jump.grounded {
                try_player_move_continuous(
                    &mut player,
                    &game_world,
                    Vec3::new(d.x, 0.0, d.z),
                    distance,
                    cfg.ground_step_threshold,
                )
            } else {
                try_player_air_move_continuous(
                    &mut player,
                    &game_world,
                    Vec3::new(d.x, 0.0, d.z),
                    distance,
                )
            };
            motion_trace.last_local_moved = moved;
            motion_trace.last_local_reason = reason;
        }
    }

    if !freefly_active {
        let jumped = keys.just_pressed(KeyCode::Space) && jump.grounded;
        if jumped {
            jump.velocity_y = JUMP_TAKEOFF_SPEED;
            jump.grounded = false;
        }
        let (jump_moved, jump_reason) = step_player_jump(
            &mut player,
            &game_world,
            &mut jump,
            time.delta_secs(),
            cfg.ground_step_threshold,
        );
        if jumped || jump_moved {
            motion_trace.last_local_attempted = true;
            motion_trace.last_local_moved |= jump_moved;
            motion_trace.last_local_reason = jump_reason;
        }
    } else {
        jump.velocity_y = 0.0;
        jump.grounded = true;
    }

    if is_offline && keys.just_pressed(KeyCode::KeyG) {
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

    if is_offline && keys.just_pressed(KeyCode::KeyF) {
        if player.nation_id.is_some() {
            info!("🚩 你已经是一个国家的王了，不能再立旗");
        } else {
            let tick_now = time.elapsed_secs() as u64;
            let cost = nations.next_flag_cost();
            match nations.found(
                &mut pool,
                0u32,
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

    if is_offline && keys.just_pressed(KeyCode::KeyJ) {
        let p = player.block_pos;
        let mut best: Option<(f32, u32, u32, u32)> = None;
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

fn set_player_position(player: &mut PlayerState, pos: Vec3, block_pos: [i32; 3]) {
    player.pos = pos;
    player.block_pos = block_pos;
}

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

fn try_player_move_continuous(
    player: &mut PlayerState,
    game_world: &GameWorld,
    dir: Vec3,
    distance: f32,
    threshold: f32,
) -> (bool, &'static str) {
    let horizontal = Vec3::new(dir.x, 0.0, dir.z);
    if horizontal.length_squared() < 0.0001 || distance <= 0.0 {
        return (false, "no_horizontal_or_distance");
    }

    let dir = horizontal.normalize();
    let candidates = [
        dir,
        Vec3::new(dir.z, 0.0, -dir.x).normalize_or_zero(),
        Vec3::new(-dir.z, 0.0, dir.x).normalize_or_zero(),
    ];

    for (i, candidate_dir) in candidates.into_iter().enumerate() {
        if candidate_dir.length_squared() < 0.0001 {
            continue;
        }
        let next = player.pos + candidate_dir * distance;
        let next_x = next.x.floor() as i32;
        let next_z = next.z.floor() as i32;
        let Some((stand_pos, block_pos)) =
            player_stand_position_at(game_world, next_x, next_z, player.pos.y, threshold)
        else {
            continue;
        };

        if stand_pos.y - player.pos.y > threshold {
            continue;
        }
        if !player_volume_clear_at(game_world, Vec3::new(next.x, stand_pos.y, next.z)) {
            continue;
        }

        set_player_position(player, Vec3::new(next.x, stand_pos.y, next.z), block_pos);
        return if i == 0 {
            (true, "moved_continuous")
        } else {
            (true, "slid_continuous")
        };
    }

    (false, "blocked_continuous")
}

fn try_player_air_move_continuous(
    player: &mut PlayerState,
    game_world: &GameWorld,
    dir: Vec3,
    distance: f32,
) -> (bool, &'static str) {
    let horizontal = Vec3::new(dir.x, 0.0, dir.z);
    if horizontal.length_squared() < 0.0001 || distance <= 0.0 {
        return (false, "air_no_horizontal_or_distance");
    }

    let next = player.pos + horizontal.normalize() * distance;
    if !player_volume_clear_at(game_world, next) {
        return (false, "air_volume_blocked");
    }

    set_player_position(
        player,
        next,
        [
            next.x.floor() as i32,
            player.block_pos[1],
            next.z.floor() as i32,
        ],
    );
    (true, "air_moved_continuous")
}

fn player_volume_clear_at(game_world: &GameWorld, pos: Vec3) -> bool {
    let foot_y = pos.y.floor() as i32;
    if !player_body_clear(
        game_world,
        pos.x.floor() as i32,
        foot_y,
        pos.z.floor() as i32,
    ) {
        return false;
    }
    for y in foot_y..=(foot_y + 1) {
        for ox in [-PLAYER_COLLISION_RADIUS, PLAYER_COLLISION_RADIUS] {
            for oz in [-PLAYER_COLLISION_RADIUS, PLAYER_COLLISION_RADIUS] {
                let x = (pos.x + ox).floor() as i32;
                let z = (pos.z + oz).floor() as i32;
                if x < 0
                    || x >= game_world.size
                    || y < 0
                    || y >= game_world.size
                    || z < 0
                    || z >= game_world.size
                {
                    return false;
                }
                if game_world.get(x, y, z).is_solid() {
                    return false;
                }
            }
        }
    }
    true
}

pub fn offline_anti_stuck(
    game_world: Res<GameWorld>,
    mut player: ResMut<PlayerState>,
    mut anti_stuck: ResMut<AntiStuckState>,
    mut player_tf_q: Query<&mut Transform, With<Player>>,
) {
    const SEARCH_RADIUS: i32 = 6;
    const UNSAFE_TICK_LIMIT: u8 = 12;

    if player_position_is_safe(&game_world, player.pos) {
        anti_stuck.last_safe_pos = player.pos;
        anti_stuck.last_safe_block = player.block_pos;
        anti_stuck.unsafe_ticks = 0;
        return;
    }

    anti_stuck.unsafe_ticks = anti_stuck.unsafe_ticks.saturating_add(1);
    if anti_stuck.unsafe_ticks < UNSAFE_TICK_LIMIT {
        return;
    }

    let resolved =
        resolve_player_stuck_near(&game_world, player.pos, SEARCH_RADIUS).or_else(|| {
            resolve_player_stuck_near(&game_world, anti_stuck.last_safe_pos, SEARCH_RADIUS)
        });
    let Some((pos, block_pos)) = resolved else {
        return;
    };

    set_player_position(&mut player, pos, block_pos);
    if let Ok(mut tf) = player_tf_q.single_mut() {
        tf.translation = pos;
    }
    anti_stuck.last_safe_pos = pos;
    anti_stuck.last_safe_block = block_pos;
    anti_stuck.unsafe_ticks = 0;
    tracing::warn!("[anti-stuck] offline resolved player to {:?}", block_pos);
}

fn step_player_jump(
    player: &mut PlayerState,
    game_world: &GameWorld,
    jump: &mut JumpState,
    dt: f32,
    step_threshold: f32,
) -> (bool, &'static str) {
    let dt = dt.clamp(0.0, 0.05);
    let Some((stand_pos, stand_block)) = player_stand_position_at(
        game_world,
        player.pos.x.floor() as i32,
        player.pos.z.floor() as i32,
        player.pos.y,
        step_threshold,
    ) else {
        jump.grounded = false;
        jump.velocity_y = (jump.velocity_y - JUMP_GRAVITY * dt).max(JUMP_TERMINAL_SPEED);
        return (false, "jump_no_floor");
    };

    let ground_y = stand_pos.y;
    if player.pos.y <= ground_y + 0.02 && jump.velocity_y <= 0.0 {
        if (player.pos.y - ground_y).abs() > 0.001 || player.block_pos != stand_block {
            set_player_position(
                player,
                Vec3::new(player.pos.x, ground_y, player.pos.z),
                stand_block,
            );
        }
        jump.velocity_y = 0.0;
        jump.grounded = true;
        return (false, "grounded");
    }

    jump.grounded = false;
    jump.velocity_y = (jump.velocity_y - JUMP_GRAVITY * dt).max(JUMP_TERMINAL_SPEED);
    let next_y = player.pos.y + jump.velocity_y * dt;

    if jump.velocity_y > 0.0 {
        let next_pos = Vec3::new(player.pos.x, next_y, player.pos.z);
        if player_volume_clear_at(game_world, next_pos) {
            set_player_position(player, next_pos, stand_block);
            return (true, "jump_rise");
        }
        jump.velocity_y = 0.0;
        return (false, "jump_head_blocked");
    }

    if next_y <= ground_y {
        set_player_position(
            player,
            Vec3::new(player.pos.x, ground_y, player.pos.z),
            stand_block,
        );
        jump.velocity_y = 0.0;
        jump.grounded = true;
        return (true, "jump_landed");
    }

    set_player_position(
        player,
        Vec3::new(player.pos.x, next_y, player.pos.z),
        stand_block,
    );
    (true, "jump_fall")
}

#[derive(Component)]
pub struct Player;

pub fn auto_demo(
    time: Res<Time>,
    mut player: ResMut<PlayerState>,
    mut game_world: ResMut<GameWorld>,
    mut pool: ResMut<lk2_core::resource::GlobalResourcePool>,
    mut nations: ResMut<lk2_core::nation::NationRegistry>,
    cfg: Res<RenderConfig>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut last: ResMut<LastMoveDirection>,
    mut walk_timer: Local<f32>,
    mut walk_step: Local<u32>,
    mut auto_frame: Local<u32>,

    mut walk_target: Local<Option<[i32; 3]>>,
    mut player_tf_q: Query<&mut Transform, With<Player>>,
    creatures: Query<&lk2_core::creature::Creature>,
    monsters: Res<lk2_core::monster::MonsterEcosystem>,
) {
    if cfg.auto_keys {
        *auto_frame += 1;

        if *auto_frame == 60 {
            keys.press(KeyCode::KeyF);
        }
        if *auto_frame == 62 {
            keys.release(KeyCode::KeyF);
        }

        if *auto_frame == 240 {
            keys.press(KeyCode::KeyJ);
        }
        if *auto_frame == 242 {
            keys.release(KeyCode::KeyJ);
        }
        if *auto_frame == 300 || *auto_frame == 520 {
            keys.press(KeyCode::KeyI);
        }
        if *auto_frame == 302 || *auto_frame == 522 {
            keys.release(KeyCode::KeyI);
        }
        if *auto_frame == 360 {
            keys.press(KeyCode::KeyK);
        }
        if *auto_frame == 362 {
            keys.release(KeyCode::KeyK);
        }

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

    let walk_interval = if cfg.auto_keys {
        0.1
    } else {
        cfg.auto_walk_interval_secs
    };
    if *walk_timer < walk_interval {
        return;
    }
    *walk_timer = 0.0;
    *walk_step += 1;

    if !nations.can_found_new() {
        *walk_target = None;
    }

    let need_refresh = match *walk_target {
        None => true,
        Some(t) => {
            let dx = (t[0] - player.block_pos[0]) as f32;
            let dz = (t[2] - player.block_pos[2]) as f32;
            dx * dx + dz * dz < 1.5 * 1.5
        }
    };
    if need_refresh && nations.can_found_new() {
        const MIN_WALK_SPREAD: f32 = 25.0;
        let mut best: Option<(f32, [i32; 3])> = None;
        let p = player.block_pos;
        let mut consider = |pos: [i32; 3], weight: f32| {
            let dx = (pos[0] - p[0]) as f32;
            let dz = (pos[2] - p[2]) as f32;
            let d2 = dx * dx + dz * dz * weight;

            if d2 < MIN_WALK_SPREAD {
                return;
            }
            match best {
                None => best = Some((d2, pos)),
                Some((bd, _)) if d2 < bd => best = Some((d2, pos)),
                _ => {}
            }
        };
        for c in creatures.iter() {
            consider(c.block_pos, 1.0);
        }
        for k in monsters.kingdoms.values() {
            if k.destroyed {
                continue;
            }
            for n in k.nests.values() {
                if n.dormant {
                    continue;
                }
                consider(n.center, 1.2);
            }
        }
        *walk_target = best.map(|(_, pos)| pos);
    }

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

    let near_y = player.block_pos[1] as f32;
    let good_dirs: Vec<[i32; 3]> = all_dirs
        .iter()
        .filter(|d| {
            let nx1 = player.block_pos[0] + d[0];
            let nz1 = player.block_pos[2] + d[2];
            let nx2 = nx1 + d[0];
            let nz2 = nz1 + d[2];
            player_stand_position_at(&game_world, nx1, nz1, near_y, cfg.ground_step_threshold)
                .and_then(|(pos, _)| {
                    player_stand_position_at(
                        &game_world,
                        nx2,
                        nz2,
                        pos.y,
                        cfg.ground_step_threshold,
                    )
                })
                .is_some()
        })
        .copied()
        .collect();

    let d: [i32; 3] = if good_dirs.is_empty() {
        match *walk_target {
            Some(t) => {
                let dx = (t[0] - player.block_pos[0]).signum();
                let dz = (t[2] - player.block_pos[2]).signum();
                [dx, 0, dz]
            }
            None => [1, 0, 0],
        }
    } else {
        match *walk_target {
            Some(t) => {
                let dx = (t[0] - player.block_pos[0]).signum();
                let dz = (t[2] - player.block_pos[2]).signum();
                good_dirs
                    .iter()
                    .copied()
                    .find(|gd| gd[0] == dx && gd[2] == dz)
                    .unwrap_or_else(|| good_dirs[(*walk_step as usize) % good_dirs.len()])
            }
            None => good_dirs[(*walk_step as usize) % good_dirs.len()],
        }
    };

    let before = player.block_pos;

    let moved = try_player_move(&mut player, &mut game_world, d, cfg.ground_step_threshold);
    if moved || (player.block_pos != before) {
        let dx = (player.block_pos[0] - before[0]) as f32;
        let dz = (player.block_pos[2] - before[2]) as f32;
        if dx != 0.0 || dz != 0.0 {
            let v = Vec3::new(dx, 0.0, dz);
            last.0 = v.normalize();
        }
        if let Ok(mut tf) = player_tf_q.single_mut() {
            tf.translation = player.pos;
        }
    }

    const AI_WALK_KING_ID_BASE: u32 = 100;
    if let Some(target) = *walk_target {
        let dx = (player.block_pos[0] - target[0]) as f32;
        let dz = (player.block_pos[2] - target[2]) as f32;
        if dx * dx + dz * dz <= 1.5 * 1.5 {
            if !nations.can_found_new() {
                *walk_target = None;
                return;
            }

            let cost = nations.next_flag_cost();

            if cfg.auto_keys {
                let have = pool.get(lk2_core::resource::ResourceKind::Soul);
                if have < cost {
                    let _ = pool.try_add(lk2_core::resource::ResourceKind::Soul, cost - have);
                }
            }
            let ai_king_id = AI_WALK_KING_ID_BASE + nations.flag_count;
            match nations.found(
                &mut pool,
                ai_king_id,
                format!("AIWalk#{}@{:?}", ai_king_id, player.block_pos),
                player.block_pos,
                *auto_frame as u64,
            ) {
                Ok(id) => {
                    tracing::info!(
                        "[auto-demo walk] ✓ 到达 walk_target, 在 ({},{},{}) 真建新国 id={} (king={}, cost={}, total={})",
                        player.block_pos[0],
                        player.block_pos[1],
                        player.block_pos[2],
                        id.0,
                        ai_king_id,
                        cost,
                        nations.flag_count
                    );
                }
                Err(e) => {
                    tracing::warn!("[auto-demo walk] 到达 walk_target 但建新国失败: {}", e);
                }
            }
            *walk_target = None;
        }
    }

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
    anim_state: Res<PlayerAnimState>,
) {
    let Ok(mut tf) = q.single_mut() else {
        return;
    };

    if freefly.enabled {
        let (sy, cy) = angles.yaw.sin_cos();
        let (sp, cp) = angles.pitch.sin_cos();
        let dir = Vec3::new(sy * cp, sp, -cy * cp);
        let look_target = freefly.position + dir * 5.0;
        tf.translation = freefly.position;
        tf.look_at(look_target, Vec3::Y);
        return;
    }

    if cfg.auto_keys && cfg.auto_orbit && !cfg.mouse_look {
        let center_x = constant::WORLD_SIZE as f32 * 0.5 + 0.5;
        let center_z = constant::WORLD_SIZE as f32 * 0.5 + 0.5;
        let ground_top =
            effective_ground_height(&world, center_x.floor() as i32, center_z.floor() as i32);
        let target = Vec3::new(center_x, ground_top + 1.8, center_z);
        tf.translation = target + Vec3::new(-8.5, 7.5, 12.0);
        tf.look_at(target, Vec3::Y);
        return;
    }

    if cfg.auto_orbit && !cfg.mouse_look {
        *orbit_angle += time.delta_secs() * cfg.auto_orbit_speed;
        let a = *orbit_angle;
        let target = player.pos - Vec3::Y * 1.25;
        let cam_pos = target
            + Vec3::new(
                a.cos() * cfg.auto_orbit_distance,
                14.0,
                a.sin() * cfg.auto_orbit_distance,
            );
        tf.translation = cam_pos;
        tf.look_at(target, Vec3::Y);
        return;
    }

    if *mode == CameraMode::ThirdPerson {
        let provisional = compute_third_person_camera(player.pos, angles.yaw, angles.pitch, 0.0);
        let ground = effective_ground_height(
            &world,
            provisional.translation.x.floor() as i32,
            provisional.translation.z.floor() as i32,
        );
        let camera = compute_third_person_camera(player.pos, angles.yaw, angles.pitch, ground);
        tf.translation = camera.translation;
        tf.look_at(camera.target, Vec3::Y);
        return;
    }

    if *mode == CameraMode::TopDown {
        let ground_top = effective_ground_height(
            &world,
            player.pos.x.floor() as i32,
            player.pos.z.floor() as i32,
        );
        let target = Vec3::new(player.pos.x, ground_top + 0.8, player.pos.z);
        let camera_offset = Vec3::new(-48.0, 64.0, 38.0);
        tf.translation = target + camera_offset;
        tf.look_at(target, Vec3::Y);
        return;
    }

    let eye_base = player.pos + Vec3::Y * 1.7;

    let phase = anim_state.step_phase;
    let speed = anim_state.smoothed_speed;
    let bob_y = phase.sin().abs() * 0.06 * speed + (phase * 0.18).sin() * 0.012;
    let bob_x = (phase * 0.5).sin() * 0.04 * speed;
    let eye = eye_base + Vec3::new(bob_x, bob_y, 0.0);

    let dir = if cfg.mouse_look || *mode == CameraMode::FirstPerson {
        let (sy, cy) = angles.yaw.sin_cos();
        let render_pitch = angles.pitch.clamp(-0.55, 0.35);
        let (sp, cp) = render_pitch.sin_cos();
        Vec3::new(sy * cp, sp, -cy * cp)
    } else {
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
    let look_target = eye + dir * 5.0;
    tf.translation = eye;
    tf.look_at(look_target, Vec3::Y);
}

#[cfg(test)]
mod tests {
    use super::*;
    use lk2_core::world::{WorldConfig, generate_world, player_spawn_position_at};

    #[test]
    fn stable_scene_baseline_can_be_enabled_by_arg_or_env() {
        assert!(!stable_scene_baseline_enabled_from(
            [false, false].into_iter(),
            false
        ));
        assert!(stable_scene_baseline_enabled_from(
            [false, true].into_iter(),
            false
        ));
        assert!(stable_scene_baseline_enabled_from(
            [false, false].into_iter(),
            true
        ));
    }

    fn flat_test_world() -> GameWorld {
        let mut world = GameWorld::new(8);
        for x in 0..world.size {
            for z in 0..world.size {
                world.set(x, 0, z, BlockType::Stone);
            }
        }
        world
    }

    #[test]
    fn jump_is_short_weighty_and_regrounds() {
        let world = flat_test_world();
        let mut player =
            PlayerState { pos: Vec3::new(3.5, 1.0, 3.5), block_pos: [3, 1, 3], ..default() };
        let mut jump = JumpState { velocity_y: JUMP_TAKEOFF_SPEED, grounded: false };
        let mut peak = player.pos.y;
        let mut landed_at = None;

        for frame in 1..=90 {
            let (moved, _) = step_player_jump(&mut player, &world, &mut jump, 1.0 / 60.0, 0.85);
            peak = peak.max(player.pos.y);
            if !moved && jump.grounded {
                landed_at = Some(frame);
                break;
            }
        }

        let landed_at = landed_at.expect("jump should land within the simulation window");
        assert!(
            (1.65..=2.15).contains(&peak),
            "jump peak should feel snappy and grounded, got y={peak}"
        );
        assert!(
            (25..=45).contains(&landed_at),
            "jump should land quickly instead of floating, frame={landed_at}"
        );
        assert_eq!(player.pos.y, 1.0);
        assert_eq!(jump.velocity_y, 0.0);
        assert!(jump.grounded);
    }

    #[test]
    fn jump_cannot_retrigger_until_grounded() {
        let world = flat_test_world();
        let mut player =
            PlayerState { pos: Vec3::new(3.5, 1.0, 3.5), block_pos: [3, 1, 3], ..default() };
        let mut jump = JumpState { velocity_y: JUMP_TAKEOFF_SPEED, grounded: false };

        let _ = step_player_jump(&mut player, &world, &mut jump, 1.0 / 60.0, 0.85);
        let mid_air_velocity = jump.velocity_y;
        let attempted_retrigger = jump.grounded;
        if attempted_retrigger {
            jump.velocity_y = JUMP_TAKEOFF_SPEED;
        }

        assert!(!attempted_retrigger);
        assert_eq!(jump.velocity_y, mid_air_velocity);
    }

    #[test]
    fn air_move_preserves_jump_height() {
        let world = flat_test_world();
        let mut player =
            PlayerState { pos: Vec3::new(3.5, 1.7, 3.5), block_pos: [3, 1, 3], ..default() };

        let (moved, reason) = try_player_air_move_continuous(&mut player, &world, Vec3::X, 0.25);

        assert!(moved, "{reason}");
        assert!((player.pos.y - 1.7).abs() < 0.001);
        assert!(player.pos.x > 3.5);
    }

    #[test]
    fn continuous_move_crosses_flat_leaf_platform() {
        let mut world = GameWorld::new(8);
        for x in 0..world.size {
            for z in 0..world.size {
                world.set(x, 0, z, BlockType::Leaves);
            }
        }
        let mut player =
            PlayerState { pos: Vec3::new(3.5, 1.0, 3.5), block_pos: [3, 1, 3], ..default() };

        let (moved, reason) = try_player_move_continuous(&mut player, &world, Vec3::X, 0.25, 0.85);

        assert!(moved, "{reason}");
        assert!(player.pos.x > 3.5);
        assert_eq!(player.block_pos, [3, 1, 3]);
    }

    #[test]
    fn continuous_move_from_default_spawn_advances() {
        let world = generate_world(&WorldConfig::default());
        let (pos, block_pos) =
            player_spawn_position_at(&world, constant::WORLD_SIZE / 2, constant::WORLD_SIZE / 2)
                .expect("default world should have a spawn position");
        let mut player = PlayerState { pos, block_pos, ..default() };

        let (moved, reason) = try_player_move_continuous(&mut player, &world, Vec3::X, 0.25, 0.85);

        assert!(moved, "{reason}");
        assert!(player.pos.x > pos.x);
    }

    #[test]
    fn continuous_move_blocks_world_edge_escape() {
        let world = flat_test_world();
        let mut player =
            PlayerState { pos: Vec3::new(7.5, 1.0, 3.5), block_pos: [7, 1, 3], ..default() };

        let (_moved, _reason) =
            try_player_move_continuous(&mut player, &world, Vec3::X, 0.75, 0.85);

        assert!(player.pos.x < world.size as f32);
        assert!(player.block_pos[0] < world.size);
    }
}
