# 万国起源：最后一国 钻石版 - 架构文档

## 一、项目概述

这是一个基于 **Bevy 0.18.1** 的体素游戏 Demo，核心特色是 **AI 闭环迭代**：游戏自动运行 → 截图 → AI 读取结果 → 决定修改 → 重建运行，形成无人干预的迭代循环。

### 项目定位
- **技术栈**：Rust + Bevy 0.18.1 + ECS 架构
- **世界规模**：32³ 体素（简化版 Demo）
- **核心玩法**：采集、造国、杀怪、资源管理

---

## 二、整体架构

### 2.1 架构层次

```
┌──────────────────────────────────────────────────────────────┐
│                    表现层 (Presentation)                      │
│  render/     → 体素渲染、相机、玩家输入、HUD                  │
│  pretty/     → 装饰物（水、树、云、旗帜、角色）              │
├──────────────────────────────────────────────────────────────┤
│                    业务层 (Domain)                            │
│  creature/   → 动物系统（猪、羊、牛、鸡）                    │
│  monster/    → 怪物生态系统                                  │
│  nation/     → 国家建立与管理                                │
│  scenario/   → 场景脚本状态机                                │
├──────────────────────────────────────────────────────────────┤
│                    数据层 (Data)                             │
│  world/      → 体素世界生成与管理                            │
│  resource/   → 全局资源池 + 转账系统                         │
│  constant/   → 常量定义                                     │
├──────────────────────────────────────────────────────────────┤
│                    基础设施 (Infrastructure)                 │
│  ai/         → TickObserver 不变量检测 + 异常检测           │
│  utils/      → 工具函数（通道、文件、随机数）                 │
└──────────────────────────────────────────────────────────────┘
```

### 2.2 模块依赖关系

```
main.rs (入口)
    ├── world/        → 无外部依赖
    ├── resource/     → 依赖 constant/
    ├── nation/       → 依赖 resource/, world/
    ├── monster/      → 依赖 resource/
    ├── creature/     → 依赖 world/, resource/
    ├── ai/           → 依赖 world/, resource/, nation/, monster/
    ├── render/       → 依赖 world/, creature/, nation/, monster/
    ├── pretty/       → 依赖 render/
    └── scenario/     → 依赖所有模块
```

---

## 三、核心模块详解

### 3.1 World 模块 (`src/world/mod.rs`)

**职责**：管理 3D 体素世界的生成、存储和查询

**核心数据结构**：
- `World`：32³ 稠密数组，`blocks: Vec<BlockType>`
- `BlockType`：12 种方块类型（Air/Dirt/Stone/Water/Wood/Ore 等）
- `Biome`：3 种生物群落（Desert/Tundra/Jungle）

**关键功能**：
- `WorldGenerator::generate()`：确定性世界生成
- `gather_block()`：挖掘方块转换为资源
- `visible_blocks()`：获取玩家视野范围内的方块

**世界生成流程**：
1. 地形高度图（双八度 noise）
2. 洞穴生成（3D noise 挖空）
3. 海平面以下填水
4. 矿石聚类撒布（cluster 间距约束）
5. 树木/仙人掌/浆果/巨砾生成

---

### 3.2 Resource 模块 (`src/resource/mod.rs`)

**职责**：全局资源池管理与转账系统

**资源类型**：
- 基础资源：Wood, Food, Apple, Soul
- 生物群落专属：Sunstone（沙漠）, Frostcore（苔原）, LivingRoot（丛林）

**转账系统**：
- `Transfer`：包含 kind、amount、src、dst
- `TransferSrc`：PlayerGather, ResourceRegen
- `TransferDst`：PlayerUse, NationFound, MonsterConsume

**审计机制**：
- `audit_added` / `audit_subtracted`：记录所有进出
- `verify_conservation()`：验证资源守恒

---

### 3.3 AI 模块 (`src/ai/mod.rs`) — 核心亮点

**职责**：Tick-level 闭环 Debug 系统

**设计目标**：
1. 每 tick 快照（TickSnapshot）
2. AI 决策日志（AiDecision）
3. 不变量断言（Invariant）
4. 异常检测（Anomaly）
5. 状态重放（Replay）

