//! A small, playable open-pit mine reached from a cave entrance.
//!
//! This is intentionally authored as a local Bevy scene rather than a flat
//! test room: the player enters on the rim, descends through two benches, and
//! can read the mine shaft, rails, cart, supports, and ore veins from the
//! bottom.

use std::f32::consts::TAU;

use avian3d::prelude::{Collider, ColliderDisabled, RigidBody};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::keybindings::{GameAction, KeyBindings};
use super::offline::OfflineNature;
use super::state::{
    CaveExit, CaveOre, CaveRealmCollider, CaveRealmVisual, CaveTravelState, DimensionId,
    DimensionTravelState, ForestRealmCollider, ForestRealmVisual, LivingCameraRig,
};
use super::state::PlayerActor;
use lk2_core::resource::ResourceKind;

const MINE_OUTER_RADIUS: f32 = 11.0;
const MINE_BENCH_ONE_INNER: f32 = 5.5;
const MINE_BENCH_ONE_OUTER: f32 = 7.4;
const MINE_BENCH_TWO_INNER: f32 = 3.5;
const MINE_BENCH_TWO_OUTER: f32 = 5.5;
const MINE_BOTTOM_RADIUS: f32 = 3.5;
const MINE_BOTTOM_DROP: f32 = 2.55;
const RING_SEGMENTS: usize = 32;

