# 服务器应用组装

拥有无头 Bevy 应用构建、插件注册、配置和启动/关闭接线。

- 将自然世界规则保留在核心中，将传输细节保留在网络中。
- 明确注册权威、持久化、复制和观察。
- 避免在单体 `main.rs` 中隐藏系统排序。

## 当前锚点

- `NatureServerPlugin` 注册 authority、replication、observation、persistence 四个插件。
- `NatureServerProjectionPlugin` 不启动独立自然权威，只消费 `LatestNatureReport`，用于兼容现有服务器主循环和聚焦测试。
