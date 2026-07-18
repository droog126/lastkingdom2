//! Interactive `--model-preview` mode.
//!
//! This is a small in-game showroom for judging GLB assets and selected
//! procedural runtime assets without gameplay or camera automation getting in
//! the way.
//!
//! Run:
//!   cargo run -p lk2-client -- --model-preview
//!
//! Useful options:
//!   --model-preview-all   recursively show every GLB under assets/
//!   --model-preview-one=<name-or-path>  show one GLB or procedural asset
//!   --model-preview-select=<stem[,stem...]>  preselect models for AI review
//!   --model-preview-shot  save screenshot and exit
//!     - for `--model-preview-one=<name>` the PNG is written to
//!       `screenshots/model_preview/<name>.png`
//!   --model-preview-view=front|side|top  select a deterministic orthographic-style view
//!   --model-preview-no-base  hide the showroom display base for AI reference captures
//!     - otherwise it goes to `screenshots/model_preview/model_preview.png`

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bevy::camera::Exposure;
use bevy::camera::primitives::Aabb as BevyAabb;
use bevy::clipboard::Clipboard;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::MouseMotion;
use bevy::pbr::{
    AtmosphereSettings, MaterialPlugin, ScreenSpaceAmbientOcclusion,
    ScreenSpaceAmbientOcclusionQualityLevel, ScreenSpaceReflections,
};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::render::{
    RenderPlugin,
    settings::{Backends, WgpuSettings},
};
use bevy::text::LetterSpacing;
use bevy::window::{PresentMode, PrimaryWindow, WindowResolution};
use bevy_world_serialization::WorldAsset;
use lk2_core::content::game_content_registry;

use crate::crisp_image_plugin;
use crate::game_scene::animate_grass_gpu_wind;
use crate::game_scene::{
    GrassWind, GrassWindExtension, GrassWindMaterial, MAIN_APP_MODEL_PATHS, WindField,
    build_grass_tuft_mesh, grass_wind_rotation,
};
use crate::ray_aabb::{RayAabb, nearest_ray_aabb_hit};

/// A generated animal that is not in the shared ecology registry yet, but is
/// intentionally available to the model showroom.
const MODEL_PREVIEW_EXTRA_MODELS: &[&str] = &["animals/fish.glb"];

pub const MODEL_PREVIEW_OUTPUT_DIR: &str = "screenshots/model_preview";

const CELL_SIZE: f32 = 3.6;
const CELLS_PER_ROW: usize = 4;
const STABILIZATION_FRAMES: u64 = 150;
const BOUNDS_FALLBACK_FRAMES: u64 = 120;
const PREVIEW_AABB_RADIUS: f32 = 1.35;
const PREVIEW_AABB_HEIGHT: f32 = 2.7;
const MODEL_GAP: f32 = 1.4;
const PROCEDURAL_GRASS_PREVIEW_PATH: &str = "procedural/grass_tuft";

#[derive(Component)]
struct PreviewProceduralGrass;

#[derive(Resource)]
pub struct ModelPreviewState {
    pub frame: u64,
    pub auto_shot: bool,
    pub shot_requested: bool,
    pub exit_deadline: Option<Instant>,
    pub png_path: PathBuf,
    pub manifest_path: PathBuf,
    pub selection_path: PathBuf,
    pub asset_root: PathBuf,
    pub scene_handles: Vec<Handle<WorldAsset>>,
    pub models: Vec<PreviewEntry>,
    pub single_filter: Option<String>,
    pub selection_filters: Vec<String>,
    /// Models selected with Shift-click for a combined AI review.
    pub selected_indices: BTreeSet<usize>,
    pub focused_index: usize,
    pub hovered_index: Option<usize>,
    pub copy_feedback: Option<String>,
    pub auto_rotate: bool,
    pub layout_ready: bool,
    pub show_bases: bool,
    view: PreviewView,
}

#[derive(Component)]
struct PreviewSun;

#[derive(Resource)]
struct PreviewCamera {
    mode: PreviewCameraMode,
    view: PreviewView,
    yaw: f32,
    distance: f32,
    height: f32,
    center: Vec3,
    free_position: Vec3,
    free_yaw: f32,
    free_pitch: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewCameraMode {
    Orbit,
    Free,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewView {
    Orbit,
    Front,
    Side,
    Top,
}

impl PreviewView {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "front" | "正面" => Self::Front,
            "side" | "侧面" => Self::Side,
            "top" | "上面" | "top-down" => Self::Top,
            _ => Self::Orbit,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Orbit => "",
            Self::Front => "_front",
            Self::Side => "_side",
            Self::Top => "_top",
        }
    }
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
    pub file_size_bytes: u64,
    pub file_size_kb: u64,
    pub asset_info: Option<PreviewAssetInfo>,
    pub asset_info_error: Option<String>,
    pub cell: [usize; 2],
    pub world_pos: [f32; 3],
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub aabb_ready: bool,
    pub aabb_fallback: bool,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PreviewAssetInfo {
    pub meshes: usize,
    pub primitives: usize,
    pub nodes: usize,
    pub materials: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub dimensions: [f32; 3],
}

#[derive(serde::Deserialize)]
struct GlbDocument {
    #[serde(default)]
    meshes: Vec<GlbMesh>,
    #[serde(default)]
    nodes: Vec<serde_json::Value>,
    #[serde(default)]
    materials: Vec<serde_json::Value>,
    #[serde(default)]
    accessors: Vec<GlbAccessor>,
}

#[derive(serde::Deserialize)]
struct GlbMesh {
    #[serde(default)]
    primitives: Vec<GlbPrimitive>,
}

#[derive(serde::Deserialize)]
struct GlbPrimitive {
    #[serde(default)]
    attributes: HashMap<String, usize>,
    indices: Option<usize>,
    mode: Option<u32>,
}

#[derive(serde::Deserialize)]
struct GlbAccessor {
    count: usize,
    min: Option<Vec<f32>>,
    max: Option<Vec<f32>>,
}

#[derive(serde::Serialize)]
struct PreviewManifest<'a> {
    schema_version: u32,
    generated_by: &'static str,
    model_count: usize,
    models: &'a [PreviewEntry],
}

#[derive(serde::Serialize)]
struct PreviewSelection {
    schema_version: u32,
    generated_by: &'static str,
    requested_filters: Vec<String>,
    selected_count: usize,
    selected_indices: Vec<usize>,
    selected_models: Vec<SelectedModel>,
}

