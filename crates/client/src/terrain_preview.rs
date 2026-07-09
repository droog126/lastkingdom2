//! Isometric terrain preview mode for judging world composition.
//!
//! Run:
//!   cargo run -p lk2-client -- --terrain-preview
//!   cargo run -p lk2-client -- --terrain-preview --terrain-preview-shot

use std::path::PathBuf;
use std::time::{Duration, Instant};

use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::{AtmosphereSettings, ScreenSpaceReflections};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::text::LetterSpacing;
use bevy::window::{PresentMode, WindowResolution};
use bevy_world_serialization::WorldAsset;

pub const TERRAIN_PREVIEW_OUTPUT_DIR: &str = "screenshots/terrain_preview";

const STABILIZATION_FRAMES: u64 = 180;

#[derive(Resource)]
struct TerrainPreviewState {
    frame: u64,
    auto_shot: bool,
    shot_requested: bool,
    exit_deadline: Option<Instant>,
    png_path: PathBuf,
    scene_handles: Vec<Handle<WorldAsset>>,
}

#[derive(Component)]
struct TerrainPreviewCamera;

pub fn run_terrain_preview() {
    let args: Vec<String> = std::env::args().collect();
    let auto_shot = args.iter().any(|a| a == "--terrain-preview-shot");
    let asset_root = workspace_asset_root();
    let output_dir = PathBuf::from(TERRAIN_PREVIEW_OUTPUT_DIR);
    let _ = std::fs::create_dir_all(&output_dir);
    let png_path = output_dir.join("terrain_preview.png");
    if auto_shot {
        let _ = std::fs::remove_file(&png_path);
    }

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin { file_path: asset_root.to_string_lossy().into_owned(), ..default() })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "lk2 terrain preview".into(),
                    resolution: WindowResolution::new(1280, 960),
                    present_mode: PresentMode::Immediate,
                    focused: true,
                    visible: true,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::log::LogPlugin { level: bevy::log::Level::INFO, ..default() }),
    );
    app.insert_resource(TerrainPreviewState {
        frame: 0,
        auto_shot,
        shot_requested: false,
        exit_deadline: None,
        png_path,
        scene_handles: Vec::new(),
    });
    app.add_systems(
        Startup,
        (
            setup_rendering,
            setup_lights,
            setup_world,
            setup_camera,
            setup_overlay,
        )
            .chain(),
    );
    app.add_systems(Update, (maybe_take_screenshot, exit_preview).chain());
    app.run();
}

fn workspace_asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets")
}

fn setup_rendering(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.58, 0.68, 0.76)));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.94, 0.94, 0.86),
        brightness: 0.72,
        affects_lightmapped_meshes: true,
    });
}

