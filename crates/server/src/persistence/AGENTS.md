# 持久化

拥有保存/加载、快照、事件日志、模式版本和恢复。

- 保留恢复世界所需的确定性状态。
- 不要序列化客户端表现实体或 GPU 资产。
- 使用原子化、版本化的写入，并在权威恢复前验证加载的不变量。

## 当前锚点

- 当前实现是 `NatureSave<T>`、`NatureRegionSave<T>` 和 `TerrainSave` 的版本化 DTO，`CURRENT_SCHEMA_VERSION` 为 `1`；区域保存同时保留快照、资源池、LOD 和恢复 tick，区域文件使用数组记录避免结构体键无法编码为 JSON。
- 设置 `LK2_NATURE_REGION_SAVE_PATH` 后，服务器在世界初始化前读取并校验区域保存，区域状态变化时使用临时文件加备份原子写入；未设置时仍只保留内存快照。
- 设置 `LK2_TERRAIN_SAVE_PATH` 后，服务器启动会读取并校验地形编辑，revision 变化时自动写入；未设置时仍只保留内存快照。