#[derive(serde::Serialize)]
struct SelectedModel {
    index: usize,
    path: String,
    display_name: String,
    category: String,
    file_size_bytes: u64,
    file_size_kb: u64,
    asset_info: Option<PreviewAssetInfo>,
    asset_info_error: Option<String>,
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
    selected: Handle<StandardMaterial>,
}

#[derive(Component)]
struct PreviewCameraMarker;

#[derive(Component)]
struct PreviewOverlayText;

#[derive(Component)]
struct CopyOptimizationButton;

pub fn run_model_preview() {
    let args: Vec<String> = std::env::args().collect();
    let show_all = args.iter().any(|a| a == "--model-preview-all");
    let auto_shot = args.iter().any(|a| a == "--model-preview-shot");
    let show_bases = !args.iter().any(|a| a == "--model-preview-no-base");
    let view = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--model-preview-view="))
        .map(PreviewView::parse)
        .unwrap_or(PreviewView::Orbit);
    let single_filter = args
        .iter()
        .find_map(|a| {
            a.strip_prefix("--model-preview-one=")
                .or_else(|| a.strip_prefix("--model-preview-model="))
                .or_else(|| a.strip_prefix("--model-preview-path="))
        })
        .map(str::to_string);
    let selection_filters = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--model-preview-select="))
        .map(parse_selection_filters)
        .unwrap_or_default();

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
            output_dir.join(format!("{stem}{}.png", view.suffix()))
        }
        None => output_dir.join(format!("model_preview{}.png", view.suffix())),
    };
    if auto_shot {
        let _ = std::fs::remove_file(&png_path);
    }

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(crisp_image_plugin())
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
    app.add_plugins(MaterialPlugin::<GrassWindMaterial>::default());

    app.insert_resource(ModelPreviewState {
        frame: 0,
        auto_shot,
        shot_requested: false,
        exit_deadline: None,
        png_path,
        manifest_path: output_dir.join("manifest.json"),
        selection_path: output_dir.join("selection.json"),
        asset_root,
        scene_handles: Vec::new(),
        models: Vec::new(),
        single_filter,
        selection_filters,
        selected_indices: BTreeSet::new(),
        focused_index: 0,
        hovered_index: None,
        copy_feedback: None,
        // Keep automated evidence deterministic. Interactive previews still
        // enable rotation after startup through the Space toggle.
        auto_rotate: !auto_shot,
        layout_ready: false,
        show_bases,
        view,
    });
    app.insert_resource(ShowAllModels(show_all));
    app.insert_resource(WindField::default());

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
            advance_preview_frame,
            animate_grass_gpu_wind,
            animate_preview_grass_wind,
            camera_controls,
            focus_controls,
            rotate_focused_model,
            update_model_aabbs,
            fallback_model_aabbs,
            pack_model_grid,
            ray_aabb_focus_controls,
            handle_preview_buttons,
            keyboard_selection_controls,
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

fn advance_preview_frame(mut state: ResMut<ModelPreviewState>) {
    state.frame = state.frame.saturating_add(1);
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
        let normalized = normalize_filter(filter);
        if normalized == "grass" || normalized == "grass_tuft" {
            return vec![(
                PROCEDURAL_GRASS_PREVIEW_PATH.to_string(),
                "procedural".to_string(),
            )];
        }
        let mut all = Vec::new();
        collect_glbs_recursive(asset_root, asset_root, &mut all);
        all.sort_by(|a, b| a.0.cmp(&b.0));
        let matches: Vec<(String, String)> = all
            .iter()
            .filter(|(path, _)| preview_path_matches(path, &normalized))
            .cloned()
            .collect();
        if !matches.is_empty() {
            return matches;
        }
        let procedural_matches: Vec<(String, String)> = collect_main_app_glbs(asset_root)
            .into_iter()
            .filter(|(path, _)| preview_path_matches(path, &normalized))
            .collect();
        if !procedural_matches.is_empty() {
            return procedural_matches;
        }
        warn!(
            "[model-preview] no asset matched '{}'; showing main-app set",
            filter
        );
    }

    if !show_all {
        return collect_main_app_glbs(asset_root);
    }

    let mut out = Vec::new();
    collect_glbs_recursive(asset_root, asset_root, &mut out);
    out.push((
        PROCEDURAL_GRASS_PREVIEW_PATH.to_string(),
        "procedural".to_string(),
    ));
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn collect_main_app_glbs(asset_root: &Path) -> Vec<(String, String)> {
    let mut paths = BTreeSet::new();
    for definition in game_content_registry().definitions() {
        if let Some(visual) = &definition.visual {
            paths.insert(visual.model_path.clone());
        }
    }
    for path in MAIN_APP_MODEL_PATHS
        .iter()
        .chain(MODEL_PREVIEW_EXTRA_MODELS.iter())
    {
        paths.insert((*path).to_string());
    }
    paths.insert(PROCEDURAL_GRASS_PREVIEW_PATH.to_string());

    paths
        .into_iter()
        .filter_map(|path| {
            let procedural = path == PROCEDURAL_GRASS_PREVIEW_PATH;
            if !procedural && !asset_root.join(&path).is_file() {
                return None;
            }
            Some((
                path,
                if procedural {
                    "procedural".to_string()
                } else {
                    "main-app".to_string()
                },
            ))
        })
        .collect()
}

fn inspect_preview_asset(path: &Path) -> (Option<PreviewAssetInfo>, Option<String>) {
    match inspect_preview_asset_file(path) {
        Ok(info) => (Some(info), None),
        Err(error) => (None, Some(error)),
    }
}

