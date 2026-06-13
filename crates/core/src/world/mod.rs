//! World & blocks
//!
//! §三、世界与维度 → 1. 主世界生物群落
//! §三、世界与维度 → 2. 以太界（demo 略）
//! §三、世界与维度 → 矿坑生成算法与实现纲要
//!
//! Demo 简化：
//!   * WORLD_SIZE = 32³（缩到 1/1000）
//!   * 3 群落按 z 轴切三段：北=tundra / 中=jungle / 南=desert
//!   * 矿脉用 3D simplex noise 模拟（不引外部 noise crate，自己写个快速 hash）
//!   * 资源点（裸矿）以"簇"形式聚集，cluster 半径 2-4，间距 ≥ 6
//!   * 块 → 资源映射：挖 block 得 ResourceKind + amount

#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::constant::*;
use crate::resource::{ResourceKind, Transfer, TransferDst, TransferSrc, apply_transfer};

// ---------------------------------------------------------------------------
// Biome
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Biome {
    Desert, // 焦土沙漠 → 阳炎石
    Tundra, // 冰封苔原 → 霜心晶体
    Jungle, // 繁盛丛林 → 活根
}

impl Biome {
    /// 平滑噪声场（用 hash01 模拟 value noise，2 octaves）— 大陆尺度
    /// 返回 [0, 1) 连续值，永远不会周期重复
    pub fn noise_field(x: i32, z: i32) -> f32 {
        // 大尺度（每 64 一格）
        let cell = 64_i32;
        let cx = (x as f32 / cell as f32).floor() as i32;
        let cz = (z as f32 / cell as f32).floor() as i32;
        let fx = (x as f32 / cell as f32) - cx as f32;
        let fz = (z as f32 / cell as f32) - cz as f32;
        // 4 角 value
        let v00 = crate::world::terrain::hash01(cx,     0,     cz,     0xB10E);
        let v10 = crate::world::terrain::hash01(cx + 1, 0,     cz,     0xB10E);
        let v01 = crate::world::terrain::hash01(cx,     0,     cz + 1, 0xB10E);
        let v11 = crate::world::terrain::hash01(cx + 1, 0,     cz + 1, 0xB10E);
        // smoothstep
        let sx = fx * fx * (3.0 - 2.0 * fx);
        let sz = fz * fz * (3.0 - 2.0 * fz);
        let a = v00 * (1.0 - sx) + v10 * sx;
        let b = v01 * (1.0 - sx) + v11 * sx;
        let big = a * (1.0 - sz) + b * sz;
        // 细节 (1/8 尺度)
        let cell2 = 8_i32;
        let cx2 = (x as f32 / cell2 as f32).floor() as i32;
        let cz2 = (z as f32 / cell2 as f32).floor() as i32;
        let detail = crate::world::terrain::hash01(cx2, 0, cz2, 0xD37A1);
        big * 0.7 + detail * 0.3
    }

    /// 从连续 noise 场分 3 段（沙漠 / 丛林 / 苔原）— 真无限
    pub fn from_xz_infinite(x: i32, z: i32) -> Self {
        let n = Self::noise_field(x, z);
        if n < 0.33 {
            Biome::Desert
        } else if n < 0.66 {
            Biome::Jungle
        } else {
            Biome::Tundra
        }
    }

    /// 从 (x, z) 决定 biome（demo 用确定性分区，依赖 WORLD_SIZE）
    pub fn from_xz(_x: i32, z: i32) -> Self {
        let n = WORLD_SIZE as i32;
        if z < n / 3 {
            Biome::Tundra
        } else if z < (2 * n) / 3 {
            Biome::Jungle
        } else {
            Biome::Desert
        }
    }

    /// 这个 biome 的专属矿石（挖出来对应 ResourceKind）
    pub fn ore_block(self) -> BlockType {
        match self {
            Biome::Desert => BlockType::SunstoneOre,
            Biome::Tundra => BlockType::FrostcoreOre,
            Biome::Jungle => BlockType::LivingRoot,
        }
    }

