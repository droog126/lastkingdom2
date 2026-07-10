//! Bevy presentation driven exclusively by authoritative natural-world snapshots.

pub mod animals;
pub mod plants;
pub mod sky;
pub mod weather;

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use lk2_core::protocol::components::EcoSnapshot;

use self::animals::{rabbit_scale, wildlife_scale};
use self::plants::{berry_bush_scale, plant_scale};
use self::sky::SkyPresentation;
use self::weather::WeatherPresentation;
use crate::synchronization::NatureSnapshotBuffer;

const RAIN_DROPS_PER_CLOUD: u8 = 5;
const MAX_VISIBLE_FRUIT_PER_BUSH: u8 = 3;

pub struct NaturePresentationPlugin;

impl Plugin for NaturePresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NatureSnapshotBuffer>()
            .init_resource::<NaturePresentationCursor>()
            .init_resource::<SkyPresentation>()
            .init_resource::<WeatherPresentation>()
            .add_systems(Startup, setup_nature_visual_assets)
            .add_systems(
                Update,
                (reconcile_nature_visuals, animate_nature_visuals).chain(),
            );
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NatureVisualKey {
    Cloud(u32),
    RainDrop { cloud_id: u32, index: u8 },
    Plant(u32),
    BerryBush(u32),
    BerryFruit { bush_id: u32, index: u8 },
    Rabbit(u32),
    Wildlife(u32),
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct NatureVisualState {
    pub position: Vec2,
    pub scale: Vec3,
    pub intensity: f32,
    pub phase: f32,
    pub kind: u8,
}

#[derive(Resource, Default)]
struct NaturePresentationCursor {
    last_applied_tick: Option<u64>,
}

#[derive(Resource)]
struct NatureVisualAssets {
    cloud_mesh: Handle<Mesh>,
    rain_mesh: Handle<Mesh>,
    plant_mesh: Handle<Mesh>,
    fruit_mesh: Handle<Mesh>,
    animal_mesh: Handle<Mesh>,
    cloud_material: Handle<StandardMaterial>,
    rain_material: Handle<StandardMaterial>,
    plant_material: Handle<StandardMaterial>,
    berry_material: Handle<StandardMaterial>,
    fruit_material: Handle<StandardMaterial>,
    rabbit_material: Handle<StandardMaterial>,
    wildlife_material: Handle<StandardMaterial>,
}

fn setup_nature_visual_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cloud_mesh = meshes.add(Sphere::new(1.0));
    let rain_mesh = meshes.add(Cuboid::new(0.035, 0.68, 0.035));
    let plant_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let fruit_mesh = meshes.add(Sphere::new(0.12));
    let animal_mesh = meshes.add(Sphere::new(0.72));

    commands.insert_resource(NatureVisualAssets {
        cloud_mesh,
        rain_mesh,
        plant_mesh,
        fruit_mesh,
        animal_mesh,
        cloud_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.91, 0.95, 1.0, 0.86),
            perceptual_roughness: 0.94,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        rain_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.28, 0.58, 0.96, 0.78),
            emissive: Color::srgb(0.08, 0.20, 0.52).into(),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        plant_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.28, 0.72, 0.30),
            perceptual_roughness: 0.92,
            ..default()
        }),
        berry_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.20, 0.58, 0.24),
            perceptual_roughness: 0.94,
            ..default()
        }),
        fruit_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.91, 0.16, 0.30),
            perceptual_roughness: 0.70,
            ..default()
        }),
        rabbit_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.88, 0.76, 0.62),
            perceptual_roughness: 0.90,
            ..default()
        }),
        wildlife_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.64, 0.42, 0.25),
            perceptual_roughness: 0.88,
            ..default()
        }),
    });
}

