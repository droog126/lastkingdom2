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

### ISSUE-009 — auto-demo TopDown 路径 12s 截图空蓝屏
- 来源: iter_08 PARTIAL 6.0
- 现象: render/mod.rs:2346 hardcode `camera_offset = Vec3::new(-48, 64, 38)` + ortho
  viewport 72m → 玩家脚下 18m 范围在 720p 截图里只占 ~10px, 完全看不到
- 失败断言:
  - `png.color_dominance` 95.2% 蓝主导 (空场景的派生指标)
  - `visual.player_readability_marker_present` 0 (TopDown 模式没渲染 player marker)
- 修复路径: 三选一
  - (a) TopDown 相机降低到 Y=22-25m, 跟玩家 6-8m 距离
  - (b) auto-demo 截图前切回 FirstPerson 模式 (用户视角)
  - (c) ortho viewport height 从 72m 减到 18-25m (village-scale)
- 优先级: P0 (跟 user 之前说"地图光秃秃"是同根, ISSUE-001 关闭前提是 ground 真渲染)


## Resolved

### ISSUE-001 — kenney landmark Y≈19-20m 高空, first-person 看不到
- 关闭: iter_08 (2026-07-10 23:25)
- 修法: 9d7eb15 pretty 重构 (KenneyLandmark removed, GroundDetail layer, effective_ground_height
  贴地) + 后续 iter_199/201
- 证据: iter_08 luma 172.8 vs iter_07 171.2 (Δ=1.6, no flicker); frame_spikes
  109→OK, terrain_despawns 224→OK
- 关闭人: orchestrator (本轮)

### ISSUE-008 — auto-demo TopDown 截图空蓝屏, 不能验证 ground/landmark/monster
- 关闭: iter_08 同次 close, **重新打开为 ISSUE-009 (见 Open)**, 状态变更而已
- (此处不重复登记, 详情见 Open 段)

## Wontfix

(none yet)