"""Quick debug: import kenney_bear and print AABB."""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))

from rotate_kenney import import_glb, world_aabb, decide_rotation

import bpy

glb = Path(r"F:\rustProject\lastkingdom2\assets\kenney\curated\animals\kenney_bear.glb")
n = import_glb(glb)
print(f"meshes: {n}")
print(f"scene objects:")
for obj in bpy.context.scene.objects:
    print(f"  {obj.name}  type={obj.type}")
dims, mn = world_aabb()
print(f"aabb dims: {dims}, min: {mn}")
print(f"decide_rotation('kenney_bear', {dims}) -> {decide_rotation('kenney_bear', dims)}")