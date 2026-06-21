"""check_export_params.py — 列出 export_scene.gltf 所有参数."""
import bpy
import sys

# 强制让 help 不阻塞
old = sys.argv
sys.argv = ["blender", "--background", "--python-exit-code", "0"]
try:
    # 不真正 export, 但触发 operator 参数 introspection
    ops = bpy.ops.export_scene.gltf
    # 用 get_rna_type 列出参数
    op = ops.get_rna_type()
    for prop in op.properties:
        # 跳过 id 之类内部参数
        if prop.identifier.startswith("rna"):
            continue
        print(f"  {prop.identifier}  type={prop.type}  default={prop.default!r}")
except Exception as e:
    print(f"err: {e}")
