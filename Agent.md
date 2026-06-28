# Agent.md

本文件是 `lastkingdom2` 项目的 AI 开发协议。目标不是“多改代码”，而是持续交付可验证、可回滚、质量稳定的游戏增量。

核心原则：任务聚焦、质量优先、TDD 驱动、闭环验证。

---

## 1. 工作模式

每次接手任务都按这个顺序执行：

1. 明确任务边界
2. 读取相关代码和现有测试
3. 先写或补测试
4. 实现最小改动
5. 跑验证命令
6. 必要时跑闭环截图
7. 总结结果、风险和下一步

不要在没有定位问题的情况下大面积重构。不要为了“顺手优化”扩大改动范围。

---

## 2. 任务聚焦

开始前先回答三个问题：

- 本轮要解决的具体问题是什么？
- 哪些文件/模块最可能相关？
- 什么结果算完成？

优先级固定：

1. 编译失败、测试失败、panic、死循环、数据越界
2. 规则错误、状态不同步、资源守恒破坏、网络/客户端预测不一致
3. 玩家可见问题：看不见、卡住、HUD 错、操作无反馈
4. 性能问题：体素过多、系统每 tick 重复重活、日志刷屏
5. 视觉和体验增强

一次任务只改 1 到 3 个强相关问题。发现更多问题时记录下来，不要混进当前改动。

---

## 3. TDD 规则

所有纯逻辑、规则、状态机、资源变更、协议解析都必须优先走 TDD。

推荐节奏：

1. 写一个会失败的测试，复现 bug 或锁定新规则
2. 确认测试失败原因正确
3. 写最小实现让测试变绿
4. 补边界测试
5. 跑相关 crate 测试

测试优先放在 `crates/core`，因为反馈最快、最稳定。

必须有测试的场景：

- 资源增减、掉落、转移、守恒
- 国家、怪物、动物、战斗、保护期、阶段时间
- CLI/协议/网络参数解析
- AI 决策、scenario 推进、tick observer 异常检测
- 曾经出过 bug 的边界条件

可以暂不先写测试的场景：

- 纯视觉参数微调
- 临时 debug 日志
- 截图构图、灯光、相机角度

即使是视觉任务，也要优先为背后的纯函数、状态或数据选择逻辑补测试。

---

## 4. 代码质量标准

改动必须满足：

- 小而明确，能用一句话解释
- 不破坏已有用户改动
- 不绕开已有抽象
- 不引入重复规则表
- 不依赖本机环境变量、当前时间、随机数顺序来通过测试
- 每 tick 日志必须节流
- 错误要有清楚原因，不能静默失败

Bevy 0.18.1 约束：

- 使用 `Mesh3d` / `MeshMaterial3d`
- 不使用废弃的 `PbrBundle` / `MaterialMeshBundle`
- 游戏世界资源不要和 Bevy `World` 混名，必要时用 `World as GameWorld`
- 共享同类方块的 `Handle<Mesh>` / `Handle<StandardMaterial>`

Rust 约束：

- 遵守 `rustfmt.toml`
- 不随便 `unwrap()`，测试代码除外
- 不把环境变量读写放进并发单元测试
- 不用大范围 `allow` 掩盖新 warning

---

## 5. 建模与美术资产

需要生成或修改 3D 模型时，必须使用 Blender 的 Python 脚本流程，不要手工在编辑器里改完再让仓库状态不可复现。

本机 Blender 启动器固定为：

```powershell
F:\BLENDER\blender-launcher.exe
```

推荐命令格式：

```powershell
& "F:\BLENDER\blender-launcher.exe" --background --python tools\build_all_models.py
& "F:\BLENDER\blender-launcher.exe" --background --python tools\create_eco_models.py
```

建模规则：

- 新模型优先放在 `assets/procedural/pretty/` 或 `assets/procedural/eco/`
- 生成脚本放在 `tools/`，让模型可以从脚本重新生成
- 修改模型时同步更新对应 `MANIFEST.json`
- 生成后运行资产验证脚本：

```powershell
python tools\validate_pretty_glbs.py
python tools\verify_poly_budget.py
```

不要提交 `__pycache__/`、临时导出文件、Blender 自动备份文件或本机绝对路径配置。需要在代码里引用模型时，使用 Bevy asset server 的仓库相对路径。

---

## 6. 验证命令

按改动范围选择最小验证集。

统一入口优先用：

```powershell
.\tdd.ps1 -Scope core
.\tdd.ps1 -Scope changed
.\tdd.ps1 -Scope workspace
.\tdd.ps1 -Scope audit
```

TDD backlog 和测试分层见 `docs/notes/tdd.md`。

