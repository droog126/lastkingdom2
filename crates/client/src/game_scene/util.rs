//! Asset paths, scene constants, and small helper functions used by the
//! other `game_scene` submodules.

use avian3d::prelude::{Collider, GravityScale, LockedAxes, RigidBody, TransformInterpolation};
use bevy::prelude::*;
use bevy_world_serialization::WorldAssetRoot;

use super::state::{ExplorableBuilding, ForestRealmVisual, Inspectable};
use super::stylized_material::{StylizedAssetRoot, StylizedReadableAsset, StylizedTreeAsset};

pub const BOSS_PATH: &str = "procedural/pretty/hoplite_ender_dragon.glb";
pub const PLAYER_MODEL_PATH: &str = "procedural/pretty/sokpop_gatherer.glb";
pub const VILLAGER_MODEL_PATH: &str = "procedural/pretty/villager.glb";
pub const DRAGON_KATANA_PATH: &str = "procedural/pretty/hoplite_dragon_katana.glb";
pub const REAPER_SCYTHE_PATH: &str = "procedural/pretty/hoplite_reaper_scythe.glb";
pub const GOLEM_HAMMER_PATH: &str = "procedural/pretty/hoplite_golem_hammer.glb";
pub const MIDAS_SWORD_PATH: &str = "procedural/pretty/hoplite_midas_sword.glb";
pub const TREE_PATH: &str = "procedural/pretty/sokpop_tree.glb";
pub const ROUND_TREE_PATH: &str = "procedural/pretty/granular_round_tree.glb";
pub const PINE_TREE_PATH: &str = "procedural/pretty/granular_pine_tree.glb";
pub const FOREST_STONE_SPIRE_PATH: &str = "procedural/pretty/forest_stone_spire.glb";
pub const BIRCH_TREE_PATH: &str = "procedural/pretty/granular_birch_tree.glb";
pub const AUTUMN_TREE_PATH: &str = "procedural/pretty/granular_autumn_tree.glb";
pub const WILLOW_TREE_PATH: &str = "procedural/pretty/granular_willow_tree.glb";
pub const BRANCH_PATH: &str = "procedural/pretty/fallen_stick.glb";
pub const CLOUD_PATH: &str = "procedural/pretty/cloud_puff.glb";
// Keep the authored ecology clouds inside the opening camera's visual band;
// the previous high/small placement made the sky read as an empty flat color.
pub const CLOUD_VISUAL_HEIGHT: f32 = 11.5;
pub const CLOUD_VISUAL_SCALE: f32 = 1.05;
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
pub const FLOWER_PATH: &str = "procedural/pretty/flower_0.glb";
pub const GRASS_FOOD_PATH: &str = "procedural/pretty/granular_wildflowers.glb";
pub const ROCK_MID_PATH: &str = "procedural/pretty/rock_mid.glb";
pub const CRYSTAL_PINK_PATH: &str = "procedural/pretty/crystal_pink.glb";
pub const CRYSTAL_BLUE_PATH: &str = "procedural/pretty/crystal_blue.glb";
pub const DEER_PATH: &str = "animals/deer.glb";
pub const DEER_FAWN_PATH: &str = "animals/deer_fawn.glb";
pub const FOX_PATH: &str = "animals/fox.glb";
pub const FOX_SILVER_PATH: &str = "animals/fox_silver.glb";
pub const BEAR_PATH: &str = "procedural/pretty/bear.glb";
pub const CART_PATH: &str = "procedural/pretty/cart.glb";
pub const FISH_PATH: &str = "animals/fish.glb";
pub const AETHER_WRAITH_PATH: &str = "procedural/pretty/monster_aether_wraith.glb";