fn setup_lights(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 32_000.0,
            shadow_maps_enabled: true,
            color: Color::srgb(1.0, 0.90, 0.74),
            ..default()
        },
        Transform::from_xyz(-32.0, 54.0, 26.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 6_500.0,
            shadow_maps_enabled: false,
            color: Color::srgb(0.58, 0.72, 1.0),
            ..default()
        },
        Transform::from_xyz(32.0, 42.0, -34.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn setup_world(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<TerrainPreviewState>,
) {
    let grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.68, 0.30),
        perceptual_roughness: 0.86,
        ..default()
    });
    let wild_grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.60, 0.24),
        perceptual_roughness: 0.92,
        ..default()
    });
    let road = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.67, 0.59),
        perceptual_roughness: 0.78,
        ..default()
    });
    let road_light = materials.add(StandardMaterial {
        base_color: Color::srgb(0.76, 0.74, 0.66),
        perceptual_roughness: 0.82,
        ..default()
    });
    let road_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.56, 0.55, 0.50),
        perceptual_roughness: 0.86,
        ..default()
    });
    let field = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.68, 0.25),
        perceptual_roughness: 0.88,
        ..default()
    });
    let farm_soil = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.24, 0.14),
        perceptual_roughness: 0.9,
        ..default()
    });
    let cyan = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.95, 0.95),
        emissive: Color::srgb(0.0, 1.4, 1.3).into(),
        perceptual_roughness: 0.3,
        ..default()
    });
    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.50, 0.46),
        perceptual_roughness: 0.8,
        ..default()
    });
    let dark_grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.53, 0.20),
        perceptual_roughness: 0.92,
        ..default()
    });
    let flower_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.74, 0.28),
        perceptual_roughness: 0.8,
        ..default()
    });
    let path_dirt = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.42, 0.28),
        perceptual_roughness: 0.9,
        ..default()
    });
    let raised_grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.43, 0.64, 0.28),
        perceptual_roughness: 0.88,
        ..default()
    });
    let edge_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.45, 0.20),
        perceptual_roughness: 0.9,
        ..default()
    });

    let plane = meshes.add(Plane3d::default().mesh().size(84.0, 84.0));
    commands.spawn((
        Mesh3d(plane),
        MeshMaterial3d(grass.clone()),
        Transform::from_translation(Vec3::new(0.0, -0.04, 0.0)),
    ));

    spawn_raised_lots(&mut commands, &mut meshes, &raised_grass, &edge_mat);
    spawn_ground_detail(
        &mut commands,
        &mut meshes,
        &dark_grass,
        &flower_mat,
        &path_dirt,
    );
    spawn_wild_side(
        &mut commands,
        &asset_server,
        &mut state,
        &mut meshes,
        &wild_grass,
        &rock_mat,
    );
    spawn_village_roads(&mut commands, &mut meshes, &road, &road_light, &road_dark);
    spawn_road_edges(&mut commands, &mut meshes, &edge_mat, &rock_mat);
    spawn_farms(&mut commands, &mut meshes, &farm_soil, &field);
    spawn_buildings(&mut commands, &asset_server, &mut state);
    spawn_village_decor(
        &mut commands,
        &asset_server,
        &mut state,
        &mut meshes,
        &cyan,
        &rock_mat,
    );
}

fn spawn_raised_lots(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    raised_grass: &Handle<StandardMaterial>,
    edge_mat: &Handle<StandardMaterial>,
) {
    let lot_mesh = meshes.add(Cuboid::new(1.0, 0.16, 1.0));
    let edge_mesh = meshes.add(Cuboid::new(1.0, 0.12, 1.0));
    let tuft_mesh = meshes.add(Cuboid::new(0.32, 0.28, 0.24));
    for (cx, cz, sx, sz) in [
        (-26.0, -18.0, 15.0, 12.0),
        (-26.0, 8.0, 17.0, 17.0),
        (4.0, -17.0, 9.5, 9.5),
        (16.0, -17.0, 9.5, 9.5),
        (28.0, -17.0, 9.5, 9.5),
        (4.0, 0.0, 9.5, 11.0),
        (16.0, 0.0, 9.5, 11.0),
        (28.0, 0.0, 9.5, 11.0),
        (4.0, 17.0, 9.5, 11.5),
        (16.0, 17.0, 9.5, 11.5),
        (28.0, 17.0, 9.5, 11.5),
    ] {
        commands.spawn((
            Mesh3d(lot_mesh.clone()),
            MeshMaterial3d(raised_grass.clone()),
            Transform::from_xyz(cx, 0.025, cz).with_scale(Vec3::new(sx, 1.0, sz)),
        ));
        for (edge_idx, (ex, ez, esx, esz)) in [
            (cx, cz - sz * 0.5, sx, 0.10),
            (cx, cz + sz * 0.5, sx, 0.10),
            (cx - sx * 0.5, cz, 0.10, sz),
            (cx + sx * 0.5, cz, 0.10, sz),
        ]
        .iter()
        .enumerate()
        {
            commands.spawn((
                Mesh3d(edge_mesh.clone()),
                MeshMaterial3d(edge_mat.clone()),
                Transform::from_xyz(*ex, 0.08, *ez).with_scale(Vec3::new(*esx, 0.55, *esz)),
            ));
            let count = if *esx > *esz {
                (*esx / 2.4) as usize
            } else {
                (*esz / 2.4) as usize
            };
            for n in 0..count.max(1) {
                if (n + edge_idx) % 3 == 0 {
                    continue;
                }
                let t = (n as f32 + 0.5) / count.max(1) as f32 - 0.5;
                let jitter = (hash01(n + edge_idx * 31, 211) - 0.5) * 0.45;
                let pos = if *esx > *esz {
                    Vec3::new(*ex + t * *esx, 0.17, *ez + jitter)
                } else {
                    Vec3::new(*ex + jitter, 0.17, *ez + t * *esz)
                };
                commands.spawn((
                    Mesh3d(tuft_mesh.clone()),
                    MeshMaterial3d(edge_mat.clone()),
                    Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(
                        hash01(n, 213) * std::f32::consts::TAU,
                    )),
                ));
            }
        }
    }
}

