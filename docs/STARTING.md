# 启动指南 / How to Run

> 当前工程入口在 `F:\rustProject\lastkingdom2`。根 `minecraft_bevy`、旧 `launchers/`、根 PowerShell 闭环脚本和 `scripts/` 工作流运行时都是历史口径。当前自动化事实源是 Rust `xtask`，`justfile` 只做短命令别名。

## 0. 一次性准备

第一次 clone 后要装依赖并编译一次：

```powershell
cd F:\rustProject\lastkingdom2
just build
```

> Rust edition 2024，需要 Rust 1.75+。`Cargo.toml` 已固定 `compt = ">=1.9, <1.10"`（broccoli 0.6 配套版本）— **不要 bump 它**。

### 国内镜像加速（可选）

如果访问 crates.io 慢，配置 rsproxy 镜像：

```powershell
# 用户环境变量
$env:RUSTUP_DIST_SERVER = "https://rsproxy.cn"
$env:RUSTUP_UPDATE_ROOT = "https://rsproxy.cn/rustup"
```

`~/.cargo/config.toml`：

```toml
[source.crates-io]
replace-with = 'mirror'

[source.mirror]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
```

## 1. 三种运行姿势

### 1a. 手动玩（离线客户端）

```powershell
cd F:\rustProject\lastkingdom2
$env:BEVY_DISABLE_ACCESSIBILITY="1"
just offline
```

打开一个 1280×720 的窗口，出生在 `96³` 世界的中心。

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

### 1b. 自动演示（无输入 / 调试用）

```powershell
cd F:\rustProject\lastkingdom2
$env:BEVY_DISABLE_ACCESSIBILITY="1"
$env:RUST_LOG="info"
cargo run -p lk2-client -- --offline --auto-demo
```

自动演示会驱动基础场景、HUD 和截图输出。需要完整 AI 闭环时优先用 `just loop`，因为它会同时处理 build、health、状态差异和决策模板。

### 1c. 项目闭环（推荐的 AI 迭代姿势）

```powershell
cd F:\rustProject\lastkingdom2
just loop
```

等价于：

```powershell
just xtask loop --offline --seconds 60
```

`xtask loop` 会按需 build `lk2-client`，运行离线 auto-demo，生成 `screenshots\iter_NN\` 目录，执行 health 检查，并写入 `decision.template.md`。下一轮运行前，上一轮必须有 `decision.md`。

闭环阅读顺序：

1. 先读 `screenshots\iter_NN\health.json`。
2. 如果是 `PARTIAL` 或 `FAIL`，读 `assertions.json`。
3. 需要解释状态变化时读 `final_state.json` 和 `diff.json`。
4. 只有视觉判断需要时再打开 `iter_NN.png`。
5. 把本轮判断写入 `decision.md`。

## 2. 改完代码怎么看效果？

常用验证：

```powershell
just test-changed
just test
just audit-tdd
just fmt
just clippy
```

视觉、玩法体验、HUD、auto-demo、截图或状态观察改变后，再跑：

```powershell
just loop
```

## 3. 常见问题

| 现象 | 原因 | 解决 |
| --- | --- | --- |
| 启动后窗口黑屏几秒 | Vulkan 加载 + 96³ Greedy Mesh 构建 | 等 1-2 秒；首次会很慢 |
| 终端一片 `VK_LAYER_KHRONOS_validation` 红字 | 没装 Vulkan 验证层 | 忽略，不影响运行 |
| HUD 中文显示豆腐块 / 终端 `Path not found: fonts/NotoSansCJKsc-Regular.otf` | 字体 asset 路径找不到 | 检查 `assets/fonts/NotoSansCJKsc-Regular.otf` 是否可被 Bevy asset root 找到 |
| 鼠标锁死在窗口中央 | FPS mouse-look 默认开 | 按 `Esc` 解锁；或用 `--auto-demo` |
| `just loop` 拒绝运行并提示缺 `decision.md` | 上一轮闭环没有记录决策 | 根据 `decision.template.md` 写 `screenshots\iter_NN\decision.md` 后再跑 |

## 4. 输出文件位置

| 文件 | 说明 |
| --- | --- |
| `target\debug\lk2-client.exe` | 客户端二进制 |
| `target\debug\lk2-server.exe` | 服务端二进制 |
| `screenshots\iter_NN\iter_NN.png` | 每轮主截图（含 HUD overlay） |
| `screenshots\iter_NN\final_state.json` | 每轮终态 sim 状态 |
| `screenshots\iter_NN\diff.json` | 相对上一轮的关键状态差异 |
| `screenshots\iter_NN\assertions.json` | health 断言详情 |
| `screenshots\iter_NN\health.json` | 闭环健康结论，优先读取 |
| `screenshots\iter_NN\decision.template.md` | xtask 生成的决策记录模板 |
| `screenshots\iter_NN\decision.md` | 本轮人工/AI 决策记录，下一轮前必须存在 |
| `run-logs\*.log` | xtask 编译/运行日志 |

运行产物写到 `screenshots/` 或 `run-logs/`，不要往根目录写 `build_xxx.log`。

## 5. 文档入口

- `AGENTS.md`：项目技能路由和同步策略。
- `.codex/skills/*/SKILL.md`：AI agent 的详细操作规则。
- `docs/architecture/engineering-baseline.md`：当前工程边界和自动化事实源。
- `docs/plans/closed-loop-iteration.md`：当前闭环维护计划。
- `docs/notes/tdd.md`：TDD 入口和 backlog。

## 6. TL;DR

```powershell
cd F:\rustProject\lastkingdom2

# 玩
just build
just offline

# 改完代码
just test-changed

# AI 闭环
just loop
# 先读 screenshots\iter_NN\health.json，再按需读 assertions/final_state/diff/PNG
```