fn reconcile_nature_visuals(
    mut commands: Commands,
    buffer: Res<NatureSnapshotBuffer>,
    assets: Res<NatureVisualAssets>,
    mut cursor: ResMut<NaturePresentationCursor>,
    mut sky: ResMut<SkyPresentation>,
    mut weather: ResMut<WeatherPresentation>,
    existing: Query<(Entity, &NatureVisualKey)>,
) {
    let Some(snapshot) = buffer.latest() else {
        return;
    };
    if cursor.last_applied_tick == Some(snapshot.tick) {
        return;
    }

    let desired = desired_visuals(snapshot);
    let existing_by_key =
        existing.iter().map(|(entity, key)| (*key, entity)).collect::<HashMap<_, _>>();
    let desired_keys = desired.iter().map(|(key, _)| *key).collect::<HashSet<_>>();

    for (key, state) in desired {
        let transform = visual_transform(key, state, 0.0);
        if let Some(entity) = existing_by_key.get(&key) {
            commands.entity(*entity).insert((state, transform));
        } else {
            let (mesh, material) = visual_assets_for(key, &assets);
            commands.spawn((
                Name::new(format!("nature::{key:?}")),
                key,
                state,
                Mesh3d(mesh),
                MeshMaterial3d(material),
                transform,
            ));
        }
    }

    for (key, entity) in existing_by_key {
        if !desired_keys.contains(&key) {
            commands.entity(entity).despawn();
        }
    }

    sky.apply(snapshot);
    weather.apply(snapshot);
    cursor.last_applied_tick = Some(snapshot.tick);
}

fn animate_nature_visuals(
    time: Res<Time>,
    mut visuals: Query<(&NatureVisualKey, &NatureVisualState, &mut Transform)>,
) {
    let elapsed = time.elapsed_secs();
    for (key, state, mut transform) in &mut visuals {
        *transform = visual_transform(*key, *state, elapsed);
    }
}

#[must_use]
pub fn desired_visuals(snapshot: &EcoSnapshot) -> Vec<(NatureVisualKey, NatureVisualState)> {
    let mut visuals = Vec::new();

    for cloud in &snapshot.clouds {
        let cloud_state = NatureVisualState {
            position: Vec2::new(cloud.x, cloud.z),
            scale: Vec3::new(1.75, 0.58, 1.18) * (1.0 + cloud.rain.clamp(0.0, 1.0) * 0.22),
            intensity: cloud.rain.max(0.0),
            phase: cloud.phase,
            kind: 0,
        };
        visuals.push((NatureVisualKey::Cloud(cloud.id), cloud_state));
        if cloud.rain > 0.05 {
            for index in 0..RAIN_DROPS_PER_CLOUD {
                visuals.push((
                    NatureVisualKey::RainDrop { cloud_id: cloud.id, index },
                    NatureVisualState { scale: Vec3::ONE, kind: index, ..cloud_state },
                ));
            }
        }
    }

    for plant in &snapshot.plants {
        visuals.push((
            NatureVisualKey::Plant(plant.id),
            NatureVisualState {
                position: Vec2::new(plant.x, plant.z),
                scale: plant_scale(plant.kind, plant.stock),
                intensity: plant.stock as f32,
                phase: plant.id as f32 * 0.71,
                kind: plant.kind,
            },
        ));
    }

    for berry in &snapshot.berries {
        let bush_state = NatureVisualState {
            position: Vec2::new(berry.x, berry.z),
            scale: berry_bush_scale(berry.fruit),
            intensity: berry.fruit as f32,
            phase: berry.id as f32 * 0.53,
            kind: 0,
        };
        visuals.push((NatureVisualKey::BerryBush(berry.id), bush_state));
        for index in 0..berry.fruit.min(MAX_VISIBLE_FRUIT_PER_BUSH as u32) as u8 {
            visuals.push((
                NatureVisualKey::BerryFruit { bush_id: berry.id, index },
                NatureVisualState { scale: Vec3::ONE, kind: index, ..bush_state },
            ));
        }
    }

    for rabbit in &snapshot.rabbits {
        visuals.push((
            NatureVisualKey::Rabbit(rabbit.id),
            NatureVisualState {
                position: Vec2::new(rabbit.x, rabbit.z),
                scale: rabbit_scale(rabbit.energy),
                intensity: rabbit.energy,
                phase: rabbit.id as f32 * 0.91,
                kind: 0,
            },
        ));
    }

    for animal in &snapshot.wildlife {
        visuals.push((
            NatureVisualKey::Wildlife(animal.id),
            NatureVisualState {
                position: Vec2::new(animal.x, animal.z),
                scale: wildlife_scale(animal.kind, animal.energy),
                intensity: animal.energy,
                phase: animal.id as f32 * 1.17,
                kind: animal.kind,
            },
        ));
    }

    visuals
}

