<!-- doc-status: current -->
# 当前渲染优化沉淀

本文记录当前代码已经落地的渲染优化，以及目前可以、不能据此得出的结论。事实来源是
`crates/client`、`assets/shaders` 和 `xtask`；本文不是 FPS 基准报告，也不把设计草案当成
已完成实现。

## 结论先说

这轮优化确实已经产生了可见的改善：地形从“规则网格的硬折线”转为更连续的高度场，水面、
水底和岸线有了独立的表现层；阴影、抗锯齿、贴图采样和材质明暗的默认配置更稳定；挖掘后
替换地形网格时也补上了延迟释放旧 Mesh 的生命周期管理。

更准确地说，这是一轮“画面质量 + 渲染稳定性 + 资源管理”的综合优化，不应直接表述为“FPS
已经提升了多少”。当前闭环状态会记录帧时间，但地形 Mesh 重建次数、重建耗时和地形实体
替换次数在离线/在线截图状态中仍写入占位值 `0`，因此还没有可信的地形性能数字。

## 已经做了什么

### 1. 把玩法地形采样和表现地形采样分开

玩法、碰撞和内容放置继续使用稳定的较粗网格；表现层单独建立更密的 `TerrainRenderGrid`：

- 每个玩法网格单元在渲染侧细分为 `2 × 2`，渲染步长为 `0.5`。
- 使用带边界采样的三次插值生成高度，并根据邻域高度计算平滑法线。
- 插值结果限制在当前四个中心采样的高度范围内，避免过冲造成地形尖刺。
- 岸线单元按海平面裁剪，避免陆地三角形伸进水里形成长尖角、穿插和闪烁。

这样做的重点不是盲目增加所有模拟数据的精度，而是让“看起来平滑”和“玩法碰撞稳定”各自
使用合适的采样策略。实现集中在 `crates/client/src/game_scene/state.rs` 与
`crates/client/src/game_scene/setup.rs`。

### 2. 把水体拆成可读的几层

当前表现不再把水当成一块盖在地形上的平面，而是拆成：

- 陆地表面：草地、沙地、山体、雪地四类共享对应的材质句柄。
- 水面：独立的透明网格和水面材质。
- 水下地面：独立、下沉的底面，避免透明水面与地形共面争夺深度。
- 洞穴/地下挖掘面：独立的表现网格。

水面着色器在 GPU 片段阶段计算波纹和岸线泡沫；水下时隐藏水面、切换较短的雾化范围和水色，
避免水面深度测试挡住鱼、水草和水底。水下鱼和水草还复用 Mesh/Material 句柄，减少重复资源。
相关实现位于 `crates/client/src/game_scene/setup.rs`、
`crates/client/src/game_scene/stylized_material.rs` 和
`assets/shaders/stylized_water.wgsl`。

### 3. 统一并调稳风格化材质

导入的 GLB 材质会在加载完成后转换到和程序化地形相同的 `ExtendedMaterial` 路径，同时保留
原始材质颜色和贴图。这样地形、树木、建筑和生态物件使用一致的阴影可读性规则，不会因为
进入 PBR 阴影就整块变黑。

地形着色器的默认 cel quantization 已关闭，风格主要由颜色、灯光和较弱的坡度/高度色调完成；
阴影采用连续的亮度抬升与混合，而不是把大面积片元硬压成同一个颜色。可读性较高的首领、地标
和传奇武器只在自己的材质上启用克制的边缘光，没有增加全屏轮廓后处理。

实现锚点是 `crates/client/src/game_scene/stylized_material.rs` 和
`assets/shaders/stylized_terrain.wgsl`。

### 4. 把动态细节尽量放进共享材质和着色器

草地使用共享的 `GrassWindMaterial`，时间只更新每个唯一材质句柄的 uniform，顶点偏移在
`assets/shaders/grass_wind.wgsl` 中计算；水面同样通过一个共享水面材质的时间 uniform 驱动
波纹。它减少了“每个草片/每个水面实体都单独创建动态资源”的压力，但不意味着所有动画都已
GPU 化：树木、角色和动物仍有 ECS/Transform 动画。

### 5. 调整相机、阴影和贴图采样的默认路径

当前游戏场景和在线场景共用线性贴图采样，并启用 8 倍各向异性过滤，改善斜视地形和 GLB 表面
的远距离清晰度。主相机配置包括：

- HDR、ACES 色调映射和大气/距离雾。
- `Msaa::Off` + TAA + temporal jitter，针对密集高度场的移动边缘和岸线减少闪烁。
- 深度预通道、运动向量预通道和中等质量 SSAO，改善接地感与时间抗锯齿所需的运动信息。
- 2048 级联阴影贴图、三段级联和 Temporal 阴影过滤。

在第一波现代光影接入中，主太阳光已启用 Bevy 0.19 的 Contact Shadows，主相机也启用对应的
短程深度光线步进；级联阴影过滤改为 Temporal，利用现有 TAA 历史稳定远处阴影边缘。接触阴影
只覆盖近距离细节，级联阴影仍负责大范围遮蔽，因此没有用它替代现有阴影图。

