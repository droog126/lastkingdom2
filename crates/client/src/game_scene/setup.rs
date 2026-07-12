//! Initial scene construction: rendering resources, lighting, terrain, the
//! living scene population (player, boss, trees, grass, clouds, rain), and
//! the camera. Everything runs once on `Startup`.

use avian3d::prelude::{Collider, LockedAxes, RigidBody, TransformInterpolation, TrimeshFlags};
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
use bevy::pbr::{AtmosphereSettings, ScreenSpaceAmbientOcclusion};
use bevy::prelude::*;
use bevy::render::camera::{MipBias, TemporalJitter};
use bevy_hanabi::EffectAsset;
use bevy_tnua::builtins::{TnuaBuiltinJumpConfig, TnuaBuiltinWalkConfig};
use bevy_tnua::prelude::{TnuaConfig, TnuaController};
use bevy_tnua_avian3d::prelude::TnuaAvian3dSensorShape;

use super::combat_fx::create_hit_effect;
use super::content_visuals::{
    LivingContentLayout, grounded_content_position, spawn_content_visuals,
    spawn_exported_content_visuals,
};
use super::offline::OfflineNature;
use super::player::{PlayerControlScheme, PlayerControlSchemeConfig};
use super::procedural_motion::ProceduralTreeSway;
use super::state::{
    BossActor, Cloud, CollisionDebugState, DragonKatanaPickup, FARM_PLOT_POSITIONS,
    FarmVisualMaterials, GrassTuft, HeldWeaponVisual, HitReaction, LivingCameraRig,
    LivingSceneCamera, LivingSun, PROCEDURAL_TERRAIN_CENTER, PROCEDURAL_TERRAIN_RADIUS,
    PlayerActor, PlayerIkPart, PlayerIkPartKind, PlayerJump, PlayerMotion, PlayerSkillState,
    ProceduralTerrainSurface, RainDrop, SceneMaterials, configure_collision_debug_gizmos,
};
use super::stylized_material::{
    StylizedTerrainExtension, StylizedTerrainMaterial, TreeShadowAssets,
};
use super::util::{
    AUTUMN_TREE_PATH, BIRCH_TREE_PATH, BOSS_PATH, BRANCH_PATH, CLOUD_PATH, DEER_FAWN_PATH,
    DRAGON_KATANA_PATH, FOREST_STONE_SPIRE_PATH, FOX_SILVER_PATH, PINE_TREE_PATH,
    PLAYER_JUMP_HEIGHT, PLAYER_PHYSICS_CENTER_HEIGHT, PLAYER_SPEED, RABBIT_BROWN_PATH, ROCK_PATH,
    ROUND_TREE_PATH, TREE_PATH, WILLOW_TREE_PATH, hash01, spawn_asset,
};
use lk2_core::pvp::{Health, PvpCombatant, SimpleWeapon};
use lk2_core::world::terrain::LandformModule;

const PROCEDURAL_TERRAIN_STEP: i32 = 2;
const PROCEDURAL_WATER_STEP: i32 = 1;

pub fn setup_rendering(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.47, 0.61, 0.72)));
    commands.insert_resource(DirectionalLightShadowMap { size: 2048 });
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.78, 0.86, 0.82),
        // Bevy 0.19 uses cd/m² here. Sub-unit values leave shadowed PBR
        // surfaces with no readable sky fill and turn them nearly black.
        brightness: 80.0,
        affects_lightmapped_meshes: true,
    });
}

