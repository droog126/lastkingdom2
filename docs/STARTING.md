<!-- doc-status: current -->
# 启动指南 / How to Run

所有命令都从仓库根目录运行。项目使用 Rust stable、edition 2024、Bevy 0.19 和 Rust
`xtask`；`justfile` 只提供短别名。

## 1. 一次性准备

安装 Rust、Cargo 和 `just`，然后构建客户端：

```powershell
just build
```

需要生成模型时再安装 Blender；普通构建和运行不需要 Blender。

## 2. 运行游戏

### 离线游玩

```powershell
$env:BEVY_DISABLE_ACCESSIBILITY="1"
just offline
```

离线模式在客户端进程内调用共享的 `lk2-core` 模拟逻辑。主要操作为 WASD/方向键移动、
Space 跳跃、Ctrl 加速；其他调试和玩法按键以当前 HUD 提示为准。

### 自动演示

```powershell
$env:BEVY_DISABLE_ACCESSIBILITY="1"
$env:RUST_LOG="info"
just play --auto-demo
```

此入口适合手动观察自动演示。需要完整截图、状态、health 和决策契约时使用闭环命令。

### 在线模式

```powershell
just play --online --first-person
```

`xtask play` 会管理本地服务端和客户端，并把日志写入 `run-logs/`。

## 3. 闭环迭代

```powershell
just loop
```

该别名等价于：

```powershell
just xtask loop --offline --seconds 60
```

闭环会按需构建客户端、运行 auto-demo、生成迭代目录、执行 health 检查并创建决策模板。
开始下一轮之前，上一轮必须有 `decision.md`。

阅读顺序：

1. `health.json`
2. `assertions.json`（`PARTIAL`、`FAIL` 或需要断言细节时）
3. `final_state.json` 与 `diff.json`（解释状态变化时）
4. `iter_NN.png`、`perception_manifest.json` 与 `regression.json`（视觉判断时）
5. `decision.md`（记录结论、证据和下一步）

## 4. 验证命令

```powershell
just test-changed
just test-core
just test
just audit-tdd
just audit-architecture
just audit-docs
just audit-skills
just fmt
just clippy
```

视觉、HUD、玩法体验、auto-demo、截图或观察状态发生变化后，还要运行 `just loop`。

## 5. 输出位置

闭环目录 `screenshots/iter_NN/` 包含：

| 文件 | 用途 |
| --- | --- |
| `iter_NN.png` | 主截图 |
| `final_state.json` | 最终模拟状态 |
| `diff.json` | 相对前一轮的关键状态差异 |
| `assertions.json` | health 断言详情 |
| `health.json` / `health.txt` | 机器可读与文本健康结论 |
| `error_logs.json` / `error_logs.txt` | 错误级日志摘要 |
| `perception_manifest.json` | 截图与观察证据清单 |
| `regression.json` | 与前一轮的视觉回归摘要 |
| `decision.template.md` | 生成的决策模板 |
| `decision.md` | 完成后的人工/AI 决策记录 |

普通 `just play` 写入：

- `run-logs/play.log` 和 `run-logs/play.log.err`
- `run-logs/error_logs.json` 和 `run-logs/error_logs.txt`
- 在线模式的 `run-logs/play_server.log` 和 `run-logs/play_server.log.err`

不要把构建日志或运行产物写到仓库根目录。

## 6. 常见问题

- 如果 Vulkan 驱动路径异常，可用 `just play --gpu-backend=dx12` 对比。
- 如果 Windows 报可执行文件被占用，先确认该进程属于本仓库，再结束对应客户端、服务端或构建进程。
- 如果闭环拒绝启动，先读取上一轮 `health.json` 和 `decision.template.md`，补全或修正上一轮决策。
- 如果 health 为 `PARTIAL`，它仍然需要在 `decision.md` 中解释，不能按 `PASS` 处理。

## 7. 文档入口

- [文档地图](README.md)
- [工程基线](architecture/engineering-baseline.md)
- [TDD 工作流](notes/tdd.md)
- [闭环契约](plans/closed-loop-iteration.md)
