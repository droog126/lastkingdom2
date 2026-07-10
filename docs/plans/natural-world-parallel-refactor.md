<!-- doc-status: proposal -->
# 自然世界整体重构：四 AI 并行执行计划

> 本文是待执行提案，不代表目录已经接入编译，也不代表旧线路已经删除。当前代码和工程基线优先。

## 1. 本轮目标

本轮只建立一条可继续演进的骨架，不尝试一次完成生态游戏：

```text
Scenario / WorldRecipe
        ↓
共享确定性自然模拟
        ↓
Server Authority 或 Offline Authority
        ↓
Snapshot + Events
        ↓
Client Presentation
        ↓
xtask Observation + Assertions
```

第一条需要打通的自然因果链是：

```text
云量和降雨
→ 土壤水分
→ 植物状态
→ 动物可用食物与行为
→ 状态、事件和闭环证据
```

本轮不承诺真实流体、完整食物网、大地图重写、体素重写、最终美术或大规模目录搬迁。

## 2. 协作规则

四个 AI 使用独立 branch/worktree，禁止在同一个工作目录同时写文件。

### 这是迁移式重构，不是绿地重写

现有代码是主要素材和行为基线。四个角色开始写代码前，必须先搜索并阅读自己领域的旧实现，然后建立一份迁移账本：

| 状态 | 含义 |
| --- | --- |
| `keep` | 实现和位置都保留 |
| `move` | 行为保持不变，只迁到新目录 |
| `adapt` | 保留实现，通过新接口接入唯一主线 |
| `merge` | 多个旧实现合并，先用测试锁定共同语义 |
| `retire` | 已确认不可达、重复或错误，集成验证后删除 |

强制原则：

- 能移动就不重写，能适配就不复制，能复用测试就不另造验收口径。
- 不得仅因为旧文件大、命名旧或目录不理想就重写正确逻辑。
- 任何 `retire` 都要给出调用点搜索、替代实现和验证证据。
- 临时复制只允许用于可回滚迁移，并必须在同一角色交付中注明旧入口何时删除。
- 新接口应包住现有行为，再逐步把调用方切过去；不要先写一套理想实现再等待旧代码消失。
- 发现旧行为与产品核心冲突时先记录冲突，不擅自把文档或新实现改成其中一方。

每个角色的交付说明必须包含：

```text
reused:
moved:
adapted:
retired:
still_legacy:
public_integration_needed:
```

统一规则：

- 开始前阅读根 `AGENTS.md` 和自己写入范围内的所有 `AGENTS.md`。
- 保留现有未提交修改，不清理、不回滚、不格式化无关文件。
- 不修改其他角色的写入范围。
- 各角色可以修改自己明确拥有的 crate 入口，但不得修改其他角色的入口。
- Cargo 文件、`justfile`、根 `AGENTS.md` 和 current 文档统一留给最终集成人。
- 不复制旧规则形成第二套长期实现；旧代码只能作为迁移参考或适配来源。
- 每个角色提交一个独立 commit，提交信息以角色编号开头。

推荐分支：

```text
codex/nature-core
codex/nature-server
codex/nature-client
codex/nature-loop
```

## 3. 执行时序

### Gate 0：接口冻结

1 号 AI 先完成最小共享契约并提交。2、3、4 号 AI 在该提交上 rebase 后并行实现适配器。

冻结的概念至少包括：

- `SimulationTick`
- `WorldInput`
- `TickReport`
- `NatureSnapshot`
- `NatureEvent`
- `step_world`

接口冻结后，除集成修复外不得随意改名或改变语义。

### Gate 1：四路并行

四个角色只在各自写入范围内工作，产出可单独审查的 commit。

### Gate 2：顺序合并

建议合并顺序：

1. 共享核心
2. 服务端权威
3. 客户端表现
4. 闭环与场景
5. 集成人补公共模块注册和冲突修复

## 4. 角色 1：共享自然模拟与集成契约

### 使命

建立唯一的确定性模拟入口，并给云、水、植物和动物提供最小共享状态。重点是接口和可测试因果链，不是复杂模拟。

### 独占写入范围

```text
crates/core/src/atmosphere/**
crates/core/src/hydrology/**
crates/core/src/ecology/**
crates/core/src/simulation/**
crates/core/src/world/generation/**
crates/core/tests/nature_*.rs
crates/core/src/lib.rs
crates/core/src/sim.rs
crates/core/src/eco_cycle.rs
crates/core/src/creature/**
crates/core/src/monster/**
crates/core/src/world/content/**
crates/core/src/world/terrain/**
```

这些旧目录是优先复用来源。先盘点现有生态、动物、世界内容、地形和模拟逻辑，再决定 `keep/move/adapt/merge/retire`。不得删除 `world` 主状态或大规模改写地形；本轮只允许为统一自然模拟入口做必要接线。

### 最小交付

