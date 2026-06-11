# 可配置地形生成系统（斗魂大乱斗风格）

> 日期: 2026-06-07
> 状态: 设计稿
> 目标: 把"硬编码 generate_voxel"换成"模块化 pipeline + preset 配置 + 随机选择"

---

## 1. 核心抽象

### 1.1 `TerrainModule` trait
每个模块是"看一个 voxel 位置，根据规则决定它是什么 block"的纯函数。

```rust
pub trait TerrainModule: Send + Sync {
    fn name(&self) -> &str;
    fn weight(&self) -> f32 { 1.0 }  // 同位置多模块冲突时的优先级
    
    /// 决定这个位置放什么 block
    /// - `ctx`: 上下文 (biome, height, neighbor info)
    /// - 返回 `Some(block)`: 放这个 block
    /// - 返回 `None`: 这个位置我不管，让下一个模块决定
    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType>;
}

pub struct TerrainContext {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub biome: Biome,
    pub surface_y: i32,    // 计算出的地表高度
    pub cave_y: i32,       // 洞穴 threshold
    pub seed: u64,
}
```

### 1.2 具体模块（每个独立文件）

```rust
// src/world/terrain/heightmap.rs
pub struct HeightmapModule {
    pub base_height: f32,         // SEA_LEVEL 默认 12
    pub amplitude_big: f32,       // 大尺度山脉振幅 14
    pub frequency_big: f32,       // 大尺度噪声频率 1/8
    pub amplitude_detail: f32,    // 细节振幅 4
    pub frequency_detail: f32,    // 细节频率 1
    pub biome_bias: BiomeBias,    // Tundra/Jungle/Desert 各自偏移
    pub surface_blocks: HashMap<Biome, BlockType>,  // 表层用什么
}

// src/world/terrain/cave.rs
pub struct CaveModule {
    pub density: f32,             // 0..1, > 此值挖空
    pub scale: f32,               // 噪声尺度 1/4
    pub min_y: i32,               // 不挖空上限
}

// src/world/terrain/water.rs
pub struct WaterFillModule {
    pub sea_level: i32,
}

// src/world/terrain/ore.rs
pub struct OreModule {
    pub ore: BlockType,
    pub frequency: f32,           // 单位体积出现概率
    pub vein_size: i32,           // 簇半径
    pub min_y: i32,
    pub max_y: i32,
    pub biome: Option<Biome>,     // 只在某个 biome 出
}

// src/world/terrain/tree.rs
pub struct TreeModule {
    pub trunk: BlockType,
    pub leaves: BlockType,
    pub density: f32,             // 每格生成树概率
    pub min_trunk_height: i32,
    pub max_trunk_height: i32,
    pub canopy_radius: i32,
    pub biome: Option<Biome>,
}

// src/world/terrain/noise_paint.rs
pub struct NoisePaintModule {
    pub target: BlockType,        // 替换这种
    pub replacement: BlockType,   // 替换成这种
    pub scale: f32,               // 噪声控制替换密度
    pub threshold: f32,
}
```

### 1.3 `TerrainPipeline`

```rust
pub struct TerrainPipeline {
    pub name: String,
    pub modules: Vec<Box<dyn TerrainModule>>,
    pub vertical_min: i32,
    pub vertical_max: i32,
}

impl TerrainPipeline {
    /// 主入口：决定 (x, y, z) 是什么 block
    pub fn generate(&self, x: i32, y: i32, z: i32) -> BlockType {
        if y < self.vertical_min || y >= self.vertical_max { return BlockType::Air; }
        let mut ctx = TerrainContext::new(x, y, z, self.seed);
        for module in &self.modules {
            if let Some(block) = module.decide(&mut ctx) {
                return block;
            }
        }
        BlockType::Air
    }
}
```

### 1.4 `Preset` 配置

```rust
// src/world/terrain/presets.rs
pub struct Preset {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub pipeline: TerrainPipeline,
    pub tags: Vec<String>,  // "default", "lold_arena", "flat", "mountainous"
}

pub fn default_preset() -> Preset { ... }
pub fn flat_arena_preset() -> Preset { ... }      // 全平地
pub fn mountainous_preset() -> Preset { ... }     // 高山
pub fn lold_arena_preset() -> Preset { ... }      // 斗魂大乱斗风格
pub fn random_preset() -> Preset { ... }          // 随机选一个

// 注册表
pub fn all_presets() -> Vec<Preset> { vec![
    default_preset(),
    flat_arena_preset(),
    mountainous_preset(),
    lold_arena_preset(),
] }
```

### 1.5 CLI 集成

