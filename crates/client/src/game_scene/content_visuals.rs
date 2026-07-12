//! Visual adapter from shared content generation to the living forest scene.

use bevy::prelude::*;

use super::procedural_motion::ProceduralTreeSway;
use super::state::ProceduralTerrainSurface;
use super::util::{
    CAMPFIRE_PATH, CAVE_PATH, CHEST_PATH, HOUSE_PATH, MUSHROOM_BROWN_PATH, MUSHROOM_RED_PATH,
    PILLAR_RED_PATH, PINE_TREE_PATH, ROCK_DARK_PATH, ROCK_PATH, ROUND_TREE_PATH, TREE_PATH, hash01,
    spawn_asset,
};
use lk2_core::content::{ContentCategory, ContentRegistry, ContentStatus};
use lk2_core::world::content::{
    ContentSpiceProfile, ContentVolume, GAME_CONTENT_CAVERN, GAME_CONTENT_DUNGEON,
    GAME_CONTENT_EMPTY, GAME_CONTENT_MONSTER_TERRITORY, GAME_CONTENT_SETTLEMENT,
    GAME_CONTENT_TREASURE_VAULT, GAME_CONTENT_VERTICAL_PASSAGE, GAME_CONTENT_WILDERNESS,
    GameContentTheme, GameContentVolumeConfig, game_content_catalog, generate_game_content_volume,
    resolve_content_spice_profile,
};

pub const LIVING_CONTENT_DIMENSIONS: [usize; 3] = [5, 3, 5];
pub const DEFAULT_LIVING_CONTENT_SEED: u64 = 0x1A57_51CE;
pub const CONTENT_CELL_SPACING: f32 = 8.4;
pub(crate) const MONSTER_ANCHOR_PILLAR_SCALE: f32 = 0.52;
pub(crate) const MONSTER_DECORATIVE_MARKER_SCALE: f32 = 0.08;

#[derive(Component, Clone, Copy, Debug)]
pub struct ExportedContentVisual {
    pub status: ContentStatus,
    pub phase: f32,
}

#[derive(Resource, Clone, Debug)]
pub struct LivingContentLayout {
    pub volume: ContentVolume,
    pub profile: ContentSpiceProfile,
}

impl LivingContentLayout {
    pub fn generate(seed: u64, profile: Option<ContentSpiceProfile>) -> Self {
        let catalog = game_content_catalog(GameContentTheme::Frontier);
        let active_profile = profile.unwrap_or_else(|| resolve_content_spice_profile(seed));
        let mut config = GameContentVolumeConfig::new(LIVING_CONTENT_DIMENSIONS, seed);
        config = config.with_profile(active_profile);
        let volume = generate_game_content_volume(&catalog, &config)
            .expect("living scene content dimensions and catalog should generate");
        Self {
            volume,
            profile: active_profile,
        }
    }
}

pub fn living_content_layout_from_args(args: &[String]) -> LivingContentLayout {
    LivingContentLayout::generate(parse_content_seed(args), parse_content_profile(args))
}

pub fn parse_content_seed(args: &[String]) -> u64 {
    args.iter()
        .find_map(|arg| {
            arg.strip_prefix("--content-seed=")
                .and_then(|value| parse_seed_value(value))
        })
        .unwrap_or(DEFAULT_LIVING_CONTENT_SEED)
}

pub fn parse_content_profile(args: &[String]) -> Option<ContentSpiceProfile> {
    args.iter()
        .find_map(|arg| arg.strip_prefix("--content-profile="))
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "wilds" | "homestead" | "homestead-wilds" => Some(ContentSpiceProfile::HomesteadWilds),
            "monster" | "monster-march" => Some(ContentSpiceProfile::MonsterMarch),
            "crystal" | "crystal-descent" => Some(ContentSpiceProfile::CrystalDescent),
            "cavern" | "cavern-garden" => Some(ContentSpiceProfile::CavernGarden),
            _ => None,
        })
}

pub fn content_cell_position(dimensions: [usize; 3], cell: [usize; 3]) -> Vec3 {
    let center_x = (dimensions[0].saturating_sub(1)) as f32 * 0.5;
    let center_z = (dimensions[2].saturating_sub(1)) as f32 * 0.5;
    Vec3::new(
        (cell[0] as f32 - center_x) * CONTENT_CELL_SPACING,
        0.04,
        (cell[2] as f32 - center_z) * CONTENT_CELL_SPACING,
    )
}