pub fn setup_lighting(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<StylizedTerrainMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let ground = terrain_materials.add(stylized_terrain_material(
        Color::srgb(0.34, 0.57, 0.25),
        0.58,
        0.035,
        0.94,
    ));
    let sand = terrain_materials.add(stylized_terrain_material(
        Color::srgb(0.82, 0.69, 0.43),
        0.62,
        0.032,
        0.90,
    ));
    let mountain = terrain_materials.add(stylized_terrain_material(
        Color::srgb(0.43, 0.46, 0.43),
        0.60,
        0.035,
        0.92,
    ));
    let snow = terrain_materials.add(stylized_terrain_material(
        Color::srgb(0.86, 0.91, 0.92),
        0.64,
        0.030,
        0.90,
    ));
    let water = materials.add(StandardMaterial {
        base_color: Color::srgba(0.20, 0.50, 0.66, 0.88),
        unlit: true,
        metallic: 0.05,
        perceptual_roughness: 0.24,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let hit_effect = effects.add(create_hit_effect());
    let tree_shadow_mesh = meshes.add(Plane3d::default().mesh().size(2.0, 2.0));
    let tree_shadow_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.030, 0.050, 0.025, 0.28),
        base_color_texture: Some(asset_server.load("shadow/shadow.png")),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.insert_resource(SceneMaterials {
        ground: ground.clone(),
        hit_effect,
    });
    commands.insert_resource(TreeShadowAssets {
        mesh: tree_shadow_mesh,
        material: tree_shadow_material,
    });
    let terrain_surface = ProceduralTerrainSurface::default_world();
    commands.insert_resource(terrain_surface.clone());

    spawn_procedural_terrain(
        &mut commands,
        &mut meshes,
        &terrain_surface,
        &ground,
        &sand,
        &mountain,
        &snow,
        water,
    );
    let collision_mesh = build_collision_surface_mesh(&terrain_surface);
    let terrain_collider =
        Collider::trimesh_from_mesh_with_config(&collision_mesh, TrimeshFlags::all())
            .expect("procedural terrain collision mesh should produce a trimesh");
    commands.spawn((
        RigidBody::Static,
        terrain_collider,
        Name::new("procedural_terrain_collider"),
    ));
}

pub fn setup_collision_debug(
    mut gizmos: ResMut<GizmoConfigStore>,
    debug: Res<CollisionDebugState>,
) {
    configure_collision_debug_gizmos(&mut gizmos, debug.enabled);
}

fn build_collision_surface_mesh(terrain: &ProceduralTerrainSurface) -> Mesh {
    // Keep the physics grid aligned with the 1x1 water cells so the water edge
    // does not become a diagonal wall between a water vertex and a land vertex.
    const STEP: i32 = PROCEDURAL_WATER_STEP;
    let mut positions = Vec::new();
    let mut indices = Vec::new();

    for z in (-PROCEDURAL_TERRAIN_RADIUS..PROCEDURAL_TERRAIN_RADIUS).step_by(STEP as usize) {
        for x in (-PROCEDURAL_TERRAIN_RADIUS..PROCEDURAL_TERRAIN_RADIUS).step_by(STEP as usize) {
            if [(x, z), (x + STEP, z), (x + STEP, z + STEP), (x, z + STEP)]
                .into_iter()
                .any(|(cell_x, cell_z)| terrain_cell_is_water(terrain, cell_x, cell_z))
            {
                continue;
            }
            let base = positions.len() as u32;
            positions.extend([
                physics_collision_vertex(terrain, x, z),
                physics_collision_vertex(terrain, x + STEP, z),
                physics_collision_vertex(terrain, x + STEP, z + STEP),
                physics_collision_vertex(terrain, x, z + STEP),
            ]);
            indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
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
        terrain.ground_height(Vec3::new(x as f32, 0.0, z as f32)),
        z as f32,
    ]
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
    water: Handle<StandardMaterial>,
) {
    let pipeline = terrain_surface.pipeline.as_ref();
    let landform = &terrain_surface.landform;
    let terrain_meshes = build_procedural_surface_meshes(pipeline, landform, terrain_surface);
    let water_mesh = build_procedural_water_mesh(pipeline, terrain_surface);

    for (mesh, material, name) in [
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
    ] {
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            Name::new(name),
        ));
    }
    commands.spawn((
        Mesh3d(meshes.add(water_mesh)),
        MeshMaterial3d(water),
        Name::new("procedural_water_surface"),
    ));
}

fn build_procedural_surface_meshes(
    pipeline: &lk2_core::world::terrain::TerrainPipeline,
    landform: &LandformModule,
    terrain_surface: &ProceduralTerrainSurface,
) -> [Mesh; 4] {
    let mut data = std::array::from_fn(|_| SurfaceMeshData::default());
    for z in (-PROCEDURAL_TERRAIN_RADIUS..PROCEDURAL_TERRAIN_RADIUS)
        .step_by(PROCEDURAL_TERRAIN_STEP as usize)
    {
        for x in (-PROCEDURAL_TERRAIN_RADIUS..PROCEDURAL_TERRAIN_RADIUS)
            .step_by(PROCEDURAL_TERRAIN_STEP as usize)
        {
            let world_x = PROCEDURAL_TERRAIN_CENTER + x;
            let world_z = PROCEDURAL_TERRAIN_CENTER + z;
            let surface = pipeline
                .surface_f32(world_x, world_z)
                .unwrap_or(lk2_core::constant::SEA_LEVEL as f32 + 1.0);
            let category = terrain_surface_category(surface, landform, world_x, world_z);
            let heights = [
                terrain_vertex_height(pipeline, terrain_surface, world_x, world_z),
                terrain_vertex_height(
                    pipeline,
                    terrain_surface,
                    world_x + PROCEDURAL_TERRAIN_STEP,
                    world_z,
                ),
                terrain_vertex_height(
                    pipeline,
                    terrain_surface,
                    world_x + PROCEDURAL_TERRAIN_STEP,
                    world_z + PROCEDURAL_TERRAIN_STEP,
                ),
                terrain_vertex_height(
                    pipeline,
                    terrain_surface,
                    world_x,
                    world_z + PROCEDURAL_TERRAIN_STEP,
                ),
            ];
            data[category].push_quad(
                [x as f32, z as f32],
                PROCEDURAL_TERRAIN_STEP as f32,
                heights,
            );
        }
    }
    data.map(SurfaceMeshData::into_mesh)
}