fn inspect_preview_asset_file(path: &Path) -> Result<PreviewAssetInfo, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return Err("not a binary glTF file".to_string());
    }
    let json_length = u32::from_le_bytes(
        bytes[12..16]
            .try_into()
            .map_err(|_| "invalid GLB JSON length")?,
    ) as usize;
    if &bytes[16..20] != b"JSON" || 20 + json_length > bytes.len() {
        return Err("GLB JSON chunk is missing or truncated".to_string());
    }
    let mut json_bytes = &bytes[20..20 + json_length];
    while json_bytes
        .last()
        .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == 0)
    {
        json_bytes = &json_bytes[..json_bytes.len() - 1];
    }
    let document: GlbDocument =
        serde_json::from_slice(json_bytes).map_err(|error| error.to_string())?;
    let mut primitives = 0;
    let mut vertices = 0;
    let mut triangles = 0;
    let mut bounds_min = [f32::INFINITY; 3];
    let mut bounds_max = [f32::NEG_INFINITY; 3];

    for mesh in &document.meshes {
        for primitive in &mesh.primitives {
            primitives += 1;
            let positions = primitive
                .attributes
                .get("POSITION")
                .and_then(|index| document.accessors.get(*index));
            let vertex_count = positions.map_or(0, |accessor| accessor.count);
            vertices += vertex_count;
            let index_count = primitive
                .indices
                .and_then(|index| document.accessors.get(index))
                .map_or(vertex_count, |accessor| accessor.count);
            triangles += match primitive.mode.unwrap_or(4) {
                4 => index_count / 3,
                5 | 6 => index_count.saturating_sub(2),
                _ => 0,
            };

            let Some(positions) = positions else {
                continue;
            };
            let Some(min) = bounds3(positions.min.as_deref()) else {
                continue;
            };
            let Some(max) = bounds3(positions.max.as_deref()) else {
                continue;
            };
            for axis in 0..3 {
                bounds_min[axis] = bounds_min[axis].min(min[axis]);
                bounds_max[axis] = bounds_max[axis].max(max[axis]);
            }
        }
    }

    if !bounds_min.iter().all(|value| value.is_finite())
        || !bounds_max.iter().all(|value| value.is_finite())
    {
        bounds_min = [0.0; 3];
        bounds_max = [0.0; 3];
    }
    let dimensions = [
        bounds_max[0] - bounds_min[0],
        bounds_max[1] - bounds_min[1],
        bounds_max[2] - bounds_min[2],
    ];

    Ok(PreviewAssetInfo {
        meshes: document.meshes.len(),
        primitives,
        nodes: document.nodes.len(),
        materials: document.materials.len(),
        vertices,
        triangles,
        bounds_min,
        bounds_max,
        dimensions,
    })
}

fn bounds3(values: Option<&[f32]>) -> Option<[f32; 3]> {
    let values = values?;
    Some([
        values.first().copied()?,
        values.get(1).copied()?,
        values.get(2).copied()?,
    ])
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
    mut grass_materials: ResMut<Assets<GrassWindMaterial>>,
    mut state: ResMut<ModelPreviewState>,
    show_all: Res<ShowAllModels>,
) {
    let glbs = collect_preview_glbs(
        &state.asset_root,
        show_all.0,
        state.single_filter.as_deref(),
    );
    // Open the showroom on the player silhouette when it is available. The
    // focused model can still be changed with [ and ] or by clicking a base.
    if let Some(player_index) = glbs
        .iter()
        .position(|(path, _)| path == "procedural/pretty/sokpop_gatherer.glb")
    {
        state.focused_index = player_index;
    }
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
    let selected_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.35, 0.78),
        perceptual_roughness: 0.70,
        ..default()
    });
    commands.insert_resource(PreviewBaseMaterials {
        normal: base_mat.clone(),
        hovered: hovered_mat,
        focused: focused_mat,
        selected: selected_mat,
    });

    let grass_mesh = meshes.add(build_grass_tuft_mesh());
    let grass_materials = [
        grass_materials.add(GrassWindMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.20, 0.48, 0.18),
                unlit: true,
                perceptual_roughness: 0.96,
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

    for (idx, (asset_path, category)) in glbs.iter().enumerate() {
        if state
            .selection_filters
            .iter()
            .any(|filter| preview_path_matches(asset_path, filter))
        {
            state.selected_indices.insert(idx);
        }
        let col = idx % CELLS_PER_ROW;
        let row = idx / CELLS_PER_ROW;
        let pos = preview_grid_position(idx, glbs.len());
        let is_procedural_grass = asset_path == PROCEDURAL_GRASS_PREVIEW_PATH;
        let root = if is_procedural_grass {
            commands
                .spawn((
                    Transform::from_translation(pos),
                    PreviewModelRoot { index: idx },
                    Name::new(asset_path.clone()),
                ))
                .id()
        } else {
            let scene: Handle<WorldAsset> =
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(asset_path.clone()));
            let root = commands
                .spawn((
                    WorldAssetRoot(scene.clone()),
                    Transform::from_translation(pos),
                    PreviewModelRoot { index: idx },
                    Name::new(asset_path.clone()),
                ))
                .id();
            state.scene_handles.push(scene);
            root
        };

        if is_procedural_grass {
            commands.spawn((
                Mesh3d(grass_mesh.clone()),
                MeshMaterial3d(grass_materials[idx % grass_materials.len()].clone()),
                Transform::from_scale(Vec3::splat(2.5)),
                GrassWind {
                    base_yaw: 0.0,
                    phase: 0.35,
                    strength: 0.075,
                },
                PreviewProceduralGrass,
                ChildOf(root),
                Name::new("procedural_grass_tuft_mesh"),
            ));
        }

        if state.show_bases {
            commands.spawn((
                Mesh3d(base_mesh.clone()),
                MeshMaterial3d(base_mat.clone()),
                Transform::from_translation(pos + Vec3::new(0.0, 0.02, 0.0)),
                PreviewBaseDisc { index: idx },
            ));
        }

        let display_name = asset_path
            .rsplit('/')
            .next()
            .unwrap_or(asset_path)
            .trim_end_matches(".glb")
            .to_string();
        let (file_size_bytes, file_size_kb, asset_info, asset_info_error) = if is_procedural_grass {
            (0, 0, Some(procedural_grass_preview_info()), None)
        } else {
            let file_size_kb = std::fs::metadata(state.asset_root.join(asset_path))
                .map(|m| (m.len() + 1023) / 1024)
                .unwrap_or(0);
            let file_size_bytes = std::fs::metadata(state.asset_root.join(asset_path))
                .map(|m| m.len())
                .unwrap_or(0);
            let (asset_info, asset_info_error) =
                inspect_preview_asset(&state.asset_root.join(asset_path));
            (file_size_bytes, file_size_kb, asset_info, asset_info_error)
        };
        let aabb = preview_model_aabb(pos);
        state.models.push(PreviewEntry {
            path: asset_path.clone(),
            display_name,
            category: category.clone(),
            file_size_bytes,
            file_size_kb,
            asset_info,
            asset_info_error,
            cell: [col, row],
            world_pos: [pos.x, pos.y, pos.z],
            aabb_min: [aabb.min.x, aabb.min.y, aabb.min.z],
            aabb_max: [aabb.max.x, aabb.max.y, aabb.max.z],
            aabb_ready: false,
            aabb_fallback: false,
        });
    }

    write_preview_manifest(&state);
    write_selection_manifest(&state);
    info!("[model-preview] spawned {} models", state.models.len());
}

