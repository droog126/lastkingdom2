

pub mod shapes;

pub use shapes::{
    BoxShape, CylinderShape, EllipsoidShape, FillMode, HillShape, ModuleSpec, NoiseFieldShape,
    NoiseHillShape, PipelineSpec, PlaneShape, Shape, ShapeLayer, ShapeSpec, SphereShape,
    SubtractShape,
};

use crate::world::{Biome, BlockType, SEA_LEVEL};

#[derive(Clone, Debug)]
pub struct TerrainContext {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub seed: u64,

    pub surface_y: Option<i32>,

    pub biome: Option<Biome>,
}

pub trait TerrainModule: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;

    fn weight(&self) -> f32 {
        1.0
    }

    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType>;

    fn surface_f32(&self, _x: i32, _z: i32) -> Option<f32> {
        None
    }
}

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

#[derive(Debug, Clone)]
pub struct SpawnHillModule {
    pub name: String,
    pub center_x: i32,
    pub center_z: i32,
    pub radius: i32,
    pub max_height: i32,
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
            max_height: 22,
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
        }

        let dist = (dist2 as f32).sqrt();
        let t = (1.0 - dist / r as f32).clamp(0.0, 1.0);
        let dome_h = (1.0 - (t * std::f32::consts::FRAC_PI_2).cos()) * self.max_height as f32;
        let surface = SEA_LEVEL + 1 + dome_h as i32;

        ctx.surface_y = Some(surface);
        ctx.biome = Some(Biome::Jungle);

        if ctx.y > surface {
            return Some(BlockType::Air);
        }
        if ctx.y == surface {
            return Some(BlockType::Leaves);
        }
        if ctx.y >= surface - 3 {
            return Some(BlockType::Dirt);
        }
        Some(BlockType::Stone)
    }

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

#[derive(Debug, Clone)]
pub struct VillageMarkModule {
    pub name: String,
    pub sites: Vec<(i32, i32)>,
    pub pole_height: i32,
    pub flag_w: i32,
    pub flag_h: i32,
    pub enabled: bool,
    pub weight: f32,
}

impl Default for VillageMarkModule {
    fn default() -> Self {

        Self {
            name: "village_mark".into(),
            sites: vec![(48, 70), (70, 48), (26, 48), (70, 70), (26, 70)],
            pole_height: 8,
            flag_w: 2,
            flag_h: 2,
            enabled: true,
            weight: 9.0,
        }
    }
}

impl VillageMarkModule {

