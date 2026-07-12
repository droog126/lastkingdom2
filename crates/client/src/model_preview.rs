//! Interactive `--model-preview` mode.
//!
//! This is a small in-game showroom for judging GLB assets without terrain,
//! gameplay, or camera automation getting in the way.
//!
//! Run:
//!   cargo run -p lk2-client -- --model-preview
//!
//! Useful options:
//!   --model-preview-all   recursively show every GLB under assets/
//!   --model-preview-one=<name-or-path>  show one GLB by file stem or asset path
//!   --model-preview-shot  save screenshot and exit
//!     - for `--model-preview-one=<name>` the PNG is written to
//!       `screenshots/model_preview/<name>.png`
//!     - otherwise it goes to `screenshots/model_preview/model_preview.png`

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::Exposure;
use bevy::camera::primitives::Aabb as BevyAabb;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::VolumetricLight;
use bevy::pbr::{
    AtmosphereSettings, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
    ScreenSpaceReflections,
};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::render::{
    RenderPlugin,
    settings::{Backends, WgpuSettings},
};
use bevy::text::LetterSpacing;
use bevy::window::{PresentMode, PrimaryWindow, WindowResolution};
use bevy_world_serialization::WorldAsset;

use crate::ray_aabb::{RayAabb, nearest_ray_aabb_hit};

const FEATURED_MODELS: &[&str] = &[
    "procedural/pretty/sokpop_gatherer.glb",
    "procedural/pretty/villager.glb",
    "procedural/pretty/wolf.glb",
    "procedural/pretty/hoplite_ender_dragon.glb",
    "procedural/pretty/sokpop_tree.glb",
    "procedural/pretty/granular_round_tree.glb",
    "procedural/pretty/granular_pine_tree.glb",
    "procedural/pretty/granular_birch_tree.glb",
    "procedural/pretty/granular_autumn_tree.glb",
    "procedural/pretty/granular_willow_tree.glb",
    "procedural/pretty/granular_wildflowers.glb",
    "procedural/pretty/ground_patch.glb",
    "procedural/pretty/house_small.glb",
    "procedural/pretty/windmill.glb",
    "procedural/pretty/bridge_stone.glb",
    "procedural/pretty/lighthouse.glb",
    "procedural/pretty/campfire.glb",
    "procedural/pretty/treasure_chest.glb",
    "procedural/pretty/boat.glb",
    "procedural/pretty/cart.glb",
    "procedural/pretty/hoplite_reaper_scythe.glb",
    "procedural/pretty/hoplite_dragon_katana.glb",
    "procedural/pretty/hoplite_golem_hammer.glb",
    "procedural/pretty/hoplite_midas_sword.glb",
];

pub const MODEL_PREVIEW_OUTPUT_DIR: &str = "screenshots/model_preview";

const CELL_SIZE: f32 = 3.6;
const CELLS_PER_ROW: usize = 4;
const STABILIZATION_FRAMES: u64 = 90;
const PREVIEW_AABB_RADIUS: f32 = 1.35;
const PREVIEW_AABB_HEIGHT: f32 = 2.7;
const MODEL_GAP: f32 = 1.4;

#[derive(Resource)]
pub struct ModelPreviewState {
    pub frame: u64,
    pub auto_shot: bool,
    pub shot_requested: bool,
    pub exit_deadline: Option<Instant>,
    pub png_path: PathBuf,
    pub manifest_path: PathBuf,
    pub asset_root: PathBuf,
    pub scene_handles: Vec<Handle<WorldAsset>>,
    pub models: Vec<PreviewEntry>,
    pub single_filter: Option<String>,
    pub focused_index: usize,
    pub hovered_index: Option<usize>,
    pub auto_rotate: bool,
    pub layout_ready: bool,
}

#[derive(Component)]
struct PreviewSun;

#[derive(Resource)]
struct PreviewCamera {
    yaw: f32,
    distance: f32,
    height: f32,
    center: Vec3,
}

