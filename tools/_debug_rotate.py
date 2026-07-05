"""Debug why rotation doesn't change dims."""
import sys
from pathlib import Path

import bpy
from mathutils import Vector

glb = Path(r"F:\rustProject\lastkingdom2\assets\kenney\curated\animals\kenney_bunny.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.object.select_all(action="SELECT")
bpy.ops.object.delete()
bpy.ops.import_scene.gltf(filepath=str(glb))


def aabb():
    mn = [1e9, 1e9, 1e9]
    mx = [-1e9, -1e9, -1e9]
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH":
            continue
        mw = obj.matrix_world
        for corner in obj.bound_box:
            w = mw @ Vector(corner)
            for i in range(3):
                if w[i] < mn[i]:
                    mn[i] = w[i]
                if w[i] > mx[i]:
                    mx[i] = w[i]
    return (mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]), (mn[0], mn[1], mn[2])


print(f"BEFORE rotation: dims={aabb()}")

bpy.ops.object.select_all(action="DESELECT")
for obj in bpy.context.scene.objects:
    if obj.type == "MESH":
        obj.select_set(True)
bpy.context.view_layer.objects.active = bpy.context.selected_objects[0]
import math
bpy.ops.transform.rotate(value=math.radians(-90), orient_axis="X")
print(f"After transform.rotate: dims={aabb()}")

bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
print(f"After transform_apply: dims={aabb()}")