fn build_procedural_water_mesh(
    pipeline: &lk2_core::world::terrain::TerrainPipeline,
    terrain_surface: &ProceduralTerrainSurface,
) -> Mesh {
    let mut data = SurfaceMeshData::default();
    let sea_y = terrain_surface.render_height(lk2_core::constant::SEA_LEVEL as f32);
    for z in (-PROCEDURAL_TERRAIN_RADIUS..PROCEDURAL_TERRAIN_RADIUS)
        .step_by(PROCEDURAL_WATER_STEP as usize)
    {
        for x in (-PROCEDURAL_TERRAIN_RADIUS..PROCEDURAL_TERRAIN_RADIUS)
            .step_by(PROCEDURAL_WATER_STEP as usize)
        {
            let world_x = PROCEDURAL_TERRAIN_CENTER + x;
            let world_z = PROCEDURAL_TERRAIN_CENTER + z;
            let surface = pipeline
                .surface_f32(world_x, world_z)
                .unwrap_or(lk2_core::constant::SEA_LEVEL as f32 + 1.0);
            if terrain_surface.is_water_at(world_x, world_z, surface) {
                data.push_quad(
                    [x as f32, z as f32],
                    PROCEDURAL_WATER_STEP as f32,
                    [sea_y; 4],
                );
            }
        }
    }
    data.into_mesh()
}

