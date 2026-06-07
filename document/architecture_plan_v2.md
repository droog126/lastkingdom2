# 万国起源 2.0 — 计划书：从"能跑"到"百万实体级"现代架构演进

> **作者**：Mavis (orchestrator)
> **版本**：v0.2（2026-06-06 决策版）
> **状态**：1-4 决策已锁，**P1 启动中**

---

## 〇、为什么做这个

`lastkingdom2` 当前已经是一台能跑的 demo：96³ 体素世界、3 群落、12 种方块、玩家 avatar + 怪物 AI + 国家系统 + 资源池、自截图自检、160 fps。

但当我们看一份《Minecraft 与现代游戏架构对比》的技术 essay，**会发现当前实现距离"现代游戏架构"还有 6 个明显的代差**：

| 维度 | 当前实现 | 现代架构 | 性能上限 |
| --- | --- | --- | --- |
| **ECS** | Bevy 0.18.1 已是 Archetype ECS | Archetype + SoA | 已是 |
| **DOD / 缓存布局** | `Vec<BlockType>`（AoS 稠密）+ 每块一个 entity | 列式存储 + 1 mesh/chunk | 200 ms 帧时间抖动 |
| **并行** | 全部单线程 tick（1Hz sim） | Job System + 任务图 | 8 核用 1 核 |
| **确定性** | 哈希噪声是确定性的，但 RNG 是 `rand::thread_rng()`，浮点用 f32 | 固定点数学 + PCG / SplitMix | 不可重放、不可同步 |
| **现代渲染** | CPU per-block entity spawn | Chunk Mesh + GPU-driven 剔除 | 移动时帧时间翻倍 |
| **模组 / 扩展** | Java 版 hack 字节码；我们目前没设计插件机制 | 注册 Component + System，热插拔，跨平台编译 | 扩展门槛高、生态分裂 |

这份计划的目标：**用 7 个阶段、约 9-13 周的实际工程量，把这个 demo 推到"百万级模拟 + 60 fps 稳定 + 可热插拔扩展 + 可重放可同步"的水位，同时不破坏现有的自闭环工作流。**

---

## 一、设计原则

1. **不重写，只演进**。每个阶段必须能独立 build、独立跑、保留 `loop.ps1` 闭环可见。
2. **阶段之间不破坏接口**。`World::get(x,y,z)` 这种公共 API 在所有阶段都必须能编译（即使内部从稠密数组换成 chunk 表）。
3. **每阶段必须有可量化指标**。闭环比对：旧 `iter_NN.png` vs 新 `iter_NN.png`；旧 `state_NN.json` 字段在新版本中必须仍存在。
4. **Bevy 0.18 的 ECS 红利能用就用**。Bevy 的 scheduler、change detection、ParallelCommands 已经是 Job System 的雏形，**不要自己造轮子**。
5. **不要 bump `compt`**。`Cargo.toml` 锁的 `compt = ">=1.9, <1.10"` 是 broccoli 0.6 的硬约束，任何 PR 试图 bump 它都拒收。
6. **VR 是 P4 确定性的强驱动**。VR 对延迟极敏感，确定性模拟是低延迟多人 VR 的硬基础。**P4 不允许在确定性上妥协**。

---

## 二、当前状态快照（基线）

来自最后一次 `loop.ps1`（2026-06-06 15:39）：

| 指标 | 值 |
| --- | --- |
| 世界尺寸 | 96³（已从 32³ 扩到 96³） |
| 同屏方块数 | ~3000（受 `max_blocks` 限制） |
| FPS | 160 |
| tick 频率 | 1 Hz（`SLOW_TICK_SECS`） |
| TickObserver 不变量违例 | 0 |
| 单元测试 | 50 通过 |
| 模块数 | 11（main / world / render / pretty / nation / monster / ai / scenario / resource / creature / constant / utils） |
| 已用 crate | bevy 0.18.1、avian3d 0.5、broccoli 0.6、sepax2d 0.3、bevy-inspector-egui 0.36、rand 0.8、crossbeam-channel、serde、serde_json |

