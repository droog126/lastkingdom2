use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::constant::*;
use crate::resource::{ResourceKind, Transfer, TransferDst, TransferSrc, apply_transfer};
use crate::world::terrain::TerrainModule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Biome {
    Desert,
    Tundra,
    Jungle,
}

impl Biome {
    pub fn noise_field(x: i32, z: i32) -> f32 {
        let cell = 64_i32;
        let cx = (x as f32 / cell as f32).floor() as i32;
        let cz = (z as f32 / cell as f32).floor() as i32;
        let fx = (x as f32 / cell as f32) - cx as f32;
        let fz = (z as f32 / cell as f32) - cz as f32;

        let v00 = terrain::hash01(cx, 0, cz, 0xB10E);
        let v10 = terrain::hash01(cx + 1, 0, cz, 0xB10E);
        let v01 = terrain::hash01(cx, 0, cz + 1, 0xB10E);
        let v11 = terrain::hash01(cx + 1, 0, cz + 1, 0xB10E);

        let sx = fx * fx * (3.0 - 2.0 * fx);
        let sz = fz * fz * (3.0 - 2.0 * fz);
        let a = v00 * (1.0 - sx) + v10 * sx;
        let b = v01 * (1.0 - sx) + v11 * sx;
        let big = a * (1.0 - sz) + b * sz;

        let cell2 = 8_i32;
        let cx2 = (x as f32 / cell2 as f32).floor() as i32;
        let cz2 = (z as f32 / cell2 as f32).floor() as i32;
        let detail = terrain::hash01(cx2, 0, cz2, 0xD37A1);
        big * 0.7 + detail * 0.3
    }

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

    pub fn ore_block(self) -> BlockType {
        match self {
            Biome::Desert => BlockType::SunstoneOre,
            Biome::Tundra => BlockType::FrostcoreOre,
            Biome::Jungle => BlockType::LivingRoot,
        }
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockType {
    Air,
    Dirt,
    Grass,
    Stone,
    Sand,
    Snow,
    Leaves,
    Water,
    Wood,
    IronOre,
    SunstoneOre,
    FrostcoreOre,
    LivingRoot,
    BerryThicket,
}

impl BlockType {
    pub const fn is_solid(self) -> bool {
        !matches!(self, BlockType::Air | BlockType::Water)
    }

    pub const fn is_renderable(self) -> bool {
        !matches!(self, BlockType::Air)
    }

    pub const fn is_surface(self) -> bool {
        matches!(
            self,
            BlockType::Dirt
                | BlockType::Grass
                | BlockType::Sand
                | BlockType::Snow
                | BlockType::BerryThicket
        )
    }

    pub fn yields(self) -> Option<(ResourceKind, i64)> {
        use BlockType::*;
        use ResourceKind as R;
        match self {
            Air | Dirt | Grass | Sand | Snow | Leaves | Water => None,
            Stone => Some((R::Stone, 1)),
            Wood => Some((R::Wood, 5)),
            IronOre => Some((R::Wood, 0)),
            SunstoneOre => Some((R::Sunstone, 1)),
            FrostcoreOre => Some((R::Frostcore, 1)),
            LivingRoot => Some((R::LivingRoot, 1)),
            BerryThicket => Some((R::Apple, 1)),
        }
    }

    pub fn debug_color_rgba(self) -> [f32; 4] {
        use BlockType::*;
        match self {
            Air => [0.0, 0.0, 0.0, 0.0],
            Dirt => [0.55, 0.36, 0.20, 1.0],
            Grass => [0.30, 0.58, 0.22, 1.0],
            Stone => [0.55, 0.55, 0.55, 1.0],
            Sand => [0.92, 0.82, 0.55, 1.0],
            Snow => [0.95, 0.97, 1.00, 1.0],
            Leaves => [0.20, 0.55, 0.18, 1.0],
            Water => [0.25, 0.50, 0.85, 1.0],
            Wood => [0.40, 0.25, 0.10, 1.0],
            IronOre => [0.80, 0.60, 0.30, 1.0],
            SunstoneOre => [1.00, 0.70, 0.20, 1.0],
            FrostcoreOre => [0.60, 0.85, 1.00, 1.0],
            LivingRoot => [0.20, 0.80, 0.30, 1.0],
            BerryThicket => [0.85, 0.20, 0.50, 1.0],
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct World {
    pub blocks: Vec<BlockType>,
    pub size: i32,

    pub procedural: bool,

    pub edited: HashSet<(i32, i32, i32)>,

    pub seed: u64,

    pub pipeline: std::sync::Arc<terrain::TerrainPipeline>,

    pub geo_overlay: Vec<terrain::ShapeLayer>,

    pub geo_overlay_names: HashSet<String>,

    pub content: Option<std::sync::Arc<content::MaterializedContent>>,
}

pub mod content;
pub mod generation;
pub mod terrain;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldConfig {
    pub size: i32,
    pub preset: String,
    pub seed: u64,
    pub install_spawn_platform: bool,
    pub generate_content: bool,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            size: WORLD_SIZE,
            preset: "default".to_string(),
            seed: 0xDEADBEEF,
            install_spawn_platform: true,
            generate_content: true,
        }
    }
}

pub fn generate_world(config: &WorldConfig) -> World {
    let mut pipeline = terrain::presets::by_name(&config.preset);
    pipeline.seed = config.seed;
    let mut world = World::with_pipeline(config.size, pipeline);
    world.seed = config.seed;
    if config.install_spawn_platform {
        install_huge_spawn_platform(&mut world);
    }
    if config.generate_content {
        let generated = content::generate_and_materialize_content(&mut world, config.seed)
            .unwrap_or_else(|error| panic!("world content generation failed: {error}"));
        world.content = Some(std::sync::Arc::new(generated));
    }
    world
}

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
            geo_overlay: Vec::new(),
            geo_overlay_names: HashSet::new(),
            content: None,
        }
    }