pub fn grounded_content_position(terrain: &ProceduralTerrainSurface, position: Vec3) -> Vec3 {
    Vec3::new(
        position.x,
        terrain.ground_height(position) + 0.04,
        position.z,
    )
}

pub(crate) const fn content_profile_label(profile: ContentSpiceProfile) -> &'static str {
    match profile {
        ContentSpiceProfile::HomesteadWilds => "HomesteadWilds",
        ContentSpiceProfile::MonsterMarch => "MonsterMarch",
        ContentSpiceProfile::CrystalDescent => "CrystalDescent",
        ContentSpiceProfile::CavernGarden => "CavernGarden",
    }
}

pub fn spawn_content_visuals(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    layout: &LivingContentLayout,
    terrain: &ProceduralTerrainSurface,
) {
    let dimensions = layout.volume.dimensions;
    for z in 0..dimensions[2] {
        for x in 0..dimensions[0] {
            let surface_cell = [x, 1, z];
            let Some(surface_content) = layout.volume.get(surface_cell) else {
                continue;
            };
            let base = content_cell_position(dimensions, surface_cell);
            spawn_surface_content(
                commands,
                asset_server,
                terrain,
                base,
                surface_cell,
                surface_content,
            );

            let underground_cell = [x, 0, z];
            if let Some(underground_content) = layout.volume.get(underground_cell) {
                spawn_underground_hint(
                    commands,
                    asset_server,
                    terrain,
                    base + Vec3::new(1.7, 0.0, -1.7),
                    underground_cell,
                    underground_content,
                );
            }
        }
    }
}

/// Places every registry definition that has a model in a readable gallery.
/// This is presentation-only: planned entries remain planned in the registry
/// and do not acquire simulation or interaction rules here.
pub fn spawn_exported_content_visuals(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    registry: &ContentRegistry,
    terrain: &ProceduralTerrainSurface,
) {
    let mut definitions = registry
        .definitions()
        .iter()
        .filter(|definition| definition.visual.is_some())
        .collect::<Vec<_>>();
    definitions.sort_by(|left, right| {
        let left_planned = left.status == ContentStatus::Planned;
        let right_planned = right.status == ContentStatus::Planned;
        right_planned
            .cmp(&left_planned)
            .then_with(|| left.key.cmp(&right.key))
    });

    for (index, definition) in definitions.into_iter().enumerate() {
        let Some(visual) = definition.visual.as_ref() else {
            continue;
        };
        let position = exported_content_position(index, definition.category);
        let grounded = Vec3::new(
            position.x,
            terrain.ground_height(position) + 0.04,
            position.z,
        );
        let yaw = (index as f32 * 0.71).rem_euclid(std::f32::consts::TAU);
        spawn_asset(
            commands,
            asset_server,
            &visual.model_path,
            grounded,
            visual.scale[0],
            yaw,
            format!("content_visual_{}", definition.key.replace('.', "_")),
        )
        .insert(ExportedContentVisual {
            status: definition.status,
            phase: index as f32 * 0.83,
        });
    }
}

fn exported_content_position(index: usize, category: ContentCategory) -> Vec3 {
    let (columns, spacing, origin_x, origin_z) = match category {
        ContentCategory::Creature | ContentCategory::Item => (5, 3.3, -10.0, -3.0),
        _ => (6, 5.0, -25.0, -8.0),
    };
    let row = index / columns;
    let column = index % columns;
    Vec3::new(
        origin_x + column as f32 * spacing,
        0.0,
        origin_z - row as f32 * spacing,
    )
}

