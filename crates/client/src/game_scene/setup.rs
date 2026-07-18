//! Initial scene construction: rendering resources, lighting, terrain, the
//! living scene population (player, boss, trees, grass, clouds, rain), and
//! the camera. Everything runs once on `Startup`.

use avian3d::prelude::{
    Collider, GravityScale, LockedAxes, RigidBody, TransformInterpolation, TrimeshFlags,
};
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::camera::Hdr;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{
    AtmosphereEnvironmentMapLight, CascadeShadowConfigBuilder, DirectionalLightShadowMap,
    ShadowFilteringMethod, VolumetricLight,
};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{
    AtmosphereSettings, ContactShadows, DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion,
    ScreenSpaceAmbientOcclusionQualityLevel,
};
use bevy::prelude::*;
use bevy::render::camera::{MipBias, TemporalJitter};
use bevy_hanabi::{EffectAsset, ParticleEffect};
use bevy_tnua::builtins::{TnuaBuiltinJumpConfig, TnuaBuiltinWalkConfig};
use bevy_tnua::prelude::{TnuaConfig, TnuaController};
use bevy_tnua_avian3d::prelude::TnuaAvian3dSensorShape;

use super::combat_fx::{create_ambient_mote_effect, create_enemy_hit_effect, create_hit_effect};
use super::content_visuals::{
    LivingContentLayout, grounded_content_position, spawn_content_visuals,
    spawn_exported_content_visuals,
};
use super::offline::OfflineNature;
use super::player::{PlayerControlScheme, PlayerControlSchemeConfig};
use super::procedural_motion::{GrassWind, ProceduralTreeSway};
use super::state::{
    BossActor, Cloud, CollisionDebugState, DragonKatanaPickup, FARM_PLOT_POSITIONS,
    FarmVisualMaterials, ForestRealmCollider, ForestRealmVisual, GrassTuft, HeldWeaponVisual,
    HitReaction, LivingCameraRig, LivingSceneCamera, LivingSun, MINE_SURFACE_DEFORMATION_RADIUS,
    PROCEDURAL_COLLISION_GRID_STEP, PROCEDURAL_TERRAIN_CENTER, PROCEDURAL_TERRAIN_RADIUS,
    PlayerActor, PlayerGroundShadow,
    PlayerIkPart, PlayerIkPartKind, PlayerJump, PlayerMotion, PlayerSkillState, PlayerSwimState,
    ProceduralTerrainSurface, RainDrop, ReaperScythePickup, RetiredTerrainMesh, SceneMaterials,
    TERRAIN_MESH_RETIRE_FRAMES, TerrainRebuildState, TerrainUndergroundState, WaterFish,
    WaterSeaweed, configure_collision_debug_gizmos,
};
use super::stylized_material::{
    GrassWindExtension, GrassWindMaterial, StylizedTerrainExtension, StylizedTerrainMaterial,
    TreeShadowAssets, WaterSurfaceExtension, WaterSurfaceMaterial,
};
use super::util::{
    AUTUMN_TREE_PATH, BIRCH_TREE_PATH, BOSS_PATH, BRANCH_PATH, CAMPFIRE_PATH, CLOUD_PATH,
    CLOUD_VISUAL_HEIGHT,
    CLOUD_VISUAL_SCALE, DEER_FAWN_PATH, DRAGON_KATANA_PATH, FISH_PATH, FOREST_STONE_SPIRE_PATH,
    FOX_SILVER_PATH, PINE_TREE_PATH, PLAYER_COLLIDER_RADIUS, PLAYER_COLLIDER_SEGMENT,
    PLAYER_JUMP_HEIGHT, PLAYER_PHYSICS_CENTER_HEIGHT, PLAYER_SPEED, RABBIT_BROWN_PATH,
    REAPER_SCYTHE_PATH, ROCK_PATH, ROUND_TREE_PATH, TREE_PATH, WILLOW_TREE_PATH, hash01,
    spawn_asset,
};
use lk2_core::legendary::LegendaryWeapon;
use lk2_core::pvp::{Health, PvpCombatant, SimpleWeapon};
use lk2_core::world::terrain::LandformModule;
use lk2_core::world::voxel_mesh::{SurfaceNetsMesh, build_surface_nets};
use lk2_core::world::{BlockType, TERRAIN_CHUNK_SIZE, TerrainChunkCoord};

// Keep the collision grid cheap, while the presentation grid below can use a
// finer smooth heightfield. A coarser land cell must never span both land and
// submerged samples, or sand triangles will poke through the shoreline.
const PROCEDURAL_TERRAIN_RENDER_SUBDIVISIONS: i32 = 2;
const PROCEDURAL_TERRAIN_RENDER_STEP: f32 = 1.0 / PROCEDURAL_TERRAIN_RENDER_SUBDIVISIONS as f32;
const PROCEDURAL_WATER_SURFACE_LIFT: f32 = 0.025;

pub fn setup_rendering(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.47, 0.61, 0.72)));
    commands.insert_resource(DirectionalLightShadowMap { size: 2048 });
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.78, 0.86, 0.82),
        // Bevy 0.19 uses cd/m² here. Sub-unit values leave shadowed PBR
        // surfaces with no readable sky fill and turn them nearly black.
        // Keep a stronger sky fill so surface variation remains visible under
        // trees instead of collapsing into black-green shadow blobs.
        brightness: 255.0,
        affects_lightmapped_meshes: true,
    });
}

pub fn setup_lighting(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            // Cascaded maps establish the broad shadow shape; contact shadows
            // restore the short-range attachment under props and characters.
            contact_shadows_enabled: true,
            color: Color::srgb(1.0, 0.91, 0.76),
            ..default()
        },
        Transform::from_xyz(-24.0, 38.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
        CascadeShadowConfigBuilder {
            num_cascades: 3,
            minimum_distance: 0.1,
            first_cascade_far_bound: 12.0,
            maximum_distance: 70.0,
            overlap_proportion: 0.2,
        }
        .build(),
        LivingSun,
        VolumetricLight,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 5_500.0,
            shadow_maps_enabled: false,
            color: Color::srgb(0.58, 0.72, 1.0),
            ..default()
        },
        Transform::from_xyz(22.0, 24.0, -26.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

pub fn setup_terrain(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    terrain_surface: Res<ProceduralTerrainSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<StylizedTerrainMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<WaterSurfaceMaterial>>,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let ground = terrain_materials.add(stylized_terrain_material(
        // Deep woodland green keeps the scene closer to an enchanted forest
        // than a bright modern survival-game lawn.
        Color::srgb(0.25, 0.45, 0.20),
        0.66,
        0.050,
        0.94,
    ));
    let sand = terrain_materials.add(stylized_terrain_material(
        Color::srgb(0.72, 0.58, 0.34),
        0.68,
        0.045,
        0.90,
    ));
    let mountain = terrain_materials.add(stylized_terrain_material(
        Color::srgb(0.32, 0.35, 0.34),
        0.68,
        0.050,
        0.92,
    ));
    let snow = terrain_materials.add(stylized_terrain_material(
        Color::srgb(0.86, 0.91, 0.92),
        0.70,
        0.045,
        0.90,
    ));
    let cavity = terrain_materials.add(stylized_terrain_material(
        Color::srgb(0.16, 0.18, 0.15),
        0.44,
        0.032,
        0.98,
    ));
    let underwater_floor = terrain_materials.add(stylized_terrain_material(
        Color::srgb(0.24, 0.48, 0.40),
        0.48,
        0.065,
        0.92,
    ));
    if let Some(mut material) = terrain_materials.get_mut(&underwater_floor) {
        material.base.unlit = true;
        material.base.emissive = Color::srgb(0.035, 0.10, 0.075).into();
        // Shore transition quads are visible from both the water and land
        // sides. They are a thin presentation wall, not a second collider.
        material.base.cull_mode = None;
    }
    let water = water_materials.add(WaterSurfaceMaterial {
        base: StandardMaterial {
            // The seabed is now a separate, recessed mesh, so the water can
            // be translucent without fighting coplanar terrain. This keeps
            // fish and seaweed readable from above the surface.
            base_color: Color::srgba(0.08, 0.34, 0.52, 0.48),
            unlit: true,
            metallic: 0.05,
            perceptual_roughness: 0.24,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        },
        extension: WaterSurfaceExtension::default(),
    });
    let hit_effect = effects.add(create_hit_effect());
    let enemy_hit_effect = effects.add(create_enemy_hit_effect());
    let ambient_motes = effects.add(create_ambient_mote_effect());
    commands.spawn((
        ParticleEffect::new(ambient_motes),
        Transform::from_xyz(0.0, 6.0, 0.0),
        ForestRealmVisual,
        Name::new("forest_ambient_motes"),
    ));
    let tree_shadow_mesh = meshes.add(Plane3d::default().mesh().size(2.0, 2.0));
    let tree_shadow_material = materials.add(StandardMaterial {
        // Many tree decals overlap in the living scene. Keep this auxiliary
        // contact cue translucent so stacked decals never turn the terrain or
        // water into opaque black cutouts; directional-light shadows provide
        // the actual scene-wide shadowing.
        base_color: Color::srgba(0.045, 0.065, 0.040, 0.10),
        base_color_texture: Some(asset_server.load("shadow/shadow.png")),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.insert_resource(SceneMaterials {
        ground: ground.clone(),
        hit_effect,
        enemy_hit_effect,
    });
    commands.insert_resource(TreeShadowAssets {
        mesh: tree_shadow_mesh,
        material: tree_shadow_material,
    });
    spawn_procedural_terrain(
        &mut commands,
        &mut meshes,
        &terrain_surface,
        &ground,
        &sand,
        &mountain,
        &snow,
        &cavity,
        &underwater_floor,
        water,
    );
    let collision_mesh = build_collision_surface_mesh(&terrain_surface);
    let terrain_collider =
        Collider::trimesh_from_mesh_with_config(&collision_mesh, TrimeshFlags::all())
            .expect("procedural terrain collision mesh should produce a trimesh");
    commands.spawn((
        RigidBody::Static,
        terrain_collider,
        ForestRealmCollider,
        TerrainCollider,
        Name::new("procedural_terrain_collider"),
    ));
}

#[derive(Component)]
pub struct TerrainSurfacePatch(pub usize);

#[derive(Component)]
pub struct TerrainCollider;

#[derive(Component)]
pub struct TerrainCavitySurface;

#[derive(Component)]
pub struct TerrainUndergroundSurface;

#[derive(Component)]
pub struct TerrainWaterFloorSurface;

#[derive(Component)]
pub struct WaterSurfaceVisual;

pub fn rebuild_edited_terrain(
    terrain: Res<ProceduralTerrainSurface>,
    nature: Res<OfflineNature>,
    mut rebuild: ResMut<TerrainRebuildState>,
    mut underground: ResMut<TerrainUndergroundState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_queries: ParamSet<(
        Query<(&TerrainSurfacePatch, &mut Mesh3d)>,
        Query<&mut Mesh3d, With<TerrainCavitySurface>>,
        Query<&mut Mesh3d, With<TerrainUndergroundSurface>>,
        Query<&mut Mesh3d, With<TerrainWaterFloorSurface>>,
    )>,
    colliders: Query<Entity, With<TerrainCollider>>,
    mut commands: Commands,
) {
    if !rebuild.requested && !underground.requested {
        return;
    }
    rebuild.requested = false;

    let render_grid = TerrainRenderGrid::build(&terrain);
    let terrain_meshes = build_procedural_surface_meshes(
        terrain.pipeline.as_ref(),
        &terrain.landform,
        &terrain,
        &render_grid,
    );
    let water_mesh = build_procedural_water_mesh(&terrain, &render_grid);
    let underwater_floor_mesh = build_procedural_water_floor_mesh(&terrain, &render_grid);
    let mut meshes_by_patch = terrain_meshes
        .into_iter()
        .map(|mesh| meshes.add(mesh))
        .collect::<Vec<_>>();
    meshes_by_patch.push(meshes.add(water_mesh));

    for (patch, mut mesh) in terrain_queries.p0().iter_mut() {
        if let Some(handle) = meshes_by_patch.get(patch.0) {
            let old_handle = mesh.0.clone();
            mesh.0 = handle.clone();
            rebuild.retired_meshes.push(RetiredTerrainMesh {
                handle: old_handle,
                frames_remaining: TERRAIN_MESH_RETIRE_FRAMES,
            });
        }
    }

    let underwater_floor_handle = meshes.add(underwater_floor_mesh);
    for mut mesh in terrain_queries.p3().iter_mut() {
        let old_handle = mesh.0.clone();
        mesh.0 = underwater_floor_handle.clone();
        rebuild.retired_meshes.push(RetiredTerrainMesh {
            handle: old_handle,
            frames_remaining: TERRAIN_MESH_RETIRE_FRAMES,
        });
    }

    let cavity_mesh = meshes.add(build_cavity_mesh(&terrain));
    for mut mesh in terrain_queries.p1().iter_mut() {
        let old_handle = mesh.0.clone();
        mesh.0 = cavity_mesh.clone();
        rebuild.retired_meshes.push(RetiredTerrainMesh {
            handle: old_handle,
            frames_remaining: TERRAIN_MESH_RETIRE_FRAMES,
        });
    }

    if underground.requested {
        if let (Some(chunk), Some(target)) = (underground.chunk, underground.target) {
            let underground_mesh =
                meshes.add(build_underground_mesh(&terrain, &nature, chunk, target));
            for mut mesh in terrain_queries.p2().iter_mut() {
                let old_handle = mesh.0.clone();
                mesh.0 = underground_mesh.clone();
                rebuild.retired_meshes.push(RetiredTerrainMesh {
                    handle: old_handle,
                    frames_remaining: TERRAIN_MESH_RETIRE_FRAMES,
                });
            }
        }
        underground.requested = false;
    }

    for entity in &colliders {
        let collision_mesh = build_collision_surface_mesh(&terrain);
        let collider =
            Collider::trimesh_from_mesh_with_config(&collision_mesh, TrimeshFlags::all())
                .expect("edited procedural terrain collision mesh should produce a trimesh");
        commands.entity(entity).insert(collider);
    }
}

pub fn cleanup_retired_terrain_meshes(
    mut rebuild: ResMut<TerrainRebuildState>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut pending = Vec::with_capacity(rebuild.retired_meshes.len());
    for mut retired in rebuild.retired_meshes.drain(..) {
        if retired.frames_remaining == 0 {
            let _ = meshes.remove(&retired.handle);
        } else {
            retired.frames_remaining -= 1;
            pending.push(retired);
        }
    }
    rebuild.retired_meshes = pending;
}

pub fn setup_collision_debug(
    mut gizmos: ResMut<GizmoConfigStore>,
    debug: Res<CollisionDebugState>,
) {
    configure_collision_debug_gizmos(&mut gizmos, debug.enabled);
}

fn build_collision_surface_mesh(terrain: &ProceduralTerrainSurface) -> Mesh {
    // Keep every 1x1 cell covered. Water cells use the submerged terrain floor;
    // the rendered sea surface is a visual volume boundary, not a solid lid.
    // Shore cells are split at the sea plane so a triangle never interpolates
    // from a walkable land top directly into the submerged floor.
    const STEP: i32 = PROCEDURAL_COLLISION_GRID_STEP;
    let sea_y = terrain.water_surface_height();
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for z in (-PROCEDURAL_TERRAIN_RADIUS..PROCEDURAL_TERRAIN_RADIUS)
        .step_by(STEP as usize)
    {
        for x in (-PROCEDURAL_TERRAIN_RADIUS..PROCEDURAL_TERRAIN_RADIUS)
            .step_by(STEP as usize)
        {
            let corners = [
                physics_collision_vertex(terrain, x, z),
                physics_collision_vertex(terrain, x + STEP, z),
                physics_collision_vertex(terrain, x + STEP, z + STEP),
                physics_collision_vertex(terrain, x, z + STEP),
            ]
            .map(|position| SurfaceVertex {
                position,
                normal: Vec3::Y,
                shore_factor: 0.0,
            });

            let (land_polygon, land_length) = clip_surface_polygon(corners, sea_y, true);
            append_collision_polygon(&mut positions, &mut indices, land_polygon, land_length);
            let (subsea_polygon, subsea_length) = clip_surface_polygon(corners, sea_y, false);
            append_collision_polygon(&mut positions, &mut indices, subsea_polygon, subsea_length);
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(Indices::U32(indices))
}

fn terrain_cell_is_water(terrain: &ProceduralTerrainSurface, x: i32, z: i32) -> bool {
    let world_x = PROCEDURAL_TERRAIN_CENTER + x;
    let world_z = PROCEDURAL_TERRAIN_CENTER + z;
    let surface = terrain
        .pipeline
        .surface_f32(world_x, world_z)
        .unwrap_or(lk2_core::constant::SEA_LEVEL as f32 + 1.0);
    terrain.is_water_at(world_x, world_z, surface)
}

fn physics_collision_vertex(terrain: &ProceduralTerrainSurface, x: i32, z: i32) -> [f32; 3] {
    [
        x as f32,
        terrain.terrain_floor_height_at_world(
            PROCEDURAL_TERRAIN_CENTER + x,
            PROCEDURAL_TERRAIN_CENTER + z,
        ),
        z as f32,
    ]
}

fn append_collision_polygon(
    positions: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    polygon: [SurfaceVertex; 6],
    length: usize,
) {
    if length < 3 {
        return;
    }
    let base = positions.len() as u32;
    positions.extend(polygon.iter().take(length).map(|vertex| vertex.position));
    for index in 1..length - 1 {
        indices.extend([base, base + index as u32 + 1, base + index as u32]);
    }
}

fn stylized_terrain_material(
    base_color: Color,
    shadow_floor: f32,
    shadow_lift: f32,
    perceptual_roughness: f32,
) -> StylizedTerrainMaterial {
    StylizedTerrainMaterial {
        base: StandardMaterial {
            base_color,
            perceptual_roughness,
            ..default()
        },
        extension: StylizedTerrainExtension::new(shadow_floor, shadow_lift),
    }
}

fn spawn_procedural_terrain(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    terrain_surface: &ProceduralTerrainSurface,
    ground: &Handle<StylizedTerrainMaterial>,
    sand: &Handle<StylizedTerrainMaterial>,
    mountain: &Handle<StylizedTerrainMaterial>,
    snow: &Handle<StylizedTerrainMaterial>,
    cavity: &Handle<StylizedTerrainMaterial>,
    underwater_floor: &Handle<StylizedTerrainMaterial>,
    water: Handle<WaterSurfaceMaterial>,
) {
    let pipeline = terrain_surface.pipeline.as_ref();
    let landform = &terrain_surface.landform;
    let render_grid = TerrainRenderGrid::build(terrain_surface);
    let terrain_meshes =
        build_procedural_surface_meshes(pipeline, landform, terrain_surface, &render_grid);
    let water_mesh = build_procedural_water_mesh(terrain_surface, &render_grid);
    let underwater_floor_mesh = build_procedural_water_floor_mesh(terrain_surface, &render_grid);

    for (index, (mesh, material, name)) in [
        (
            terrain_meshes[0].clone(),
            ground.clone(),
            "procedural_grass_surface",
        ),
        (
            terrain_meshes[1].clone(),
            sand.clone(),
            "procedural_sand_surface",
        ),
        (
            terrain_meshes[2].clone(),
            mountain.clone(),
            "procedural_mountain_surface",
        ),
        (
            terrain_meshes[3].clone(),
            snow.clone(),
            "procedural_snow_surface",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            TerrainSurfacePatch(index),
            ForestRealmVisual,
            Name::new(name),
        ));
    }
    commands.spawn((
        Mesh3d(meshes.add(water_mesh)),
        MeshMaterial3d(water),
        TerrainSurfacePatch(4),
        WaterSurfaceVisual,
        ForestRealmVisual,
        Name::new("procedural_water_surface"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(underwater_floor_mesh)),
        MeshMaterial3d(underwater_floor.clone()),
        TerrainWaterFloorSurface,
        ForestRealmVisual,
        Name::new("procedural_underwater_floor"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(build_cavity_mesh(terrain_surface))),
        MeshMaterial3d(cavity.clone()),
        TerrainCavitySurface,
        ForestRealmVisual,
        Name::new("procedural_terrain_cavities"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(empty_mesh())),
        MeshMaterial3d(cavity.clone()),
        TerrainUndergroundSurface,
        ForestRealmVisual,
        Name::new("procedural_terrain_underground"),
    ));
}

fn empty_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    // Keep an allocated, invisible placeholder instead of zero-sized GPU
    // buffers. Bevy 0.19's mesh slab allocator can otherwise report a
    // use-after-free while extracting the empty underground surface.
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0, 0.0, 0.0]])
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]])
}