pub fn setup_cave_realm(
    mut commands: Commands,
    terrain: Res<super::state::ProceduralTerrainSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let rim_y = terrain.ground_height(Vec3::ZERO) + 0.06;
    let bench_one_y = rim_y - 0.82;
    let bench_two_y = rim_y - 1.62;
    let bottom_y = rim_y - MINE_BOTTOM_DROP;

    let rim_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.14, 0.10),
        perceptual_roughness: 0.98,
        ..default()
    });
    let bench_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.31, 0.21, 0.15),
        perceptual_roughness: 0.96,
        ..default()
    });
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.11, 0.075, 0.065),
        perceptual_roughness: 1.0,
        ..default()
    });
    let bottom_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.105, 0.08),
        emissive: Color::srgb(0.008, 0.004, 0.002).into(),
        perceptual_roughness: 0.97,
        ..default()
    });

    // Concentric rings leave the centre open, so the depth is visible from
    // the rim. Each ring also has a vertical inner face instead of being a
    // floating paper-thin decal.
    spawn_ring(
        &mut commands,
        &mut meshes,
        &rim_material,
        MINE_BENCH_ONE_OUTER,
        MINE_OUTER_RADIUS,
        rim_y,
        rim_y - 0.62,
        "mine_rim",
    );
    spawn_ring(
        &mut commands,
        &mut meshes,
        &bench_material,
        MINE_BENCH_TWO_OUTER,
        MINE_BENCH_ONE_OUTER,
        bench_one_y,
        bench_one_y - 0.58,
        "mine_bench_one",
    );
    spawn_ring(
        &mut commands,
        &mut meshes,
        &bench_material,
        MINE_BOTTOM_RADIUS,
        MINE_BENCH_TWO_OUTER,
        bench_two_y,
        bench_two_y - 0.58,
        "mine_bench_two",
    );

    let bottom_mesh = meshes.add(
        Cylinder::new(MINE_BOTTOM_RADIUS, 0.32)
            .mesh()
            .resolution(RING_SEGMENTS as u32),
    );
    commands.spawn((
        Mesh3d(bottom_mesh),
        MeshMaterial3d(bottom_material),
        Transform::from_xyz(0.0, bottom_y - 0.16, 0.0),
        Visibility::Hidden,
        CaveRealmVisual,
        Name::new("mine_pit_bottom"),
    ));
    spawn_collider(
        &mut commands,
        Collider::cylinder(MINE_BOTTOM_RADIUS, 0.32),
        Vec3::new(0.0, bottom_y - 0.16, 0.0),
        Quat::IDENTITY,
        "mine_pit_bottom_collider",
    );

    spawn_ring_colliders(
        &mut commands,
        MINE_BENCH_ONE_OUTER,
        MINE_OUTER_RADIUS,
        rim_y,
        "mine_rim_collider",
    );
    spawn_ring_colliders(
        &mut commands,
        MINE_BENCH_TWO_OUTER,
        MINE_BENCH_ONE_OUTER,
        bench_one_y,
        "mine_bench_one_collider",
    );
    spawn_ring_colliders(
        &mut commands,
        MINE_BOTTOM_RADIUS,
        MINE_BENCH_TWO_OUTER,
        bench_two_y,
        "mine_bench_two_collider",
    );

    let stair_mesh = meshes.add(Cuboid::new(2.15, 0.32, 0.82));
    let stair_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.25, 0.16),
        perceptual_roughness: 0.95,
        ..default()
    });
    // A broad, readable stair cut through the south wall is the player route
    // from the entrance rim to the working floor.
    for index in 0..8 {
        let depth = index as f32 * 0.31;
        let y = rim_y - 0.16 - depth;
        let z = -7.45 + index as f32 * 0.78;
        commands.spawn((
            Mesh3d(stair_mesh.clone()),
            MeshMaterial3d(stair_material.clone()),
            Transform::from_xyz(0.0, y, z),
            Visibility::Hidden,
            CaveRealmVisual,
            Name::new(format!("mine_descent_step_{index}")),
        ));
        spawn_collider(
            &mut commands,
            Collider::cuboid(2.15, 0.32, 0.82),
            Vec3::new(0.0, y, z),
            Quat::IDENTITY,
            &format!("mine_descent_step_{index}_collider"),
        );
    }

    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.13, 0.055),
        perceptual_roughness: 0.88,
        ..default()
    });
    let wood_light = materials.add(StandardMaterial {
        base_color: Color::srgb(0.56, 0.27, 0.09),
        perceptual_roughness: 0.86,
        ..default()
    });
    let shaft_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.018, 0.012, 0.014),
        perceptual_roughness: 1.0,
        ..default()
    });
    let post_mesh = meshes.add(Cuboid::new(0.38, 3.8, 0.38));
    let beam_mesh = meshes.add(Cuboid::new(5.8, 0.42, 0.42));
    let shaft_mesh = meshes.add(Cuboid::new(5.0, 2.9, 0.22));
    // The rear tunnel makes the excavation read as a working mine, not a
    // decorative hole. The support frame sits directly on the second bench.
    for x in [-2.55, 2.55] {
        commands.spawn((
            Mesh3d(post_mesh.clone()),
            MeshMaterial3d(wood.clone()),
            Transform::from_xyz(x, bench_two_y + 1.45, 7.75),
            Visibility::Hidden,
            CaveRealmVisual,
            Name::new("mine_shaft_support_post"),
        ));
    }
    commands.spawn((
        Mesh3d(beam_mesh.clone()),
        MeshMaterial3d(wood_light.clone()),
        Transform::from_xyz(0.0, bench_two_y + 3.2, 7.75),
        Visibility::Hidden,
        CaveRealmVisual,
        Name::new("mine_shaft_support_beam"),
    ));
    commands.spawn((
        Mesh3d(shaft_mesh),
        MeshMaterial3d(shaft_dark),
        Transform::from_xyz(0.0, bench_two_y + 1.45, 7.88),
        Visibility::Hidden,
        CaveRealmVisual,
        Name::new("mine_shaft_mouth"),
    ));

    let rail_mesh = meshes.add(Cuboid::new(0.13, 0.09, 6.2));
    let rail_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.21, 0.24, 0.25),
        metallic: 0.65,
        perceptual_roughness: 0.42,
        ..default()
    });
    for x in [-0.82, 0.82] {
        commands.spawn((
            Mesh3d(rail_mesh.clone()),
            MeshMaterial3d(rail_material.clone()),
            Transform::from_xyz(x, bottom_y + 0.10, 1.10),
            Visibility::Hidden,
            CaveRealmVisual,
            Name::new("mine_rail"),
        ));
    }
    let cart_body = meshes.add(Cuboid::new(1.9, 0.62, 1.25));
    commands.spawn((
        Mesh3d(cart_body),
        MeshMaterial3d(wood_light.clone()),
        Transform::from_xyz(0.0, bottom_y + 0.62, -1.10),
        Visibility::Hidden,
        CaveRealmVisual,
        Name::new("mine_cart"),
    ));
    let cart_wheel = meshes.add(Cylinder::new(0.28, 0.16).mesh().resolution(12));
    for x in [-0.72, 0.72] {
        commands.spawn((
            Mesh3d(cart_wheel.clone()),
            MeshMaterial3d(rail_material.clone()),
            Transform::from_xyz(x, bottom_y + 0.28, -1.10)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            Visibility::Hidden,
            CaveRealmVisual,
            Name::new("mine_cart_wheel"),
        ));
    }

    let ore_mesh = meshes.add(Cone::new(0.22, 0.92).mesh().resolution(6));
    let ore_blue = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.46, 0.92),
        emissive: Color::srgb(0.04, 0.30, 0.95).into(),
        metallic: 0.15,
        perceptual_roughness: 0.30,
        ..default()
    });
    let ore_gold = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.48, 0.08),
        emissive: Color::srgb(0.80, 0.14, 0.015).into(),
        metallic: 0.30,
        perceptual_roughness: 0.28,
        ..default()
    });
    let ore_layout: [(f32, f32, f32, f32, bool); 6] = [
        (
            MINE_BENCH_TWO_INNER + 0.12,
            0.35,
            bench_two_y + 0.35,
            1.0,
            false,
        ),
        (
            MINE_BENCH_TWO_INNER + 0.12,
            2.15,
            bench_two_y + 0.48,
            0.72,
            true,
        ),
        (
            MINE_BENCH_ONE_INNER + 0.12,
            3.75,
            bench_one_y + 0.42,
            0.88,
            false,
        ),
        (
            MINE_BENCH_ONE_INNER + 0.12,
            5.05,
            bench_one_y + 0.28,
            0.64,
            true,
        ),
        (
            MINE_BENCH_ONE_INNER + 0.12,
            5.75,
            bench_one_y + 0.58,
            0.74,
            false,
        ),
        (
            MINE_BENCH_TWO_INNER + 0.12,
            4.90,
            bench_two_y + 0.30,
            0.60,
            true,
        ),
    ];
    for (radius, angle, y, scale, gold) in ore_layout {
        let position = Vec3::new(radius * angle.cos(), y, radius * angle.sin());
        commands.spawn((
            Mesh3d(ore_mesh.clone()),
            MeshMaterial3d(if gold {
                ore_gold.clone()
            } else {
                ore_blue.clone()
            }),
            Transform::from_translation(position)
                .with_scale(Vec3::new(scale, scale, scale))
                .with_rotation(Quat::from_rotation_y(angle + 0.6)),
            Visibility::Hidden,
            CaveOre {
                resource: if gold {
                    ResourceKind::Sunstone
                } else {
                    ResourceKind::Stone
                },
                amount: if gold { 1 } else { 2 },
            },
            CaveRealmVisual,
            Name::new("mine_ore_vein"),
        ));
    }

    let rock_mesh = meshes.add(
        Sphere::new(0.72)
            .mesh()
            .ico(1)
            .expect("mine rock mesh should be valid"),
    );
    let rock_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.15, 0.14),
        perceptual_roughness: 0.99,
        ..default()
    });
    let rock_layout: [(f32, f32, Vec3); 6] = [
        (0.15, 9.7, Vec3::new(1.4, 0.9, 1.0)),
        (0.78, 9.55, Vec3::new(0.9, 1.6, 1.1)),
        (2.30, 9.8, Vec3::new(1.2, 1.0, 0.85)),
        (3.10, 9.65, Vec3::new(1.0, 1.4, 1.2)),
        (4.25, 9.75, Vec3::new(1.35, 0.85, 1.0)),
        (5.30, 9.6, Vec3::new(0.85, 1.35, 1.0)),
    ];
    for (index, (angle, radius, scale)) in rock_layout.into_iter().enumerate() {
        commands.spawn((
            Mesh3d(rock_mesh.clone()),
            MeshMaterial3d(rock_material.clone()),
            Transform::from_xyz(
                radius * angle.cos(),
                rim_y + scale.y * 0.35,
                radius * angle.sin(),
            )
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_y(angle * 1.7)),
            Visibility::Hidden,
            CaveRealmVisual,
            Name::new(format!("mine_rim_rock_{index}")),
        ));
    }

    let exit = commands
        .spawn((
            Transform::from_xyz(0.0, rim_y, -9.35),
            Visibility::Hidden,
            CaveRealmVisual,
            CaveExit,
            Name::new("mine_entrance_exit"),
        ))
        .id();
    commands.entity(exit).with_children(|parent| {
        parent.spawn((
            Mesh3d(post_mesh.clone()),
            MeshMaterial3d(wood.clone()),
            Transform::from_xyz(-1.8, 1.25, 0.0),
        ));
        parent.spawn((
            Mesh3d(post_mesh),
            MeshMaterial3d(wood.clone()),
            Transform::from_xyz(1.8, 1.25, 0.0),
        ));
        parent.spawn((
            Mesh3d(beam_mesh),
            MeshMaterial3d(wood_light),
            Transform::from_xyz(0.0, 3.1, 0.0),
        ));
    });

    let warm_light = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.38, 0.08),
        emissive: Color::srgb(1.0, 0.18, 0.015).into(),
        unlit: true,
        ..default()
    });
    let lamp_mesh = meshes.add(Sphere::new(0.18).mesh().ico(1).expect("mine lamp mesh"));
    for (index, position) in [
        Vec3::new(-2.8, bench_two_y + 2.15, 6.95),
        Vec3::new(2.8, bench_two_y + 2.15, 6.95),
        Vec3::new(-4.8, bench_one_y + 1.35, -1.0),
    ]
    .into_iter()
    .enumerate()
    {
        commands.spawn((
            Mesh3d(lamp_mesh.clone()),
            MeshMaterial3d(warm_light.clone()),
            Transform::from_translation(position),
            Visibility::Hidden,
            CaveRealmVisual,
            Name::new(format!("mine_lamp_{index}")),
        ));
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.30, 0.08),
                intensity: 1_400.0,
                range: 8.0,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_translation(position),
            Visibility::Hidden,
            CaveRealmVisual,
            Name::new(format!("mine_lamp_light_{index}")),
        ));
    }
    commands.spawn((
        PointLight {
            color: Color::srgb(0.12, 0.34, 1.0),
            intensity: 2_800.0,
            range: 18.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, bottom_y + 4.0, 0.0),
        Visibility::Hidden,
        CaveRealmVisual,
        Name::new("mine_ore_fill_light"),
    ));

    // Keep the cave's low boundary explicit. The forest terrain collider is
    // disabled while travelling, and these are the only walkable mine levels.
    let boundary_mesh = meshes.add(
        Cylinder::new(MINE_OUTER_RADIUS + 0.5, 0.25)
            .mesh()
            .resolution(32),
    );
    commands.spawn((
        Mesh3d(boundary_mesh),
        MeshMaterial3d(wall_material),
        Transform::from_xyz(0.0, rim_y - 3.4, 0.0),
        Visibility::Hidden,
        CaveRealmVisual,
        Name::new("mine_depth_shadow"),
    ));
}

