# 闭环迭代系统完善计划

> 日期: 2026-06-07
> 状态: 草稿，等用户拍板后进 `.harness/reins/developer` 执行
> 范围: 闭环 (build → run → capture → read → decide → edit → 循环) 的工具链 + 游戏本体的可读性

---

## 1. 背景 & 用户诉求

用户原话: "继续完善AI模拟游戏的，迭代游戏的，抓tick 日志，抓截图的闭环系统。然后去看docs的文档，然后去开发游戏。"

拆成两件事:
1. **完善闭环系统** — 让 capture + read 阶段更自动化、AI 友好
2. **开发游戏** — 通过闭环驱动游戏本体的改进（视觉、玩法、稳定性）

读了 `Agent.md` (4 阶段闭环) / `AGENTS.md` (项目约定) / `loop.ps1` (执行脚本) / `docs/总纲.md` (3 套架构) / `docs/STARTING.md` (启动指南) / `docs/目标.md` (本次的工作方向)。

---

## 2. 现状盘点 (基于 screenshots/ 实际跑出来的产物)

| 模块 | 状态 | 证据 |
|------|------|------|
| `cargo build` | ✅ 工作 | 53s 增量 |
| `loop.ps1` 全流程 | ✅ 工作 | build → run 12s → kill → 列出 iter |
| `periodic_screenshot` | ✅ 工作 | screenshots/iter_07.png (483KB, 1280×720) |
| `tick_recorder` (每 5 tick) | ✅ 工作 | screenshots/state_t35.json (646B) |
| `scenario::record_default.jsonl` | ⚠️ **泄漏** | 29MB append-only，跨会话累积，从未清理 |
| `scenario_tick_recorder` (RecordBegin/End) | ⚠️ **可能不闭合** | 默认 scenario 启动后开始录，无明确结束触发 |
| 终点 state.json (单文件) | ❌ 缺失 | 只能拿到每 5 tick 的快照，没"这一轮世界长这样" |
| 跨 iter diff | ❌ 缺失 | 没法回答"上一轮 food=120，这轮 food=130" |
| 决策日志 | ❌ 缺失 | AI 改了什么是口述的，没留痕 |
| iter 目录化 | ❌ 平铺 | screenshots/ 下 383 个文件混在一起，定位难 |
| 视觉可读性 | ⚠️ **很差** | iter_07.png: 天空全黑，HUD 几乎不可见，地形碎片化 |

---

## 3. 看到的问题 (按优先级排)

### P0 — 闭环数据泄漏 / 阻碍 AI 决策

1. **record_default.jsonl 29MB 泄漏**
   - 原因: `src/scenario/mod.rs:254-274` `RecordBegin/RecordEnd` 没有失败关闭路径；buffer 累积到 `RecordEnd` 才 `fs::write`
   - 影响: 跨会话累积，每次 `loop.ps1` 跑都变胖，AI 看 ls 一脸懵
   - 修法: 加 (a) 最大行数截断 (b) iter 启动时把 `screenshots/record_*.jsonl` mv 到 `iter_NN/raw.jsonl` (c) 默认 scenario 不要走 RecordBegin

2. **每轮只产 state_tNN.json + iter_NN.png，没"终态快照"**
   - 影响: AI 想问"这轮游戏死没死"得自己挑最后一个 state
   - 修法: run 结束前 dump 一个 `iter_NN/final_state.json` (与 state_tNN 格式同，但最后一个 tick)

3. **没 diff 上轮**
   - 修法: 跑完一轮后 `loop.ps1` 调用一个小脚本生成 `iter_NN/diff.json` ({ "wood": +10, "monsters": -2, ... })

### P1 — 让 AI 一次读够

4. **iter 平铺**
   - 修法: `loop.ps1` 把每次 run 收进 `screenshots/iter_NN/` 目录:
     ```
     iter_07/
       iter_07.png            # 主截图 (最后一帧)
       final_state.json       # 终点 sim 状态
       raw.jsonl              # 录制流 (从根挪进来)
       diff.json              # vs iter_NN-1
       meta.json              # { started, duration, build_sec, exit_code, ... }
       build_loop.log         # 截本次 build 输出最后 200 行
     ```
   - 老的平铺文件保留 5 轮后自动 mv 进 `_archive/`

5. **决策日志**
   - 修法: `loop.ps1` 接受 `-Note "修复了 black sky"` 参数，写进 `iter_NN/meta.json:note`
   - 后续 `iter_NN/decisions.md` 累积记录: iter_NN: [black-sky] + 改了哪个文件 (git diff stat)

### P2 — 视觉可读性 (这是 AI 截图能看出东西的前提)

6. **iter_07.png 天空全黑**
   - Agent.md P0 列表 "天空不是黑色" 打了 ✅ 但**实际还是黑的** — 这是 stale checkbox
   - 修法: `setup_atmosphere` 把 `ClearColor` 改成固定天蓝色 (白天)，或实现 `day_night_cycle` 但默认 t=12:00

7. **HUD 几乎不可见**
   - 截图左上角只有一个白色小方块 — 像是某个字符显示成方块
   - 修法: 排查 `setup_hud` / `update_hud` 的中文字符，临时全换英文 + ASCII

8. **玩家 / 怪物 / 地形分不清**
   - iter_07.png 红色块 (玩家附近地面) + 蓝色块 (远处怪物) + 黑色背景
   - 修法: 调相机起始位置 (已经改 Y=80 高空起步) + 给 player avatar 加发光描边