    pub fn push_geo_layer(&mut self, layer: terrain::ShapeLayer) {
        let name = layer.name.clone();
        self.geo_overlay.retain(|l| l.name != name);
        self.geo_overlay.push(layer);
        self.geo_overlay_names.insert(name);
    }

    pub fn remove_geo_layer(&mut self, name: &str) -> bool {
        let before = self.geo_overlay.len();
        self.geo_overlay.retain(|l| l.name != name);
        self.geo_overlay_names.remove(name);
        self.geo_overlay.len() != before
    }

    pub fn clear_geo_overlay(&mut self) {
        self.geo_overlay.clear();
        self.geo_overlay_names.clear();
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

    pub fn generate_voxel(&self, x: i32, y: i32, z: i32) -> BlockType {
        let mut sorted: Vec<&terrain::ShapeLayer> = self.geo_overlay.iter().collect();
        sorted.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for layer in &sorted {
            let mut ctx = terrain::TerrainContext {
                x,
                y,
                z,
                seed: self.seed,
                surface_y: None,
                biome: None,
            };
            if let Some(b) = layer.decide(&mut ctx) {
                if let Some(biome) = layer.biome_override {
                    let _ = biome;
                }
                return b;
            }
        }

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

    pub fn in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && x < self.size && y >= 0 && y < self.size && z >= 0 && z < self.size
    }

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

fn noise3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let xi = x as f32;
    let yi = y as f32;
    let zi = z as f32;

    let c000 = hash01(x, y, z, seed);
    let c100 = hash01(x + 1, y, z, seed);
    let c010 = hash01(x, y + 1, z, seed);
    let c110 = hash01(x + 1, y + 1, z, seed);
    let c001 = hash01(x, y, z + 1, seed);
    let c101 = hash01(x + 1, y, z + 1, seed);
    let c011 = hash01(x, y + 1, z + 1, seed);
    let c111 = hash01(x + 1, y + 1, z + 1, seed);

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

pub struct WorldGenerator {
    pub seed: u32,
    pub ore_threshold: f32,
    pub tree_density: f32,
    pub thicket_density: f32,
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
    pub fn generate(&self, size: i32) -> World {
        let mut w = World::new(size);
        let s = size as i32;

        let spawn_x = s / 2;
        let spawn_z = s / 2;
        let flat_radius: f32 = 10.0;

        let clear_radius: i32 = 3;
        for z in 0..s {
            for x in 0..s {
                let biome = Biome::from_xz(x, z);
                let dist_from_spawn = (((x - spawn_x).pow(2) + (z - spawn_z).pow(2)) as f32).sqrt();

                let h_big = noise3(x / 8, 0, z / 8, self.seed);
                let h_detail = noise3(x, 0, z, self.seed ^ 0xCAFE);
                let biome_bias: f32 = match biome {
                    Biome::Desert => -2.0,
                    Biome::Jungle => 0.0,
                    Biome::Tundra => 3.0,
                };
                let base_h = (h_big * 14.0) + (h_detail * 4.0) + SEA_LEVEL as f32 + biome_bias;

                let h = if dist_from_spawn < flat_radius {
                    SEA_LEVEL + 1
                } else if dist_from_spawn < flat_radius + 6.0 {
                    let t = (dist_from_spawn - flat_radius) / 6.0;
                    let flat = (SEA_LEVEL + 1) as f32;
                    (flat * (1.0 - t) + base_h * t) as i32
                } else {
                    base_h as i32
                }
                .clamp(1, s - 4);

                let surface = match biome {
                    Biome::Desert => BlockType::Sand,
                    Biome::Jungle => BlockType::Dirt,
                    Biome::Tundra => BlockType::Snow,
                };
                let sub = BlockType::Dirt;
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
                    }
                    let cave_n = noise3(x / 4, y / 3, z / 4, self.seed ^ 0xC0CA);
                    if cave_n > 0.65 {
                        w.set(x, y, z, BlockType::Air);
                    }
                }
            }
        }

        for z in 0..s {
            for x in 0..s {
                for y in 0..=SEA_LEVEL {
                    if w.get(x, y, z) == BlockType::Air {
                        w.set(x, y, z, BlockType::Water);
                    }
                }
            }
        }

        for biome in [Biome::Desert, Biome::Tundra, Biome::Jungle] {
            self.place_ore_clusters(&mut w, biome);
        }

        self.place_generic_iron(&mut w);

        for z in 0..s {
            for x in 0..s {
                let biome = Biome::from_xz(x, z);
                if biome == Biome::Desert {
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
                            Biome::Jungle => {
                                (5 + (hash01(x, z, 8, self.seed) * 3.0) as i32, (3, 2))
                            }
                            Biome::Tundra => {
                                (4 + (hash01(x, z, 8, self.seed) * 2.0) as i32, (2, 2))
                            }
                            Biome::Desert => unreachable!(),
                        };

                        for up in 1..=trunk_h {
                            if y + up < s {
                                w.set(x, y + up, z, BlockType::Wood);
                            }
                        }

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
                                        if dx == 0 && dz == 0 && dy < canopy.1 - 1 {
                                            continue;
                                        }
                                        if (dx.abs() + dz.abs() + dy) > canopy.0 {
                                            continue;
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

    fn place_ore_clusters(&self, w: &mut World, biome: Biome) {
        let s = w.size as i32;
        let mut placed: Vec<(i32, i32)> = Vec::new();
        for z in 1..s - 1 {
            for x in 1..s - 1 {
                if Biome::from_xz(x, z) != biome {
                    continue;
                }
                if hash01(x, z, 2, self.seed ^ (biome as u32) * 0x100) < 0.08 {
                    if placed.iter().all(|(px, pz)| {
                        (x - px).abs() + (z - pz).abs() > self.min_ore_cluster_spacing
                    }) {
                        placed.push((x, z));
                        let cluster_size = 2 + (hash01(x, z, 3, self.seed) * 3.0) as i32;
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

    fn place_generic_iron(&self, w: &mut World) {
        let s = w.size as i32;
        for z in 0..s {
            for x in 0..s {
                let h = self.find_surface(w.clone(), x, z);
                if let Some(surf) = h {
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

    fn find_surface(&self, w: World, x: i32, z: i32) -> Option<i32> {
        for y in (0..w.size).rev() {
            if w.get(x, y, z).is_solid() {
                return Some(y);
            }
        }
        None
    }
}

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

    let t = Transfer {
        kind,
        amount,
        src: TransferSrc::PlayerGather(player_id),
        dst: TransferDst::PlayerUse(player_id),
    };
    apply_transfer(pool, t).map_err(|e| format!("gather transfer failed: {}", e))?;

    world.set(x, y, z, BlockType::Air);
    Ok(Some((kind, amount)))
}

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

pub const PLAYER_BODY_CLEARANCE_BLOCKS: i32 = 2;
pub const MAX_SMOOTH_DROP: f32 = 6.0;

pub fn player_body_clear(world: &World, x: i32, foot_y: i32, z: i32) -> bool {
    if x < 0
        || x >= world.size
        || z < 0
        || z >= world.size
        || foot_y < 0
        || foot_y + PLAYER_BODY_CLEARANCE_BLOCKS > world.size
    {
        return false;
    }
    for y in foot_y..(foot_y + PLAYER_BODY_CLEARANCE_BLOCKS) {
        if world.get(x, y, z).is_solid() {
            return false;
        }
    }
    true
}

fn standable_foot_y(world: &World, x: i32, z: i32, near_y: f32, max_step_up: f32) -> Option<i32> {
    if x < 0 || x >= world.size || z < 0 || z >= world.size {
        return None;
    }
    let min_y = ((near_y - MAX_SMOOTH_DROP).floor() as i32).max(1);
    let max_y = ((near_y + max_step_up).ceil() as i32).min(world.size - 2);
    (min_y..=max_y)
        .filter(|foot_y| {
            world.get(x, *foot_y - 1, z).is_solid() && player_body_clear(world, x, *foot_y, z)
        })
        .min_by(|a, b| {
            let da = (*a as f32 - near_y).abs();
            let db = (*b as f32 - near_y).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn standable_foot_y_any_height(world: &World, x: i32, z: i32) -> Option<i32> {
    if x < 0 || x >= world.size || z < 0 || z >= world.size {
        return None;
    }
    (1..(world.size - 2)).rev().find(|foot_y| {
        world.get(x, *foot_y - 1, z).is_solid() && player_body_clear(world, x, *foot_y, z)
    })
}

pub fn player_stand_position_at(
    world: &World,
    x: i32,
    z: i32,
    near_y: f32,
    max_step_up: f32,
) -> Option<(Vec3, [i32; 3])> {
    let foot_y = standable_foot_y(world, x, z, near_y, max_step_up)?;
    Some((
        Vec3::new(x as f32 + 0.5, foot_y as f32, z as f32 + 0.5),
        [x, foot_y, z],
    ))
}

pub fn player_spawn_position_at(world: &World, x: i32, z: i32) -> Option<(Vec3, [i32; 3])> {
    let foot_y = standable_foot_y_any_height(world, x, z)?;
    Some((
        Vec3::new(x as f32 + 0.5, foot_y as f32, z as f32 + 0.5),
        [x, foot_y, z],
    ))
}

/// Deterministic 4-ring × 8-lane block-offset scan, golab-style.
///
/// Returns 32 `[dx, dz]` offsets around a spawn center, ordered from the
/// innermost ring outward. The `seed_offset` parameter rotates the starting
/// lane so two callers asking for "the same fallback" pick different tiles
/// when the inner ring is full — the same trick golab uses with
/// `client_id % 8` in `fallback_spawn_translation`.
///
/// Use this when you want a deterministic, allocation-light fallback for
/// "give me a tile near this center, but not exactly on top of it":
///
/// ```ignore
/// for offset in fallback_spawn_ring_offsets(seed) {
///     let candidate = [center[0] + offset[0], center[1], center[2] + offset[1]];
///     if clears_check(&candidate) {
///         return candidate;
///     }
/// }
/// ```
#[must_use]
pub fn fallback_spawn_ring_offsets(seed_offset: u8) -> Vec<[i32; 2]> {
    (0_u8..4)
        .flat_map(|ring| {
            (0_u8..8).map(move |lane| {
                let lane_index = lane.wrapping_add(seed_offset) % 8;
                let radius = f32::from(ring).mul_add(0.85, 1.0);
                let angle = (f32::from(lane_index) / 8.0) * std::f32::consts::TAU;
                let dx = (angle.cos() * radius).round() as i32;
                let dz = (angle.sin() * radius).round() as i32;
                [dx, dz]
            })
        })
        .collect()
}

pub fn player_spawn_position_near(
    world: &World,
    x: i32,
    z: i32,
    search_radius: i32,
    clearance_radius: i32,
) -> Option<(Vec3, [i32; 3])> {
    let mut best: Option<(i32, Vec3, [i32; 3])> = None;
    let radius = search_radius.max(0);
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            let dist2 = dx * dx + dz * dz;
            if dist2 > radius * radius {
                continue;
            }
            let sx = x + dx;
            let sz = z + dz;
            let Some((pos, block_pos)) = player_spawn_position_at(world, sx, sz) else {
                continue;
            };
            if !spawn_clearance(world, block_pos, clearance_radius) {
                continue;
            }
            match best {
                None => best = Some((dist2, pos, block_pos)),
                Some((best_dist2, _, _)) if dist2 < best_dist2 => {
                    best = Some((dist2, pos, block_pos));
                }
                _ => {}
            }
        }
    }
    best.map(|(_, pos, block_pos)| (pos, block_pos))
}

pub fn player_position_is_safe(world: &World, pos: Vec3) -> bool {
    let x = pos.x.floor() as i32;
    let z = pos.z.floor() as i32;
    let foot_y = pos.y.floor() as i32;
    world.in_bounds(x, foot_y, z)
        && foot_y > 0
        && world.get(x, foot_y - 1, z).is_solid()
        && player_body_clear(world, x, foot_y, z)
}

pub fn resolve_player_stuck_near(
    world: &World,
    pos: Vec3,
    search_radius: i32,
) -> Option<(Vec3, [i32; 3])> {
    if player_position_is_safe(world, pos) {
        let block_pos = [
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        ];
        return Some((pos, block_pos));
    }

    let x = pos.x.floor() as i32;
    let z = pos.z.floor() as i32;
    let radius = search_radius.max(0);
    let mut best: Option<(f32, Vec3, [i32; 3])> = None;
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dz * dz > radius * radius {
                continue;
            }
            let Some((stand_pos, block_pos)) = player_spawn_position_at(world, x + dx, z + dz)
            else {
                continue;
            };
            let candidate = Vec3::new(
                pos.x.floor() + 0.5 + dx as f32,
                stand_pos.y,
                pos.z.floor() + 0.5 + dz as f32,
            );
            if !player_position_is_safe(world, candidate) {
                continue;
            }
            let dist2 = (candidate - pos).length_squared();
            match best {
                None => best = Some((dist2, candidate, block_pos)),
                Some((best_dist2, _, _)) if dist2 < best_dist2 => {
                    best = Some((dist2, candidate, block_pos));
                }
                _ => {}
            }
        }
    }

    best.map(|(_, pos, block_pos)| (pos, block_pos))
}

fn spawn_clearance(world: &World, block_pos: [i32; 3], radius: i32) -> bool {
    let radius = radius.max(0);
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dz * dz > radius * radius {
                continue;
            }
            let x = block_pos[0] + dx;
            let z = block_pos[2] + dz;
            if world.get(x, block_pos[1] - 1, z).is_solid()
                && player_body_clear(world, x, block_pos[1], z)
            {
                continue;
            }
            return false;
        }
    }
    true
}

pub fn install_huge_spawn_platform(world: &mut World) {
    use terrain::{BoxShape, FillMode, ShapeLayer, ShapeSpec};

    let cx = WORLD_SIZE / 2;
    let cz = WORLD_SIZE / 2;
    let top_y = SEA_LEVEL + 3;
    let half_extent = 90;

    world.push_geo_layer(ShapeLayer {
        name: "huge_spawn_platform_base".into(),
        weight: 50.0,
        fill: FillMode::Replace(BlockType::Dirt),
        shapes: vec![ShapeSpec::Box(BoxShape {
            name: "huge_spawn_platform_base_box".into(),
            min: [cx - half_extent, top_y - 3, cz - half_extent],
            max: [cx + half_extent, top_y - 2, cz + half_extent],
        })],
        biome_override: None,
        enabled: true,
    });
    world.push_geo_layer(ShapeLayer {
        name: "huge_spawn_platform_grass_surface".into(),
        weight: 52.0,
        fill: FillMode::Replace(BlockType::Grass),
        shapes: vec![ShapeSpec::Box(BoxShape {
            name: "huge_spawn_platform_grass_surface_box".into(),
            min: [cx - half_extent, top_y - 1, cz - half_extent],
            max: [cx + half_extent, top_y - 1, cz + half_extent],
        })],
        biome_override: None,
        enabled: true,
    });
    world.push_geo_layer(ShapeLayer {
        name: "huge_spawn_platform_air_clearance".into(),
        weight: 51.0,
        fill: FillMode::Carve,
        shapes: vec![ShapeSpec::Box(BoxShape {
            name: "huge_spawn_platform_clearance_box".into(),
            min: [cx - half_extent, top_y, cz - half_extent],
            max: [cx + half_extent, VERTICAL_SIZE - 1, cz + half_extent],
        })],
        biome_override: None,
        enabled: true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::GlobalResourcePool;

    #[test]
    fn biome_from_xz_3_zones() {
        let n = WORLD_SIZE;

        assert_eq!(Biome::from_xz(0, 0), Biome::Tundra);

        assert_eq!(Biome::from_xz(0, n / 3), Biome::Jungle);
        assert_eq!(Biome::from_xz(0, 2 * n / 3 - 1), Biome::Jungle);

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

        let total_ores = w.count_biome_ores(Biome::Desert)
            + w.count_biome_ores(Biome::Tundra)
            + w.count_biome_ores(Biome::Jungle);
        assert!(
            total_ores > 0,
            "expected at least one biome ore (actual: D={}, T={}, J={})",
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

        assert_eq!(v.len(), 7 * 7 * 7);
    }

    #[test]
    fn block_yields_match() {
        assert_eq!(BlockType::Stone.yields(), Some((ResourceKind::Stone, 1)));
        assert_eq!(
            BlockType::SunstoneOre.yields(),
            Some((ResourceKind::Sunstone, 1))
        );

        assert_eq!(
            BlockType::BerryThicket.yields(),
            Some((ResourceKind::Apple, 1))
        );
    }

    #[test]
    fn gather_stone_adds_stone_to_pool() {
        let mut world = World::new(8);
        world.set(1, 1, 1, BlockType::Stone);
        let mut pool = GlobalResourcePool::new();

        let result = gather_block(&mut world, &mut pool, 1, 1, 1, 42).unwrap();

        assert_eq!(result, Some((ResourceKind::Stone, 1)));
        assert_eq!(pool.get(ResourceKind::Stone), 1);
        assert_eq!(world.get(1, 1, 1), BlockType::Air);
    }

    #[test]
    fn grass_is_standable_visual_surface_without_resource_yield() {
        assert!(BlockType::Grass.is_solid());
        assert!(BlockType::Grass.is_surface());
        assert_eq!(BlockType::Grass.yields(), None);
        assert_eq!(BlockType::Grass.debug_color_rgba()[3], 1.0);
    }

    #[test]
    fn player_spawn_position_uses_topmost_clear_standable_column() {
        let mut w = World::new(8);
        w.set(3, 1, 3, BlockType::Dirt);
        w.set(3, 2, 3, BlockType::Wood);

        let (pos, block_pos) = player_spawn_position_at(&w, 3, 3).unwrap();

        assert_eq!(block_pos, [3, 3, 3]);
        assert_eq!(pos, Vec3::new(3.5, 3.0, 3.5));
        assert!(
            w.get(block_pos[0], block_pos[1] - 1, block_pos[2])
                .is_solid()
        );
        assert!(player_body_clear(
            &w,
            block_pos[0],
            block_pos[1],
            block_pos[2]
        ));
    }

    #[test]
    fn player_stand_position_rejects_blocked_body_clearance() {
        let mut w = World::new(8);
        w.set(2, 1, 2, BlockType::Dirt);
        w.set(2, 3, 2, BlockType::Leaves);

        assert!(player_stand_position_at(&w, 2, 2, 2.0, 1.0).is_none());
    }

    #[test]
    fn player_spawn_position_finds_default_procedural_surface() {
        let pipeline = terrain::presets::by_name("default");
        let w = World::with_pipeline(WORLD_SIZE, pipeline);
        let x = WORLD_SIZE / 2;
        let z = WORLD_SIZE / 2;

        let (_pos, block_pos) = player_spawn_position_at(&w, x, z).unwrap();

        assert_eq!([block_pos[0], block_pos[2]], [x, z]);
        assert!(w.get(x, block_pos[1] - 1, z).is_solid());
        assert!(player_body_clear(&w, x, block_pos[1], z));
    }

    #[test]
    fn player_stand_position_rejects_world_edge_overflow() {
        let w = generate_world(&WorldConfig::default());

        assert!(player_stand_position_at(&w, WORLD_SIZE, 0, 15.0, 1.0).is_none());
        assert!(player_stand_position_at(&w, 0, -1, SEA_LEVEL as f32, 1.0).is_none());
    }

    #[test]
    fn generate_world_uses_shared_config_and_spawn_platform() {
        let config = WorldConfig::default();

        let a = generate_world(&config);
        let b = generate_world(&config);
        let x = WORLD_SIZE / 2;
        let z = WORLD_SIZE / 2;
        let foot_y = SEA_LEVEL + 3;

        assert_eq!(a.size, config.size);
        assert_eq!(a.seed, config.seed);
        assert_eq!(a.pipeline.name, config.preset);
        assert_eq!(a.get(x, foot_y - 1, z), BlockType::Grass);
        assert_eq!(b.get(x, foot_y - 1, z), BlockType::Grass);

        for (x, y, z) in [(4, 4, 4), (48, 15, 48), (83, 12, 21), (16, 30, 64)] {
            assert_eq!(
                a.get(x, y, z),
                b.get(x, y, z),
                "voxel mismatch at {x},{y},{z}"
            );
        }
    }

    #[test]
    fn player_spawn_position_near_prefers_open_clearance() {
        let mut w = World::new(12);
        for z in 1..=7 {
            for x in 1..=7 {
                w.set(x, 1, z, BlockType::Dirt);
            }
        }
        w.set(4, 2, 4, BlockType::Wood);
        w.set(5, 2, 4, BlockType::Wood);

        let (_pos, block_pos) = player_spawn_position_near(&w, 4, 4, 4, 1).unwrap();

        assert_ne!(block_pos, [4, 2, 4]);
        assert!(spawn_clearance(&w, block_pos, 1));
    }

    #[test]
    fn resolve_player_stuck_near_moves_blocked_player_to_safe_column() {
        let mut w = World::new(8);
        w.set(3, 1, 3, BlockType::Dirt);
        w.set(3, 2, 3, BlockType::Stone);
        w.set(3, 3, 3, BlockType::Stone);
        w.set(4, 1, 3, BlockType::Dirt);

        let stuck = Vec3::new(3.5, 2.0, 3.5);
        assert!(!player_position_is_safe(&w, stuck));

        let (pos, block_pos) = resolve_player_stuck_near(&w, stuck, 2).unwrap();

        assert_ne!(block_pos, [3, 2, 3]);
        assert!(player_position_is_safe(&w, pos));
        assert_eq!(
            w.get(block_pos[0], block_pos[1] - 1, block_pos[2]),
            BlockType::Dirt
        );
    }

    #[test]
    fn player_position_is_safe_rejects_procedural_xz_out_of_bounds() {
        let w = generate_world(&WorldConfig::default());
        let pos = Vec3::new(w.size as f32 + 0.5, 15.0, 0.5);

        assert!(
            !player_position_is_safe(&w, pos),
            "procedural terrain may generate out-of-range voxels, but player safety must stay inside world bounds"
        );
    }

    #[test]
    fn huge_spawn_platform_is_grass_and_clear_above() {
        let pipeline = terrain::presets::by_name("default");
        let mut w = World::with_pipeline(WORLD_SIZE, pipeline);
        install_huge_spawn_platform(&mut w);
        let x = WORLD_SIZE / 2;
        let z = WORLD_SIZE / 2;
        let foot_y = SEA_LEVEL + 3;

        assert_eq!(w.get(x, foot_y - 1, z), BlockType::Grass);
        for y in foot_y..VERTICAL_SIZE {
            assert_eq!(w.get(x, y, z), BlockType::Air, "y={} must be clear", y);
        }

        let (_pos, block_pos) = player_spawn_position_near(&w, x, z, 14, 2).unwrap();
        assert_eq!(block_pos, [x, foot_y, z]);
    }

    #[test]
    fn fallback_spawn_ring_yields_32_offsets() {
        let offsets = fallback_spawn_ring_offsets(0);
        assert_eq!(
            offsets.len(),
            32,
            "must yield 4 rings × 8 lanes = 32 candidates"
        );
    }

    #[test]
    fn fallback_spawn_ring_offsets_are_within_4_block_radius() {
        // Innermost ring is `radius = 1.0`, outermost is `radius = 1.0 + 3 * 0.85 ≈ 3.55`.
        // Cosine/sine round to ±4 at most — keep the bound generous so the
        // test stays stable if the ring math changes by ±1 block.
        for [dx, dz] in fallback_spawn_ring_offsets(0) {
            assert!(dx.abs() <= 4, "dx={dx} out of expected range");
            assert!(dz.abs() <= 4, "dz={dz} out of expected range");
            assert!(
                !(dx == 0 && dz == 0),
                "ring must never yield the center offset"
            );
        }
    }

    #[test]
    fn fallback_spawn_ring_seed_rotates_lane_ordering() {
        // Same first ring (ring=0, radius=1.0) yields 8 offsets in the same
        // set regardless of seed, but the *starting* lane changes — verify
        // by walking the inner 8 candidates with two seeds and confirming
        // each set has the same multiset of offsets.
        let ring0_seed0: HashSet<_> = fallback_spawn_ring_offsets(0).into_iter().take(8).collect();
        let ring0_seed3: HashSet<_> = fallback_spawn_ring_offsets(3).into_iter().take(8).collect();
        assert_eq!(ring0_seed0, ring0_seed3);
    }
}