**已实现的现代架构组件（不算白做）：**
- ✅ ECS：entity / component / system 三件套（Bevy 原生）
- ✅ 资源即插件：`GlobalResourcePool / NationRegistry / MonsterEcosystem / TickObserver` 都是 `Resource`
- ✅ 确定性世界生成：`hash01` + trilinear noise
- ✅ 自动化闭环：`loop.ps1` + 截图 + JSON 状态

**还没动的部分（要补的课）：**

| 位置 | 现状 | 痛点 |
| --- | --- | --- |
| `src/world/mod.rs::World` | 稠密 `Vec<BlockType>`（AoS） | 全量扫一遍要 96³ ≈ 88 万次访问；块级访问不连续 |
| `src/render/mod.rs::spawn_terrain_around_player` | 玩家每移动 1 格 → 销毁 3000 entity → 重 spawn 3000 entity | GC 抖动 + 状态切换风暴 |
| `src/main.rs::simulation_tick` | 1 Hz 串行 | 怪物 AI、资源再生串在一起 |
| `src/utils/random.rs` | 用 `rand::thread_rng()` | 不可重放、跨平台不一致 |
| `src/main.rs::simulation_tick` 中浮点：`tick % 10 == 0` 之类用 `i64` 但位置用 `f32` | 浮点路径 | 确定性时定不下来 |
| 渲染 | CPU 提交 3000 个 draw call | GPU 空转 |
| 物理 | avian3d 引入但未挂载 | 物理碰撞没真用上 |
| 模组 / 插件 | 无 | 没有 plugin 机制 |

---

## 三、阶段路线图

```
  P1 数据布局     P2 渲染粗化    P3 并行化+物理  P4 确定性(VR) P4.5 热插拔 P5 实体规模  P6 渲染现代化
 ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
 │ World→   │→ │ Spawn 1  │→ │ System   │→ │ PCG +    │→ │ SimPlugin│→ │ SoA 化   │→ │ Frame    │
 │ Chunked  │  │ Mesh per │  │ Parallel │  │ Replay   │  │ trait +  │  │ Entity   │  │ Graph +  │
 │ SoA      │  │ Chunk    │  │ + Task   │  │ (固定点   │  │ 动态     │  │ Archetype│  │ GPU      │
 │          │  │          │  │ Graph    │  │  P4.5)   │  │ 注册     │  │ Columnar │  │ Driven   │
 │          │  │          │  │ + avian3d│  │          │  │          │  │          │  │ (等0.19) │
 └──────────┘  └──────────┘  └──────────┘  └──────────┘  └──────────┘  └──────────┘  └──────────┘
   2 周          1.5 周        2 周          2 周         1 周          1.5 周       2 周（待启动）
```

每个阶段结束时 demo 都能跑、loop.ps1 都能迭代、阶段 KPI 全部达成后才进下一阶段。

**P3 起接 avian3d 物理**（决策 3）：怪物碰撞 / 玩家重力 / 方块物理感 在 P3 阶段就实装，物理成为 sim 的 first-class 公民。

**P4 确定性是 VR 入口**（决策 4）：VR demo 排期进入视野后，P4 不允许延期 / 不允许在确定性上做妥协。

**P4.5 插件机制**（决策 1）：1 周换未来 5 周的 modding 基建，独立可验证。

**P6 渲染现代化 延后到 Bevy 0.19**（决策 2）：等 meshlet / indirect draw API 稳定后再启动，避免做两次。

---

## 四、各阶段详细设计

### P1：World 切分 Chunk + 列式存储（2 周） ← **当前阶段**

**目标**：把 `Vec<BlockType>` 拆成 16×16×16 的 chunk，每个 chunk 内部用位图（id）+ 调色板（palette）压缩。访问接口保持 `world.get(x,y,z)` 不变。

**为什么先做这个**：
- 这是后面所有阶段的地基。Job System 想并行必须按 chunk 切；GPU-driven 渲染必须按 chunk 出 mesh；SoA 化必须先有列。
- 不影响视觉，纯架构层重构。

**代码改动**：
- 新文件 `src/world/chunk.rs`
  - `pub struct Chunk { palette: Vec<BlockType>, indices: Vec<u8>, dirty: bool }`
  - `pub fn get(&self, local: [u8;3]) -> BlockType`
  - `pub fn set(&mut self, local: [u8;3], b: BlockType)` — 调色板满 256 时升级到 u16 indices
  - `pub fn for_each_solid<F: FnMut(usize, BlockType)>(&self, mut f)` — 列式扫描，跳过 Air
