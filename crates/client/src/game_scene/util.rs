//! Asset paths, scene constants, and small helper functions used by the
//! other `game_scene` submodules.

use avian3d::prelude::{Collider, GravityScale, LockedAxes, RigidBody, TransformInterpolation};
use bevy::prelude::*;
use bevy_world_serialization::WorldAssetRoot;

use super::stylized_material::{StylizedAssetRoot, StylizedReadableAsset, StylizedTreeAsset};

pub const BOSS_PATH: &str = "procedural/pretty/hoplite_ender_dragon.glb";
pub const DRAGON_KATANA_PATH: &str = "procedural/pretty/hoplite_dragon_katana.glb";
pub const TREE_PATH: &str = "procedural/pretty/sokpop_tree.glb";
pub const ROUND_TREE_PATH: &str = "procedural/pretty/granular_round_tree.glb";
pub const PINE_TREE_PATH: &str = "procedural/pretty/granular_pine_tree.glb";
pub const FOREST_STONE_SPIRE_PATH: &str = "procedural/pretty/forest_stone_spire.glb";
pub const BIRCH_TREE_PATH: &str = "procedural/pretty/granular_birch_tree.glb";
pub const AUTUMN_TREE_PATH: &str = "procedural/pretty/granular_autumn_tree.glb";
pub const WILLOW_TREE_PATH: &str = "procedural/pretty/granular_willow_tree.glb";
pub const BRANCH_PATH: &str = "procedural/pretty/fallen_stick.glb";
pub const CLOUD_PATH: &str = "procedural/pretty/cloud_puff.glb";
pub const BERRY_PATH: &str = "procedural/eco/berry_bush.glb";
pub const RABBIT_PATH: &str = "procedural/eco/rabbit.glb";
pub const RABBIT_BROWN_PATH: &str = "animals/rabbit_brown.glb";
pub const WOLF_PATH: &str = "procedural/pretty/wolf.glb";
pub const ROCK_PATH: &str = "procedural/pretty/rock_moss.glb";
pub const ROCK_DARK_PATH: &str = "procedural/pretty/rock_dark.glb";
pub const HOUSE_PATH: &str = "procedural/pretty/house_small.glb";
pub const CAMPFIRE_PATH: &str = "procedural/pretty/campfire.glb";
pub const CAVE_PATH: &str = "procedural/pretty/cave_entrance.glb";
pub const CHEST_PATH: &str = "procedural/pretty/treasure_chest.glb";
pub const PILLAR_RED_PATH: &str = "procedural/pretty/poi_pillar_red.glb";
pub const MUSHROOM_RED_PATH: &str = "procedural/pretty/mushroom_red.glb";
pub const MUSHROOM_BROWN_PATH: &str = "procedural/pretty/mushroom_brown.glb";
pub const FLOWER_PATH: &str = "procedural/pretty/flower_0.glb";
pub const ROCK_MID_PATH: &str = "procedural/pretty/rock_mid.glb";
pub const CRYSTAL_PINK_PATH: &str = "procedural/pretty/crystal_pink.glb";
pub const CRYSTAL_BLUE_PATH: &str = "procedural/pretty/crystal_blue.glb";
pub const DEER_PATH: &str = "animals/deer.glb";
pub const DEER_FAWN_PATH: &str = "animals/deer_fawn.glb";
pub const FOX_PATH: &str = "animals/fox.glb";
pub const FOX_SILVER_PATH: &str = "animals/fox_silver.glb";
pub const BEAR_PATH: &str = "procedural/pretty/bear.glb";

pub const PLAYER_SPEED: f32 = 5.2;
pub const PLAYER_PHYSICS_CENTER_HEIGHT: f32 = 0.88;
pub const PLAYER_JUMP_HEIGHT: f32 = 1.45;
pub const PLAYER_JUMP_SPEED: f32 = 5.4;
pub const SCREENSHOT_MIN_FRAME: u64 = 24;
pub const SCREENSHOT_SCENE_SECS: f32 = 12.0;

