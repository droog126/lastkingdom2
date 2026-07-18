//! Center-screen object identification and lightweight target highlighting.

use std::collections::HashMap;

use bevy::camera::primitives::Aabb as BevyAabb;
use bevy::prelude::*;

use super::state::{Inspectable, LivingSceneCamera};
use super::stylized_material::StylizedTerrainMaterial;
use crate::ray_aabb::{RayAabb, ray_aabb_hit};

const TARGET_RIM_STRENGTH: f32 = 0.92;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetSelection {
    pub entity: Entity,
    pub inspectable: Inspectable,
}

#[derive(Resource, Default)]
pub struct TargetingState {
    pub selection: Option<TargetSelection>,
    original_rim_strength: HashMap<AssetId<StylizedTerrainMaterial>, f32>,
}

/// Casts from the screen center, resolves a mesh hit back to an inspectable
/// GLTF root, and updates the material-local rim on the selected hierarchy.
pub fn update_targeting(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<LivingSceneCamera>>,
    roots: Query<(Entity, &Inspectable, Option<&Children>, Option<&Visibility>)>,
    children_query: Query<&Children>,
    mesh_bounds: Query<(&BevyAabb, &GlobalTransform)>,
    mesh_materials: Query<&MeshMaterial3d<StylizedTerrainMaterial>>,
    mut materials: ResMut<Assets<StylizedTerrainMaterial>>,
    mut state: ResMut<TargetingState>,
) {
    let Some(ray) = screen_center_ray(&windows, &cameras) else {
        clear_selection(&mut state, &children_query, &mesh_materials, &mut materials);
        return;
    };

    let mut nearest: Option<(TargetSelection, f32)> = None;
    for (entity, inspectable, _, visibility) in &roots {
        if visibility.is_some_and(|visibility| *visibility == Visibility::Hidden) {
            continue;
        }
        let Some(bounds) = hierarchy_bounds(entity, &children_query, &mesh_bounds) else {
            continue;
        };
        let Some(hit) = ray_aabb_hit(ray.origin, *ray.direction, bounds.min, bounds.max) else {
            continue;
        };
        if nearest.is_none_or(|(_, distance)| hit.distance < distance) {
            nearest = Some((
                TargetSelection {
                    entity,
                    inspectable: *inspectable,
                },
                hit.distance,
            ));
        }
    }

    let next = nearest.map(|(selection, _)| selection);
    if state.selection.map(|selection| selection.entity) != next.map(|selection| selection.entity) {
        if let Some(previous) = state.selection {
            set_hierarchy_highlight(
                previous.entity,
                false,
                &children_query,
                &mesh_materials,
                &mut materials,
                &mut state,
            );
        }
        state.selection = next;
    }

    if let Some(selection) = state.selection {
        set_hierarchy_highlight(
            selection.entity,
            true,
            &children_query,
            &mesh_materials,
            &mut materials,
            &mut state,
        );
    }
}

fn screen_center_ray(
    windows: &Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<LivingSceneCamera>>,
) -> Option<Ray3d> {
    let window = windows.single().ok()?;
    let (camera, camera_transform) = cameras.single().ok()?;
    let center = Vec2::new(
        window.resolution.width() * 0.5,
        window.resolution.height() * 0.5,
    );
    camera.viewport_to_world(camera_transform, center).ok()
}

fn hierarchy_bounds(
    root: Entity,
    children_query: &Query<&Children>,
    mesh_bounds: &Query<(&BevyAabb, &GlobalTransform)>,
) -> Option<RayAabb> {
    let mut stack = vec![root];
    let mut bounds: Option<RayAabb> = None;
    while let Some(entity) = stack.pop() {
        if let Ok((aabb, transform)) = mesh_bounds.get(entity) {
            let world_aabb = transformed_mesh_aabb(aabb, transform);
            bounds = Some(match bounds {
                Some(current) => RayAabb::new(
                    current.min.min(world_aabb.min),
                    current.max.max(world_aabb.max),
                ),
                None => world_aabb,
            });
        }
        if let Ok(children) = children_query.get(entity) {
            stack.extend(children.iter());
        }
    }
    bounds
}

fn transformed_mesh_aabb(aabb: &BevyAabb, transform: &GlobalTransform) -> RayAabb {
    let world_from_local = transform.affine();
    let center = world_from_local.transform_point3a(aabb.center);
    let half_extents = world_from_local.matrix3.abs() * aabb.half_extents.abs();
    RayAabb::new(
        (center - half_extents).into(),
        (center + half_extents).into(),
    )
}

fn clear_selection(
    state: &mut TargetingState,
    children_query: &Query<&Children>,
    mesh_materials: &Query<&MeshMaterial3d<StylizedTerrainMaterial>>,
    materials: &mut ResMut<Assets<StylizedTerrainMaterial>>,
) {
    if let Some(previous) = state.selection.take() {
        set_hierarchy_highlight(
            previous.entity,
            false,
            children_query,
            mesh_materials,
            materials,
            state,
        );
    }
}

fn set_hierarchy_highlight(
    root: Entity,
    highlighted: bool,
    children_query: &Query<&Children>,
    mesh_materials: &Query<&MeshMaterial3d<StylizedTerrainMaterial>>,
    materials: &mut ResMut<Assets<StylizedTerrainMaterial>>,
    state: &mut TargetingState,
) {
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Ok(mesh_material) = mesh_materials.get(entity) {
            let material_id = mesh_material.0.id();
            if let Some(mut material) = materials.get_mut(&mesh_material.0) {
                if highlighted {
                    state
                        .original_rim_strength
                        .entry(material_id)
                        .or_insert(material.extension.shadow_settings.w);
                    material.extension.shadow_settings.w = TARGET_RIM_STRENGTH;
                } else if let Some(original) = state.original_rim_strength.remove(&material_id) {
                    material.extension.shadow_settings.w = original;
                }
            }
        }
        if let Ok(children) = children_query.get(entity) {
            stack.extend(children.iter());
        }
    }
}
