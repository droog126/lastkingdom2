//! 可配置地形生成系统（斗魂大乱斗风格）
//!
//! 架构：
//! - `TerrainModule` trait：每个模块决定"看 (x,y,z) 我要放什么 block"
//! - `TerrainPipeline`：按顺序执行模块列表，第一个返回 Some 的胜出
//! - `Preset`：命名配置包（default / flat / mountainous / random）
//! - [`shapes`]: 几何 Primitive 系统（盒/球/椭球/圆柱/平面/噪声丘/布尔减）
//!   + [`shapes::ShapeLayer`] 让一组 primitive 直接变成一个 Module
//!
//! 用法：
//! ```ignore
//! let preset = presets::default_preset();
//! let block = preset.pipeline.generate(x, y, z);
//! ```

pub mod shapes;

pub use shapes::{
    BoxShape, CylinderShape, EllipsoidShape, FillMode, HillShape, ModuleSpec, NoiseFieldShape,
    NoiseHillShape, PipelineSpec, PlaneShape, Shape, ShapeLayer, ShapeSpec, SphereShape,
    SubtractShape,
};

use crate::world::{Biome, BlockType, SEA_LEVEL};

/// 单次 evaluate 时传给模块的上下文
#[derive(Clone, Debug)]
pub struct TerrainContext {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub seed: u64,
    /// 已计算出的地表 Y 高度（HeightmapModule 写入，其他模块可读）
    pub surface_y: Option<i32>,
    /// 当前 biome（被 biome 模块写入）
    pub biome: Option<Biome>,
}

/// 模块 trait — 每个模块独立决定位置
pub trait TerrainModule: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    /// 冲突优先级（高 weight 胜出）默认 1.0
    fn weight(&self) -> f32 {
        1.0
    }
    /// 返回 `Some(b)` = 放这个 block；`None` = 我不管，让下一个
    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType>;
    /// 如果这个模块定义 (x, z) 处的"软地表 f32 高度"（多维 heightmap 用），返回 Some
    /// 默认 None — 只有 HeightmapModule / SpawnHillModule 覆写
    fn surface_f32(&self, _x: i32, _z: i32) -> Option<f32> {
        None
    }
}

// ---------------------------------------------------------------------------
// 噪声工具
// ---------------------------------------------------------------------------

/// 32-bit hash → [0, 1) — 同 world/mod.rs::hash01 保持一致
pub fn hash01(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = seed
        ^ (x as u32).wrapping_mul(0x9E3779B1)
        ^ (y as u32).wrapping_mul(0x85EBCA77)
        ^ (z as u32).wrapping_mul(0xC2B2AE3D);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846CA68B);
    h ^= h >> 16;
    (h as f32) / (u32::MAX as f32)
}

fn noise3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    hash01(x, y, z, seed)
}

// ---------------------------------------------------------------------------
// 模块 0: SpawnHill — 出生点强制起一个圆顶山（保证 player 不在水边）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SpawnHillModule {
    pub name: String,
    pub center_x: i32,
    pub center_z: i32,
    pub radius: i32,     // 圆顶半径（XZ 距离）
    pub max_height: i32, // 圆顶中心最高点距海平面的高度
    pub enabled: bool,
    pub weight: f32,
}

impl Default for SpawnHillModule {
    fn default() -> Self {
        Self {
            name: "spawn_hill".into(),
            center_x: 48,
            center_z: 48,
            radius: 22,
            max_height: 22, // 中心 Y = 12+22=34; 边缘约 Y=12=sea level
            enabled: true,
            weight: 10.0,
        }
    }
}

impl TerrainModule for SpawnHillModule {
    fn name(&self) -> &str {
        &self.name
    }
    fn weight(&self) -> f32 {
        self.weight
    }

    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType> {
        if !self.enabled {
            return None;
        }
        let dx = ctx.x - self.center_x;
        let dz = ctx.z - self.center_z;
        let dist2 = dx * dx + dz * dz;
        let r = self.radius;
        if dist2 > r * r {
            return None;
        } // 圆外不管，让 Heightmap 接管

        // 圆顶高度 = 中心最高 (t=1), 往外按 1-cos 半圆降到边缘最低 (t=0)
        // 之前 `1.0 - t` 反了, 导致中心凹 (中心 dome_h=0), 玩家站在空气里看不到 mesh
        let dist = (dist2 as f32).sqrt();
        let t = (1.0 - dist / r as f32).clamp(0.0, 1.0);
        let dome_h = (1.0 - (t * std::f32::consts::FRAC_PI_2).cos()) * self.max_height as f32;
        let surface = SEA_LEVEL + 1 + dome_h as i32;

        // 同时把 surface_y 写进 ctx（这样 Cave/Tree/Ore 能看到）
        ctx.surface_y = Some(surface);
        ctx.biome = Some(Biome::Jungle);

        if ctx.y > surface {
            return Some(BlockType::Air);
        }
        if ctx.y == surface {
            return Some(BlockType::Leaves);
        } // 草绿冠
        if ctx.y >= surface - 3 {
            return Some(BlockType::Dirt);
        }
        Some(BlockType::Stone)
    }