**不变量检查**：
| 检查项 | 描述 |
|--------|------|
| ResourceConservation | 资源总量 ≤ 最大值，审计平衡 |
| MonsterCountConsistency | 当前个体数 = 所有 nests 之和 |
| FlagCountCap | 国旗数 ≤ 8 |
| PlayerInBounds | 玩家位置在世界范围内 |
| TickDurationBounded | Tick 时长 < 50ms |

**异常检测**：
| 异常类型 | 描述 |
|----------|------|
| Oscillation | 同一 AI 连续 5 tick 做相同决策 |
| TickSpike | Tick 耗时突然飙升 |
| ResourceJump | 资源凭空出现/消失 |
| StructuralChange | 怪物王国/nest 数量异常变化 |
| MassDissolution | 所有国家被瞬间拆除 |

---

### 3.4 Render 模块 (`src/render/mod.rs`)

**职责**：体素渲染、相机控制、玩家输入

**渲染策略**：
- 玩家周围 16 格半径内的方块
- 最多渲染 3000 个方块（防止卡顿）
- 共享材质减少 GPU 状态切换
- Painter's algorithm 按距离排序

**相机模式**：
- 第一人称视角（鼠标控制）
- 自动跟动物模式（auto-demo）

**输入系统**：
| 按键 | 功能 |
|------|------|
| WASD/方向键 | 移动 |
| Space | 跳跃 |
| Shift | 下降 |
| G | 采集 |
| K | 杀动物 |
| F | 造国 |
| J | 杀怪 |
| Q/E | 转向 |

---

### 3.5 Monster 模块 (`src/monster/mod.rs`)

**职责**：怪物生态系统管理

**三层结构**：
```
Kingdom（王国）
    └── Nest（巢穴）
            └── Individual（个体）
```

**生命周期**：
- 觅食 → 移动寻找资源
- 休眠 → 资源不足时进入休眠
- 衰亡 → 长期休眠后死亡
- 被击杀 → 玩家攻击

---

### 3.6 Nation 模块 (`src/nation/mod.rs`)

**职责**：国家建立与管理

**核心功能**：
- `found()`：消耗 Soul 建立国家
- `next_flag_cost()`：递增成本（每面旗 +20 Soul）
- 国旗上限：8 面

---

## 四、Bevy ECS 系统架构

### 4.1 资源注册（main.rs）

```rust
App::new()
    .init_resource::<GameWorld>()
    .init_resource::<GlobalResourcePool>()
    .init_resource::<NationRegistry>()
    .init_resource::<MonsterEcosystem>()
    .init_resource::<TickObserver>()
    .init_resource::<RenderConfig>()
    // ...
```

### 4.2 Startup 系统链

```rust
(
    setup_camera,      // 相机初始化
    setup_light,       // 光照（太阳 + 补光）
    setup_atmosphere,  // 天空 + 雾 + 武器
    setup_cursor_grab, // 光标锁定
    setup_world,       // 世界生成
    spawn_pretty,      // 装饰物生成
    spawn_creatures,   // 动物生成
    setup_hud,         // HUD 初始化
    self_check,        // 启动自检（100 tick）
).chain()
```

### 4.3 Update 系统链

```rust
(
    scenario_runner,           // 场景脚本执行
    auto_demo,                 // 自动演示模式
    mouse_look_system,         // 鼠标视角累积
    first_person_camera,       // 相机更新
    player_input,              // 玩家输入处理
    player_attack_creatures,   // 攻击动物
    animate_avatar,            // 角色动画
    spawn_terrain_around_player, // 动态加载地形
    simulation_tick,           // 游戏逻辑 tick
    end_tick_system,           // 不变量检查
    update_hud,                // HUD 更新
    periodic_screenshot,       // 每 5 秒截图
    day_night_cycle,           // 昼夜循环
    exit_on_esc,               // ESC 退出
).chain()
```

---

## 五、AI 闭环迭代流程

### 5.1 迭代周期