fn build_underground_mesh(
    terrain: &ProceduralTerrainSurface,
    nature: &OfflineNature,
    chunk: TerrainChunkCoord,
    target: [i32; 3],
) -> Mesh {
    const CELLS: i32 = TERRAIN_CHUNK_SIZE;
    const HALF_WINDOW: i32 = CELLS / 2;
    let chunk_origin = chunk.origin();
    let origin = [
        target[0] - HALF_WINDOW,
        chunk_origin[1],
        target[2] - HALF_WINDOW,
    ];
    let radius = MINE_SURFACE_DEFORMATION_RADIUS + 1.15;
    let center_x = target[0] as f32;
    let center_z = target[2] as f32;

    // The chunk identifies the authoritative vertical neighborhood. The mesh
    // window is centered on the mined cell so a dig near a chunk edge still
    // receives its complete smooth wall and does not create a square seam.
    let nets = build_surface_nets(CELLS, |x, y, z| {
        let world_x = origin[0] + x;
        let world_y = origin[1] + y;
        let world_z = origin[2] + z;
        match nature.terrain_world.get(world_x, world_y, world_z) {
            BlockType::Air | BlockType::Water => -1.0,
            _ => 1.0,
        }
    });
    let nets = trim_surface_nets_to_radius(nets, origin, center_x, center_z, radius);
    surface_nets_to_mesh(nets, origin, terrain)
}

fn trim_surface_nets_to_radius(
    nets: SurfaceNetsMesh,
    origin: [i32; 3],
    center_x: f32,
    center_z: f32,
    radius: f32,
) -> SurfaceNetsMesh {
    let mut remap = vec![u32::MAX; nets.positions.len()];
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for triangle in nets.indices.chunks_exact(3) {
        let centroid = triangle.iter().fold(Vec3::ZERO, |sum, index| {
            sum + Vec3::from_array(nets.positions[*index as usize])
        }) / 3.0;
        let world_distance =
            Vec2::new(origin[0] as f32 + centroid.x, origin[2] as f32 + centroid.z)
                .distance(Vec2::new(center_x, center_z));
        if world_distance > radius {
            continue;
        }
        for index in triangle {
            let old_index = *index as usize;
            if remap[old_index] == u32::MAX {
                remap[old_index] = positions.len() as u32;
                positions.push(nets.positions[old_index]);
            }
            indices.push(remap[old_index]);
        }
    }
    SurfaceNetsMesh { positions, indices }
}

fn surface_nets_to_mesh(
    nets: SurfaceNetsMesh,
    origin: [i32; 3],
    terrain: &ProceduralTerrainSurface,
) -> Mesh {
    if nets.positions.is_empty() || nets.indices.is_empty() {
        return empty_mesh();
    }

    let mut positions = Vec::with_capacity(nets.positions.len());
    for [x, y, z] in &nets.positions {
        let world_x = origin[0] as f32 + *x;
        let world_y = origin[1] as f32 + *y;
        let world_z = origin[2] as f32 + *z;
        positions.push([
            world_x - PROCEDURAL_TERRAIN_CENTER as f32,
            terrain.render_height(world_y),
            world_z - PROCEDURAL_TERRAIN_CENTER as f32,
        ]);
    }

    let mut normals = vec![Vec3::ZERO; positions.len()];
    for triangle in nets.indices.chunks_exact(3) {
        let [a, b, c] = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        let normal = (Vec3::from_array(positions[b]) - Vec3::from_array(positions[a]))
            .cross(Vec3::from_array(positions[c]) - Vec3::from_array(positions[a]));
        normals[a] += normal;
        normals[b] += normal;
        normals[c] += normal;
    }
    let normals = normals
        .into_iter()
        .map(|normal| normal.normalize_or_zero().to_array())
        .collect::<Vec<_>>();
    let uvs = positions
        .iter()
        .map(|position| [position[0] * 0.18, position[2] * 0.18])
        .collect::<Vec<_>>();

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(nets.indices))
}

fn build_procedural_surface_meshes(
    pipeline: &lk2_core::world::terrain::TerrainPipeline,
    landform: &LandformModule,
    terrain_surface: &ProceduralTerrainSurface,
    render_grid: &TerrainRenderGrid,
) -> [Mesh; 4] {
    let mut data = std::array::from_fn(|_| SurfaceMeshData::default());
    let sea_y = terrain_surface.render_height(lk2_core::constant::SEA_LEVEL as f32);
    let category_width = (PROCEDURAL_TERRAIN_RADIUS * 2 + 1) as usize;
    let mut categories = vec![0usize; category_width * category_width];
    for z in 0..category_width {
        for x in 0..category_width {
            let world_x = PROCEDURAL_TERRAIN_CENTER - PROCEDURAL_TERRAIN_RADIUS + x as i32;
            let world_z = PROCEDURAL_TERRAIN_CENTER - PROCEDURAL_TERRAIN_RADIUS + z as i32;
            let surface = pipeline
                .surface_f32(world_x, world_z)
                .unwrap_or(lk2_core::constant::SEA_LEVEL as f32 + 1.0);
            categories[z * category_width + x] =
                terrain_surface_category(surface, landform, world_x, world_z);
        }
    }
    let cell_count =
        (PROCEDURAL_TERRAIN_RADIUS * 2 * PROCEDURAL_TERRAIN_RENDER_SUBDIVISIONS) as usize;
    for z_index in 0..cell_count {
        for x_index in 0..cell_count {
            let local_x = render_grid.local_coordinate(x_index);
            let local_z = render_grid.local_coordinate(z_index);
            let cell_center = Vec2::new(
                local_x + PROCEDURAL_TERRAIN_RENDER_STEP * 0.5,
                local_z + PROCEDURAL_TERRAIN_RENDER_STEP * 0.5,
            );
            if terrain_surface.deformations.iter().any(|edit| {
                edit.center
                    .distance(cell_center + Vec2::splat(PROCEDURAL_TERRAIN_CENTER as f32))
                    < edit.radius * 0.9
            }) {
                continue;
            }
            let corners = [
                (x_index, z_index),
                (x_index + 1, z_index),
                (x_index + 1, z_index + 1),
                (x_index, z_index + 1),
            ];
            let heights = corners.map(|(x, z)| render_grid.height(x, z));
            let normals = corners.map(|(x, z)| render_grid.normal(x, z));
            let land_corner = heights
                .iter()
                .enumerate()
                .find(|(_, height)| **height > sea_y);
            let Some((corner, _)) = land_corner else {
                // The water mesh owns fully submerged cells. Keeping a land
                // quad here would create coplanar geometry and visible seams.
                continue;
            };
            let (category_grid_x, category_grid_z) = corners[corner];
            let category_x = category_grid_x / PROCEDURAL_TERRAIN_RENDER_SUBDIVISIONS as usize;
            let category_z = category_grid_z / PROCEDURAL_TERRAIN_RENDER_SUBDIVISIONS as usize;
            let category = categories[category_z * category_width + category_x];
            // Clip mixed cells against the sea plane. This removes the long
            // triangular teeth caused by rendering an entire land quad when
            // only one of its corners was above water.
            data[category].push_clipped_land_cell(
                [local_x, local_z],
                PROCEDURAL_TERRAIN_RENDER_STEP,
                heights,
                normals,
                sea_y,
            );
        }
    }
    data.map(SurfaceMeshData::into_mesh)
}