fn terrain_vertex_height(
    pipeline: &lk2_core::world::terrain::TerrainPipeline,
    terrain_surface: &ProceduralTerrainSurface,
    x: i32,
    z: i32,
) -> f32 {
    terrain_surface.render_height(
        pipeline
            .surface_f32(x, z)
            .unwrap_or(lk2_core::constant::SEA_LEVEL as f32 + 1.0),
    )
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

#[derive(Default)]
struct SurfaceMeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl SurfaceMeshData {
    fn push_quad(&mut self, origin: [f32; 2], size: f32, heights: [f32; 4]) {
        let base = self.positions.len() as u32;
        let [x, z] = origin;
        self.positions.extend([
            [x, heights[0], z],
            [x + size, heights[1], z],
            [x + size, heights[2], z + size],
            [x, heights[3], z + size],
        ]);
        let slope_x = (heights[1] + heights[2] - heights[0] - heights[3]) / (2.0 * size);
        let slope_z = (heights[2] + heights[3] - heights[0] - heights[1]) / (2.0 * size);
        let normal = Vec3::new(-slope_x, 1.0, -slope_z).normalize_or_zero();
        self.normals.extend([[normal.x, normal.y, normal.z]; 4]);
        self.uvs
            .extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        self.indices
            .extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }

    fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

pub fn setup_living_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    nature: Res<OfflineNature>,
    content_layout: Res<LivingContentLayout>,
    content_registry: Res<lk2_core::content::ContentRegistry>,
    terrain: Res<ProceduralTerrainSurface>,
    mut control_scheme_configs: ResMut<Assets<PlayerControlSchemeConfig>>,
) {
    spawn_content_visuals(&mut commands, &asset_server, &content_layout, &terrain);
    spawn_exported_content_visuals(&mut commands, &asset_server, &content_registry, &terrain);

    spawn_player_avatar(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(
            -2.0,
            terrain.ground_height(Vec3::new(-2.0, 0.0, 11.0)) + PLAYER_PHYSICS_CENTER_HEIGHT,
            11.0,
        ),
        std::f32::consts::PI,
        &mut control_scheme_configs,
    );
    spawn_dragon_katana_visuals(&mut commands, &asset_server, &terrain);
    let spire_base = grounded_content_position(&terrain, Vec3::new(-2.0, 0.0, 0.5));
    spawn_asset(
        &mut commands,
        &asset_server,
        FOREST_STONE_SPIRE_PATH,
        spire_base,
        1.12,
        -0.08,
        "forest_stone_spire",
    );
    spawn_static_collider(
        &mut commands,
        spire_base + Vec3::Y * 0.55,
        Quat::from_rotation_y(-0.08),
        Collider::cuboid(2.2, 1.1, 1.7),
        "forest_stone_spire_collider",
    );
    let boss_pos = Vec3::new(1.5, 0.05, -12.0);
    spawn_asset(
        &mut commands,
        &asset_server,
        BOSS_PATH,
        boss_pos,
        0.92,
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
        Collider::capsule(0.72, 1.8),
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
        (DEER_FAWN_PATH, Vec3::new(-5.0, 0.0, 9.0), 1.05, 0.35),
        (FOX_SILVER_PATH, Vec3::new(2.5, 0.0, 8.5), 1.00, -0.8),
        (RABBIT_BROWN_PATH, Vec3::new(-1.0, 0.0, 6.5), 1.10, 1.2),
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
        let pos = Vec3::new(angle.cos() * radius, 0.03, angle.sin() * radius);
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

    let grass_mesh = meshes.add(Cone::new(0.17, 0.72));
    let grass_materials = [
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.27, 0.66, 0.23),
            perceptual_roughness: 0.96,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.42, 0.74, 0.25),
            perceptual_roughness: 0.94,
            ..default()
        }),
    ];
    const GRASS_COUNT: usize = 420;
    for index in 0..GRASS_COUNT {
        let angle = hash01(index, 17) * std::f32::consts::TAU;
        let radius = 4.5 + hash01(index, 29).sqrt() * 39.0;
        let ground_pos = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let pos = Vec3::new(
            ground_pos.x,
            terrain.ground_height(ground_pos) + 0.02,
            ground_pos.z,
        );
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
        let base = Vec3::new(cloud.x, 11.0, cloud.z);
        spawn_asset(
            &mut commands,
            &asset_server,
            CLOUD_PATH,
            base,
            1.6,
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
                Name::new("pooled_rain_drop"),
            ));
        }
    }
}

pub fn setup_farm_plots(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let soil_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.17, 0.09),
        perceptual_roughness: 0.98,
        ..default()
    });
    let border_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.29, 0.13),
        perceptual_roughness: 0.96,
        ..default()
    });
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
    let soil_mesh = meshes.add(Cuboid::new(3.8, 0.12, 3.0));
    let border_mesh = meshes.add(Cuboid::new(4.1, 0.16, 3.3));
    let crop_mesh = meshes.add(Cone::new(0.26, 0.92));
    commands.insert_resource(FarmVisualMaterials {
        crop_materials: crop_materials.clone(),
    });

    for (id, position) in FARM_PLOT_POSITIONS.into_iter().enumerate() {
        commands.spawn((
            Mesh3d(border_mesh.clone()),
            MeshMaterial3d(border_material.clone()),
            Transform::from_translation(position - Vec3::Y * 0.01),
            Name::new("farm_plot_border"),
        ));
        commands.spawn((
            Mesh3d(soil_mesh.clone()),
            MeshMaterial3d(soil_material.clone()),
            Transform::from_translation(position + Vec3::Y * 0.08),
            Name::new("farm_plot_soil"),
        ));
        commands.spawn((
            Mesh3d(crop_mesh.clone()),
            MeshMaterial3d(crop_materials[0].clone()),
            Transform::from_translation(position + Vec3::Y * 0.58).with_scale(Vec3::ZERO),
            Visibility::Hidden,
            super::state::FarmCropVisual { id: id as u32 },
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
        Transform::from_translation(position).with_rotation(rotation),
        Name::new(name),
    ));
}