fn spawn_ground_detail(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    dark_grass: &Handle<StandardMaterial>,
    flower_mat: &Handle<StandardMaterial>,
    path_dirt: &Handle<StandardMaterial>,
) {
    let patch = meshes.add(Cuboid::new(1.0, 0.025, 1.0));
    for i in 0..30 {
        let x = -39.0 + hash01(i, 101) * 78.0;
        let z = -35.0 + hash01(i, 103) * 70.0;
        let sx = 0.8 + hash01(i, 107) * 1.8;
        let sz = 0.5 + hash01(i, 109) * 1.4;
        let mat = if i % 9 == 0 {
            path_dirt.clone()
        } else {
            dark_grass.clone()
        };
        commands.spawn((
            Mesh3d(patch.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(x, 0.0, z)
                .with_rotation(Quat::from_rotation_y(
                    hash01(i, 111) * std::f32::consts::TAU,
                ))
                .with_scale(Vec3::new(sx, 1.0, sz)),
        ));
    }

    let flower = meshes.add(Cuboid::new(0.12, 0.22, 0.12));
    for i in 0..150 {
        let x = -35.0 + hash01(i, 113) * 72.0;
        let z = -31.0 + hash01(i, 127) * 62.0;
        if x > -3.0 && hash01(i, 131) < 0.45 {
            continue;
        }
        commands.spawn((
            Mesh3d(flower.clone()),
            MeshMaterial3d(flower_mat.clone()),
            Transform::from_xyz(x, 0.13, z),
        ));
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(36.0, 52.0, 42.0).looking_at(Vec3::new(6.0, 0.0, 0.0), Vec3::Y),
        AtmosphereSettings::default(),
        Exposure { ev100: 12.6 },
        Tonemapping::AcesFitted,
        Bloom::NATURAL,
        Msaa::Off,
        ScreenSpaceReflections { min_perceptual_roughness: 0.0..0.0, ..default() },
        TerrainPreviewCamera,
    ));
}

fn setup_overlay(mut commands: Commands, state: Res<TerrainPreviewState>) {
    if state.auto_shot {
        return;
    }
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
            Text::new("TERRAIN PREVIEW\nreference: forest + village + farms\nEsc exit"),
            TextFont {
                font: FontSource::UiMonospace,
                font_size: FontSize::Rem(0.94),
                weight: FontWeight::SEMIBOLD,
                width: FontWidth::SEMI_CONDENSED,
                ..default()
            },
            LetterSpacing::Px(0.3),
            TextColor(Color::WHITE),
        )],
    ));
}