fn build_cavity_mesh(terrain: &ProceduralTerrainSurface) -> Mesh {
    const SEGMENTS: usize = 24;
    const VISUAL_DEPTH_SCALE: f32 = 1.8;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for deformation in &terrain.deformations {
        let base = positions.len() as u32;
        let local_center = deformation.center - Vec2::splat(PROCEDURAL_TERRAIN_CENTER as f32);
        let center_height = terrain.ground_height(Vec3::new(local_center.x, 0.0, local_center.y));
        let bottom_height = center_height - deformation.depth * 0.14 * VISUAL_DEPTH_SCALE - 0.035;
        let bottom_radius = deformation.radius * 0.58;

        for i in 0..SEGMENTS {
            let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            let outer = local_center + Vec2::new(cos, sin) * deformation.radius;
            let inner = local_center + Vec2::new(cos, sin) * bottom_radius;
            let top_height = terrain.ground_height(Vec3::new(outer.x, 0.0, outer.y)) - 0.015;
            positions.extend([
                [outer.x, top_height, outer.y],
                [inner.x, bottom_height, inner.y],
            ]);
            let normal = [cos, 0.18, sin];
            normals.extend([normal, normal]);
            uvs.extend([
                [i as f32 / SEGMENTS as f32, 0.0],
                [i as f32 / SEGMENTS as f32, 1.0],
            ]);
        }

        let center_index = positions.len() as u32;
        positions.push([local_center.x, bottom_height, local_center.y]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.5, 0.5]);
        for i in 0..SEGMENTS {
            let next = (i + 1) % SEGMENTS;
            let bottom = base + (i * 2 + 1) as u32;
            let next_bottom = base + (next * 2 + 1) as u32;
            indices.extend([center_index, next_bottom, bottom]);
        }

        for i in 0..SEGMENTS {
            let next = (i + 1) % SEGMENTS;
            let top = base + (i * 2) as u32;
            let bottom = top + 1;
            let next_top = base + (next * 2) as u32;
            let next_bottom = next_top + 1;
            indices.extend([top, next_top, next_bottom, top, next_bottom, bottom]);
        }
    }

    if positions.is_empty() || indices.is_empty() {
        return empty_mesh();
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn build_procedural_water_mesh(
    terrain_surface: &ProceduralTerrainSurface,
    render_grid: &TerrainRenderGrid,
) -> Mesh {
    let mut data = SurfaceMeshData::default();
    let sea_level_y = terrain_surface.render_height(lk2_core::constant::SEA_LEVEL as f32);
    let sea_y = sea_level_y + PROCEDURAL_WATER_SURFACE_LIFT;
    let cell_count =
        (PROCEDURAL_TERRAIN_RADIUS * 2 * PROCEDURAL_TERRAIN_RENDER_SUBDIVISIONS) as usize;
    for z_index in 0..cell_count {
        for x_index in 0..cell_count {
            let corners = [
                (x_index, z_index),
                (x_index + 1, z_index),
                (x_index + 1, z_index + 1),
                (x_index, z_index + 1),
            ];
            let heights = corners.map(|(x, z)| render_grid.height(x, z));
            if heights.iter().any(|height| *height <= sea_level_y) {
                data.push_clipped_water_cell(
                    [
                        render_grid.local_coordinate(x_index),
                        render_grid.local_coordinate(z_index),
                    ],
                    PROCEDURAL_TERRAIN_RENDER_STEP,
                    heights,
                    sea_level_y,
                    sea_y,
                );
            }
        }
    }
    data.into_mesh()
}

/// Render the submerged terrain floor separately from the sea surface.
///
/// Fully submerged cells are intentionally omitted from the land surface
/// mesh, and the collision mesh is not rendered. Without this presentation
/// layer a swimming camera sees an empty blue void below the water line.
fn build_procedural_water_floor_mesh(
    terrain_surface: &ProceduralTerrainSurface,
    render_grid: &TerrainRenderGrid,
) -> Mesh {
    let cell_count =
        (PROCEDURAL_TERRAIN_RADIUS * 2 * PROCEDURAL_TERRAIN_RENDER_SUBDIVISIONS) as usize;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for z_index in 0..cell_count {
        for x_index in 0..cell_count {
            let x = render_grid.local_coordinate(x_index);
            let z = render_grid.local_coordinate(z_index);
            // The render grid is finer than the gameplay water query. Use the
            // cell center as the ownership test so a shoreline corner cannot
            // make the submerged floor spill across a land cell. The clipped
            // polygon below still preserves the water-side part of a genuine
            // shoreline cell.
            let center = Vec3::new(
                x + PROCEDURAL_TERRAIN_RENDER_STEP * 0.5,
                0.0,
                z + PROCEDURAL_TERRAIN_RENDER_STEP * 0.5,
            );
            let Some(_depth) = water_floor_cell_depth(terrain_surface, center) else {
                continue;
            };
            let corners = [
                Vec3::new(x, 0.0, z),
                Vec3::new(x + PROCEDURAL_TERRAIN_RENDER_STEP, 0.0, z),
                Vec3::new(
                    x + PROCEDURAL_TERRAIN_RENDER_STEP,
                    0.0,
                    z + PROCEDURAL_TERRAIN_RENDER_STEP,
                ),
                Vec3::new(x, 0.0, z + PROCEDURAL_TERRAIN_RENDER_STEP),
            ];
            let sea_y = terrain_surface.water_surface_height();
            let top_heights = [
                render_grid.height(x_index, z_index),
                render_grid.height(x_index + 1, z_index),
                render_grid.height(x_index + 1, z_index + 1),
                render_grid.height(x_index, z_index + 1),
            ];

            // A center sample can be on land while one or more corners are
            // submerged. Skipping such a cell leaves clear-colour triangular
            // holes between the water surface and the seabed. Use the same
            // render-grid boundary as the land and water meshes, then clip
            // the floor polygon below the sea plane.
            if top_heights.iter().all(|height| *height > sea_y) {
                continue;
            }

            let floor_corners = corners.map(|corner| SurfaceVertex {
                position: [
                    corner.x,
                    terrain_surface.terrain_floor_height(corner),
                    corner.z,
                ],
                normal: Vec3::Y,
                shore_factor: 0.0,
            });
            let floor_heights = floor_corners.map(|corner| corner.position[1]);
            let xz_corners = corners.map(|corner| [corner.x, corner.z]);
            let (shore_points, shore_length) =
                collect_shoreline_points(xz_corners, top_heights, floor_heights, sea_y);
            append_shore_wall(
                &mut positions,
                &mut normals,
                &mut uvs,
                &mut indices,
                shore_points,
                shore_length,
                sea_y,
            );
            let (polygon, length) = clip_surface_polygon(floor_corners, sea_y, false);
            if length < 3 {
                continue;
            }

            let base = positions.len() as u32;
            for vertex in polygon.iter().take(length) {
                let position = [
                    vertex.position[0],
                    vertex.position[1] + 0.008,
                    vertex.position[2],
                ];
                positions.push(position);
                normals.push(Vec3::Y.to_array());
                uvs.push([position[0] * 0.16, position[2] * 0.16]);
            }
            for index in 1..length - 1 {
                indices.extend([base, base + index as u32 + 1, base + index as u32]);
            }
        }
    }

    if positions.is_empty() {
        return empty_mesh();
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn water_floor_cell_depth(terrain: &ProceduralTerrainSurface, center: Vec3) -> Option<f32> {
    terrain
        .water_depth_at(center)
        .filter(|depth| *depth >= 0.05)
}

fn terrain_surface_category(surface: f32, landform: &LandformModule, x: i32, z: i32) -> usize {
    if surface < lk2_core::constant::SEA_LEVEL as f32 || landform.river_factor(x, z) > 0.42 {
        1
    } else if surface > lk2_core::constant::SEA_LEVEL as f32 + 25.0 {
        3
    } else if landform.mountain_factor(x, z) > 0.45 {
        2
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    #[test]
    fn terrain_collision_mesh_covers_every_cell_including_water() {
        let terrain = ProceduralTerrainSurface::default_world();
        let mesh = build_collision_surface_mesh(&terrain);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("terrain collision mesh should contain float3 positions");
        };

        let cells_per_axis = (PROCEDURAL_TERRAIN_RADIUS * 2) as usize;
        assert!(
            positions.len() >= cells_per_axis * cells_per_axis * 4,
            "collision clipping should preserve coverage while allowing extra shore vertices"
        );
        let water_y = terrain.water_surface_height();
        assert!(
            positions
                .iter()
                .any(|position| position[1] < water_y - 0.01),
            "terrain collision mesh should contain submerged floor vertices"
        );
    }

    #[test]
    fn underwater_floor_mesh_contains_renderable_submerged_geometry() {
        let terrain = ProceduralTerrainSurface::default_world();
        let render_grid = TerrainRenderGrid::build(&terrain);
        let mesh = build_procedural_water_floor_mesh(&terrain, &render_grid);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("underwater floor should contain float3 positions");
        };
        assert!(!positions.is_empty());
        assert!(
            positions
                .iter()
                .any(|position| position[1] < terrain.water_surface_height() - 0.02)
        );
    }

    #[test]
    fn underwater_floor_mesh_does_not_bridge_above_the_sea_at_shore() {
        let terrain = ProceduralTerrainSurface::default_world();
        let render_grid = TerrainRenderGrid::build(&terrain);
        let mesh = build_procedural_water_floor_mesh(&terrain, &render_grid);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("underwater floor should contain float3 positions");
        };

        let sea_y = terrain.water_surface_height();
        assert!(
            positions.iter().all(|position| position[1] <= sea_y + 0.01),
            "underwater floor crossed the sea plane: sea_y={sea_y}, max_y={:?}",
            positions
                .iter()
                .map(|position| position[1])
                .fold(f32::NEG_INFINITY, f32::max)
        );
    }

    #[test]
    fn underwater_floor_mesh_has_vertical_shore_transition_geometry() {
        let terrain = ProceduralTerrainSurface::default_world();
        let render_grid = TerrainRenderGrid::build(&terrain);
        let mesh = build_procedural_water_floor_mesh(&terrain, &render_grid);
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("underwater floor should contain float3 normals");
        };

        assert!(
            normals.iter().any(|normal| normal[1].abs() < 0.2),
            "underwater floor should include side normals where the seabed meets the shore"
        );
    }

    #[test]
    fn underwater_floor_cell_ownership_rejects_land_centers() {
        let terrain = ProceduralTerrainSurface::default_world();
        let render_grid = TerrainRenderGrid::build(&terrain);
        let mut saw_mixed_shore_cell = false;

        for z_index in 0..render_grid.width - 1 {
            for x_index in 0..render_grid.width - 1 {
                let x = render_grid.local_coordinate(x_index);
                let z = render_grid.local_coordinate(z_index);
                let center = Vec3::new(
                    x + PROCEDURAL_TERRAIN_RENDER_STEP * 0.5,
                    0.0,
                    z + PROCEDURAL_TERRAIN_RENDER_STEP * 0.5,
                );
                if water_floor_cell_depth(&terrain, center).is_some() {
                    continue;
                }

                let corners = [
                    Vec3::new(x, 0.0, z),
                    Vec3::new(x + PROCEDURAL_TERRAIN_RENDER_STEP, 0.0, z),
                    Vec3::new(
                        x + PROCEDURAL_TERRAIN_RENDER_STEP,
                        0.0,
                        z + PROCEDURAL_TERRAIN_RENDER_STEP,
                    ),
                    Vec3::new(x, 0.0, z + PROCEDURAL_TERRAIN_RENDER_STEP),
                ];
                if corners
                    .iter()
                    .any(|corner| terrain.water_depth_at(*corner).is_some())
                {
                    saw_mixed_shore_cell = true;
                    break;
                }
            }
            if saw_mixed_shore_cell {
                break;
            }
        }

        assert!(
            saw_mixed_shore_cell,
            "default terrain should contain a mixed shoreline render cell"
        );
    }

    #[test]
    fn terrain_collision_triangles_do_not_connect_land_to_subsea_floor() {
        let terrain = ProceduralTerrainSurface::default_world();
        let mesh = build_collision_surface_mesh(&terrain);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("terrain collision mesh should contain float3 positions");
        };
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("terrain collision mesh should contain indices");
        };

        let sea_y = terrain.water_surface_height();
        for triangle in indices.chunks_exact(3) {
            let has_land = triangle
                .iter()
                .any(|index| positions[*index as usize][1] > sea_y + 0.01);
            let has_subsea = triangle
                .iter()
                .any(|index| positions[*index as usize][1] < sea_y - 0.01);
            assert!(
                !(has_land && has_subsea),
                "collision triangle bridged land and subsea floor around sea_y={sea_y}"
            );
        }
    }

    #[test]
    fn terrain_surface_vertex_colors_preserve_material_tint() {
        let mut data = SurfaceMeshData::default();
        data.push_clipped_land_cell([0.0, 0.0], 1.0, [1.0; 4], [Vec3::Y; 4], 0.0);
        let mesh = data.into_mesh();
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("terrain surface should contain float4 vertex colors");
        };

        assert!(
            colors.iter().all(|color| color[..3] == [1.0; 3]),
            "vertex RGB must stay white so StandardMaterial keeps its authored base color"
        );
    }

    #[test]
    fn water_surface_keeps_shore_factor_out_of_material_rgb() {
        let mut data = SurfaceMeshData::default();
        data.push_clipped_water_cell([0.0, 0.0], 1.0, [-0.2, -0.6, -1.2, -1.8], 0.0, 0.025);
        let mesh = data.into_mesh();
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("water surface should contain float4 vertex colors");
        };

        assert!(colors.iter().all(|color| color[..3] == [1.0; 3]));
        assert!(
            colors.iter().any(|color| color[3] < 1.0),
            "shore metadata should remain available in the unused opaque alpha lane"
        );
    }

    #[test]
    fn water_floor_is_below_surface_and_reports_depth() {
        let terrain = ProceduralTerrainSurface::default_world();
        let (x, z) = (0..96)
            .flat_map(|z| (0..96).map(move |x| (x, z)))
            .find(|&(x, z)| {
                terrain
                    .pipeline
                    .surface_f32(x, z)
                    .is_some_and(|surface| terrain.is_water_at(x, z, surface))
            })
            .expect("default terrain should contain a water cell");
        let position = Vec3::new(
            x as f32 - PROCEDURAL_TERRAIN_CENTER as f32 + 0.25,
            0.0,
            z as f32 - PROCEDURAL_TERRAIN_CENTER as f32 + 0.25,
        );

        let depth = terrain
            .water_depth_at(position)
            .expect("water cell should report a depth");

        assert!(depth > 0.0);
        assert!(terrain.water_floor_height(position) < terrain.water_surface_height());
    }

    #[test]
    fn default_water_leaves_room_for_a_submerged_player() {
        let terrain = ProceduralTerrainSurface::default_world();
        let deepest = (0..96)
            .flat_map(|z| (0..96).map(move |x| (x, z)))
            .filter_map(|(x, z)| {
                let position = Vec3::new(
                    x as f32 - PROCEDURAL_TERRAIN_CENTER as f32 + 0.25,
                    0.0,
                    z as f32 - PROCEDURAL_TERRAIN_CENTER as f32 + 0.25,
                );
                terrain.water_depth_at(position)
            })
            .fold(0.0, f32::max);

        assert!(
            deepest >= 2.0,
            "default water must leave a real dive volume, got depth {deepest}"
        );
    }

    #[test]
    fn terrain_deformation_lowers_the_shared_surface_query() {
        let mut terrain = ProceduralTerrainSurface::default_world();
        let x = PROCEDURAL_TERRAIN_CENTER;
        let z = PROCEDURAL_TERRAIN_CENTER;
        let before = terrain.ground_height_at_world(x, z);

        terrain.dig_at(Vec3::ZERO, 2.0, 0.9);

        let after = terrain.ground_height_at_world(x, z);
        assert!(
            after < before,
            "digging should lower the shared terrain query"
        );
    }

    #[test]
    fn mined_column_anchor_rebuilds_the_visible_surface() {
        let mut terrain = ProceduralTerrainSurface::default_world();
        let before_grid = TerrainRenderGrid::build(&terrain);
        let center = before_grid.width / 2;
        let before_height = before_grid.height(center, center);
        let before_meshes = build_procedural_surface_meshes(
            terrain.pipeline.as_ref(),
            &terrain.landform,
            &terrain,
            &before_grid,
        );
        let before_vertex_count = before_meshes
            .iter()
            .filter_map(|mesh| mesh.attribute(Mesh::ATTRIBUTE_POSITION))
            .map(|attribute| match attribute {
                VertexAttributeValues::Float32x3(values) => values.len(),
                _ => 0,
            })
            .sum::<usize>();

        terrain.dig_at_world_column(
            PROCEDURAL_TERRAIN_CENTER,
            PROCEDURAL_TERRAIN_CENTER,
            1.8,
            1.0,
        );

        let after_grid = TerrainRenderGrid::build(&terrain);
        assert!(
            after_grid.height(center, center) < before_height,
            "the render grid must sample the mined column deformation"
        );
        let after_meshes = build_procedural_surface_meshes(
            terrain.pipeline.as_ref(),
            &terrain.landform,
            &terrain,
            &after_grid,
        );
        let after_vertex_count = after_meshes
            .iter()
            .filter_map(|mesh| mesh.attribute(Mesh::ATTRIBUTE_POSITION))
            .map(|attribute| match attribute {
                VertexAttributeValues::Float32x3(values) => values.len(),
                _ => 0,
            })
            .sum::<usize>();
        assert!(
            after_vertex_count < before_vertex_count,
            "rebuilding after a mine must remove the surface cells over the edited column"
        );
    }
}

