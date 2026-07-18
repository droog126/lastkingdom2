# AGENTS.md

此文件将代理工作路由到项目技能。将操作细节保留在匹配的 `SKILL.md` 中。

## 事实来源

- 使用当前代码和 `xtask` 输出来确定仓库当前的功能。
- 使用相关技能和 `docs/architecture/engineering-baseline.md` 来确定预期的工程行为。
- 当实现与预期行为不一致时，报告冲突，而不是自动重写文档以匹配可能的 bug。
- 仅更新直接受当前更改影响的活动文档和技能契约。将 `docs/archive/` 和导入的设计笔记视为历史资料，除非活动文档指向它们。

## 当前实现锚点

- 自然世界的统一入口是 `lk2_core::simulation::step_world`；适配器不得直接创建第二套生态规则。
- 服务器自然链路按 `authority -> replication/observation/persistence` 投影报告；`NatureServerProjectionPlugin` 是现有主服务器接线。
- `xtask` 的状态工件和机器断言是健康结论的事实来源；PNG 只能证明表现，不能覆盖模拟失败。
- 仅改 `AGENTS.md` 或技能契约时运行 `just audit-skills`；只有实际代码变化才选择 Cargo 验证。

## 模型与 model-preview 沉淀

- `model-preview` 的 GLB 必须由 `tools/` 下的确定性生成器产生；不要把一次性的 Blender 手工编辑或只提交导出的 GLB 当作修复。
- 建模时先区分表面角色：有机体使用足够的基础曲面密度、平滑法线和必要的低级细分；硬表面使用克制的多段 Bevel、Harden Normals 和 Weighted Normal。不要用一条全局的 Flat/Smooth 规则处理所有部件。
- 有机部件的穿插黑缝优先从连接几何解决：静态展示体可以合并/体素重拓扑；需要分件或运行时动画的资产保留部件边界，并扩大有意重叠，不能用 Boolean 或 Remesh 盲目覆盖动画结构。
- 任何视觉结论都必须同时满足目标相机截图、GLB 结构检查和模型预算检查；PNG 只能证明表现，不能替代生成器审计或机器校验。
- 模型修改的最小充分验证顺序是：生成器审计 -> 规范构建 -> `model_pipeline validate` -> 相关集合的 GLB/多边形检查 -> `model-preview-shot` 或等价的自动退出目标相机预览。
- 当重复出现棱角、黑缝、厚片部件或预算超限时，优先更新共享 `models_lib.py` 助手和资产表面角色，再修改单个生成器；不要复制一套新的生成脚本绕过唯一归属。

## 技能选择

- 选择最小的充分集合：一个主要技能，以及仅在需要时一个验证伴随技能。
- 使用 `$local-dev` 作为普通仓库工作的后备。当更具体的路由已经覆盖任务时，不要自动添加它。
- 将审计和审查请求视为只读，除非用户还要求实现。
- 在行动之前阅读每个选定的技能。

## 验证节奏

- 优先连续完成一批相关实现，再在有意义的里程碑运行一次最窄充分验证；不要在每个小修改后重复编译、测试或闭环。
- 可见、图形、Bevy 表现层、视觉调参或 presentation-only 改动本身不触发 `just loop`。
- 仅当用户明确要求、验收条件明确要求、复现/诊断必须依赖运行时工件，或最终答复要声明“实际渲染画面已验证”时，才运行闭环、截图、auto-demo 或 health 工件。
- 交付前是否运行 `cargo check` / 聚焦测试由匹配技能的验证决策决定；低风险连续迭代可以延后验证，但最终答复必须说明选择。
- 验证失败后先根据证据修代码；没有相关代码变化时不要反复运行同一验证命令。

## 技能路由

