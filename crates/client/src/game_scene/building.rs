//! Exploreable building interiors for the living scene.

use avian3d::prelude::{Collider, ColliderDisabled, LinearVelocity, RigidBody};
use bevy::prelude::*;

use super::keybindings::{GameAction, KeyBindings};
use super::state::{
    BuildingExplorationState, BuildingInteriorRoot, BuildingPrompt, ExplorableBuilding, PlayerActor,
};
use super::util::PLAYER_PHYSICS_CENTER_HEIGHT;

const BUILDING_INTERACT_RANGE: f32 = 3.2;
const BUILDING_DOOR_LOCAL_Z: f32 = 0.58;

fn is_at_building_door(player: Vec3, building: &Transform) -> bool {
    let local =
        building.rotation.inverse() * (player - building.translation) / building.scale.x.max(0.001);
    player.distance(building.translation) <= BUILDING_INTERACT_RANGE
        && local.z >= BUILDING_DOOR_LOCAL_Z
        && local.z <= 2.0
        && local.x.abs() <= 0.65
}

pub fn setup_explorable_buildings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    buildings: Query<(Entity, &Transform), With<ExplorableBuilding>>,
) {
    let floor_mesh = meshes.add(Cuboid::new(2.9, 0.12, 2.5));
    let wall_mesh = meshes.add(Cuboid::new(0.12, 2.45, 2.5));
    let back_wall_mesh = meshes.add(Cuboid::new(2.9, 2.45, 0.12));
    let front_wall_mesh = meshes.add(Cuboid::new(1.1, 2.45, 0.12));
    let ceiling_mesh = meshes.add(Cuboid::new(2.9, 0.10, 2.5));
    let trim_mesh = meshes.add(Cuboid::new(0.10, 2.25, 0.10));
    let table_mesh = meshes.add(Cuboid::new(0.95, 0.12, 0.55));
    let table_leg_mesh = meshes.add(Cuboid::new(0.10, 0.62, 0.10));
    let bed_mesh = meshes.add(Cuboid::new(0.85, 0.35, 1.55));
    let chest_mesh = meshes.add(Cuboid::new(0.65, 0.45, 0.45));
    let rug_mesh = meshes.add(Cuboid::new(1.35, 0.025, 0.95));
    let lamp_mesh = meshes.add(Cuboid::new(0.16, 0.16, 0.16));

    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.28, 0.15),
        perceptual_roughness: 0.92,
        ..default()
    });
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.70, 0.47),
        perceptual_roughness: 0.90,
        ..default()
    });
    let ceiling_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.43, 0.25, 0.14),
        perceptual_roughness: 0.94,
        ..default()
    });
    let trim_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.14, 0.07),
        perceptual_roughness: 0.92,
        ..default()
    });
    let furniture_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.26, 0.42, 0.30),
        perceptual_roughness: 0.88,
        ..default()
    });
    let bed_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.70, 0.32, 0.32),
        perceptual_roughness: 0.86,
        ..default()
    });
    let rug_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.48, 0.18),
        perceptual_roughness: 0.96,
        ..default()
    });
    let lamp_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.65, 0.22),
        emissive: Color::srgb(1.0, 0.28, 0.04).into(),
        ..default()
    });

    for (exterior, transform) in &buildings {
        let interior = commands
            .spawn((
                Transform::from_translation(transform.translation)
                    .with_rotation(transform.rotation),
                Visibility::Hidden,
                BuildingInteriorRoot,
                Name::new("explorable_building_interior"),
            ))
            .id();

        commands.entity(exterior).insert(ExplorableBuilding {
            interior: Some(interior),
        });

        commands.entity(interior).with_children(|room| {
            spawn_room_piece(
                room,
                floor_mesh.clone(),
                floor_material.clone(),
                Vec3::new(0.0, 0.06, 0.0),
                Collider::cuboid(2.9, 0.12, 2.5),
                "interior_floor",
            );
            spawn_room_piece(
                room,
                wall_mesh.clone(),
                wall_material.clone(),
                Vec3::new(-1.4, 1.225, 0.0),
                Collider::cuboid(0.12, 2.45, 2.5),
                "interior_wall_left",
            );
            spawn_room_piece(
                room,
                wall_mesh.clone(),
                wall_material.clone(),
                Vec3::new(1.4, 1.225, 0.0),
                Collider::cuboid(0.12, 2.45, 2.5),
                "interior_wall_right",
            );
            spawn_room_piece(
                room,
                back_wall_mesh.clone(),
                wall_material.clone(),
                Vec3::new(0.0, 1.225, -1.2),
                Collider::cuboid(2.9, 2.45, 0.12),
                "interior_wall_back",
            );
            for (x, name) in [
                (-0.85, "interior_wall_front_left"),
                (0.85, "interior_wall_front_right"),
            ] {
                spawn_room_piece(
                    room,
                    front_wall_mesh.clone(),
                    wall_material.clone(),
                    Vec3::new(x, 1.225, 1.2),
                    Collider::cuboid(1.1, 2.45, 0.12),
                    name,
                );
            }
            spawn_room_piece(
                room,
                ceiling_mesh.clone(),
                ceiling_material.clone(),
                Vec3::new(0.0, 2.45, 0.0),
                Collider::cuboid(2.9, 0.10, 2.5),
                "interior_ceiling",
            );

            room.spawn((
                Mesh3d(rug_mesh.clone()),
                MeshMaterial3d(rug_material.clone()),
                Transform::from_xyz(0.0, 0.135, -0.10),
                Name::new("interior_rug"),
            ));
            spawn_room_piece(
                room,
                bed_mesh.clone(),
                bed_material.clone(),
                Vec3::new(-0.78, 0.30, -0.38),
                Collider::cuboid(0.85, 0.35, 1.55),
                "interior_bed",
            );
            spawn_room_piece(
                room,
                table_mesh.clone(),
                furniture_material.clone(),
                Vec3::new(0.70, 0.86, -0.35),
                Collider::cuboid(0.95, 0.12, 0.55),
                "interior_table_top",
            );
            for (x, z) in [(0.35, -0.55), (1.05, -0.55), (0.35, -0.15), (1.05, -0.15)] {
                spawn_room_piece(
                    room,
                    table_leg_mesh.clone(),
                    trim_material.clone(),
                    Vec3::new(x, 0.49, z),
                    Collider::cuboid(0.10, 0.62, 0.10),
                    "interior_table_leg",
                );
            }
            spawn_room_piece(
                room,
                chest_mesh.clone(),
                furniture_material.clone(),
                Vec3::new(0.80, 0.225, 0.72),
                Collider::cuboid(0.65, 0.45, 0.45),
                "interior_chest",
            );
            room.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.68, 0.36),
                    intensity: 850.0,
                    range: 7.0,
                    shadow_maps_enabled: true,
                    ..default()
                },
                Transform::from_xyz(0.0, 2.0, 0.0),
                Name::new("interior_lamp"),
            ));
            room.spawn((
                Mesh3d(lamp_mesh.clone()),
                MeshMaterial3d(lamp_material.clone()),
                Transform::from_xyz(0.0, 1.95, 0.0),
                Name::new("interior_lamp_glow"),
            ));
            for x in [-1.25, 1.25] {
                room.spawn((
                    Mesh3d(trim_mesh.clone()),
                    MeshMaterial3d(trim_material.clone()),
                    Transform::from_xyz(x, 1.15, 1.12),
                    Name::new("interior_door_trim"),
                ));
            }
        });
    }
}