struct TerrainRenderGrid {
    width: usize,
    heights: Vec<f32>,
    sample_origin: i32,
    sample_width: usize,
    sample_heights: Vec<f32>,
}

impl TerrainRenderGrid {
    fn build(terrain: &ProceduralTerrainSurface) -> Self {
        let width =
            (PROCEDURAL_TERRAIN_RADIUS * 2 * PROCEDURAL_TERRAIN_RENDER_SUBDIVISIONS + 1) as usize;
        const SAMPLE_BORDER: i32 = 2;
        let sample_origin = -PROCEDURAL_TERRAIN_RADIUS - SAMPLE_BORDER;
        let sample_width = (PROCEDURAL_TERRAIN_RADIUS * 2 + 1 + SAMPLE_BORDER * 2) as usize;
        let mut sample_heights = Vec::with_capacity(sample_width * sample_width);
        for z in 0..sample_width {
            for x in 0..sample_width {
                let local_x = sample_origin + x as i32;
                let local_z = sample_origin + z as i32;
                sample_heights.push(terrain.ground_height_at_world(
                    PROCEDURAL_TERRAIN_CENTER + local_x,
                    PROCEDURAL_TERRAIN_CENTER + local_z,
                ));
            }
        }

        let mut grid = Self {
            width,
            heights: vec![0.0; width * width],
            sample_origin,
            sample_width,
            sample_heights,
        };
        for z in 0..width {
            let local_z =
                -PROCEDURAL_TERRAIN_RADIUS as f32 + z as f32 * PROCEDURAL_TERRAIN_RENDER_STEP;
            for x in 0..width {
                let local_x =
                    -PROCEDURAL_TERRAIN_RADIUS as f32 + x as f32 * PROCEDURAL_TERRAIN_RENDER_STEP;
                grid.heights[z * width + x] = grid.smooth_height(local_x, local_z);
            }
        }
        grid
    }

    fn local_coordinate(&self, index: usize) -> f32 {
        -PROCEDURAL_TERRAIN_RADIUS as f32 + index as f32 * PROCEDURAL_TERRAIN_RENDER_STEP
    }

    fn height(&self, x: usize, z: usize) -> f32 {
        self.heights[z * self.width + x]
    }

    fn integer_height(&self, x: i32, z: i32) -> f32 {
        let max = self.sample_origin + self.sample_width as i32 - 1;
        let x = x.clamp(self.sample_origin, max);
        let z = z.clamp(self.sample_origin, max);
        let x = (x - self.sample_origin) as usize;
        let z = (z - self.sample_origin) as usize;
        self.sample_heights[z * self.sample_width + x]
    }

    fn smooth_height(&self, local_x: f32, local_z: f32) -> f32 {
        let x0 = local_x.floor() as i32;
        let z0 = local_z.floor() as i32;
        let tx = local_x - x0 as f32;
        let tz = local_z - z0 as f32;
        let mut rows = [0.0; 4];
        for (row, offset_z) in (-1..=2).enumerate() {
            rows[row] = cubic_interpolate(
                [
                    self.integer_height(x0 - 1, z0 + offset_z),
                    self.integer_height(x0, z0 + offset_z),
                    self.integer_height(x0 + 1, z0 + offset_z),
                    self.integer_height(x0 + 2, z0 + offset_z),
                ],
                tx,
            );
        }
        let value = cubic_interpolate(rows, tz);
        let central = [
            self.integer_height(x0, z0),
            self.integer_height(x0 + 1, z0),
            self.integer_height(x0 + 1, z0 + 1),
            self.integer_height(x0, z0 + 1),
        ];
        let min_height = central.into_iter().fold(f32::INFINITY, f32::min);
        let max_height = central.into_iter().fold(f32::NEG_INFINITY, f32::max);
        value.clamp(min_height, max_height)
    }

    fn normal(&self, x: usize, z: usize) -> Vec3 {
        let max_index = self.width - 1;
        let left_x = x.saturating_sub(1);
        let right_x = (x + 1).min(max_index);
        let back_z = z.saturating_sub(1);
        let front_z = (z + 1).min(max_index);
        let dx = ((right_x - left_x) as f32 * PROCEDURAL_TERRAIN_RENDER_STEP).max(0.001);
        let dz = ((front_z - back_z) as f32 * PROCEDURAL_TERRAIN_RENDER_STEP).max(0.001);
        let slope_x = (self.height(right_x, z) - self.height(left_x, z)) / dx;
        let slope_z = (self.height(x, front_z) - self.height(x, back_z)) / dz;
        Vec3::new(-slope_x, 1.0, -slope_z).normalize_or_zero()
    }
}