struct PackedPreviewLayout {
    positions: Vec<Vec3>,
    width: f32,
    depth: f32,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PreviewEntry {
    pub path: String,
    pub display_name: String,
    pub category: String,
    pub file_size_kb: u64,
    pub cell: [usize; 2],
    pub world_pos: [f32; 3],
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub aabb_ready: bool,
}

#[derive(Component)]
pub struct PreviewModelRoot {
    index: usize,
}

#[derive(Component)]
pub struct PreviewBaseDisc {
    index: usize,
}

#[derive(Resource, Clone)]
struct PreviewBaseMaterials {
    normal: Handle<StandardMaterial>,
    hovered: Handle<StandardMaterial>,
    focused: Handle<StandardMaterial>,
}

#[derive(Component)]
struct PreviewCameraMarker;

#[derive(Component)]
struct PreviewOverlayText;

pub fn run_model_preview() {
    let args: Vec<String> = std::env::args().collect();
    let show_all = args.iter().any(|a| a == "--model-preview-all");
    let auto_shot = args.iter().any(|a| a == "--model-preview-shot");
    let single_filter = args
        .iter()
        .find_map(|a| {
            a.strip_prefix("--model-preview-one=")
                .or_else(|| a.strip_prefix("--model-preview-model="))
                .or_else(|| a.strip_prefix("--model-preview-path="))
        })
        .map(str::to_string);

    let asset_root = workspace_asset_root();
    let output_dir = PathBuf::from(MODEL_PREVIEW_OUTPUT_DIR);
    let _ = std::fs::create_dir_all(&output_dir);
    // When previewing a single named model, write to a per-model PNG so a
    // driver loop can iterate every GLB without overwriting the previous shot.
    let png_path = match single_filter.as_deref() {
        Some(filter) => {
            let stem = normalize_filter(filter)
                .rsplit('/')
                .next()
                .unwrap_or("model")
                .to_string();
            output_dir.join(format!("{stem}.png"))
        }
        None => output_dir.join("model_preview.png"),
    };
    if auto_shot {
        let _ = std::fs::remove_file(&png_path);
    }

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: Some(Backends::VULKAN),
                    ..default()
                }
                .into(),
                ..default()
            })
            .set(AssetPlugin {
                file_path: asset_root.to_string_lossy().into_owned(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "lk2 model preview".into(),
                    resolution: WindowResolution::new(1280, 720),
                    present_mode: PresentMode::Immediate,
                    focused: true,
                    visible: true,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::log::LogPlugin {
                level: bevy::log::Level::INFO,
                ..default()
            }),
    );

    app.insert_resource(ModelPreviewState {
        frame: 0,
        auto_shot,
        shot_requested: false,
        exit_deadline: None,
        png_path,
        manifest_path: output_dir.join("manifest.json"),
        asset_root,
        scene_handles: Vec::new(),
        models: Vec::new(),
        single_filter,
        focused_index: 0,
        hovered_index: None,
        auto_rotate: true,
        layout_ready: false,
    });
    app.insert_resource(ShowAllModels(show_all));

    app.add_systems(
        Startup,
        (
            setup_atmosphere,
            setup_lighting,
            setup_ground,
            spawn_models,
            setup_camera,
            setup_overlay,
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (
            camera_controls,
            focus_controls,
            rotate_focused_model,
            update_model_aabbs,
            pack_model_grid,
            ray_aabb_focus_controls,
            update_preview_base_highlights,
            update_overlay,
            maybe_take_screenshot,
            exit_model_preview,
        )
            .chain(),
    );

    info!("[model-preview] interactive showroom starting");
    app.run();
}

#[derive(Resource)]
struct ShowAllModels(bool);

fn workspace_asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

fn collect_preview_glbs(
    asset_root: &Path,
    show_all: bool,
    single_filter: Option<&str>,
) -> Vec<(String, String)> {
    if let Some(filter) = single_filter {
        let mut all = Vec::new();
        collect_glbs_recursive(asset_root, asset_root, &mut all);
        all.sort_by(|a, b| a.0.cmp(&b.0));
        let normalized = normalize_filter(filter);
        let matches: Vec<(String, String)> = all
            .iter()
            .filter(|(path, _)| preview_path_matches(path, &normalized))
            .cloned()
            .collect();
        if !matches.is_empty() {
            return matches;
        }
        warn!(
            "[model-preview] no GLB matched '{}'; showing featured set",
            filter
        );
    }

    if !show_all {
        return FEATURED_MODELS
            .iter()
            .map(|path| ((*path).to_string(), "featured".to_string()))
            .collect();
    }

    let mut out = Vec::new();
    collect_glbs_recursive(asset_root, asset_root, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn normalize_filter(filter: &str) -> String {
    let filter = filter.trim().trim_matches('"').replace('\\', "/");
    filter
        .strip_suffix(".glb")
        .unwrap_or(&filter)
        .to_ascii_lowercase()
}

fn preview_path_matches(asset_path: &str, normalized_filter: &str) -> bool {
    let normalized_path = asset_path
        .strip_suffix(".glb")
        .unwrap_or(asset_path)
        .to_ascii_lowercase();
    let stem = normalized_path
        .rsplit('/')
        .next()
        .unwrap_or(&normalized_path);
    normalized_path == normalized_filter
        || stem == normalized_filter
        || normalized_path.ends_with(&format!("/{}", normalized_filter))
}

fn collect_glbs_recursive(asset_root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_glbs_recursive(asset_root, &path, out);
            continue;
        }
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("glb"))
            != Some(true)
        {
            continue;
        }
        let rel = path
            .strip_prefix(asset_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let category = path
            .parent()
            .and_then(|parent| parent.strip_prefix(asset_root).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| "assets".to_string());
        out.push((rel, category));
    }
}

fn setup_atmosphere(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.58, 0.66, 0.74)));
}

