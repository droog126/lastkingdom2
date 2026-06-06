# 万国起源：最后一国 钻石版 — Demo Skeleton

bevy 0.18.1 上的 **sim + 渲染 demo 骨架**。

> 状态：✅ `cargo check` / `cargo build` / `cargo test` 全部通过；✅ `cargo run` 自动跑 sim + 截图 + HUD，**支持闭环 AI 迭代**。

---

## 零、闭环 AI 迭代

> 这是核心设计：游戏自己跑、自己截屏、截屏可以读出来评估、再改代码再跑。

```
   ┌──────────┐     每 5 秒      ┌──────────┐
   │ 运行 .exe │───────────────▶│iter_N.png│
   └──────────┘                  └──────────┘
        ▲                              │
        │                              ▼
   修代码 (main.rs / render.rs)   AI 读图 (Read tool)
        │                              │
        └────── 下一次迭代 ◀───────────┘
```

每一张 iter_N.png 都是带 HUD 的全屏截屏（1280×720）。HUD 直接显示 tick 数、玩家坐标、4 个关键资源、怪物数、invariant 状态。

---

## 一、目录

```
F:\rustProject\lastkingdom2\
├── Cargo.toml                  # bevy 0.18.1, broccoli 0.6.6, compt 1.9.x, serde
├── src/
│   ├── main.rs                 # 入口 + Bevy 系统接线 + HUD + 截图 + 启动自检
│   ├── constant/mod.rs         # 25 资源上限 / 国旗上限 / 怪物上限 / tick 速率
│   ├── resource/mod.rs         # GlobalResourcePool + Transfer + 守恒审计
│   ├── world/mod.rs            # 32³ 世界 + 3 Biome + 8 BlockType + 生成器
│   ├── nation/mod.rs           # 8 国旗上限 + pop 5/10/15/20
│   ├── monster/mod.rs          # 5 王国 + 80 巢 + 1500 个体生态
│   ├── render/mod.rs           # 体素地形渲染（玩家周围 16 格内）
│   ├── pretty/mod.rs           # 水面 + 玩家 avatar + 怪物 cube + 树 + 云 + 旗
│   └── ai/mod.rs               # TickObserver 闭环 debug
├── target/debug/minecraft_bevy.exe
├── screenshots/                # 闭环截图
│   ├── iter_01.png             # tick ~4
│   └── iter_02.png             # tick ~9
└── README.md
```

---

## 二、运行

```powershell
$env:BEVY_DISABLE_ACCESSIBILITY = "1"
$env:RUST_LOG = "info"
cargo run
```

或者直接跑 release binary：

```powershell
.\target\debug\minecraft_bevy.exe
```

### 自动 demo 行为

启动后**不需要任何键盘输入**，游戏自己跑：

| 阶段 | 时间 | 行为 |
| --- | --- | --- |
| 启动自检 | 0-1s | 跑 100 tick headless，全部 invariants 通过则 ✅ |
| 自动 demo | 持续 | 玩家每 1.2 秒自动向随机方向走 1 格，遇 solid 向上飞 |
| 自动 orbit | 持续 | 相机绕玩家旋转，每 5 秒转 110° |
| 自动截图 | 每 5s | 保存 `screenshots/iter_NN.png`（NN 递增） |
| HUD 更新 | 每 3s | 左上角文字 overlay 更新 sim 状态 |

### 手动操作（可选）

| 键 | 动作 |
| --- | --- |
| `WASD` / `Space` / `Shift` | 玩家 1 格移动（不能穿 solid） |
| `G` | 采集当前方块 |
| `F` | 创国（消耗 wood） |
| `U` | 升级人口上限 |
| `J` | 攻击 4 格内怪物 |
| `T` | 加速 60 tick |
| `ESC` | 退出 |

---

## 三、闭环 AI 迭代演示

`screenshots/` 目录保留每次迭代的截图。实际迭代过程：

| Iter | 改动 | 结果 |
| --- | --- | --- |
| 1 | 基础体素地形 + sky 蓝 + 600 块 | 全是棕色，没水/玩家/动效 |
| 2 | 加双灯、加 ambient 1.2 | 不再黑面，水/光可见 |
| 3 | 拉近相机 24→12，加水+玩家+5 怪物 cube | 玩家仍藏在山后 |
| 4 | 加 HUD overlay (英文) | tick/资源实时可见 |
| 5 | 加旗杆+红旗+5 树+6 朵云 | 玩家从远处也能看到 |
| 6 | 玩家 demo 模式可"飞"过障碍 | 玩家升到 y=20 高空，截图完美 |

