# Doc-Todo 开发计划
## 文档改进 + 待实现功能路线图

> **版本**: v1.1
> **日期**: 2026-07-06
> **状态**: 文档清理已完成，功能开发待推进

---

## 一、文档管理改进

> ✅ **本节已于 2026-07-06 完成**

### 1.1 待清理文件

| 文件/目录 | 状态 | 优先级 | 备注 |
|----------|------|--------|------|
| `docs/architecture.md`（根目录） | ✅ 已删除 | 高 | 与 `docs/architecture/architecture.md` 重复 |
| `docs/architecture.html` | ✅ 已删除 | 高 | 过时的自动生成 HTML |
| `docs/architecture/hand-written.md` | ✅ 已删除 | 高 | 内容已整合到 game.md/server.md |
| `docs/design/goals.md` | ✅ 已删除 | 高 | 内容过于简略（仅6行） |
| `docs/design/voxel.md` | ✅ 已删除 | 高 | 内容过于简略（仅2行） |
| `docs/plans/short_term_plan_v3.md` | ✅ 已删除 | 高 | 空文件 |
| `docs/notes/STARTING.md` | ✅ 已删除 | 中 | 镜像配置已整合到 `docs/STARTING.md` |
| `docs/notes/drafts/config.md` | ✅ 已删除 | 中 | 空草稿文件 |
| `docs/design/gameplay.md` | ✅ 已删除 | 中 | 空文件（保留 gameplay-v1.md 和 kimi-gameplay.md） |
| `docs/design/gameplay-v1.md` | ✅ 保留 | 中 | 有独特内容（设计哲学） |
| `docs/design/kimi-gameplay.md` | ✅ 保留 | 中 | 有独特内容（ECS 实现方案） |
| `docs/notes/document-notes/config.md` | ✅ 已删除 | 低 | 已整合到 STARTING.md |
| `docs/notes/document-notes/笔记/config.md` | ✅ 已删除 | 低 | 已整合到 STARTING.md |
| `docs/notes/legacy-notes/` | ✅ 已删除 | 低 | 与 document-notes 重复 |
| `docs/archive/万国余烬_王冠赛季_legacy-imports/` | ✅ 已删除 | 低 | 与万国余烬_王冠赛季目录重复 |
| `docs/archive/legacy-imports/` | ✅ 已删除 | 低 | 重复的 Notion 导入包 |

### 1.2 待整合文档

| 目标 | 来源文档 | 状态 | 备注 |
|------|---------|------|------|
| 拆分 content.md 为 MVP 和远期目标 | `docs/design/content.md` | ✅ 已完成 | 添加了 Part A（MVP）和 Part B（远期目标） |
| 更新 overview.md 架构索引 | `docs/design/overview.md` | ✅ 已完成 | 指向所有现存文档，标注状态 |
| 归档 client-server-split.md | `docs/plans/client-server-split.md` | ✅ 已完成 | 移至 `docs/archive/` |
| 镜像配置整合到 STARTING.md | `docs/STARTING.md` | ✅ 已完成 | 添加了国内镜像加速章节 |

### 1.3 文档维护规则

- **新增功能必须更新对应文档**：代码变更后同步更新架构/设计文档
- **文档版本标记**：所有文档添加版本号和更新日期
- **过期标记**：不再维护的文档添加 `> **注意**: 本文档已过期，仅供历史参考`
- **交叉引用检查**：定期检查文档间的链接是否有效

---

## 二、功能聚焦：分阶段推进

### 阶段划分原则

| 阶段 | 目标 | 时间 |
|------|------|------|
| **Phase 1** | 核心体验（无限世界 + 地形预设） | 2-3 周 |
| **Phase 2** | 玩法深化（商队物流 + 外交战争） | 3-4 周 |
| **Phase 3** | 终局内容（以太界 + 神器） | 2-3 周 |

### 当前设计目标 vs 实际实现差距

