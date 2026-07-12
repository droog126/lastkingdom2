# 模型工具规则

这些说明适用于 `tools/` 下的每个 Python 文件。

## 目标风格

- 构建一个连贯的 Sokpop 风格玩具世界：紧凑的轮廓、粗壮的连接形态、平面或刻意的多面着色、克制的倒角、哑光材质，以及一个可读的强调色。
- 使用 Hoplite 资产作为轮廓、确定性细节、材质角色分离、对象命名、预览支持和可复现导出的质量参考。不要将其黑暗幻想调色板复制到不相关的资产上。
- 在游戏内相机比例下评估资产。通过 GLB 结构和三角形检查是必要的，但还不够。
- 每个道具优先使用 3 到 7 种材质角色：基础、次要、深色/支撑、浅色/边缘，以及最多两个强调色。保持自发光几何形状在可见表面的大约 10% 以下。
- 保持连接部件明显重叠。拒绝浮动把手、断开的肢体、隐藏的插槽、意外的交叉、通用展示底座，以及在模型预览中消失的细节。

## 几何契约

- 以米为单位工作，使用 Blender Z 轴向上。通过 `models_lib.export_glb` 导出 glTF Y 轴向上。
- 将可放置资产放在地平面上，并将其足迹居中在原点附近。武器可以使用刻意的握把/柄头原点。
- 对文件、对象、网格和材质使用 snake_case 名称。
- 优先使用 6 到 12 面的圆柱体/圆锥体、单段倒角，以及细分级 1 或 2 的 ico 球体。仅当轮廓明显需要时才使用更密集的几何形状。
- 将每个生产 GLB 保持在 `model_style.py` 中类别预算以下；仓库硬上限为 5000 个三角形。
- 使用 `random.Random(<稳定整数>)` 为每个随机细节设定种子。绝不使用全局随机状态或基于时间的种子。

## 生成器架构

- 在导出之前，将每个生产资产在 `model_catalog.py` 中注册到恰好一个生成器下。
- 使用 `models_lib.py` 进行场景清理、材质、基元和导出。在那里添加可重用的助手，而不是将 Blender 操作符复制到另一个生成器中。
- 不要从生成器调用 `bpy.ops.export_scene.gltf`。不要硬编码绝对路径或修改 `models_lib.OUT_DIR`。
- 不要从生成器写入 `MANIFEST.json`。在文件名稳定后运行 `python tools/sync_model_manifests.py`。
- 保持生成器确定性和导入安全。将执行放在 `main()` 和 `if __name__ == "__main__"` 之后。
- 已退役的生成器必须失败并提供替换命令。它们绝不能静默覆盖当前资产。
- 临时导出和预览必须使用 `LK2_MODEL_OUTPUT_ROOT` 或 `.tmp/model-previews/`，而不是故意构建之外的生产路径。

## 必需工作流

1. 在编辑之前运行 `python tools/audit_model_generators.py` 以识别所有权和遗留冲突。
2. 修改规范生成器或共享助手。不要为现有资产添加第二个生成器。
3. 通过 `python tools/model_pipeline.py build --asset <stem>` 或所属的 Blender 脚本构建。
4. 运行 `python tools/model_pipeline.py validate`。
5. 为更改的资产运行 `just model-preview-shot <stem>` 并检查 PNG。
6. 仅在输出集最终确定后运行 `python tools/sync_model_manifests.py`。

完成需要生成器审计、GLB 结构验证、多边形预算验证和渲染视觉检查。仅成功导出的脚本是不完整的。

## 当前锚点

- 生产模型的唯一归属由 `model_catalog.py` 决定，生成器不直接写 manifest；稳定输出后才同步 manifest。
- 游戏内生态模型路径由核心生态目录消费，改名时需同时检查生成输出、manifest 和目录引用。
