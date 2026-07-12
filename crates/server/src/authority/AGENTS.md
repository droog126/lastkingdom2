# 服务器权威

拥有固定节拍驱动、经过验证的输入应用、世界生命周期和权威事件发布。

- 调用共享核心模拟步骤；不要重新实现规则。
- 在模拟前拒绝或规范化无效输入。
- 保持节拍进度确定性和可观察性。

## 当前锚点

- `NatureAuthority::advance` 拒绝不递增 tick，并在发布前检查 `NatureSnapshot::is_finite`。
- `NatureAuthorityPlugin` 在 `FixedUpdate` 中推进并写入 `LatestNatureReport`；下游模块只能消费该报告。
