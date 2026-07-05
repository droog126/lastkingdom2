# 万国起源：最后一国 钻石版 - 架构文档

> 当前说明：这是更新后的架构文档，反映 Bevy 0.19 + 3-crate workspace 结构。工程边界以 `docs/architecture/engineering-baseline.md` 为准；运行/闭环入口以 `docs/STARTING.md`、`AGENTS.md` 和 `.codex/skills/*/SKILL.md` 为准。

## 一、项目概述

这是一个基于 **Bevy 0.19** 的体素游戏 Demo，核心特色是 **AI 闭环迭代**：游戏自动运行 → 截图 → AI 读取结果 → 决定修改 → 重建运行，形成无人干预的迭代循环。

### 项目定位
- **技术栈**：Rust + Bevy 0.19 + ECS 架构 + Lightyear 网络同步
- **世界规模**：96³ 体素（可配置预设）
- **核心玩法**：采集、造国、杀怪、资源管理、PvP 战斗
- **架构模式**：Client/Server 分离 + 共享核心逻辑

---

## 二、整体架构

### 2.1 3-Crate Workspace 结构

```
F:\rustProject\lastkingdom2\
├── Cargo.toml                  ← workspace 根
├── crates/
│   ├── core/                   ← lk2-core   (lib) — 共享游戏状态、规则、协议
│   │   ├── src/
│   │   │   ├── lib.rs          ← re-export 所有模块
│   │   │   ├── world/          ← 体素世界生成与管理
│   │   │   ├── resource/       ← 全局资源池 + 转账系统
│   │   │   ├── nation/         ← 国家建立与管理
│   │   │   ├── monster/        ← 怪物生态系统
│   │   │   ├── creature/       ← 动物系统
│   │   │   ├── ai/             ← TickObserver 不变量检测
│   │   │   ├── scenario/       ← 场景脚本状态机
│   │   │   ├── combat/         ← V2 战斗系统（HP/STA/Block/Parry/Stun/Knockback）
│   │   │   ├── pvp/            ← PvP 组件与系统
│   │   │   ├── protocol/       ← lightyear 协议定义（消息 + 组件）
│   │   │   ├── player/         ← 玩家状态与逻辑
│   │   │   ├── constant/       ← 常量定义
│   │   │   ├── clock/          ← 模拟时钟
│   │   │   ├── sim/            ← 模拟步进逻辑
│   │   │   ├── v2/             ← V2 框架（状态、系统集）
│   │   │   └── ...
│   ├── server/                 ← lk2-server (bin) — 无头权威服务器
│   │   ├── src/main.rs         ← MinimalPlugins + lightyear ServerPlugins
│   │   └── src/pvp_systems.rs  ← 服务端 PvP 系统
│   └── client/                 ← lk2-client (bin) — 渲染客户端
│       ├── src/main.rs         ← DefaultPlugins + lightyear ClientPlugins
│       ├── src/render/         ← 体素渲染、相机、输入
│       ├── src/pretty/         ← 装饰物（水、树、云、旗帜、角色）
│       └── src/ui.rs           ← HUD 系统
└── xtask/                      ← 自动化任务（闭环迭代、测试、审计）
```

### 2.2 架构层次

```
┌──────────────────────────────────────────────────────────────┐
│                    客户端表现层 (Client)                      │
│  lk2-client/                                                 │
│    ├── render/     → 体素渲染、相机、玩家输入、HUD            │
│    ├── pretty/     → 装饰物（水、树、云、旗帜、角色）        │
│    └── ui.rs       → HUD 界面、状态显示                      │
├──────────────────────────────────────────────────────────────┤
│                    共享核心层 (Core)                          │
│  lk2-core/                                                   │
│    ├── world/      → 体素世界生成与管理                      │
│    ├── resource/   → 全局资源池 + 转账系统                   │
│    ├── nation/     → 国家建立与管理                          │
│    ├── monster/    → 怪物生态系统                            │
│    ├── creature/   → 动物系统                                │
│    ├── combat/     → V2 战斗系统                            │
│    ├── pvp/        → PvP 组件                               │
│    ├── protocol/   → lightyear 协议定义                      │
│    ├── scenario/   → 场景脚本状态机                          │
│    ├── ai/         → TickObserver 不变量检测                 │
│    └── sim/        → 模拟步进逻辑                           │
├──────────────────────────────────────────────────────────────┤
│                    服务端权威层 (Server)                      │
│  lk2-server/                                                 │
│    ├── main.rs     → 无头服务器入口                          │
│    └── pvp_systems.rs → 服务端 PvP 权威逻辑                 │
└──────────────────────────────────────────────────────────────┘
```