fn cubic_interpolate(samples: [f32; 4], t: f32) -> f32 {
    let [p0, p1, p2, p3] = samples;
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

#[derive(Clone, Copy, Default)]
struct SurfaceVertex {
    position: [f32; 3],
    normal: Vec3,
    shore_factor: f32,
}

#[derive(Clone, Copy, Default)]
struct ShorePoint {
    x: f32,
    z: f32,
    floor_y: f32,
}

fn collect_shoreline_points(
    xz_corners: [[f32; 2]; 4],
    top_heights: [f32; 4],
    floor_heights: [f32; 4],
    sea_y: f32,
) -> ([ShorePoint; 2], usize) {
    let mut points = [ShorePoint::default(); 2];
    let mut length = 0;
    for index in 0..4 {
        let next = (index + 1) % 4;
        let current_land = top_heights[index] > sea_y;
        let next_land = top_heights[next] > sea_y;
        if current_land == next_land || length == points.len() {
            continue;
        }

        let denominator = top_heights[next] - top_heights[index];
        let t = if denominator.abs() > f32::EPSILON {
            ((sea_y - top_heights[index]) / denominator).clamp(0.0, 1.0)
        } else {
            0.0
        };
        points[length] = ShorePoint {
            x: xz_corners[index][0] + (xz_corners[next][0] - xz_corners[index][0]) * t,
            z: xz_corners[index][1] + (xz_corners[next][1] - xz_corners[index][1]) * t,
            floor_y: floor_heights[index] + (floor_heights[next] - floor_heights[index]) * t,
        };
        length += 1;
    }
    (points, length)
}

fn append_shore_wall(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    points: [ShorePoint; 2],
    length: usize,
    sea_y: f32,
) {
    if length < 2 {
        return;
    }
    let [start, end] = points;
    let bottom_start = (start.floor_y + 0.008).min(sea_y - 0.008);
    let bottom_end = (end.floor_y + 0.008).min(sea_y - 0.008);
    if bottom_start >= sea_y - 0.001 && bottom_end >= sea_y - 0.001 {
        return;
    }

    let edge = Vec2::new(end.x - start.x, end.z - start.z);
    let normal = Vec3::new(-edge.y, 0.0, edge.x)
        .normalize_or_zero()
        .to_array();
    let base = positions.len() as u32;
    positions.extend([
        [start.x, sea_y + 0.002, start.z],
        [end.x, sea_y + 0.002, end.z],
        [end.x, bottom_end, end.z],
        [start.x, bottom_start, start.z],
    ]);
    normals.extend([normal; 4]);
    uvs.extend([
        [start.x * 0.16, 0.0],
        [end.x * 0.16, 0.0],
        [end.x * 0.16, 1.0],
        [start.x * 0.16, 1.0],
    ]);
    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn clip_surface_polygon(
    corners: [SurfaceVertex; 4],
    threshold: f32,
    keep_above: bool,
) -> ([SurfaceVertex; 6], usize) {
    let mut polygon = [SurfaceVertex::default(); 6];
    let mut length = 0;
    for index in 0..corners.len() {
        let current = corners[index];
        let next = corners[(index + 1) % corners.len()];
        let current_inside = if keep_above {
            current.position[1] >= threshold
        } else {
            current.position[1] <= threshold
        };
        let next_inside = if keep_above {
            next.position[1] >= threshold
        } else {
            next.position[1] <= threshold
        };

        if current_inside != next_inside {
            let denominator = next.position[1] - current.position[1];
            let t = if denominator.abs() > f32::EPSILON {
                ((threshold - current.position[1]) / denominator).clamp(0.0, 1.0)
            } else {
                0.0
            };
            polygon[length] = SurfaceVertex {
                position: [
                    current.position[0] + (next.position[0] - current.position[0]) * t,
                    threshold,
                    current.position[2] + (next.position[2] - current.position[2]) * t,
                ],
                normal: current.normal.lerp(next.normal, t).normalize_or_zero(),
                shore_factor: current.shore_factor + (next.shore_factor - current.shore_factor) * t,
            };
            length += 1;
        }
        if next_inside {
            polygon[length] = next;
            length += 1;
        }
    }
    (polygon, length)
}

#[derive(Default)]
struct SurfaceMeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl SurfaceMeshData {
    fn push_clipped_land_cell(
        &mut self,
        origin: [f32; 2],
        size: f32,
        heights: [f32; 4],
        normals: [Vec3; 4],
        sea_y: f32,
    ) {
        let [x, z] = origin;
        let corners = [
            SurfaceVertex {
                position: [x, heights[0], z],
                normal: normals[0],
                shore_factor: 0.0,
            },
            SurfaceVertex {
                position: [x + size, heights[1], z],
                normal: normals[1],
                shore_factor: 0.0,
            },
            SurfaceVertex {
                position: [x + size, heights[2], z + size],
                normal: normals[2],
                shore_factor: 0.0,
            },
            SurfaceVertex {
                position: [x, heights[3], z + size],
                normal: normals[3],
                shore_factor: 0.0,
            },
        ];
        let (polygon, length) = clip_surface_polygon(corners, sea_y, true);
        if length < 3 {
            return;
        }

        let base = self.positions.len() as u32;
        for vertex in polygon.iter().take(length) {
            self.positions.push(vertex.position);
            self.normals.push(vertex.normal.to_array());
            // StandardMaterial multiplies its base color by vertex RGB before
            // our material extension runs. White preserves the authored
            // terrain palette; black here makes the whole surface black.
            self.colors.push([1.0, 1.0, 1.0, 1.0]);
            self.uvs
                .push([vertex.position[0] * 0.18, vertex.position[2] * 0.18]);
        }
        for index in 1..length - 1 {
            // The surface mesh uses clockwise x/z winding for an upward
            // normal, matching the old quad path.
            self.indices
                .extend([base, base + index as u32 + 1, base + index as u32]);
        }
    }

    fn push_clipped_water_cell(
        &mut self,
        origin: [f32; 2],
        size: f32,
        heights: [f32; 4],
        sea_level_y: f32,
        water_y: f32,
    ) {
        let [x, z] = origin;
        let corners = [
            SurfaceVertex {
                position: [x, heights[0], z],
                normal: Vec3::Y,
                shore_factor: ((sea_level_y - heights[0]) / 1.8).clamp(0.0, 1.0),
            },
            SurfaceVertex {
                position: [x + size, heights[1], z],
                normal: Vec3::Y,
                shore_factor: ((sea_level_y - heights[1]) / 1.8).clamp(0.0, 1.0),
            },
            SurfaceVertex {
                position: [x + size, heights[2], z + size],
                normal: Vec3::Y,
                shore_factor: ((sea_level_y - heights[2]) / 1.8).clamp(0.0, 1.0),
            },
            SurfaceVertex {
                position: [x, heights[3], z + size],
                normal: Vec3::Y,
                shore_factor: ((sea_level_y - heights[3]) / 1.8).clamp(0.0, 1.0),
            },
        ];
        let (polygon, length) = clip_surface_polygon(corners, sea_level_y, false);
        if length < 3 {
            return;
        }

        let base = self.positions.len() as u32;
        for vertex in polygon.iter().take(length) {
            self.positions
                .push([vertex.position[0], water_y, vertex.position[2]]);
            self.normals.push(Vec3::Y.to_array());
            // Keep RGB neutral because StandardMaterial consumes it as a
            // tint. Opaque water does not use vertex alpha, so that lane can
            // carry the shoreline factor to the extension shader.
            self.colors.push([1.0, 1.0, 1.0, vertex.shore_factor]);
            self.uvs
                .push([vertex.position[0] * 0.18, vertex.position[2] * 0.18]);
        }
        for index in 1..length - 1 {
            self.indices
                .extend([base, base + index as u32 + 1, base + index as u32]);
        }
    }

    fn into_mesh(self) -> Mesh {
        if self.positions.is_empty() || self.indices.is_empty() {
            return empty_mesh();
        }
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

#[derive(Clone, Copy)]
struct WaterDemoSpawn {
    position: Vec3,
    yaw: f32,
}

fn find_water_demo_spawn(terrain: &ProceduralTerrainSurface) -> Option<WaterDemoSpawn> {
    const DIRECTIONS: [Vec2; 8] = [
        Vec2::new(0.0, -1.0),
        Vec2::new(0.707, -0.707),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.707, 0.707),
        Vec2::new(0.0, 1.0),
        Vec2::new(-0.707, 0.707),
        Vec2::new(-1.0, 0.0),
        Vec2::new(-0.707, -0.707),
    ];
    let mut best: Option<(f32, WaterDemoSpawn)> = None;
    for z in (-48i32..48).step_by(2) {
        for x in (-48i32..48).step_by(2) {
            let position = Vec3::new(x as f32 + 0.5, 0.0, z as f32 + 0.5);
            let Some(depth) = terrain.water_depth_at(position) else {
                continue;
            };
            if depth < 1.2 {
                continue;
            }

            let mut openness = 0.0;
            let mut best_direction = DIRECTIONS[0];
            let mut best_direction_score = f32::NEG_INFINITY;
            for direction in DIRECTIONS {
                let direction_score = [3.0, 6.0, 9.0]
                    .into_iter()
                    .filter_map(|distance| {
                        let sample = position
                            + Vec3::new(direction.x * distance, 0.0, direction.y * distance);
                        terrain.water_depth_at(sample)
                    })
                    .map(|sample_depth| sample_depth.min(3.0))
                    .sum::<f32>();
                openness += direction_score;
                if direction_score > best_direction_score {
                    best_direction_score = direction_score;
                    best_direction = direction;
                }
            }

            let floor = terrain.water_floor_height(position);
            let desired_y = terrain.water_surface_height() - (depth * 0.55).clamp(1.5, 2.4);
            let spawn = WaterDemoSpawn {
                position: Vec3::new(
                    position.x,
                    desired_y.max(floor + PLAYER_PHYSICS_CENTER_HEIGHT + 0.08),
                    position.z,
                ),
                yaw: best_direction.x.atan2(best_direction.y),
            };
            let distance = position.x * position.x + position.z * position.z;
            let score = openness * 20.0 + depth * 12.0 - distance * 0.004;
            if best.is_none_or(|(best_score, _)| score > best_score) {
                best = Some((score, spawn));
            }
        }
    }
    best.map(|(_, spawn)| spawn)
}

pub fn setup_living_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut grass_materials: ResMut<Assets<GrassWindMaterial>>,
    nature: Res<OfflineNature>,
    content_layout: Res<LivingContentLayout>,
    content_registry: Res<lk2_core::content::ContentRegistry>,
    terrain: Res<ProceduralTerrainSurface>,
    shadow_assets: Res<TreeShadowAssets>,
    mut control_scheme_configs: ResMut<Assets<PlayerControlSchemeConfig>>,
) {
    spawn_content_visuals(&mut commands, &asset_server, &content_layout, &terrain);
    // The registry gallery is an audit surface, not gameplay content. Keep it
    // behind the existing content-debug entry point so the normal living scene
    // only contains the authored world layout and runtime ecology.
    if std::env::args().any(|arg| arg == "--content-debug-open") {
        spawn_exported_content_visuals(&mut commands, &asset_server, &content_registry, &terrain);
    }

    let default_player_position = Vec3::new(
        0.0,
        terrain.ground_height(Vec3::new(0.0, 0.0, 11.0)) + PLAYER_PHYSICS_CENTER_HEIGHT,
        11.0,
    );
    let player_position = if std::env::args().any(|arg| arg == "--water-demo") {
        find_water_demo_spawn(&terrain).map_or(default_player_position, |spawn| spawn.position)
    } else {
        default_player_position
    };
    spawn_player_avatar(
        &mut commands,
        &mut meshes,
        &mut materials,
        player_position,
        std::f32::consts::PI,
        &shadow_assets,
        &mut control_scheme_configs,
    );
    spawn_main_scene_landmarks(
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut materials,
        &terrain,
    );
    spawn_legendary_weapon_visuals(&mut commands, &asset_server, &terrain);
    // Keep the landmark readable in the background without placing its
    // seven-unit silhouette directly in the first-person camera corridor.
    let spire_base = grounded_content_position(&terrain, Vec3::new(-15.0, 0.0, -22.0));
    spawn_asset(
        &mut commands,
        &asset_server,
        FOREST_STONE_SPIRE_PATH,
        spire_base,
        0.50,
        -0.08,
        "forest_stone_spire",
    );
    spawn_static_collider(
        &mut commands,
        spire_base + Vec3::Y * 0.25,
        Quat::from_rotation_y(-0.08),
        Collider::cuboid(1.05, 0.50, 0.82),
        "forest_stone_spire_collider",
    );
    let boss_pos = Vec3::new(0.0, 0.05, -24.0);
    spawn_asset(
        &mut commands,
        &asset_server,
        BOSS_PATH,
        boss_pos,
        0.48,
        0.0,
        "forest_boss",
    )
    .insert((
        BossActor { base: boss_pos },
        HitReaction::default(),
        Health {
            current: 24.0,
            max: 24.0,
            invuln_until_tick: 0,
        },
    ));
    spawn_static_collider(
        &mut commands,
        boss_pos + Vec3::Y * 0.9,
        Quat::IDENTITY,
        Collider::capsule(0.42, 1.10),
        "forest_boss_collider",
    );
    let tree_layout = [
        (-20.0, -4.0, 0.1, 1.05, TREE_PATH),
        (-17.0, 6.0, -0.5, 0.92, ROUND_TREE_PATH),
        (-12.0, 17.0, 0.4, 1.08, PINE_TREE_PATH),
        (-4.0, 20.0, -0.2, 0.90, AUTUMN_TREE_PATH),
        (7.0, 20.0, 0.7, 1.02, ROUND_TREE_PATH),
        (17.0, 14.0, -0.8, 0.95, BIRCH_TREE_PATH),
        (21.0, 3.0, 0.3, 1.08, TREE_PATH),
        (19.0, -9.0, -0.4, 0.96, WILLOW_TREE_PATH),
        (11.0, -19.0, 0.6, 1.06, PINE_TREE_PATH),
        (-8.0, -20.0, -0.1, 0.92, TREE_PATH),
        (-19.0, -15.0, 0.8, 1.02, ROUND_TREE_PATH),
        (-34.0, -27.0, 0.2, 1.10, PINE_TREE_PATH),
        (-26.0, -37.0, -0.6, 0.98, WILLOW_TREE_PATH),
        (-5.0, -39.0, 0.4, 1.12, ROUND_TREE_PATH),
        (18.0, -36.0, -0.3, 1.04, AUTUMN_TREE_PATH),
        (35.0, -25.0, 0.7, 1.08, TREE_PATH),
        (40.0, -5.0, -0.4, 0.96, BIRCH_TREE_PATH),
        (37.0, 18.0, 0.1, 1.10, PINE_TREE_PATH),
        (25.0, 35.0, -0.7, 1.00, ROUND_TREE_PATH),
        (2.0, 40.0, 0.5, 1.06, WILLOW_TREE_PATH),
        (-22.0, 36.0, -0.2, 1.02, AUTUMN_TREE_PATH),
        (-38.0, 21.0, 0.8, 1.08, TREE_PATH),
        (-40.0, -3.0, -0.5, 1.00, BIRCH_TREE_PATH),
    ];
    for (index, (x, z, yaw, scale, path)) in tree_layout.into_iter().enumerate() {
        // Leave a deterministic camera corridor around the spawn point. A
        // third-person camera can start inside a broad imported canopy even
        // when the trunk collider is just outside the player capsule.
        if Vec2::new(x + 2.0, z - 11.0).length() < 24.0 {
            continue;
        }
        // The authored foliage assets have broad canopies. Keep the initial
        // third-person corridor readable instead of letting one nearby tree
        // fill the entire camera frustum.
        let scale = scale * 0.60;
        let position = grounded_content_position(&terrain, Vec3::new(x, 0.0, z));
        spawn_asset(
            &mut commands,
            &asset_server,
            path,
            position,
            scale,
            yaw,
            "forest_tree",
        )
        .insert(ProceduralTreeSway {
            base_translation: position,
            base_yaw: yaw,
            base_scale: scale,
            phase: index as f32 * 0.73 + hash01(index, 83) * 2.0,
            strength: 0.018 + hash01(index, 97) * 0.014,
        });
        spawn_static_collider(
            &mut commands,
            position + Vec3::Y * (1.25 * scale),
            Quat::IDENTITY,
            Collider::cylinder(0.42 * scale, 2.5 * scale),
            "forest_tree_collider",
        );
    }

    let tall_pine_layout = [
        (-15.0, 5.0, -0.18, 2.15),
        (10.5, 4.0, 0.42, 2.35),
        (-13.5, -7.0, 0.28, 1.82),
        (9.0, -8.5, -0.36, 2.05),
        (-10.0, -18.0, 0.62, 2.30),
        (7.5, -19.0, -0.42, 2.45),
        (-22.0, -28.0, 0.10, 2.18),
        (18.0, -27.0, -0.28, 2.55),
        (-31.0, -12.0, 0.52, 2.30),
        (27.0, -10.0, -0.52, 2.10),
        (-28.0, 8.0, -0.18, 2.00),
        (24.0, 12.0, 0.36, 2.25),
    ];
    for (index, (x, z, yaw, scale)) in tall_pine_layout.into_iter().enumerate() {
        if Vec2::new(x + 2.0, z - 11.0).length() < 24.0 {
            continue;
        }
        let scale = scale * 0.50;
        let position = grounded_content_position(&terrain, Vec3::new(x, 0.0, z));
        spawn_asset(
            &mut commands,
            &asset_server,
            PINE_TREE_PATH,
            position,
            scale,
            yaw,
            "forest_tall_pine",
        )
        .insert(ProceduralTreeSway {
            base_translation: position,
            base_yaw: yaw,
            base_scale: scale,
            phase: 1.7 + index as f32 * 0.47,
            strength: 0.012 + hash01(index, 131) * 0.012,
        });
        spawn_static_collider(
            &mut commands,
            position + Vec3::Y * (1.25 * scale),
            Quat::IDENTITY,
            Collider::cylinder(0.42 * scale, 2.5 * scale),
            "forest_tall_pine_collider",
        );
    }

    for (path, base_position, scale, yaw) in [
        (DEER_FAWN_PATH, Vec3::new(4.5, 0.0, 4.0), 1.05, 0.35),
        (FOX_SILVER_PATH, Vec3::new(6.0, 0.0, 8.0), 1.00, -0.8),
        (RABBIT_BROWN_PATH, Vec3::new(-2.0, 0.0, 2.5), 1.10, 1.2),
    ] {
        let position = Vec3::new(
            base_position.x,
            terrain.ground_height(base_position) + 0.04,
            base_position.z,
        );
        spawn_asset(
            &mut commands,
            &asset_server,
            path,
            position,
            scale,
            yaw,
            "forest_wildlife_variant",
        );
        spawn_static_collider(
            &mut commands,
            position + Vec3::Y * (0.32 * scale),
            Quat::from_rotation_y(yaw),
            Collider::capsule(0.24 * scale, 0.64 * scale),
            "forest_wildlife_variant_collider",
        );
    }

    for index in 0..16 {
        let angle = index as f32 * 2.399;
        let radius = 7.0 + hash01(index, 7) * 12.0;
        let ground_pos = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        if terrain_cell_is_water(
            &terrain,
            ground_pos.x.floor() as i32,
            ground_pos.z.floor() as i32,
        ) {
            continue;
        }
        let pos = Vec3::new(
            ground_pos.x,
            terrain.ground_height(ground_pos) + 0.03,
            ground_pos.z,
        );
        let detail_scale = 0.72 + hash01(index, 11) * 0.45;
        let detail_yaw = angle + 0.4;
        let is_rock = index % 3 == 0;
        spawn_asset(
            &mut commands,
            &asset_server,
            if is_rock { ROCK_PATH } else { BRANCH_PATH },
            pos,
            detail_scale,
            detail_yaw,
            "forest_floor_detail",
        );
        spawn_static_collider(
            &mut commands,
            pos + Vec3::Y
                * if is_rock {
                    0.22 * detail_scale
                } else {
                    0.08 * detail_scale
                },
            Quat::from_rotation_y(detail_yaw),
            if is_rock {
                Collider::sphere(0.34 * detail_scale)
            } else {
                Collider::cuboid(
                    0.52 * detail_scale,
                    0.10 * detail_scale,
                    0.12 * detail_scale,
                )
            },
            "forest_floor_detail_collider",
        );
    }

    // Low, irregular moss islands break up the large grass surface without
    // introducing coplanar decals. They are presentation-only and share two
    // materials/one mesh, so the extra color variation stays inexpensive.
    let moss_mesh = meshes.add(
        Sphere::new(0.72)
            .mesh()
            .ico(1)
            .expect("ground moss mesh should be valid"),
    );
    let moss_materials = [
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.14, 0.32, 0.10),
            perceptual_roughness: 0.98,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.30, 0.48, 0.14),
            perceptual_roughness: 0.96,
            ..default()
        }),
    ];
    for index in 0..56 {
        let angle = hash01(index, 211) * std::f32::consts::TAU;
        let radius = 6.0 + hash01(index, 223).sqrt() * 40.0;
        let ground_pos = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        if terrain_cell_is_water(
            &terrain,
            ground_pos.x.floor() as i32,
            ground_pos.z.floor() as i32,
        ) {
            continue;
        }
        let position = Vec3::new(
            ground_pos.x,
            terrain.ground_height(ground_pos) + 0.06,
            ground_pos.z,
        );
        if position.distance(Vec3::new(-2.0, 0.0, 11.0)) < 4.2
            || position.distance(Vec3::new(0.0, 0.0, 7.0)) < 4.0
        {
            continue;
        }
        let width = 0.75 + hash01(index, 227) * 0.85;
        let depth = 0.55 + hash01(index, 229) * 0.65;
        commands.spawn((
            Mesh3d(moss_mesh.clone()),
            MeshMaterial3d(moss_materials[index % 2].clone()),
            Transform::from_translation(position).with_scale(Vec3::new(width, 0.10, depth)),
            ForestRealmVisual,
            Name::new("forest_ground_moss"),
        ));
    }

    let grass_mesh = meshes.add(build_grass_tuft_mesh());
    let grass_materials = [
        grass_materials.add(GrassWindMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.20, 0.48, 0.18),
                // Grass must remain readable when the directional light or a
                // stylized terrain shadow would otherwise turn the blades black.
                unlit: true,
                perceptual_roughness: 0.96,
                // Vegetation is readable from both sides while the runtime
                // billboard keeps its silhouette facing the camera.
                cull_mode: None,
                ..default()
            },
            extension: GrassWindExtension::new(),
        }),
        grass_materials.add(GrassWindMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.34, 0.62, 0.20),
                unlit: true,
                perceptual_roughness: 0.94,
                cull_mode: None,
                ..default()
            },
            extension: GrassWindExtension::new(),
        }),
    ];
    const GRASS_COUNT: usize = 480;
    for index in 0..GRASS_COUNT {
        // Four nearby tufts share an anchor, producing readable clumps with
        // open grass between them instead of a uniformly noisy lawn.
        let cluster = index / 4;
        let angle = hash01(cluster, 17) * std::f32::consts::TAU + (hash01(index, 19) - 0.5) * 0.34;
        let radius = 4.5 + hash01(cluster, 29).sqrt() * 39.0 + (hash01(index, 23) - 0.5) * 1.8;
        let ground_pos = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        if !terrain.is_renderable_land_at(ground_pos) {
            continue;
        }
        let ground_y = terrain.ground_height(ground_pos);
        let pos = Vec3::new(ground_pos.x, ground_y + 0.02, ground_pos.z);
        if pos.x.abs() < 3.0 || pos.distance(Vec3::new(12.5, 0.0, 7.5)) < 5.8 {
            continue;
        }
        let mature_scale = Vec3::new(
            0.75 + hash01(index, 41) * 0.55,
            0.70 + hash01(index, 47) * 0.75,
            0.75 + hash01(index, 53) * 0.55,
        );
        commands.spawn((
            Mesh3d(grass_mesh.clone()),
            MeshMaterial3d(grass_materials[index % 2].clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(angle))
                .with_scale(Vec3::ZERO),
            GrassTuft {
                growth_threshold: index as f32 / GRASS_COUNT as f32 * 32.0,
                mature_scale,
            },
            GrassWind {
                // The runtime billboard system owns the camera-facing yaw.
                base_yaw: angle,
                phase: index as f32 * 0.47,
                strength: 0.055 + hash01(index, 59) * 0.035,
            },
            ForestRealmVisual,
            Name::new("rain_grown_grass"),
        ));
    }

    let rain_mesh = meshes.add(Cuboid::new(0.035, 0.72, 0.035));
    let rain_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.48, 0.74, 1.0, 0.72),
        emissive: Color::srgb(0.08, 0.24, 0.58).into(),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    for (cloud_index, cloud) in nature.snapshot.detailed_ecology.clouds.iter().enumerate() {
        let base = Vec3::new(cloud.x, CLOUD_VISUAL_HEIGHT, cloud.z);
        spawn_asset(
            &mut commands,
            &asset_server,
            CLOUD_PATH,
            base,
            CLOUD_VISUAL_SCALE,
            cloud_index as f32 * 0.4,
            "rain_cloud",
        )
        .insert(Cloud {
            snapshot_index: cloud_index,
            phase: cloud.phase,
        });
        for index in 0..32 {
            let local = Vec3::new(
                (hash01(index, cloud_index * 31 + 3) - 0.5) * 7.5,
                0.0,
                (hash01(index, cloud_index * 47 + 5) - 0.5) * 5.5,
            );
            commands.spawn((
                Mesh3d(rain_mesh.clone()),
                MeshMaterial3d(rain_material.clone()),
                Transform::from_translation(base + local),
                RainDrop {
                    cloud_index,
                    local,
                    phase: hash01(index, cloud_index * 71 + 9) * 10.0,
                },
                ForestRealmVisual,
                Name::new("pooled_rain_drop"),
            ));
        }
    }
}