- 改 `src/world/mod.rs::World`
  - `chunks: HashMap<ChunkPos, Chunk>` 替代 `blocks: Vec<BlockType>`
  - `get/set/in_bounds` 委派给 chunk
  - **保留** `for_each_solid` 接口（用迭代器 + chunk 分块）
- 改 `src/world/mod.rs::WorldGenerator::generate`
  - 不再 fill `Vec<BlockType>`，改用 `chunks.entry(pos).or_insert_with(...).set(local, b)`
  - 多 chunk 之间的边界（地形连续）通过 chunk 邻接查询处理

**不变量**：
- `for_each_solid` 调用次数不变；产出的 `(x, y, z, BlockType)` 序列 byte-equal（旧实现 vs 新实现）
- `state_t*.json` 中的 `world.size` 字段不变

**KPI（闭环比对）**：
- `cargo test --workspace` 全绿（旧测试零修改）
- `loop.ps1` 输出 `iter_*.png` 与基线像素级接近（地形看起来一样）
- `bench_world_scan`（新加）— 全 96³ `for_each_solid` 用时 < 旧实现 70%

**风险**：
- 哈希噪声生成在 chunk 边界可能出现 1-voxel 接缝。预案：边界处做一次 8-邻域平滑
- 玩家跨 chunk 移动时 `spawn_terrain_around_player` 可能双计数。预案：先 `HashSet<ChunkPos>` 去重

---

### P2：Chunk Mesh 替代 Entity-per-Block（1.5 周）

**目标**：每个 chunk 渲染时只有 **1 个 entity**（1 个 `Mesh3d` + 1 个 `MeshMaterial3d`），不再 spawn 3000 个独立 cube。

**为什么**：
- 玩家移动 → 旧实现销毁 3000 + spawn 3000 = 6000 次 entity 操作 → 帧时间从 5ms 跳到 30ms+
- 新实现：玩家移动 → 最多重新生成 8 个 chunk 的 mesh（跨边界时），每个 chunk ~500 三角形，总 draw call 从 3000 降到 8

**代码改动**：
- 新文件 `src/render/chunk_mesh.rs`
  - `pub fn build_chunk_mesh(chunk: &Chunk, neighbors: [&Chunk; 6]) -> Mesh` — 经典 greedy meshing（合并相邻同色面）
  - 仅渲染面对空气的可见面（face culling）
  - 半透明（水）单独一 pass，单独一个 mesh
- 改 `src/render/mod.rs`
  - 删除 `spawn_terrain_around_player` 中循环 spawn 的部分
  - 新增 `chunk_mesher_system`：每帧检查玩家所在 chunk 是否变 → 重 mesh
  - `SpawnedBlocks` 改名为 `SpawnedChunkMeshes`，存 `HashMap<ChunkPos, Entity>`
- 新增 `src/render/materials.rs` — 调色板材质，每个面用顶点色替代 per-block material（一次 material 切换，零状态切换）

**不变量**：
- 旧 `state_t*.json` 中所有字段保留
- `iter_*.png` 视觉等效（水半透明、方块颜色、阴影朝向一致）

**KPI**：
- `iter_*.png` 截图与基线对比，PSNR ≥ 35 dB（视觉无明显差异）
- 玩家持续移动时帧时间方差 < 20%（旧实现方差 > 100%）
- 移动时 CPU 占用 < 5%（旧实现峰值 > 30%）

**风险**：
- greedy meshing 出 bug 会出现面缺失。预案：先实现朴素 meshing（不合并），KPI 达成后再优化
- 水的 alpha blend 与不透明方块的渲染顺序。预案：双 sub-pass + `RenderLayers`

---

### P3：System 并行化 + 任务图 + 物理实装（2 周）

**目标**：
1. 把当前 1 Hz 的串行 tick 拆成 4 个独立 system，让 Bevy scheduler 自动并行。
2. **挂上 avian3d 物理**（决策 3）：玩家重力、怪物碰撞、方块刚体。

