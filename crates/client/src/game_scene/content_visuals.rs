//! Visual adapter from shared content generation to the living forest scene.

use std::collections::HashMap;

use bevy::{prelude::*, sprite::Text2dShadow};

use super::procedural_motion::ProceduralTreeSway;
use super::state::{CaveEntrance, ProceduralTerrainSurface};
use super::util::{
    CAMPFIRE_PATH, CAVE_PATH, CHEST_PATH, HOUSE_PATH, PILLAR_RED_PATH, PINE_TREE_PATH,
    ROCK_DARK_PATH, ROCK_PATH, ROUND_TREE_PATH, TREE_PATH, hash01, spawn_asset,
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
// Keep the authored introduction clear. The generated content volume remains
// present, but its secondary structures begin beyond the readable hub ring.
pub const CONTENT_CELL_SPACING: f32 = 18.0;
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

    let label_font = asset_server.load("fonts/NotoSansCJKsc-Regular.otf");
    let mut category_slots = HashMap::<ContentCategory, usize>::new();
    for (index, definition) in definitions.into_iter().enumerate() {
        let Some(visual) = definition.visual.as_ref() else {
            continue;
        };
        let slot = category_slots.entry(definition.category).or_default();
        let position = exported_content_position(*slot, definition.category);
        *slot += 1;
        let is_tree = visual.model_path.contains("tree");
        if is_tree && position.distance_squared(Vec3::new(-2.0, 0.0, 11.0)) < 24.0 * 24.0 {
            continue;
        }
        let Some(grounded) = terrain.grounded_land_position(position, 26, 0.04) else {
            continue;
        };
        let yaw = (index as f32 * 0.71).rem_euclid(std::f32::consts::TAU);
        let visual_scale = if is_tree {
            visual.scale[0] * 0.55
        } else {
            visual.scale[0]
        };
        spawn_asset(
            commands,
            asset_server,
            &visual.model_path,
            grounded,
            visual_scale,
            yaw,
            format!("content_visual_{}", definition.key.replace('.', "_")),
        )
        .insert(ExportedContentVisual {
            status: definition.status,
            phase: index as f32 * 0.83,
        });

        // The gallery is a content-audit surface, so every ambiguous low-poly
        // prop gets a small world-space name. This is especially useful for
        // crystals and flowers that otherwise collapse into one
        // pale cluster from the gameplay camera.
        commands.spawn((
            Text2d::new(content_visual_label(definition.key.as_str())),
            TextFont {
                font: label_font.clone().into(),
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.92, 0.68)),
            Text2dShadow {
                offset: Vec2::new(2.0, -2.0),
                color: Color::srgba(0.04, 0.02, 0.01, 0.92),
            },
            Transform::from_translation(
                grounded + Vec3::Y * (1.15 + visual.scale[1].abs().max(0.35)),
            ),
            Name::new(format!(
                "content_label_{}",
                definition.key.replace('.', "_")
            )),
        ));
    }
}

fn exported_content_position(index: usize, category: ContentCategory) -> Vec3 {
    let (columns, spacing, origin_x, origin_z) = match category {
        ContentCategory::Creature => (4, 3.3, -9.0, 12.0),
        ContentCategory::Item => (2, 4.2, 7.0, 12.0),
        ContentCategory::Wildlife => (5, 3.6, -9.0, 7.0),
        ContentCategory::Plant => (3, 4.0, -5.0, 1.0),
        ContentCategory::ResourceNode => (4, 3.5, -8.0, -7.0),
        ContentCategory::Drop => (3, 3.5, 5.0, -7.0),
        ContentCategory::Resource => (4, 3.5, -8.0, -12.0),
    };
    let row = index / columns;
    let column = index % columns;
    Vec3::new(
        origin_x + column as f32 * spacing,
        0.0,
        origin_z - row as f32 * spacing,
    )
}

fn content_visual_label(key: &str) -> &'static str {
    match key {
        "wildlife.rabbit" => "兔子",
        "wildlife.deer" => "鹿",
        "wildlife.fox" => "狐狸",
        "wildlife.bear" => "熊",
        "wildlife.wolf" => "狼",
        "plant.sokpop_tree" => "树",
        "plant.fallen_stick" => "枯枝",
        "plant.palm" => "棕榈树",
        "resource_node.berry_bush" => "浆果灌木",
        "resource_node.flower" => "花",
        "resource_node.rock_mid" => "岩石",
        "resource_node.rock_moss" => "苔岩",
        "resource_node.sunstone_crystal" => "太阳晶体",
        "resource_node.frost_crystal" => "霜晶",
        "drop.berry_fruit" => "浆果",
        "drop.stone" => "石头",
        "item.reaper_scythe" => "收割者镰刀",
        "item.dragon_katana" => "龙之刀",
        "creature.pig" => "猪",
        "creature.sheep" => "羊",
        "creature.cow" => "牛",
        "creature.chicken" => "鸡",
        _ => "内容",
    }
}