- 可序列化、可比较的最小自然状态和快照。
- 一个纯确定性的 `step_world`。
- 最小顺序：大气 → 水分 → 植物 → 动物 → 不变量。
- 至少证明：降雨增加土壤水分，土壤水分影响植物状态，植物状态进入动物可用食物摘要。
- 相同初始状态和输入运行相同 tick 后结果一致的测试。
- 水量/生物量变化不能出现 NaN、负值或无解释增长。
- 一份覆盖 `sim.rs`、`eco_cycle.rs`、`creature`、`monster`、`world/content` 和 `world/terrain` 的迁移账本。
- 优先调用或提取已有规则；新增逻辑只填补现有代码确实没有的云—水因果最小缺口。

### 不做

- 不做 Bevy 渲染。
- 不做网络连接。
- 不做完整动物寻路、捕食或繁殖。
- 不把体素结构写进模拟契约。

### 建议验证

```text
cargo test -p lk2-core --test nature_simulation
cargo test -p lk2-core
```

### 可直接发送给 AI 的任务

> 你是 1 号 AI，负责共享自然模拟与接口冻结。这是迁移式重构，不是从零重写。阅读根 AGENTS.md、crates/core/src/AGENTS.md 及写入目录中的 AGENTS.md；先搜索并阅读现有 sim.rs、eco_cycle.rs、creature、monster、ecology、world/content 和 world/terrain，建立 keep/move/adapt/merge/retire 迁移账本。优先提取和适配已有规则，只为现有缺口新增最小代码。建立唯一 step_world、NatureSnapshot、NatureEvent 和云→水→植物→动物摘要链，并用现有及新增聚焦测试锁定行为。不要修改 Cargo 或其他 crate。完成后提交独立 commit并报告 reused/moved/adapted/retired/still_legacy。

## 5. 角色 2：服务端权威与复制边界

### 使命

把服务端整理成“持有并推进自然世界”的权威适配器，先建立模块边界和可测试组件，不立即重写现有网络流程。

### 独占写入范围

```text
crates/server/src/app/**
crates/server/src/authority/**
crates/server/src/networking/**
crates/server/src/replication/**
crates/server/src/persistence/**
crates/server/src/observation/**
crates/server/tests/nature_*.rs
crates/server/src/main.rs
crates/server/src/pvp_systems.rs
```

现有 `main.rs` 和 `pvp_systems.rs` 是迁移来源且只归本角色修改；不得修改 core 或 client。先提取插件和权威驱动，保持当前网络与 PvP 行为。

### 最小交付

- `authority` 中定义固定 tick 驱动器或适配器，调用 1 号提供的共享模拟入口。
- 明确输入验证、模拟推进、快照构建和事件发布的顺序。
- `replication` 定义自然快照和事件的复制边界，不实现第二套生态状态。
- `observation` 输出闭环需要的只读摘要：tick、云量/降雨、土壤水分、植物和动物摘要。
- `persistence` 只定义最小保存边界或 DTO，不承诺完整存档格式。
- 给集成人一份 `main.rs` 插件注册清单。
- 一份现有服务端启动、authority、网络和 PvP 路径迁移账本。

### 不做

- 不改当前在线协议行为。
- 不把渲染资源引入服务端。
- 不在服务端重写自然规则。
- 不直接改闭环 artifact schema。

### 建议验证

```text
cargo check -p lk2-server
cargo test -p lk2-server
```

如果模块尚未接入编译，至少保证文件内部结构完整，并明确集成后的验证命令。

### 可直接发送给 AI 的任务

> 你是 2 号 AI，负责服务端权威、复制和观测边界。这是迁移式重构，不是新写一套 server。基于 1 号接口冻结提交，先阅读 main.rs、pvp_systems.rs 和现有网络/authority 路径，建立 keep/move/adapt/merge/retire 账本。把现有行为提取到新目录并保持兼容，接入共享 step_world、只读自然观测摘要和复制边界。只修改 server 写入范围，不修改 core、client、Cargo。完成后提交独立 commit并报告 reused/moved/adapted/retired/still_legacy。

## 6. 角色 3：客户端自然世界表现

### 使命

建立统一的 snapshot-to-presentation 路径，让云、天气、植物和动物都来自权威自然状态，不再依赖客户端随机布景。

### 独占写入范围

```text
crates/client/src/app/**
crates/client/src/synchronization/**
crates/client/src/presentation/**
crates/client/tests/nature_*.rs
crates/client/src/main.rs
crates/client/src/render/**
crates/client/src/pretty/**
crates/client/src/ui.rs
```

现有 `main.rs`、`render`、`pretty` 和 `ui.rs` 是迁移来源且只归本角色修改；不得修改 core/server。先提取可复用的镜头、地形表现、模型加载、动物视觉、云和 HUD 行为，避免建立第三套表现线路。

### 最小交付

- 一个统一的 `NatureSnapshot` 接收/缓存边界。
- sky/weather/plant/animal 的 presentation 组件或映射函数骨架。
- 云量、降雨、植物状态、动物状态都必须由 snapshot/event 驱动。
- 允许使用占位几何或已有资产 handle，但不得随机生成语义实体。
- 给集成人一份从旧 `pretty`、`render` 迁移哪些调用的清单。
- 给出 offline 和 online 共用同一 presentation 输入的接线方案。
- 一份现有 `main/render/pretty/ui` 行为的迁移账本，明确哪些逻辑直接复用。