每一轮都遵循：
1. 看上一轮截图（`Read` 工具看 `iter_NN.png`）
2. 找问题（"玩家看不到" / "水不够" / "阴影全黑"）
3. 改 1-2 个函数
4. 重建（`cargo build`）
5. 重跑 + 截图
6. 对比

---

## 四、模块架构

### 资源池（守恒总线）
- 25 种资源 + max 上限
- `try_add` / `try_sub` / `force_add`
- `apply_transfer(pool, transfer)`：按 src 分类
  - 收入类（Regen/Init/PlayerGather/MonsterDrop）→ `force_add`
  - 支出类（Nation）→ `try_sub`
- `verify_conservation()` 每 tick 断言

### 3 个生态群落
| Biome | Z 区间 | 专属矿石 |
| --- | --- | --- |
| Desert | z ≥ 2n/3 | SunstoneOre |
| Tundra | n/3 ≤ z < 2n/3 | FrostcoreOre |
| Jungle | z < n/3 | LivingRoot |

### 国家系统
- 最多 8 面国旗
- 创国成本 `[10, 15, 20, 25, 30, 40, 50, 60]` 灵魂
- 人口上限 5 → 10 → 15 → 20
- HP=100，归零解散 + 释放 founding_order

### 怪物生态
- 5 王国 × 3-6 小巢 上限
- 80 小巢 + 1500 个体上限
- 5 分钟无活动 → 休眠
- 死亡：`food * 25%` 转 soul

### 体素渲染（render/mod.rs）
- 玩家周围 R 半径内的 solid 块 → spawn PBR cube
- 共享 cube mesh + 8 种 BlockType 的 shared materials
- 距离衰减 fog (start=18, end=48)

### 视觉增强（pretty/mod.rs）
- 水面（半透明蓝 plane，sea level+0.45）
- 玩家 avatar（身体/头/发/眼/腿/旗杆/红旗）— 9 个 cube
- 5 种怪物（按颜色区分 Snake/FrostElf/SandWurm/Treant/AetherWraith）
- 5 棵树（深棕树干 + 绿色 2x2x2 树冠）
- 6 朵云（白色半透明 cube，y=24-26）

### Tick 闭环 debug（ai/mod.rs）
- 50 单元测试覆盖
- 5 个 invariants：资源守恒 / 怪物计数 / 国旗上限 / 玩家出界 / tick 时长
- 5 个 anomaly 检测：决策振荡 / tick spike / 资源跳变 / 结构异变 / 国家瞬灭
- snapshot digest 便于重放

---

## 五、性能

- ~50 单元测试全过（50 passed, 0 failed）
- 启动自检 100 tick headless < 1ms
- 运行时 ~150-170 fps（AMD RX 7700 XT + 32³ 体素）

---

## 六、迭代历史

### 修过的 bug
| # | 描述 | 修复 |
| --- | --- | --- |
| 1 | `apply_transfer` 把 `PlayerGather` 当"转出"，池子里没资源 | 改按 src 分类 |
| 2 | `idx()` i32 → usize 类型错 | `as usize` |
| 3 | `Wood` BlockType 和 ResourceKind 同名歧义 | `ResourceKind as R` |
| 4 | `visible_blocks` 半开区间少 1 元素 | `..+1` |
| 5 | u64 vs i64 比较 | `as i64` |
| 6 | `try_sub(0)` 失败 | `if x > 0` |
| 7 | `dissolve` 不释放 founding_order | BTreeSet.remove |
| 8 | HUD 中文字体方块 | 改英文 |
| 9 | GameWorld 没 init_resource | 加 init_resource |
| 10 | 资源 force_add(50) 越界 | `min(50, max/2)` |
| 11 | 玩家 demo 卡在方块里 | 飞行模式：被挡时向上找空位 |

### 后续可加
- 战争迷雾（球形 + 阻挡衰减）
- 怪物 AI 决策（当前只有 MonsterMove 占位）
- 资源点再生具体逻辑
- save/load
- 阴影（当前 `shadows_enabled = false` 性能优先）

---

## 七、依赖

```toml
bevy = "0.18.1"
broccoli = "0.6.6"  # 锁定（0.6.7 触发 compt 1.10 转换）
compt = ">=1.9, <1.10"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 编译环境
- rustup 1.89.0-x86_64-pc-windows-gnu（msvc 装不下，gnu 够用）
- D:\cargo\config.toml 走 rsproxy.cn 镜像
- `BEVY_DISABLE_ACCESSIBILITY=1`

