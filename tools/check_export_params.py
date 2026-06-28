
import bpy
import sys

old = sys.argv
sys.argv = ["blender", "--background", "--python-exit-code", "0"]
try:

    ops = bpy.ops.export_scene.gltf

    op = ops.get_rna_type()
    for prop in op.properties:

        if prop.identifier.startswith("rna"):
            continue
        print(f"  {prop.identifier}  type={prop.type}  default={prop.default!r}")
except Exception as e:
    print(f"err: {e}")