/// A small authored introduction space. The procedural content volume still
/// owns the wider world, but the first twenty metres need to communicate
/// ordinary nouns immediately: camp, farm, portal, and danger.
fn spawn_main_scene_landmarks(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    terrain: &ProceduralTerrainSurface,
) {
    let platform_mesh = meshes.add(Cylinder::new(2.7, 0.10).mesh().resolution(24));
    let platform_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.58, 0.39, 0.20),
        unlit: true,
        perceptual_roughness: 0.98,
        ..default()
    });
    let hub = Vec3::new(0.0, 0.0, 2.0);
    commands.spawn((
        Mesh3d(platform_mesh),
        MeshMaterial3d(platform_material),
        Transform::from_xyz(
            hub.x,
            terrain.ground_height(hub) + 0.045,
            hub.z,
        ),
        ForestRealmVisual,
        Name::new("main_scene_camp_platform"),
    ));

    spawn_asset(
        commands,
        asset_server,
        CAMPFIRE_PATH,
        grounded_content_position(terrain, hub + Vec3::new(0.0, 0.0, -0.2)),
        0.58,
        0.0,
        "main_scene_campfire",
    );

    // The fire explains the gathering point; a small, deliberately simple
    // cabin explains that this is also the player's home. Keep it offset from
    // the route so its silhouette reads without becoming another wall.
    let cabin_wall_mesh = meshes.add(Cuboid::new(2.7, 1.65, 2.05));
    let cabin_roof_mesh = meshes.add(Cone::new(1.95, 1.0).mesh().resolution(4));
    let cabin_door_mesh = meshes.add(Cuboid::new(0.62, 1.05, 0.08));
    let cabin_wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.51, 0.25),
        unlit: true,
        ..default()
    });
    let cabin_roof_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.12, 0.08),
        unlit: true,
        ..default()
    });
    let cabin_door_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.055, 0.025),
        unlit: true,
        ..default()
    });
    let cabin = Vec3::new(4.0, 0.0, 1.5);
    let cabin_ground = terrain.ground_height(cabin);
    commands.spawn((
        Mesh3d(cabin_wall_mesh),
        MeshMaterial3d(cabin_wall_material),
        Transform::from_xyz(cabin.x, cabin_ground + 0.825, cabin.z),
        ForestRealmVisual,
        Name::new("main_scene_home_cabin"),
    ));
    commands.spawn((
        Mesh3d(cabin_roof_mesh),
        MeshMaterial3d(cabin_roof_material),
        Transform::from_xyz(cabin.x, cabin_ground + 2.15, cabin.z),
        ForestRealmVisual,
        Name::new("main_scene_home_roof"),
    ));
    commands.spawn((
        Mesh3d(cabin_door_mesh),
        MeshMaterial3d(cabin_door_material),
        Transform::from_xyz(cabin.x, cabin_ground + 0.52, cabin.z + 1.05),
        ForestRealmVisual,
        Name::new("main_scene_home_door"),
    ));

    let fire_mesh = meshes.add(Cone::new(0.34, 1.05).mesh().resolution(6));
    let fire_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.28, 0.04),
        emissive: Color::srgb(1.0, 0.16, 0.01).into(),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(fire_mesh),
        MeshMaterial3d(fire_material),
        Transform::from_xyz(hub.x, terrain.ground_height(hub) + 0.58, hub.z),
        ForestRealmVisual,
        Name::new("main_scene_campfire_flame"),
    ));

    let log_mesh = meshes.add(Cylinder::new(0.13, 1.25).mesh().resolution(8));
    let log_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.10, 0.035),
        unlit: true,
        ..default()
    });
    for (index, rotation) in [0.25_f32, 2.35, 1.30].into_iter().enumerate() {
        commands.spawn((
            Mesh3d(log_mesh.clone()),
            MeshMaterial3d(log_material.clone()),
            Transform::from_xyz(hub.x, terrain.ground_height(hub) + 0.16, hub.z)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_y(rotation)),
            ForestRealmVisual,
            Name::new(format!("main_scene_campfire_log_{index}")),
        ));
    }
}

pub fn setup_farm_plots(
    mut commands: Commands,
    terrain: Res<ProceduralTerrainSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Keep the farm readable even when no crop is planted. The raised rim and
    // soil surface are separate shallow meshes, so they do not z-fight with
    // the procedural terrain or become authoritative terrain overlays.
    let plot_rim_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.24, 0.12),
        unlit: true,
        perceptual_roughness: 0.98,
        ..default()
    });
    let soil_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.12, 0.065),
        unlit: true,
        perceptual_roughness: 0.99,
        ..default()
    });
    let furrow_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.13, 0.06, 0.032),
        unlit: true,
        perceptual_roughness: 1.0,
        ..default()
    });
    let plot_rim_mesh = meshes.add(Cuboid::new(3.35, 0.10, 2.30));
    let soil_mesh = meshes.add(Cuboid::new(3.02, 0.075, 1.98));
    let furrow_mesh = meshes.add(Cuboid::new(2.65, 0.026, 0.09));
    let crop_materials = [
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.78, 0.67, 0.18),
            perceptual_roughness: 0.84,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.90, 0.24, 0.12),
            perceptual_roughness: 0.82,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.58, 0.38, 0.20),
            perceptual_roughness: 0.90,
            ..default()
        }),
    ];
    let crop_mesh = meshes.add(Cone::new(0.26, 0.92).mesh().resolution(6));
    commands.insert_resource(FarmVisualMaterials {
        crop_materials: crop_materials.clone(),
    });

    let path_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.50, 0.32, 0.17),
        perceptual_roughness: 0.99,
        ..default()
    });
    let path_mesh = meshes.add(Cuboid::new(1.25, 0.055, 4.8));
    let cross_path_mesh = meshes.add(Cuboid::new(9.8, 0.055, 1.05));
    let branch_mesh = meshes.add(Cuboid::new(0.82, 0.055, 2.5));
    for (mesh, position, name) in [
        (path_mesh.clone(), Vec3::new(-8.0, 0.0, 6.1), "farm_path_entry"),
        (
            cross_path_mesh.clone(),
            Vec3::new(-5.8, 0.0, 6.25),
            "farm_path_crossing",
        ),
    ] {
        let ground_y = terrain.ground_height(position);
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(path_material.clone()),
            Transform::from_xyz(position.x, ground_y + 0.05, position.z),
            ForestRealmVisual,
            Name::new(name),
        ));
    }
    for (index, (x, z)) in [(-8.0_f32, 4.5), (-8.0, 8.0), (-4.0, 6.25)]
        .into_iter()
        .enumerate()
    {
        let position = Vec3::new(x, 0.0, z);
        commands.spawn((
            Mesh3d(branch_mesh.clone()),
            MeshMaterial3d(path_material.clone()),
            Transform::from_xyz(
                position.x,
                terrain.ground_height(position) + 0.05,
                position.z,
            ),
            ForestRealmVisual,
            Name::new(format!("farm_path_branch_{index}")),
        ));
    }

    for (id, position) in FARM_PLOT_POSITIONS.into_iter().enumerate() {
        let ground_y = terrain.ground_height(position);
        let plot_scale = Vec3::ONE;
        commands.spawn((
            Mesh3d(plot_rim_mesh.clone()),
            MeshMaterial3d(plot_rim_material.clone()),
            Transform::from_xyz(position.x, ground_y + 0.04, position.z).with_scale(plot_scale),
            ForestRealmVisual,
            Name::new("farm_plot_rim"),
        ));
        commands.spawn((
            Mesh3d(soil_mesh.clone()),
            MeshMaterial3d(soil_material.clone()),
            Transform::from_xyz(position.x, ground_y + 0.095, position.z).with_scale(plot_scale),
            ForestRealmVisual,
            Name::new("farm_plot_soil"),
        ));
        for row in [-0.72_f32, -0.24, 0.24, 0.72] {
            commands.spawn((
                Mesh3d(furrow_mesh.clone()),
                MeshMaterial3d(furrow_material.clone()),
                Transform::from_xyz(position.x, ground_y + 0.145, position.z + row)
                    .with_scale(Vec3::ONE),
                ForestRealmVisual,
                Name::new("farm_plot_furrow"),
            ));
        }
        commands.spawn((
            Mesh3d(crop_mesh.clone()),
            MeshMaterial3d(crop_materials[0].clone()),
            Transform::from_translation(Vec3::new(position.x, ground_y + 0.48, position.z))
                .with_scale(Vec3::ZERO),
            Visibility::Hidden,
            super::state::FarmCropVisual { id: id as u32 },
            ForestRealmVisual,
            Name::new("farm_plot_crop"),
        ));
    }
}

fn spawn_static_collider(
    commands: &mut Commands,
    position: Vec3,
    rotation: Quat,
    collider: Collider,
    name: &'static str,
) {
    commands.spawn((
        RigidBody::Static,
        collider,
        ForestRealmCollider,
        Transform::from_translation(position).with_rotation(rotation),
        Name::new(name),
    ));
}