/// Start directly at the mine rim for a target-camera screenshot. This is a
/// validation-only switch; normal gameplay still reaches the mine through the
/// cave entrance and the interaction system.
pub fn setup_mine_preview(
    terrain: Res<super::state::ProceduralTerrainSurface>,
    mut travel: ResMut<CaveTravelState>,
    mut camera_rig: ResMut<LivingCameraRig>,
    mut players: Query<&mut Transform, With<PlayerActor>>,
    mut cave_visuals: Query<&mut Visibility, With<CaveRealmVisual>>,
) {
    if std::env::var_os("LK2_MINE_PREVIEW").is_none() {
        return;
    }
    let rim_y = terrain.ground_height(Vec3::ZERO) + 0.06;
    for mut player in &mut players {
        player.translation = Vec3::new(
            0.0,
            rim_y + super::util::PLAYER_PHYSICS_CENTER_HEIGHT + 0.22,
            -8.25,
        );
    }
    for mut visibility in &mut cave_visuals {
        *visibility = Visibility::Visible;
    }
    travel.active = true;
    camera_rig.yaw = 0.0;
    camera_rig.pitch = -0.58;
    info!("[cave] mine preview enabled at the open-pit rim");
}

pub fn mine_cave_ore(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    travel: Res<CaveTravelState>,
    mut nature: ResMut<OfflineNature>,
    mut commands: Commands,
    players: Query<&Transform, With<PlayerActor>>,
    ores: Query<(Entity, &Transform, &CaveOre)>,
) {
    if !travel.active
        || bindings.menu_open
        || !bindings.just_pressed(GameAction::Mine, &keys, &mouse)
    {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let Some((entity, _, ore)) = ores
        .iter()
        .filter(|(_, transform, _)| {
            transform.translation.distance_squared(player.translation) <= 2.4_f32.powi(2)
        })
        .min_by(|left, right| {
            left.1
                .translation
                .distance_squared(player.translation)
                .total_cmp(&right.1.translation.distance_squared(player.translation))
        })
    else {
        return;
    };

    let amount = nature.resources.force_add(ore.resource, ore.amount);
    commands.entity(entity).despawn();
    info!(
        "[cave] mined {:?} x{} (total {})",
        ore.resource, ore.amount, amount
    );
}

fn spawn_ring(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    inner_radius: f32,
    outer_radius: f32,
    top_y: f32,
    bottom_y: f32,
    name: &str,
) {
    let mesh = meshes.add(ring_mesh(inner_radius, outer_radius, top_y, bottom_y));
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material.clone()),
        Transform::IDENTITY,
        Visibility::Hidden,
        CaveRealmVisual,
        Name::new(name.to_string()),
    ));
}