pub fn setup_camera(mut commands: Commands, debug: Res<CollisionDebugState>) {
    commands.insert_resource(LivingCameraRig {
        mode: if debug.enabled {
            super::state::CameraMode::ThirdPerson
        } else {
            super::state::CameraMode::FirstPerson
        },
        ..LivingCameraRig::default()
    });
    commands
        .spawn((
            Camera3d::default(),
            Hdr,
            Transform::from_xyz(20.0, 18.0, 26.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
            AtmosphereSettings::default(),
            AtmosphereEnvironmentMapLight::default(),
            Exposure { ev100: 12.7 },
            Tonemapping::AcesFitted,
            Msaa::Off,
            ScreenSpaceAmbientOcclusion::default(),
            ShadowFilteringMethod::Gaussian,
            TemporalAntiAliasing::default(),
            TemporalJitter::default(),
            MipBias(0.0),
            DepthPrepass,
            MotionVectorPrepass,
        ))
        .insert(LivingSceneCamera);
}

fn spawn_player_avatar(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    yaw: f32,
    control_scheme_configs: &mut ResMut<Assets<PlayerControlSchemeConfig>>,
) {
    commands.spawn((
        (
            Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
            PlayerActor,
            PlayerMotion::default(),
            PlayerJump::default(),
            PlayerSkillState::default(),
            PvpCombatant::default(),
            SimpleWeapon::default(),
            Health::default(),
        ),
        (
            RigidBody::Dynamic,
            Collider::capsule(0.38, 1.0),
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
    ));

    let skin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.63, 0.43),
        emissive: Color::srgb(0.018, 0.008, 0.004).into(),
        perceptual_roughness: 0.95,
        ..default()
    });
    let shirt = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.52, 0.30),
        emissive: Color::srgb(0.008, 0.022, 0.010).into(),
        perceptual_roughness: 0.96,
        ..default()
    });
    let trousers = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.36, 0.55),
        emissive: Color::srgb(0.006, 0.012, 0.024).into(),
        perceptual_roughness: 0.96,
        ..default()
    });
    let boot = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.13, 0.07),
        perceptual_roughness: 0.98,
        ..default()
    });
    let eye = materials.add(StandardMaterial {
        base_color: Color::srgb(0.025, 0.025, 0.022),
        perceptual_roughness: 0.9,
        ..default()
    });
    let basket = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.36, 0.16),
        perceptual_roughness: 0.99,
        ..default()
    });
    let twig = materials.add(StandardMaterial {
        base_color: Color::srgb(0.36, 0.20, 0.09),
        perceptual_roughness: 0.99,
        ..default()
    });

    let torso_mesh = meshes.add(Cuboid::new(0.46, 0.62, 0.32));
    let pants_mesh = meshes.add(Cuboid::new(0.38, 0.30, 0.30));
    let head_mesh = meshes.add(Sphere::new(0.34));
    let neck_mesh = meshes.add(Cylinder::new(0.12, 0.22));
    let eye_mesh = meshes.add(Cuboid::new(0.045, 0.055, 0.018));
    let arm_mesh = meshes.add(Cylinder::new(0.055, 1.0));
    let leg_mesh = meshes.add(Cylinder::new(0.070, 1.0));
    let hand_mesh = meshes.add(Sphere::new(0.075));
    let boot_mesh = meshes.add(Cuboid::new(0.17, 0.12, 0.30));
    let basket_mesh = meshes.add(Cuboid::new(0.36, 0.28, 0.16));
    let stick_mesh = meshes.add(Cylinder::new(0.026, 1.0));

    for (kind, mesh, material) in [
        (PlayerIkPartKind::Torso, torso_mesh.clone(), shirt.clone()),
        (
            PlayerIkPartKind::Pants,
            pants_mesh.clone(),
            trousers.clone(),
        ),
        (PlayerIkPartKind::Neck, neck_mesh.clone(), skin.clone()),
        (PlayerIkPartKind::Head, head_mesh.clone(), skin.clone()),
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
        (
            PlayerIkPartKind::Basket,
            basket_mesh.clone(),
            basket.clone(),
        ),
        (PlayerIkPartKind::Stick, stick_mesh.clone(), twig.clone()),
    ] {
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(position),
            PlayerIkPart { kind },
            Name::new("player_ik_part"),
        ));
    }
}

fn spawn_dragon_katana_visuals(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    terrain: &ProceduralTerrainSurface,
) {
    let pickup_position = Vec3::new(
        -1.1,
        terrain.ground_height(Vec3::new(-1.1, 0.0, 9.5)) + 0.08,
        9.5,
    );
    spawn_asset(
        commands,
        asset_server,
        DRAGON_KATANA_PATH,
        pickup_position,
        0.28,
        0.45,
        "dragon_katana_pickup",
    )
    .insert(DragonKatanaPickup);
    spawn_asset(
        commands,
        asset_server,
        DRAGON_KATANA_PATH,
        Vec3::ZERO,
        0.32,
        0.0,
        "player_dragon_katana",
    )
    .insert((HeldWeaponVisual, Visibility::Hidden));
}