fn procedural_grass_preview_info() -> PreviewAssetInfo {
    PreviewAssetInfo {
        meshes: 1,
        primitives: 1,
        nodes: 1,
        materials: 1,
        vertices: 25,
        triangles: 15,
        bounds_min: [-0.22, 0.0, -0.22],
        bounds_max: [0.22, 0.72, 0.22],
        dimensions: [0.44, 0.72, 0.44],
    }
}

fn animate_preview_grass_wind(
    time: Res<Time>,
    wind: Res<WindField>,
    mut grass: Query<(&GrassWind, &mut Transform), With<PreviewProceduralGrass>>,
) {
    for (sway, mut transform) in &mut grass {
        transform.rotation = grass_wind_rotation(time.elapsed_secs(), *wind, *sway);
    }
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
        entry.aabb_fallback = false;
    }
}

fn fallback_model_aabbs(mut state: ResMut<ModelPreviewState>) {
    if state.frame < BOUNDS_FALLBACK_FRAMES {
        return;
    }

    let mut fallback_count = 0;
    for entry in &mut state.models {
        if entry.aabb_ready {
            continue;
        }
        let fallback = preview_model_aabb(Vec3::from_array(entry.world_pos));
        entry.aabb_min = [fallback.min.x, fallback.min.y, fallback.min.z];
        entry.aabb_max = [fallback.max.x, fallback.max.y, fallback.max.z];
        entry.aabb_ready = true;
        entry.aabb_fallback = true;
        fallback_count += 1;
    }
    if fallback_count > 0 {
        warn!(
            "[model-preview] using fallback bounds for {} model(s) after {} frames",
            fallback_count, state.frame
        );
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

    if !state.auto_shot && state.models.len() > 1 {
        if let Some(focused) = state.models.get(state.focused_index) {
            camera.center = Vec3::new(
                focused.world_pos[0],
                focused.world_pos[1] + 1.2,
                focused.world_pos[2],
            );
            camera.distance = 4.8;
            camera.height = 3.7;
        }
    } else {
        camera.center = Vec3::new(0.0, max_height * 0.35, 0.0);
    }
    if state.models.len() == 1 {
        let span = layout.width.max(layout.depth).max(max_height);
        let minimum_distance = if state.show_bases { 2.2 } else { 1.35 };
        camera.distance = (span * 1.7).max(minimum_distance);
        camera.height = match camera.view {
            PreviewView::Front | PreviewView::Side => 0.0,
            // Leave enough margin for the presentation disc in the top-down
            // frame; the perspective camera otherwise crops the disc edges.
            PreviewView::Top if state.show_bases => (span * 2.7).max(4.2),
            PreviewView::Top => (span * 1.5).max(1.8),
            PreviewView::Orbit => (max_height * 0.8).max(1.2),
        };
    } else if state.auto_shot {
        let span = layout.width.max(layout.depth);
        camera.distance = (span * 1.30).max(12.0);
        camera.height = (max_height + layout.depth * 0.35).max(8.0);
    }
    state.layout_ready = true;
}

fn write_preview_manifest(state: &ModelPreviewState) {
    let manifest = PreviewManifest {
        schema_version: 2,
        generated_by: "lk2-client --model-preview",
        model_count: state.models.len(),
        models: &state.models,
    };
    if let Ok(json) = serde_json::to_string_pretty(&manifest) {
        let _ = std::fs::write(&state.manifest_path, json);
    }
}

fn parse_selection_filters(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(normalize_filter)
        .filter(|filter| !filter.is_empty())
        .collect()
}

fn write_selection_manifest(state: &ModelPreviewState) {
    let selected_models = state
        .selected_indices
        .iter()
        .filter_map(|index| {
            state.models.get(*index).map(|model| SelectedModel {
                index: *index,
                path: model.path.clone(),
                display_name: model.display_name.clone(),
                category: model.category.clone(),
                file_size_bytes: model.file_size_bytes,
                file_size_kb: model.file_size_kb,
                asset_info: model.asset_info.clone(),
                asset_info_error: model.asset_info_error.clone(),
            })
        })
        .collect::<Vec<_>>();
    let selection = PreviewSelection {
        schema_version: 1,
        generated_by: "lk2-client --model-preview",
        requested_filters: state.selection_filters.clone(),
        selected_count: selected_models.len(),
        selected_indices: state.selected_indices.iter().copied().collect(),
        selected_models,
    };
    if let Ok(json) = serde_json::to_string_pretty(&selection) {
        let _ = std::fs::write(&state.selection_path, json);
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
        mode: PreviewCameraMode::Orbit,
        view: state.view,
        yaw: -0.55,
        distance,
        height,
        center,
        free_position: Vec3::ZERO,
        free_yaw: 0.0,
        free_pitch: 0.0,
    };
    let pos = camera_pos(&preview_camera);
    let (free_yaw, free_pitch) = free_angles(pos, center);
    let mut preview_camera = preview_camera;
    preview_camera.free_position = pos;
    preview_camera.free_yaw = free_yaw;
    preview_camera.free_pitch = free_pitch;
    let camera_up = camera_up(&preview_camera);
    commands.insert_resource(preview_camera);
    let mut camera_entity = commands.spawn((
        Camera3d::default(),
        Transform::from_translation(pos).looking_at(center, camera_up),
        AtmosphereSettings::default(),
        Exposure { ev100: 12.75 },
        Tonemapping::AcesFitted,
        // SSAO requires MSAA to be disabled; the matte showroom lighting and
        // Bevy's temporal resolve keep the generated silhouettes readable.
        Msaa::Off,
        PreviewCameraMarker,
    ));
    if state.view == PreviewView::Orbit {
        camera_entity.insert((
            ScreenSpaceReflections {
                min_perceptual_roughness: 0.0..0.0,
                ..default()
            },
            ScreenSpaceAmbientOcclusion {
                quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
                ..default()
            },
        ));
    }
}

fn setup_overlay(
    mut commands: Commands,
    state: Res<ModelPreviewState>,
    asset_server: Res<AssetServer>,
) {
    if state.auto_shot {
        return;
    }
    let cjk_font: Handle<Font> = asset_server.load("fonts/NotoSansCJKsc-Regular.otf");
    let focused = state
        .models
        .get(state.focused_index)
        .map(|m| m.path.as_str())
        .unwrap_or("无");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                top: px(12),
                max_width: px(760),
                padding: UiRect::all(px(10)),
                row_gap: px(8),
                flex_direction: FlexDirection::Column,
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.62)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(preview_overlay_text_unicode(&state, focused)),
                TextFont {
                    font: cjk_font.clone().into(),
                    font_size: FontSize::Rem(0.92),
                    weight: FontWeight::SEMIBOLD,
                    width: FontWidth::SEMI_CONDENSED,
                    ..default()
                },
                LetterSpacing::Px(0.3),
                TextColor(Color::WHITE),
                PreviewOverlayText,
            ));
            panel
                .spawn((
                    Button,
                    Node {
                        min_height: px(30),
                        padding: UiRect::axes(px(12), px(5)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(px(5)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.35, 0.39, 0.96)),
                    CopyOptimizationButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("复制 AI 优化提示"),
                        TextFont {
                            font: cjk_font.clone().into(),
                            font_size: FontSize::Px(14.0),
                            weight: FontWeight::SEMIBOLD,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn preview_overlay_text_unicode(state: &ModelPreviewState, fallback: &str) -> String {
    let controls = concat!(
        "Tab: \u{5207}\u{6362}\u{8f68}\u{9053}/\u{81ea}\u{7531}\u{76f8}\u{673a}\n",
        "\u{81ea}\u{7531}\u{76f8}\u{673a}: \u{9f20}\u{6807}\u{53f3}\u{952e}\u{62d6}\u{62fd}\u{89c6}\u{89d2}, WASD\u{79fb}\u{52a8}, Q/E\u{5347}\u{964d}, Shift\u{52a0}\u{901f}\n",
        "\u{8f68}\u{9053}\u{76f8}\u{673a}: A/D\u{65cb}\u{8f6c}, W/S\u{7f29}\u{653e}, Q/E\u{8c03}\u{6574}\u{9ad8}\u{5ea6}, R\u{91cd}\u{7f6e}\n",
        "\u{5de6}\u{952e}: \u{9009}\u{62e9}\u{5e76}\u{805a}\u{7126}, Shift/Ctrl+\u{5de6}\u{952e}: \u{591a}\u{9009}, [ / ]: \u{5207}\u{6362}\u{6a21}\u{578b}, Esc: \u{9000}\u{51fa}",
    );
    match state.models.get(state.focused_index) {
        Some(model) => {
            let geometry = match (&model.asset_info, &model.asset_info_error) {
                (Some(info), _) => format!(
                    "\u{51e0}\u{4f55}: {} triangles / {} vertices / {} meshes / {} nodes / {} materials\n\u{5305}\u{56f4}\u{76d2}: {:.2} x {:.2} x {:.2} m",
                    info.triangles,
                    info.vertices,
                    info.meshes,
                    info.nodes,
                    info.materials,
                    info.dimensions[0],
                    info.dimensions[1],
                    info.dimensions[2],
                ),
                (None, Some(error)) => {
                    format!("\u{51e0}\u{4f55}: \u{8bfb}\u{53d6}\u{5931}\u{8d25} ({error})")
                }
                (None, None) => "\u{51e0}\u{4f55}: \u{6682}\u{4e0d}\u{53ef}\u{7528}".to_string(),
            };
            let feedback = state
                .copy_feedback
                .as_deref()
                .map_or(String::new(), |feedback| {
                    format!("\n\u{72b6}\u{6001}: {feedback}")
                });
            format!(
                "\u{6a21}\u{578b}\u{9884}\u{89c8}\n{controls}\n\n\u{5f53}\u{524d}\u{805a}\u{7126}: {}\n\u{8def}\u{5f84}: {}\n\u{7c7b}\u{522b}: {}  \u{5927}\u{5c0f}: {} KB  \u{5e8f}\u{53f7}: {}/{}\n{geometry}{feedback}",
                model.display_name,
                model.path,
                model.category,
                model.file_size_kb,
                state.focused_index + 1,
                state.models.len(),
            )
        }
        None => format!(
            "\u{6a21}\u{578b}\u{9884}\u{89c8}\n{controls}\n\n\u{5f53}\u{524d}\u{6ca1}\u{6709}\u{53ef}\u{9884}\u{89c8}\u{6a21}\u{578b}: {fallback}"
        ),
    }
}

#[allow(dead_code)]
fn preview_overlay_text(state: &ModelPreviewState, fallback: &str) -> String {
    let controls = "Tab：切换相机模式\n自由相机：右键拖拽视角，WASD移动，Q/E升降，Shift加速\n轨道相机：A/D旋转，W/S缩放，Q/E调整高度，R重置\n点击：单选，Shift/Ctrl+点击：多选，[ / ]：切换模型，Space：自动旋转\nCtrl+A：全选，C：清空，Ctrl+C：复制 AI 优化提示，Esc：退出";
    match state.models.get(state.focused_index) {
        Some(model) => {
            let geometry = match (&model.asset_info, &model.asset_info_error) {
                (Some(info), _) => format!(
                    "几何：{} 三角形 / {} 顶点 / {} 网格 / {} 节点 / {} 材质\n包围盒：{:.2} × {:.2} × {:.2} 米",
                    info.triangles,
                    info.vertices,
                    info.meshes,
                    info.nodes,
                    info.materials,
                    info.dimensions[0],
                    info.dimensions[1],
                    info.dimensions[2],
                ),
                (None, Some(error)) => format!("几何：读取失败（{error}）"),
                (None, None) => "几何：暂不可用".to_string(),
            };
            format!(
                "模型预览\n{controls}\n\n当前聚焦：{}\n鼠标悬停：{}\n已选模型：{}\n路径：{}\n类别：{}  大小：{} KB  序号：{}/{}\n{}{}",
                model.display_name,
                hovered_model_name(state),
                selected_model_summary(state),
                model.path,
                model.category,
                model.file_size_kb,
                state.focused_index + 1,
                state.models.len(),
                geometry,
                state
                    .copy_feedback
                    .as_deref()
                    .map_or(String::new(), |feedback| format!("\n状态：{feedback}")),
            )
        }
        None => format!("模型预览\n{controls}\n\n当前没有可预览模型：{fallback}"),
    }
}

#[allow(dead_code)]
fn legacy_preview_overlay_text(state: &ModelPreviewState, fallback: &str) -> String {
    match state.models.get(state.focused_index) {
        Some(model) => {
            let asset_info = match (&model.asset_info, &model.asset_info_error) {
                (Some(info), _) => format!(
                    "几何：{} 三角形 / {} 顶点 / {} 网格 / {} 节点 / {} 材质\n包围盒：{:.2} × {:.2} × {:.2} 米",
                    info.triangles,
                    info.vertices,
                    info.meshes,
                    info.nodes,
                    info.materials,
                    info.dimensions[0],
                    info.dimensions[1],
                    info.dimensions[2],
                ),
                (None, Some(error)) => format!("几何：读取失败（{error}）"),
                (None, None) => "几何：暂不可用".to_string(),
            };
            format!(
                "模型预览\nA/D：旋转相机  W/S：缩放  Q/E：调整高度  R：重置\n点击：单选  Shift+点击：多选  [/]：切换模型  空格：自动旋转  J/L：旋转模型\nCtrl+A：全选  C：清空选择  Ctrl+C：复制 AI 优化提示  Esc：退出\n当前聚焦：{}\n鼠标悬停：{}\n已选模型：{}\n路径：{}\n类别：{}  大小：{} KB  序号：{}/{}\n{}{}",
                model.display_name,
                hovered_model_name(state),
                selected_model_summary(state),
                model.path,
                model.category,
                model.file_size_kb,
                state.focused_index + 1,
                state.models.len(),
                asset_info,
                state
                    .copy_feedback
                    .as_deref()
                    .map_or(String::new(), |feedback| format!("\n状态：{feedback}")),
            )
        }
        None => format!(
            "模型预览\nA/D：旋转相机  W/S：缩放  Q/E：调整高度  R：重置\n点击：单选  Shift+点击：多选  [/]：切换模型  空格：自动旋转  J/L：旋转模型\nCtrl+A：全选  C：清空选择  Ctrl+C：复制 AI 优化提示  Esc：退出\n当前聚焦：{fallback}"
        ),
    }
}

fn hovered_model_name(state: &ModelPreviewState) -> &str {
    state
        .hovered_index
        .and_then(|index| state.models.get(index))
        .map(|model| model.display_name.as_str())
        .unwrap_or("无")
}

fn selected_model_summary(state: &ModelPreviewState) -> String {
    if state.selected_indices.is_empty() {
        return "无（Shift+点击以多选）".to_string();
    }
    let mut names = state
        .selected_indices
        .iter()
        .filter_map(|index| state.models.get(*index))
        .map(|model| model.display_name.as_str())
        .take(3)
        .collect::<Vec<_>>();
    let selected_count = state.selected_indices.len();
    if selected_count > names.len() {
        names.push("…");
    }
    format!("{}（{} 个）", names.join("、"), selected_count)
}

fn update_overlay(
    state: Res<ModelPreviewState>,
    mut q: Query<&mut Text, With<PreviewOverlayText>>,
) {
    if !state.is_changed() {
        return;
    }
    if let Ok(mut text) = q.single_mut() {
        text.0 = preview_overlay_text_unicode(&state, "\u{65e0}");
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
    if camera.mode == PreviewCameraMode::Orbit {
        let width = model.aabb_max[0] - model.aabb_min[0];
        let height = model.aabb_max[1] - model.aabb_min[1];
        let depth = model.aabb_max[2] - model.aabb_min[2];
        let span = width.max(height).max(depth).max(0.1);
        camera.distance = (span * 1.7).max(2.2);
        camera.height = (height * 0.8).max(1.2);
    }
}

fn ray_aabb_focus_controls(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<PreviewCameraMarker>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
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
        let multi_select = keys.pressed(KeyCode::ShiftLeft)
            || keys.pressed(KeyCode::ShiftRight)
            || keys.pressed(KeyCode::ControlLeft)
            || keys.pressed(KeyCode::ControlRight);
        if multi_select {
            if !state.selected_indices.insert(index) {
                state.selected_indices.remove(&index);
            }
        } else {
            state.selected_indices.clear();
            state.selected_indices.insert(index);
        }
        focus_preview_index(index, &mut state, &mut preview_camera);
        write_selection_manifest(&state);
    }
}

fn handle_preview_buttons(
    mut state: ResMut<ModelPreviewState>,
    mut clipboard: ResMut<Clipboard>,
    buttons: Query<(&Interaction, &CopyOptimizationButton), Changed<Interaction>>,
) {
    for (interaction, _) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        copy_model_selection(&mut state, &mut clipboard);
    }
}

fn keyboard_selection_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ModelPreviewState>,
    mut clipboard: ResMut<Clipboard>,
) {
    let modifier = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if modifier && keys.just_pressed(KeyCode::KeyA) {
        state.selected_indices = (0..state.models.len()).collect();
        state.copy_feedback = Some(format!("已选中 {} 个模型", state.selected_indices.len()));
        write_selection_manifest(&state);
    }
    if modifier && keys.just_pressed(KeyCode::KeyC) {
        copy_model_selection(&mut state, &mut clipboard);
    }
}

fn copy_model_selection(state: &mut ModelPreviewState, clipboard: &mut Clipboard) {
    if state.selected_indices.is_empty() && state.focused_index < state.models.len() {
        state.selected_indices.insert(state.focused_index);
        write_selection_manifest(state);
    }
    let indices = selection_indices_for_copy(state);
    let payload = optimization_prompt(state, &indices);
    let count = indices.len();
    state.copy_feedback = match clipboard.set_text(payload) {
        Ok(()) => Some(format!("已复制 {} 个模型的 AI 优化提示", count)),
        Err(error) => Some(format!("复制失败：{error}")),
    };
}

fn selection_indices_for_copy(state: &ModelPreviewState) -> Vec<usize> {
    if state.selected_indices.is_empty() {
        return state
            .models
            .get(state.focused_index)
            .map(|_| vec![state.focused_index])
            .unwrap_or_default();
    }
    state
        .selected_indices
        .iter()
        .copied()
        .filter(|index| *index < state.models.len())
        .collect()
}

fn optimization_prompt(state: &ModelPreviewState, indices: &[usize]) -> String {
    let mut prompt = String::from(
        "你是 Last Kingdom 2 的模型优化助手。请只优化下面列出的模型，并根据它们的 GLB 信息，\
         判断每个模型最值得优先修复的视觉问题，给出可执行方案。\n\n\
         项目约束：先阅读 tools/model_catalog.py，找到每个资产唯一的确定性生成器；\
         优先修改 tools/models_lib.py 或资产所属的 tools/ 生成器，不要直接手工修改或只替换 GLB。\
         完成后运行生成器审计、规范构建、model_pipeline validate，以及对应的 model-preview-shot。\n\n",
    );
    prompt.push_str(&format!("选中模型数：{}\n\n", indices.len()));
    for (ordinal, index) in indices.iter().enumerate() {
        let Some(model) = state.models.get(*index) else {
            continue;
        };
        prompt.push_str(&format!("## {}. {}\n", ordinal + 1, model.display_name));
        prompt.push_str(&format!("- 路径：{}\n", model.path));
        prompt.push_str(&format!("- 类别：{}\n", model.category));
        prompt.push_str(&format!("- 文件大小：{} KB\n", model.file_size_kb));
        match (&model.asset_info, &model.asset_info_error) {
            (Some(info), _) => {
                prompt.push_str(&format!(
                    "- 几何：{} 三角形 / {} 顶点 / {} 网格 / {} 节点 / {} 材质\n",
                    info.triangles, info.vertices, info.meshes, info.nodes, info.materials
                ));
                prompt.push_str(&format!(
                    "- 包围盒：{:.2} × {:.2} × {:.2} m\n",
                    info.dimensions[0], info.dimensions[1], info.dimensions[2]
                ));
            }
            (None, Some(error)) => prompt.push_str(&format!("- 几何：读取失败：{error}\n")),
            (None, None) => prompt.push_str("- 几何：暂不可用\n"),
        }
        prompt.push('\n');
    }
    prompt.push_str(
        "请按模型分别输出：1）当前最可能的问题；2）问题的证据；3）\
         生成器/共享助手的具体修改建议；4）需要运行的验证命令。先给方案，\
         不要直接假设所有问题都应通过增加细节解决。不要修改未选中的模型。\n",
    );
    prompt
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
        } else if state.selected_indices.contains(&base.index) {
            materials.selected.clone()
        } else if Some(base.index) == state.hovered_index {
            materials.hovered.clone()
        } else {
            materials.normal.clone()
        };
    }
}

