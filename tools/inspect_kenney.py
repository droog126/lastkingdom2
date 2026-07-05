"""Inspect Kenney GLB bounds and recommend rotation fix.

Usage:
    blender --background --python tools/inspect_kenney.py -- --root assets/kenney/curated

Reads every *.glb under --root, computes combined AABB per file, and prints
a one-line summary like:
    kenney_tent      WxHxD = 2.0 x 1.6 x 2.4  axis=H  status=ok
    kenney_block_*   WxHxD = 2.0 x 0.6 x 2.4  axis=H  status=ok
    kenney_bunny     WxHxD = 1.4 x 0.6 x 0.8  axis=W  status=needs_rotate_-90Z

A model needs rotation when its smallest axis is Y (i.e. it is currently
lying flat when it should be standing up).
"""
from __future__ import annotations

import math
import os
import sys
from pathlib import Path

import bpy
from mathutils import Vector


def parse_args() -> Path:
    argv = sys.argv
    if "--" not in argv:
        print("usage: blender --background --python inspect_kenney.py -- --root <dir>")
        sys.exit(2)
    argv = argv[argv.index("--") + 1 :]
    root_arg = None
    i = 0
    while i < len(argv):
        if argv[i] == "--root":
            root_arg = argv[i + 1]
            i += 2
            continue
        i += 1
    if not root_arg:
        print("missing --root")
        sys.exit(2)
    return Path(root_arg).resolve()


def collect(root: Path) -> list[Path]:
    return sorted(p for p in root.rglob("*.glb") if p.is_file())


def axis_for_dims(w: float, h: float, d: float) -> tuple[str, str]:
    """Return (longest_axis, status)."""
    dims = {"W": w, "H": h, "D": d}
    longest = max(dims, key=lambda k: dims[k])
    shortest = min(dims, key=lambda k: dims[k])
    if longest == "H":
        return longest, "ok"
    if shortest == "H":
        return shortest, f"needs_rotate_to_make_H_longest (currently longest={longest})"
    return longest, "ambiguous"


def inspect(glb: Path) -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    bpy.ops.import_scene.gltf(filepath=str(glb))

    min_v = [math.inf, math.inf, math.inf]
    max_v = [-math.inf, -math.inf, -math.inf]
    n_verts = 0
    n_objects = 0
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH":
            continue
        n_objects += 1
        # Compute world-space bounds so any GLB node transforms are folded in.
        for corner in obj.bound_box:
            world = obj.matrix_world @ Vector(corner)
            for axis in range(3):
                if world[axis] < min_v[axis]:
                    min_v[axis] = world[axis]
                if world[axis] > max_v[axis]:
                    max_v[axis] = world[axis]
        n_verts += len(obj.data.vertices)
    if n_objects == 0:
        print(f"{glb.stem:32s} NO_MESH")
        return
    w = max_v[0] - min_v[0]
    h = max_v[1] - min_v[1]
    d = max_v[2] - min_v[2]
    axis, status = axis_for_dims(w, h, d)
    rel = glb
    print(
        f"{glb.stem:32s} WxHxD = {w:5.2f} x {h:5.2f} x {d:5.2f}  "
        f"verts={n_verts:5d}  axis={axis}  status={status}  "
        f"min=({min_v[0]:.2f},{min_v[1]:.2f},{min_v[2]:.2f})"
    )


def main() -> None:
    root = parse_args()
    if not root.exists():
        print(f"missing: {root}")
        sys.exit(2)
    files = collect(root)
    print(f"inspecting {len(files)} GLB(s) under {root}")
    for f in files:
        try:
            inspect(f)
        except Exception as e:
            print(f"{f.stem:32s} ERROR: {e}")


if __name__ == "__main__":
    main()