- 对于普通代码/文档/工具工作、验证选择、工作树卫生和 git 准备，当没有更具体的路由适用时，使用 `$local-dev`。
- 对于可以在实现前用聚焦测试指定的确定性行为，使用 `$tdd-iteration`：规则、状态机、解析、不变量、场景、AI 决策和回归。
- 对于 Bevy 0.19 客户端/服务器/运行时工作，使用 `$bevy-gameplay-dev`：ECS 系统、渲染、输入、HUD、相机、网络、模拟接线、场景、性能和日志记录。
- 对于运行时资产或 GPU 生命周期症状，如内存不足、无效纹理、重复生成的资产分配、渲染重建泄漏或生成的资产在实体清理后仍然存在，使用 `$bevy-resource-lifecycle`。
- 仅当任务明确要求运行或诊断观察-决策-行动循环、自动演示、运行时截图、健康工件或 `decision.md` 时，使用 `$closed-loop-ai-dev`。不要因为改动可见或表现层相关而自动添加它。
- 仅用于定量循环截图评分、先前/当前比较或 `decision.md` 的评分部分，使用 `$screenshot-scoring`；普通 PNG 检查不需要它。
- 对于可复现的 Blender/GLB 生成、资产清单、模型预览、多边形预算和连接生成的模型，使用 `$ai-modeling`。
- 对于由证据支持的、只读的端到端游戏规则、权威流、不可达系统或客户端/核心/服务器差异的审计，使用 `$game-logic-audit`。
- 对于文档分类、当前/参考/提案边界、过时事实、损坏链接、命令/依赖/工件契约指导以及 `audit-docs` 失败，使用 `$docs-governance`。
- 当用户明确要求将经验教训编码、更新技能路由、创建/更新技能或通过持久指令防止重复的代理失败时，使用 `$skill-sedimentation`。

## 组合

- 游戏玩法规则实现：`$tdd-iteration`；仅当 ECS 调度或运行时接线是更改的一部分时，添加 `$bevy-gameplay-dev`。
- 可见的 Bevy/运行时实现：`$bevy-gameplay-dev`。不要自动添加 `$closed-loop-ai-dev`；只有用户明确要求、验收条件明确要求，或需要声明已观察真实渲染画面时才添加。
- 资源生命周期 bug：`$bevy-resource-lifecycle`；仅当复现或验证必须依赖循环/运行时证据时，添加 `$closed-loop-ai-dev`。
- 循环评分：仅当需要数字分数或带评分的 `decision.md` 时，使用 `$closed-loop-ai-dev` 加上 `$screenshot-scoring`。
- 生成的资产：`$ai-modeling`；仅当用户/验收要求在游戏目标相机中验证，或最终结论要声明目标相机下已验证时，添加 `$closed-loop-ai-dev`。
- 逻辑审计：单独使用 `$game-logic-audit` 生成报告；在后续或明确组合的修复任务中使用实现路由。
- 文档更改：`$docs-governance`；仅当更改 `xtask` 文档审计行为时，添加 `$tdd-iteration`。
- 技能更改：`$skill-sedimentation` 加上系统 `$skill-creator` 技能。

## 仓库约束

- 框架：Bevy 0.19。视觉目标：小型、可读、多彩、玩具般的 Sokpop 风格表现。
- 将持久自动化放在 Rust `xtask` 中；将 Python 用于 Blender/资产和聚焦分析；将 `justfile` 保持为简短别名。
- 不要添加根目录 PowerShell 工作流脚本或 Python 循环编排。
- 对于公共互联网访问，当本地代理可访问时，使用 SOCKS5 `127.0.0.1:7890`。如果不可用，报告该事实，而不是通过死代理反复重试。

## 技能文件

- `.codex/skills/local-dev/SKILL.md`
- `.codex/skills/tdd-iteration/SKILL.md`
- `.codex/skills/bevy-gameplay-dev/SKILL.md`
- `.codex/skills/bevy-resource-lifecycle/SKILL.md`
- `.codex/skills/closed-loop-ai-dev/SKILL.md`
- `.codex/skills/screenshot-scoring/SKILL.md`
- `.codex/skills/ai-modeling/SKILL.md`
- `.codex/skills/game-logic-audit/SKILL.md`
- `.codex/skills/docs-governance/SKILL.md`
- `.codex/skills/skill-sedimentation/SKILL.md`