pub fn setup_camera(
    mut commands: Commands,
    debug: Res<CollisionDebugState>,
    terrain: Res<ProceduralTerrainSurface>,
) {
    let force_third_person = std::env::args().any(|arg| arg == "--third-person");
    let preview_front = std::env::args().any(|arg| arg == "--third-person-front");
    let boss_scene_shot = std::env::args().any(|arg| arg == "--boss-scene-shot");
    let water_demo = std::env::args().any(|arg| arg == "--water-demo");
    let water_demo_spawn = water_demo
        .then(|| find_water_demo_spawn(&terrain))
        .flatten();
    let default_camera = LivingCameraRig::default();
    commands.insert_resource(LivingCameraRig {
        mode: if boss_scene_shot {
            super::state::CameraMode::FreeCam
        } else if debug.enabled || force_third_person {
            super::state::CameraMode::ThirdPerson
        } else {
            default_camera.mode
        },
        yaw: if preview_front {
            0.0
        } else {
            water_demo_spawn.map_or(default_camera.yaw, |spawn| spawn.yaw)
        },
        pitch: if water_demo {
            -0.12
        } else {
            default_camera.pitch
        },
        free_position: if boss_scene_shot {
            Vec3::new(0.0, 7.0, 15.0)
        } else {
            default_camera.free_position
        },
        free_yaw: if boss_scene_shot {
            std::f32::consts::PI
        } else {
            default_camera.free_yaw
        },
        free_pitch: if boss_scene_shot { -0.08 } else { default_camera.free_pitch },
        ..default_camera
    });
    commands
        .spawn((
            Camera3d::default(),
            Hdr,
            Transform::from_xyz(20.0, 18.0, 26.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
            AtmosphereSettings::default(),
            AtmosphereEnvironmentMapLight::default(),
            Exposure { ev100: 11.5 },
            Tonemapping::AcesFitted,
            // The terrain is a dense interpolated heightfield. Temporal AA
            // removes the sub-pixel shimmer along its many shoreline and
            // ridge edges; MSAA alone cannot smooth those moving edges.
            Msaa::Off,
            // The gameplay scene already owns a depth prepass, so SSAO adds
            // cheap grounding under props and characters without another
            // geometry pass. Keep it medium to preserve the toy-like palette.
            ScreenSpaceAmbientOcclusion {
                quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
                ..default()
            },
            TemporalAntiAliasing::default(),
            TemporalJitter::default(),
            MipBias(0.0),
            MotionVectorPrepass,
            // Temporal filtering uses the existing TAA history to stabilize
            // the cascaded shadow edge without increasing the shadow atlas.
            ShadowFilteringMethod::Temporal,
            DepthPrepass,
        ))
        // Contact shadows are a short-range depth ray march that fills the
        // gap where a shadow map cannot resolve feet, foliage, and props. Keep
        // this outside the camera bundle to stay within Bevy's tuple limit.
        .insert(ContactShadows::default())
        .insert(DistanceFog {
            // Keep a very light atmospheric layer above water so distant
            // terrain recedes into the sky instead of ending at a hard band.
            color: Color::srgba(0.47, 0.61, 0.72, 0.10),
            directional_light_color: Color::NONE,
            directional_light_exponent: 12.0,
            falloff: FogFalloff::Linear {
                start: 22.0,
                end: 58.0,
            },
        })
        .insert(LivingSceneCamera);
}

pub(crate) fn build_grass_tuft_mesh() -> Mesh {
    // Five tapered blades make a readable grass silhouette from the gameplay
    // camera. Each blade is a bent ribbon rather than a generic cone, while
    // the shared mesh keeps the large field cheap to render.
    const BLADE_COUNT: usize = 5;
    const HEIGHTS: [f32; BLADE_COUNT] = [0.68, 0.58, 0.72, 0.63, 0.54];
    const WIDTHS: [f32; BLADE_COUNT] = [0.11, 0.09, 0.12, 0.10, 0.085];
    const LEANS: [f32; BLADE_COUNT] = [0.12, 0.16, 0.09, 0.14, 0.18];

    let mut positions = Vec::with_capacity(BLADE_COUNT * 5);
    let mut indices = Vec::with_capacity(BLADE_COUNT * 9);
    for blade in 0..BLADE_COUNT {
        let angle = blade as f32 / BLADE_COUNT as f32 * std::f32::consts::TAU + 0.18;
        let radial = Vec3::new(angle.cos(), 0.0, angle.sin());
        let side = Vec3::new(-angle.sin(), 0.0, angle.cos());
        let base_center = radial * (0.012 + blade as f32 * 0.004);
        let middle_center = base_center + radial * (LEANS[blade] * 0.36);
        let tip = base_center + radial * LEANS[blade] + side * (0.008 * (blade as f32 - 2.0));
        let height = HEIGHTS[blade];
        let width = WIDTHS[blade];
        let middle_width = width * 0.62;
        let base = positions.len() as u32;

        positions.extend([
            base_center - side * (width * 0.5),
            base_center + side * (width * 0.5),
            middle_center - side * (middle_width * 0.5) + Vec3::Y * (height * 0.46),
            middle_center + side * (middle_width * 0.5) + Vec3::Y * (height * 0.46),
            tip + Vec3::Y * height,
        ]);
        indices.extend([
            base,
            base + 2,
            base + 1,
            base + 1,
            base + 2,
            base + 3,
            base + 2,
            base + 4,
            base + 3,
        ]);
    }

    let mut normals = vec![[0.0; 3]; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let a = positions[triangle[0] as usize];
        let b = positions[triangle[1] as usize];
        let c = positions[triangle[2] as usize];
        let normal = (b - a).cross(c - a);
        for index in triangle {
            let accumulated = Vec3::from_array(normals[*index as usize]) + normal;
            normals[*index as usize] = accumulated.to_array();
        }
    }
    for normal in &mut normals {
        *normal = Vec3::from_array(*normal).normalize_or_zero().to_array();
    }

    let uvs = positions
        .iter()
        .map(|position| [position[0] + 0.5, position[1] / 0.72])
        .collect::<Vec<_>>();

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn build_player_torso_mesh() -> Mesh {
    // (height, horizontal radius, depth radius). The narrow lower ring
    // disappears into the trousers; the broad upper rings make the shirt and
    // shoulders read as one compact toy body instead of a floating capsule.
    const RINGS: &[(f32, f32, f32)] = &[
        (-0.31, 0.12, 0.10),
        (-0.25, 0.21, 0.15),
        (-0.10, 0.28, 0.19),
        (0.08, 0.31, 0.21),
        (0.22, 0.29, 0.19),
        (0.30, 0.16, 0.12),
    ];
    const SEGMENTS: usize = 10;

    let mut positions = Vec::with_capacity(RINGS.len() * SEGMENTS + 2);
    for &(height, radius_x, radius_z) in RINGS {
        for segment in 0..SEGMENTS {
            let angle = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            positions.push([radius_x * angle.cos(), height, radius_z * angle.sin()]);
        }
    }

    let bottom_pole = positions.len() as u32;
    positions.push([0.0, RINGS[0].0 - 0.015, 0.0]);
    let top_pole = positions.len() as u32;
    positions.push([0.0, RINGS[RINGS.len() - 1].0 + 0.015, 0.0]);

    let mut indices = Vec::with_capacity((RINGS.len() - 1) * SEGMENTS * 6 + SEGMENTS * 6);
    for ring in 0..RINGS.len() - 1 {
        let current = (ring * SEGMENTS) as u32;
        let next = ((ring + 1) * SEGMENTS) as u32;
        for segment in 0..SEGMENTS {
            let next_segment = (segment + 1) as u32 % SEGMENTS as u32;
            let a = current + segment as u32;
            let b = current + next_segment;
            let c = next + next_segment;
            let d = next + segment as u32;
            // Counter-clockwise from the outside of the body.
            indices.extend([a, d, b, b, d, c]);
        }
    }
    for segment in 0..SEGMENTS {
        let next_segment = (segment + 1) as u32 % SEGMENTS as u32;
        let bottom = segment as u32;
        let bottom_next = next_segment;
        let top = ((RINGS.len() - 1) * SEGMENTS) as u32 + segment as u32;
        let top_next = ((RINGS.len() - 1) * SEGMENTS) as u32 + next_segment;
        indices.extend([bottom, bottom_next, bottom_pole]);
        indices.extend([top, top_pole, top_next]);
    }

    let mut normals = vec![[0.0; 3]; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let a = Vec3::from_array(positions[triangle[0] as usize]);
        let b = Vec3::from_array(positions[triangle[1] as usize]);
        let c = Vec3::from_array(positions[triangle[2] as usize]);
        let normal = (b - a).cross(c - a);
        for index in triangle {
            let accumulated = Vec3::from_array(normals[*index as usize]) + normal;
            normals[*index as usize] = accumulated.to_array();
        }
    }
    for normal in &mut normals {
        *normal = Vec3::from_array(*normal).normalize_or_zero().to_array();
    }

    let uvs = positions
        .iter()
        .map(|position| [position[0] * 1.8 + 0.5, position[2] * 1.8 + 0.5])
        .collect::<Vec<_>>();

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn spawn_player_avatar(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    yaw: f32,
    shadow_assets: &TreeShadowAssets,
    control_scheme_configs: &mut ResMut<Assets<PlayerControlSchemeConfig>>,
) {
    let player = commands
        .spawn((
            (
                Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
                Visibility::Visible,
                InheritedVisibility::default(),
                PlayerActor,
                PlayerMotion::default(),
                PlayerJump::default(),
                PlayerSwimState {
                    active: std::env::args().any(|arg| arg == "--water-demo"),
                    ..default()
                },
                PlayerSkillState::default(),
                PvpCombatant::default(),
                SimpleWeapon::default(),
                Health::default(),
            ),
            (
                RigidBody::Dynamic,
                Collider::capsule(PLAYER_COLLIDER_RADIUS, PLAYER_COLLIDER_SEGMENT),
                GravityScale(1.0),
                TransformInterpolation,
                TnuaController::<PlayerControlScheme>::default(),
                TnuaConfig::<PlayerControlScheme>(control_scheme_configs.add(
                    PlayerControlSchemeConfig {
                        basis: TnuaBuiltinWalkConfig {
                            speed: PLAYER_SPEED,
                            float_height: PLAYER_PHYSICS_CENTER_HEIGHT,
                            max_slope: std::f32::consts::FRAC_PI_4,
                            ..default()
                        },
                        jump: TnuaBuiltinJumpConfig {
                            height: PLAYER_JUMP_HEIGHT,
                            ..default()
                        },
                    },
                )),
                TnuaAvian3dSensorShape(Collider::cylinder(0.34, 0.0)),
                LockedAxes::new().lock_rotation_x().lock_rotation_z(),
                Name::new("player"),
            ),
        ))
        .id();

    // Keep the contact cue on the physics root so it follows authoritative
    // movement without making any IK child responsible for world position.
    commands.spawn((
        Mesh3d(shadow_assets.mesh.clone()),
        MeshMaterial3d(shadow_assets.material.clone()),
        Transform::from_xyz(0.0, -PLAYER_PHYSICS_CENTER_HEIGHT + 0.015, 0.0)
            .with_scale(Vec3::new(0.52, 0.34, 0.52)),
        PlayerGroundShadow,
        ChildOf(player),
        Name::new("player_ground_shadow"),
    ));

    let skin = materials.add(StandardMaterial {
        // Warm skin, forest-dyed cloth, dark iron-blue trousers and leather
        // accessories establish an adventurer silhouette.
        base_color: Color::srgb(0.90, 0.60, 0.40),
        emissive: Color::srgb(0.028, 0.012, 0.006).into(),
        unlit: true,
        perceptual_roughness: 0.95,
        ..default()
    });
    let shirt = materials.add(StandardMaterial {
        base_color: Color::srgb(0.54, 0.18, 0.21),
        emissive: Color::srgb(0.075, 0.020, 0.026).into(),
        unlit: true,
        perceptual_roughness: 0.96,
        ..default()
    });
    let trousers = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.27, 0.42),
        emissive: Color::srgb(0.010, 0.020, 0.040).into(),
        unlit: true,
        perceptual_roughness: 0.96,
        ..default()
    });
    let boot = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.12, 0.055),
        unlit: true,
        perceptual_roughness: 0.98,
        ..default()
    });
    let eye = materials.add(StandardMaterial {
        base_color: Color::srgb(0.025, 0.025, 0.022),
        unlit: true,
        perceptual_roughness: 0.9,
        ..default()
    });
    let hair = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.12, 0.060),
        emissive: Color::srgb(0.060, 0.016, 0.006).into(),
        unlit: true,
        perceptual_roughness: 0.98,
        ..default()
    });
    let twig = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.12, 0.035),
        unlit: true,
        perceptual_roughness: 0.99,
        ..default()
    });

    // Build the torso as one continuous, low-resolution organic volume. This
    // follows the same silhouette-first idea as the fish asset: a single
    // readable body does the structural work, while the IK-driven limbs stay
    // separate only where animation requires it.
    let torso_mesh = meshes.add(build_player_torso_mesh());
    // Bevy 0.19's capsule builder requires at least four latitudes; three
    // causes an unsigned underflow while calculating the southern cap.
    let pants_mesh = meshes.add(Capsule3d::new(0.19, 0.0).mesh().longitudes(6).latitudes(4));
    let head_mesh = meshes.add(
        Sphere::new(0.215)
            .mesh()
            .ico(1)
            .expect("low-poly player head mesh should be valid"),
    );
    let hair_mesh = meshes.add(
        Sphere::new(0.225)
            .mesh()
            .ico(1)
            .expect("low-poly player hair mesh should be valid"),
    );
    let hair_lock_mesh = meshes.add(
        Capsule3d::new(0.058, 0.08)
            .mesh()
            .longitudes(6)
            .latitudes(4),
    );
    let neck_mesh = meshes.add(Cylinder::new(0.12, 0.22).mesh().resolution(6));
    let eye_mesh = meshes.add(Cuboid::new(0.045, 0.055, 0.018));
    let arm_mesh = meshes.add(
        // Segment transforms scale the Y axis by the solved IK length. Use a
        // unit-height source mesh so the capsule ends meet the joints instead
        // of ending short or overshooting them.
        Capsule3d::new(0.082, 0.836)
            .mesh()
            .longitudes(6)
            .latitudes(4),
    );
    let leg_mesh = meshes.add(Capsule3d::new(0.09, 0.82).mesh().longitudes(6).latitudes(4));
    let hand_mesh = meshes.add(
        Sphere::new(0.075)
            .mesh()
            .ico(1)
            .expect("low-poly player hand mesh should be valid"),
    );
    let boot_mesh = meshes.add(
        Capsule3d::new(0.085, 0.13)
            .mesh()
            .longitudes(6)
            .latitudes(4),
    );
    let stick_mesh = meshes.add(Cylinder::new(0.034, 1.0).mesh().resolution(6));

    for (kind, mesh, material) in [
        (PlayerIkPartKind::Torso, torso_mesh.clone(), shirt.clone()),
        (
            PlayerIkPartKind::Pants,
            pants_mesh.clone(),
            trousers.clone(),
        ),
        (PlayerIkPartKind::Neck, neck_mesh.clone(), skin.clone()),
        (PlayerIkPartKind::Head, head_mesh.clone(), skin.clone()),
        (PlayerIkPartKind::Hair, hair_mesh.clone(), hair.clone()),
        (
            PlayerIkPartKind::HairSideL,
            hair_lock_mesh.clone(),
            hair.clone(),
        ),
        (
            PlayerIkPartKind::HairSideR,
            hair_lock_mesh.clone(),
            hair.clone(),
        ),
        (PlayerIkPartKind::EyeL, eye_mesh.clone(), eye.clone()),
        (PlayerIkPartKind::EyeR, eye_mesh.clone(), eye.clone()),
        (PlayerIkPartKind::ArmUpperL, arm_mesh.clone(), skin.clone()),
        (PlayerIkPartKind::ArmLowerL, arm_mesh.clone(), skin.clone()),
        (PlayerIkPartKind::ArmUpperR, arm_mesh.clone(), skin.clone()),
        (PlayerIkPartKind::ArmLowerR, arm_mesh.clone(), skin.clone()),
        (PlayerIkPartKind::HandL, hand_mesh.clone(), skin.clone()),
        (PlayerIkPartKind::HandR, hand_mesh.clone(), skin.clone()),
        (
            PlayerIkPartKind::LegUpperL,
            leg_mesh.clone(),
            trousers.clone(),
        ),
        (
            PlayerIkPartKind::LegLowerL,
            leg_mesh.clone(),
            trousers.clone(),
        ),
        (
            PlayerIkPartKind::LegUpperR,
            leg_mesh.clone(),
            trousers.clone(),
        ),
        (
            PlayerIkPartKind::LegLowerR,
            leg_mesh.clone(),
            trousers.clone(),
        ),
        (PlayerIkPartKind::BootL, boot_mesh.clone(), boot.clone()),
        (PlayerIkPartKind::BootR, boot_mesh.clone(), boot.clone()),
        (PlayerIkPartKind::Stick, stick_mesh.clone(), twig.clone()),
    ] {
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(position),
            Visibility::Visible,
            PlayerIkPart { kind },
            Name::new("player_ik_part"),
        ));
    }
}