/// GLB assets used directly by the playable game scene.
///
/// Content-registry visuals are added by `model_preview` from the shared core
/// registry; this list owns the scene-only assets that do not have a content
/// definition of their own.
pub(crate) const MAIN_APP_MODEL_PATHS: &[&str] = &[
    BOSS_PATH,
    PLAYER_MODEL_PATH,
    VILLAGER_MODEL_PATH,
    DRAGON_KATANA_PATH,
    REAPER_SCYTHE_PATH,
    GOLEM_HAMMER_PATH,
    MIDAS_SWORD_PATH,
    TREE_PATH,
    ROUND_TREE_PATH,
    PINE_TREE_PATH,
    FOREST_STONE_SPIRE_PATH,
    BIRCH_TREE_PATH,
    AUTUMN_TREE_PATH,
    WILLOW_TREE_PATH,
    BRANCH_PATH,
    CLOUD_PATH,
    BERRY_PATH,
    RABBIT_PATH,
    RABBIT_BROWN_PATH,
    WOLF_PATH,
    ROCK_PATH,
    ROCK_DARK_PATH,
    HOUSE_PATH,
    CAMPFIRE_PATH,
    CAVE_PATH,
    CHEST_PATH,
    PILLAR_RED_PATH,
    FLOWER_PATH,
    GRASS_FOOD_PATH,
    ROCK_MID_PATH,
    CRYSTAL_PINK_PATH,
    CRYSTAL_BLUE_PATH,
    DEER_PATH,
    DEER_FAWN_PATH,
    FOX_PATH,
    FOX_SILVER_PATH,
    BEAR_PATH,
    CART_PATH,
    FISH_PATH,
    AETHER_WRAITH_PATH,
];

// The cart mesh's handles (its front) are authored along local +X, while the
// player/avatar forward direction is local +Z. Keep this offset at the asset
// boundary so the cart always faces the same way as its rider.
pub const CART_FRONT_YAW_OFFSET: f32 = -std::f32::consts::FRAC_PI_2;

pub const PLAYER_SPEED: f32 = 5.2;
pub const PLAYER_SPRINT_SPEED_MULTIPLIER: f32 = 1.65;
pub const PLAYER_COLLIDER_RADIUS: f32 = 0.38;
pub const PLAYER_COLLIDER_SEGMENT: f32 = 1.0;
pub const PLAYER_CROUCH_COLLIDER_SEGMENT: f32 = 0.40;
pub const PLAYER_PHYSICS_CENTER_HEIGHT: f32 = 0.88;
pub const PLAYER_JUMP_HEIGHT: f32 = 1.45;
pub const PLAYER_JUMP_SPEED: f32 = 5.4;
pub const SCREENSHOT_MIN_FRAME: u64 = 24;
pub const SCREENSHOT_SCENE_SECS: f32 = 12.0;

pub fn player_physics_half_height(crouch_amount: f32) -> f32 {
    PLAYER_COLLIDER_RADIUS
        + PLAYER_COLLIDER_SEGMENT.lerp(
            PLAYER_CROUCH_COLLIDER_SEGMENT,
            crouch_amount.clamp(0.0, 1.0),
        ) * 0.5
}

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
        ForestRealmVisual,
        Name::new(name.clone()),
    ));
    if let Some(inspectable) = inspectable_for_asset(path) {
        entity.insert(inspectable);
    }
    if name.starts_with("spice_") {
        if let Some(collider) = static_collider_for_asset(path) {
            entity.insert((RigidBody::Static, collider));
        }
    }
    if path == CAMPFIRE_PATH {
        entity.with_children(|campfire| {
            campfire.spawn((
                RectLight {
                    // RectLight provides a broad local warm fill. Bevy 0.19
                    // does not shadow from area lights, so the sun/contact
                    // shadow path remains responsible for occlusion.
                    color: Color::srgb(1.0, 0.30, 0.08),
                    intensity: 4_500.0,
                    width: 0.85,
                    height: 0.65,
                    range: 7.0,
                },
                Transform::from_xyz(0.0, 0.72, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
                Name::new("campfire_area_light"),
            ));
        });
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
    } else if path == HOUSE_PATH {
        entity.insert(ExplorableBuilding { interior: None });
        entity.insert(StylizedReadableAsset);
    } else if matches!(
        path,
        BOSS_PATH
            | FOREST_STONE_SPIRE_PATH
            | DRAGON_KATANA_PATH
            | REAPER_SCYTHE_PATH
            | BERRY_PATH
            | FLOWER_PATH
            | GRASS_FOOD_PATH
            | ROCK_MID_PATH
            | CRYSTAL_PINK_PATH
            | CRYSTAL_BLUE_PATH
            | CHEST_PATH
    ) {
        // Resource nodes and pickups get the same restrained rim as major
        // landmarks, making them readable against grass and tree shadows.
        entity.insert(StylizedReadableAsset);
    }
    entity
}

