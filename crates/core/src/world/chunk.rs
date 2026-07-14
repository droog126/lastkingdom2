use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::BlockType;

pub const TERRAIN_CHUNK_SIZE: i32 = 16;
const TERRAIN_CHUNK_VOLUME: usize =
    (TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE) as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainEdit {
    pub position: [i32; 3],
    pub from: BlockType,
    pub to: BlockType,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TerrainChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl TerrainChunkCoord {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn origin(self) -> [i32; 3] {
        [
            self.x * TERRAIN_CHUNK_SIZE,
            self.y * TERRAIN_CHUNK_SIZE,
            self.z * TERRAIN_CHUNK_SIZE,
        ]
    }

    pub fn from_block(position: [i32; 3]) -> Self {
        Self {
            x: position[0].div_euclid(TERRAIN_CHUNK_SIZE),
            y: position[1].div_euclid(TERRAIN_CHUNK_SIZE),
            z: position[2].div_euclid(TERRAIN_CHUNK_SIZE),
        }
    }

    pub fn local_position(position: [i32; 3]) -> [i32; 3] {
        [
            position[0].rem_euclid(TERRAIN_CHUNK_SIZE),
            position[1].rem_euclid(TERRAIN_CHUNK_SIZE),
            position[2].rem_euclid(TERRAIN_CHUNK_SIZE),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainChunk {
    pub coord: TerrainChunkCoord,
    pub revision: u64,
    blocks: Box<[BlockType]>,
}

impl TerrainChunk {
    pub fn empty(coord: TerrainChunkCoord) -> Self {
        Self {
            coord,
            revision: 0,
            blocks: vec![BlockType::Air; TERRAIN_CHUNK_VOLUME].into_boxed_slice(),
        }
    }

    pub fn get(&self, local: [i32; 3]) -> Option<BlockType> {
        self.index(local).map(|index| self.blocks[index])
    }

    pub fn set(&mut self, local: [i32; 3], block: BlockType) -> Option<BlockType> {
        let index = self.index(local)?;
        let previous = self.blocks[index];
        if previous != block {
            self.blocks[index] = block;
            self.revision = self.revision.saturating_add(1);
        }
        Some(previous)
    }

    pub fn blocks(&self) -> &[BlockType] {
        &self.blocks
    }

    fn index(&self, [x, y, z]: [i32; 3]) -> Option<usize> {
        if !(0..TERRAIN_CHUNK_SIZE).contains(&x)
            || !(0..TERRAIN_CHUNK_SIZE).contains(&y)
            || !(0..TERRAIN_CHUNK_SIZE).contains(&z)
        {
            return None;
        }
        Some(((y * TERRAIN_CHUNK_SIZE + z) * TERRAIN_CHUNK_SIZE + x) as usize)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainChunkStore {
    chunks: HashMap<TerrainChunkCoord, TerrainChunk>,
}

impl TerrainChunkStore {
    pub fn get(&self, coord: TerrainChunkCoord) -> Option<&TerrainChunk> {
        self.chunks.get(&coord)
    }

    pub fn get_mut(&mut self, coord: TerrainChunkCoord) -> Option<&mut TerrainChunk> {
        self.chunks.get_mut(&coord)
    }

    pub fn load_empty(&mut self, coord: TerrainChunkCoord) -> &mut TerrainChunk {
        self.chunks
            .entry(coord)
            .or_insert_with(|| TerrainChunk::empty(coord))
    }

    pub fn unload(&mut self, coord: TerrainChunkCoord) -> Option<TerrainChunk> {
        self.chunks.remove(&coord)
    }

    pub fn insert(&mut self, coord: TerrainChunkCoord, chunk: TerrainChunk) -> Option<TerrainChunk> {
        self.chunks.insert(coord, chunk)
    }

    pub fn get_block(&self, position: [i32; 3]) -> Option<BlockType> {
        let coord = TerrainChunkCoord::from_block(position);
        self.get(coord)
            .and_then(|chunk| chunk.get(TerrainChunkCoord::local_position(position)))
    }

    pub fn set_block(&mut self, position: [i32; 3], block: BlockType) -> Option<BlockType> {
        let coord = TerrainChunkCoord::from_block(position);
        self.load_empty(coord)
            .set(TerrainChunkCoord::local_position(position), block)
    }

    pub fn loaded_coords(&self) -> impl Iterator<Item = TerrainChunkCoord> + '_ {
        self.chunks.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_world_positions_use_euclidean_chunk_coordinates() {
        let coord = TerrainChunkCoord::from_block([-1, -16, 32]);
        assert_eq!(coord, TerrainChunkCoord::new(-1, -1, 2));
        assert_eq!(TerrainChunkCoord::local_position([-1, -16, 32]), [15, 0, 0]);
    }

    #[test]
    fn chunk_revision_changes_only_when_material_changes() {
        let mut chunk = TerrainChunk::empty(TerrainChunkCoord::new(1, 2, 3));
        assert_eq!(chunk.get([15, 15, 15]), Some(BlockType::Air));
        assert_eq!(chunk.set([15, 15, 15], BlockType::Stone), Some(BlockType::Air));
        assert_eq!(chunk.revision, 1);
        assert_eq!(chunk.set([15, 15, 15], BlockType::Stone), Some(BlockType::Stone));
        assert_eq!(chunk.revision, 1);
        assert_eq!(chunk.set([16, 0, 0], BlockType::Stone), None);
    }

    #[test]
    fn chunk_store_handles_world_coordinates_across_boundaries() {
        let mut store = TerrainChunkStore::default();
        assert_eq!(store.get_block([-1, 0, 16]), None);
        assert_eq!(store.set_block([-1, 0, 16], BlockType::IronOre), Some(BlockType::Air));
        assert_eq!(store.get_block([-1, 0, 16]), Some(BlockType::IronOre));
        assert_eq!(store.loaded_coords().count(), 1);
        assert_eq!(store.unload(TerrainChunkCoord::new(-1, 0, 1)).unwrap().revision, 1);
    }
}