fn camera_pos(camera: &PreviewCamera) -> Vec3 {
    camera.center
        + match camera.view {
            PreviewView::Orbit => Vec3::new(
                camera.yaw.sin() * camera.distance,
                camera.height,
                camera.yaw.cos() * camera.distance,
            ),
            // Generated animals face +X, so front looks along the length axis
            // toward the head and side looks through the depth axis.
            PreviewView::Front => Vec3::new(camera.distance, camera.height, 0.0),
            PreviewView::Side => Vec3::new(0.0, camera.height, camera.distance),
            PreviewView::Top => Vec3::new(0.0, camera.height, 0.0),
        }
}

fn camera_up(camera: &PreviewCamera) -> Vec3 {
    if camera.view == PreviewView::Top {
        Vec3::Z
    } else {
        Vec3::Y
    }
}

fn free_forward(yaw: f32, pitch: f32) -> Vec3 {
    let cos_pitch = pitch.cos();
    Vec3::new(yaw.sin() * cos_pitch, pitch.sin(), -yaw.cos() * cos_pitch).normalize_or_zero()
}

fn free_angles(position: Vec3, target: Vec3) -> (f32, f32) {
    let direction = (target - position).normalize_or_zero();
    if direction.length_squared() <= f32::EPSILON {
        return (0.0, 0.0);
    }
    (
        direction.x.atan2(-direction.z),
        direction.y.clamp(-1.0, 1.0).asin(),
    )
}

