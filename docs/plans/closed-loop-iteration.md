# 闭环迭代维护计划

> 状态：当前口径。早期 `loop.ps1`、根 PowerShell 脚本、`.harness/reins`、`scripts/diff_state.py` 方案已经收敛到 Rust `xtask`。

## 当前事实源

- 入口：`just loop`
- 直接命令：`cargo run -q -p xtask -- loop --offline --seconds 60`
- 健康检查：`just health`
- 实现位置：`xtask/src/loop_cmd.rs` 和 `xtask/src/health.rs`
- 人类运行指南：`docs/STARTING.md`
- Agent 操作规则：`.codex/skills/closed-loop-ai-dev/SKILL.md`

## 闭环契约

一轮闭环是：

```text
observe -> decide -> act -> build -> run -> health -> decision
```

`xtask loop` 负责 build/run/capture/health/template。AI 或开发者负责读取产物、判断结果，并写 `decision.md`。下一轮运行前，上一轮必须有 `decision.md`。

## 产物结构

```text
screenshots/iter_NN/
  iter_NN.png
  final_state.json
  diff.json
  assertions.json
  health.json
  health.txt
  decision.template.md
  decision.md
```

读取顺序：

1. `health.json`
2. `assertions.json`（仅 `PARTIAL` / `FAIL` 或需要断言细节时）
3. `final_state.json` 和 `diff.json`（解释状态变化时）
4. `iter_NN.png`（视觉判断需要时）
5. `decision.md`（记录本轮结论）

## Health 含义

- `PASS`：截图可读、终态可解析、tick 达标、observer invariant 通过、auto-demo 有基本进展。
- `PARTIAL`：进程运行了，但未满足完整闭环契约，通常是 tick/progression 不足。
- `FAIL`：黑屏、坏图、缺状态、observer 错误、invariant 失败、越界或视觉不可读。

## 维护原则

- 闭环逻辑留在 Rust `xtask`。不要新增根 PowerShell 工作流脚本。
- `justfile` 只保留短别名，不放复杂状态、重试、目录整理或 JSON 合约逻辑。
- Python 只用于 Blender、资产生成和一次性分析，不作为闭环运行时。
- 每次改变产物字段或 health 判定，要同步更新 `.codex/skills/closed-loop-ai-dev/SKILL.md`、`docs/STARTING.md` 和本文件。
- 视觉或体验变化必须检查 loop artifacts；PNG 评分使用 `.codex/skills/screenshot-scoring/SKILL.md`。

## 当前改进清单

- [ ] `decision.template.md` 字段和 screenshot scoring 输出格式持续保持一致。
- [ ] `final_state.json` 保持 tick、player、resource、monster/creature 关键字段稳定。
- [ ] `diff.json` 对关键资源 delta 做稳定排序，便于 AI 对比。
- [ ] `health.json` 失败原因保持短、具体、可定位。
- [ ] 自动 demo 运行 N 秒后应产生可解释的玩家/资源/actor 状态变化。
- [ ] 玩家越界、截图过小、图片近似单色、状态解析失败必须进入 assertions。
- [ ] 旧文档中引用 `loop.ps1` 的活跃入口逐步改为 `just loop` / `xtask loop`。

## 验收标准

一次闭环相关改动完成时至少满足：

- `just test-changed` 或更窄相关测试通过。
- `just loop` 能生成新的 `screenshots/iter_NN/`。
- `health.json` 存在且 verdict 可解释。
- `decision.template.md` 存在。
- 如果上一轮已有产物，`diff.json` 能解释关键状态差异。
