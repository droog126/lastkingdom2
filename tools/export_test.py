"""export_test.py — 验证 export_apply 是否真的把 scale 写进 vertex."""
import bpy
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import models_lib
models_lib.OUT_DIR = HERE.parent / "assets" / "procedural" / "pretty"

from models_lib import clear_scene, cube, mat

clear_scene()
m = mat("test_red", (1.0, 0.0, 0.0))
# 单位 cube, scale (0.3, 0.7, 0.5) → 期望 POSITION 在 (-0.15, -0.35, -0.25) ~ (0.15, 0.35, 0.25)
cube("test_cube", (10.0, 0.0, 0.0), (0.3, 0.7, 0.5), m)
# export
import os
out = r"F:\rustProject\lastkingdom2\assets\procedural\pretty\test_cube.glb"
bpy.ops.object.select_all(action="SELECT")
bpy.ops.export_scene.gltf(
    filepath=out,
    export_format="GLB",
    use_selection=True,
    export_apply=True,  # <-- 这个参数
    export_materials="EXPORT",
)
print(f"exported {out}")
