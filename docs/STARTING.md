# 启动指南 / How to Run

> 万国起源：最后一国 钻石版 — `F:\rustProject\lastkingdom2` 的所有"怎么跑起来"在这里。

---

## 0. 一次性准备

第一次 clone 后要装依赖 + 编译一次（冷编译 ~22 分钟，增量 ~1-30 秒）：

```powershell
cd F:\rustProject\lastkingdom2
cargo build
```

> Rust edition 2024，需要 Rust 1.75+。`Cargo.toml` 已固定 `compt = ">=1.9, <1.10"`（broccoli 0.6 配套版本）— **不要 bump 它**。

---

## 1. 三种运行姿势

### 1a. 手动玩（FPS 第一视角）

```powershell
cd F:\rustProject\lastkingdom2
$env:BEVY_DISABLE_ACCESSIBILITY="1"   # 跳过 Windows 辅助 API，启动快很多
.\target\debug\minecraft_bevy.exe
```

打开一个 1280×720 的窗口，出生在 `96³` 世界的中心。

**操作**：

| 键 | 动作 |
| --- | --- |
| `WASD` / 方向键 | 移动（相对相机） |
| `Space` | 跳 |
| `Shift` | 下潜 / 缓慢下降 |
| `Q` / `E` | 转向 22.5°（无鼠标时备胎） |
| 鼠标移动 | 视角（mouse-look 默认开） |
| `G` | 挖当前脚下方块 |
| `K` | 挥剑 |
| `F` | 造国（消耗 10 灵魂） |
| `J` | 攻击 2 格内最近怪物 |
| `Esc` | 退出 |

逻辑驱动的移动（不是物理）：尝试往 solid 块方向走会自动向上找空位（最多 +6）。

### 1b. 自动演示（无输入 / 适合 AI 迭代）

```powershell
cd F:\rustProject\lastkingdom2
$env:BEVY_DISABLE_ACCESSIBILITY="1"
$env:RUST_LOG="info,minecraft_bevy=info,pvp=warn,controller=warn"
.\target\debug\minecraft_bevy.exe --auto-demo
```

行为：
- 玩家不动（保留在出生点附近，能看见起始牧场）
- 相机自动跟最近的动物（auto-follow），**不读鼠标**
- `t=1.0s` 自动按 F（造国）+ `t=4.0s` 自动按 J（杀怪）+ `t=8.0s` 再按 F（验证"已有国家"分支）
- **每 5 秒**自动截一张全屏图到 `screenshots\iter_NN.png`
- **每 5 tick** 自动 dump 一次状态 JSON 到 `screenshots\state_tNN.json`（t=5, 10, 15, ...）

HUD 直接显示在截图上（左上角）：tick、玩家坐标、4 个资源、怪物数、invariant 状态、动物方向箭头。

### 1c. 项目自带的闭环脚本（`Agent.md` 推荐的 AI 迭代姿势）

```powershell
cd F:\rustProject\lastkingdom2
.\loop.ps1
```

等价于：`cargo build` + 启动 .exe 12 秒 + 杀进程 + 准备下一轮。AI agent 读最新截图 → 决定改什么 → 改代码 → 再跑。

---

## 2. 改完代码怎么看效果？

**增量编译**（只改了 src/ 下文件）：

```powershell
cd F:\rustProject\lastkingdom2
cargo build
# 第一次：~22 分钟（cold）  后续：1-30 秒
```

然后启动 .exe 看效果（参考上面 §1a 或 §1b）。

**只跑测试**（不生成 .exe）：

```powershell
cargo test --workspace
```

**代码质量**：

```powershell
cargo clippy --workspace   # lint
cargo fmt                  # 自动格式化（rustfmt.toml: max_width=100）
```

---

## 3. 常见问题 / "为什么没效果"

| 现象 | 原因 | 解决 |
| --- | --- | --- |
| 启动后窗口黑屏几秒 | Vulkan 加载 + 96³ Greedy Mesh 构建 | 等 1-2 秒；首次会很慢 |
| 终端一片 `VK_LAYER_KHRONOS_validation` 红字 | 没装 Vulkan 验证层 | 忽略，不影响运行 |
| HUD 中文显示豆腐块 / 终端 `Path not found: fonts/NotoSansCJKsc-Regular.otf` | 字体 asset 路径找不到 | 把 `assets/fonts/NotoSansCJKsc-Regular.otf` 复制到 `target\debug\assets\fonts/`，或在 Cargo.toml 加 `asset` |
| 天空是纯黑色 | `day_night_cycle` 的 ClearColor 逻辑有 bug | pre-existing 毛病，不在 Greedy Mesh 范围；等修 |
| 准星在左上角而不是正中央 | `left: px(50.0)` 是 50px，不是 50% | pre-existing UI bug，等修 |
| 启动后立刻 panic，提示 `bevy_pvp::ActionState not found` | PvP 的 InputMap 资源没插 | 已知，运行 demo 无影响（已用 `Option<ResMut>` 容错） |
| 启动后立刻 panic，提示 `Message not initialized` | 某个 `Message<T>` 没 `.add_message::<T>()` | 已知，运行 demo 无影响（已注册全部消息） |
| 启动后立刻 panic，提示 `min=[0,0,0] max=[96,96,96] out of bounds` | `block-mesh` 0.2 的 3×3×3 kernel 需要 `chunk_shape` 比世界 +1 | 已用 `ConstShape3u32<97, 97, 97>` 解决 |
| 鼠标锁死在窗口中央 | FPS mouse-look 默认开 | 按 `Esc` 解锁；或 `--auto-demo` 模式自动关 mouse-look |
| 看不到 `.exe` 产物 | 第一次 build 没跑完 | 跑一次 `cargo build` 即可（22 分钟） |

---

## 4. 输出文件位置

| 文件 | 说明 |
| --- | --- |
| `target\debug\minecraft_bevy.exe` | 主二进制 |
| `screenshots\iter_NN.png` | 启动后每 5 秒一张的截图（含 HUD overlay） |
| `screenshots\state_tNN.json` | 每 5 tick 的 sim 状态（玩家坐标、资源、怪物、invariant 违例） |
| `target\debug\deps\` | 增量编译缓存（删掉等于 cold rebuild） |
| `log\*.log` | 编译/运行日志（`cargo build > log\build.log` 这种） |

> `.gitignore` 已经忽略 `target/`、`screenshots/iter_*.png`、`*.log` 和 `log/`。  
> **约定：所有 build/run log 写到 `log/` 下，别再往根目录喷 `build_xxx.log`。**

---

## 5. 想要更详细的 dev 流程？

看项目根目录：

- `Agent.md` — 给接手 AI agent 的完整操作手册（4 阶段闭环 + 视觉目标 + 踩坑清单）
- `AGENTS.md` — 项目级约定（cargo 风格、PR 流程、闭包架构）
- `docs/`（本目录）— 各种专题文档：架构、规划、动画系统、资源系统等
  - `docs/architecture_plan_v2.md` — 架构总览
  - `docs/short_term_plan_v3.md` — 短期迭代计划

---

## 6. TL;DR

```powershell
# 玩
cd F:\rustProject\lastkingdom2
cargo build
$env:BEVY_DISABLE_ACCESSIBILITY="1"
.\target\debug\minecraft_bevy.exe

# AI 迭代（截图 + 状态）
.\target\debug\minecraft_bevy.exe --auto-demo
# → 读 screenshots\iter_NN.png 和 state_tNN.json

# 改完代码
cargo build
# 再跑
```