fn spawn_room_piece(
    room: &mut ChildSpawnerCommands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    translation: Vec3,
    collider: Collider,
    name: &'static str,
) {
    room.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(translation),
        RigidBody::Static,
        ColliderDisabled,
        collider,
        Name::new(name),
    ));
}

pub fn explore_buildings(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    mut commands: Commands,
    mut state: ResMut<BuildingExplorationState>,
    mut players: Query<(&mut Transform, &mut LinearVelocity), With<PlayerActor>>,
    buildings: Query<(Entity, &Transform, &ExplorableBuilding), Without<PlayerActor>>,
    children: Query<&Children>,
) {
    if bindings.menu_open || !bindings.just_pressed(GameAction::Interact, &keys, &mouse) {
        return;
    }
    let Ok((mut player, mut velocity)) = players.single_mut() else {
        return;
    };

    if let Some(exterior) = state.active {
        let Ok((_, building, data)) = buildings.get(exterior) else {
            state.active = None;
            return;
        };
        if let Some(interior) = data.interior {
            commands.entity(interior).insert(Visibility::Hidden);
            disable_interior_colliders(interior, &children, &mut commands);
        }
        commands.entity(exterior).insert(Visibility::Visible);
        player.translation =
            building.transform_point(Vec3::new(0.0, PLAYER_PHYSICS_CENTER_HEIGHT, 1.75));
        velocity.0 = Vec3::ZERO;
        state.active = None;
        state.prompt = None;
        return;
    }

    let Some((exterior, building, data)) = buildings
        .iter()
        .filter_map(|(entity, transform, data)| {
            is_at_building_door(player.translation, transform).then_some((entity, transform, data))
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

    let Some(interior) = data.interior else {
        return;
    };
    commands.entity(exterior).insert(Visibility::Visible);
    // The rebuilt GLB contains the room art. The generated interior remains a
    // hidden collision fallback so gameplay does not depend on asset children.
    commands.entity(interior).insert(Visibility::Hidden);
    enable_interior_colliders(interior, &children, &mut commands);
    player.translation =
        building.transform_point(Vec3::new(0.0, PLAYER_PHYSICS_CENTER_HEIGHT, 0.25));
    player.rotation = building.rotation;
    velocity.0 = Vec3::ZERO;
    state.active = Some(exterior);
    state.prompt = Some(BuildingPrompt::Exit);
}

fn disable_interior_colliders(
    interior: Entity,
    children: &Query<&Children>,
    commands: &mut Commands,
) {
    let Ok(children) = children.get(interior) else {
        return;
    };
    for child in children.iter() {
        commands.entity(child).insert(ColliderDisabled);
    }
}

fn enable_interior_colliders(
    interior: Entity,
    children: &Query<&Children>,
    commands: &mut Commands,
) {
    let Ok(children) = children.get(interior) else {
        return;
    };
    for child in children.iter() {
        commands.entity(child).remove::<ColliderDisabled>();
    }
}

pub fn update_building_prompt(
    mut state: ResMut<BuildingExplorationState>,
    players: Query<&Transform, With<PlayerActor>>,
    buildings: Query<&Transform, With<ExplorableBuilding>>,
) {
    if state.active.is_some() {
        state.prompt = Some(BuildingPrompt::Exit);
        return;
    }
    let Ok(player) = players.single() else {
        state.prompt = None;
        return;
    };
    state.prompt = buildings
        .iter()
        .any(|building| is_at_building_door(player.translation, building))
        .then_some(BuildingPrompt::Enter);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn building_door_prompt_only_accepts_the_front_door() {
        let building = Transform::default();
        assert!(is_at_building_door(Vec3::new(0.0, 0.88, 1.2), &building));
        assert!(!is_at_building_door(Vec3::new(0.0, 0.88, -1.2), &building));
        assert!(!is_at_building_door(Vec3::new(0.9, 0.88, 1.2), &building));
        assert!(!is_at_building_door(Vec3::new(0.0, 0.88, 4.0), &building));
    }

    #[test]
    fn interaction_toggles_between_exterior_and_interior() {
        let mut app = App::new();
        app.insert_resource(KeyBindings::default())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(BuildingExplorationState::default())
            .add_systems(Update, explore_buildings);

        let interior = app
            .world_mut()
            .spawn((Visibility::Hidden, Name::new("interior")))
            .id();
        let exterior = app
            .world_mut()
            .spawn((
                Transform::default(),
                Visibility::Visible,
                ExplorableBuilding {
                    interior: Some(interior),
                },
                Name::new("exterior"),
            ))
            .id();
        app.world_mut().spawn((
            PlayerActor,
            Transform::from_xyz(0.0, PLAYER_PHYSICS_CENTER_HEIGHT, 1.2),
            LinearVelocity::default(),
        ));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update();
        assert_eq!(
            app.world().resource::<BuildingExplorationState>().active,
            Some(exterior)
        );
        assert_eq!(
            app.world().get::<Visibility>(exterior),
            Some(&Visibility::Visible)
        );
        assert_eq!(
            app.world().get::<Visibility>(interior),
            Some(&Visibility::Hidden)
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyF);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update();
        assert!(
            app.world()
                .resource::<BuildingExplorationState>()
                .active
                .is_none()
        );
        assert_eq!(
            app.world().get::<Visibility>(exterior),
            Some(&Visibility::Visible)
        );
        assert_eq!(
            app.world().get::<Visibility>(interior),
            Some(&Visibility::Hidden)
        );
    }
}
