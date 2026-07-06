# Known Issues

活清单。每轮 `decision.md` 收尾时必须同步：把 `problems:` 段里没解决的项开 ISSUE，把已修的标 resolved。
下一轮 coder 启动前必读 **open** 段，否则 `xtask loop` 的 decision gate 会 warn。

## Status legend

- `open` — 未修，下轮必须处理或显式 carry
- `in-progress` — 当前 iter 正在修
- `resolved` — 已修，写明 commit hash + 关闭 iter
- `wontfix` — 不修，写明原因

## Open

### ISSUE-007 — health.rs assertion 太薄 (13 条) 漏掉 HUD/网络/视野/日志
- 来源: iter_201 元调研 (本轮建设规划)
- 现象: health verdict = PASS 但用户视觉/手感上大量问题
- 漏查项:
  - HUD hint 文案合理性（"Sheep 25m"、"Nest 6m / 20 mobs"）
  - kenney landmark Y 坐标是否在玩家视野内 (1-2m 高, 不是 19m 高空)
  - `network_command.move_world_sent` / `last_dx_milli` 是否真的在涨
  - `camera.mode` 是否与任务目标一致 (first-person 任务必须是 first-person)
  - stderr 日志关键字 (`OUT OF BOUNDS`, `deserialize invalid entity`, `体素过多`, `Attempting to deserialize`)
  - vs-prev 回归 (monsters/nations/nests 数量级跌 50% 报 partial)
- 根因: `xtask/src/health.rs::built_in_assertions` 只查 13 条, 都是"程序没崩"
- 修复路径: 下轮扩到 30+ 条, 加 stderr 扫描 + diff regression (本期方案 #2 + #3)
- 优先级: P0

### ISSUE-006 — fail-silent: decision.md 缺失时 loop 还能跑
- 来源: iter_199 → iter_200 (coder 失联后无人决策)
- 现象: iter_199/200 没 `screenshots/iter_NNN/decision.md` 但 iter_200 跑出了数据
- 根因: `enforce_decision_gate` 检查文件存在即可, 没检查 health.json verdict
- 修复路径: 本期方案 #4 已落, 加强 gate 读 health.json verdict
- 优先级: P0

### ISSUE-003 — server 端 deserialize invalid entity 风暴 (lightyear)
- 来源: iter_199 task.md Root Cause A, iter_199_e2e_server.log
- 现象: server 每 ~5ms 报 `Attempting to deserialize an invalid entity`, 100+ 条连续
- 怀疑: lightyear protocol registry 缺组件 (`PlayerPos`/`PlayerState`/`Replicate`/`ControlledBy`)
- 修复路径: 修 lightyear 协议注册对齐; 不行就 throttle ERROR 日志路径
- 优先级: P0

### ISSUE-002 — WASD 走两步卡一下 (MOVE 链路未通)
- 来源: iter_199 task.md Root Cause B
- 现象: client 发出 MOVE, server `apply_udp_gameplay_commands` 不动; `last_move_applied=true` 后 player.pos 不变
- 怀疑: i16 截断溢出 / player_q 找不到 / lightyear receive 线程阻塞
- 修复路径: 跑 `just client-online 30s` trace 实际定位, **不要猜**
- 优先级: P0

### ISSUE-001 — kenney landmark Y≈19-20m 高空, first-person 看不到
- 来源: iter_199 task.md Root Cause C, iter_200 final_state.json monsters.current=20 (vs prev 60)
- 现象: pretty 模块 spawn kenney landmark 时 Y=34.65-35.35 (≈19-20m), 玩家 spawn Y=15, 平视看不到
- 根因: `crates/client/src/pretty/mod.rs` kenney landmark spawn 没贴地
- 修复: Y 改成 hardcode 16.0 或读 terrain (x,z) 表面高度
- 优先级: P0 (用户当前最直接不满的"地图光秃秃"症状)

### ISSUE-004 — walk_step 间隔 3s 太久, 12s loop 捕不到
- 来源: iter_196 verifier_recheck.md
- 现象: auto-demo walk 12s 跑只挪 2 步, 视觉验证弱
- 修复路径: walk_step 1s 一步, 或 trigger-on-spawn
- 优先级: P1

### ISSUE-005 — 截图节流 150 ticks 太保守, 12s hidden window 跑不到
- 来源: iter_196 verifier_recheck.md, iter_197 实际有改到 75
- 现象: 12s hidden window sim 跑得比预期慢 (~5.6 TPS vs 30 TPS), 触发不到 150 ticks 阈值
- 状态: iter_197 已改到 75, 待验证稳定性
- 优先级: P2

## Resolved

(none yet)

## Wontfix

(none yet)