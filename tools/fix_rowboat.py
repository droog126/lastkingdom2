"""Targeted rotation for the small row boat only — leave all other Kenney
GLBs untouched (they are correctly oriented)."""
import math
from pathlib import Path

import bpy
from mathutils import Vector

glb = Path(r"F:\rustProject\lastkingdom2\assets\kenney\curated\coastal_and_pirate\kenney_row_boat_small.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.object.select_all(action="SELECT")
bpy.ops.object.delete()
bpy.ops.import_scene.gltf(filepath=str(glb))


def collect_meshes():
    out = []
    stack = list(bpy.context.scene.objects)
    while stack:
        obj = stack.pop()
        if obj.type == "MESH":
            out.append(obj)
        stack.extend(list(obj.children))
    return out


def aabb():
    mn = [1e9, 1e9, 1e9]
    mx = [-1e9, -1e9, -1e9]
    for obj in collect_meshes():
        mw = obj.matrix_world
        for corner in obj.bound_box:
            v = mw @ Vector(corner)
            for axis in range(3):
                if v[axis] < mn[axis]:
                    mn[axis] = v[axis]
                if v[axis] > mx[axis]:
                    mx[axis] = v[axis]
    return (mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]), (mn[0], mn[1], mn[2])


print(f"BEFORE: {aabb()}")

bpy.ops.object.select_all(action="DESELECT")
for obj in collect_meshes():
    obj.select_set(True)
bpy.context.view_layer.objects.active = bpy.context.selected_objects[0]
# -90 X swaps Y and Z. The row boat has hull along Y in the GLB; rotating
# -90 X puts the hull along Z and the deck faces up.
bpy.ops.transform.rotate(value=math.radians(-90), orient_axis="X")
bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)

# Re-center on Y=0 after rotation.
dims, mn = aabb()
bpy.ops.transform.translate(value=(-mn[0], -mn[1], -mn[2]))

print(f"AFTER: {aabb()}")

bpy.ops.object.select_all(action="SELECT")
bpy.ops.export_scene.gltf(
    filepath=str(glb),
    export_format="GLB",
    use_selection=True,
    export_apply=True,
    export_materials="EXPORT",
)
print(f"saved {glb}")