pub fn setup_water_fish(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    terrain: Res<ProceduralTerrainSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const FISH_COUNT: usize = 18;
    const SEAWEED_COUNT: usize = 42;
    let seaweed_mesh = meshes.add(build_seaweed_mesh());
    let seaweed_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.62, 0.34),
        emissive: Color::srgb(0.015, 0.13, 0.045).into(),
        perceptual_roughness: 0.72,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let water_demo_spawn = std::env::args()
        .any(|arg| arg == "--water-demo")
        .then(|| find_water_demo_spawn(&terrain))
        .flatten();
    let mut water_cells = Vec::new();
    let mut candidate: usize = 0;
    for z in (-56i32..56i32).step_by(4) {
        for x in (-56i32..56i32).step_by(4) {
            let position = Vec3::new(x as f32 + 0.5, 0.0, z as f32 + 0.5);
            let Some(depth) = terrain.water_depth_at(position) else {
                continue;
            };
            if depth < 0.55 {
                continue;
            }
            water_cells.push((position, depth, candidate));
            candidate = candidate.wrapping_add(1);
        }
    }
    if let Some(spawn) = water_demo_spawn {
        water_cells.sort_by(|(left, _, _), (right, _, _)| {
            let left_distance = left.distance_squared(spawn.position);
            let right_distance = right.distance_squared(spawn.position);
            left_distance.total_cmp(&right_distance)
        });
    }

    let mut spawned = 0;
    let mut seaweed_spawned = 0;
    for (position, depth, candidate) in water_cells {
        if spawned >= FISH_COUNT && seaweed_spawned >= SEAWEED_COUNT {
            break;
        }
        let variation = hash01(candidate, 31);
        let origin = Vec3::new(
            position.x,
            terrain.water_floor_height(position) + depth * (0.48 + variation * 0.18),
            position.z,
        );
        if spawned < FISH_COUNT && candidate % 2 == 0 {
            let fish_yaw = hash01(candidate, 23) * std::f32::consts::TAU;
            spawn_asset(
                &mut commands,
                &asset_server,
                FISH_PATH,
                origin,
                1.0,
                fish_yaw,
                format!("water_fish_{spawned}"),
            )
            .insert(WaterFish {
                origin,
                phase: hash01(candidate, 29) * std::f32::consts::TAU,
            });
            spawned += 1;
        }
        if seaweed_spawned < SEAWEED_COUNT && candidate % 2 == 1 {
            let height = 0.85 + hash01(candidate, 37) * 0.95;
            let seaweed_origin = Vec3::new(
                position.x,
                terrain.water_floor_height(position) + 0.02,
                position.z,
            );
            commands.spawn((
                Mesh3d(seaweed_mesh.clone()),
                MeshMaterial3d(seaweed_material.clone()),
                Transform::from_translation(seaweed_origin)
                    .with_scale(Vec3::new(1.0, height, 1.0))
                    .with_rotation(Quat::from_rotation_y(
                        hash01(candidate, 41) * std::f32::consts::TAU,
                    )),
                WaterSeaweed {
                    origin: seaweed_origin,
                    phase: hash01(candidate, 43) * std::f32::consts::TAU,
                    height,
                },
                ForestRealmVisual,
                Name::new(format!("water_seaweed_{seaweed_spawned}")),
            ));
            seaweed_spawned += 1;
        }
    }

    // Keep the validation view useful even when procedural terrain chooses a
    // different deep-water basin on a new run. The normal world distribution
    // above remains the gameplay content; this small local school and kelp
    // ring is only enabled by the explicit water-demo flag.
    if let Some(spawn) = water_demo_spawn {
        const DEMO_FISH_OFFSETS: [(f32, f32); 8] = [
            (0.0, -1.8),
            (1.8, -0.6),
            (-1.8, -0.4),
            (2.4, 1.2),
            (-2.4, 1.4),
            (0.6, 2.6),
            (-1.2, 3.1),
            (2.8, 3.0),
        ];
        for (index, (offset_x, offset_z)) in DEMO_FISH_OFFSETS.into_iter().enumerate() {
            let sample = spawn.position + Vec3::new(offset_x, 0.0, offset_z);
            let Some(depth) = terrain.water_depth_at(sample) else {
                continue;
            };
            if depth < 0.75 {
                continue;
            }
            let origin = Vec3::new(
                sample.x,
                terrain.water_floor_height(sample) + depth * (0.52 + hash01(index, 67) * 0.14),
                sample.z,
            );
            spawn_asset(
                &mut commands,
                &asset_server,
                FISH_PATH,
                origin,
                1.0,
                hash01(index, 71) * std::f32::consts::TAU,
                format!("water_demo_fish_{index}"),
            )
            .insert(WaterFish {
                origin,
                phase: hash01(index, 73) * std::f32::consts::TAU,
            });
        }

        const DEMO_SEAWEED_OFFSETS: [(f32, f32); 12] = [
            (-3.0, -2.5),
            (0.8, -2.8),
            (3.0, -1.8),
            (-2.8, 0.0),
            (2.8, 0.2),
            (-3.2, 2.2),
            (0.0, 2.4),
            (3.2, 2.8),
            (-1.8, 4.0),
            (1.5, 4.2),
            (-4.0, 1.0),
            (4.0, 1.4),
        ];
        for (index, (offset_x, offset_z)) in DEMO_SEAWEED_OFFSETS.into_iter().enumerate() {
            let sample = spawn.position + Vec3::new(offset_x, 0.0, offset_z);
            let Some(depth) = terrain.water_depth_at(sample) else {
                continue;
            };
            if depth < 0.55 {
                continue;
            }
            let height = 1.0 + hash01(index, 79) * 0.75;
            let origin = Vec3::new(
                sample.x,
                terrain.water_floor_height(sample) + 0.02,
                sample.z,
            );
            commands.spawn((
                Mesh3d(seaweed_mesh.clone()),
                MeshMaterial3d(seaweed_material.clone()),
                Transform::from_translation(origin)
                    .with_scale(Vec3::new(1.0, height, 1.0))
                    .with_rotation(Quat::from_rotation_y(
                        hash01(index, 83) * std::f32::consts::TAU,
                    )),
                WaterSeaweed {
                    origin,
                    phase: hash01(index, 89) * std::f32::consts::TAU,
                    height,
                },
                ForestRealmVisual,
                Name::new(format!("water_demo_seaweed_{index}")),
            ));
        }
    }
}

fn build_seaweed_mesh() -> Mesh {
    const BLADES: usize = 5;
    let mut positions = Vec::with_capacity(BLADES * 5);
    let mut indices = Vec::with_capacity(BLADES * 9);
    for blade in 0..BLADES {
        let angle = blade as f32 / BLADES as f32 * std::f32::consts::TAU + 0.22;
        let radial = Vec3::new(angle.cos(), 0.0, angle.sin());
        let side = Vec3::new(-angle.sin(), 0.0, angle.cos());
        let base_center = radial * (blade as f32 * 0.012);
        let middle_center = base_center + radial * (0.10 + blade as f32 * 0.018);
        let tip = base_center
            + radial * (0.18 + blade as f32 * 0.02)
            + side * (0.015 * (blade as f32 - 2.0));
        let width = 0.075 - blade as f32 * 0.006;
        let base = positions.len() as u32;
        positions.extend([
            base_center - side * width,
            base_center + side * width,
            middle_center - side * width * 0.62 + Vec3::Y * 0.46,
            middle_center + side * width * 0.62 + Vec3::Y * 0.46,
            tip + Vec3::Y,
        ]);
        indices.extend([
            base,
            base + 2,
            base + 1,
            base + 1,
            base + 2,
            base + 3,
            base + 2,
            base + 4,
            base + 3,
        ]);
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let uvs = positions
        .iter()
        .map(|position| [position[0] + 0.5, position[1]])
        .collect::<Vec<_>>();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

pub fn update_underwater_presentation(
    players: Query<&PlayerSwimState, With<PlayerActor>>,
    mut cameras: Query<(&mut Exposure, &mut DistanceFog), With<LivingSceneCamera>>,
    mut water: Query<&mut Visibility, With<WaterSurfaceVisual>>,
    mut clear_color: ResMut<ClearColor>,
) {
    let submerged = players.single().is_ok_and(|swim| swim.active);
    if let Ok((mut exposure, mut fog)) = cameras.single_mut() {
        if submerged {
            // A short, saturated visibility range gives the small toy-like
            // underwater scene depth without washing nearby ecology flat.
            *clear_color = ClearColor(Color::srgb(0.02, 0.22, 0.28));
            exposure.ev100 = 12.0;
            *fog = DistanceFog {
                color: Color::srgba(0.035, 0.30, 0.34, 0.36),
                directional_light_color: Color::srgba(0.20, 0.72, 0.70, 0.14),
                directional_light_exponent: 12.0,
                falloff: FogFalloff::Linear {
                    start: 1.2,
                    end: 38.0,
                },
            };
        } else {
            *clear_color = ClearColor(Color::srgb(0.47, 0.61, 0.72));
            exposure.ev100 = 11.5;
            fog.color = Color::srgba(0.47, 0.61, 0.72, 0.10);
            fog.directional_light_color = Color::NONE;
            fog.falloff = FogFalloff::Linear {
                start: 22.0,
                end: 58.0,
            };
        }
    }
    for mut visibility in &mut water {
        // The camera is already inside the water volume. Hiding the surface
        // while submerged prevents the opaque sea ceiling from depth-testing
        // in front of the floor, fish, and seaweed.
        *visibility = if submerged {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

fn spawn_legendary_weapon_visuals(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    terrain: &ProceduralTerrainSurface,
) {
    // These fixed pickups are an interaction/model-audit fixture, not normal
    // world loot. Keep them out of the authored scene unless content debug is
    // explicitly requested; the hidden held-weapon visuals below are still
    // loaded for the player's equipped-state presentation.
    if std::env::args().any(|arg| arg == "--content-debug-open") {
        let dragon_pickup_position = Vec3::new(
            -1.1,
            terrain.ground_height(Vec3::new(-1.1, 0.0, 9.5)) + 0.08,
            9.5,
        );
        spawn_asset(
            commands,
            asset_server,
            DRAGON_KATANA_PATH,
            dragon_pickup_position,
            0.28,
            0.45,
            "dragon_katana_pickup",
        )
        .insert(DragonKatanaPickup);
        let reaper_pickup_position = Vec3::new(
            1.2,
            terrain.ground_height(Vec3::new(1.2, 0.0, 9.5)) + 0.08,
            9.5,
        );
        spawn_asset(
            commands,
            asset_server,
            REAPER_SCYTHE_PATH,
            reaper_pickup_position,
            0.30,
            -0.35,
            "reaper_scythe_pickup",
        )
        .insert(ReaperScythePickup);
    }
    spawn_asset(
        commands,
        asset_server,
        DRAGON_KATANA_PATH,
        Vec3::ZERO,
        0.32,
        0.0,
        "player_dragon_katana",
    )
    .insert((
        HeldWeaponVisual {
            weapon: LegendaryWeapon::DragonKatana,
        },
        Visibility::Hidden,
    ));
    spawn_asset(
        commands,
        asset_server,
        REAPER_SCYTHE_PATH,
        Vec3::ZERO,
        0.34,
        0.0,
        "player_reaper_scythe",
    )
    .insert((
        HeldWeaponVisual {
            weapon: LegendaryWeapon::ReaperScythe,
        },
        Visibility::Hidden,
    ));
}