fn visual_assets_for(
    key: NatureVisualKey,
    assets: &NatureVisualAssets,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    match key {
        NatureVisualKey::Cloud(_) => (assets.cloud_mesh.clone(), assets.cloud_material.clone()),
        NatureVisualKey::RainDrop { .. } => {
            (assets.rain_mesh.clone(), assets.rain_material.clone())
        }
        NatureVisualKey::Plant(_) => (assets.plant_mesh.clone(), assets.plant_material.clone()),
        NatureVisualKey::BerryBush(_) => (assets.plant_mesh.clone(), assets.berry_material.clone()),
        NatureVisualKey::BerryFruit { .. } => {
            (assets.fruit_mesh.clone(), assets.fruit_material.clone())
        }
        NatureVisualKey::Rabbit(_) => (assets.animal_mesh.clone(), assets.rabbit_material.clone()),
        NatureVisualKey::Wildlife(_) => {
            (assets.animal_mesh.clone(), assets.wildlife_material.clone())
        }
    }
}

fn visual_transform(key: NatureVisualKey, state: NatureVisualState, elapsed: f32) -> Transform {
    let x = state.position.x;
    let z = state.position.y;
    match key {
        NatureVisualKey::Cloud(id) => {
            let phase = elapsed * 0.28 + state.phase + id as f32 * 0.13;
            Transform::from_xyz(x + phase.sin() * 0.24, 8.6 + phase.cos() * 0.12, z)
                .with_scale(state.scale)
        }
        NatureVisualKey::RainDrop { index, .. } => {
            let phase = elapsed * 2.4 + state.phase + index as f32 * 0.79;
            let spread = 0.34 + index as f32 * 0.17;
            Transform::from_xyz(
                x + phase.sin() * spread,
                7.7 - phase.rem_euclid(2.5),
                z + phase.cos() * spread,
            )
            .with_rotation(Quat::from_rotation_z(0.12))
        }
        NatureVisualKey::Plant(id) => {
            let sway = (elapsed * 1.2 + state.phase).sin() * 0.08;
            Transform::from_xyz(x, state.scale.y * 0.5, z)
                .with_rotation(
                    Quat::from_rotation_y(id as f32 * 0.71) * Quat::from_rotation_z(sway),
                )
                .with_scale(state.scale)
        }
        NatureVisualKey::BerryBush(_) => {
            let sway = (elapsed * 1.1 + state.phase).sin() * 0.035;
            Transform::from_xyz(x, state.scale.y * 0.5, z)
                .with_rotation(Quat::from_rotation_z(sway))
                .with_scale(state.scale)
        }
        NatureVisualKey::BerryFruit { index, .. } => {
            let angle = index as f32 * std::f32::consts::TAU / 3.0 + state.phase;
            Transform::from_xyz(x + angle.cos() * 0.28, 0.48, z + angle.sin() * 0.28)
        }
        NatureVisualKey::Rabbit(id) => {
            let phase = elapsed * 4.8 + state.phase;
            let hop = phase.sin().max(0.0) * 0.24;
            Transform::from_xyz(x, state.scale.y + hop, z)
                .with_rotation(Quat::from_rotation_y(
                    (elapsed * 0.35 + id as f32).sin() * 0.32,
                ))
                .with_scale(state.scale)
        }
        NatureVisualKey::Wildlife(id) => {
            let bob = (elapsed * 1.8 + state.phase).sin().max(0.0) * 0.06;
            Transform::from_xyz(x, state.scale.y + bob, z)
                .with_rotation(Quat::from_rotation_y(
                    (elapsed * 0.20 + id as f32).sin() * 0.42,
                ))
                .with_scale(state.scale)
        }
    }
}
