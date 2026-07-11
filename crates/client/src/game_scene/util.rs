//! Asset paths, scene constants, and small helper functions used by the
//! other `game_scene` submodules.

use bevy::prelude::*;
use bevy_world_serialization::WorldAssetRoot;

pub const BOSS_PATH: &str = "procedural/pretty/hoplite_ender_dragon.glb";
pub const TREE_PATH: &str = "procedural/pretty/sokpop_tree.glb";
pub const ROUND_TREE_PATH: &str = "procedural/pretty/granular_round_tree.glb";
pub const PINE_TREE_PATH: &str = "procedural/pretty/granular_pine_tree.glb";
pub const BRANCH_PATH: &str = "procedural/pretty/fallen_stick.glb";
pub const CLOUD_PATH: &str = "procedural/pretty/cloud_puff.glb";
pub const BERRY_PATH: &str = "procedural/eco/berry_bush.glb";
pub const RABBIT_PATH: &str = "procedural/eco/rabbit.glb";
pub const ROCK_PATH: &str = "procedural/pretty/rock_moss.glb";
pub const ROCK_DARK_PATH: &str = "procedural/pretty/rock_dark.glb";
pub const HILL_PATH: &str = "procedural/pretty/hill.glb";
pub const HOUSE_PATH: &str = "procedural/pretty/house_small.glb";
pub const CAMPFIRE_PATH: &str = "procedural/pretty/campfire.glb";
pub const CAVE_PATH: &str = "procedural/pretty/cave_entrance.glb";
pub const CHEST_PATH: &str = "procedural/pretty/treasure_chest.glb";
pub const PILLAR_RED_PATH: &str = "procedural/pretty/poi_pillar_red.glb";
pub const MUSHROOM_RED_PATH: &str = "procedural/pretty/mushroom_red.glb";
pub const MUSHROOM_BROWN_PATH: &str = "procedural/pretty/mushroom_brown.glb";

pub const PLAYER_SPEED: f32 = 5.2;
pub const PLAYER_JUMP_SPEED: f32 = 7.5;
pub const PLAYER_GRAVITY: f32 = -20.0;
pub const SCREENSHOT_MIN_FRAME: u64 = 24;
pub const SCREENSHOT_SCENE_SECS: f32 = 12.0;

pub fn spawn_asset<'a>(
    commands: &'a mut Commands,
    asset_server: &Res<AssetServer>,
    path: &'static str,
    position: Vec3,
    scale: f32,
    yaw: f32,
    name: &'static str,
) -> EntityCommands<'a> {
    commands.spawn((
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))),
        Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(scale)),
        Name::new(name),
    ))
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
