# 无限体素模拟基底改造计划

> 日期: 2026-06-07
> 状态: 设计稿，等你拍板
> 范围: XZ 无限延伸 + 视距渲染 + 按需生成。体素是世界状态表达层，不是把玩法目标限定成 Minecraft 复刻。

---

## 1. 现状

- `WORLD_SIZE = 128`（硬边界）
- `GameWorld` 持有一个固定大小 `Vec<u8>` 存所有 voxel
- `spawn_terrain_around_player` 把**整个世界**塞进一个 129³ chunk 给 greedy mesh
- 玩家 XZ 被 clamp 到 [0, WORLD_SIZE]
- 12s loop 跑 128³ 已经够呛，192³ 要 30s，再大就崩

## 2. 目标（体素模拟基底）

- 玩家在 XZ 方向可以走 ±∞，**永远撞不到边界**
- 新地形按需生成（玩家靠近时算，离得远就卸载或不管）
- 渲染只算玩家视野内的方块
- 出生点不变（0,0），但出生后往任意方向走都是新地形

## 3. 三种实现路径

### 方案 A: 假无限（30 分钟）
- `WORLD_SIZE = 4096`（看上去无限，但实际是固定大数组）
- 玩家 clamp 改为 ±2048 而不是 [0, 4096]
- **问题**：greedy mesh 还是 O(4096³) ≈ 5 分钟一次，**完全不可用**
- 结论：**否决**，光改大数字没用

### 方案 B: 真无限稀疏存储 + AABB 渲染（推荐，2-3 小时）
- `GameWorld::voxels: HashMap<(i32,i32,i32), u8>`（稀疏，没生成的位置=空气或按需算）
- 加 `generate_voxel(x,y,z) -> BlockType` 确定性函数（noise + heightmap + biome）
- 玩家 XZ 不 clamp，只 Y clamp 到 [0, 96]
- `spawn_terrain_around_player` 只算 `player ± R` 的 AABB（不是整个世界）
- `greedy_mesh_for_type` 改成接受 AABB 而不是固定世界大小
- 内存按"走过的路"增长，没走过的永远不算 → 实际是无限

**优点**：代码改动聚焦在 2 个文件（`world/mod.rs` + `render/greedy_mesh.rs` + `render/mod.rs`），其余不动
**缺点**：HashMap 慢；走太远会 OOM（demo 可以接受）
**性能预期**：40³ AABB × 12 type ≈ 150ms/re-mesh，跟现在 96³ 一样快甚至更快

### 方案 C: 完整 chunk 系统（4-6 小时）
- 16×128×16 chunks 按 `(cx, cz)` 索引
- `ChunkManager: HashMap<(i32,i32), Chunk>`
- 玩家 ±N chunks 才生成 / 加载（默认 ±4 = 65³ visible）
- 远 chunks 卸载（节省内存）
- 每 chunk 独立 mesh
- 视锥剔除（不看的不 mesh）

**优点**：真正的分块世界架构，内存可控
**缺点**：4-6 小时；改 4-5 个文件；要小心的边界很多（chunk 邻居、跨 chunk mesh、unload 时机）

## 4. 我推荐

**方案 B**（真无限稀疏 + AABB 渲染）。

理由：
- 2-3 小时能搞完
- 解决用户的核心诉求（无限地图）— 100%
- 性能跟当前持平
- 后续要升级到方案 C 也是"在 B 之上加 chunk 抽象"，不是推倒重来
- 内存随行走增长这个代价，demo 阶段可以接受（走 1 小时 = 几千个 cube，HashMap 还撑得住）

## 5. 方案 B 详细设计

### 5.1 数据结构
```rust
// src/world/mod.rs
pub struct World {
    pub voxels: HashMap<IVec3, u8>,  // 稀疏存储：玩家改过的 + 显式 set 的
    pub vertical_min: i32,           // Y=0（基岩）
    pub vertical_max: i32,           // Y=96（天空）
    pub seed: u64,
}

impl World {
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockType {
        if y < self.vertical_min || y >= self.vertical_max { return BlockType::Air; }
        let key = IVec3::new(x, y, z);
        if let Some(&b) = self.voxels.get(&key) { return BlockType::from_u8(b); }
        // 缓存到 map 避免每次都 noise（可选）
        self.generate_voxel(x, y, z)
    }
    
    pub fn set(&mut self, x: i32, y: i32, z: i32, b: BlockType) {
        if y < self.vertical_min || y >= self.vertical_max { return; }
        self.voxels.insert(IVec3::new(x, y, z), b as u8);
    }
    
    pub fn generate_voxel(&self, x: i32, y: i32, z: i32) -> BlockType {
        // 复用 WorldGenerator 现有逻辑：heightmap, biome, cave, ore
        // 确定性：同样 (x, y, z, seed) → 同样结果
        // 内部用 noise3(x/8, 0, z/8, seed) 等
    }
}
```

### 5.2 玩家边界
```rust
// src/render/mod.rs (player_input 或 try_player_move)
fn clamp_player_pos(pos: &mut [i32; 3]) {
    pos[1] = pos[1].clamp(0, 95);  // Y 不能超出垂直范围
    // XZ 不 clamp
}
```

