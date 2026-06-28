
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

cube("test_cube", (10.0, 0.0, 0.0), (0.3, 0.7, 0.5), m)

import os
out = r"F:\rustProject\lastkingdom2\assets\procedural\pretty\test_cube.glb"
bpy.ops.object.select_all(action="SELECT")
bpy.ops.export_scene.gltf(
    filepath=out,
    export_format="GLB",
    use_selection=True,
    export_apply=True,
    export_materials="EXPORT",
)
print(f"exported {out}")