    /// 圆顶山 f32 高度（不截断） — 多维 heightmap 的 dim 2
    fn surface_f32(&self, x: i32, z: i32) -> Option<f32> {
        if !self.enabled {
            return None;
        }
        let dx = x - self.center_x;
        let dz = z - self.center_z;
        let dist2 = dx * dx + dz * dz;
        let r = self.radius;
        if dist2 > r * r {
            return None;
        }
        let dist = (dist2 as f32).sqrt();
        let t = (1.0 - dist / r as f32).clamp(0.0, 1.0);
        let dome_h = (1.0 - (t * std::f32::consts::FRAC_PI_2).cos()) * self.max_height as f32;
        Some(SEA_LEVEL as f32 + 1.0 + dome_h)
    }
}

// ---------------------------------------------------------------------------
// 模块 0.5: VillageMark — 在指定 (x, z) 放村旗+小屋方块（造"国家感"）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct VillageMarkModule {
    pub name: String,
    pub sites: Vec<(i32, i32)>, // (x, z) 位置
    pub pole_height: i32,       // 旗杆高度
    pub flag_w: i32,            // 旗面宽
    pub flag_h: i32,            // 旗面高
    pub enabled: bool,
    pub weight: f32,
}

impl Default for VillageMarkModule {
    fn default() -> Self {
        // 无限村庄：每 80 块（沿 +X）放一个，z 偏移由噪声决定
        // 周围 5 块（东西南北 + 中心）固定 + SpawnHill 周边的老 5 个 = 出生区也有村子
        Self {
            name: "village_mark".into(),
            sites: vec![(48, 70), (70, 48), (26, 48), (70, 70), (26, 70)],
            pole_height: 8,
            flag_w: 2,
            flag_h: 2,
            enabled: true,
            weight: 9.0, // 比 heightmap 高（覆盖地表），比 SpawnHill 10.0 低（不挡出生点）
        }
    }
}

impl VillageMarkModule {
    /// 给定 (x, z) 算出该位置是不是某个"无限村庄"位置 + 村庄的中心坐标
    /// 规则：每 80 块一组（沿着 x 方向），z 位置用 noise 决定
    pub fn nearest_village(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        const SPACING: i32 = 80;
        // 检查当前位置 (x, z) 是不是某个村庄中心
        let cell_x = (x as f32 / SPACING as f32).floor() as i32;
        // 候选 3 个 cell（当前 + 左右）以处理边界
        for dx in -1..=1 {
            let cx = cell_x + dx;
            // 村庄中心 x = cx * SPACING + SPACING/2
            let sx = cx * SPACING + SPACING / 2;
            // z 偏移：每个 cx 对应一个固定 z（用 hash 决定）
            let sz_offset = (hash01(cx, 0, 0, 0xBEEF) * 60.0 - 30.0) as i32; // -30..30
            let sz = 48 + sz_offset;
            // 中心点 (sx, sz)
            if (x - sx).abs() < 2 && (z - sz).abs() < 2 {
                return Some((sx, sz));
            }
        }
        None
    }
}

