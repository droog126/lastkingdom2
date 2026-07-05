use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use block_mesh::{
    GreedyQuadsBuffer, MergeVoxel, RIGHT_HANDED_Y_UP_CONFIG, Voxel, VoxelVisibility, greedy_quads,
};
use ndshape::{ConstShape, ConstShape3u32};

use lk2_core::world::{BlockType, World as GameWorld};

type ChunkShape = ConstShape3u32<97, 97, 97>;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug)]
struct Vox(u8);

impl Voxel for Vox {
    fn get_visibility(&self) -> VoxelVisibility {
        if self.0 == 0 {
            VoxelVisibility::Empty
        } else {
            VoxelVisibility::Opaque
        }
    }
}

impl MergeVoxel for Vox {
    type MergeValue = Vox;

    fn merge_value(&self) -> Self::MergeValue {
        *self
    }
}

pub struct BlockTypeMesh {
    pub block_type: BlockType,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl BlockTypeMesh {
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn to_bevy_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.insert_indices(Indices::U32(self.indices.clone()));
        mesh
    }
}

fn quads_to_mesh_data(
    buffer: &GreedyQuadsBuffer,
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let faces = &RIGHT_HANDED_Y_UP_CONFIG.faces;

    for (face, quads) in faces.iter().zip(buffer.quads.groups.iter()) {
        for quad in quads.iter() {
            let corners = face.quad_mesh_positions(quad, 1.0);
            let normal_arr = face.quad_mesh_normals();
            let base = positions.len() as u32;
            for i in 0..4 {
                positions.push(corners[i]);
                normals.push(normal_arr[i]);
                uvs.push([0.0, 0.0]);
            }

            let idx = face.quad_mesh_indices(base);
            indices.extend_from_slice(&idx);
        }
    }
    (positions, normals, uvs, indices)
}

pub fn greedy_mesh_for_type_aabb(
    world: &GameWorld,
    target: BlockType,
    min: [i32; 3],
    max: [i32; 3],
) -> BlockTypeMesh {
    let size_x = (max[0] - min[0]) as u32;
    let size_y = (max[1] - min[1]) as u32;
    let size_z = (max[2] - min[2]) as u32;

    let size = ChunkShape::SIZE as usize;
    let mut voxels: Vec<Vox> = vec![Vox(0); size];
    for z in 0..size_z {
        for y in 0..size_y {
            for x in 0..size_x {
                let i = ChunkShape::linearize([x, y, z]) as usize;
                let wx = min[0] + x as i32;
                let wy = min[1] + y as i32;
                let wz = min[2] + z as i32;
                let b = world.get(wx, wy, wz);
                voxels[i] = if b == target { Vox(1) } else { Vox(0) };
            }
        }
    }

    let mut buffer = GreedyQuadsBuffer::new(size);
    let chunk_max = [size_x, size_y, size_z];
    greedy_quads(
        &voxels,
        &ChunkShape {},
        [0, 0, 0],
        chunk_max,
        &RIGHT_HANDED_Y_UP_CONFIG.faces,
        &mut buffer,
    );

    let (positions, normals, uvs, indices) = quads_to_mesh_data(&buffer);

    BlockTypeMesh { block_type: target, positions, normals, uvs, indices }
}

const RENDERABLE_TYPES: &[BlockType] = &[
    BlockType::Dirt,
    BlockType::Stone,
    BlockType::Sand,
    BlockType::Snow,
    BlockType::Leaves,
    BlockType::Water,
    BlockType::Wood,
    BlockType::IronOre,
    BlockType::SunstoneOre,
    BlockType::FrostcoreOre,
    BlockType::LivingRoot,
    BlockType::BerryThicket,
];

pub fn build_all_terrain_meshes_aabb(
    world: &GameWorld,
    min: [i32; 3],
    max: [i32; 3],
) -> Vec<BlockTypeMesh> {
    RENDERABLE_TYPES
        .iter()
        .map(|&t| greedy_mesh_for_type_aabb(world, t, min, max))
        .filter(|m| !m.is_empty())
        .collect()
}