第二波在统一的 `spawn_asset` 入口为固定聚落篝火和玩家建造篝火挂载了 Bevy 0.19
`RectLight`。它提供小范围的暖色面光，`area_light_luts` 由客户端 Cargo 特性启用；Bevy 0.19
当前的矩形区域光不投射阴影，所以遮蔽仍由太阳光的级联阴影和 Contact Shadows 负责。灯光作为
篝火根实体的子节点生成，篝火清理时会一并清理。

第三波对 Bevy 0.19 的 deferred opaque renderer 和 physically based SSR 做了兼容性验证，但暂不
接入主离线场景。`area_light_luts` 与 Contact Shadows/deferred lighting 在当前 Bevy 0.19
运行时组合中出现视图 BindGroup 布局不一致，首次绘制会触发 GPU validation error；因此主场景
当前保持 forward 路径，优先保留已验证的 Contact Shadows 与 RectLight。已有地形材质仍保留
forward/deferred 两套片元入口，待后续升级或隔离渲染管线后再重新接入 SSR。透明水面继续使用
自己的 forward 水面着色器。

这些设置主要提升稳定性和可读性，同时也改变了 GPU 工作量，不能简单地统称为“更快”。
共享采样入口在 `crates/client/src/rendering.rs`，场景配置在
`crates/client/src/game_scene/setup.rs` 与 `crates/client/src/game_scene/mod.rs`。

### 6. 补上运行时 Mesh 的安全替换和释放

编辑地形时，系统现在先生成新 Mesh、替换实体上的句柄，再把旧句柄放入延迟回收队列；旧
资源等待 `TERRAIN_MESH_RETIRE_FRAMES` 个渲染帧后才从 `Assets<Mesh>` 移除。空的地下表面也
使用一个带单顶点的合法占位 Mesh，避免零大小 GPU buffer 在 Bevy 0.19 的提取/上传阶段触发
生命周期问题。

生成的表现 Mesh 使用 `RenderAssetUsages::RENDER_WORLD`，不再要求保留主世界侧数据。这个
改动的直接收益是减少运行时资源滞留和旧资源过早释放造成的 GPU/渲染错误；它是稳定性优化，
不是单独的画质功能。实现位于 `crates/client/src/game_scene/setup.rs` 和
`crates/client/src/game_scene/state.rs`。

## 成效应该怎样判断

目前能从代码和机器契约确认的成效如下：

| 目标 | 当前证据 | 结论边界 |
| --- | --- | --- |
| 地形边缘更连续、岸线少穿插 | 独立平滑渲染网格、中心范围裁剪、岸线裁剪 | 需要目标相机截图确认最终观感 |
| 远处斜面更清晰、移动边缘更稳定 | 各向异性过滤、TAA、temporal jitter、深度/运动预通道 | 需要实际运行时截图或录屏确认 |
| 水下不再被水面盖住 | 独立水底、潜水时隐藏水面、专用雾化配置 | 需要水下目标相机检查 |
| 角色和物件贴地阴影更完整 | 主太阳光 Contact Shadows + 主相机深度预通道 | 需要目标相机截图确认，实际成本取决于屏幕覆盖率 |
| 篝火附近有柔和暖色局部照明 | `RectLight` + `area_light_luts`，挂在篝火根实体下 | 需要篝火目标相机截图确认；区域光本身没有阴影 |
| 不透明材质获得屏幕空间反射 | 当前未接入主场景；deferred + `ScreenSpaceReflections` 因 BindGroup 布局冲突暂缓 | 需要单独隔离管线或升级 Bevy 后重新验证 |
| 编辑地形时资源不无限滞留 | 延迟退休队列、`Assets<Mesh>` 移除、合法空 Mesh | 可做代码/测试检查；GPU 长跑仍需运行时观察 |
| 帧时间和地形重建受控 | `xtask health` 有对应断言 | 当前截图写出的 Mesh 统计仍是占位 `0`，不能据此报告真实性能 |

## 现有验证入口

只检查文档和技能契约：

```text
just audit-docs
just audit-skills
```

检查表现层时可使用：

```text
just terrain-preview-shot
just game-scene-shot
```

需要声明“实际渲染画面已验证”或需要闭环工件时，才运行 `just loop` / `just health`，并先读
`screenshots/iter_NN/health.json`。闭环的健康阈值目前包括：超过 50 ms 的帧不超过 10 次、
平滑地形重建不超过 3 次、单次重建不超过 60 ms、地形实体替换不超过 6 次；这些是告警阈值，
不是已经测得的本轮成绩。

## 下一步：把“看起来有效”变成可比较数据

下一次做性能结论前，需要让离线和在线的截图状态真实填充以下字段，而不是固定写 `0`：

- `render.terrain.smooth_mesh_builds`
- `render.terrain.smooth_mesh_max_ms`
- `render.terrain.terrain_despawns`

同时保留 `render.frame.dt_over_50ms` 和 `render.frame.max_ms`，在相同场景、相同 GPU 后端、
相同运行时长下比较优化前后。PNG 负责证明画面表现，`health.json` 和 `assertions.json` 负责
证明机器可判定的运行时结论，两者不能互相替代。