纯 core 改动：

```powershell
.\tdd.ps1 -Scope core
```

客户端改动：

```powershell
.\tdd.ps1 -Scope client
```

服务端改动：

```powershell
.\tdd.ps1 -Scope server
```

跨 crate 或公共接口改动：

```powershell
.\tdd.ps1 -Scope workspace
```

格式化：

```powershell
.\tdd.ps1 -Scope fmt
```

如果全仓库已有未格式化文件，不要顺手格式化整个仓库；只格式化本轮触碰文件，并在总结里说明 `cargo fmt --check` 的剩余问题。

---

## 7. 闭环运行

视觉、玩法、客户端体验、自动 demo 相关任务必须跑闭环。

```powershell
$env:BEVY_DISABLE_ACCESSIBILITY="1"
$env:RUST_LOG="info"
.\loop.ps1
```

闭环产物：

- `screenshots/iter_NN/iter_NN.png`
- `screenshots/iter_NN/final_state.json`
- `screenshots/iter_NN/diff.json`
- `screenshots/iter_NN/decision.md`
- `screenshots/iter_NN/health.json` —— AI 必须先读这个

观察顺序：

1. **先读 `health.json`**（~330 B 一行 JSON verdict：PNG luma + sim tick）—— 通常足以判断 PASS/PARTIAL/FAIL
2. 只有 `health.json` 报 PARTIAL/FAIL 时，才去读 `final_state.json` + `diff.json`
3. 只有 `health.json` 报 FAIL 且 reason 涉及视觉时，才去读 PNG（节省 token）
4. 写 `decision.md`

`health.json` 体积比 PNG 小 ~2000 倍。AI 不应每轮都解码 600KB 的 PNG —— **先读 health.json，按需深挖**。

没有 `decision.md` 的闭环等于没有复盘。

---

## 8. decision.md 模板

每轮闭环结束后写：

```markdown
# iter_NN decision

task: 本轮目标
result: pass / partial / fail

score:
- sky: X/10
- player: X/10
- terrain: X/10
- decor: X/10
- hud: X/10
- gameplay: X/10
- total: X.X/10

vs_prev:
- visual: improved / same / worse，原因
- state: 引用 diff.json 的关键 delta

problems:
- 具体问题 1
- 具体问题 2
- 具体问题 3

tests:
- 已跑命令
- 结果

next:
- 下一轮最该做的一件事
```

评分不是装饰。连续 3 轮没有进步，必须停止当前方向，重新定位问题。

---

## 9. 闭环评分重点

视觉维度：

- Sky：不是黑屏/白屏，天空读得清楚
- Player：玩家可见，有朝向和高度感
- Terrain：体素地形可辨认，出生点可站立
- Decor：树、水、动物、怪物等装饰有层次
- HUD：可读，不遮挡关键画面

玩法维度：

- 自动 demo 有动作
- 资源变化能解释
- 怪物/动物/玩家状态能闭环
- 没有 out of bounds、体素过多、日志刷屏

---

## 10. 项目结构速查

- `crates/core/src/`：共享逻辑，TDD 首选位置
- `crates/client/src/main.rs`：Bevy client 入口、HUD、截图、offline demo
- `crates/client/src/render/`：渲染、相机、自动 demo 表现
- `crates/server/src/main.rs`：headless server、自检、权威模拟
- `scenarios/`：场景脚本
- `screenshots/`：闭环输出
- `loop.ps1`：构建、运行、截图、状态导出
- `scripts/loop/run_scenario.ps1`：scenario 验证

---

## 11. 常见坑

- `cargo build` 不带 package/feature 会拖慢 Bevy 开发链路
- dev 阶段 client/server build 必须带 `--features dev-dynamic-linking`
- release/CI 不要带 `dev-dynamic-linking`
- `LK2_PORT` 这类环境变量不要直接在并发测试里改
- `MatchClock.wall_secs` 变了不代表 `phase` 一定已经 refresh
- 保护期规则要同时考虑 attacker 和 target
- 资源掉落和手动击杀不要各写一套 match
- 视觉变好了但 `diff.json` 没变化，可能只是表现层假象
- 数据变了但截图没变化，可能是玩家/相机/实体不可见

---

## 12. 完成标准

一个任务完成必须同时满足：

- 有明确改动说明
- 有对应测试或说明为什么不适合测试
- 相关验证命令通过
- 如果影响视觉/玩法，闭环截图已检查
- 未解决问题被明确列出

不要只说“应该好了”。要说清楚：

- 改了什么
- 为什么这样改
- 怎么验证
- 还剩什么风险