fn toggle_camera_mode(camera: &mut PreviewCamera) {
    match camera.mode {
        PreviewCameraMode::Orbit => {
            let position = camera_pos(camera);
            let (yaw, pitch) = free_angles(position, camera.center);
            camera.free_position = position;
            camera.free_yaw = yaw;
            camera.free_pitch = pitch;
            camera.mode = PreviewCameraMode::Free;
        }
        PreviewCameraMode::Free => {
            let offset = camera.free_position - camera.center;
            let horizontal_distance = (offset.x * offset.x + offset.z * offset.z).sqrt();
            if horizontal_distance > f32::EPSILON {
                camera.yaw = offset.x.atan2(offset.z);
                camera.distance = horizontal_distance.max(2.2);
                camera.height = offset.y;
            }
            camera.mode = PreviewCameraMode::Orbit;
        }
    }
}

fn reset_camera(camera: &mut PreviewCamera) {
    match camera.mode {
        PreviewCameraMode::Orbit => {
            camera.yaw = -0.30;
            camera.distance = 7.0;
            camera.height = 3.7;
        }
        PreviewCameraMode::Free => {
            camera.free_position = camera.center + Vec3::new(-2.0, 2.4, 6.0);
            (camera.free_yaw, camera.free_pitch) = free_angles(camera.free_position, camera.center);
        }
    }
}