### 2.3 模块依赖关系

```
lk2-core (lib)
    ├── world/        → 无外部依赖
    ├── resource/     → 依赖 constant/
    ├── nation/       → 依赖 resource/, world/
    ├── monster/      → 依赖 resource/
    ├── creature/     → 依赖 world/, resource/
    ├── ai/           → 依赖 world/, resource/, nation/, monster/
    ├── scenario/     → 依赖所有模块
    ├── combat/       → 依赖 pvp/, player/
    ├── pvp/          → 依赖 protocol/
    └── protocol/     → 依赖 world/, constant/

lk2-server (bin) → 依赖 lk2-core
lk2-client (bin) → 依赖 lk2-core
```

---

## 三、核心模块详解

### 3.1 World 模块 (`crates/core/src/world/mod.rs`)

**职责**：管理 3D 体素世界的生成、存储和查询

**核心数据结构**：
- `World`：96³ 体素世界，支持多种地形预设
- `BlockType`：13 种方块类型（Air/Dirt/Stone/Water/Wood/Ore 等）
- `TerrainPipeline`：可配置的地形生成管线（支持多种预设）

**关键功能**：
- `World::with_pipeline()`：使用指定预设生成世界
- `get(x, y, z)` / `set(x, y, z, block)`：方块访问
- `player_spawn_position_at()`：计算玩家出生位置

**地形预设**：支持多种预设切换（default、spawn_hill、canyon 等）

---

### 3.2 Resource 模块 (`crates/core/src/resource/mod.rs`)

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

### 3.3 AI 模块 (`crates/core/src/ai/mod.rs`)

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

### 3.4 Combat 模块 (`crates/core/src/combat.rs`) — V2 战斗系统

**职责**：PvP 战斗核心逻辑

**战斗组件**：
- `CombatHealth`：生命值
- `CombatStamina`：耐力值
- `CombatBlockState`：格挡状态
- `CombatParryWindow`：招架窗口
- `CombatStunState`：眩晕状态
- `CombatKnockback`：击退效果
- `CombatAttackState`：攻击状态
- `CombatDowned`：倒地状态
- `CombatInputBuffer`：输入缓冲区

---

### 3.5 Protocol 模块 (`crates/core/src/protocol.rs`)

**职责**：lightyear 网络协议定义

**消息类型**：
- `AttackInput`：攻击输入（Client → Server）
- `HitConfirm`：命中确认（Server → Client）
- `DamageResult`：伤害结果（Server → Client）
- `KnockbackEvent`：击退事件（Server → Client）
- `GameplayCommand`：游戏命令（移动、跳跃、采集等）

**复制组件**：
- `PlayerPos`：玩家位置
- `PlayerRot`：玩家旋转
- `Health`：玩家血量
- `VoxelDelta`：体素变更
- `GameplayHudState`：HUD 状态

---

### 3.6 Render 模块 (`crates/client/src/render/mod.rs`)

**职责**：体素渲染、相机控制、玩家输入

**渲染策略**：
- 玩家周围动态加载地形
- 支持平滑地形（marching cubes）和传统体素（greedy mesh）
- Bevy 0.19 延迟渲染管线 + 体积雾 + SSR + TAA

**相机模式**：
- 第一人称视角（鼠标控制）
- 第三人称环绕视角
- 自由飞行模式
- 自动跟动物模式（auto-demo）

**输入系统**：
| 按键 | 功能 |
|------|------|
| WASD/方向键 | 移动 |
| Space | 跳跃 |
| Shift | 下降/冲刺 |
| G | 采集 |
| F | 造国 |
| J/K | 杀怪/杀动物 |
| Q/E | 转向 |
| Escape | 退出 |

---

### 3.7 Monster 模块 (`crates/core/src/monster/mod.rs`)

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

### 3.8 Nation 模块 (`crates/core/src/nation/mod.rs`)

**职责**：国家建立与管理

**核心功能**：
- `found()`：消耗 Soul 建立国家
- `next_flag_cost()`：递增成本（每面旗 +20 Soul）
- 国旗上限：8 面

---

## 四、Bevy ECS 系统架构

### 4.1 资源注册

```rust
App::new()
    .init_resource::<GameWorld>()
    .init_resource::<GlobalResourcePool>()
    .init_resource::<NationRegistry>()
    .init_resource::<MonsterEcosystem>()
    .init_resource::<TickObserver>()
    .init_resource::<SimClock>()
    .init_resource::<EcoCycle>()
    // ...
```

### 4.2 服务端系统链（lk2-server）

