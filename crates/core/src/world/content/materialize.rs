use super::{
    ContentId, ContentSolveError, ContentVolume, GAME_CONTENT_CAVERN, GAME_CONTENT_DUNGEON,
    GAME_CONTENT_EMPTY, GAME_CONTENT_MONSTER_TERRITORY, GAME_CONTENT_SETTLEMENT,
    GAME_CONTENT_TREASURE_VAULT, GAME_CONTENT_VERTICAL_PASSAGE, GAME_CONTENT_WILDERNESS,
    GameContentTheme, GameContentVolumeConfig, game_content_catalog, generate_game_content_volume,
};
use crate::constant::SEA_LEVEL;
use crate::world::{BlockType, World, player_body_clear};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fmt;

pub const CONTENT_CELL_SIZE: i32 = 14;
const CONTENT_HORIZONTAL_CELLS: usize = 5;
const CONTENT_LAYERS: usize = 3;
const CONTENT_HALF_SPAN: i32 = CONTENT_CELL_SIZE * (CONTENT_HORIZONTAL_CELLS as i32 / 2);
const SURFACE_BLOCK_Y: i32 = SEA_LEVEL + 2;
const SURFACE_FOOT_Y: i32 = SURFACE_BLOCK_Y + 1;
const DUNGEON_FLOOR_Y: i32 = SEA_LEVEL - 4;
const DUNGEON_FOOT_Y: i32 = DUNGEON_FLOOR_Y + 1;
const ROOM_HALF_SIZE: i32 = 4;
const ROOM_WALL_OFFSET: i32 = ROOM_HALF_SIZE + 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentAnchor {
    pub content: ContentId,
    pub cell: [usize; 3],
    pub block_position: [i32; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedContent {
    pub volume: ContentVolume,
    pub origin: [i32; 3],
    pub cell_size: i32,
    pub anchors: Vec<ContentAnchor>,
    pub settlement_position: [i32; 3],
    pub monster_position: [i32; 3],
    pub entrance_position: [i32; 3],
    pub treasure_position: [i32; 3],
}

impl MaterializedContent {
    pub fn anchor(&self, content: ContentId) -> Option<&ContentAnchor> {
        self.anchors.iter().find(|anchor| anchor.content == content)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentMaterializeError {
    WorldTooSmall { size: i32, minimum: i32 },
    Solve(ContentSolveError),
    MissingRequiredContent(ContentId),
}

impl fmt::Display for ContentMaterializeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorldTooSmall { size, minimum } => {
                write!(
                    formatter,
                    "world size {size} is too small for content; need at least {minimum}"
                )
            }
            Self::Solve(error) => write!(formatter, "3D content solve failed: {error}"),
            Self::MissingRequiredContent(content) => {
                write!(formatter, "3D content is missing required id {}", content.0)
            }
        }
    }
}

impl std::error::Error for ContentMaterializeError {}

impl From<ContentSolveError> for ContentMaterializeError {
    fn from(value: ContentSolveError) -> Self {
        Self::Solve(value)
    }
}

pub fn generate_and_materialize_content(
    world: &mut World,
    seed: u64,
) -> Result<MaterializedContent, ContentMaterializeError> {
    let minimum_size = CONTENT_HALF_SPAN * 2 + ROOM_WALL_OFFSET * 2 + 2;
    if world.size < minimum_size {
        return Err(ContentMaterializeError::WorldTooSmall {
            size: world.size,
            minimum: minimum_size,
        });
    }

    let dimensions = [
        CONTENT_HORIZONTAL_CELLS,
        CONTENT_LAYERS,
        CONTENT_HORIZONTAL_CELLS,
    ];
    let catalog = game_content_catalog(GameContentTheme::Frontier);
    let volume = generate_game_content_volume(
        &catalog,
        &GameContentVolumeConfig::new(dimensions, seed ^ 0xC017_E17),
    )?;
    let world_center = world.size / 2;
    let origin = [
        world_center - CONTENT_HALF_SPAN,
        DUNGEON_FLOOR_Y,
        world_center - CONTENT_HALF_SPAN,
    ];
    let mut anchors = Vec::new();

    for y in 0..dimensions[1] {
        for z in 0..dimensions[2] {
            for x in 0..dimensions[0] {
                let cell = [x, y, z];
                let Some(content) = volume.get(cell) else {
                    continue;
                };
                if content == GAME_CONTENT_EMPTY {
                    continue;
                }
                let center = cell_center(origin, cell);
                let foot_y = if y == 0 {
                    DUNGEON_FOOT_Y
                } else {
                    SURFACE_FOOT_Y
                };
                anchors.push(ContentAnchor {
                    content,
                    cell,
                    block_position: [center[0], foot_y, center[2]],
                });
            }
        }
    }

    materialize_underground(world, &volume, origin);
    materialize_surface(world, &volume, origin, seed);

    let center_cell = [dimensions[0] / 2, 1, dimensions[2] / 2];
    let monster_cell = [1, 1, 1];
    let treasure_cell = [dimensions[0] - 2, 0, dimensions[2] - 2];
    let settlement_center = cell_center(origin, center_cell);
    let monster_center = cell_center(origin, monster_cell);
    let treasure_center = cell_center(origin, treasure_cell);
    let settlement_position = [settlement_center[0], SURFACE_FOOT_Y, settlement_center[2]];
    let monster_position = [monster_center[0], SURFACE_FOOT_Y, monster_center[2]];
    let entrance_position = [
        settlement_center[0] + 5,
        SURFACE_FOOT_Y,
        settlement_center[2],
    ];
    let treasure_position = [treasure_center[0], DUNGEON_FOOT_Y, treasure_center[2]];

    for &(required, cell) in &[
        (GAME_CONTENT_SETTLEMENT, center_cell),
        (GAME_CONTENT_MONSTER_TERRITORY, monster_cell),
        (GAME_CONTENT_TREASURE_VAULT, treasure_cell),
    ] {
        if volume.get(cell) != Some(required) {
            return Err(ContentMaterializeError::MissingRequiredContent(required));
        }
    }

    Ok(MaterializedContent {
        volume,
        origin,
        cell_size: CONTENT_CELL_SIZE,
        anchors,
        settlement_position,
        monster_position,
        entrance_position,
        treasure_position,
    })
}

pub fn content_route_is_walkable(world: &World, content: &MaterializedContent) -> bool {
    let start = content.entrance_position;
    let goal = content.treasure_position;
    if !is_standable(world, start) || !is_standable(world, goal) {
        return false;
    }

    let horizontal_span = CONTENT_HALF_SPAN + ROOM_WALL_OFFSET + 2;
    let center_x = content.settlement_position[0];
    let center_z = content.settlement_position[2];
    let min_x = center_x - horizontal_span;
    let max_x = center_x + horizontal_span;
    let min_z = center_z - horizontal_span;
    let max_z = center_z + horizontal_span;
    let min_y = DUNGEON_FOOT_Y;
    let max_y = SURFACE_FOOT_Y;
    let mut queue = VecDeque::from([start]);
    let mut visited = HashSet::from([start]);

    while let Some(current) = queue.pop_front() {
        if current == goal {
            return true;
        }
        for [dx, dz] in [[-1, 0], [1, 0], [0, -1], [0, 1]] {
            for dy in -1..=1 {
                let candidate = [current[0] + dx, current[1] + dy, current[2] + dz];
                if candidate[0] < min_x
                    || candidate[0] > max_x
                    || candidate[1] < min_y
                    || candidate[1] > max_y
                    || candidate[2] < min_z
                    || candidate[2] > max_z
                    || visited.contains(&candidate)
                    || !is_standable(world, candidate)
                {
                    continue;
                }
                visited.insert(candidate);
                queue.push_back(candidate);
            }
        }
    }
    false
}

fn is_standable(world: &World, foot: [i32; 3]) -> bool {
    world.get(foot[0], foot[1] - 1, foot[2]).is_solid()
        && player_body_clear(world, foot[0], foot[1], foot[2])
}

fn cell_center(origin: [i32; 3], cell: [usize; 3]) -> [i32; 3] {
    [
        origin[0] + cell[0] as i32 * CONTENT_CELL_SIZE,
        origin[1] + cell[1] as i32 * CONTENT_CELL_SIZE,
        origin[2] + cell[2] as i32 * CONTENT_CELL_SIZE,
    ]
}

fn materialize_underground(world: &mut World, volume: &ContentVolume, origin: [i32; 3]) {
    let [width, _, depth] = volume.dimensions;
    for z in 0..depth {
        for x in 0..width {
            let cell = [x, 0, z];
            if volume.get(cell).is_none() {
                continue;
            }
            let center = cell_center(origin, cell);
            build_underground_room(world, center);
        }
    }

    for z in 0..depth {
        for x in 0..width {
            let center = cell_center(origin, [x, 0, z]);
            if x + 1 < width {
                let neighbor = cell_center(origin, [x + 1, 0, z]);
                carve_horizontal_corridor(world, center, neighbor);
            }
            if z + 1 < depth {
                let neighbor = cell_center(origin, [x, 0, z + 1]);
                carve_horizontal_corridor(world, center, neighbor);
            }
        }
    }

    let center = cell_center(origin, [width / 2, 0, depth / 2]);
    build_stairway(world, center);

    for z in 0..depth {
        for x in 0..width {
            let cell = [x, 0, z];
            let Some(content) = volume.get(cell) else {
                continue;
            };
            decorate_underground_room(world, cell_center(origin, cell), content);
        }
    }
}

fn build_underground_room(world: &mut World, center: [i32; 3]) {
    let cx = center[0];
    let cz = center[2];
    for z in (cz - ROOM_WALL_OFFSET)..=(cz + ROOM_WALL_OFFSET) {
        for x in (cx - ROOM_WALL_OFFSET)..=(cx + ROOM_WALL_OFFSET) {
            let boundary = (x - cx).abs() == ROOM_WALL_OFFSET || (z - cz).abs() == ROOM_WALL_OFFSET;
            world.set(x, DUNGEON_FLOOR_Y, z, BlockType::Stone);
            world.set(x, DUNGEON_FLOOR_Y + 4, z, BlockType::Stone);
            for y in (DUNGEON_FLOOR_Y + 1)..=(DUNGEON_FLOOR_Y + 3) {
                world.set(
                    x,
                    y,
                    z,
                    if boundary {
                        BlockType::Stone
                    } else {
                        BlockType::Air
                    },
                );
            }
        }
    }
}

fn decorate_underground_room(world: &mut World, center: [i32; 3], content: ContentId) {
    let cx = center[0];
    let cz = center[2];
    if content == GAME_CONTENT_DUNGEON {
        for [dx, dz] in [[-3, -3], [3, -3], [-3, 3], [3, 3]] {
            for y in (DUNGEON_FLOOR_Y + 1)..=(DUNGEON_FLOOR_Y + 3) {
                world.set(cx + dx, y, cz + dz, BlockType::Wood);
            }
        }
    } else if content == GAME_CONTENT_TREASURE_VAULT {
        for [dx, dz] in [[0, 0], [-1, 0], [1, 0], [0, -1], [0, 1]] {
            let treasure = if dx == 0 && dz == 0 {
                BlockType::SunstoneOre
            } else {
                BlockType::FrostcoreOre
            };
            world.set(cx + dx, DUNGEON_FLOOR_Y, cz + dz, treasure);
        }
    } else if content == GAME_CONTENT_CAVERN {
        world.set(cx - 2, DUNGEON_FLOOR_Y + 1, cz + 2, BlockType::LivingRoot);
        world.set(cx + 2, DUNGEON_FLOOR_Y + 1, cz - 2, BlockType::LivingRoot);
    }
}

fn carve_horizontal_corridor(world: &mut World, start: [i32; 3], end: [i32; 3]) {
    if start[0] != end[0] {
        let min_x = start[0].min(end[0]);
        let max_x = start[0].max(end[0]);
        for x in min_x..=max_x {
            carve_corridor_column(world, x, start[2], true);
        }
    } else {
        let min_z = start[2].min(end[2]);
        let max_z = start[2].max(end[2]);
        for z in min_z..=max_z {
            carve_corridor_column(world, start[0], z, false);
        }
    }
}

fn carve_corridor_column(world: &mut World, x: i32, z: i32, along_x: bool) {
    for offset in -1..=1 {
        let (column_x, column_z) = if along_x {
            (x, z + offset)
        } else {
            (x + offset, z)
        };
        world.set(column_x, DUNGEON_FLOOR_Y, column_z, BlockType::Stone);
        world.set(column_x, DUNGEON_FLOOR_Y + 4, column_z, BlockType::Stone);
        for y in (DUNGEON_FLOOR_Y + 1)..=(DUNGEON_FLOOR_Y + 3) {
            world.set(column_x, y, column_z, BlockType::Air);
        }
    }
}

fn build_stairway(world: &mut World, center: [i32; 3]) {
    let cx = center[0];
    let cz = center[2];
    let depth = SURFACE_BLOCK_Y - DUNGEON_FLOOR_Y;
    for step in 0..=depth {
        let x = cx + 5 - step;
        let floor_y = SURFACE_BLOCK_Y - step;
        for z in (cz - 1)..=(cz + 1) {
            world.set(x, floor_y, z, BlockType::Stone);
            for y in (floor_y + 1)..=(floor_y + 3) {
                world.set(x, y, z, BlockType::Air);
            }
        }
    }

    for z in [cz - 2, cz + 2] {
        for y in SURFACE_FOOT_Y..=(SURFACE_FOOT_Y + 3) {
            world.set(cx + 6, y, z, BlockType::Wood);
        }
    }
    for z in (cz - 2)..=(cz + 2) {
        world.set(cx + 6, SURFACE_FOOT_Y + 3, z, BlockType::Wood);
    }
}

fn materialize_surface(world: &mut World, volume: &ContentVolume, origin: [i32; 3], seed: u64) {
    let [width, _, depth] = volume.dimensions;
    for z in 0..depth {
        for x in 0..width {
            let cell = [x, 1, z];
            let Some(content) = volume.get(cell) else {
                continue;
            };
            let center = cell_center(origin, cell);
            if content == GAME_CONTENT_SETTLEMENT {
                build_settlement(world, center);
            } else if content == GAME_CONTENT_MONSTER_TERRITORY {
                build_monster_territory(world, center);
            } else if content == GAME_CONTENT_VERTICAL_PASSAGE {
                build_surface_well(world, center);
            } else if content == GAME_CONTENT_WILDERNESS {
                build_wilderness_landmark(world, center, seed);
            }
        }
    }
}

fn build_settlement(world: &mut World, center: [i32; 3]) {
    let cx = center[0];
    let cz = center[2];
    for offset in -6_i32..=6 {
        if offset.abs() > 1 {
            world.set(cx + offset, SURFACE_BLOCK_Y, cz, BlockType::Stone);
            world.set(cx, SURFACE_BLOCK_Y, cz + offset, BlockType::Stone);
        }
    }
    world.set(cx, SURFACE_BLOCK_Y, cz, BlockType::Grass);

    for [dx, dz] in [[-5, -5], [5, -5], [-5, 5]] {
        build_hut(world, cx + dx, cz + dz, -dx, -dz);
    }
    for z in (cz - 2)..=(cz + 2) {
        for x in (cx - 2)..=(cx + 2) {
            for y in SURFACE_FOOT_Y..=(SURFACE_FOOT_Y + 3) {
                world.set(x, y, z, BlockType::Air);
            }
        }
    }
}

fn build_hut(world: &mut World, cx: i32, cz: i32, toward_x: i32, toward_z: i32) {
    let half = 2;
    for z in (cz - half)..=(cz + half) {
        for x in (cx - half)..=(cx + half) {
            let boundary = (x - cx).abs() == half || (z - cz).abs() == half;
            for y in SURFACE_FOOT_Y..=(SURFACE_FOOT_Y + 2) {
                world.set(
                    x,
                    y,
                    z,
                    if boundary {
                        BlockType::Wood
                    } else {
                        BlockType::Air
                    },
                );
            }
            world.set(x, SURFACE_FOOT_Y + 3, z, BlockType::Leaves);
        }
    }

    let (door_x, door_z) = if toward_x.abs() >= toward_z.abs() {
        (cx + toward_x.signum() * half, cz)
    } else {
        (cx, cz + toward_z.signum() * half)
    };
    world.set(door_x, SURFACE_FOOT_Y, door_z, BlockType::Air);
    world.set(door_x, SURFACE_FOOT_Y + 1, door_z, BlockType::Air);
}

fn build_monster_territory(world: &mut World, center: [i32; 3]) {
    let cx = center[0];
    let cz = center[2];
    for dz in -5_i32..=5 {
        for dx in -5_i32..=5 {
            let distance_sq = dx * dx + dz * dz;
            if (16..=25).contains(&distance_sq) {
                world.set(cx + dx, SURFACE_BLOCK_Y, cz + dz, BlockType::Stone);
            }
        }
    }
    for [dx, dz] in [[-4, 0], [4, 0], [0, -4], [0, 4]] {
        for y in SURFACE_FOOT_Y..=(SURFACE_FOOT_Y + 4) {
            world.set(cx + dx, y, cz + dz, BlockType::SunstoneOre);
        }
    }
}

fn build_surface_well(world: &mut World, center: [i32; 3]) {
    let cx = center[0];
    let cz = center[2];
    for dz in -2_i32..=2 {
        for dx in -2_i32..=2 {
            let edge = dx.abs() == 2 || dz.abs() == 2;
            if edge {
                world.set(cx + dx, SURFACE_FOOT_Y, cz + dz, BlockType::Stone);
            }
        }
    }
}

fn build_wilderness_landmark(world: &mut World, center: [i32; 3], seed: u64) {
    let cx = center[0];
    let cz = center[2];
    let shift =
        ((seed ^ (cx as u64).wrapping_mul(31) ^ (cz as u64).wrapping_mul(17)) % 3) as i32 - 1;
    let tree_x = cx + shift;
    let tree_z = cz - shift;
    for y in SURFACE_FOOT_Y..=(SURFACE_FOOT_Y + 3) {
        world.set(tree_x, y, tree_z, BlockType::Wood);
    }
    for dz in -2_i32..=2 {
        for dx in -2_i32..=2 {
            if dx.abs() + dz.abs() <= 3 {
                world.set(
                    tree_x + dx,
                    SURFACE_FOOT_Y + 4,
                    tree_z + dz,
                    BlockType::Leaves,
                );
            }
        }
    }
}