fn spawn_wild_side(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    state: &mut TerrainPreviewState,
    meshes: &mut ResMut<Assets<Mesh>>,
    wild_grass: &Handle<StandardMaterial>,
    rock_mat: &Handle<StandardMaterial>,
) {
    let patch_mesh = meshes.add(Cuboid::new(0.18, 0.16, 1.0));
    let rock_mesh = meshes.add(Cuboid::new(0.55, 0.35, 0.45));
    for i in 0..420 {
        let x = -38.0 + hash01(i, 3) * 42.0;
        let z = -34.0 + hash01(i, 7) * 68.0;
        let h = 0.25 + hash01(i, 11) * 0.5;
        commands.spawn((
            Mesh3d(patch_mesh.clone()),
            MeshMaterial3d(wild_grass.clone()),
            Transform::from_xyz(x, h * 0.5, z)
                .with_rotation(Quat::from_rotation_y(hash01(i, 13) * std::f32::consts::TAU))
                .with_scale(Vec3::new(0.8, h, 0.8)),
        ));
    }
    for i in 0..70 {
        let x = -39.0 + hash01(i, 23) * 42.0;
        let z = -34.0 + hash01(i, 29) * 68.0;
        commands.spawn((
            Mesh3d(rock_mesh.clone()),
            MeshMaterial3d(rock_mat.clone()),
            Transform::from_xyz(x, 0.18, z)
                .with_rotation(Quat::from_rotation_y(hash01(i, 31) * std::f32::consts::TAU))
                .with_scale(Vec3::splat(0.7 + hash01(i, 37) * 0.9)),
        ));
    }

    let trees = [
        (-35.0, -31.0, 1.55),
        (-31.0, -20.0, 1.65),
        (-37.0, -8.0, 1.35),
        (-25.0, 2.0, 1.55),
        (-33.0, 17.0, 1.65),
        (-20.0, 28.0, 1.45),
        (-14.0, -28.0, 1.45),
        (-9.0, -15.0, 1.55),
        (-13.0, 9.0, 1.35),
        (-8.0, 23.0, 1.55),
        (-40.0, 5.0, 1.40),
        (-27.0, 35.0, 1.55),
        (-16.0, -4.0, 1.45),
        (-3.0, 34.0, 1.30),
        (-38.0, 31.0, 1.35),
        (-24.0, -10.0, 1.40),
        (-19.0, 14.0, 1.35),
        (-6.0, 4.0, 1.25),
        (-36.0, 25.0, 1.25),
        (-32.0, 27.0, 1.15),
        (-29.0, 23.0, 1.30),
        (-22.0, 21.0, 1.20),
        (-18.0, 24.0, 1.28),
        (-12.0, 18.0, 1.18),
        (-34.0, -2.0, 1.20),
        (-29.0, -5.0, 1.25),
        (-22.0, -1.0, 1.18),
    ];
    for (x, z, scale) in trees {
        spawn_asset(
            commands,
            asset_server,
            state,
            "procedural/pretty/sokpop_tree.glb",
            Vec3::new(x, 0.0, z),
            scale,
            0.0,
        );
    }
    for i in 0..16 {
        let x = -30.0 + hash01(i, 41) * 24.0;
        let z = -25.0 + hash01(i, 43) * 48.0;
        spawn_asset(
            commands,
            asset_server,
            state,
            "procedural/eco/rabbit.glb",
            Vec3::new(x, 0.0, z),
            0.55,
            hash01(i, 47) * std::f32::consts::TAU,
        );
    }
}