fn setup_lighting(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 24_000.0,
            shadow_maps_enabled: true,
            color: Color::srgb(1.0, 0.94, 0.82),
            ..default()
        },
        Transform::from_xyz(28.0, 46.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
        PreviewSun,
        VolumetricLight,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            shadow_maps_enabled: false,
            color: Color::srgb(0.62, 0.74, 1.0),
            ..default()
        },
        Transform::from_xyz(-28.0, 34.0, -24.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.92, 0.94, 0.88),
        brightness: 0.78,
        affects_lightmapped_meshes: true,
    });
}

fn setup_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(80.0, 80.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.48, 0.55, 0.42),
            perceptual_roughness: 0.82,
            ..default()
        })),
        Transform::from_translation(Vec3::new(0.0, -0.02, 0.0)),
    ));
}

fn spawn_models(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<ModelPreviewState>,
    show_all: Res<ShowAllModels>,
) {
    let glbs = collect_preview_glbs(
        &state.asset_root,
        show_all.0,
        state.single_filter.as_deref(),
    );
    let base_mesh = meshes.add(Cylinder::new(CELL_SIZE * 0.30, 0.035));
    let base_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.52, 0.54, 0.48),
        perceptual_roughness: 0.78,
        ..default()
    });
    let hovered_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.84, 0.75, 0.43),
        perceptual_roughness: 0.72,
        ..default()
    });
    let focused_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.68, 0.78),
        perceptual_roughness: 0.70,
        ..default()
    });
    commands.insert_resource(PreviewBaseMaterials {
        normal: base_mat.clone(),
        hovered: hovered_mat,
        focused: focused_mat,
    });

    for (idx, (asset_path, category)) in glbs.iter().enumerate() {
        let col = idx % CELLS_PER_ROW;
        let row = idx / CELLS_PER_ROW;
        let pos = preview_grid_position(idx, glbs.len());
        let scene: Handle<WorldAsset> =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(asset_path.clone()));
        commands.spawn((
            WorldAssetRoot(scene.clone()),
            Transform::from_translation(pos),
            PreviewModelRoot { index: idx },
            Name::new(asset_path.clone()),
        ));
        state.scene_handles.push(scene);

        commands.spawn((
            Mesh3d(base_mesh.clone()),
            MeshMaterial3d(base_mat.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 0.02, 0.0)),
            PreviewBaseDisc { index: idx },
        ));

        let display_name = asset_path
            .rsplit('/')
            .next()
            .unwrap_or(asset_path)
            .trim_end_matches(".glb")
            .to_string();
        let file_size_kb = std::fs::metadata(state.asset_root.join(asset_path))
            .map(|m| (m.len() + 1023) / 1024)
            .unwrap_or(0);
        let aabb = preview_model_aabb(pos);
        state.models.push(PreviewEntry {
            path: asset_path.clone(),
            display_name,
            category: category.clone(),
            file_size_kb,
            cell: [col, row],
            world_pos: [pos.x, pos.y, pos.z],
            aabb_min: [aabb.min.x, aabb.min.y, aabb.min.z],
            aabb_max: [aabb.max.x, aabb.max.y, aabb.max.z],
            aabb_ready: false,
        });
    }

    write_preview_manifest(&state);
    info!("[model-preview] spawned {} models", state.models.len());
}