### P3 — gameplay / quality of life (已部分在做)

9. **出生在 Y=80 高空** — 已改未测
10. **C 切 3rd person** — 已改未测
11. **F5 紧急传送** — 已加
12. **F3 灵魂出窍** — 已加
13. **平坦出生地** — Agent.md P1 "未做"，未启动
14. **远景雾 / 阴影** — Agent.md P2，未启动
15. **怪物 AI 真实移动** — Agent.md P2，未启动

---

## 4. 推荐路径

> **一个总原则: 闭环是工具，游戏是产出。** 先让 AI 能稳定地"看图 → 决策 → 改 → 再看图"，再让游戏变好看。

### Sprint 1: 闭环健壮性 (1-2 轮 iter)
- 修 record_default.jsonl 泄漏 (3 选 1: A 加截断 / B 改名 + 移走 / C 默认 scenario 不录)
- 加 `iter_NN/` 目录化输出
- 加 `final_state.json` + `diff.json` + `meta.json`
- 老的平铺 file 跑完一次后 mv 到 `_archive/`

**验证**: 跑 `loop.ps1` 三次，看 `screenshots/iter_03/diff.json` 真的有数据，看 jsonl 不再涨

### Sprint 2: 视觉可读性 (1-2 轮 iter, 用新闭环)
- 修天空 (固定白天蓝)
- 修 HUD (排查字体路径 / 临时换英文)
- 重新 mesh 测试，确保相机在 Y=80 起步能看到 96³ 世界

**验证**: iter 截图能一眼看出 "地形 + 玩家 + HUD + 天空"，AI 看图能识别具体问题

### Sprint 3: 玩法级 (看游戏状态决定)
- 平坦出生地 (在 (48, ground, 48) 周围 8×8 填草地)
- F3/F5/C 已加的，等 build 完测试
- 怪物 AI: 引入 Wander state (P2)

### Sprint 4: 远期 (P2/P3)
- 阴影 / 远景雾 / save/load

---

## 5. 关键决策 (我倾向的选项，等你点头)

| 决策点 | 我的推荐 | 备选 |
|--------|----------|------|
| 闭环目录化 vs 平铺 | **目录化** `iter_NN/` | 保持平铺 + 加 manifest |
| 旧平铺文件怎么办 | **mv 到 `_archive/`** | 直接 `mavis-trash` (可恢复) |
| record jsonl 默认录不录 | **不录** (默认 scenario 去掉 RecordBegin) | 留但加截断 |
| 天空修法 | **固定天蓝** (15 行代码) | 实现完整 day_night_cycle (200+ 行) |
| HUD 字体 | **临时换英文** (3 处改) | 让你提供 CJK .otf 路径 |
| 玩家出生地 | **Y=80 高空** (已改) | 保持 SEA_LEVEL+1=13 |
| 团队协作 | **`mavis-team` plan** 分配给 developer/iter-tester/code-reviewer | 我直接全做 |

---

## 6. 关键文件 / 系统

| 文件 | 改什么 |
|------|--------|
| `src/scenario/mod.rs:240-274` | RecordBegin/End 加截断；默认 scenario 不录 |
| `src/main.rs:621-637` | `periodic_screenshot` 路径改为 `iter_NN/iter_NN.png` |
| `src/main.rs:698-740` | `tick_recorder` 加 final_state.json dump 钩子 |
| `src/render/mod.rs` | `setup_atmosphere` 修天空；`setup_hud` 改英文 |
| `loop.ps1` | 加 iter dir 化 + diff + meta + note 参数 |
| `scripts/diff_state.py` (新) | 算两 state.json 的 diff |
| `docs/STARTING.md` | 更新新 key bindings (F3/F5/C) + iter 目录结构 |

---

## 7. 验证标准 (每 sprint 收尾)

- `loop.ps1` 跑 3 轮, 0 build 失败, 3 个 `iter_NN/` 目录齐全
- `diff.json` 有有效字段 (wood/food/monsters/anomalies/invariant_violations)
- 截图可读: 天空非黑 + HUD 含 tick/坐标 + 玩家/地形可分辨
- `cargo test --workspace` 全过 (50 unit + 任何新加的)

---

## 8. Next Step

**等用户确认 Sprint 1-2 的范围 + 关键决策，然后**:
- 派 `.harness/reins/developer` 改 src/ (P0 数据泄漏 + 视觉修)
- 派 `.harness/reins/iter-tester` 跑 loop.ps1 + 读 diff
- 派 `.harness/reins/code-reviewer` 审 src 改动

我直接做的 vs 派 team:
- **直接做**: loop.ps1 + scripts/diff_state.py (PowerShell 脚本，不进 src)
- **派 developer**: src/main.rs / src/scenario/mod.rs / src/render/mod.rs 的 Rust 改动
- **派 iter-tester**: 跑闭环、读 diff、写 observations
- **派 code-reviewer**: 审 Rust diff 的 bevy 0.18 idiom

---

## 9. 备注: 这次为什么没直接动代码

按 plan-mode 规矩: 多文件、多方案、影响后续所有 iter 的改动，先把路径定下来再下手。
特别因为: `record_default.jsonl` 怎么修有 3 个方案，iter 目录化 vs 平铺是 2 个方案,
这些选择一旦做下去就影响后续所有 iteration 的目录结构，所以先让你看一眼。