fn spawn_village_roads(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    road: &Handle<StandardMaterial>,
    road_light: &Handle<StandardMaterial>,
    road_dark: &Handle<StandardMaterial>,
) {
    let road_mesh = meshes.add(Cuboid::new(1.0, 0.045, 1.0));
    for x in [-2.0, 10.0, 22.0, 34.0] {
        commands.spawn((
            Mesh3d(road_mesh.clone()),
            MeshMaterial3d(road.clone()),
            Transform::from_xyz(x, 0.02, 0.0).with_scale(Vec3::new(1.35, 1.0, 64.0)),
        ));
    }
    for z in [-24.0, -8.0, 8.0, 24.0] {
        commands.spawn((
            Mesh3d(road_mesh.clone()),
            MeshMaterial3d(road.clone()),
            Transform::from_xyz(16.0, 0.025, z).with_scale(Vec3::new(42.0, 1.0, 1.35)),
        ));
    }

    let stone_mesh = meshes.add(Cuboid::new(0.72, 0.035, 0.56));
    for i in 0..280 {
        let vertical = i % 2 == 0;
        let lane = i / 2;
        let mat = if i % 5 == 0 {
            road_dark.clone()
        } else {
            road_light.clone()
        };
        let jitter = (hash01(i, 171) - 0.5) * 0.32;
        if vertical {
            let x = [-2.0, 10.0, 22.0, 34.0][lane % 4] + jitter;
            let z = -31.0 + ((lane / 4) as f32 % 58.0) * 1.1;
            commands.spawn((
                Mesh3d(stone_mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_xyz(x, 0.075, z)
                    .with_rotation(Quat::from_rotation_y((hash01(i, 173) - 0.5) * 0.18))
                    .with_scale(Vec3::new(0.95 + hash01(i, 175) * 0.25, 1.0, 0.9)),
            ));
        } else {
            let x = -4.5 + ((lane / 4) as f32 % 38.0) * 1.1;
            let z = [-24.0, -8.0, 8.0, 24.0][lane % 4] + jitter;
            commands.spawn((
                Mesh3d(stone_mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_xyz(x, 0.08, z)
                    .with_rotation(Quat::from_rotation_y(
                        std::f32::consts::FRAC_PI_2 + (hash01(i, 177) - 0.5) * 0.18,
                    ))
                    .with_scale(Vec3::new(0.95 + hash01(i, 179) * 0.25, 1.0, 0.9)),
            ));
        }
    }
}

fn spawn_road_edges(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    edge_mat: &Handle<StandardMaterial>,
    rock_mat: &Handle<StandardMaterial>,
) {
    let pebble = meshes.add(Cuboid::new(0.18, 0.12, 0.16));
    let bush = meshes.add(Cuboid::new(0.22, 0.24, 0.22));
    for i in 0..180 {
        let along = -31.0 + hash01(i, 151) * 62.0;
        let vertical = i % 2 == 0;
        let road_coord = [-2.0, 10.0, 22.0, 34.0][i % 4];
        let cross_coord = [-24.0, -8.0, 8.0, 24.0][i % 4];
        let side = if hash01(i, 153) > 0.5 { 1.0 } else { -1.0 };
        let pos = if vertical {
            Vec3::new(road_coord + side * 0.95, 0.11, along)
        } else {
            Vec3::new(along + 16.0, 0.11, cross_coord + side * 0.95)
        };
        let mat = if i % 3 == 0 {
            edge_mat.clone()
        } else {
            rock_mat.clone()
        };
        let mesh = if i % 3 == 0 {
            bush.clone()
        } else {
            pebble.clone()
        };
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(
                    hash01(i, 157) * std::f32::consts::TAU,
                ))
                .with_scale(Vec3::splat(0.65 + hash01(i, 159) * 0.8)),
        ));
    }
}

fn spawn_farms(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    farm_soil: &Handle<StandardMaterial>,
    field: &Handle<StandardMaterial>,
) {
    let soil_mesh = meshes.add(Cuboid::new(1.0, 0.04, 1.0));
    let wheat_mesh = meshes.add(Cuboid::new(0.18, 0.75, 0.18));
    for (cx, cz, sx, sz) in [
        (4.0, -18.0, 7.0, 8.0),
        (28.0, -17.0, 8.0, 8.0),
        (16.0, 18.0, 10.0, 7.0),
        (5.0, 16.0, 6.0, 6.0),
        (30.0, 18.0, 6.0, 6.0),
    ] {
        commands.spawn((
            Mesh3d(soil_mesh.clone()),
            MeshMaterial3d(farm_soil.clone()),
            Transform::from_xyz(cx, 0.04, cz).with_scale(Vec3::new(sx, 1.0, sz)),
        ));
        let mut n = 0;
        for ix in -5..=5 {
            for iz in -4..=4 {
                if n % 2 == 0 {
                    commands.spawn((
                        Mesh3d(wheat_mesh.clone()),
                        MeshMaterial3d(field.clone()),
                        Transform::from_xyz(cx + ix as f32 * 0.62, 0.38, cz + iz as f32 * 0.62),
                    ));
                }
                n += 1;
            }
        }
    }
}