fn spawn_surface_content(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    terrain: &ProceduralTerrainSurface,
    base: Vec3,
    cell: [usize; 3],
    content: lk2_core::world::content::ContentId,
) {
    if is_initial_view_corridor(base) {
        return;
    }
    let index = cell[0] + cell[2] * 11;
    let yaw = hash01(index, 601) * std::f32::consts::TAU;
    match content {
        GAME_CONTENT_SETTLEMENT => {
            // The central settlement cell sits directly in front of the
            // player spawn. Its house is large enough to put the first-person
            // camera under the roof, so keep large structures out of the same
            // initial-view corridor already reserved from wilderness trees.
            if !is_initial_view_corridor(base) {
                spawn_asset(
                    commands,
                    asset_server,
                    HOUSE_PATH,
                    grounded_content_position(terrain, base + Vec3::new(-1.3, 0.0, -0.9)),
                    0.95,
                    yaw,
                    "spice_settlement_house",
                );
            }
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
            let to_spawn = Vec2::new(-2.0 - base.x, 11.0 - base.z);
            let entrance_yaw = if to_spawn.length_squared() > 0.01 {
                to_spawn.x.atan2(-to_spawn.y)
            } else {
                yaw
            };
            spawn_asset(
                commands,
                asset_server,
                CAVE_PATH,
                grounded_content_position(terrain, base),
                0.68,
                entrance_yaw,
                "spice_vertical_passage",
            )
            .insert(CaveEntrance);
        }
        GAME_CONTENT_WILDERNESS => {
            if is_initial_view_corridor(base) {
                return;
            }
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
            let tree_scale = 0.58 + hash01(index, 613) * 0.14;
            spawn_asset(
                commands,
                asset_server,
                path,
                position,
                tree_scale,
                yaw,
                "spice_wilderness_tree",
            )
            .insert(ProceduralTreeSway {
                base_translation: position,
                base_yaw: yaw,
                base_scale: tree_scale,
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
    if is_initial_view_corridor(base) {
        return;
    }
    let index = cell[0] + cell[2] * 13;
    // The entrance mesh opens toward local -Z. Point it toward the initial
    // spawn corridor instead of using a random yaw; otherwise the gameplay
    // camera often sees only the solid back of the rock.
    let to_spawn = Vec2::new(-2.0 - base.x, 11.0 - base.z);
    let spawn_facing_yaw = to_spawn.x.atan2(-to_spawn.y);
    let yaw = if to_spawn.length_squared() > 0.01 {
        spawn_facing_yaw
    } else {
        hash01(index, 701) * std::f32::consts::TAU
    };
    let entrance_offset = if cell == [2, 0, 2] {
        // Keep the authored central mine mouth in the same readable corridor
        // as the player spawn, rather than hiding it behind the settlement.
        Vec3::new(-1.0, 0.0, 2.8)
    } else {
        Vec3::new(1.7, 0.0, -1.7)
    };
    let entrance_position = grounded_content_position(terrain, base + entrance_offset);
    match content {
        GAME_CONTENT_CAVERN if index % 3 == 0 => {
            spawn_asset(
                commands,
                asset_server,
                CAVE_PATH,
                entrance_position,
                0.58,
                yaw,
                "spice_cavern_entrance",
            )
            .insert(CaveEntrance);
        }
        GAME_CONTENT_TREASURE_VAULT => {
            spawn_asset(
                commands,
                asset_server,
                CHEST_PATH,
                entrance_position,
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
                entrance_position,
                0.58,
                yaw,
                "spice_dungeon_hint",
            );
        }
        GAME_CONTENT_VERTICAL_PASSAGE => {
            spawn_asset(
                commands,
                asset_server,
                CAVE_PATH,
                entrance_position,
                0.58,
                yaw,
                "spice_underpass_hint",
            )
            .insert(CaveEntrance);
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
    // The first view is an authored introduction, not a content-registry
    // gallery. Keep generated caves, houses, rocks, and wilderness props out
    // of the 20 m around the approach so a player can identify the camp,
    // farm, and routes before discovering secondary content.
    position.distance_squared(Vec3::new(0.0, 0.0, 5.0)) <= 28.0 * 28.0
}

pub(crate) fn is_monster_anchor(cell: [usize; 3]) -> bool {
    cell == [1, 1, 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_view_corridor_includes_the_central_settlement_cell() {
        assert!(is_initial_view_corridor(Vec3::ZERO));
        assert!(is_initial_view_corridor(Vec3::new(-2.0, 0.0, 8.4)));
        assert!(is_initial_view_corridor(Vec3::new(20.0, 0.0, 0.0)));
        assert!(!is_initial_view_corridor(Vec3::new(30.0, 0.0, 0.0)));
        assert!(is_initial_view_corridor(Vec3::new(0.0, 0.0, -20.0)));
        assert!(!is_initial_view_corridor(Vec3::new(0.0, 0.0, -30.0)));
    }
}
