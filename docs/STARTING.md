<!-- doc-status: current -->
# 启动指南 / How to Run

所有命令都从仓库根目录运行。项目使用 `rust-toolchain.toml` 固定的 Rust 1.96.0、edition 2024、Bevy 0.19 和 Rust
`xtask`；`justfile` 只提供短别名。

## 1. 一次性准备

安装 Rust、Cargo 和 `just`，然后构建客户端：

```powershell
just build
```

## Three-view model optimization

Render front, side, and top views for one GLB and create an AI-readable task bundle:

```powershell
just model-optimize animals/rabbit
```

The bundle is written under `screenshots/model_optimize/` and contains `front.png`,
`side.png`, `top.png`, `task.json`, `task.md`, and the preview manifest. Use an
assets-relative path when a model stem is ambiguous across collections.

## Content export

```powershell
just export
```

This writes a human-readable `content.md` catalog, the complete registry plus
`content_active.*` and `content_planned.*` views, `recipes.csv`, and `manifest.json` to the
current directory. `just export-content` remains an alias. Use `just export --out=PATH` for
another output directory.

需要生成模型时再安装 Blender；普通构建和运行不需要 Blender。

额外工程命令需要安装 `cargo-nextest`、`cargo-machete`、`cargo-llvm-cov` 和 `cargo-insta`：

```powershell
cargo install --locked cargo-nextest cargo-machete cargo-llvm-cov cargo-insta
```

CI 明确使用 crates.io sparse 索引。国内开发环境需要镜像时，将下面配置写入用户级
`$CARGO_HOME/config.toml`，不要修改仓库的 `.cargo/config.toml`：

```toml
[source.crates-io]
replace-with = "ustc"

[source.ustc]
registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"
```

## 2. 运行游戏

### 离线游玩

```powershell
$env:BEVY_DISABLE_ACCESSIBILITY="1"
just offline
```

离线模式在客户端进程内调用共享的 `lk2-core` 自然模拟和近战规则。主要操作为
WASD/方向键移动，鼠标左键或 Space 攻击，Esc 退出。

### 在线模式

`just play --online` 会构建并启动 `lk2-server`，然后连接 focused client 的 Lightyear 在线场景。
当前在线场景已接入 Leafwing 输入、移动/跳跃、基础 gameplay 消息、基础 HUD 和生态计数表现；更完整的 HUD、生态表现和多玩家状态仍在迁移中。

### Codex 客户端

Codex 客户端是一个进入在线主场景的真实 Lightyear 客户端，不拥有权威状态；它使用同一套窗口、相机、地形、玩家视觉和 HUD，
只把人的输入替换为本机 `codex exec` 返回的受限结构化行动，再通过现有 `GameplayCommand` 发给服务器。服务器仍负责校验。

```powershell
# 确定性本地回退客户端（先在另一个终端启动 just server）
just ai-client --connect=127.0.0.1:5000 --seconds=60

# 使用本机 Codex CLI 作为决策者
just codex-client --connect=127.0.0.1:5000 --seconds=60

# 同时启动服务器、画面客户端和 Codex 客户端，并生成闭环工件
just loop

# 显式 Codex 别名，等价于 just loop
just loop-codex
```

To keep the latest loop image as a durable milestone, run:

```powershell
just milestone
```

This copies the latest `screenshots/iter_NN/iter_NN.png` to a timestamped file under the
top-level `milestones/` directory. The archive is visible to Git and is preserved by
`just clean-runs`; it does not rerun the loop, modify `screenshots/`, or archive the JSON evidence.

During normal offline or online play, press `F12` to save the current game window directly as
`milestones/milestone_<timestamp>_<sequence>.png`. This player shortcut does not run a loop and
does not write under `screenshots/`.

`loop`（或 `loop-codex`）会在当前 `screenshots/iter_NN/` 写入 `codex_client.json`；其中的连接、观察 tick、Codex 决策、行动和
服务器反馈计数是 Codex 客户端是否真正入场的机器证据。`loop-ai` 和 `ai_client.json` 保留为确定性本地回退路径。

## 3. 闭环迭代

原有 artifact schema 和历史迭代仍保留。focused client 现在能生成基础 auto-demo、
`final_state.json`、`diff.json` 和 `iter_NN.png`；`health.json`、assertions、regression 和
decision 模板仍由 `xtask` 生成。不要把旧迭代结果当作当前客户端的运行证据。

自然世界观测还会在可用时记录 `event_count`、`region_tick`、`catch_up_remaining` 和
`save_restored`；这些字段是可选的，缺失只表示生产者仍使用旧状态格式，不替代模拟成功、投影成功和存档成功的独立断言。

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
just test-nextest
just coverage
just deps-unused
just snapshots
just audit-tdd
just audit-architecture
just audit-docs
just audit-skills
just fmt
just clippy
```

视觉、HUD、玩法体验、auto-demo、截图或观察状态发生变化后，不要默认运行 `just loop`。仅当用户明确要求、验收条件明确要求、复现/诊断必须依赖运行时工件，或最终结论要声明真实渲染画面已验证时，才运行闭环。

## 5. 输出位置

闭环目录 `screenshots/iter_NN/` 包含：

| 文件 | 用途 |
| --- | --- |
| `codex_client.json` | Codex client connection, observation, decision, and server-feedback artifact |
| `ai_client.json` | Deterministic fallback AI client artifact |
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
