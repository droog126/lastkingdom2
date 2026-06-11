//! Greedy Mesh + Trimesh 碰撞 — 一次生成全地形，按 block type 分 mesh
//!
//! 之前每个 solid 块一个 entity（3000+ 个）→ 现在每 block type 一个 mesh（~12 个）。
//!
//! 算法：
//! 1. 对每个 renderable block type T，构建 voxel 数组 (1=T, 0=其他)
//! 2. block_mesh::greedy_quads → 合并所有相邻同向面
//! 3. 输出 Bevy Mesh（视觉）+ 顶点/索引（avian3d Collider::trimesh 碰撞）
//!
//! 性能：96³ 全地形，单 type 顶多 ~100 块，greedy 后 ~20 quad。Trimesh 顶多 ~120 三角形。
//! 视觉 z-fighting 风险：用 backface culling 解决（T 的"背向"面在 T 邻居 mesh 上有反向法线，
//!  被 bevy 默认 cull_mode=Back 剔除）。
//!
//! 后续可优化：
//! - 局部更新（玩家挖一个方块只重算所在 chunk）
//! - 列合并 Cuboid 替代 Trimesh（碰撞性能更好）

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use block_mesh::{
    GreedyQuadsBuffer, MergeVoxel, RIGHT_HANDED_Y_UP_CONFIG, Voxel, VoxelVisibility, greedy_quads,
};
use ndshape::{ConstShape, ConstShape3u32};

use lk2_core::world::{BlockType, World as GameWorld};

/// 玩家周围的 AABB chunk：41 × 41 × 41（半径 20 + 1 padding）。
/// 渲染只算这 41³ 范围（68921 cells = ~70KB，stack 安全）。
/// 玩家移动时改变 AABB 的 min/max，世界无限延伸。

/// 41³ 留 1 格 kernel padding（block-mesh 的 greedy_quads 内部需要）。
type ChunkShape = ConstShape3u32<41, 41, 41>;

/// Voxel：0=空气，1=实心（此 block type）
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

/// 一个 block type 的 greedy mesh 输出
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

    /// 转 bevy Mesh（视觉）
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

/// 把 block_mesh 的 quads 转成 bevy 的 positions/normals/uvs/indices
fn quads_to_mesh_data(
    buffer: &GreedyQuadsBuffer,
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let faces = &RIGHT_HANDED_Y_UP_CONFIG.faces;
    // faces 是 [OrientedBlockFace; 6]，buffer.quads.groups 也是 [Vec<UnorientedQuad>; 6]
    for (face, quads) in faces.iter().zip(buffer.quads.groups.iter()) {
        for quad in quads.iter() {
            let corners = face.quad_mesh_positions(quad, 1.0); // [[f32;3]; 4]
            let normal_arr = face.quad_mesh_normals(); // [[f32;3]; 4] (其实是 [f32; 3] 重复 4 次)
            let base = positions.len() as u32;
            for i in 0..4 {
                positions.push(corners[i]);
                normals.push(normal_arr[i]);
                uvs.push([0.0, 0.0]);
            }
            // quad_mesh_indices 已经是 [u32; 6]（2 个三角形）
            let idx = face.quad_mesh_indices(base);
            indices.extend_from_slice(&idx);
        }
    }
    (positions, normals, uvs, indices)
}

/// 对单个 block type 跑 greedy meshing（玩家周围 AABB，41³ 范围）
/// `min` 和 `max` 是世界坐标，max - min 应 = (40, 40, 40) 或更小（kernel padding 留 1 格）
pub fn greedy_mesh_for_type_aabb(
    world: &GameWorld,
    target: BlockType,
    min: [i32; 3],
    max: [i32; 3],
) -> BlockTypeMesh {
    // 局部坐标范围：0..(max-min)
    let size_x = (max[0] - min[0]) as u32;
    let size_y = (max[1] - min[1]) as u32;
    let size_z = (max[2] - min[2]) as u32;

    // 用 Vec 放堆上：41³ = 68921 cells = ~70KB，stack 安全
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
    // [size, size+1) 范围保持 Vox(0) = 空气，作为 kernel padding

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

/// 所有 renderable block types（不含 Air）
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

/// 对所有 renderable block types 跑 greedy meshing（玩家周围 AABB）
/// `min` / `max` 是世界坐标
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