pub fn spawn_asset<'a>(
    commands: &'a mut Commands,
    asset_server: &Res<AssetServer>,
    path: &str,
    position: Vec3,
    scale: f32,
    yaw: f32,
    name: impl Into<String>,
) -> EntityCommands<'a> {
    let name = name.into();
    let mut entity = commands.spawn((
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path.to_owned()))),
        Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(scale)),
        StylizedAssetRoot,
        Name::new(name.clone()),
    ));
    if name.starts_with("spice_") {
        if let Some(collider) = static_collider_for_asset(path) {
            entity.insert((RigidBody::Static, collider));
        }
    }
    if matches!(
        path,
        TREE_PATH
            | ROUND_TREE_PATH
            | PINE_TREE_PATH
            | BIRCH_TREE_PATH
            | AUTUMN_TREE_PATH
            | WILLOW_TREE_PATH
    ) {
        entity.insert(StylizedTreeAsset);
    } else if matches!(path, BOSS_PATH | HOUSE_PATH | FOREST_STONE_SPIRE_PATH) {
        entity.insert(StylizedReadableAsset);
    }
    entity
}

pub fn wildlife_physics_components() -> impl Bundle {
    (
        RigidBody::Dynamic,
        Collider::compound(vec![(
            Vec3::new(0.0, 0.35, 0.0),
            Quat::IDENTITY,
            Collider::capsule(0.24, 0.64),
        )]),
        GravityScale(0.0),
        LockedAxes::ROTATION_LOCKED,
        TransformInterpolation,
    )
}

fn static_collider_for_asset(path: &str) -> Option<Collider> {
    let collider = if matches!(
        path,
        TREE_PATH
            | ROUND_TREE_PATH
            | PINE_TREE_PATH
            | BIRCH_TREE_PATH
            | AUTUMN_TREE_PATH
            | WILLOW_TREE_PATH
    ) {
        Collider::compound(vec![(
            Vec3::new(0.0, 1.05, 0.0),
            Quat::IDENTITY,
            Collider::cylinder(0.32, 2.1),
        )])
    } else if path == HOUSE_PATH {
        Collider::compound(vec![(
            Vec3::new(0.0, 1.15, 0.0),
            Quat::IDENTITY,
            Collider::cuboid(2.8, 2.3, 2.8),
        )])
    } else if path == FOREST_STONE_SPIRE_PATH {
        Collider::compound(vec![(
            Vec3::new(0.0, 0.55, 0.0),
            Quat::IDENTITY,
            Collider::cuboid(2.2, 1.1, 1.7),
        )])
    } else if path == CAVE_PATH {
        Collider::compound(vec![(
            Vec3::new(0.0, 0.85, 0.0),
            Quat::IDENTITY,
            Collider::cuboid(2.4, 1.7, 2.4),
        )])
    } else if path == PILLAR_RED_PATH {
        Collider::compound(vec![(
            Vec3::new(0.0, 1.1, 0.0),
            Quat::IDENTITY,
            Collider::cuboid(0.5, 2.2, 0.5),
        )])
    } else if matches!(path, ROCK_PATH | ROCK_DARK_PATH | ROCK_MID_PATH) {
        Collider::compound(vec![(
            Vec3::new(0.0, 0.35, 0.0),
            Quat::IDENTITY,
            Collider::cuboid(0.9, 0.7, 0.9),
        )])
    } else if path == CAMPFIRE_PATH {
        Collider::compound(vec![(
            Vec3::new(0.0, 0.3, 0.0),
            Quat::IDENTITY,
            Collider::cylinder(0.5, 0.6),
        )])
    } else if path == CHEST_PATH {
        Collider::compound(vec![(
            Vec3::new(0.0, 0.35, 0.0),
            Quat::IDENTITY,
            Collider::cuboid(0.8, 0.7, 0.8),
        )])
    } else {
        return None;
    };
    Some(collider)
}

pub fn hash01(index: usize, salt: usize) -> f32 {
    let mut value = index as u32 ^ (salt as u32).wrapping_mul(0x9E37_79B9);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^= value >> 16;
    value as f32 / u32::MAX as f32
}

pub fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}