fn preview_grid_position(index: usize, model_count: usize) -> Vec3 {
    let rows = model_count.max(1).div_ceil(CELLS_PER_ROW);
    let row = index / CELLS_PER_ROW;
    let col = index % CELLS_PER_ROW;
    let models_in_row = model_count
        .saturating_sub(row * CELLS_PER_ROW)
        .min(CELLS_PER_ROW)
        .max(1);
    Vec3::new(
        (col as f32 - (models_in_row - 1) as f32 * 0.5) * CELL_SIZE,
        0.0,
        (row as f32 - (rows - 1) as f32 * 0.5) * CELL_SIZE,
    )
}

fn packed_preview_layout(footprints: &[(f32, f32)]) -> PackedPreviewLayout {
    if footprints.is_empty() {
        return PackedPreviewLayout {
            positions: Vec::new(),
            width: 0.0,
            depth: 0.0,
        };
    }

    let rows = footprints.len().div_ceil(CELLS_PER_ROW);
    let row_depths: Vec<f32> = (0..rows)
        .map(|row| {
            footprints[row * CELLS_PER_ROW..((row + 1) * CELLS_PER_ROW).min(footprints.len())]
                .iter()
                .map(|(_, depth)| depth.max(0.1))
                .fold(0.0, f32::max)
        })
        .collect();
    let depth = row_depths.iter().sum::<f32>() + MODEL_GAP * rows.saturating_sub(1) as f32;
    let mut positions = vec![Vec3::ZERO; footprints.len()];
    let mut z_cursor = -depth * 0.5;
    let mut width: f32 = 0.0;

    for (row, row_depth) in row_depths.into_iter().enumerate() {
        let start = row * CELLS_PER_ROW;
        let end = ((row + 1) * CELLS_PER_ROW).min(footprints.len());
        let row_width = footprints[start..end]
            .iter()
            .map(|(model_width, _)| model_width.max(0.1))
            .sum::<f32>()
            + MODEL_GAP * (end - start).saturating_sub(1) as f32;
        width = width.max(row_width);
        let mut x_cursor = -row_width * 0.5;
        let z = z_cursor + row_depth * 0.5;

        for (offset, (model_width, _)) in footprints[start..end].iter().enumerate() {
            let model_width = model_width.max(0.1);
            positions[start + offset] = Vec3::new(x_cursor + model_width * 0.5, 0.0, z);
            x_cursor += model_width + MODEL_GAP;
        }
        z_cursor += row_depth + MODEL_GAP;
    }

    PackedPreviewLayout {
        positions,
        width,
        depth,
    }
}

fn preview_model_aabb(pos: Vec3) -> RayAabb {
    RayAabb::new(
        pos + Vec3::new(-PREVIEW_AABB_RADIUS, 0.0, -PREVIEW_AABB_RADIUS),
        pos + Vec3::new(
            PREVIEW_AABB_RADIUS,
            PREVIEW_AABB_HEIGHT,
            PREVIEW_AABB_RADIUS,
        ),
    )
}