服务端使用 `MinimalPlugins` + `lightyear::ServerPlugins`，所有逻辑跑在 `FixedUpdate`：

```rust
app.add_systems(FixedUpdate, (
    advance_fixed_authority_tick,
    // ... 服务端权威逻辑
));
```

### 4.3 客户端系统链（lk2-client）

客户端使用 `DefaultPlugins` + `lightyear::ClientPlugins`：

```rust
app.add_systems(Startup, (
    setup_fonts, setup_camera, setup_light, setup_atmosphere,
    setup_world, spawn_creatures, setup_hud, setup_player_pvp,
).chain());

app.add_systems(Update, (
    scenario_runner, auto_demo, player_input, first_person_camera,
    simulation_tick, end_tick_system, update_hud, periodic_screenshot,
    day_night_cycle,
).chain());
```

---

## 五、AI 闭环迭代流程

### 5.1 迭代周期

```
┌──────────────────────────────────────────────────────────────┐
│  Phase 1: CAPTURE                                           │
│    cargo xtask loop --offline --seconds 60                   │
│    → screenshots/iter_NN/iter_NN.png                         │
│    → screenshots/iter_NN/final_state.json                    │
│    → screenshots/iter_NN/health.json                         │
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
│    Edit 改代码 → cargo build → just loop → 回到 Phase 1     │
└──────────────────────────────────────────────────────────────┘
```

### 5.2 状态输出格式

```json
{
  "tick": 100,
  "wall_secs": 10.5,
  "player": {
    "block_pos": [48, 15, 48],
    "pos": [48.5, 15.5, 48.5],
    "monsters_killed": 3,
    "blocks_gathered": 15,
    "inventory": { "wood": 42, "food": 28, "apple": 15, "soul": 80 }
  },
  "pool": { "wood": 100, "food": 80, "apple": 50, "soul": 200 },
  "nations": { "flag_count": 2, "total_nations": 1 },
  "monsters": { "current": 12, "kingdoms": 2, "nests": 5 },
  "observer": { "snapshots": 20, "decisions": 156, "anomalies": 0, "invariant_violations": 0 }
}
```

---

## 六、技术栈

| 依赖 | 版本 | 用途 |
|------|------|------|
| bevy | 0.19 | 游戏引擎（ECS + 渲染 + 输入） |
| avian3d | 0.7.0 | 物理引擎 |
| lightyear | 0.28.0 | 网络同步（消息 + 复制） |
| lightyear_avian3d | 0.28.0 | lightyear 物理同步 |
| lightyear_inputs_leafwing | 0.28.0 | 输入同步 |
| leafwing-input-manager | 0.21.0 | 输入管理 |
| rand | 0.10.1 | 随机数生成 |
| bevy-inspector-egui | 0.37.0 | 调试工具 |
| bevy-tnua | 0.32.0 | 角色控制器 |
| bevy-tnua-avian3d | 0.12.0 | TNUA 物理集成 |
| serde + serde_json | 1.x | 状态序列化 |
| block-mesh | 0.2.0 | 体素网格生成 |
| ndshape | 0.3.0 | 多维数组形状 |
| bevy_panorbit_camera | 0.35.0 | 环绕相机 |

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

### 7.5 Client/Server 模式
- 服务端权威（lk2-server）+ 客户端预测（lk2-client）
- lightyear 处理网络同步和插值

---

## 八、性能优化策略

1. **视锥剔除**：只渲染玩家周围范围内的方块
2. **方块数量限制**：最多 3000 个方块，防止卡顿
3. **材质共享**：同类型方块共享 Mesh 和 Material，减少 GPU 状态切换
4. **距离排序**：Painter's algorithm 正确处理半透明
5. **Tick 限流**：每 1 秒才输出一次"体素过多"警告
6. **延迟渲染管线**：Bevy 0.19 deferred renderer + 体积雾 + SSR

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

### P0（已完成）
- [x] 天空颜色（Bevy 0.19 Atmosphere）
- [x] 体素地形（96³）
- [x] 玩家可见（Avatar + 武器）
- [x] HUD 显示
- [x] 自动截图（xtask loop）
- [x] Client/Server 分离
- [x] Lightyear 网络同步
- [x] V2 战斗系统

### P1（进行中）
- [ ] 出生地平坦区域优化
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
cargo build --workspace

# 运行客户端（离线模式）
cargo run -p lk2-client -- --offline

# 运行客户端（在线模式，先启服务端）
cargo run -p lk2-client -- --connect=127.0.0.1:5000

# 运行服务端
cargo run -p lk2-server

# 闭环迭代
just loop

# 测试
just test-changed

# 代码检查
cargo clippy --workspace
```