**为什么**：
- 怪物 AI 决策（30 ms/60 个体）、资源再生（5 ms）、国旗状态机（2 ms）、日志记录（1 ms）之间无数据依赖 → 可以并行
- Bevy 0.18 的 `IntoScheduleConfigs` 支持 `.in_set(SystemSet)` 和 `.before()` / `.after()`，相当于轻量级任务图
- 物理作为 first-class 公民是 P5 大规模实体（怪物有质量、玩家有重力）和 P5+ 流体模拟的基础

**代码改动**：
- 改 `src/main.rs`
  - 新增 `#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)] enum SimSet { Input, AiDecision, ResourceRegen, FlagState, PhysicsStep, Record }`
  - `simulation_tick` 拆为：
    - `monster_ai_decision_system` — 放 `SimSet::AiDecision`
    - `resource_regen_system` — 放 `SimSet::ResourceRegen`
    - `nation_state_machine_system` — 放 `SimSet::FlagState`
    - `tick_recorder` — 放 `SimSet::Record`，依赖前三者
  - 在 `add_systems(Update, ...)` 中配 `.in_set(SimSet::X)` + 顺序约束
- 改 `src/monster/mod.rs::MonsterEcosystem::tick`
  - 拆为 `plan_phase` 和 `apply_phase` — `apply_phase` 之前必须等所有 `plan_phase` 跑完（cross-system barrier）
- 新增 `src/utils/par_chunks.rs`
  - `pub fn for_each_chunk_par<F: Fn(ChunkPos, &Chunk) + Send + Sync>(...)` — 用 `rayon` 或 `bevy::tasks::ComputeTaskPool`
- **物理实装**（新）
  - 在 `setup_world` 中给玩家 avatar entity 加 `RigidBody::Dynamic` + `Collider::capsule`
  - 怪物 entity 加 `RigidBody::KinematicVelocityBased` + `Collider::cuboid`
  - 方块 entity 加 `RigidBody::Fixed` + `Collider::cuboid`（生成时）
  - 在 `SimSet::PhysicsStep` 跑 `PhysicsSet::Step`
  - `src/main.rs` 启动时 `app.add_plugins(PhysicsPlugins::default())`
- 玩家移动 system 改用 `LinearVelocity` 而不是直接改 `Transform.translation`
- 怪物 AI 决策产出"想要的速度向量" → 在 `apply_phase` 通过 `LinearVelocity` 设置

**Cargo 改动**：
- `bevy_tasks` 已在 bevy 内
- `avian3d` 已在 Cargo.toml

**不变量**：
- 同样输入序列下，所有 sim 输出（资源数、个体数、国旗数、玩家位置）byte-equal
- `tick_recorder` dump 的 `state_t*.json` 与旧实现一致
- 玩家从出生点掉落高度 < 5 格（防止物理 bug 让玩家穿地）

**KPI**：
- 单 tick wall time 从 ~40ms 降到 < 15ms（4 核机器）
- `cargo bench sim_tick`（新加）— 1000 tick benchmark
- 玩家站在地上 30 秒不抖动（物理稳定）

**风险**：
- 隐式数据竞争：例如怪物 AI 决策改 `MonsterEcosystem.current_individuals`，同时 `tick_recorder` 读它。预案：用 `Resource` + 所有写都在 `apply_phase` 集中
- Bevy 的并行 system 在 `ResMut` 上有自动 conflict detection，但跨 `Resource` 字段的细粒度竞争检测不到。预案：写一个 `conflict_checker_system` 在 debug build 中做 shadow check
- avian3d 与 Bevy 0.18 的兼容性问题。预案：先跑一个 1 天的 spike（一个 RigidBody 立方体掉到方块地面上）确认能工作再全面接入
- 物理稳定性 vs determinism 冲突：avian3d 内部用浮点，**P3 阶段接受物理不参与确定性**；到 P4 时只确定性化 sim 层，物理是"渲染"层的不确定分量

---

### P4：确定性模拟内核（2 周）

**目标**：把 sim 层的随机源和数值路径全部换成确定性版本，做到"同 seed + 同 input 序列 → 同 state byte-equal"。

**为什么**：
- 当前的 `rand::thread_rng()` 在不同线程、不同 OS、甚至不同 Rust 版本下序列都不同
- 浮点 f32 在跨平台时（例如 ARM NEON vs x86 SSE）累积误差会偏
- 确定性是后面"输入同步 → 多客户端零延迟" + **VR demo 多人场景**的硬基础