```rust
// src/main.rs
#[derive(Parser)]
struct Cli {
    /// 地形 preset：default / flat_arena / mountainous / lold_arena / random
    #[arg(long, default_value = "default")]
    preset: String,
}

// setup_world 时：
let preset = match cli.preset.as_str() {
    "default" => default_preset(),
    "flat_arena" => flat_arena_preset(),
    "mountainous" => mountainous_preset(),
    "lold_arena" => lold_arena_preset(),
    "random" => random_preset(),  // 每局随机
    name => return Err(format!("unknown preset: {name}")),
};
*game_world = World::from_preset(preset);
```

---

## 2. 斗魂大乱斗"模式实现

`lold_arena` preset 怎么生成（每次不一样）：

```rust
pub fn lold_arena_preset(rng: &mut impl Rng) -> Preset {
    let mut pipeline = TerrainPipeline {
        name: format!("lold_arena_{}", rng.gen_range(0..10000)),
        vertical_min: 0,
        vertical_max: 96,
        modules: vec![],
    };
    
    // 1) 随机地形：可能全平、可能高山、可能湖泊
    match rng.gen_range(0..3) {
        0 => pipeline.modules.push(Box::new(HeightmapModule {
            base_height: 12.0,
            amplitude_big: rng.gen_range(0.0..25.0),
            ..default()
        })),
        1 => pipeline.modules.push(Box::new(FlatHeightmapModule {})),  // 全平 SEA_LEVEL+1
        _ => pipeline.modules.push(Box::new(HeightmapModule {
            amplitude_big: 25.0,
            frequency_big: 1.0 / 16.0,  // 缓坡
            ..default()
        })),
    }
    
    // 2) 随机洞穴密度
    pipeline.modules.push(Box::new(CaveModule {
        density: rng.gen_range(0.0..0.85),
        ..default()
    }));
    
    // 3) 随机加点料 (50% 概率加树 / 加矿 / 加浆果)
    if rng.gen_bool(0.5) {
        pipeline.modules.push(Box::new(TreeModule {
            density: rng.gen_range(0.001..0.05),
            ..default()
        }));
    }
    if rng.gen_bool(0.3) {
        pipeline.modules.push(Box::new(OreModule {
            ore: pick_random_ore(&rng),
            frequency: rng.gen_range(0.001..0.01),
            ..default()
        }));
    }
    
    pipeline.modules.push(Box::new(WaterFillModule::default()));
    Preset { name: pipeline.name.clone(), pipeline, .. }
}
```

这样每次 `--preset=random` 跑起来地形都不同。

---

## 3. 落地步骤

### Phase 1: trait + 现有逻辑迁过来（1 小时）
- 新建 `src/world/terrain/mod.rs`，定义 `TerrainModule` trait + `TerrainContext`
- 把现有 `World::generate_voxel` 的逻辑拆成 `HeightmapModule` + `CaveModule` + `WaterFillModule`
- `World::generate_voxel` 改成调用 `default_pipeline().generate(x, y, z)`

### Phase 2: Preset 系统（30 分钟）
- 新建 `src/world/terrain/presets.rs`
- 实现 3-4 个 preset：default / flat_arena / mountainous / random
- World 接受 preset 构造

### Phase 3: CLI 集成（15 分钟）
- 用 clap 或者简单 match 解析 `--preset=xxx`
- setup_world 用 preset 替换硬编码的 `WorldGenerator::default().generate(...)`

### Phase 4: random preset 实现（30 分钟）
- `lold_arena` preset 随机选地形
- `--preset=random` 跑起来

**总计 2-2.5 小时**

---

## 4. 用户怎么用

```powershell
# 默认地形（和现在一样）
cargo run -p lk2-client -- --offline

# 全平地图（适合造建筑）
cargo run -p lk2-client -- --offline --preset=flat_arena

# 高山地图
cargo run -p lk2-client -- --offline --preset=mountainous

# 斗魂大乱斗（每局不同地形）
cargo run -p lk2-client -- --offline --preset=random
```

未来扩展（不在这次范围）：
- JSON 配置文件 `presets/*.json` 让用户自己写
- 运行时切换 preset（按 F8）
- 地形种子可视化（按 F9 打印当前 preset 配置）

---

## 5. 风险 & 缓解

| 风险 | 缓解 |
|------|------|
| trait object 性能开销 | 每个 voxel 调一次，开销 ~50ns；41³ × 12 = 70K cells × 50ns = 3.5ms（可接受） |
| 模块顺序冲突 | 用 `weight()` 排序；冲突时高 weight 胜 |
| 现有 `WorldGenerator::generate(size)` 还要不要保留？ | **保留**用于 spawn 周围 ±64 的预生成 features（树/矿），外面用 pipeline 按需 |
| 用户写自定义 preset 的可配置性边界 | Phase 4 不做 JSON，先用 Rust 函数，迭代后再加 |

---

## 6. 下一步

要不要我现在开干 Phase 1（trait + 模块）？