| 设计目标 | 当前实现 | 差距 | 建议 |
|---------|---------|------|------|
| 元素系统（氢、氧、碳...） | 基础资源（gold, food, wood） | 很大 | Phase 3 再考虑 |
| 记忆约束系统 | TickObserver 不变量检测 | 较大 | Phase 2 扩展 |
| 涌现规则系统 | 简单的生态循环 | 较大 | Phase 2 逐步添加 |
| 热传导/流体模拟 | avian3d 基础物理 | 很大 | Phase 3 再考虑 |
| 基因系统 | 简单的怪物/动物生成 | 很大 | Phase 3 再考虑 |

---

## 三、测试覆盖提升

### 3.1 当前测试状态

| 层级 | 位置 | 状态 | 建议 |
|------|------|------|------|
| 单元规则 | `crates/core/src/**` | ✅ 有基础测试 | 扩展到所有模块 |
| crate 集成 | `crates/client` / `crates/server` | ⬜ 较少 | 添加编译接口测试 |
| 闭环测试 | `xtask loop` | ✅ 有健康检查 | 增加状态断言 |

### 3.2 必须补测试的规则

| 模块 | 测试项 | 优先级 |
|------|-------|--------|
| resource | 资源上限、守恒验证、掉落回流 | P0 |
| combat | 伤害结算、格挡招架、硬直击退 | P0 |
| protection | 保护期检查 attacker/target | P0 |
| match_state | 阶段边界、wall_secs 刷新 | P0 |
| monster/creature | 死亡掉落、计数回流 | P0 |
| nation | 创建解散、旗帜上限、人口上限 | P1 |
| protocol | CLI 参数解析、端口配置 | P1 |
| scenario | 推进完成/失败原因 | P1 |
| ai | 连续重复决策检测 | P1 |

### 3.3 测试审计目标

- [ ] `crates/core/src` 每个 `.rs` 文件至少有 1 个 `#[test]`
- [ ] 核心模块（resource, combat, nation）测试覆盖率 ≥ 60%
- [ ] 每次 PR 必须通过 `just test-changed`
- [ ] 玩法变更必须先写测试再实现

---

## 四、待实现功能开发计划

---

### 4.1 无限世界（当前有边界）

> **优先级**: 高
> **估计时间**: 2-3 小时

#### 现状
- `WORLD_SIZE = 128`（硬边界）
- `GameWorld` 持有固定大小 `Vec<u8>` 存所有 voxel
- 玩家 XZ 被 clamp 到 [0, WORLD_SIZE]

#### 目标
- 玩家在 XZ 方向可以走 ±∞，永远撞不到边界
- 新地形按需生成（玩家靠近时算）
- 渲染只算玩家视野内的方块

#### 推荐方案：方案 B（真无限稀疏存储 + AABB 渲染）

**数据结构变更**：
```rust
pub struct World {
    pub voxels: HashMap<IVec3, u8>,  // 稀疏存储：玩家改过的 + 显式 set 的
    pub vertical_min: i32,           // Y=0（基岩）
    pub vertical_max: i32,           // Y=96（天空）
    pub seed: u64,
}
```

**改动文件**：
| 文件 | 改动内容 |
|------|---------|
| `crates/core/src/world/mod.rs` | `voxels: Vec<u8>` → `HashMap<IVec3, u8>`；加 `generate_voxel` |
| `crates/client/src/render/greedy_mesh.rs` | 新增 `greedy_mesh_for_type_aabb` |
| `crates/client/src/render/mod.rs` | `spawn_terrain_around_player` 改用 AABB |
| `crates/core/src/constant/mod.rs` | 删除 `WORLD_SIZE` 或改为 `VERTICAL_MAX` |

#### 验收标准
- 玩家 WASD 走 1000 步不崩溃
- 截图能看到玩家走了很远（不在边界）
- `diff.json` 正常记录状态变化
- `cargo test` 通过

---

### 4.2 地形预设系统（可配置地形）

> **优先级**: 高
> **估计时间**: 2-2.5 小时

#### 现状
- 地形生成硬编码在 `WorldGenerator`
- 每次生成的地形固定

#### 目标
- 模块化 pipeline + preset 配置 + 随机选择
- 支持多种地形类型：默认、全平、高山、随机

#### 核心抽象

```rust
pub trait TerrainModule: Send + Sync {
    fn name(&self) -> &str;
    fn weight(&self) -> f32 { 1.0 }
    fn decide(&self, ctx: &mut TerrainContext) -> Option<BlockType>;
}

pub struct TerrainPipeline {
    pub name: String,
    pub modules: Vec<Box<dyn TerrainModule>>,
    pub vertical_min: i32,
    pub vertical_max: i32,
}
```