**VR 优先级**（决策 4）：
- P4 不允许在确定性上妥协。VR 模式需要 90+ Hz 渲染 + < 20ms motion-to-photon 延迟
- 确定性让"本地预测 + 服务器校验"成为可能，避免 VR 头显的"延迟漂移"感
- P4 KPI 中"跨平台 byte-equal"是硬指标

**代码改动**：
- 改 `src/utils/random.rs`
  - 新增 `pub struct PcgRng(u64)` — PCG-XSH-RR 实现
  - `pub fn with_seed(seed: u64) -> Self`
  - 替换 `rand::thread_rng()` 的所有调用点（用 `cargo clippy` 找）
  - 提供 `serde::Serialize` 用于 tick 录制
- 改 `src/main.rs`
  - `SimClock` 加 `seed: u64` 字段（从 `Scenario` 读）
  - 启动时把 `PcgRng` 作为 `Resource` 注入
- 改 `src/world/mod.rs`
  - 地形生成噪声已经是确定性的（`hash01`）— 不动
  - 矿脉簇位置用 `PcgRng`（旧用 `thread_rng`）— **找出旧调用并替换**
- 改 `src/ai/mod.rs`
  - 玩家 AI 决策和怪物 AI 决策都用 `PcgRng::with_seed(tick)`
  - 把 `tick` 作为 RNG 序列的一部分 → 重放只需要 tick 数
- 新增 `src/sim/fixed_point.rs`（P4 末段，可选）
  - `pub struct Fp32(u32)` — 24-bit 整数 + 8-bit 小数（适合位置 / 速度这类精度需求低的量）
  - 只在 sim 内部用；渲染时再转 f32
  - 优先级：**先不动浮点**，P4 阶段先把 RNG 确定性做完，固定点数学列为 P4 末段

**测试新增**：
- `src/utils/random.rs` 测试 — 同样 seed 输出 1000 个数，跨平台 byte-equal（用 snapshot test）
- `tests/determinism.rs` — 跑 100 tick 两次，state_t*.json diff = 0

**KPI**：
- `cargo test determinism` 全过
- `loop.ps1` 跑两次（同 scenario、同 seed），state_t*.json byte-equal
- `loop.ps1` 跑两次（不同 seed），怪物分布 / 矿脉位置不同

**风险**：
- Bevy 内部的随机性（例如 entity id 分配）会污染确定性。预案：所有需要稳定的 id 用 `#[derive(Component)] struct StableId(u64)`
- 浮点 f32 的累积误差。P4 末段接受 / 不接受看 P4 进度

---

### P4.5：热插拔 System 注册器（1 周） ← **新插入**

**目标**：实现"plugin = 声明 Component + System + SimSet" 的注册机制，让 demo 能力可热插拔。

**为什么**：
- essay §4 "模组从黑客艺术变标准工程" 的核心抓手
- 有了它，P5 之后我们再做"玩家社区贡献 plugin"就有基建
- 不影响 P1-P3 的架构稳定性，可以后置
- 1 周换未来 5 周的 modding 基建，划算

**代码改动**：
- 新文件 `src/sim/plugin.rs`
  - `pub trait SimPlugin: Send + Sync {`
  - `    fn name(&self) -> &str;`
  - `    fn register(&self, app: &mut App);`
  - `    fn dependencies(&self) -> Vec<&str> { vec![] }`
  - `}`
- 新文件 `assets/plugins/manifest.toml` — 插件清单
  ```toml
  [[plugin]]
  name = "gravity_sand"
  path = "plugins/gravity_sand.rs"
  enabled = true
  sim_set = "PhysicsStep"
  ```
- 新文件 `src/sim/plugin_loader.rs`
  - 启动时读 `assets/plugins/manifest.toml`
  - 按依赖拓扑排序，依次 `app.add_plugins(MyPlugin)`
  - 支持 `--plugin <name>` CLI flag 启用 / `--no-plugin <name>` 禁用
- 写示例 plugin：`assets/plugins/gravity_sand.rs`
  - 沙方块（`BlockType::Sand`）在每个 tick 末尾，如果下方是空气 / 水，则下坠一格
  - 注册 `gravity_sand_system` 到 `SimSet::PhysicsStep`