fn preview_entry_aabb(entry: &PreviewEntry) -> RayAabb {
    RayAabb::new(
        Vec3::new(entry.aabb_min[0], entry.aabb_min[1], entry.aabb_min[2]),
        Vec3::new(entry.aabb_max[0], entry.aabb_max[1], entry.aabb_max[2]),
    )
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

fn update_model_aabbs(
    roots: Query<(Entity, &PreviewModelRoot)>,
    children: Query<&Children>,
    mesh_bounds: Query<(&BevyAabb, &GlobalTransform)>,
    mut state: ResMut<ModelPreviewState>,
) {
    for (root_entity, root) in &roots {
        let mut stack = vec![root_entity];
        let mut model_aabb: Option<RayAabb> = None;

        while let Some(entity) = stack.pop() {
            if let Ok((aabb, transform)) = mesh_bounds.get(entity) {
                let world_aabb = transformed_mesh_aabb(aabb, transform);
                model_aabb = Some(match model_aabb {
                    Some(current) => RayAabb::new(
                        current.min.min(world_aabb.min),
                        current.max.max(world_aabb.max),
                    ),
                    None => world_aabb,
                });
            }
            if let Ok(entity_children) = children.get(entity) {
                stack.extend(entity_children.iter());
            }
        }

        let Some(aabb) = model_aabb else {
            continue;
        };
        let Some(entry) = state.models.get_mut(root.index) else {
            continue;
        };
        entry.aabb_min = [aabb.min.x, aabb.min.y, aabb.min.z];
        entry.aabb_max = [aabb.max.x, aabb.max.y, aabb.max.z];
        entry.aabb_ready = true;
    }
}

fn pack_model_grid(
    mut roots: Query<(&PreviewModelRoot, &mut Transform), Without<PreviewBaseDisc>>,
    mut bases: Query<(&PreviewBaseDisc, &mut Transform), Without<PreviewModelRoot>>,
    mut state: ResMut<ModelPreviewState>,
    mut camera: ResMut<PreviewCamera>,
) {
    if state.layout_ready
        || state.models.is_empty()
        || state.models.iter().any(|entry| !entry.aabb_ready)
    {
        return;
    }

    let footprints: Vec<(f32, f32)> = state
        .models
        .iter()
        .map(|entry| {
            (
                entry.aabb_max[0] - entry.aabb_min[0],
                entry.aabb_max[2] - entry.aabb_min[2],
            )
        })
        .collect();
    let layout = packed_preview_layout(&footprints);
    let max_height = state
        .models
        .iter()
        .map(|entry| entry.aabb_max[1] - entry.aabb_min[1])
        .fold(0.0, f32::max);

    for (root, mut transform) in &mut roots {
        let Some(entry) = state.models.get_mut(root.index) else {
            continue;
        };
        let target_center = layout.positions[root.index];
        let current_center = Vec3::new(
            (entry.aabb_min[0] + entry.aabb_max[0]) * 0.5,
            (entry.aabb_min[1] + entry.aabb_max[1]) * 0.5,
            (entry.aabb_min[2] + entry.aabb_max[2]) * 0.5,
        );
        let current_root = Vec3::from_array(entry.world_pos);
        let center_offset = current_center - current_root;
        transform.translation.x = target_center.x - center_offset.x;
        transform.translation.z = target_center.z - center_offset.z;
        entry.world_pos = transform.translation.to_array();
        entry.aabb_ready = false;
    }
    for (base, mut transform) in &mut bases {
        let target = layout.positions[base.index];
        transform.translation = target + Vec3::new(0.0, 0.02, 0.0);
    }

    camera.center = Vec3::new(0.0, max_height * 0.35, 0.0);
    if state.models.len() == 1 {
        let span = layout.width.max(layout.depth).max(max_height);
        camera.distance = (span * 1.7).max(4.8);
        camera.height = (max_height * 0.8).max(3.0);
    } else {
        let span = layout.width.max(layout.depth);
        camera.distance = (span * 1.30).max(12.0);
        camera.height = (max_height + layout.depth * 0.35).max(8.0);
    }
    state.layout_ready = true;
}

fn write_preview_manifest(state: &ModelPreviewState) {
    if let Ok(json) = serde_json::to_string_pretty(&state.models) {
        let _ = std::fs::write(&state.manifest_path, json);
    }
}

fn setup_camera(mut commands: Commands, state: Res<ModelPreviewState>) {
    let rows = ((state.models.len().max(1) + CELLS_PER_ROW - 1) / CELLS_PER_ROW) as f32;
    let single = state.models.len() == 1;
    let center = Vec3::new(0.0, 1.2, 0.0);
    let distance = if single { 4.8 } else { (rows * 4.5).max(9.0) };
    let height = if single {
        3.7
    } else {
        (rows * 2.4).clamp(5.5, 30.0)
    };
    let preview_camera = PreviewCamera {
        yaw: -0.55,
        distance,
        height,
        center,
    };
    let pos = camera_pos(&preview_camera);
    commands.insert_resource(preview_camera);
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(pos).looking_at(center, Vec3::Y),
        AtmosphereSettings::default(),
        Exposure { ev100: 12.75 },
        Tonemapping::AcesFitted,
        Bloom::NATURAL,
        Msaa::Off,
        TemporalAntiAliasing::default(),
        ScreenSpaceReflections {
            min_perceptual_roughness: 0.0..0.0,
            ..default()
        },
        ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            ..default()
        },
        PreviewCameraMarker,
    ));
}

