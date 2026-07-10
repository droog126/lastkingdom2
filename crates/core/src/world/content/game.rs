use super::{
    CellConstraint, ContentCatalog, ContentId, ContentSolveConfig, ContentSolveError,
    ContentVolume, Direction3, solve_content_volume,
};
use serde::{Deserialize, Serialize};

pub const GAME_CONTENT_EMPTY: ContentId = ContentId(0);
pub const GAME_CONTENT_WILDERNESS: ContentId = ContentId(1);
pub const GAME_CONTENT_SETTLEMENT: ContentId = ContentId(2);
pub const GAME_CONTENT_MONSTER_TERRITORY: ContentId = ContentId(3);
pub const GAME_CONTENT_CAVERN: ContentId = ContentId(4);
pub const GAME_CONTENT_DUNGEON: ContentId = ContentId(5);
pub const GAME_CONTENT_TREASURE_VAULT: ContentId = ContentId(6);
pub const GAME_CONTENT_VERTICAL_PASSAGE: ContentId = ContentId(7);

const SURFACE_CONTENT: [ContentId; 4] = [
    GAME_CONTENT_WILDERNESS,
    GAME_CONTENT_SETTLEMENT,
    GAME_CONTENT_MONSTER_TERRITORY,
    GAME_CONTENT_VERTICAL_PASSAGE,
];
const UNDERGROUND_CONTENT: [ContentId; 4] = [
    GAME_CONTENT_CAVERN,
    GAME_CONTENT_DUNGEON,
    GAME_CONTENT_TREASURE_VAULT,
    GAME_CONTENT_VERTICAL_PASSAGE,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameContentTheme {
    Frontier,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameContentVolumeConfig {
    pub dimensions: [usize; 3],
    pub seed: u64,
}

impl GameContentVolumeConfig {
    pub const fn new(dimensions: [usize; 3], seed: u64) -> Self {
        Self { dimensions, seed }
    }
}

pub fn game_content_catalog(theme: GameContentTheme) -> ContentCatalog {
    match theme {
        GameContentTheme::Frontier => frontier_catalog(),
    }
}

pub fn generate_game_content_volume(
    catalog: &ContentCatalog,
    config: &GameContentVolumeConfig,
) -> Result<ContentVolume, ContentSolveError> {
    if config.dimensions[0] < 4 || config.dimensions[1] < 3 || config.dimensions[2] < 3 {
        return Err(ContentSolveError::EmptyDimensions);
    }

    let mut solve = ContentSolveConfig::new(config.dimensions, config.seed);
    for y in 0..config.dimensions[1] {
        let layer_content: &[ContentId] = match y {
            0 => &UNDERGROUND_CONTENT,
            1 => &SURFACE_CONTENT,
            _ => &[GAME_CONTENT_EMPTY],
        };
        for z in 0..config.dimensions[2] {
            for x in 0..config.dimensions[0] {
                solve.constraints.push(CellConstraint::any_of(
                    [x, y, z],
                    layer_content.iter().copied(),
                ));
            }
        }
    }

    let center = [config.dimensions[0] / 2, 1, config.dimensions[2] / 2];
    let underground_center = [config.dimensions[0] / 2, 0, config.dimensions[2] / 2];
    let monster = [1, 1, 1];
    let treasure = [config.dimensions[0] - 2, 0, config.dimensions[2] - 2];
    let dungeon_neighbor = [config.dimensions[0] - 3, 0, config.dimensions[2] - 2];
    solve.constraints.extend([
        CellConstraint::only(center, GAME_CONTENT_SETTLEMENT),
        CellConstraint::only(underground_center, GAME_CONTENT_VERTICAL_PASSAGE),
        CellConstraint::only(monster, GAME_CONTENT_MONSTER_TERRITORY),
        CellConstraint::only(treasure, GAME_CONTENT_TREASURE_VAULT),
        CellConstraint::only(dungeon_neighbor, GAME_CONTENT_DUNGEON),
    ]);

    solve_content_volume(catalog, &solve)
}

fn frontier_catalog() -> ContentCatalog {
    let mut builder = ContentCatalog::builder()
        .archetype(GAME_CONTENT_EMPTY, "empty", 1)
        .archetype(GAME_CONTENT_WILDERNESS, "wilderness", 12)
        .archetype(GAME_CONTENT_SETTLEMENT, "settlement", 2)
        .archetype(GAME_CONTENT_MONSTER_TERRITORY, "monster_territory", 3)
        .archetype(GAME_CONTENT_CAVERN, "cavern", 10)
        .archetype(GAME_CONTENT_DUNGEON, "dungeon", 3)
        .archetype(GAME_CONTENT_TREASURE_VAULT, "treasure_vault", 1)
        .archetype(GAME_CONTENT_VERTICAL_PASSAGE, "vertical_passage", 2)
        .allow_same_position_neighbors(GAME_CONTENT_EMPTY);

    for &first in &SURFACE_CONTENT {
        for &second in &SURFACE_CONTENT {
            builder = allow_horizontal(builder, first, second);
        }
        builder = builder.allow_bidirectional(first, Direction3::PosY, GAME_CONTENT_EMPTY);
    }

    for &first in &UNDERGROUND_CONTENT {
        for &second in &UNDERGROUND_CONTENT {
            let treasure_pair =
                first == GAME_CONTENT_TREASURE_VAULT || second == GAME_CONTENT_TREASURE_VAULT;
            let treasure_supported = matches!(
                (first, second),
                (
                    GAME_CONTENT_TREASURE_VAULT,
                    GAME_CONTENT_DUNGEON | GAME_CONTENT_TREASURE_VAULT
                ) | (GAME_CONTENT_DUNGEON, GAME_CONTENT_TREASURE_VAULT)
            );
            if !treasure_pair || treasure_supported {
                builder = allow_horizontal(builder, first, second);
            }
        }
        for &surface in &SURFACE_CONTENT {
            builder = builder.allow_bidirectional(first, Direction3::PosY, surface);
        }
    }

    builder.build().expect("frontier content catalog is internally valid")
}

fn allow_horizontal(
    mut builder: super::ContentCatalogBuilder,
    first: ContentId,
    second: ContentId,
) -> super::ContentCatalogBuilder {
    builder = builder.allow_bidirectional(first, Direction3::PosX, second);
    builder = builder.allow_bidirectional(second, Direction3::PosX, first);
    builder = builder.allow_bidirectional(first, Direction3::PosZ, second);
    builder.allow_bidirectional(second, Direction3::PosZ, first)
}