- 集成到 `src/main.rs` — Startup 时调 `plugin_loader::load_all(app)`

**不变量**：
- 不挂任何 plugin 时，sim 输出与 P4 完全一致
- 默认 manifest 关掉所有示例 plugin，回归 P4 行为

**KPI**：
- 写一个测试 `tests/plugin_loader.rs`：注册 3 个 plugin（其中 1 个故意制造依赖环），验证拓扑排序抛错
- demo 验证：开 `gravity_sand` → 把玩家脚下挖空 → 看到沙方块逐 tick 下落 → 关闭 plugin → 沙方块静止
- `cargo test --workspace` 全过
- `loop.ps1` 截图对比：开 / 关 plugin 的 iter_*.png 视觉差异符合预期

**风险**：
- 插件隔离性：plugin 1 不能访问 plugin 2 的内部 Component。预案：靠 Rust 类型系统（每个 plugin 自己的 mod 空间）
- 插件崩溃：plugin panic 整个 app 挂。预案：每个 plugin 的 system 用 `catch_unwind` 包裹（debug build），错误时 disable plugin

---

### P5：SoA 化实体 + Archetype 优化（1.5 周）

**目标**：把怪物的"每实体一个 Entity + Transform + MonsterAi component"模式，重组为"每怪物类型一个 Archetype + 列式存储"。

**为什么**：
- 当前怪物用 Bevy entity 表达，60 个怪物 = 60 个 entity。1000 个怪物 = 1000 个 entity → scheduler overhead 涨
- Archetype ECS 在同 archetype 内做 SoA 是自然演进：Bevy 的 `Table` 已经是列存，但跨 archetype 的 archetype graph traversal 仍然是开销
- 我们的 sim 没那么动态（怪物类型是固定 5 种），适合用一个紧凑的 `Monsters { archetypes: [Archetype; 5] }` 显式 SoA

**代码改动**：
- 新文件 `src/monster/storage.rs`
  - `pub struct MonsterArchetype { alive: Vec<bool>, pos_fp: Vec<Fp32>, hp: Vec<i32>, ai_state: Vec<AiState>, target: Vec<Option<EntityId>> }`
  - 列式布局，迭代 `for i in 0..alive.len() { if alive[i] { ... } }`
  - 死亡时把最后一个 swap 到 i → O(1) 删除
- 改 `src/monster/mod.rs`
  - 保留 `MonsterEcosystem` 的对外 API（`current_individuals`、`tick(&mut pool)`）
  - 内部改用 `Monsters { archetypes: [Archetype; 5] }` 替代 `Vec<Monster>`
- 改 `src/ai/mod.rs`
  - AI 决策遍历从"对每个 entity 决策"改为"对每个 archetype 批量决策"（cache friendly）
  - 决策结果写回 archetype 的 `ai_state` 列
- P4.5 plugin 接入：把"添加新怪物类型"做成 plugin demo（manifest 加一行就出现新怪物）

**不变量**：
- `MonsterEcosystem::current_individuals` API 不变
- 同样 tick 序列下，AI 决策结果一致（接 P4 确定性）

**KPI**：
- 1000 个怪物 + 100 tick benchmark：单 tick wall time < 20 ms
- 内存占用：1000 怪物 < 5 MB（旧实现 ~12 MB）
- P4.5 plugin 验证：写一个 "spawn_fire_elemental" plugin，启用后下次启动有火元素怪物

**风险**：
- Bevy 0.18 的 `Query` API 不直接支持自定义 SoA — 我们要在 `Resource` 层面做，绕过 `Query`
- 调试不便：chrome://inspect、bevy-inspector-egui 看不到 archetype 内部。预案：实现一个 `DebugArchetype` system 在 dev build 把数据 dump 到 JSON

---

### P6：Frame Graph + GPU-Driven 渲染（2 周，**延后到 Bevy 0.19**） ← **决策 2**

**目标**：把渲染管线从"CPU 端 despawn 全部再 spawn"改为"GPU 端 frustum culling + indirect drawing"。

**为什么延后**：
- Bevy 0.18 的 indirect draw + meshlet 路径还在演进，**API 可能在 0.19 变**
- 等 0.19 meshlet API 稳定后启动 P6，避免做两次
- P2 已经把 draw call 降到 8 级别，**P6 是锦上添花，不是 P3 阶段不 work 的救命稻草**

