"""Inspect a specific Kenney GLB to understand its actual up axis.

For kenney_pirate_flag, find the highest Y of any vertex in WORLD space.
"""
import sys
from pathlib import Path

import bpy
from mathutils import Vector

glb = Path(sys.argv[-1])
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.object.select_all(action="SELECT")
bpy.ops.object.delete()
bpy.ops.import_scene.gltf(filepath=str(glb))

# Walk every mesh, accumulate world-space AABB.
mn = [1e9, 1e9, 1e9]
mx = [-1e9, -1e9, -1e9]
def walk(obj):
    if obj.type == "MESH":
        mw = obj.matrix_world
        for corner in obj.bound_box:
            v = mw @ Vector(corner)
            for axis in range(3):
                if v[axis] < mn[axis]:
                    mn[axis] = v[axis]
                if v[axis] > mx[axis]:
                    mx[axis] = v[axis]
    for child in obj.children:
        walk(child)

walk(bpy.context.scene.objects[0]) if bpy.context.scene.objects else None
# Walk all roots
for obj in list(bpy.context.scene.objects):
    walk(obj)

print(f"world AABB min: {mn}")
print(f"world AABB max: {mx}")
print(f"world dims: ({mx[0]-mn[0]:.3f}, {mx[1]-mn[1]:.3f}, {mx[2]-mn[2]:.3f})")

# Check the parent of any mesh — is there a Y-up correction rotation?
print("\nObject hierarchy and transforms:")
def dump(obj, indent=0):
    t = obj.matrix_local.translation
    r = obj.matrix_local.to_euler()
    s = obj.matrix_local.to_scale()
    print(f"{'  ' * indent}{obj.name} type={obj.type} loc=({t[0]:.2f},{t[1]:.2f},{t[2]:.2f}) rot=({r[0]:.2f},{r[1]:.2f},{r[2]:.2f}) scale=({s[0]:.2f},{s[1]:.2f},{s[2]:.2f})")
    for c in obj.children:
        dump(c, indent + 1)
for obj in list(bpy.context.scene.objects):
    dump(obj)