fn spawn_buildings(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    state: &mut TerrainPreviewState,
) {
    let buildings = [
        ("procedural/pretty/house_small.glb", 7.0, -27.0, 1.4, 0.2),
        ("procedural/pretty/tavern.glb", 20.0, -27.0, 1.25, -0.25),
        ("procedural/pretty/house_small.glb", 33.0, -27.0, 1.25, 0.35),
        ("procedural/pretty/barn.glb", 31.0, -3.0, 1.25, 0.15),
        ("procedural/pretty/house_small.glb", 8.0, 3.0, 1.35, -0.35),
        ("procedural/pretty/house_small.glb", 18.0, 3.0, 1.20, 0.1),
        ("procedural/pretty/house_small.glb", 29.0, 3.5, 1.05, 0.45),
        ("procedural/pretty/chapel.glb", 21.0, 13.0, 1.25, 0.0),
        ("procedural/pretty/tavern.glb", 32.0, 22.0, 1.15, 0.4),
        ("procedural/pretty/house_small.glb", 8.0, 29.0, 1.25, -0.15),
        ("procedural/pretty/house_small.glb", 20.0, 30.0, 1.05, 0.2),
        ("procedural/pretty/watchtower.glb", -3.0, 29.0, 1.4, 0.15),
        ("procedural/pretty/watchtower.glb", 36.0, 32.0, 1.35, -0.2),
        ("procedural/pretty/windmill.glb", 4.0, 18.0, 1.25, 0.3),
        ("procedural/pretty/well.glb", 18.0, -3.0, 1.2, 0.0),
        ("procedural/pretty/market_stall.glb", 28.0, 9.0, 1.25, 0.7),
    ];
    for (path, x, z, scale, yaw) in buildings {
        spawn_asset(
            commands,
            asset_server,
            state,
            path,
            Vec3::new(x, 0.0, z),
            scale,
            yaw,
        );
    }
}

