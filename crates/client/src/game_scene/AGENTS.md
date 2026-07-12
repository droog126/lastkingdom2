# 可玩场景

此目录拥有默认客户端的 living forest 纵向切片：离线自然推进、地形、玩家、相机、HUD、农场和截图。

- `run_game_scene` 的系统链是有意分阶段的：推进/接收状态 -> 对账实体 -> 输入与动画 -> 截图/退出；新增系统应放入正确阶段。
- `OfflineNature` 可以提供离线权威，但必须复用核心 `step_world`；`reconcile_nature_entities` 只把快照差异投影为可见实体。
- `setup` 只做启动时资源和实体构建；运行时不要反复创建等价的 Mesh、Material 或世界几何。
- `--auto-demo`/`--game-scene-shot` 的截图输出由 `LivingSceneState` 管理，自动截图必须在场景稳定后退出。
- 表现层动画可以改变位置、缩放和材质，但不能改变云、雨、植物、动物或农作物的权威语义。