    pub fn nearest_village(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        const SPACING: i32 = 80;

        let cell_x = (x as f32 / SPACING as f32).floor() as i32;

        for dx in -1..=1 {
            let cx = cell_x + dx;

            let sx = cx * SPACING + SPACING / 2;

            let sz_offset = (hash01(cx, 0, 0, 0xBEEF) * 60.0 - 30.0) as i32;
            let sz = 48 + sz_offset;

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

        let mut all_sites: Vec<(i32, i32)> = self.sites.clone();
        if let Some((sx, sz)) = self.nearest_village(ctx.x, ctx.z) {
            if !all_sites.contains(&(sx, sz)) {
                all_sites.push((sx, sz));
            }
        }
        for (sx, sz) in &all_sites {
            let dx = (ctx.x - sx).abs();
            let dz = (ctx.z - sz).abs();

            if dx <= 0 && dz <= 0 {
                if ctx.y <= self.pole_height {
                    return Some(BlockType::Wood);
                }
                return None;
            }

            if dx >= 1
                && dx <= self.flag_w
                && dz <= 0
                && ctx.y >= self.pole_height - self.flag_h
                && ctx.y < self.pole_height
            {
                return Some(BlockType::Sand);
            }

            if dx == 0 && dz == 0 {
                if let Some(surf) = ctx.surface_y {
                    if ctx.y == surf + 1 {
                        return Some(BlockType::Wood);
                    }
                    if ctx.y == surf + 2 {
                        return Some(BlockType::Wood);
                    }
                    if ctx.y == surf + 3 {
                        return Some(BlockType::Leaves);
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct HeightmapModule {
    pub name: String,
    pub seed: u64,
    pub base_height: f32,
    pub amplitude_big: f32,
    pub frequency_big: f32,
    pub amplitude_detail: f32,
    pub frequency_detail: f32,
    pub weight: f32,

    pub biome_bias_desert: f32,

    pub biome_bias_jungle: f32,

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

        if ctx.y >= surface {
            return Some(BlockType::Air);
        }

        if ctx.y == surface - 1 {
            return Some(self.surface_block(biome));
        }

        if ctx.y >= surface - 3 {
            return Some(BlockType::Dirt);
        }

        Some(BlockType::Stone)
    }

    fn surface_f32(&self, x: i32, z: i32) -> Option<f32> {
        Some(self.compute_surface_f32(x, z))
    }
}

#[derive(Debug)]
pub struct CaveModule {
    pub name: String,
    pub seed: u64,
    pub density: f32,
    pub scale_xz: f32,
    pub scale_y: f32,
    pub min_y: i32,
    pub preserve_top: i32,
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
            weight: 0.9,
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
            None
        }
    }
}

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

        if let Some(surface) = ctx.surface_y {
            if ctx.y >= surface {
                return Some(BlockType::Water);
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct TreeModule {
    pub name: String,
    pub seed: u64,
    pub density: f32,
    pub min_height: i32,
    pub max_height: i32,
    pub canopy_radius: i32,
    pub biome: Option<Biome>,
    pub weight: f32,
}

impl Default for TreeModule {
    fn default() -> Self {
        Self {
            name: "tree".into(),
            seed: 0xBEEF,
            density: 0.15,
            min_height: 5,
            max_height: 8,
            canopy_radius: 3,
            biome: None,
            weight: 0.8,
        }
    }
}

impl TreeModule {

    fn is_tree_center(&self, x: i32, z: i32) -> bool {

        let cell_x = x.div_euclid(8);
        let cell_z = z.div_euclid(8);
        let _cell_seed = (cell_x as u64).wrapping_mul(0x9E3779B1)
            ^ (cell_z as u64).wrapping_mul(0x85EBCA77)
            ^ self.seed;
        let r = hash01(cell_x, 0, cell_z, self.seed as u32);
        if r > self.density {
            return false;
        }

        let lx = x.rem_euclid(8);
        let lz = z.rem_euclid(8);
        if lx == 3 && lz == 3 {
            return true;
        }

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

        let surface = ctx.surface_y?;
        if ctx.y > surface {
            return None;
        }

        if let Some(required) = self.biome {
            if ctx.biome? != required {
                return None;
            }
        }

        if !self.is_tree_center(ctx.x, ctx.z) {
            return None;
        }

        let trunk_height = self.min_height
            + (hash01(ctx.x, 0, ctx.z, (self.seed ^ 0x1234) as u32) as i32)
                .rem_euclid(self.max_height - self.min_height + 1);

        if ctx.y > surface && ctx.y <= surface + trunk_height {
            return Some(BlockType::Wood);
        }

        let canopy_top = surface + trunk_height;
        let canopy_bottom = canopy_top - self.canopy_radius;
        if ctx.y > canopy_bottom && ctx.y <= canopy_top + 1 {

            let n = hash01(ctx.x, ctx.y, ctx.z, (self.seed ^ 0xCAFE) as u32);

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

#[derive(Debug)]
pub struct OreModule {
    pub name: String,
    pub seed: u64,
    pub ore: BlockType,
    pub cluster_size: i32,
    pub clusters_per_64: f32,
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
            clusters_per_64: 2.0,
            min_y: 1,
            max_y: 60,
            biome: None,
            weight: 0.4,
        }
    }
}

impl OreModule {
    fn is_in_ore_cluster(&self, x: i32, y: i32, z: i32) -> bool {

        let cell_size = 16;
        let cx_cell = x.div_euclid(cell_size);
        let cy_cell = y.div_euclid(cell_size);
        let cz_cell = z.div_euclid(cell_size);

        let cell_hash = hash01(cx_cell, cy_cell, cz_cell, (self.seed ^ 0x3333) as u32);

        let threshold = self.clusters_per_64 / 64.0;
        if cell_hash > threshold {
            return false;
        }

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

        if self.is_in_ore_cluster(ctx.x, ctx.y, ctx.z) {
            return Some(self.ore);
        }
        None
    }
}

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

pub mod presets {
    use super::*;
    use rand::prelude::*;

    pub fn default_preset() -> TerrainPipeline {

        let mut h = HeightmapModule { seed: 0xDEADBEEF, ..Default::default() };
        h.amplitude_big = 16.0;
        h.amplitude_detail = 5.0;

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
                Box::new(spawn_island),
                Box::new(SpawnHillModule::default()),
                Box::new(VillageMarkModule::default()),
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

    pub fn flat_preset() -> TerrainPipeline {
        let mut h = HeightmapModule::default();
        h.amplitude_big = 0.0;
        h.amplitude_detail = 0.0;
        TerrainPipeline {
            name: "flat".into(),
            modules: vec![
                Box::new(h),
                Box::new(WaterFillModule::default()),
                Box::new(TreeModule { density: 0.01, ..Default::default() }),
            ],
            vertical_min: 0,
            vertical_max: crate::world::VERTICAL_SIZE,
            seed: 0xDEADBEEF,
        }
    }

    pub fn superflat_preset() -> TerrainPipeline {
        let mut h = HeightmapModule::default();
        h.amplitude_big = 0.0;
        h.amplitude_detail = 0.0;

        h.biome_bias_desert = 0.0;
        h.biome_bias_jungle = 0.0;
        h.biome_bias_tundra = 0.0;
        TerrainPipeline {
            name: "superflat".into(),
            modules: vec![
                Box::new(h),

                Box::new(WaterFillModule { weight: 0.0, ..WaterFillModule::default() }),
            ],
            vertical_min: 0,
            vertical_max: crate::world::VERTICAL_SIZE,
            seed: 0xDEADBEEF,
        }
    }

    pub fn mountainous_preset() -> TerrainPipeline {
        let mut h = HeightmapModule::default();
        h.amplitude_big = 22.0;
        h.amplitude_detail = 6.0;
        TerrainPipeline {
            name: "mountainous".into(),
            modules: vec![
                Box::new(SpawnHillModule::default()),
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

        if rng.random_bool(0.5) {
            modules.push(Box::new(TreeModule {
                seed: rng.random(),
                density: rng.random_range(0.005..0.05),
                ..Default::default()
            }));
        }

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
            weight: 11.0,
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

    #[test]
    fn default_preset_spawn_hill_is_above_player_y() {
        let pipeline = presets::by_name("default");
        let ground_at_spawn = pipeline
            .surface_f32(48, 48)
            .expect("default preset should have surface_f32 at spawn (48, 48)");

        assert!(
            ground_at_spawn >= 30.0,
            "spawn ground height should be >= 30 (SpawnHill 圆顶山顶 ≈ 35), got {} — \
             若此断言失败, auto-demo 的 player.pos.y=16 硬编码需要同步更新, 否则漂浮 bug 会复发",
            ground_at_spawn
        );

        assert!(
            ground_at_spawn - 16.0 >= 10.0,
            "spawn ground ({}) 应比 auto-demo 硬编码 y=16 高至少 10m, 否则玩家仍站在空气里",
            ground_at_spawn
        );
    }

    #[test]
    fn auto_demo_spawn_y_matches_ground_at_spawn() {
        let pipeline = presets::by_name("default");
        let ground_at_spawn =
            pipeline.surface_f32(48, 48).expect("default preset has surface_f32 at spawn");

        let auto_demo_y = ground_at_spawn + 0.5;

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

        assert!(
            (edge - (SEA_LEVEL as f32 + 1.0)).abs() < 0.5,
            "spawn hill edge should be ≈ SEA_LEVEL+1={}, got {}",
            SEA_LEVEL as f32 + 1.0,
            edge
        );
    }

    #[test]
    fn superflat_preset_is_completely_flat() {
        let pipeline = presets::superflat_preset();
        assert_eq!(pipeline.name, "superflat", "preset 名应是 superflat");

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

        let bad_water = pipeline.modules.iter().any(|m| m.name() == "water" && m.weight() >= 0.01);
        assert!(
            !bad_water,
            "superflat 不应填水, got modules: {:?}",
            pipeline.modules.iter().map(|m| (m.name(), m.weight())).collect::<Vec<_>>()
        );

        let has_tree = pipeline.modules.iter().any(|m| m.name() == "trees");
        assert!(
            !has_tree,
            "superflat 不应生成自然树, modules: {:?}",
            pipeline.modules.iter().map(|m| m.name()).collect::<Vec<_>>()
        );

        let ground_at_spawn = pipeline.surface_f32(48, 48).expect("superflat spawn surface");
        assert!(
            (ground_at_spawn - 13.0).abs() < 0.01,
            "superflat 出生点 ground 应 = 13 (SEA_LEVEL+1), got {}",
            ground_at_spawn
        );
    }
}