fn spawn_village_decor(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    state: &mut TerrainPreviewState,
    meshes: &mut ResMut<Assets<Mesh>>,
    cyan: &Handle<StandardMaterial>,
    rock_mat: &Handle<StandardMaterial>,
) {
    for (x, z) in [
        (0.0, -24.0),
        (12.0, -8.0),
        (24.0, 8.0),
        (36.0, 24.0),
        (10.0, 24.0),
        (34.0, -8.0),
    ] {
        spawn_asset(
            commands,
            asset_server,
            state,
            "procedural/pretty/fountain.glb",
            Vec3::new(x, 0.0, z),
            0.65,
            0.0,
        );
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.08, 1.4))),
            MeshMaterial3d(cyan.clone()),
            Transform::from_xyz(x, 0.7, z),
        ));
    }
    for (x, z) in [
        (3.0, 8.0),
        (12.0, 24.0),
        (30.0, -26.0),
        (34.0, -8.0),
        (3.0, -10.0),
        (25.0, 30.0),
    ] {
        spawn_asset(
            commands,
            asset_server,
            state,
            "procedural/pretty/sokpop_tree.glb",
            Vec3::new(x, 0.0, z),
            1.0,
            0.0,
        );
    }
    for (x, z, yaw) in [
        (3.0, -4.0, 0.0),
        (14.0, -19.0, 1.57),
        (25.0, -4.0, 0.0),
        (4.0, 24.0, 1.57),
        (22.0, 25.0, 0.0),
        (34.0, 4.0, 1.57),
    ] {
        spawn_asset(
            commands,
            asset_server,
            state,
            "procedural/pretty/fence.glb",
            Vec3::new(x, 0.0, z),
            0.75,
            yaw,
        );
    }
    for (x, z, yaw) in [
        (6.0, -8.0, 0.2),
        (18.0, -16.0, -0.2),
        (30.0, 8.0, 0.6),
        (14.0, 8.0, -0.4),
    ] {
        spawn_asset(
            commands,
            asset_server,
            state,
            "procedural/pretty/market_stall.glb",
            Vec3::new(x, 0.0, z),
            0.85,
            yaw,
        );
    }
    for (x, z, yaw) in [
        (2.0, -30.0, 0.0),
        (13.0, -30.0, 1.57),
        (24.0, -22.0, 0.3),
        (3.0, 2.0, 0.1),
        (16.0, 2.0, -0.4),
        (28.0, 2.0, 0.8),
        (8.0, 14.0, 1.1),
        (21.0, 22.0, -0.6),
        (32.0, 30.0, 0.2),
    ] {
        spawn_asset(
            commands,
            asset_server,
            state,
            "procedural/pretty/crate.glb",
            Vec3::new(x, 0.0, z),
            0.55,
            yaw,
        );
    }
    for (x, z, yaw) in [
        (12.0, -30.0, 0.1),
        (25.0, -30.0, 0.3),
        (3.0, 14.0, 0.0),
        (15.0, 30.0, -0.2),
        (36.0, 12.0, 0.5),
    ] {
        spawn_asset(
            commands,
            asset_server,
            state,
            "procedural/pretty/bench.glb",
            Vec3::new(x, 0.0, z),
            0.65,
            yaw,
        );
    }
    for i in 0..26 {
        let x = -4.0 + hash01(i, 61) * 42.0;
        let z = -30.0 + hash01(i, 67) * 64.0;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.32, 0.22, 0.28))),
            MeshMaterial3d(rock_mat.clone()),
            Transform::from_xyz(x, 0.12, z)
                .with_rotation(Quat::from_rotation_y(hash01(i, 71) * std::f32::consts::TAU)),
        ));
    }
}

fn spawn_asset(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    state: &mut TerrainPreviewState,
    path: &'static str,
    pos: Vec3,
    scale: f32,
    yaw: f32,
) {
    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(path));
    commands.spawn((
        WorldAssetRoot(scene.clone()),
        Transform::from_translation(pos)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(scale)),
        Name::new(path),
    ));
    state.scene_handles.push(scene);
}

fn hash01(i: usize, salt: usize) -> f32 {
    let mut x =
        (i as u32).wrapping_mul(1_664_525).wrapping_add((salt as u32).wrapping_mul(1_013_904_223));
    x ^= x >> 16;
    x = x.wrapping_mul(2_246_822_519);
    ((x >> 8) as f32) / ((u32::MAX >> 8) as f32)
}

fn maybe_take_screenshot(
    mut state: ResMut<TerrainPreviewState>,
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
    if state.frame < STABILIZATION_FRAMES {
        return;
    }
    if pending > 0 {
        warn!("[terrain-preview] screenshot with {pending} assets still pending");
    }
    commands.spawn(Screenshot::primary_window()).observe(save_to_disk(state.png_path.clone()));
    state.shot_requested = true;
    state.exit_deadline = Some(Instant::now() + Duration::from_secs(60));
    info!(
        "[terrain-preview] screenshot requested: {}",
        state.png_path.display()
    );
}

fn exit_preview(keys: Res<ButtonInput<KeyCode>>, state: Res<TerrainPreviewState>) {
    if keys.just_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }
    if state.auto_shot && state.shot_requested {
        if let Ok(meta) = std::fs::metadata(&state.png_path) {
            if meta.len() > 0 {
                std::process::exit(0);
            }
        }
        if state.exit_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            warn!("[terrain-preview] screenshot did not flush before timeout");
            std::process::exit(1);
        }
    }
}
