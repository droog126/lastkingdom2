# TDD 建设方案

这份文档定义本项目的测试驱动开发入口、测试分层和 backlog。目标是让每个 bug 修复和规则增量都能先落成可重复的测试，再进入闭环截图验证。

## 快速入口

本地默认使用根目录脚本：

```sh
just test-core
```

常用范围：

```sh
just test-core
just test-changed
just test-client
just test-server
just test
just fmt
just audit-tdd
```

默认 `core`，因为 `crates/core` 是规则、状态机、资源和协议的主战场，反馈最快。

`changed` 默认对比 `HEAD`，需要对比其他基线时：

```sh
cargo run -q -p xtask -- tdd --scope changed --base-ref master
```

## 红绿流程

每个非纯视觉任务都按这个顺序：

1. 写一个失败测试，命名直接描述规则或 bug。
2. 跑 `just test-core` 或更小的 `cargo test -p lk2-core <test_name>`。
3. 确认失败原因是预期问题，而不是测试写错。
4. 写最小实现。
5. 重跑同一个测试。
6. 补边界测试。
7. 跑对应 scope。
8. 影响视觉/玩法时再跑 `just loop`。

不要先大改再补测试。后补测试只能作为债务补救，不算完整 TDD。

## 测试分层

| 层级 | 位置 | 目标 | 命令 |
| --- | --- | --- | --- |
| 单元规则 | `crates/core/src/**` | 资源、战斗、国家、保护期、AI 决策 | `just test-core` |
| crate 集成 | `crates/client` / `crates/server` | 编译接口、Bevy system 接线 | `just test-client` / `just test-server` |
| workspace | 全仓库 | 发布前兜底 | `just test` |
| 闭环 | `xtask loop` + `screenshots` | 截图、HUD、自动 demo、状态输出 | `just loop` |

## 测试审计

定期运行：

```sh
just audit-tdd
```

它会扫描 `crates/core/src`：

- 每个 `.rs` 文件有多少 `#[test]`
- 哪些模块没有直接单元测试
- 当前 core 测试总数

审计不是覆盖率报告，但能快速发现“完全没测试的模块”。新增核心模块时，至少要让 audit 不出现新的长期空白模块。

## 必须补测试的规则

- 资源增减、掉落、转移、上限、守恒
- 战斗伤害、格挡、招架、硬直、击退、无敌帧
- 保护期同时检查 attacker 和 target
- 阶段时间边界和 `MatchClock` 刷新前后的读数
- 怪物/动物死亡、掉落、计数、回流
- 国家创建、解散、旗帜上限、人口上限
- CLI、端口、协议 ID、连接参数解析
- scenario 推进必须完成或给出失败原因
- tick observer 的异常检测
- 曾经修过的 bug

## 不适合先写单元测试的内容

- 纯灯光参数
- 相机角度微调
- 色彩/材质审美调整
- 截图构图

这些任务仍要测试背后的数据和状态，例如玩家是否在 bounds 内、HUD 数据是否可读取、截图/state 文件是否生成。

## 高优先级 Backlog

### P0：先防止核心规则回退

- [x] `transport`：端口解析不依赖并发测试里的环境变量改写。
- [x] `match_state`：`wall_secs` 改变但 `phase` 未刷新时，剩余时间仍按真实 wall time 计算。
- [x] `protection`：中途加入保护期玩家不能攻击，也不能被攻击。
- [x] `creature`：动物死亡和手动击杀使用同一套掉落映射。
- [x] `resource`：动物掉落、矿坑产出、scenario 采集在资源上限失败时必须返回或记录原因，不能静默吞掉。
- [ ] `resource`：继续审计 demo/self-check/spark 回流等非主玩法 `try_add` 路径，明确哪些允许跳过、哪些必须返回错误。
- [ ] `combat`：攻击 active 窗口内同一目标不能被重复多次结算，除非设计明确允许。
- [ ] `combat`：招架成功后 attacker stun 必须阻止下一次 `try_start`。
- [ ] `world`：出生点必须有可站立空气格，下方必须 solid。
- [ ] `ai`：连续重复决策超过阈值必须被 observer 标记。
- [ ] `scenario`：每个 scenario 都必须终止为 success 或明确 fail，不允许无限 pending。

### P1：玩法闭环

- [ ] 动物被 PvP/CombatHealth 杀死时，资源掉落、击杀计数、entity despawn 一致。
- [ ] 手动 K 键击杀和 combat 击杀的掉落、计数一致。
- [ ] 怪物死亡释放生态计数，不破坏 `verify_count`。
- [ ] 建国消耗和失败原因覆盖 soul 不足、重复建国、旗帜上限。
- [ ] 采集空气返回 none，不改变资源池。
- [ ] 采矿 slot 占用、完成、清空都可重复测试。
- [ ] 保护期过期 system 移除组件后，攻击谓词恢复放行。
- [ ] match phase 边界 299.9/300/1079.9/1080/2099.9/2100 覆盖所有规则。

### P2：闭环和可观测性

- [ ] `xtask loop` 每次生成 `decision.template.md` 的字段和闭环协议保持一致。
- [ ] `final_state.json` 至少包含 tick、player、resource、monster/creature 关键字段。
- [ ] `diff.json` 对关键资源 delta 做稳定排序，便于 AI 对比。
- [ ] 自动 demo 运行 N 秒后玩家位置发生可解释变化。
- [ ] 玩家位置永远不越界；越界必须写入 anomaly。
- [ ] HUD 数据源缺失时显示 fallback，不 panic。

## 测试命名规范

测试名写清楚行为：

```rust
#[test]
fn midjoin_protected_player_cannot_attack_or_be_targeted() {}

#[test]
fn phase_remaining_uses_wall_secs_before_phase_refresh() {}

#[test]
fn chicken_drops_apple_and_livestock_drops_food() {}
```

避免：

```rust
#[test]
fn test1() {}

#[test]
fn works() {}
```

## 新 bug 处理模板

记录到本文件 backlog 时使用：

```markdown
- [ ] `module`: 一句话描述 bug。
  - repro: 如何复现
  - expected: 期望行为
  - test: 计划新增的测试名
```

## 完成定义

一个 TDD 任务完成必须有：

- 新增或修正测试
- 测试先红后绿，或说明这是补历史缺口
- `just test-core` 或对应 scope 通过
- 影响视觉/玩法时跑 `just loop`
- 总结剩余风险