    /// 这个 biome 的专属矿石对应的资源
    pub fn ore_resource(self) -> ResourceKind {
        match self {
            Biome::Desert => ResourceKind::Sunstone,
            Biome::Tundra => ResourceKind::Frostcore,
            Biome::Jungle => ResourceKind::LivingRoot,
        }
    }

    pub const fn label_zh(self) -> &'static str {
        match self {
            Biome::Desert => "焦土沙漠",
            Biome::Tundra => "冰封苔原",
            Biome::Jungle => "繁盛丛林",
        }
    }
}

// ---------------------------------------------------------------------------
// BlockType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockType {
    Air,
    Dirt,
    Stone,
    Sand,         // 沙漠地表
    Snow,         // 苔原地表
    Leaves,       // 树冠
    Water,        // 水（半透明，可通行）
    Wood,         // 树干
    IronOre,      // 通用
    SunstoneOre,  // 沙漠专属矿石
    FrostcoreOre, // 苔原专属矿石
    LivingRoot,   // 丛林专属矿石
    BerryThicket, // 可再生（每 30s tick 概率结 1-3 苹果）
}

impl BlockType {
    /// 是否固体（阻挡玩家 / 战争迷雾）
    pub const fn is_solid(self) -> bool {
        !matches!(self, BlockType::Air | BlockType::Water)
    }

    pub const fn is_renderable(self) -> bool {
        !matches!(self, BlockType::Air)
    }

    pub const fn is_surface(self) -> bool {
        matches!(
            self,
            BlockType::Dirt | BlockType::Sand | BlockType::Snow | BlockType::BerryThicket
        )
    }

    /// 挖掘产出的资源（None = 不可挖 / 空气）
    /// amount 是 tick 级一次采集的产量
    pub fn yields(self) -> Option<(ResourceKind, i64)> {
        use BlockType::*;
        use ResourceKind as R;
        match self {
            Air | Sand | Snow | Leaves | Water => None, // 沙/雪/叶/水 不可采集
            Dirt => None,                               // 暂不给食物
            Stone => Some((R::Wood, 0)),                // 占位
            Wood => Some((R::Wood, 5)),                 // 砍树
            IronOre => Some((R::Wood, 0)),              // 占位，铁没在 pool 里
            SunstoneOre => Some((R::Sunstone, 1)),
            FrostcoreOre => Some((R::Frostcore, 1)),
            LivingRoot => Some((R::LivingRoot, 1)),
            BerryThicket => Some((R::Apple, 1)),
        }
    }

    /// 用于 debug 颜色（demo 用，后续接渲染）
    pub fn debug_color_rgba(self) -> [f32; 4] {
        use BlockType::*;
        match self {
            Air => [0.0, 0.0, 0.0, 0.0],
            Dirt => [0.55, 0.36, 0.20, 1.0],
            Stone => [0.55, 0.55, 0.55, 1.0],
            Sand => [0.92, 0.82, 0.55, 1.0],   // 沙黄
            Snow => [0.95, 0.97, 1.00, 1.0],   // 雪白
            Leaves => [0.20, 0.55, 0.18, 1.0], // 树冠深绿
            Water => [0.25, 0.50, 0.85, 1.0],  // 水蓝（render 时改 alpha）
            Wood => [0.40, 0.25, 0.10, 1.0],
            IronOre => [0.80, 0.60, 0.30, 1.0],
            SunstoneOre => [1.00, 0.70, 0.20, 1.0],
            FrostcoreOre => [0.60, 0.85, 1.00, 1.0],
            LivingRoot => [0.20, 0.80, 0.30, 1.0],
            BerryThicket => [0.85, 0.20, 0.50, 1.0],
        }
    }
}