```
┌──────────────────────────────────────────────────────────────┐
│  Phase 1: CAPTURE                                           │
│    cargo run -- --auto-demo (12秒)                           │
│    → screenshots/iter_NN.png (每5秒自动截图)                 │
│    → screenshots/state_NN.json (每5 tick 状态dump)           │
├──────────────────────────────────────────────────────────────┤
│  Phase 2: OBSERVE                                           │
│    AI 读取截图 + JSON 状态                                    │
│    识别问题（视觉/逻辑/Bug）                                  │
├──────────────────────────────────────────────────────────────┤
│  Phase 3: DECIDE                                            │
│    优先级：Bug > 视觉缺失 > 性能 > 装饰                        │
│    一次改 1-3 个相关改动                                      │
├──────────────────────────────────────────────────────────────┤
│  Phase 4: ACT                                               │
│    Edit 改代码 → cargo build → run_loop.ps1 → 回到 Phase 1   │
└──────────────────────────────────────────────────────────────┘
```

### 5.2 状态输出格式

```json
{
  "tick": 100,
  "wall_secs": 10.5,
  "player": {
    "block_pos": [16, 5, 16],
    "pos": [16.5, 5.5, 16.5],
    "monsters_killed": 3,
    "blocks_gathered": 15
  },
  "pool": {
    "wood": 42,
    "food": 28,
    "apple": 15,
    "soul": 80
  },
  "nations": {
    "flag_count": 2,
    "total_nations": 1
  },
  "monsters": {
    "current": 12,
    "kingdoms": 2,
    "nests": 5
  },
  "observer": {
    "snapshots": 20,
    "decisions": 156,
    "anomalies": 0,
    "invariant_violations": 0
  }
}
```

---

## 六、技术栈

| 依赖 | 版本 | 用途 |
|------|------|------|
| bevy | 0.18.1 | 游戏引擎（ECS + 渲染 + 输入） |
| avian3d | 0.5 | 物理引擎 |
| broccoli | 0.6 | 碰撞检测 |
| rand | 0.8.5 | 随机数生成 |
| sepax2d | 0.3 | 2D 碰撞检测 |
| bevy-inspector-egui | 0.36 | 调试工具 |
| serde + serde_json | 1.x | 状态序列化 |
| crossbeam-channel | 0.5 | 并发通道 |

> **注意**：broccoli 0.6 不兼容 compt 1.10，需锁定 `compt >=1.9, <1.10`

---

## 七、关键设计模式

### 7.1 ECS 模式
- 使用 Bevy 的 ECS 架构，数据驱动
- 系统按功能划分，通过资源和组件通信

### 7.2 观察者模式
- `TickObserver` 订阅所有 tick 事件
- 记录状态快照、决策日志、异常检测

### 7.3 状态机模式
- `Scenario` 场景脚本使用状态机执行
- 支持 WaitTicks、Log、MoveTo 等步骤

### 7.4 单例模式
- 全局资源（GameWorld、GlobalResourcePool 等）作为 Bevy Resource

---

## 八、性能优化策略

1. **视锥剔除**：只渲染玩家周围 16 格范围内的方块
2. **方块数量限制**：最多 3000 个方块，防止卡顿
3. **材质共享**：同类型方块共享 Mesh 和 Material，减少 GPU 状态切换
4. **距离排序**：Painter's algorithm 正确处理半透明
5. **Tick 限流**：每 1 秒才输出一次"体素过多"警告

---

## 九、安全与稳定性

### 9.1 不变量保护
- 每 tick 自动检查资源守恒
- 玩家位置边界检查
- Tick 时长限制（防止死循环）

### 9.2 启动自检
- 启动时自动运行 100 tick 测试
- 验证所有核心系统正常工作

### 9.3 错误处理
- 资源操作返回 Result，强制处理错误
- 详细的错误日志和异常报告

---

## 十、扩展方向

### P0（必须）
- [x] 天空颜色
- [x] 体素地形
- [x] 玩家可见
- [x] HUD 显示
- [x] 自动截图

### P1（强烈建议）
- [ ] 出生地平坦区域
- [ ] 装饰物围绕出生地
- [ ] 相机不卡地下

### P2（加分项）
- [ ] 阴影
- [ ] 远景雾
- [ ] 战争迷雾
- [ ] 怪物 AI 移动
- [ ] 方块挖掉消失

### P3（长期目标）
- [ ] 存档/读档
- [ ] 多人大厅
- [ ] Aether 维度

---

## 附录：运行命令

```powershell
# 编译
$env:BEVY_DISABLE_ACCESSIBILITY="1"
cargo build

# 运行（自动演示模式）
cargo run -- --auto-demo

# 闭环迭代
.\loop.ps1

# 测试
cargo test --workspace

# 代码检查
cargo clippy --workspace
```