fn setup_overlay(mut commands: Commands, state: Res<ModelPreviewState>) {
    if state.auto_shot {
        return;
    }
    let focused = state
        .models
        .get(state.focused_index)
        .map(|m| m.path.as_str())
        .unwrap_or("none");
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
            padding: UiRect::all(px(10)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.62)),
        children![(
            Text::new(preview_overlay_text(&state, focused)),
            TextFont {
                font: FontSource::UiMonospace,
                font_size: FontSize::Rem(0.94),
                weight: FontWeight::SEMIBOLD,
                width: FontWidth::SEMI_CONDENSED,
                ..default()
            },
            LetterSpacing::Px(0.3),
            TextColor(Color::WHITE),
            PreviewOverlayText,
        )],
    ));
}

fn preview_overlay_text(state: &ModelPreviewState, fallback: &str) -> String {
    match state.models.get(state.focused_index) {
        Some(model) => format!(
            "MODEL PREVIEW\nA/D orbit  W/S zoom  Q/E height  R reset\nMouse hover/click Ray-AABB  [/] focus model  Space autorotate  J/L rotate model\nFocused: {}\nHovered: {}\nPath: {}\nCategory: {}  Size: {} KB  Index: {}/{}",
            model.display_name,
            hovered_model_name(state),
            model.path,
            model.category,
            model.file_size_kb,
            state.focused_index + 1,
            state.models.len()
        ),
        None => format!(
            "MODEL PREVIEW\nA/D orbit  W/S zoom  Q/E height  R reset\nMouse hover/click Ray-AABB  [/] focus model  Space autorotate  J/L rotate model\nFocused: {fallback}"
        ),
    }
}

fn hovered_model_name(state: &ModelPreviewState) -> &str {
    state
        .hovered_index
        .and_then(|index| state.models.get(index))
        .map(|model| model.display_name.as_str())
        .unwrap_or("none")
}

fn update_overlay(
    state: Res<ModelPreviewState>,
    mut q: Query<&mut Text, With<PreviewOverlayText>>,
) {
    if !state.is_changed() {
        return;
    }
    if let Ok(mut text) = q.single_mut() {
        text.0 = preview_overlay_text(&state, "none");
    }
}

fn focus_preview_index(index: usize, state: &mut ModelPreviewState, camera: &mut PreviewCamera) {
    let Some(model) = state.models.get(index) else {
        return;
    };
    state.focused_index = index;
    camera.center = Vec3::new(
        model.world_pos[0],
        model.world_pos[1] + 1.2,
        model.world_pos[2],
    );
    camera.distance = 4.8;
    camera.height = 3.7;
}

fn ray_aabb_focus_controls(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<PreviewCameraMarker>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<ModelPreviewState>,
    mut preview_camera: ResMut<PreviewCamera>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        if state.hovered_index.is_some() {
            state.hovered_index = None;
        }
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };
    let hovered = nearest_ray_aabb_hit(
        ray.origin,
        *ray.direction,
        state
            .models
            .iter()
            .enumerate()
            .map(|(index, entry)| (index, preview_entry_aabb(entry))),
    )
    .map(|(index, _)| index);

    if state.hovered_index != hovered {
        state.hovered_index = hovered;
    }
    if mouse.just_pressed(MouseButton::Left)
        && let Some(index) = hovered
    {
        focus_preview_index(index, &mut state, &mut preview_camera);
    }
}