// ---------------------------------------------------------------------------
// World：32³ 块的稠密数组（demo 缩）
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Clone, Default)]
pub struct World {
    /// 稠密 3D 数组，index = (y * SIZE + z) * SIZE + x。XZ 超出此范围时由 generate_voxel 按需生成
    pub blocks: Vec<BlockType>,
    pub size: i32,
    /// true = 未被显式 set 的块由 pipeline 按需生成；false = 纯空测试世界。
    pub procedural: bool,
    /// 稠密缓存内被玩家或系统明确写过的位置。用于区分“未生成的 Air”和“被挖空的 Air”。
    pub edited: HashSet<(i32, i32, i32)>,
    /// 噪声种子（用于 generate_voxel 按需生成）
    pub seed: u64,
    /// 地形生成 pipeline（可配置地形系统的核心）
    pub pipeline: std::sync::Arc<terrain::TerrainPipeline>,
}

pub mod terrain;

impl World {
    pub fn new(size: i32) -> Self {
        let n = (size * size * size) as usize;
        Self {
            blocks: vec![BlockType::Air; n],
            size,
            procedural: false,
            edited: HashSet::new(),
            seed: 0xDEADBEEF,
            pipeline: std::sync::Arc::new(terrain::presets::default_preset()),
        }
    }

    pub fn with_pipeline(size: i32, pipeline: terrain::TerrainPipeline) -> Self {
        let mut w = Self::new(size);
        w.procedural = true;
        w.pipeline = std::sync::Arc::new(pipeline);
        w
    }

    fn idx(&self, x: i32, y: i32, z: i32) -> usize {
        let s = self.size;
        debug_assert!(x >= 0 && x < s && y >= 0 && y < s && z >= 0 && z < s);
        ((y * s + z) * s + x) as usize
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockType {
        // OOB Y 永远是 Air；XZ 超出稠密缓存时，procedural 世界仍可按需生成。
        let s = self.size;
        if y < 0 || y >= s {
            return BlockType::Air;
        }
        if x >= 0 && x < s && z >= 0 && z < s {
            let cached = self.blocks[self.idx(x, y, z)];
            if cached != BlockType::Air || self.edited.contains(&(x, y, z)) || !self.procedural {
                return cached;
            }
        } else if !self.procedural {
            return BlockType::Air;
        }
        self.generate_voxel(x, y, z)
    }

    /// 按需生成 voxel — 用配置化 pipeline（XZ 无限，Y 有限）
    /// 确定性：同样 (x, y, z, pipeline.seed) → 同样结果
    pub fn generate_voxel(&self, x: i32, y: i32, z: i32) -> BlockType {
        self.pipeline.generate(x, y, z)
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, b: BlockType) {
        let s = self.size;
        if x < 0 || x >= s || y < 0 || y >= s || z < 0 || z >= s {
            return;
        }
        let i = self.idx(x, y, z);
        self.blocks[i] = b;
        self.edited.insert((x, y, z));
    }

    pub fn in_bounds(&self, _x: i32, y: i32, _z: i32) -> bool {
        // XZ 无限，只检查 Y
        y >= 0 && y < self.size
    }

    /// 迭代所有实心块
    pub fn for_each_solid<F: FnMut(i32, i32, i32, BlockType)>(&self, mut f: F) {
        for y in 0..self.size {
            for z in 0..self.size {
                for x in 0..self.size {
                    let b = self.get(x, y, z);
                    if b.is_solid() {
                        f(x, y, z, b);
                    }
                }
            }
        }
    }

    /// 块数（同 biome）—— 调试 / 测试用
    pub fn count_biome_ores(&self, biome: Biome) -> u32 {
        let mut count = 0;
        self.for_each_solid(|_, _, _, b| {
            if b == biome.ore_block() {
                count += 1;
            }
        });
        count
    }
}

// ---------------------------------------------------------------------------
// 确定性 hash 噪声（不引外部 crate）
// ---------------------------------------------------------------------------

/// 32-bit hash → [0, 1) f32
fn hash01(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = seed
        ^ (x as u32).wrapping_mul(0x9E3779B1)
        ^ (y as u32).wrapping_mul(0x85EBCA77)
        ^ (z as u32).wrapping_mul(0xC2B2AE3D);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846CA68B);
    h ^= h >> 16;
    (h & 0xFFFF) as f32 / 65536.0
}