#### 具体模块

| 模块 | 功能 |
|------|------|
| `HeightmapModule` | 高度图生成（山脉、平原） |
| `CaveModule` | 洞穴生成 |
| `WaterFillModule` | 水面填充 |
| `OreModule` | 矿石生成 |
| `TreeModule` | 树木生成 |

#### Preset 配置

| Preset | 描述 |
|--------|------|
| `default` | 默认地形（和现在一样） |
| `flat_arena` | 全平地图（适合建造） |
| `mountainous` | 高山地图 |
| `random` | 斗魂大乱斗（每局不同） |

#### CLI 集成

```powershell
cargo run -p lk2-client -- --offline --preset=flat_arena
cargo run -p lk2-client -- --offline --preset=mountainous
cargo run -p lk2-client -- --offline --preset=random
```

#### 改动文件
| 文件 | 改动内容 |
|------|---------|
| `crates/core/src/world/terrain/mod.rs` | 定义 `TerrainModule` trait + `TerrainContext` |
| `crates/core/src/world/terrain/presets.rs` | 实现 preset 配置 |
| `crates/core/src/world/mod.rs` | 地形生成改为 pipeline |
| `crates/client/src/main.rs` | 添加 `--preset` CLI 参数 |

---

### 4.3 商队和物流系统

> **优先级**: 高
> **估计时间**: 3-4 小时

#### 现状
- 无商队系统
- 资源直接进入全局资源池

#### 目标
- 物理化商队：有位置、可移动、可被攻击
- 护送系统：玩家在附近时商队有减伤
- 掉落系统：商队被击杀掉落货物

#### 核心实体

| 实体 | 组件 | 说明 |
|------|------|------|
| `Caravan` | `OwnerNationId`, `TraderPlayerId`, `PackAnimalEntity`, `CargoInventory`, `RoutePath` | 商队实体 |
| `PackAnimal` | `Position`, `Health`, `Inventory`, `MovementSpeed`, `IsBeingLed` | 驮兽 |

#### 核心系统

| 系统 | 功能 |
|------|------|
| `caravan_movement_system` | 商队跟随牵引者移动 |
| `escort_aura_system` | 10 格范围内有护送者时减伤 20% |
| `caravan_death_system` | 商队被击杀掉落货物 |
| `caravan_spawn_system` | 定期生成商队 |

#### 资源流转

```
生产点（矿洞/农场）
    │
    ▼ 采集
临时存储（背包/箱子）
    │
    ▼ 商队运输（物理移动，可被劫）
国家仓库
    │
    ▼ 加工/制造
成品
    │
    ▼ 分配
前线 / 市场 / 建筑
```

#### 改动文件
| 文件 | 改动内容 |
|------|---------|
| `crates/core/src/economy/caravan.rs` | 商队组件和系统 |
| `crates/core/src/economy/mod.rs` | 注册商队系统 |
| `crates/core/src/resources/global_pool.rs` | 添加商队相关资源 |
| `crates/core/src/protocol/mod.rs` | 添加商队同步消息 |

---

### 4.4 终局内容（以太界、神器）

> **优先级**: 中
> **估计时间**: 3-4 小时

#### 现状
- 无终局内容
- 游戏没有明确的胜利条件

#### 目标
- 位面传送门建造系统
- 以太界（特殊维度，重力扭曲）
- 以太幽魂（高难度怪物）
- 神器系统（终极装备）

#### 位面传送门

**建造条件**：
- 国家等级 ≥ 王国
- 消耗：50 阳炎石 + 50 霜心晶体 + 50 活根
- 可被攻击/摧毁

#### 以太界特性

| 特性 | 描述 |
|------|------|
| 重力扭曲 | 60-120 秒周期性切换（0.5x ~ 1.5x） |
| 以太幽魂 | 高机动飞行怪物，击杀掉落虚空精华 |
| 虚空精华 | 终极资源，用于合成神器 |

#### 神器系统

