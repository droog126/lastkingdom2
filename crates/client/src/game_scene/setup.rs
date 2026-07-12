//! Initial scene construction: rendering resources, lighting, terrain, the
//! living scene population (player, boss, trees, grass, clouds, rain), and
//! the camera. Everything runs once on `Startup`.

use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::VolumetricLight;
use bevy::pbr::{AtmosphereSettings, ScreenSpaceAmbientOcclusion};
use bevy::prelude::*;

use super::content_visuals::{spawn_content_visuals, LivingContentLayout};
use super::offline::OfflineNature;
use super::procedural_motion::ProceduralTreeSway;
use super::state::{
    BossActor, Cloud, FarmVisualMaterials, GrassTuft, LivingCameraRig, LivingSceneCamera,
    LivingSun, PlayerActor, PlayerIkPart, PlayerIkPartKind, PlayerJump, PlayerMotion, RainDrop,
    SceneMaterials, FARM_PLOT_POSITIONS,
};
use super::util::{
    hash01, spawn_asset, BOSS_PATH, BRANCH_PATH, CLOUD_PATH, HILL_PATH, PINE_TREE_PATH, ROCK_PATH,
    ROUND_TREE_PATH, TREE_PATH,
};
use lk2_core::pvp::{Health, PvpCombatant, SimpleWeapon};

pub fn setup_rendering(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.47, 0.61, 0.72)));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.82, 0.90, 0.84),
        brightness: 0.82,
        affects_lightmapped_meshes: true,
    });
}

pub fn setup_lighting(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 20_000.0,
            shadow_maps_enabled: true,
            color: Color::srgb(1.0, 0.91, 0.76),
            ..default()
        },
        Transform::from_xyz(-24.0, 38.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
        LivingSun,
        VolumetricLight,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
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
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.49, 0.22),
        perceptual_roughness: 0.94,
        ..default()
    });
    let path = materials.add(StandardMaterial {
        base_color: Color::srgb(0.54, 0.43, 0.29),
        perceptual_roughness: 0.90,
        ..default()
    });
    let water = materials.add(StandardMaterial {
        base_color: Color::srgba(0.20, 0.50, 0.66, 0.88),
        metallic: 0.05,
        perceptual_roughness: 0.24,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let slash_mesh = meshes.add(Torus::new(1.2, 1.36));
    let slash_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.98, 0.83, 0.32, 0.82),
        emissive: Color::srgb(1.5, 0.72, 0.08).into(),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.insert_resource(SceneMaterials {
        ground: ground.clone(),
        slash_mesh,
        slash_material,
    });

    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(27.0, 0.65))),
        MeshMaterial3d(ground),
        Transform::from_xyz(0.0, -0.36, 0.0),
        Name::new("living_forest_basin"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(5.0, 0.05, 34.0))),
        MeshMaterial3d(path),
        Transform::from_xyz(-1.5, 0.018, 0.0).with_rotation(Quat::from_rotation_y(-0.10)),
        Name::new("forest_path"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(5.2, 0.08))),
        MeshMaterial3d(water),
        Transform::from_xyz(12.5, -0.02, 7.5),
        Name::new("rain_pool"),
    ));

    for (index, pos) in [
        Vec3::new(-19.0, -0.05, -8.0),
        Vec3::new(18.0, -0.05, -10.0),
        Vec3::new(-15.0, -0.05, 15.0),
        Vec3::new(17.0, -0.05, 15.0),
    ]
    .into_iter()
    .enumerate()
    {
        spawn_asset(
            &mut commands,
            &asset_server,
            HILL_PATH,
            pos,
            1.35 + index as f32 * 0.07,
            index as f32 * 0.8,
            "terrain_hill",
        );
    }
}

pub fn setup_living_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    nature: Res<OfflineNature>,
    content_layout: Res<LivingContentLayout>,
) {
    spawn_content_visuals(&mut commands, &asset_server, &content_layout);

    spawn_player_avatar(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(-2.0, 0.0, 11.0),
        std::f32::consts::PI,
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
        Health {
            current: 24.0,
            max: 24.0,
            invuln_until_tick: 0,
        },
    ));

    let tree_layout = [
        (-20.0, -4.0, 0.1, 1.05, TREE_PATH),
        (-17.0, 6.0, -0.5, 0.92, ROUND_TREE_PATH),
        (-12.0, 17.0, 0.4, 1.08, PINE_TREE_PATH),
        (-4.0, 20.0, -0.2, 0.90, TREE_PATH),
        (7.0, 20.0, 0.7, 1.02, ROUND_TREE_PATH),
        (17.0, 14.0, -0.8, 0.95, PINE_TREE_PATH),
        (21.0, 3.0, 0.3, 1.08, TREE_PATH),
        (19.0, -9.0, -0.4, 0.96, ROUND_TREE_PATH),
        (11.0, -19.0, 0.6, 1.06, PINE_TREE_PATH),
        (-8.0, -20.0, -0.1, 0.92, TREE_PATH),
        (-19.0, -15.0, 0.8, 1.02, ROUND_TREE_PATH),
    ];
    for (index, (x, z, yaw, scale, path)) in tree_layout.into_iter().enumerate() {
        let position = Vec3::new(x, 0.0, z);
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
    }

    for index in 0..16 {
        let angle = index as f32 * 2.399;
        let radius = 7.0 + hash01(index, 7) * 12.0;
        let pos = Vec3::new(angle.cos() * radius, 0.03, angle.sin() * radius);
        spawn_asset(
            &mut commands,
            &asset_server,
            if index % 3 == 0 {
                ROCK_PATH
            } else {
                BRANCH_PATH
            },
            pos,
            0.72 + hash01(index, 11) * 0.45,
            angle + 0.4,
            "forest_floor_detail",
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
    for index in 0..150 {
        let angle = hash01(index, 17) * std::f32::consts::TAU;
        let radius = 4.5 + hash01(index, 29).sqrt() * 19.0;
        let pos = Vec3::new(angle.cos() * radius, 0.02, angle.sin() * radius);
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
                growth_threshold: index as f32 / 150.0 * 32.0,
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

pub fn setup_camera(mut commands: Commands) {
    commands.insert_resource(LivingCameraRig::default());
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(20.0, 18.0, 26.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        AtmosphereSettings::default(),
        Exposure { ev100: 12.7 },
        Tonemapping::AcesFitted,
        Msaa::Off,
        ScreenSpaceAmbientOcclusion::default(),
        LivingSceneCamera,
    ));
}

fn spawn_player_avatar(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    yaw: f32,
) {
    commands.spawn((
        Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
        PlayerActor,
        PlayerMotion::default(),
        PlayerJump::default(),
        PvpCombatant::default(),
        SimpleWeapon::default(),
        Health::default(),
        Name::new("player"),
    ));

    let skin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.63, 0.43),
        perceptual_roughness: 0.95,
        ..default()
    });
    let shirt = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.52, 0.30),
        perceptual_roughness: 0.96,
        ..default()
    });
    let trousers = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.36, 0.55),
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