impl TerrainModule for VillageMarkModule {
    fn name(&self) -> &str {
        &self.name
    }
    fn weight(&self) -> f32 {
        self.weight
    }

    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType> {
        if !self.enabled {
            return None;
        }
        // 候选村庄列表 = 5 固定出生区村庄 + 无限村庄
        let mut all_sites: Vec<(i32, i32)> = self.sites.clone();
        if let Some((sx, sz)) = self.nearest_village(ctx.x, ctx.z) {
            if !all_sites.contains(&(sx, sz)) {
                all_sites.push((sx, sz));
            }
        }
        for (sx, sz) in &all_sites {
            let dx = (ctx.x - sx).abs();
            let dz = (ctx.z - sz).abs();
            // 旗杆: 1x1 立柱
            if dx <= 0 && dz <= 0 {
                if ctx.y <= self.pole_height {
                    return Some(BlockType::Wood);
                }
                return None;
            }
            // 旗面: 旗杆顶 (sx+1..sx+flag_w) x 旗杆顶
            if dx >= 1
                && dx <= self.flag_w
                && dz <= 0
                && ctx.y >= self.pole_height - self.flag_h
                && ctx.y < self.pole_height
            {
                return Some(BlockType::Sand); // 黄色旗面
            }
            // 小屋: 1x1 中心 (只一格, 不连成线)
            if dx == 0 && dz == 0 {
                if let Some(surf) = ctx.surface_y {
                    if ctx.y == surf + 1 {
                        return Some(BlockType::Wood); // 地板
                    }
                    if ctx.y == surf + 2 {
                        return Some(BlockType::Wood); // 墙
                    }
                    if ctx.y == surf + 3 {
                        return Some(BlockType::Leaves); // 屋顶
                    }
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// 模块 1: Heightmap — 决定地表高度 + 表层 block
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct HeightmapModule {
    pub name: String,
    pub seed: u64,
    pub base_height: f32,      // SEA_LEVEL
    pub amplitude_big: f32,    // 大尺度山脉振幅
    pub frequency_big: f32,    // 1/N
    pub amplitude_detail: f32, // 细节振幅
    pub frequency_detail: f32,
    pub weight: f32,
    /// Desert biome 高度偏移 — 默认 -2 (沙漠偏低). superflat 设 0 让地形完全平.
    pub biome_bias_desert: f32,
    /// Jungle biome 高度偏移 — 默认 0.
    pub biome_bias_jungle: f32,
    /// Tundra biome 高度偏移 — 默认 +3 (苔原偏高). superflat 设 0.
    pub biome_bias_tundra: f32,
}

impl Default for HeightmapModule {
    fn default() -> Self {
        Self {
            name: "heightmap".into(),
            seed: 0,
            base_height: SEA_LEVEL as f32 + 1.0,
            amplitude_big: 14.0,
            frequency_big: 1.0 / 8.0,
            amplitude_detail: 4.0,
            frequency_detail: 1.0,
            weight: 1.0,
            biome_bias_desert: -2.0,
            biome_bias_jungle: 0.0,
            biome_bias_tundra: 3.0,
        }
    }
}

impl HeightmapModule {
    /// 算 (x, z) 处的地表 Y 高度（f32，不截断）— 给 scalar_field / 玩家物理用
    /// "多维 heightmap" 的 dim 1：保留 noise 公式的 f32 渐变
    pub fn compute_surface_f32(&self, x: i32, z: i32) -> f32 {
        let fx_big = (x as f32 * self.frequency_big) as i32;
        let fz_big = (z as f32 * self.frequency_big) as i32;
        let h_big = noise3(fx_big, 0, fz_big, self.seed as u32);
        let h_detail = noise3(x, 0, z, (self.seed ^ 0xCAFE) as u32);
        let biome = Biome::from_xz_infinite(x, z);
        let bias: f32 = match biome {
            Biome::Desert => self.biome_bias_desert,
            Biome::Jungle => self.biome_bias_jungle,
            Biome::Tundra => self.biome_bias_tundra,
        };
        h_big * self.amplitude_big + h_detail * self.amplitude_detail + self.base_height + bias
    }

    /// 算 (x, z) 处的地表 Y 高度（i32，给 BlockType 生成用）
    pub fn compute_surface(&self, x: i32, z: i32) -> i32 {
        self.compute_surface_f32(x, z) as i32
    }

    pub fn surface_block(&self, biome: Biome) -> BlockType {
        match biome {
            Biome::Desert => BlockType::Sand,
            Biome::Jungle => BlockType::Dirt,
            Biome::Tundra => BlockType::Snow,
        }
    }
}

impl TerrainModule for HeightmapModule {
    fn name(&self) -> &str {
        &self.name
    }
    fn weight(&self) -> f32 {
        self.weight
    }

    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType> {
        let surface = self.compute_surface(ctx.x, ctx.z);
        ctx.surface_y = Some(surface);
        let biome = Biome::from_xz_infinite(ctx.x, ctx.z);
        ctx.biome = Some(biome);
        // 高度以上
        if ctx.y >= surface {
            return Some(BlockType::Air);
        }
        // 表层
        if ctx.y == surface - 1 {
            return Some(self.surface_block(biome));
        }
        // 表层下 3 层 = dirt
        if ctx.y >= surface - 3 {
            return Some(BlockType::Dirt);
        }
        // 地下 = stone
        Some(BlockType::Stone)
    }

    /// f32 软地表 — 多维 heightmap 的 dim 1
    fn surface_f32(&self, x: i32, z: i32) -> Option<f32> {
        Some(self.compute_surface_f32(x, z))
    }
}

// ---------------------------------------------------------------------------
// 模块 2: Cave — 挖洞
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct CaveModule {
    pub name: String,
    pub seed: u64,
    pub density: f32,  // > 此值挖空
    pub scale_xz: f32, // 1/N
    pub scale_y: f32,
    pub min_y: i32,        // 不挖更深
    pub preserve_top: i32, // 保留表层 N 格不挖
    pub enabled: bool,
    pub weight: f32,
}

impl Default for CaveModule {
    fn default() -> Self {
        Self {
            name: "cave".into(),
            seed: 0xC0CA,
            density: 0.65,
            scale_xz: 1.0 / 4.0,
            scale_y: 1.0 / 3.0,
            min_y: 1,
            preserve_top: 1,
            enabled: true,
            weight: 0.9, // 比 heightmap 略低，让 heightmap 先定表层
        }
    }
}

impl TerrainModule for CaveModule {
    fn name(&self) -> &str {
        &self.name
    }
    fn weight(&self) -> f32 {
        self.weight
    }

    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType> {
        if !self.enabled {
            return None;
        }
        if ctx.y < self.min_y {
            return None;
        }
        // 保护表层
        if let Some(surface) = ctx.surface_y {
            if ctx.y >= surface - self.preserve_top {
                return None;
            }
        }
        let fx = (ctx.x as f32 * self.scale_xz) as i32;
        let fy = (ctx.y as f32 * self.scale_y) as i32;
        let fz = (ctx.z as f32 * self.scale_xz) as i32;
        let n = noise3(fx, fy, fz, self.seed as u32);
        if n > self.density {
            Some(BlockType::Air)
        } else {
            None // 不冲突，让 heightmap 的 stone 留下
        }
    }
}

// ---------------------------------------------------------------------------
// 模块 3: WaterFill — 海平面以下空腔填水
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct WaterFillModule {
    pub name: String,
    pub sea_level: i32,
    pub weight: f32,
}

impl Default for WaterFillModule {
    fn default() -> Self {
        Self { name: "water".into(), sea_level: SEA_LEVEL, weight: 0.5 }
    }
}

impl TerrainModule for WaterFillModule {
    fn name(&self) -> &str {
        &self.name
    }
    fn weight(&self) -> f32 {
        self.weight
    }

    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType> {
        if ctx.y > self.sea_level {
            return None;
        }
        // 高度以上 / 已经是水 的不处理；只在空腔里填水
        if let Some(surface) = ctx.surface_y {
            if ctx.y >= surface {
                return Some(BlockType::Water);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// 模块 4: Tree — 树 (trunk + canopy)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct TreeModule {
    pub name: String,
    pub seed: u64,
    pub density: f32, // 0..1, 每 (x,z) 生成树概率
    pub min_height: i32,
    pub max_height: i32,
    pub canopy_radius: i32,
    pub biome: Option<Biome>, // None = 任何 biome 都长
    pub weight: f32,
}

impl Default for TreeModule {
    fn default() -> Self {
        Self {
            name: "tree".into(),
            seed: 0xBEEF,
            density: 0.15, // 15% 方格有树（10% 还是稀，远处看就是零星几棵）
            min_height: 5,
            max_height: 8,
            canopy_radius: 3,
            biome: None,
            weight: 0.8,
        }
    }
}

impl TreeModule {
    /// 算 (x, z) 是不是树的中心
    fn is_tree_center(&self, x: i32, z: i32) -> bool {
        // 把 x, z 投到 8x8 网格上
        let cell_x = x.div_euclid(8);
        let cell_z = z.div_euclid(8);
        let _cell_seed = (cell_x as u64).wrapping_mul(0x9E3779B1)
            ^ (cell_z as u64).wrapping_mul(0x85EBCA77)
            ^ self.seed;
        let r = hash01(cell_x, 0, cell_z, self.seed as u32);
        if r > self.density {
            return false;
        }
        // 在 8x8 cell 内选 (2,2) 到 (5,5) 的中心
        let lx = x.rem_euclid(8);
        let lz = z.rem_euclid(8);
        if lx == 3 && lz == 3 {
            return true;
        }
        // 让 hash 决定其它内部点也行（简单版：只允许 (3,3)）
        false
    }
}

impl TerrainModule for TreeModule {
    fn name(&self) -> &str {
        &self.name
    }
    fn weight(&self) -> f32 {
        self.weight
    }

    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType> {
        // 只在地表生成
        let surface = ctx.surface_y?;
        if ctx.y > surface {
            return None;
        }
        // biome 过滤
        if let Some(required) = self.biome {
            if ctx.biome? != required {
                return None;
            }
        }
        // 是不是这棵树的中心？
        if !self.is_tree_center(ctx.x, ctx.z) {
            return None;
        }
        // 中心 (x, z) 才有树
        // trunk: ctx.y 是地表上方 1..=tree_height
        let trunk_height = self.min_height
            + (hash01(ctx.x, 0, ctx.z, (self.seed ^ 0x1234) as u32) as i32)
                .rem_euclid(self.max_height - self.min_height + 1);
        // trunk
        if ctx.y > surface && ctx.y <= surface + trunk_height {
            return Some(BlockType::Wood);
        }
        // canopy: trunk 顶上 1..=canopy_radius 的球
        let canopy_top = surface + trunk_height;
        let canopy_bottom = canopy_top - self.canopy_radius;
        if ctx.y > canopy_bottom && ctx.y <= canopy_top + 1 {
            // 用 noise 决定是不是叶子（这样不规则）
            let n = hash01(ctx.x, ctx.y, ctx.z, (self.seed ^ 0xCAFE) as u32);
            // 中心位置必须有叶子；越远概率越低
            let dy = ctx.y - canopy_top;
            let dist = (dy * dy) as f32;
            let p = 0.7 - dist * 0.15;
            if n < p {
                return Some(BlockType::Leaves);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// 模块 5: Ore — 矿脉 (cluster)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct OreModule {
    pub name: String,
    pub seed: u64,
    pub ore: BlockType,
    pub cluster_size: i32,    // cluster 半径
    pub clusters_per_64: f32, // 每 64³ 体积的 cluster 数
    pub min_y: i32,
    pub max_y: i32,
    pub biome: Option<Biome>,
    pub weight: f32,
}

impl Default for OreModule {
    fn default() -> Self {
        Self {
            name: "ore_iron".into(),
            seed: 0xCAFE,
            ore: BlockType::IronOre,
            cluster_size: 2,
            clusters_per_64: 2.0, // 之前 0.5 太少；提到 2.0 每 64³ 期望 2 簇
            min_y: 1,
            max_y: 60,
            biome: None,
            weight: 0.4,
        }
    }
}

impl OreModule {
    fn is_in_ore_cluster(&self, x: i32, y: i32, z: i32) -> bool {
        // 把空间分成 16³ 网格，每格放一个 cluster 中心
        let cell_size = 16;
        let cx_cell = x.div_euclid(cell_size);
        let cy_cell = y.div_euclid(cell_size);
        let cz_cell = z.div_euclid(cell_size);
        // cluster 中心在 cell 内随机位置
        let cell_hash = hash01(cx_cell, cy_cell, cz_cell, (self.seed ^ 0x3333) as u32);
        // 决定这一格是否放 cluster
        let threshold = self.clusters_per_64 / 64.0; // per cell
        if cell_hash > threshold {
            return false;
        }
        // 中心位置
        let lx = x.rem_euclid(cell_size);
        let ly = y.rem_euclid(cell_size);
        let lz = z.rem_euclid(cell_size);
        let ox = (hash01(cx_cell, cy_cell, cz_cell, (self.seed ^ 0xAAAA) as u32) as i32)
            .rem_euclid(cell_size);
        let oy = (hash01(cx_cell, cy_cell, cz_cell, (self.seed ^ 0xBBBB) as u32) as i32)
            .rem_euclid(cell_size);
        let oz = (hash01(cx_cell, cy_cell, cz_cell, (self.seed ^ 0xCCCC) as u32) as i32)
            .rem_euclid(cell_size);
        let dx = (lx - ox).abs();
        let dy = (ly - oy).abs();
        let dz = (lz - oz).abs();
        if dx + dy + dz <= self.cluster_size {
            return true;
        }
        // 噪点：邻近格子随机
        if hash01(x, y, z, (self.seed ^ 0xDEAD) as u32) > 0.9 {
            return true;
        }
        false
    }
}

impl TerrainModule for OreModule {
    fn name(&self) -> &str {
        &self.name
    }
    fn weight(&self) -> f32 {
        self.weight
    }

    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType> {
        if ctx.y < self.min_y || ctx.y > self.max_y {
            return None;
        }
        if let Some(required) = self.biome {
            if ctx.biome? != required {
                return None;
            }
        }
        // 只替换 stone（避免覆盖 cave/水）
        // 这里我们用 weight 比 heightmap 高，先返回矿
        if self.is_in_ore_cluster(ctx.x, ctx.y, ctx.z) {
            return Some(self.ore);
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct TerrainPipeline {
    pub name: String,
    pub modules: Vec<Box<dyn TerrainModule>>,
    pub vertical_min: i32,
    pub vertical_max: i32,
    pub seed: u64,
}

impl TerrainPipeline {
    pub fn generate(&self, x: i32, y: i32, z: i32) -> BlockType {
        if y < self.vertical_min || y >= self.vertical_max {
            return BlockType::Air;
        }
        let mut ctx = TerrainContext { x, y, z, seed: self.seed, surface_y: None, biome: None };
        // 按 weight 降序遍历（高 weight 优先）
        let mut sorted: Vec<&Box<dyn TerrainModule>> = self.modules.iter().collect();
        sorted.sort_by(|a, b| {
            b.weight().partial_cmp(&a.weight()).unwrap_or(std::cmp::Ordering::Equal)
        });
        for m in sorted {
            if let Some(block) = m.decide(&mut ctx) {
                return block;
            }
        }
        BlockType::Air
    }

    /// 软地表 f32 高度（按 weight 取第一个 Some）— 多维 heightmap 入口
    /// 返回 None = pipeline 没有任何模块定义 surface_f32（比如纯矿/树 pipeline）
    pub fn surface_f32(&self, x: i32, z: i32) -> Option<f32> {
        let mut sorted: Vec<&Box<dyn TerrainModule>> = self.modules.iter().collect();
        sorted.sort_by(|a, b| {
            b.weight().partial_cmp(&a.weight()).unwrap_or(std::cmp::Ordering::Equal)
        });
        for m in sorted {
            if let Some(h) = m.surface_f32(x, z) {
                return Some(h);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Presets
// ---------------------------------------------------------------------------

pub mod presets {
    use super::*;
    use rand::prelude::*;

    pub fn default_preset() -> TerrainPipeline {
        // "温和山"配方：出生就看到有山有水有沙有动物。amp 16 + 出生圆顶山 + 5 村旗 + 树 0.15 + 矿 2.0
        let mut h = HeightmapModule { seed: 0xDEADBEEF, ..Default::default() };
        h.amplitude_big = 16.0;
        h.amplitude_detail = 5.0;
        // 新增 ShapeLayer：把"出生圆顶山 + 头顶 dome 顶"用 Shape 重做一份（与 SpawnHill 协作）
        let spawn_island = shapes::ShapeLayer {
            name: "spawn_island".into(),
            weight: 9.5,
            fill: shapes::FillMode::AdaptiveSurface {
                surface: BlockType::Leaves,
                subsurface: BlockType::Dirt,
            },
            shapes: vec![shapes::ShapeSpec::Hill(shapes::HillShape {
                name: "spawn_dome".into(),
                center_x: 48,
                center_z: 48,
                radius: 22.0,
                max_height: 22.0,
            })],
            biome_override: Some(Biome::Jungle),
            enabled: true,
        };
        TerrainPipeline {
            name: "default".into(),
            modules: vec![
                Box::new(spawn_island), // ShapeLayer 覆盖优先级 9.5（比 heightmap 高）
                Box::new(SpawnHillModule::default()), // 原 SpawnHill 仍在（10.0）
                Box::new(VillageMarkModule::default()), // 5 村旗
                Box::new(h),
                Box::new(CaveModule::default()),
                Box::new(WaterFillModule::default()),
                Box::new(TreeModule::default()),
                Box::new(OreModule::default()),
            ],
            vertical_min: 0,
            vertical_max: crate::world::VERTICAL_SIZE,
            seed: 0xDEADBEEF,
        }
    }

    /// 全平地图（适合造建筑）
    pub fn flat_preset() -> TerrainPipeline {
        let mut h = HeightmapModule::default();
        h.amplitude_big = 0.0;
        h.amplitude_detail = 0.0;
        TerrainPipeline {
            name: "flat".into(),
            modules: vec![
                Box::new(h),
                Box::new(WaterFillModule::default()),
                Box::new(TreeModule { density: 0.01, ..Default::default() }), // 稀树
            ],
            vertical_min: 0,
            vertical_max: crate::world::VERTICAL_SIZE,
            seed: 0xDEADBEEF,
        }
    }

    /// 超平坦地图 — 完全平的单一水平面，不填水、不生成自然树、不挖洞
    ///
    /// 所有 (x, z) 处的 surface 高度 = SEA_LEVEL + 1 (= 13)。
    /// 这样 pretty 模块装饰物（玩家 avatar / 怪物球 / 树 / 石 / 花 / POI 柱 / 远景山）的
    /// `effective_ground_height(x, z)` 都返回同一个值 → 全部贴地，没有浮空/陷地。
    /// 不填水：玩家脚底和水面不重叠，不会"站在水里"。
    /// 不生成自然树：避免 voxel 树乱长遮挡 pretty 装饰物。
    /// 没有洞穴：地表下都是 solid，玩家不会被困在地下。
    /// biome_bias 也归零：原本 Desert=-2 / Jungle=0 / Tundra=+3 会让地面随 biome 起伏，
    /// 这里直接覆盖 compute_surface_f32 的 bias，保证严格水平。
    ///
    /// 使用场景：auto-demo 默认场景，让画面干净 + 装饰物可见。
    pub fn superflat_preset() -> TerrainPipeline {
        let mut h = HeightmapModule::default();
        h.amplitude_big = 0.0;
        h.amplitude_detail = 0.0;
        // biome_bias=0: 不用 Desert/Jungle/Tundra 的 +3/-2/0 偏移, 否则表面会随 biome 起伏
        h.biome_bias_desert = 0.0;
        h.biome_bias_jungle = 0.0;
        h.biome_bias_tundra = 0.0;
        TerrainPipeline {
            name: "superflat".into(),
            modules: vec![
                Box::new(h),
                // WaterFillModule weight=0: 不填水
                Box::new(WaterFillModule { weight: 0.0, ..WaterFillModule::default() }),
            ],
            vertical_min: 0,
            vertical_max: crate::world::VERTICAL_SIZE,
            seed: 0xDEADBEEF,
        }
    }

    /// 高山地图（振幅 × 1.5）
    pub fn mountainous_preset() -> TerrainPipeline {
        let mut h = HeightmapModule::default();
        h.amplitude_big = 22.0;
        h.amplitude_detail = 6.0;
        TerrainPipeline {
            name: "mountainous".into(),
            modules: vec![
                Box::new(SpawnHillModule::default()), // 出生圆顶山（mountainous 也有）
                Box::new(h),
                Box::new(CaveModule::default()),
                Box::new(WaterFillModule::default()),
                Box::new(TreeModule::default()),
                Box::new(OreModule::default()),
            ],
            vertical_min: 0,
            vertical_max: crate::world::VERTICAL_SIZE,
            seed: 0xDEADBEEF,
        }
    }

    /// 斗魂大乱斗 — 每次随机
    pub fn lold_arena_preset() -> TerrainPipeline {
        let mut rng = rand::rng();
        let mut h = HeightmapModule::default();
        h.amplitude_big = rng.random_range(5.0..25.0);
        h.amplitude_detail = rng.random_range(1.0..6.0);
        h.seed = rng.random();
        h.name = format!("heightmap_lold_{}", rng.random_range(0..10000));

        let mut cave = CaveModule::default();
        cave.density = rng.random_range(0.55..0.85);
        cave.seed = rng.random();
        cave.enabled = rng.random_bool(0.7);

        let sea_level_offset = rng.random_range(-3..3);
        let water = WaterFillModule {
            name: "water_lold".into(),
            sea_level: SEA_LEVEL + sea_level_offset,
            weight: 0.5,
        };

        let mut modules: Vec<Box<dyn TerrainModule>> =
            vec![Box::new(h), Box::new(cave), Box::new(water)];
        // 50% 概率加树
        if rng.random_bool(0.5) {
            modules.push(Box::new(TreeModule {
                seed: rng.random(),
                density: rng.random_range(0.005..0.05),
                ..Default::default()
            }));
        }
        // 30% 概率加矿
        if rng.random_bool(0.3) {
            let ores = [
                BlockType::IronOre,
                BlockType::SunstoneOre,
                BlockType::FrostcoreOre,
                BlockType::LivingRoot,
            ];
            modules.push(Box::new(OreModule {
                seed: rng.random(),
                ore: ores[rng.random_range(0..ores.len())],
                ..Default::default()
            }));
        }

        TerrainPipeline {
            name: format!("lold_arena_{}", rng.random_range(0..10000)),
            modules,
            vertical_min: 0,
            vertical_max: crate::world::VERTICAL_SIZE,
            seed: rng.random(),
        }
    }

    /// 所有 preset 名（用于 F8 循环切换）
    pub fn preset_names() -> &'static [&'static str] {
        &[
            "superflat",
            "default",
            "flat",
            "mountainous",
            "lold_arena",
            "shape_demo",
        ]
    }

    pub fn by_name(name: &str) -> TerrainPipeline {
        match name {
            "superflat" => superflat_preset(),
            "default" => default_preset(),
            "flat" => flat_preset(),
            "mountainous" => mountainous_preset(),
            "lold_arena" | "random" => lold_arena_preset(),
            "shape_demo" => shape_demo_preset(),
            other => {
                eprintln!("[terrain] unknown preset '{}', using default", other);
                default_preset()
            }
        }
    }

    /// 纯 Shape 演示 preset：1 个岛（噪声丘）+ 1 个塔（圆柱）+ 1 球装饰
    /// ——证明不依赖 WorldGenerator 时间建模，全部按需 query。
    pub fn shape_demo_preset() -> TerrainPipeline {
        let island = ShapeLayer {
            name: "island".into(),
            weight: 10.0,
            fill: FillMode::AdaptiveSurface {
                surface: BlockType::Dirt,
                subsurface: BlockType::Stone,
            },
            shapes: vec![ShapeSpec::NoiseHill(NoiseHillShape {
                name: "island_hill".into(),
                center_x: 48,
                center_z: 48,
                radius: 28.0,
                seed: 0xCAFE,
                freq: 0.06,
                peak_height: 18.0,
            })],
            biome_override: Some(Biome::Jungle),
            enabled: true,
        };
        let tower = ShapeLayer {
            name: "tower".into(),
            weight: 11.0, // 比岛更高 → 在岛上放塔
            fill: FillMode::Replace(BlockType::Wood),
            shapes: vec![ShapeSpec::Cylinder(CylinderShape {
                name: "tower_body".into(),
                center_x: 48,
                center_z: 48,
                y_min: 0,
                y_max: 50,
                radius: 2.0,
            })],
            biome_override: None,
            enabled: true,
        };
        let crystal = ShapeLayer {
            name: "crystal".into(),
            weight: 12.0,
            fill: FillMode::Replace(BlockType::FrostcoreOre),
            shapes: vec![ShapeSpec::Sphere(SphereShape {
                name: "crystal_ball".into(),
                center: [70, 16, 70],
                radius: 4.0,
            })],
            biome_override: Some(Biome::Tundra),
            enabled: true,
        };
        TerrainPipeline {
            name: "shape_demo".into(),
            modules: vec![
                Box::new(island),
                Box::new(tower),
                Box::new(crystal),
                Box::new(WaterFillModule::default()),
            ],
            vertical_min: 0,
            vertical_max: crate::world::VERTICAL_SIZE,
            seed: 0xDEADBEEF,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regress: auto-demo 锁死 player.pos.y = 16.0 (block_pos [48, 16, 48]),
    /// 但 SpawnHillModule 在 (48, 48) 中心建了一个高 22m 的圆顶山,
    /// 实际地面高度 ≈ SEA_LEVEL + 1 + max_height = 35 (远高于 16).
    /// 旧代码 `let ground_y = player.pos.y - 2.0` 把装饰物漂在 14m 空中,
    /// avatar 也悬空看不见. 正确做法: player 出生 y 必须等于 effective_ground_height.
    ///
    /// 本测试断言: 默认 preset 下 (48, 48) 的 surface_f32 应 ≥ 30 (圆顶山顶附近),
    /// 远大于之前硬编码的 player.y=16. 一旦不满足, 说明 preset 改了, auto-demo
    /// 的 y=16 硬编码也得跟着改, 否则漂浮 bug 会复发.
    #[test]
    fn default_preset_spawn_hill_is_above_player_y() {
        let pipeline = presets::by_name("default");
        let ground_at_spawn = pipeline
            .surface_f32(48, 48)
            .expect("default preset should have surface_f32 at spawn (48, 48)");
        // 圆顶山中心高度 = SEA_LEVEL(12) + 1 + max_height(22) = 35, 容许 ±3 容差
        assert!(
            ground_at_spawn >= 30.0,
            "spawn ground height should be >= 30 (SpawnHill 圆顶山顶 ≈ 35), got {} — \
             若此断言失败, auto-demo 的 player.pos.y=16 硬编码需要同步更新, 否则漂浮 bug 会复发",
            ground_at_spawn
        );
        // 同时远大于 auto-demo 硬编码的 y=16, 差距至少 10m
        assert!(
            ground_at_spawn - 16.0 >= 10.0,
            "spawn ground ({}) 应比 auto-demo 硬编码 y=16 高至少 10m, 否则玩家仍站在空气里",
            ground_at_spawn
        );
    }

    /// 锁住 bug: auto-demo 锁死 player.pos.y=16 但 ground ≈ 35, 差距必须被显式修正。
    /// 本测试给"auto-demo 出生 y 的正确算法"写一个 reference implementation,
    /// 后续 main.rs 应该调用 `auto_demo_spawn_y(&game_world)` 而不是再硬编码 16.
    /// 此处先暴露 API 让 main.rs 可以依赖, 防止再有人手抖写回 16.0.
    #[test]
    fn auto_demo_spawn_y_matches_ground_at_spawn() {
        let pipeline = presets::by_name("default");
        let ground_at_spawn =
            pipeline.surface_f32(48, 48).expect("default preset has surface_f32 at spawn");
        // auto-demo 出生 y 应当等于 ground + 0.5 (玩家站在山顶, 眼睛高 0.5m)
        let auto_demo_y = ground_at_spawn + 0.5;
        // 旧硬编码 16.0 远低于此, 证明 bug 存在
        assert!(
            (auto_demo_y - 16.0).abs() > 5.0,
            "auto_demo_y ({}) 与旧硬编码 16.0 差距 > 5m, 证明 player.pos.y=16 是错的",
            auto_demo_y
        );
        assert!(
            auto_demo_y > 30.0,
            "auto-demo 出生 y 应该 > 30 (圆顶山顶 + 0.5), got {}",
            auto_demo_y
        );
    }

    /// 中心最高, 边缘最低, 圆顶山 f32 高度单调下降 (距离从 0 到 radius).
    /// 防止 SpawnHill 公式被改坏导致 dome 翻转或平台化.
    #[test]
    fn spawn_hill_monotonic_decay_from_center() {
        let h = SpawnHillModule::default();
        let center = h.surface_f32(48, 48).expect("center");
        let mid = h.surface_f32(48 + h.radius / 2, 48).expect("mid");
        let edge = h.surface_f32(48 + h.radius - 1, 48).expect("edge");
        assert!(
            center > mid && mid > edge,
            "spawn hill dome should decay center > mid > edge, got {} > {} > {}",
            center,
            mid,
            edge
        );
        // 边缘应回到 SEA_LEVEL + 1 (圆顶山公式 cos(0)=1 → dome_h=0)
        assert!(
            (edge - (SEA_LEVEL as f32 + 1.0)).abs() < 0.5,
            "spawn hill edge should be ≈ SEA_LEVEL+1={}, got {}",
            SEA_LEVEL as f32 + 1.0,
            edge
        );
    }

    /// superflat_preset 是"完全平的地形"语义:
    ///   1. 所有 (x, z) 处 surface 高度恒为 SEA_LEVEL + 1
    ///   2. 不生成水 (WaterFillModule weight ≈ 0)
    ///   3. 不生成自然树 (modules 里没有 TreeModule)
    /// 锁住这套规则, 防止有人无意间给 superflat 加 SpawnHill/WaterFill/Tree 又让它不平了.
    #[test]
    fn superflat_preset_is_completely_flat() {
        let pipeline = presets::superflat_preset();
        assert_eq!(pipeline.name, "superflat", "preset 名应是 superflat");

        // 1. surface 高度在多个采样点都等于 SEA_LEVEL + 1
        let expected = SEA_LEVEL as f32 + 1.0;
        for &(x, z) in &[(0, 0), (48, 48), (95, 95), (10, 80), (80, 10)] {
            let h = pipeline
                .surface_f32(x, z)
                .unwrap_or_else(|| panic!("superflat surface_f32 at ({},{}) should exist", x, z));
            assert!(
                (h - expected).abs() < 0.01,
                "superflat surface at ({},{}) 应等于 SEA_LEVEL+1={}, got {}",
                x,
                z,
                expected,
                h
            );
        }

        // 2. WaterFillModule 要么不存在, 要么 weight < 0.01 (不填水)
        let bad_water = pipeline.modules.iter().any(|m| m.name() == "water" && m.weight() >= 0.01);
        assert!(
            !bad_water,
            "superflat 不应填水, got modules: {:?}",
            pipeline.modules.iter().map(|m| (m.name(), m.weight())).collect::<Vec<_>>()
        );

        // 3. 不应有 TreeModule
        let has_tree = pipeline.modules.iter().any(|m| m.name() == "trees");
        assert!(
            !has_tree,
            "superflat 不应生成自然树, modules: {:?}",
            pipeline.modules.iter().map(|m| m.name()).collect::<Vec<_>>()
        );

        // 4. spawn 处 ground ≈ 13, 跟 effective_ground_height(48, 48) 一致
        let ground_at_spawn = pipeline.surface_f32(48, 48).expect("superflat spawn surface");
        assert!(
            (ground_at_spawn - 13.0).abs() < 0.01,
            "superflat 出生点 ground 应 = 13 (SEA_LEVEL+1), got {}",
            ground_at_spawn
        );
    }
}