fn update_preview_base_highlights(
    state: Res<ModelPreviewState>,
    materials: Option<Res<PreviewBaseMaterials>>,
    mut bases: Query<(&PreviewBaseDisc, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    if !state.is_changed() {
        return;
    }
    let Some(materials) = materials else {
        return;
    };
    for (base, mut material) in &mut bases {
        material.0 = if base.index == state.focused_index {
            materials.focused.clone()
        } else if Some(base.index) == state.hovered_index {
            materials.hovered.clone()
        } else {
            materials.normal.clone()
        };
    }
}

fn camera_pos(camera: &PreviewCamera) -> Vec3 {
    camera.center
        + Vec3::new(
            camera.yaw.sin() * camera.distance,
            camera.height,
            camera.yaw.cos() * camera.distance,
        )
}

fn camera_controls(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut camera: ResMut<PreviewCamera>,
    mut q: Query<&mut Transform, With<PreviewCameraMarker>>,
) {
    let dt = time.delta_secs();
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        camera.yaw -= 1.2 * dt;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        camera.yaw += 1.2 * dt;
    }
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        camera.distance = (camera.distance - 8.0 * dt).max(4.0);
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        camera.distance = (camera.distance + 8.0 * dt).min(60.0);
    }
    if keys.pressed(KeyCode::KeyQ) {
        camera.height = (camera.height - 4.0 * dt).max(1.2);
    }
    if keys.pressed(KeyCode::KeyE) {
        camera.height = (camera.height + 4.0 * dt).min(24.0);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        camera.yaw = -0.30;
        camera.distance = 7.0;
        camera.height = 3.7;
    }

    if let Ok(mut transform) = q.single_mut() {
        transform.translation = camera_pos(&camera);
        transform.look_at(camera.center, Vec3::Y);
    }
}

fn focus_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ModelPreviewState>,
    mut camera: ResMut<PreviewCamera>,
) {
    let len = state.models.len();
    if len == 0 {
        return;
    }
    let mut changed = false;
    if keys.just_pressed(KeyCode::BracketLeft) {
        state.focused_index = if state.focused_index == 0 {
            len - 1
        } else {
            state.focused_index - 1
        };
        changed = true;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        state.focused_index = (state.focused_index + 1) % len;
        changed = true;
    }
    if keys.just_pressed(KeyCode::Space) {
        state.auto_rotate = !state.auto_rotate;
    }
    if changed {
        focus_preview_index(state.focused_index, &mut state, &mut camera);
    }
}

fn rotate_focused_model(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<ModelPreviewState>,
    mut q: Query<(&PreviewModelRoot, &mut Transform)>,
) {
    let mut delta = 0.0;
    if state.auto_rotate && state.models.len() == 1 {
        delta += 0.7 * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyJ) {
        delta += 1.8 * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyL) {
        delta -= 1.8 * time.delta_secs();
    }
    if delta.abs() <= f32::EPSILON {
        return;
    }
    for (root, mut transform) in &mut q {
        if root.index == state.focused_index {
            transform.rotate_y(delta);
        }
    }
}

fn maybe_take_screenshot(
    mut state: ResMut<ModelPreviewState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !state.auto_shot || state.shot_requested {
        return;
    }
    state.frame = state.frame.saturating_add(1);
    let pending = state
        .scene_handles
        .iter()
        .filter(|handle| {
            !matches!(
                asset_server.load_state(*handle),
                bevy::asset::LoadState::Loaded
            )
        })
        .count();
    let unresolved_bounds = state
        .models
        .iter()
        .filter(|entry| !entry.aabb_ready)
        .count();
    if state.frame < STABILIZATION_FRAMES
        || pending > 0
        || unresolved_bounds > 0
        || !state.layout_ready
    {
        return;
    }
    write_preview_manifest(&state);
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(state.png_path.clone()));
    state.shot_requested = true;
    state.exit_deadline = Some(Instant::now() + Duration::from_secs(20));
    info!(
        "[model-preview] screenshot requested: {}",
        state.png_path.display()
    );
}