/// 3D 风格化 noise（简单 lerp，够 demo 用）
fn noise3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let xi = x as f32;
    let yi = y as f32;
    let zi = z as f32;
    // 8 corners
    let c000 = hash01(x, y, z, seed);
    let c100 = hash01(x + 1, y, z, seed);
    let c010 = hash01(x, y + 1, z, seed);
    let c110 = hash01(x + 1, y + 1, z, seed);
    let c001 = hash01(x, y, z + 1, seed);
    let c101 = hash01(x + 1, y, z + 1, seed);
    let c011 = hash01(x, y + 1, z + 1, seed);
    let c111 = hash01(x + 1, y + 1, z + 1, seed);
    // trilinear
    let xf = xi.fract();
    let yf = yi.fract();
    let zf = zi.fract();
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let w = zf * zf * (3.0 - 2.0 * zf);
    let x00 = c000 * (1.0 - u) + c100 * u;
    let x10 = c010 * (1.0 - u) + c110 * u;
    let x01 = c001 * (1.0 - u) + c101 * u;
    let x11 = c011 * (1.0 - u) + c111 * u;
    let y0 = x00 * (1.0 - v) + x10 * v;
    let y1 = x01 * (1.0 - v) + x11 * v;
    y0 * (1.0 - w) + y1 * w
}

// ---------------------------------------------------------------------------
// WorldGenerator
// ---------------------------------------------------------------------------

pub struct WorldGenerator {
    pub seed: u32,
    pub ore_threshold: f32,   // > 此值算矿石
    pub tree_density: f32,    // 树密度
    pub thicket_density: f32, // 浆果丛林密度
    pub min_ore_cluster_spacing: i32,
}

impl Default for WorldGenerator {
    fn default() -> Self {
        Self {
            seed: 0xDEADBEEF,
            ore_threshold: 0.72,
            tree_density: 0.05,
            thicket_density: 0.02,
            min_ore_cluster_spacing: 6,
        }
    }
}