fn camera_controls(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut camera: ResMut<PreviewCamera>,
    mut q: Query<&mut Transform, With<PreviewCameraMarker>>,
) {
    let dt = time.delta_secs();
    if keys.just_pressed(KeyCode::Tab) {
        toggle_camera_mode(&mut camera);
    }

    let mut mouse_delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        mouse_delta += event.delta;
    }

    match camera.mode {
        PreviewCameraMode::Orbit => {
            let modifier =
                keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
            if !modifier && keys.pressed(KeyCode::KeyA) {
                camera.yaw -= 1.2 * dt;
            }
            if keys.pressed(KeyCode::KeyD) {
                camera.yaw += 1.2 * dt;
            }
            if keys.pressed(KeyCode::KeyW) {
                camera.distance = (camera.distance - 8.0 * dt).max(4.0);
            }
            if keys.pressed(KeyCode::KeyS) {
                camera.distance = (camera.distance + 8.0 * dt).min(60.0);
            }
            if keys.pressed(KeyCode::KeyQ) {
                camera.height = (camera.height - 4.0 * dt).max(1.2);
            }
            if keys.pressed(KeyCode::KeyE) {
                camera.height = (camera.height + 4.0 * dt).min(24.0);
            }
        }
        PreviewCameraMode::Free => {
            if mouse.pressed(MouseButton::Right) {
                camera.free_yaw -= mouse_delta.x * 0.004;
                camera.free_pitch = (camera.free_pitch - mouse_delta.y * 0.004).clamp(-1.50, 1.50);
            }
            let forward = free_forward(camera.free_yaw, 0.0);
            let right = Vec3::new(-forward.z, 0.0, forward.x);
            let mut movement = Vec3::ZERO;
            if keys.pressed(KeyCode::KeyW) {
                movement += forward;
            }
            if keys.pressed(KeyCode::KeyS) {
                movement -= forward;
            }
            if keys.pressed(KeyCode::KeyA) {
                movement -= right;
            }
            if keys.pressed(KeyCode::KeyD) {
                movement += right;
            }
            if keys.pressed(KeyCode::KeyQ) {
                movement -= Vec3::Y;
            }
            if keys.pressed(KeyCode::KeyE) {
                movement += Vec3::Y;
            }
            if movement.length_squared() > 0.0 {
                let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
                {
                    24.0
                } else {
                    8.0
                };
                camera.free_position += movement.normalize() * speed * dt;
            }
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        reset_camera(&mut camera);
    }

    if let Ok(mut transform) = q.single_mut() {
        let (position, target) = match camera.mode {
            PreviewCameraMode::Orbit => (camera_pos(&camera), camera.center),
            PreviewCameraMode::Free => (
                camera.free_position,
                camera.free_position + free_forward(camera.free_yaw, camera.free_pitch),
            ),
        };
        transform.translation = position;
        transform.look_at(target, camera_up(&camera));
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
    let mut selection_changed = false;
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
    let modifier = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if keys.just_pressed(KeyCode::KeyC) && !modifier {
        selection_changed = !state.selected_indices.is_empty();
        state.selected_indices.clear();
    }
    if changed {
        focus_preview_index(state.focused_index, &mut state, &mut camera);
    }
    if selection_changed {
        write_selection_manifest(&state);
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
    if state.frame < STABILIZATION_FRAMES {
        return;
    }
    if pending > 0 || unresolved_bounds > 0 || !state.layout_ready {
        if state.frame < BOUNDS_FALLBACK_FRAMES + STABILIZATION_FRAMES {
            return;
        }
        warn!(
            "[model-preview] taking screenshot with incomplete assets: pending={}, unresolved_bounds={}, layout_ready={}",
            pending, unresolved_bounds, state.layout_ready
        );
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
        BevyAabb, MAIN_APP_MODEL_PATHS, MODEL_GAP, MODEL_PREVIEW_EXTRA_MODELS,
        PROCEDURAL_GRASS_PREVIEW_PATH, PreviewCamera, PreviewCameraMode, PreviewView, camera_pos,
        collect_main_app_glbs, collect_preview_glbs, free_forward, game_content_registry,
        inspect_preview_asset_file, packed_preview_layout, parse_selection_filters,
        preview_grid_position, preview_path_matches, toggle_camera_mode, transformed_mesh_aabb,
        workspace_asset_root,
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
    fn free_camera_toggle_preserves_view_direction() {
        let center = Vec3::new(2.0, 1.0, -3.0);
        let mut camera = PreviewCamera {
            mode: PreviewCameraMode::Orbit,
            view: PreviewView::Orbit,
            yaw: 0.7,
            distance: 8.0,
            height: 3.0,
            center,
            free_position: Vec3::ZERO,
            free_yaw: 0.0,
            free_pitch: 0.0,
        };
        let orbit_position = camera_pos(&camera);
        let expected_direction = (center - orbit_position).normalize();

        toggle_camera_mode(&mut camera);

        assert_eq!(camera.mode, PreviewCameraMode::Free);
        assert!(free_forward(camera.free_yaw, camera.free_pitch).dot(expected_direction) > 0.999);
        assert_eq!(camera.free_position, orbit_position);

        toggle_camera_mode(&mut camera);

        assert_eq!(camera.mode, PreviewCameraMode::Orbit);
        assert!(camera_pos(&camera).distance(orbit_position) < 0.0001);
    }

    #[test]
    fn preview_view_parsing_supports_three_deterministic_views() {
        assert_eq!(PreviewView::parse("front"), PreviewView::Front);
        assert_eq!(PreviewView::parse("side"), PreviewView::Side);
        assert_eq!(PreviewView::parse("top"), PreviewView::Top);
        assert_eq!(PreviewView::Top.suffix(), "_top");
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
    fn preview_asset_metadata_reads_geometry_from_fish() {
        let root = workspace_asset_root();
        let info = inspect_preview_asset_file(&root.join("animals/fish.glb")).unwrap();

        assert_eq!(info.meshes, 1);
        assert_eq!(info.nodes, 1);
        assert!(info.materials > 0);
        assert!(info.vertices > 0);
        assert!(info.triangles > 0);
        assert!(info.dimensions.iter().all(|value| *value > 0.0));
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
        let main_app = collect_main_app_glbs(&root);
        assert!(all.len() >= main_app.len());
    }

    #[test]
    fn main_app_model_preview_uses_gameplay_asset_sources() {
        let root = workspace_asset_root();
        let main_app = collect_preview_glbs(&root, false, None);

        assert!(!main_app.is_empty());
        for (path, category) in main_app {
            if path == PROCEDURAL_GRASS_PREVIEW_PATH {
                assert_eq!(category, "procedural");
                continue;
            }
            assert_eq!(category, "main-app");
            assert!(
                root.join(&path).is_file(),
                "main-app model does not exist: {path}"
            );
        }
        let main_app = collect_main_app_glbs(&root);
        for path in MAIN_APP_MODEL_PATHS
            .iter()
            .chain(MODEL_PREVIEW_EXTRA_MODELS.iter())
        {
            assert!(
                main_app.iter().any(|(candidate, _)| candidate == path),
                "missing direct main-app model: {path}"
            );
        }
        for definition in game_content_registry().definitions() {
            let Some(visual) = &definition.visual else {
                continue;
            };
            assert!(
                main_app
                    .iter()
                    .any(|(candidate, _)| candidate == &visual.model_path),
                "missing registry model: {}",
                visual.model_path
            );
        }
        for required in [
            "procedural/pretty/sokpop_gatherer.glb",
            "procedural/pretty/villager.glb",
            "procedural/pretty/hoplite_golem_hammer.glb",
            "procedural/pretty/hoplite_midas_sword.glb",
            "procedural/pretty/cart.glb",
        ] {
            assert!(
                main_app.iter().any(|(path, _)| path == required),
                "required preview model missing: {required}"
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

    #[test]
    fn procedural_grass_is_selectable_in_model_preview() {
        let root = workspace_asset_root();
        let grass = collect_preview_glbs(&root, false, Some("grass"));

        assert_eq!(grass.len(), 1);
        assert_eq!(grass[0].0, PROCEDURAL_GRASS_PREVIEW_PATH);
        assert_eq!(grass[0].1, "procedural");
    }

    #[test]
    fn selection_filters_accept_multiple_stems() {
        assert_eq!(
            parse_selection_filters("fish,deer.glb, "),
            vec!["fish", "deer"]
        );
        assert!(preview_path_matches("animals/fish.glb", "fish"));
        assert!(preview_path_matches("animals/deer.glb", "animals/deer"));
        assert!(!preview_path_matches("animals/fox.glb", "fish"));
    }
}