fn ring_mesh(inner_radius: f32, outer_radius: f32, top_y: f32, bottom_y: f32) -> Mesh {
    let mut positions = Vec::with_capacity(RING_SEGMENTS * 4);
    let mut normals = Vec::with_capacity(RING_SEGMENTS * 4);
    let mut uvs = Vec::with_capacity(RING_SEGMENTS * 4);
    let mut indices = Vec::with_capacity(RING_SEGMENTS * 18);

    for index in 0..RING_SEGMENTS {
        let angle = index as f32 / RING_SEGMENTS as f32 * TAU;
        let base = positions.len() as u32;
        for (radius, y, normal) in [
            (outer_radius, top_y, [0.0, 1.0, 0.0]),
            (inner_radius, top_y, [0.0, 1.0, 0.0]),
            (inner_radius, bottom_y, [-angle.cos(), 0.0, -angle.sin()]),
            (outer_radius, bottom_y, [angle.cos(), 0.0, angle.sin()]),
        ] {
            positions.push([radius * angle.cos(), y, radius * angle.sin()]);
            normals.push(normal);
            uvs.push([radius * angle.cos() * 0.08, radius * angle.sin() * 0.08]);
        }
        let next_base = (base + 4) % (RING_SEGMENTS as u32 * 4);
        indices.extend_from_slice(&[
            base,
            base + 1,
            next_base + 1,
            base,
            next_base + 1,
            next_base,
            base + 1,
            next_base + 1,
            next_base + 2,
            base + 1,
            next_base + 2,
            base + 2,
            base + 3,
            next_base + 3,
            next_base,
            base + 3,
            next_base,
            base,
        ]);
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

fn spawn_ring_colliders(
    commands: &mut Commands,
    inner_radius: f32,
    outer_radius: f32,
    top_y: f32,
    name: &str,
) {
    let radial_width = outer_radius - inner_radius + 0.16;
    let tangential_width = (TAU * outer_radius / RING_SEGMENTS as f32) * 1.12;
    for index in 0..RING_SEGMENTS {
        let angle = index as f32 / RING_SEGMENTS as f32 * TAU;
        let position = Vec3::new(
            angle.cos() * (inner_radius + outer_radius) * 0.5,
            top_y - 0.16,
            angle.sin() * (inner_radius + outer_radius) * 0.5,
        );
        spawn_collider(
            commands,
            Collider::cuboid(radial_width, 0.32, tangential_width),
            position,
            Quat::from_rotation_y(-angle),
            &format!("{name}_{index}"),
        );
    }
}

fn spawn_collider(
    commands: &mut Commands,
    collider: Collider,
    position: Vec3,
    rotation: Quat,
    name: &str,
) {
    commands.spawn((
        RigidBody::Static,
        collider,
        ColliderDisabled,
        Transform::from_translation(position).with_rotation(rotation),
        CaveRealmCollider,
        Name::new(name.to_string()),
    ));
}

pub fn update_cave_presentation(
    travel: Res<CaveTravelState>,
    dimension: Res<DimensionTravelState>,
    mut clear_color: ResMut<ClearColor>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut forest_visuals: Query<&mut Visibility, (With<ForestRealmVisual>, Without<CaveRealmVisual>)>,
    mut cave_visuals: Query<&mut Visibility, (With<CaveRealmVisual>, Without<ForestRealmVisual>)>,
    forest_colliders: Query<(Entity, Option<&ColliderDisabled>), With<ForestRealmCollider>>,
    cave_colliders: Query<(Entity, Option<&ColliderDisabled>), With<CaveRealmCollider>>,
    mut commands: Commands,
) {
    if dimension.current == DimensionId::Starfall {
        return;
    }
    for mut visibility in &mut forest_visuals {
        *visibility = if travel.active {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    for mut visibility in &mut cave_visuals {
        *visibility = if travel.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if travel.active {
        clear_color.0 = Color::srgb(0.012, 0.008, 0.012);
        ambient.color = Color::srgb(0.24, 0.16, 0.12);
        ambient.brightness = 105.0;
    } else {
        clear_color.0 = Color::srgb(0.47, 0.61, 0.72);
        ambient.color = Color::srgb(0.78, 0.86, 0.82);
        ambient.brightness = 190.0;
    }
    toggle_colliders(&mut commands, forest_colliders, travel.active);
    toggle_colliders(&mut commands, cave_colliders, !travel.active);
}

fn toggle_colliders<'a>(
    commands: &mut Commands,
    colliders: impl IntoIterator<Item = (Entity, Option<&'a ColliderDisabled>)>,
    disable: bool,
) {
    for (entity, disabled) in colliders {
        if disable {
            if disabled.is_none() {
                commands.entity(entity).insert(ColliderDisabled);
            }
        } else if disabled.is_some() {
            commands.entity(entity).remove::<ColliderDisabled>();
        }
    }
}
