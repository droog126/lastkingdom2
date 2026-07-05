"""Compare local vs world AABB to detect Y-up vs Z-up Kenney models.

Imports each GLB and prints both the local mesh AABB (in the model's own
coordinates) and the world AABB (after Blender's GLB-import transforms).
If the longest dim matches between local and world, the model is Y-up. If
they differ, the GLB was authored Z-up and Blender has applied a +90 X
rotation to align it with Y-up.
"""
from __future__ import annotations

import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector


def parse_args() -> Path:
    argv = sys.argv
    if "--" not in argv:
        print("usage: blender --background --python inspect_kenney_local.py -- --root <dir>")
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


def aabb_local(obj) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
    mn = [math.inf, math.inf, math.inf]
    mx = [-math.inf, -math.inf, -math.inf]
    for corner in obj.bound_box:
        v = Vector(corner)
        for axis in range(3):
            if v[axis] < mn[axis]:
                mn[axis] = v[axis]
            if v[axis] > mx[axis]:
                mx[axis] = v[axis]
    return (
        (mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]),
        (mn[0], mn[1], mn[2]),
    )


def aabb_world(obj) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
    mn = [math.inf, math.inf, math.inf]
    mx = [-math.inf, -math.inf, -math.inf]
    mw = obj.matrix_world
    for corner in obj.bound_box:
        v = mw @ Vector(corner)
        for axis in range(3):
            if v[axis] < mn[axis]:
                mn[axis] = v[axis]
            if v[axis] > mx[axis]:
                mx[axis] = v[axis]
    return (
        (mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]),
        (mn[0], mn[1], mn[2]),
    )


def axis_up(dims: tuple[float, float, float]) -> str:
    return "XYZ"[dims.index(max(dims))]


def collect_all_meshes() -> list:
    """Walk every descendant of every object so we don't miss nested meshes
    that some Kenney GLBs use as their root structure."""
    out = []
    stack = list(bpy.context.scene.objects)
    while stack:
        obj = stack.pop()
        if obj.type == "MESH":
            out.append(obj)
        stack.extend(list(obj.children))
    return out


def inspect(glb: Path) -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    bpy.ops.import_scene.gltf(filepath=str(glb))
    meshes = collect_all_meshes()
    if not meshes:
        print(f"{glb.stem:32s} NO_MESH")
        return
    # Authored coords (as the model was exported, ignoring any import-time
    # Y-up axis fix-up) — we use matrix_local.
    lmn = [math.inf, math.inf, math.inf]
    lmx = [-math.inf, -math.inf, -math.inf]
    # World coords after Blender's GLB import — matrix_world.
    wmn = [math.inf, math.inf, math.inf]
    wmx = [-math.inf, -math.inf, -math.inf]
    for obj in meshes:
        ml = obj.matrix_local
        mw = obj.matrix_world
        for corner in obj.bound_box:
            for axis in range(3):
                lv = ml[axis][0] * corner[0] + ml[axis][1] * corner[1] + ml[axis][2] * corner[2] + ml[axis][3]
                wv = mw[axis][0] * corner[0] + mw[axis][1] * corner[1] + mw[axis][2] * corner[2] + mw[axis][3]
                if lv < lmn[axis]:
                    lmn[axis] = lv
                if lv > lmx[axis]:
                    lmx[axis] = lv
                if wv < wmn[axis]:
                    wmn[axis] = wv
                if wv > wmx[axis]:
                    wmx[axis] = wv
    ldims = tuple(lmx[i] - lmn[i] for i in range(3))
    wdims = tuple(wmx[i] - wmn[i] for i in range(3))
    print(
        f"{glb.stem:32s}  authored WxHxD={ldims[0]:4.2f}/{ldims[1]:4.2f}/{ldims[2]:4.2f}"
        f"  world WxHxD={wdims[0]:4.2f}/{wdims[1]:4.2f}/{wdims[2]:4.2f}"
        f"  authored_up={axis_up(ldims)}  world_up={axis_up(wdims)}"
    )


def main() -> None:
    root = parse_args()
    files = collect(root)
    print(f"inspecting {len(files)} GLB(s) under {root}")
    for f in files:
        try:
            inspect(f)
        except Exception as e:
            print(f"{f.stem:32s} ERROR: {e}")


if __name__ == "__main__":
    main()