fn exit_model_preview(keys: Res<ButtonInput<KeyCode>>, state: Res<ModelPreviewState>) {
    if keys.just_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }
    if state.auto_shot && state.shot_requested {
        if let Ok(meta) = std::fs::metadata(&state.png_path) {
            if meta.len() > 0 {
                std::process::exit(0);
            }
        }
        if state
            .exit_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            warn!("[model-preview] screenshot did not flush before timeout");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{GlobalTransform, Transform, Vec3};

    use super::{
        BevyAabb, FEATURED_MODELS, MODEL_GAP, collect_preview_glbs, packed_preview_layout,
        preview_grid_position, transformed_mesh_aabb, workspace_asset_root,
    };

    #[test]
    fn preview_grid_positions_are_centered() {
        let positions: Vec<_> = (0..16)
            .map(|index| preview_grid_position(index, 16))
            .collect();
        let min_x = positions
            .iter()
            .map(|pos| pos.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = positions
            .iter()
            .map(|pos| pos.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_z = positions
            .iter()
            .map(|pos| pos.z)
            .fold(f32::INFINITY, f32::min);
        let max_z = positions
            .iter()
            .map(|pos| pos.z)
            .fold(f32::NEG_INFINITY, f32::max);

        assert!((min_x + max_x).abs() < f32::EPSILON);
        assert!((min_z + max_z).abs() < f32::EPSILON);
        assert_eq!(preview_grid_position(0, 1), Vec3::ZERO);
    }

    #[test]
    fn transformed_mesh_bounds_preserve_world_scale() {
        let local = BevyAabb::from_min_max(Vec3::new(-1.0, 0.0, -0.5), Vec3::new(1.0, 2.0, 0.5));
        let transform = GlobalTransform::from(
            Transform::from_xyz(4.0, 1.0, -2.0).with_scale(Vec3::new(2.0, 3.0, 4.0)),
        );

        let world = transformed_mesh_aabb(&local, &transform);

        assert_eq!(world.min, Vec3::new(2.0, 1.0, -4.0));
        assert_eq!(world.max, Vec3::new(6.0, 7.0, 0.0));
    }

    #[test]
    fn packed_layout_separates_models_by_their_real_footprints() {
        let footprints = [(1.0, 2.0), (6.0, 3.0), (2.0, 5.0), (4.0, 1.0), (8.0, 2.0)];
        let layout = packed_preview_layout(&footprints);

        for index in 1..4 {
            let previous_right = layout.positions[index - 1].x + footprints[index - 1].0 * 0.5;
            let current_left = layout.positions[index].x - footprints[index].0 * 0.5;
            assert!((current_left - previous_right - MODEL_GAP).abs() < 0.0001);
        }
        assert!(layout.positions[4].z > layout.positions[0].z);
        assert!(layout.width >= 17.0);
        assert!(layout.depth >= 5.0 + MODEL_GAP + 2.0);
    }

    #[test]
    fn model_preview_all_scans_assets_recursively() {
        let root = workspace_asset_root();
        let all = collect_preview_glbs(&root, true, None);
        assert!(
            all.iter()
                .any(|(path, _)| path == "procedural/pretty/sokpop_tree.glb")
        );
        assert!(
            all.iter()
                .any(|(path, _)| path == "procedural/pretty/fallen_stick.glb")
        );
        assert!(all.len() >= FEATURED_MODELS.len());
    }

    #[test]
    fn featured_model_preview_is_a_representative_showroom() {
        let root = workspace_asset_root();
        let featured = collect_preview_glbs(&root, false, None);

        assert!(
            featured.len() >= 12,
            "featured showroom only has {} models",
            featured.len()
        );
        for (path, category) in featured {
            assert_eq!(category, "featured");
            assert!(
                root.join(&path).is_file(),
                "featured model does not exist: {path}"
            );
        }
        for weapon in [
            "hoplite_reaper_scythe",
            "hoplite_dragon_katana",
            "hoplite_golem_hammer",
            "hoplite_midas_sword",
        ] {
            assert!(
                FEATURED_MODELS
                    .iter()
                    .any(|path| path.ends_with(&format!("/{weapon}.glb"))),
                "missing Hoplite weapon from featured showroom: {weapon}"
            );
        }
    }

    #[test]
    fn model_preview_one_selects_by_stem_or_path() {
        let root = workspace_asset_root();
        let by_stem = collect_preview_glbs(&root, false, Some("sokpop_tree"));
        assert_eq!(by_stem.len(), 1);
        assert_eq!(by_stem[0].0, "procedural/pretty/sokpop_tree.glb");

        let by_path =
            collect_preview_glbs(&root, false, Some("procedural/pretty/fallen_stick.glb"));
        assert_eq!(by_path.len(), 1);
        assert_eq!(by_path[0].0, "procedural/pretty/fallen_stick.glb");
    }
}