### 不做

- 不重新设计最终美术。
- 不删除旧渲染线路。
- 不让粒子或动画反向修改模拟。
- 不把客户端预测变成生态权威。

### 建议验证

```text
cargo check -p lk2-client
```

接入运行后再使用闭环验证；本角色不单独声称截图效果已经成立。

### 可直接发送给 AI 的任务

> 你是 3 号 AI，负责客户端自然世界表现。这是迁移式重构，不是重做客户端。基于 1 号接口冻结提交，先阅读 main.rs、render、pretty、ui 及现有 capture/preview，建立 keep/move/adapt/merge/retire 账本。优先提取现有相机、模型、云、植物、动物和材质逻辑到 app/synchronization/presentation，建立 NatureSnapshot 的唯一表现输入，禁止再建第三套渲染线路或随机创造语义实体。只修改 client 写入范围，不修改 core/server/Cargo。完成后提交独立 commit并报告 reused/moved/adapted/retired/still_legacy。

## 7. 角色 4：闭环、场景与验收

### 使命

为自然世界建立机器可判断的闭环证据，避免只看截图判断生态是否正确。

### 独占写入范围

```text
xtask/src/runner/**
xtask/src/observer/**
xtask/src/assertions/**
xtask/src/artifacts/**
xtask/src/main.rs
xtask/src/loop_cmd.rs
xtask/src/health.rs
scenarios/playable/**
scenarios/simulation/**
scenarios/regression/**
scenarios/visual/**
```

现有 `main.rs`、`loop_cmd.rs`、`health.rs` 是闭环事实来源且只归本角色修改；不得修改 current 文档或运行时 crate。扩展现有 artifact 合同，不另建平行闭环。

### 最小交付

- 一个最小生态盆地 simulation scenario 规格。
- observer 能读取或定义自然摘要：云、降雨、土壤水分、植物、动物、错误日志。
- assertions 至少覆盖：确定性摘要存在、数值有限、无负库存、因果链有进展、客户端没有幽灵语义实体。
- artifacts 定义向现有 `health.json` / `assertions.json` 扩展所需的数据结构建议，不直接破坏现有 schema。
- 给集成人一份接入 `loop_cmd.rs` 和 `health.rs` 的最小补丁清单。
- 一份现有 loop、health、scenario 和 artifact 路径迁移账本。

### 不做

- 不直接实现自然模拟。
- 不用截图推断水量或生物量守恒。
- 不改现有 artifact 名称或 current 闭环契约。
- 不启动下一轮闭环前绕过已有 `decision.md` 门禁。

### 建议验证

```text
cargo test -p xtask
just audit-docs
just audit-skills
```

### 可直接发送给 AI 的任务

> 你是 4 号 AI，负责自然世界闭环、场景和验收。这是迁移式重构，不是另写一套 loop。基于 1 号接口冻结提交，先阅读 xtask 的 main.rs、loop_cmd.rs、health.rs、现有 scenario loader 和 artifact 合同，建立 keep/move/adapt/merge/retire 账本。扩展现有 runner/observer/assertions/artifacts，建立生态盆地场景和机器断言，保持现有 health/decision 门禁。只修改 xtask/scenarios 写入范围，不修改运行时 crate、Cargo 或 current 文档。完成后提交独立 commit并报告 reused/moved/adapted/retired/still_legacy。

## 8. 集成人职责

单独的集成人在四个 commit 完成后负责跨 crate 公共文件，其他角色不得抢改：

```text
Cargo.toml 与各 crate Cargo.toml
justfile
docs/current 文档
```

集成步骤：

1. 合并 1 号并冻结共享类型。
2. 依次合并 2、3、4 号，解决路径和命名冲突。
3. 只做必要模块注册，不在集成 commit 中扩展玩法。
4. 运行格式化、包级检查、工作区测试和文档/技能审计。
5. 接通最小场景后再运行闭环，读取状态证据后检查截图。

## 9. 首轮完成定义

以下条件全部满足才算首轮完成：

- 只有一个共享自然模拟 step。
- server 和 offline 计划接入同一个 step，没有复制规则。
- client presentation 只消费自然 snapshot/event。
- 云、降雨、土壤水分、植物和动物摘要贯穿状态链。
- 至少一个纯模拟场景能验证云→水→植物→动物摘要的进展。
- 现有游戏仍能编译运行，旧线路尚未迁移部分被明确记录。
- 不宣称完整生态、最终视觉或旧代码删除已经完成。

## 10. 合并验证

```text
just fmt
cargo test -p lk2-core
cargo check -p lk2-server
cargo check -p lk2-client
cargo test -p xtask
just audit-architecture
just audit-docs
just audit-skills
```

只有在运行路径接通后才执行 `just loop`；编译通过不能替代运行和视觉证据。