### 5.3 渲染
```rust
// src/render/greedy_mesh.rs
pub fn greedy_mesh_for_type_aabb(
    world: &World, 
    target: BlockType, 
    min: IVec3,  // 例如 player - R
    max: IVec3,  // 例如 player + R
) -> BlockTypeMesh {
    let size = (max - min);  // 40³ 之类
    let total = (size.x as usize) * (size.y as usize) * (size.z as usize);
    let mut voxels: Vec<Vox> = vec![Vox(0); total];
    // 填充
    for z in 0..size.z {
        for y in 0..size.y {
            for x in 0..size.x {
                let wx = min.x + x;
                let wy = min.y + y;
                let wz = min.z + z;
                let b = world.get(wx, wy, wz);
                let i = (x as usize) + (y as usize) * (size.x as usize) 
                       + (z as usize) * (size.x as usize) * (size.y as usize);
                voxels[i] = if b == target { Vox(1) } else { Vox(0) };
            }
        }
    }
    // greedy_quads 用 size 数组作为 shape（用 ConstShape3u32<...> 但 size 在 runtime 变 → 不可行）
    // 替代方案：写一个新的 AABB greedy mesher，不依赖 ConstShape3u32
    ...
}
```

**问题**：greedy_quads 0.2 需要 `impl ConstShape<...>` 类型，runtime shape 不可行。
**解决**：
- 选 R 固定（比如 20）→ 41³ 是编译期常量 → 用 `ConstShape3u32<41, 41, 41>`
- 玩家位置影响 min/max offset，但 shape size 不变
- 这样不需要 runtime shape

### 5.4 改动文件清单

| 文件 | 改什么 |
|------|--------|
| `src/world/mod.rs` | `World::voxels: Vec<u8>` → `HashMap<IVec3, u8>`；加 `generate_voxel`；去掉 `size` 字段 |
| `src/world/mod.rs` | `WorldGenerator::generate` 改为只生成出生点周围 ±N（比如 ±4）= 9³ 小方块（用于初始 spawn 周围），其余用 `generate_voxel` 按需 |
| `src/render/greedy_mesh.rs` | 新增 `greedy_mesh_for_type_aabb` 用 `ConstShape3u32<41, 41, 41>` (R=20) |
| `src/render/mod.rs` | `spawn_terrain_around_player` 改用 AABB 版本，迭代 `player ± R` |
| `src/render/mod.rs` | 玩家 XZ 不 clamp（try_player_move 删 XZ clamp） |
| `src/creature/mod.rs` | `pasture_spawn` 用 `generate_voxel` 找地面（不再依赖固定 size） |
| `src/constant/mod.rs` | 删 `WORLD_SIZE`（或改为 `VERTICAL_MAX = 96`） |

### 5.5 不改的
- `pretty/`（云、树、动物）— 已经在用 world.get(), 改完自动能用
- `monster/` — 同样
- HUD、camera、input — 全部不碰

## 6. 性能预期

- 生成 1 voxel: noise3 + 几次比较 = ~200ns
- 玩家 ±20 AABB = 41³ = 68K voxels, 12 type → 12 * 68K = 820K voxel lookups → 12 * 68K * 200ns = 1.6s/re-mesh
- HashMap.get 比 Vec 慢 3-5x → 5-8s/re-mesh（慢了）

**优化**：
- `generate_voxel` 加 LRU cache（最近访问的 XZ 列缓存高度，避免重复算）
- 或：预生成出生点周围 ±4 chunks（缓存进 HashMap），其余按需

**实际预期**：1-2s/re-mesh，1.5s throttle 刚好够

## 7. 验收

- 启动后玩家在 (64, 15, 64)
- WASD 走 1000 步到 (1000, ?, ?)，没崩溃
- 截图能看到玩家已经走了 1000m（不是停在边界）
- `diff.json` 里 `vs_iter` 正常
- 重新启动游戏回到 (64, 15, 64)，旧位置地形因为是 deterministic noise 还能看到（虽然 HashMap 清空了）
- `cargo test` 不挂

## 8. 时间估算

| 阶段 | 时间 |
|------|------|
| World 改 HashMap + generate_voxel | 30 min |
| greedy_mesh AABB 版本 | 30 min |
| spawn_terrain_around_player 改 AABB | 15 min |
| 玩家 XZ 不 clamp | 5 min |
| creature/monster 适配 | 15 min |
| 测试 + 调 | 30 min |
| **总计** | **~2.5 小时** |

## 9. 风险

- 旧 `GameWorld::size` 字段被多处使用，删要小心（player 边界、creature spawn、pretty、worldgen）
- 噪声函数 `noise3(x, y, z, seed)` 之前是 O(world²) 用 `x, z` 循环算 heightmap，现在改成单点查询要保证返回一致
- HashMap 内存：走太远会涨。要 demo 阶段加个 "hard cap 100k entries" 防止 OOM

## 10. 下一步

**你拍板：**
- 走方案 B（推荐，2-3 小时真无限）
- 还是方案 A 假无限（30 分钟凑合，但 4096³ 实际跑不动）
- 还是方案 C 完整 chunk 系统（4-6 小时但架构干净）

我个人强烈推 B — 目标是无限、可编辑、可审计的体素模拟基底，B 能先给到核心体验，C 是后续工程化升级。

要不要我现在就开干方案 B？