fn inspectable_for_asset(path: &str) -> Option<Inspectable> {
    let (display_name, category) = match path {
        BOSS_PATH => ("末日巨龙", "危险生物"),
        TREE_PATH | ROUND_TREE_PATH | PINE_TREE_PATH | BIRCH_TREE_PATH | AUTUMN_TREE_PATH
        | WILLOW_TREE_PATH => ("树", "自然物"),
        BRANCH_PATH => ("枯枝", "自然物"),
        CLOUD_PATH => ("云", "天气"),
        BERRY_PATH => ("浆果灌木", "植物"),
        RABBIT_PATH | RABBIT_BROWN_PATH => ("兔子", "野生动物"),
        WOLF_PATH => ("狼", "野生动物"),
        DEER_PATH | DEER_FAWN_PATH => ("鹿", "野生动物"),
        FOX_PATH | FOX_SILVER_PATH => ("狐狸", "野生动物"),
        BEAR_PATH => ("熊", "野生动物"),
        ROCK_PATH | ROCK_DARK_PATH | ROCK_MID_PATH => ("岩石", "自然物"),
        FLOWER_PATH => ("花", "植物"),
        GRASS_FOOD_PATH => ("野草", "植物"),
        CRYSTAL_PINK_PATH => ("日耀水晶", "资源"),
        CRYSTAL_BLUE_PATH => ("霜蓝水晶", "资源"),
        HOUSE_PATH => ("小屋", "建筑"),
        CAMPFIRE_PATH => ("篝火", "设施"),
        CAVE_PATH => ("矿洞入口", "地点"),
        CHEST_PATH => ("宝箱", "容器"),
        PILLAR_RED_PATH => ("红色石柱", "地标"),
        FOREST_STONE_SPIRE_PATH => ("森林石尖碑", "地标"),
        DRAGON_KATANA_PATH => ("龙之太刀", "武器"),
        REAPER_SCYTHE_PATH => ("死神镰刀", "武器"),
        GOLEM_HAMMER_PATH => ("石像鬼战锤", "武器"),
        MIDAS_SWORD_PATH => ("点金剑", "武器"),
        CART_PATH => ("矿车", "载具"),
        FISH_PATH => ("鱼", "水生动物"),
        AETHER_WRAITH_PATH => ("以太幽魂", "危险生物"),
        _ => return None,
    };
    Some(Inspectable {
        display_name,
        category,
    })
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
        // Leave the front doorway open so the player can reach the interaction point.
        Collider::compound(vec![
            (
                Vec3::new(0.0, 0.90, -0.58),
                Quat::IDENTITY,
                Collider::cuboid(1.60, 1.70, 0.16),
            ),
            (
                Vec3::new(-0.71, 0.90, 0.0),
                Quat::IDENTITY,
                Collider::cuboid(0.18, 1.70, 1.16),
            ),
            (
                Vec3::new(0.71, 0.90, 0.0),
                Quat::IDENTITY,
                Collider::cuboid(0.18, 1.70, 1.16),
            ),
            (
                Vec3::new(-0.55, 0.90, 0.58),
                Quat::IDENTITY,
                Collider::cuboid(0.50, 1.70, 0.16),
            ),
            (
                Vec3::new(0.55, 0.90, 0.58),
                Quat::IDENTITY,
                Collider::cuboid(0.50, 1.70, 0.16),
            ),
        ])
    } else if path == FOREST_STONE_SPIRE_PATH {
        Collider::compound(vec![(
            Vec3::new(0.0, 0.55, 0.0),
            Quat::IDENTITY,
            Collider::cuboid(2.2, 1.1, 1.7),
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