**代码改动（计划，等 0.19 稳定后细化）**：
- 新文件 `src/render/frame_graph.rs`
  - 声明 `RenderGraph` 节点：`OpaquePass`、`WaterPass`、`HudPass`、`ScreenshotPass`
  - 资源 aliasing：水 pass 复用 opaque pass 的 depth buffer
- 新文件 `src/render/gpu_cull.rs`
  - WGSL compute shader：`cull_chunks.comp` — 读 `ChunkBoundingSpheres` + 相机视锥 → 输出 `VisibleChunks`
  - `IndirectDraw` buffer 由 compute shader 直接写
- 改 `src/render/mod.rs`
  - 玩家移动只更新 `CameraPosition` uniform，**不重 mesh**
  - Mesh 重生成只在 chunk 实际被修改时（`Chunk.dirty` 标志）
- 引入 `bevy_pbr` 0.19 的 `MeshletMesh` — 配合 GPU 端 meshlet culling

**不变量**：
- 同样 chunk 修改序列下，渲染结果一致
- `iter_*.png` 视觉等效

**KPI**：
- 96³ 完整世界（不只是 ±16 半径）一次性渲染，FPS > 60
- 玩家移动时 0 mesh 重生成（除非挖方块）
- Draw call 总数 < 50（旧实现 3000）

**风险**：
- Bevy 0.19 实际发布时间不确定。预案：P2 完成后如果 0.19 在 3 个月内没出，先做 P5；出了直接进 P6
- WGSL 跨平台兼容（DX12 / Metal / Vulkan）。预案：CI 跑 `cargo build --target x86_64-pc-windows-msvc` + `--target x86_64-unknown-linux-gnu`（docker）

---

## 五、跨阶段共享工作

| 工作项 | 负责人 | 时间 | 阶段 |
| --- | --- | --- | --- |
| 单元测试基线 | developer | 0.5 天 | P1 起点 |
| 性能 benchmark suite（`cargo bench`） | developer | 1 天 | P1 起点 |
| `loop.ps1` 视觉回归脚本（PSNR） | iter-tester | 1 天 | P2 起点 |
| 多平台 CI（Win + Linux docker） | developer | 1 天 | P4 中期 |
| 阶段性 demo 视频（每阶段一段 30s 录屏） | iter-tester | 每阶段 0.5 天 | 全部 |

---

## 六、组织方式

每个阶段 = 1 个 sprint。建议节奏：

```
周一    ：developer 启动阶段，看上一阶段的 KPI / 红绿
周二-四 ：developer 改代码 + iter-tester 同步跑 loop.ps1
周五    ：iter-tester 出"本周 demo 视频 + 性能 report"
        code-reviewer 审 PR（架构改动 PR 必须 reviewer 通过才能合）
        阶段 KPI 全绿 → 进入下一阶段
```

**关键角色**（来自 `.harness/reins/`）：
- **developer**：写代码；PR owner；性能 benchmark 责任人
- **iter-tester**：跑 `loop.ps1`；视觉回归；性能 regression 报警
- **code-reviewer**：审架构 PR；特别关注 P3 的 data race、P6 的 GPU API 兼容

---

## 七、阶段 → KPI 总表

| 阶段 | 完成的硬指标 | 验证手段 |
| --- | --- | --- |
| P1 | `cargo test` 全过；`for_each_solid` < 70% 旧耗时 | `cargo bench` + 测试 |
| P2 | 移动时帧时间方差 < 20%；PSNR ≥ 35 dB | loop.ps1 + 视觉回归 |
| P3 | 单 tick wall < 15ms（4 核）；输出 byte-equal；玩家物理稳定 | bench + 状态 dump + 物理稳定性测试 |
| P4 | 同 seed state byte-equal；新 PcgRng 测试通过；VR 延迟基线测量 | 测试 + loop 跑两次 |
| **P4.5** | plugin 加载/卸载 demo 成功；拓扑排序测试通过 | plugin 测试 + 视觉对比 |
| P5 | 1000 怪物 / tick < 20ms；内存 < 5MB；plugin 接入验证 | bench |
| P6 | 全 96³ 渲染 60fps；draw call < 50 | loop + GPU 计时 |

