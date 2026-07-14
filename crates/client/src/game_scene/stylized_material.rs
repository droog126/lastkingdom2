//! Stylized PBR material extensions used by the living forest scene.

use std::collections::HashMap;

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

pub const STYLIZED_TERRAIN_SHADER: &str = "shaders/stylized_terrain.wgsl";

pub type StylizedTerrainMaterial = ExtendedMaterial<StandardMaterial, StylizedTerrainExtension>;
pub type GrassWindMaterial = ExtendedMaterial<StandardMaterial, GrassWindExtension>;

#[derive(Resource, Clone)]
pub struct TreeShadowAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct StylizedAssetRoot;

#[derive(Component)]
pub struct StylizedTreeAsset;

#[derive(Component)]
pub struct StylizedReadableAsset;

#[derive(Component)]
pub(crate) struct StylizedAssetMaterialsApplied;

#[derive(Component)]
pub(crate) struct StylizedTreeShadowApplied;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct StylizedTerrainExtension {
    /// X: shadow multiplier, Y: linear ambient lift for shadowed surfaces.
    #[uniform(100)]
    pub shadow_settings: Vec4,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct GrassWindExtension {
    #[uniform(100)]
    pub wind: Vec4,
}

impl GrassWindExtension {
    pub const fn new() -> Self {
        Self {
            wind: Vec4::new(0.0, 0.055, 1.0, 0.0),
        }
    }
}

impl MaterialExtension for GrassWindExtension {
    fn vertex_shader() -> ShaderRef {
        "shaders/grass_wind.wgsl".into()
    }
}

pub fn animate_grass_gpu_wind(
    time: Res<Time>,
    mut materials: ResMut<Assets<GrassWindMaterial>>,
    grass: Query<&MeshMaterial3d<GrassWindMaterial>>,
) {
    let mut updated = std::collections::HashSet::new();
    for handle in &grass {
        if updated.insert(handle.0.id()) {
            if let Some(mut material) = materials.get_mut(&handle.0) {
                material.extension.wind.x = time.elapsed_secs();
            }
        }
    }
}

impl StylizedTerrainExtension {
    pub const fn new(shadow_floor: f32, shadow_lift: f32) -> Self {
        Self::with_cel_steps(shadow_floor, shadow_lift, 4.0)
    }

    pub const fn with_cel_steps(shadow_floor: f32, shadow_lift: f32, cel_steps: f32) -> Self {
        Self {
            shadow_settings: Vec4::new(shadow_floor, shadow_lift, cel_steps, 0.0),
        }
    }
}

impl MaterialExtension for StylizedTerrainExtension {
    fn fragment_shader() -> ShaderRef {
        STYLIZED_TERRAIN_SHADER.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        STYLIZED_TERRAIN_SHADER.into()
    }
}

/// Converts loaded GLTF materials to the same stylized PBR path as the
/// procedural terrain while preserving their textures and base colors.
///
/// Every asset spawned through `spawn_asset` uses this path. Keeping imported
/// props and ecology assets on the same shadow floor prevents small objects
/// from disappearing into black PBR shadows while retaining their authored
/// material colors.
pub fn stylize_asset_materials(
    mut commands: Commands,
    roots: Query<
        (Entity, &Children, &Transform, Option<&StylizedTreeAsset>),
        (
            With<StylizedAssetRoot>,
            Without<StylizedAssetMaterialsApplied>,
        ),
    >,
    children_query: Query<&Children>,
    mesh_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    source_materials: Res<Assets<StandardMaterial>>,
    mut stylized_materials: ResMut<Assets<StylizedTerrainMaterial>>,
    shadow_assets: Res<TreeShadowAssets>,
    terrain: Res<super::state::ProceduralTerrainSurface>,
) {
    for (root, root_children, root_transform, tree_asset) in &roots {
        let mut pending_assets = false;
        let mut converted = 0;
        let mut stack = root_children.iter().collect::<Vec<_>>();
        let mut cache = HashMap::new();

        while let Some(entity) = stack.pop() {
            if let Ok(children) = children_query.get(entity) {
                stack.extend(children.iter());
            }

            let Ok(mesh_material) = mesh_materials.get(entity) else {
                continue;
            };
            let material_id = mesh_material.0.id();
            let Some(handle) = cache.get(&material_id).cloned().or_else(|| {
                let base = source_materials.get(&mesh_material.0)?.clone();
                let handle = stylized_materials.add(StylizedTerrainMaterial {
                    base,
                    // Preserve hue and silhouette detail in tree/building
                    // shadows while retaining the directional shadow shape.
                    extension: StylizedTerrainExtension::new(0.70, 0.045),
                });
                cache.insert(material_id, handle.clone());
                Some(handle)
            }) else {
                pending_assets = true;
                continue;
            };

            commands
                .entity(entity)
                .insert(MeshMaterial3d(handle))
                .remove::<MeshMaterial3d<StandardMaterial>>();
            converted += 1;
        }

        if converted > 0 && !pending_assets {
            if tree_asset.is_some() {
                let base = root_transform.translation;
                let shadow_y = terrain.ground_height(base) + 0.035;
                let scale = root_transform
                    .scale
                    .x
                    .abs()
                    .max(root_transform.scale.z.abs());
                commands.spawn((
                    Mesh3d(shadow_assets.mesh.clone()),
                    MeshMaterial3d(shadow_assets.material.clone()),
                    Transform::from_xyz(base.x + 0.30, shadow_y, base.z - 0.22)
                        .with_rotation(Quat::from_rotation_y(-0.55))
                        .with_scale(Vec3::new(1.35 * scale, 1.0, 0.72 * scale)),
                    Name::new("stylized_tree_ground_shadow"),
                ));
                commands.entity(root).insert(StylizedTreeShadowApplied);
            }
            commands.entity(root).insert(StylizedAssetMaterialsApplied);
        }
    }
}
