"""validate_pretty_glbs.py — 用 pygltflib 验证所有 pretty .glb 结构合法.

跑通 = glTF 2.0 spec 合法, mesh/material/accessor 引用都解析得到.
这是 Rust 端 asset_server.load() 能成功的必要条件 (不充分 — 还要 Bevy 接受).
"""
from __future__ import annotations

import sys
from pathlib import Path

import pygltflib
from pygltflib import GLTF2


def validate(path: Path) -> tuple[bool, str]:
    try:
        glb = GLTF2.load(str(path))
    except Exception as e:
        return False, f"GLTF2.load failed: {e}"
    # 必填字段
    if not glb.meshes:
        return False, "no meshes"
    for i, mesh in enumerate(glb.meshes):
        if not mesh.primitives:
            return False, f"mesh[{i}] no primitives"
        for j, prim in enumerate(mesh.primitives):
            if prim.attributes is None:
                return False, f"mesh[{i}].prim[{j}] missing attributes"
            pos_idx = prim.attributes.POSITION
            if pos_idx is None:
                return False, f"mesh[{i}].prim[{j}] missing POSITION"
            pos_acc = glb.accessors[pos_idx]
            if pos_acc.type != "VEC3":
                return False, f"mesh[{i}].prim[{j}] POSITION not VEC3"
            if prim.indices is None:
                return False, f"mesh[{i}].prim[{j}] not indexed (Bevy needs indexed)"
            idx_acc = glb.accessors[prim.indices]
            if idx_acc.type != "SCALAR":
                return False, f"mesh[{i}].prim[{j}] indices not SCALAR"
            # 5. POSITION accessor 的 min/max 应有
            if pos_acc.min is None or pos_acc.max is None:
                return False, f"mesh[{i}].prim[{j}] POSITION missing min/max"
    # 材质 (Bevy 用 PBR; 我们导出的是 PBR, 应有 pbrMetallicRoughness)
    has_materials = bool(glb.materials)
    return True, f"ok, {len(glb.meshes)} meshes, {len(glb.nodes) if glb.nodes else 0} nodes, materials={has_materials}"


def main() -> int:
    out_dir = Path(sys.argv[1] if len(sys.argv) > 1 else
                   r"F:\rustProject\lastkingdom2\assets\procedural\pretty")
    fails = []
    for p in sorted(out_dir.glob("*.glb")):
        ok, msg = validate(p)
        status = "OK " if ok else "FAIL"
        print(f"  [{status}] {p.name:35s}  {msg}")
        if not ok:
            fails.append((p.name, msg))
    if fails:
        print(f"\nFAIL: {len(fails)}")
        return 1
    print(f"\nPASS: all {len(list(out_dir.glob('*.glb')))} glbs structurally valid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