| 神器 | 效果 | 合成材料 |
|------|------|---------|
| 时间沙漏 | 时间流速 ±50% | 虚空精华 ×5 + 阳炎石 ×10 |
| 重力宝珠 | 局部重力操控 | 虚空精华 ×5 + 霜心晶体 ×10 |
| 世界之心 | 自定义物理规则 | 虚空精华 ×10 + 活根 ×10 |

#### 胜利条件

| 类型 | 条件 | 奖励 |
|------|------|------|
| 征服胜利 | 摧毁所有敌对国家旗帜 | 王冠世界持有权 |
| 科技胜利 | 解锁终极科技"世界之心" | 自定义物理规则 |
| 生态胜利 | 建立可持续生态循环（100年） | 永恒春天 |

#### 改动文件
| 文件 | 改动内容 |
|------|---------|
| `crates/core/src/endgame/portal.rs` | 传送门建造系统 |
| `crates/core/src/endgame/aether.rs` | 以太界系统 |
| `crates/core/src/endgame/artifacts.rs` | 神器系统 |
| `crates/core/src/endgame/victory.rs` | 胜利条件判定 |
| `crates/core/src/monster/aether_wraith.rs` | 以太幽魂 |

---

### 4.5 完整外交和战争系统

> **优先级**: 中
> **估计时间**: 3-4 小时

#### 现状
- 简单的国家创建
- 基础战斗系统
- 无外交机制

#### 目标
- 完整的外交关系系统（战争/敌对/中立/贸易/同盟/附庸）
- 战争疲劳机制
- 条约系统（宣战、求和、结盟）
- 国家接管系统

#### 外交关系

```rust
enum DiplomaticRelation {
    War,           // 战争：可互相攻击
    Hostile,       // 敌对：无条约，紧张
    Neutral,       // 中立：默认状态
    Trade,         // 贸易：可互通市场
    Alliance,      // 同盟：共享视野、协防
    Vassal,        // 附庸：进贡、受保护
}
```

#### 战争疲劳机制

| 疲劳值 | 效果 |
|--------|------|
| 0-60 | 正常 |
| 61-80 | 治疗效率 -50% |
| 81-100 | 无法签署条约 |
| ≥100 | 旗帜易受攻击（+25% 伤害） |

#### 条约系统

| 条约类型 | 效果 | 持续时间 |
|---------|------|---------|
| 和平条约 | 停止战争，进入中立 | 30 分钟 |
| 贸易条约 | 可互通市场 | 60 分钟 |
| 同盟条约 | 共享视野、协防 | 120 分钟 |
| 附庸条约 | 附庸国进贡，受保护 | 永久（可解除） |

#### 国家接管系统

- 国家被摧毁后，旗帜位置产生"接管窗口"（10 分钟）
- 其他国家可占领接管窗口，获得原国家的部分资源
- 接管窗口过期后，遗址变为废墟

#### 改动文件
| 文件 | 改动内容 |
|------|---------|
| `crates/core/src/nation/diplomacy.rs` | 外交关系系统 |
| `crates/core/src/nation/treaty.rs` | 条约系统 |
| `crates/core/src/nation/war_exhaustion.rs` | 战争疲劳系统 |
| `crates/core/src/nation/takeover.rs` | 国家接管系统 |

---

## 五、开发优先级排序

### Phase 1（核心体验）

1. **无限世界** — 解决玩家撞边界的核心痛点
2. **地形预设系统** — 增加游戏多样性
3. **文档清理** — 删除过时文档

### Phase 2（玩法深化）

4. **商队和物流系统** — 增加经济深度
5. **外交和战争系统** — 增加策略深度
6. **测试覆盖提升** — 确保质量

### Phase 3（终局内容）

7. **以太界** — 终局挑战
8. **神器系统** — 终极目标
9. **设计文档整合** — 分阶段更新 content.md

---

## 六、验收标准模板

每个功能完成时必须满足：

- [ ] `just test-changed` 通过
- [ ] `just loop` 能生成有效产物
- [ ] `health.json` 为 `PASS`
- [ ] 核心逻辑有单元测试
- [ ] 相关文档已更新
- [ ] 无编译警告（`cargo clippy --workspace` 通过）
- [ ] 代码格式化（`cargo fmt` 通过）