fn spawn_surface_content(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    terrain: &ProceduralTerrainSurface,
    base: Vec3,
    cell: [usize; 3],
    content: lk2_core::world::content::ContentId,
) {
    let index = cell[0] + cell[2] * 11;
    let yaw = hash01(index, 601) * std::f32::consts::TAU;
    match content {
        GAME_CONTENT_SETTLEMENT => {
            spawn_asset(
                commands,
                asset_server,
                HOUSE_PATH,
                grounded_content_position(terrain, base + Vec3::new(-1.3, 0.0, -0.9)),
                0.95,
                yaw,
                "spice_settlement_house",
            );
            spawn_asset(
                commands,
                asset_server,
                CAMPFIRE_PATH,
                grounded_content_position(terrain, base + Vec3::new(1.7, 0.0, 1.1)),
                0.72,
                yaw + 0.4,
                "spice_settlement_campfire",
            );
        }
        GAME_CONTENT_MONSTER_TERRITORY => {
            let is_anchor = is_monster_anchor(cell);
            if is_anchor || !is_initial_view_corridor(base) {
                let pillar_scale = if is_anchor {
                    MONSTER_ANCHOR_PILLAR_SCALE
                } else {
                    MONSTER_DECORATIVE_MARKER_SCALE
                };
                spawn_asset(
                    commands,
                    asset_server,
                    PILLAR_RED_PATH,
                    grounded_content_position(terrain, base + Vec3::new(-1.4, 0.0, 0.0)),
                    pillar_scale,
                    yaw,
                    if is_anchor {
                        "spice_monster_anchor_pillar"
                    } else {
                        "spice_monster_small_marker"
                    },
                );
            }
            spawn_asset(
                commands,
                asset_server,
                ROCK_DARK_PATH,
                grounded_content_position(terrain, base + Vec3::new(1.4, 0.0, 0.9)),
                if is_initial_view_corridor(base) {
                    0.64
                } else {
                    0.78
                },
                yaw + 1.2,
                "spice_monster_rock",
            );
        }
        GAME_CONTENT_VERTICAL_PASSAGE => {
            spawn_asset(
                commands,
                asset_server,
                CAVE_PATH,
                grounded_content_position(terrain, base),
                0.86,
                yaw,
                "spice_vertical_passage",
            );
        }
        GAME_CONTENT_WILDERNESS => {
            let path = if index % 3 == 0 {
                PINE_TREE_PATH
            } else if index % 2 == 0 {
                ROUND_TREE_PATH
            } else {
                TREE_PATH
            };
            let position = grounded_content_position(
                terrain,
                base + Vec3::new(-0.8 + hash01(index, 607) * 1.6, 0.0, -0.6),
            );
            spawn_asset(
                commands,
                asset_server,
                path,
                position,
                0.72 + hash01(index, 613) * 0.18,
                yaw,
                "spice_wilderness_tree",
            )
            .insert(ProceduralTreeSway {
                base_translation: position,
                base_yaw: yaw,
                base_scale: 0.72 + hash01(index, 613) * 0.18,
                phase: index as f32 * 0.41,
                strength: 0.018,
            });
        }
        GAME_CONTENT_EMPTY => {}
        _ => {}
    }
}

fn spawn_underground_hint(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    terrain: &ProceduralTerrainSurface,
    base: Vec3,
    cell: [usize; 3],
    content: lk2_core::world::content::ContentId,
) {
    let index = cell[0] + cell[2] * 13;
    let yaw = hash01(index, 701) * std::f32::consts::TAU;
    match content {
        GAME_CONTENT_TREASURE_VAULT => {
            spawn_asset(
                commands,
                asset_server,
                CHEST_PATH,
                grounded_content_position(terrain, base),
                0.78,
                yaw,
                "spice_treasure_hint",
            );
        }
        GAME_CONTENT_DUNGEON if index % 2 == 0 => {
            spawn_asset(
                commands,
                asset_server,
                ROCK_PATH,
                grounded_content_position(terrain, base),
                0.58,
                yaw,
                "spice_dungeon_hint",
            );
        }
        GAME_CONTENT_CAVERN if index % 3 == 0 => {
            let path = if index % 2 == 0 {
                MUSHROOM_RED_PATH
            } else {
                MUSHROOM_BROWN_PATH
            };
            spawn_asset(
                commands,
                asset_server,
                path,
                grounded_content_position(terrain, base),
                0.68,
                yaw,
                "spice_cavern_hint",
            );
        }
        GAME_CONTENT_VERTICAL_PASSAGE => {
            spawn_asset(
                commands,
                asset_server,
                CAVE_PATH,
                grounded_content_position(terrain, base),
                0.52,
                yaw + 0.8,
                "spice_underpass_hint",
            );
        }
        _ => {}
    }
}

fn parse_seed_value(value: &str) -> Option<u64> {
    value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(
            || value.parse::<u64>().ok(),
            |hex| u64::from_str_radix(hex, 16).ok(),
        )
}

fn is_initial_view_corridor(position: Vec3) -> bool {
    position.z > 0.0 && position.x.abs() < CONTENT_CELL_SPACING * 1.35
}

pub(crate) fn is_monster_anchor(cell: [usize; 3]) -> bool {
    cell == [1, 1, 1]
}