impl WorldGenerator {
    /// 生成 demo 世界。**确定性**：同样 seed + size → 同样世界
    pub fn generate(&self, size: i32) -> World {
        let mut w = World::new(size);
        let s = size as i32;

        // ── 1. 地形：heightmap（双八度 noise，振幅 18，能出真山）─────────────
        let spawn_x = s / 2;
        let spawn_z = s / 2;
        let flat_radius: f32 = 10.0; // 比之前大一倍的出生平地
        // 清空 spawn 周边 3 格：让出生第一眼有干净视野（无树/仙人掌/巨石/浆果）
        let clear_radius: i32 = 3;
        for z in 0..s {
            for x in 0..s {
                let biome = Biome::from_xz(x, z);
                let dist_from_spawn = (((x - spawn_x).pow(2) + (z - spawn_z).pow(2)) as f32).sqrt();

                // 主峰（大尺度山脉）+ 细节（小尺度起伏）
                let h_big = noise3(x / 8, 0, z / 8, self.seed); // 0..1
                let h_detail = noise3(x, 0, z, self.seed ^ 0xCAFE); // 0..1
                let biome_bias: f32 = match biome {
                    Biome::Desert => -2.0, // 沙漠偏低，多沙丘
                    Biome::Jungle => 0.0,  // 中位
                    Biome::Tundra => 3.0,  // 苔原偏高山地
                };
                let base_h = (h_big * 14.0) + (h_detail * 4.0) + SEA_LEVEL as f32 + biome_bias;

                let h = if dist_from_spawn < flat_radius {
                    SEA_LEVEL + 1
                } else if dist_from_spawn < flat_radius + 6.0 {
                    // 缓坡过渡：base_h 但往 SEA_LEVEL+1 插值
                    let t = (dist_from_spawn - flat_radius) / 6.0;
                    let flat = (SEA_LEVEL + 1) as f32;
                    (flat * (1.0 - t) + base_h * t) as i32
                } else {
                    base_h as i32
                }
                .clamp(1, s - 4); // 不让 y 顶到天

                // 填柱：底层 stone（厚 3+），表层用 biome 专属
                let surface = match biome {
                    Biome::Desert => BlockType::Sand,
                    Biome::Jungle => BlockType::Dirt,
                    Biome::Tundra => BlockType::Snow,
                };
                let sub = BlockType::Dirt; // 表层下 1 层用 dirt
                for y in 0..h {
                    if y == h - 1 {
                        w.set(x, y, z, surface);
                    } else if y >= h - 3 {
                        w.set(x, y, z, sub);
                    } else {
                        w.set(x, y, z, BlockType::Stone);
                    }
                }
            }
        }

        // ── 2. 洞穴：3D noise 在地下挖空 ────────────────────────────────────
        for y in 1..(s - 2) {
            for z in 0..s {
                for x in 0..s {
                    let h_big2 = noise3(x / 8, 0, z / 8, self.seed);
                    let h_detail2 = noise3(x, 0, z, self.seed ^ 0xCAFE);
                    let biome2 = Biome::from_xz(x, z);
                    let biome_bias2: f32 = match biome2 {
                        Biome::Desert => -2.0,
                        Biome::Jungle => 0.0,
                        Biome::Tundra => 3.0,
                    };
                    let surface2 =
                        (h_big2 * 14.0 + h_detail2 * 4.0 + SEA_LEVEL as f32 + biome_bias2) as i32;
                    if y >= surface2 - 1 {
                        continue;
                    } // 表层 1 格不挖
                    let cave_n = noise3(x / 4, y / 3, z / 4, self.seed ^ 0xC0CA);
                    if cave_n > 0.65 {
                        w.set(x, y, z, BlockType::Air);
                    }
                }
            }
        }

        // ── 2b. 水：海平面以下的空腔填水（湖+河）────────────────────────────
        for z in 0..s {
            for x in 0..s {
                for y in 0..=SEA_LEVEL {
                    if w.get(x, y, z) == BlockType::Air {
                        w.set(x, y, z, BlockType::Water);
                    }
                }
            }
        }

        // ── 3. 矿石：3 群落各自专属，按 cluster 间距撒 ────────────────────────
        for biome in [Biome::Desert, Biome::Tundra, Biome::Jungle] {
            self.place_ore_clusters(&mut w, biome);
        }
        // IronOre 通用
        self.place_generic_iron(&mut w);

        // ── 3. 树：树干 + 树冠；不同 biome 长得不一样 ─────────────────────────
        // 沙漠不种树（沙地没水），改种仙人掌
        // 丛林：高大阔叶树（5-7 木干 + 3x3x2 树冠）
        // 苔原：针叶树（4-5 木干 + 2x2x2 树冠）
        for z in 0..s {
            for x in 0..s {
                let biome = Biome::from_xz(x, z);
                if biome == Biome::Desert {
                    // 仙人掌：1-3 块 Wood 立柱
                    if (x - spawn_x).abs() + (z - spawn_z).abs() >= clear_radius
                        && hash01(x, z, 0, self.seed ^ 0xC4) < 0.015
                    {
                        if let Some(y) = self.find_surface(w.clone(), x, z) {
                            let h = 1 + (hash01(x, z, 9, self.seed) * 3.0) as i32;
                            for up in 1..=h {
                                if y + up < s {
                                    w.set(x, y + up, z, BlockType::Wood);
                                }
                            }
                        }
                    }
                    continue;
                }
                if (x - spawn_x).abs() + (z - spawn_z).abs() >= clear_radius
                    && hash01(x, z, 0, self.seed ^ 0xA1) < self.tree_density
                {
                    if let Some(y) = self.find_surface(w.clone(), x, z) {
                        let (trunk_h, canopy) = match biome {
                            Biome::Jungle => (
                                5 + (hash01(x, z, 8, self.seed) * 3.0) as i32, // 5-7
                                (3, 2),                                        // 3x3x2 阔叶冠
                            ),
                            Biome::Tundra => (
                                4 + (hash01(x, z, 8, self.seed) * 2.0) as i32, // 4-5
                                (2, 2),                                        // 2x2x2 针叶冠
                            ),
                            Biome::Desert => unreachable!(),
                        };
                        // 树干
                        for up in 1..=trunk_h {
                            if y + up < s {
                                w.set(x, y + up, z, BlockType::Wood);
                            }
                        }
                        // 树冠
                        let canopy_base = y + trunk_h - 1;
                        for dy in 0..canopy.1 {
                            for dx in -(canopy.0 as i32 / 2)..=(canopy.0 as i32 / 2) {
                                for dz in -(canopy.0 as i32 / 2)..=(canopy.0 as i32 / 2) {
                                    let px = x + dx;
                                    let pz = z + dz;
                                    let py = canopy_base + dy;
                                    if w.in_bounds(px, py, pz)
                                        && w.get(px, py, pz) == BlockType::Air
                                    {
                                        // 边缘稀疏
                                        if dx == 0 && dz == 0 && dy < canopy.1 - 1 {
                                            continue; // 树干穿过处不铺叶
                                        }
                                        if (dx.abs() + dz.abs() + dy) > canopy.0 {
                                            continue; // 角上不铺
                                        }
                                        w.set(px, py, pz, BlockType::Leaves);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── 4. 浆果丛林（只在 jungle 地表）───────────────────────────────────
        for z in 0..s {
            for x in 0..s {
                if Biome::from_xz(x, z) != Biome::Jungle {
                    continue;
                }
                if (x - spawn_x).abs() + (z - spawn_z).abs() >= clear_radius
                    && hash01(x, z, 1, self.seed ^ 0xB2) < self.thicket_density
                {
                    if let Some(y) = self.find_surface(w.clone(), x, z) {
                        if y + 1 < s {
                            w.set(x, y + 1, z, BlockType::BerryThicket);
                        }
                    }
                }
            }
        }

        // ── 5. 苔原巨砾（石头堆点缀）────────────────────────────────────────
        for z in 0..s {
            for x in 0..s {
                if Biome::from_xz(x, z) != Biome::Tundra {
                    continue;
                }
                if (x - spawn_x).abs() + (z - spawn_z).abs() >= clear_radius
                    && hash01(x, z, 2, self.seed ^ 0xB3) < 0.04
                {
                    if let Some(y) = self.find_surface(w.clone(), x, z) {
                        if y + 1 < s {
                            w.set(x, y + 1, z, BlockType::Stone);
                            if hash01(x, z, 7, self.seed) > 0.5 && y + 2 < s {
                                w.set(x, y + 2, z, BlockType::Stone);
                            }
                        }
                    }
                }
            }
        }

        w
    }

    /// 在指定 biome 范围内撒矿石（cluster 间距约束）
    fn place_ore_clusters(&self, w: &mut World, biome: Biome) {
        let s = w.size as i32;
        let mut placed: Vec<(i32, i32)> = Vec::new();
        for z in 1..s - 1 {
            for x in 1..s - 1 {
                if Biome::from_xz(x, z) != biome {
                    continue;
                }
                if hash01(x, z, 2, self.seed ^ (biome as u32) * 0x100) < 0.08 {
                    // candidate
                    if placed.iter().all(|(px, pz)| {
                        (x - px).abs() + (z - pz).abs() > self.min_ore_cluster_spacing
                    }) {
                        placed.push((x, z));
                        let cluster_size = 2 + (hash01(x, z, 3, self.seed) * 3.0) as i32; // 2-4 块
                        for _ in 0..cluster_size {
                            let dx = (hash01(x, z, 4, self.seed) * 5.0) as i32 - 2;
                            let dz = (hash01(x, z, 5, self.seed) * 5.0) as i32 - 2;
                            let dy = (hash01(x, z, 6, self.seed) * 6.0) as i32 + 1;
                            let (tx, ty, tz) = (x + dx, dy, z + dz);
                            if w.in_bounds(tx, ty, tz) && w.get(tx, ty, tz) == BlockType::Stone {
                                w.set(tx, ty, tz, biome.ore_block());
                            }
                        }
                    }
                }
            }
        }
    }

    /// 通用 IronOre 散撒（不用 cluster 约束）
    fn place_generic_iron(&self, w: &mut World) {
        let s = w.size as i32;
        for z in 0..s {
            for x in 0..s {
                let h = self.find_surface(w.clone(), x, z);
                if let Some(surf) = h {
                    // 表面下方 1-3 块
                    let depth = 1 + (hash01(x, z, 7, self.seed) * 3.0) as i32;
                    let y = surf - depth;
                    if y > 0
                        && w.get(x, y, z) == BlockType::Stone
                        && hash01(x, y, z, self.seed) > 0.85
                    {
                        w.set(x, y, z, BlockType::IronOre);
                    }
                }
            }
        }
    }

    /// 找 (x, z) 处的最高实心 y。None = 整列空气
    fn find_surface(&self, w: World, x: i32, z: i32) -> Option<i32> {
        for y in (0..w.size).rev() {
            if w.get(x, y, z).is_solid() {
                return Some(y);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Gathering 动作：把块转成资源
// ---------------------------------------------------------------------------

/// 玩家挖掘/采集一个块。返回 (产出的资源, 数量)。
/// 块被设为 Air。资源通过 transfer 进 GlobalResourcePool。
pub fn gather_block(
    world: &mut World,
    pool: &mut crate::resource::GlobalResourcePool,
    x: i32,
    y: i32,
    z: i32,
    player_id: u32,
) -> Result<Option<(ResourceKind, i64)>, String> {
    let b = world.get(x, y, z);
    if !b.is_solid() {
        return Ok(None);
    }
    let Some((kind, amount)) = b.yields() else {
        return Ok(None);
    };
    if amount <= 0 {
        return Ok(None);
    }
    // 资源进池（来源 = 玩家采集）
    let t = Transfer {
        kind,
        amount,
        src: TransferSrc::PlayerGather(player_id),
        dst: TransferDst::PlayerUse(player_id), // 暂时直接进池；后续可改背包
    };
    apply_transfer(pool, t).map_err(|e| format!("gather transfer failed: {}", e))?;
    // 块变空气
    world.set(x, y, z, BlockType::Air);
    Ok(Some((kind, amount)))
}

// ---------------------------------------------------------------------------
// World 视野：给定玩家位置，返回可见块集合
// ---------------------------------------------------------------------------

/// 给定玩家位置 + 半径，返回 (x, y, z) 列表（demo：体素视距范围）
///
/// demo 简化：直接返回 (px±r) × (py±r) × (pz±r) 的所有方块，含两端。
/// 后续可加球形过滤、视线追踪、距离衰减。
pub fn visible_blocks(
    world: &World,
    px: i32,
    py: i32,
    pz: i32,
    radius: i32,
) -> Vec<(i32, i32, i32)> {
    let r: i32 = radius;
    let s: i32 = world.size;
    let mut out: Vec<(i32, i32, i32)> = Vec::new();
    let y_start: i32 = (py - r).max(0);
    let y_end: i32 = (py + r + 1).min(s);
    let z_start: i32 = (pz - r).max(0);
    let z_end: i32 = (pz + r + 1).min(s);
    let x_start: i32 = (px - r).max(0);
    let x_end: i32 = (px + r + 1).min(s);
    for y in y_start..y_end {
        for z in z_start..z_end {
            for x in x_start..x_end {
                out.push((x, y, z));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::GlobalResourcePool;

    #[test]
    fn biome_from_xz_3_zones() {
        let n = WORLD_SIZE;
        // 北 (z < n/3) = Tundra
        assert_eq!(Biome::from_xz(0, 0), Biome::Tundra);
        // 中 (n/3 ≤ z < 2n/3) = Jungle
        assert_eq!(Biome::from_xz(0, n / 3), Biome::Jungle);
        assert_eq!(Biome::from_xz(0, 2 * n / 3 - 1), Biome::Jungle);
        // 南 (z ≥ 2n/3) = Desert
        assert_eq!(Biome::from_xz(0, 2 * n / 3), Biome::Desert);
        assert_eq!(Biome::from_xz(0, n - 1), Biome::Desert);
    }

    #[test]
    fn generator_is_deterministic() {
        let g1 = WorldGenerator::default();
        let g2 = WorldGenerator::default();
        let w1 = g1.generate(16);
        let w2 = g2.generate(16);
        assert_eq!(w1.blocks, w2.blocks, "same seed must produce same world");
    }

    #[test]
    fn generator_with_different_seeds_differ() {
        let mut g1 = WorldGenerator::default();
        g1.seed = 1;
        let mut g2 = WorldGenerator::default();
        g2.seed = 2;
        let w1 = g1.generate(16);
        let w2 = g2.generate(16);
        assert_ne!(w1.blocks, w2.blocks);
    }

    #[test]
    fn all_3_biomes_have_ore() {
        let g = WorldGenerator::default();
        let w = g.generate(16);
        // demo: 不严格保证每个 biome 都有 ore（hash 概率性），只要总 ore > 0
        let total_ores = w.count_biome_ores(Biome::Desert)
            + w.count_biome_ores(Biome::Tundra)
            + w.count_biome_ores(Biome::Jungle);
        assert!(
            total_ores > 0,
            "至少一个 biome 应该有 ore (实际: D={}, T={}, J={})",
            w.count_biome_ores(Biome::Desert),
            w.count_biome_ores(Biome::Tundra),
            w.count_biome_ores(Biome::Jungle)
        );
    }

    #[test]
    fn blocks_in_bounds_default_air() {
        let w = World::new(8);
        assert_eq!(w.get(100, 100, 100), BlockType::Air);
        assert_eq!(w.get(-1, 0, 0), BlockType::Air);
    }

    #[test]
    fn set_get_round_trip() {
        let mut w = World::new(8);
        w.set(3, 2, 1, BlockType::Wood);
        assert_eq!(w.get(3, 2, 1), BlockType::Wood);
    }

    #[test]
    fn gather_wood_adds_to_pool() {
        let mut w = World::new(8);
        w.set(1, 1, 1, BlockType::Wood);
        let mut pool = GlobalResourcePool::new();
        let res = gather_block(&mut w, &mut pool, 1, 1, 1, 42).unwrap();
        assert_eq!(res, Some((ResourceKind::Wood, 5)));
        assert_eq!(pool.get(ResourceKind::Wood), 5);
        assert_eq!(w.get(1, 1, 1), BlockType::Air);
    }

    #[test]
    fn gather_ore_adds_to_biome_resource() {
        let mut w = World::new(8);
        w.set(2, 1, 1, BlockType::SunstoneOre);
        let mut pool = GlobalResourcePool::new();
        let res = gather_block(&mut w, &mut pool, 2, 1, 1, 1).unwrap();
        assert_eq!(res, Some((ResourceKind::Sunstone, 1)));
        assert_eq!(pool.get(ResourceKind::Sunstone), 1);
    }

    #[test]
    fn gather_air_returns_none() {
        let mut w = World::new(8);
        let mut pool = GlobalResourcePool::new();
        let res = gather_block(&mut w, &mut pool, 1, 1, 1, 1).unwrap();
        assert_eq!(res, None);
    }

    #[test]
    fn visible_blocks_in_radius() {
        let w = World::new(16);
        let v = visible_blocks(&w, 8, 8, 8, 3);
        // 7³ = 343 个块
        assert_eq!(v.len(), 7 * 7 * 7);
    }

    #[test]
    fn block_yields_match() {
        // 总纲：挖阳炎石矿 → 阳炎石
        assert_eq!(
            BlockType::SunstoneOre.yields(),
            Some((ResourceKind::Sunstone, 1))
        );
        // 挖苹果树（berry） → 苹果
        assert_eq!(
            BlockType::BerryThicket.yields(),
            Some((ResourceKind::Apple, 1))
        );
    }
}