---

## 八、风险与未决问题

| 风险 | 缓解 |
| --- | --- |
| Bevy 0.18 的 GPU-driven API 不稳定 | P6 已延后到 0.19；P6 失败时回退到 P2 + 多线程 mesh builder |
| 固定点数学改造范围扩大导致 P4 延期 | 严格 scope — P4 主任务只动 RNG；定点数学延后到 P4 末段 |
| `compt = ">=1.9, <1.10"` 在某次 cargo update 后被自动 bump | 加 PR check：检测 `Cargo.lock` 中 compt 是否仍在 1.9.x |
| iter-tester 在 12s 截图窗口抓不到 P6 的 GPU 状态 | P6 起把窗口扩到 30s，加 `gpu_time` 字段到 `state_t*.json` |
| 多人协作时 `loop.ps1` 改了一处把别人 P3 阶段搞坏 | 启用 `feature flag`：`cargo run --features p3-parallel` 隔离阶段 |
| avian3d 与 Bevy 0.18 不兼容 | P3 起点先做 1 天 spike 验证基础物理可跑 |
| VR 头显 SDK 接入时间（OpenXR）| P4 末段启动 VR spike，验证延迟基线，避免 P5 之后发现 VR 不 work |
| 插件系统被滥用（plugin panic 整个 app 挂）| P4.5 用 `catch_unwind` 隔离；测试包含 panic 恢复场景 |

---

## 九、不在本次计划范围（future work）

明确**不**做：
- ❌ **WASM 编译目标**（决策 4 砍掉）— 浏览器部署不是 demo 当前核心场景
- 真网络同步（Avian3D 物理接 multiplayer）
- 100 玩家大厅
- Aether 维度
- save/load 的版本兼容
- 移动端（Android / iOS）
- 脚本系统（Lua / WASM modding — 已被 P4.5 替代为 Rust 插件）

这些作为"v3.0 路线图"留待 v2 完成后再规划。

**未来条件变化时可重新评估**：
- WASM：如果未来有 web 部署需求，3 周可补一个 `wasm-pack` 适配层
- 移动端：bevy 0.19+ 对 mobile 端的支持稳定后再讨论

---

## 十、阶段签收（go/no-go 门槛）

每个阶段结束**必须**：

- [ ] 该阶段 KPI 全绿
- [ ] `cargo test --workspace` 全过
- [ ] `cargo clippy --workspace` 无新增 warning
- [ ] `loop.ps1` 跑通，生成 `iter_*.png` + `state_t*.json`
- [ ] code-reviewer 签字
- [ ] demo 视频上传到 `document/iterations/pN_*.mp4`
- [ ] 阶段总结追加到 `Agent.md` 的"当前状态"段

未签字不进下一阶段。

---

## 十一、决策日志（v0.2 起替换开放讨论）

> 本节是 v0.2 新增，把 v0.1 的"开放讨论"区替换为已签字的决策记录。新决策追加在顶部。

### 2026-06-06 v0.2 决策

| # | 决策 | 选择 | 理由 |
| --- | --- | --- | --- |
| 1 | P4.5 是否插入（modding / hot-plug 机制）| **加** | 1 周换未来 5 周 modding 基建；不影响 P1-P3 稳定性 |
| 2 | P6 渲染现代化时机 | **延后到 Bevy 0.19** | 0.18 indirect draw + meshlet API 不稳定，等 0.19 meshlet 稳定后启动 P6，避免做两次 |
| 3 | 物理（avian3d）何时接 | **P3 阶段** | 物理是 sim first-class 公民；P3 同时做并行 + 物理，节省 spike 成本 |
| 4 | VR / WASM 编译目标 | **VR 保留，WASM 砍** | VR 是 P4 确定性的强驱动；WASM 当前不是核心场景（future work） |

**v0.1 开放讨论已全部回答**。v0.3 之前的潜在决策点：
- 是否给 `SimClock` 加 deterministic wall clock 抽象（接 wall-time 但跨平台一致）
- P6 启动时是否同步启动 WebGPU 后端支持
- 玩家重生 / 死亡 / save 的实现时点（v2 阶段后还是 v3 阶段？）

---

*— Mavis, 2026-06-06, v0.2*
