<!-- doc-status: current -->
# 闭环迭代契约

## 事实源

- 入口：`just loop`
- 目标直接命令：`just xtask loop --offline --seconds 60`
- 健康检查：`just health`
- 实现：`xtask/src/loop_cmd.rs`、`xtask/src/health.rs`、`xtask/src/audit.rs`
- 人类运行指南：[STARTING.md](../STARTING.md)
- Agent 工作流：`.codex/skills/closed-loop-ai-dev/SKILL.md`

## 单轮流程

```text
observe -> decide -> act -> build -> run -> health -> decision
```

`xtask loop` 负责构建、运行、截图、状态采集、health 和决策模板。开发者或 AI 负责读取
证据、判断结果并完成 `decision.md`。上一轮没有决策记录时，不得开始下一轮。

当前 focused client 已恢复基础 auto-demo、状态记录和截图生产者。下述流程和产物结构是
兼容契约；一次迭代是否成功仍以实际运行后生成的 health/assertions/decision 证据为准。

## 产物结构

```text
screenshots/iter_NN/
  iter_NN.png
  final_state.json
  diff.json
  assertions.json
  health.json
  health.txt
  error_logs.json
  error_logs.txt
  perception_manifest.json
  regression.json
  decision.template.md
  decision.md
```

读取顺序：

1. `health.json`
2. `assertions.json`（非 `PASS` 或需要断言细节时）
3. `final_state.json` 与 `diff.json`（解释状态变化时）
4. `iter_NN.png`、`perception_manifest.json` 与 `regression.json`（判断视觉证据时）
5. `error_logs.json`（诊断运行错误时）
6. `decision.md`（记录结论与下一步）

## Health 语义

- `PASS`：硬门槛通过，截图、状态、观察者和进度证据满足闭环契约。
- `PARTIAL`：运行完成但一个或多个软门槛未满足，需要在决策中解释和跟进。
- `FAIL`：硬门槛失败，例如坏图、缺失状态、越界、observer/invariant 错误或运行错误。

不要把 `PARTIAL` 当成成功，也不要只凭 PNG 覆盖机器可读失败证据。

## 维护规则

- 闭环逻辑留在 Rust `xtask`，`justfile` 只保留短别名。
- Python 仅用于 Blender、资产生成和一次性分析，不作为闭环运行时。
- 改变产物文件、JSON 字段、health 规则或决策模板时，同步更新本文件、
  `STARTING.md` 和 `$closed-loop-ai-dev`。
- 视觉或体验变化本身不要求检查 loop artifacts；仅当用户明确要求、验收条件明确要求、复现/诊断必须依赖运行时工件，或最终结论要声明真实渲染画面已验证时，才检查闭环产物。
- 运行 `just audit-docs`，防止当前契约重新漂移。

## 验收标准

维护 `just loop` 时，以下条件必须保持成立：

- `xtask loop` 能生成完整迭代目录。
- `health.json` 能区分 `PASS`、`PARTIAL` 和 `FAIL`。
- `assertions.json` 为非通过结论提供可定位原因。
- `final_state.json` 与 `diff.json` 能解释关键模拟变化。
- `error_logs.*` 能汇总错误级运行日志。
- 前一轮缺少 `decision.md` 时，下一轮被明确阻止。
