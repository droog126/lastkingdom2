"""Re-orient a Kenney GLB so it stands up correctly in Bevy (Y-up).

For models authored Z-up, rotate -90 X in Blender so that Z becomes Y.
For models authored Y-up that are actually correct, skip.

Strategy:
  - Look at world AABB AFTER import (this is what Bevy sees).
  - If longest dim is Z, rotate -90 X (Z -> Y) to stand the model up.
  - If longest dim is Y already, skip.
  - If longest dim is X, rotate +90 Z (X -> Y).
  - Apply rotation, re-center to origin, re-export.
"""
from __future__ import annotations

import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector


def parse_args() -> tuple[Path, bool]:
    argv = sys.argv
    if "--" not in argv:
        print("usage: blender --background --python rotate_kenney_v2.py -- --root <dir> [--dry-run]")
        sys.exit(2)
    argv = argv[argv.index("--") + 1 :]
    root = None
    dry = False
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == "--root":
            root = Path(argv[i + 1]).resolve()
            i += 2
            continue
        if a == "--dry-run":
            dry = True
        i += 1
    if not root:
        print("missing --root")
        sys.exit(2)
    return root, dry


def collect(root: Path) -> list[Path]:
    return sorted(p for p in root.rglob("*.glb") if p.is_file())


def reset_scene() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()


def collect_meshes() -> list:
    out = []
    stack = list(bpy.context.scene.objects)
    while stack:
        obj = stack.pop()
        if obj.type == "MESH":
            out.append(obj)
        stack.extend(list(obj.children))
    return out


def world_aabb() -> tuple[tuple[float, float, float], tuple[float, float, float]]:
    meshes = collect_meshes()
    if not meshes:
        return (0, 0, 0), (0, 0, 0)
    mn = [math.inf, math.inf, math.inf]
    mx = [-math.inf, -math.inf, -math.inf]
    for obj in meshes:
        mw = obj.matrix_world
        for corner in obj.bound_box:
            v = mw @ Vector(corner)
            for axis in range(3):
                if v[axis] < mn[axis]:
                    mn[axis] = v[axis]
                if v[axis] > mx[axis]:
                    mx[axis] = v[axis]
    return tuple(mx[i] - mn[i] for i in range(3)), tuple(mn)


def decide_action(stem: str, dims: tuple[float, float, float]) -> str:
    """Return one of: skip, rotate_-90_X, rotate_+90_Z."""
    w, h, d = dims
    # Find largest dim, with tie tolerance.
    items = [("W", w), ("H", h), ("D", d)]
    items.sort(key=lambda kv: kv[1], reverse=True)
    top_axis, top_val = items[0]
    second_val = items[1][1]
    if (top_val - second_val) / max(top_val, 1e-6) < 0.04:
        # Tie at the top. If H is among the top two, leave it alone.
        for ax, _ in items[:2]:
            if ax == "H":
                return "skip"
    # Boats should have longest dim horizontal (Z) — actually most Kenney
    # row boats are Y-up and look correct already; only flip if clearly
    # wrong.
    boat_stems = {"kenney_row_boat_small"}
    if stem in boat_stems:
        # For a boat lying on water: smallest dim should be Y (height).
        smallest = min(w, h, d)
        if smallest == h:
            return "skip"
        if smallest == w:
            return "rotate_+90_Z"
        return "rotate_-90_X"
    if top_axis == "H":
        return "skip"
    if top_axis == "D":
        return "rotate_-90_X"
    return "rotate_+90_Z"


def apply_rotation(mode: str) -> None:
    bpy.ops.object.select_all(action="DESELECT")
    for obj in collect_meshes():
        obj.select_set(True)
    if not bpy.context.selected_objects:
        return
    bpy.context.view_layer.objects.active = bpy.context.selected_objects[0]
    if mode == "rotate_-90_X":
        bpy.ops.transform.rotate(value=math.radians(-90), orient_axis="X")
    elif mode == "rotate_+90_Z":
        bpy.ops.transform.rotate(value=math.radians(90), orient_axis="Z")
    else:
        return
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)


def re_center() -> None:
    dims, mn = world_aabb()
    if abs(mn[0]) < 1e-5 and abs(mn[1]) < 1e-5 and abs(mn[2]) < 1e-5:
        return
    bpy.ops.object.select_all(action="DESELECT")
    for obj in collect_meshes():
        obj.select_set(True)
    if not bpy.context.selected_objects:
        return
    bpy.context.view_layer.objects.active = bpy.context.selected_objects[0]
    bpy.ops.transform.translate(value=(-mn[0], -mn[1], -mn[2]))


def export_glb(glb: Path) -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=str(glb),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_materials="EXPORT",
    )


def process(glb: Path, dry: bool) -> str:
    reset_scene()
    bpy.ops.import_scene.gltf(filepath=str(glb))
    if not collect_meshes():
        return f"{glb.stem:32s} NO_MESH"
    dims, mn = world_aabb()
    mode = decide_action(glb.stem, dims)
    w, h, d = dims
    if mode == "skip":
        return (
            f"{glb.stem:32s} WxHxD={w:4.2f}/{h:4.2f}/{d:4.2f}  "
            f"up_world=Y  action=skip"
        )
    if dry:
        return (
            f"{glb.stem:32s} WxHxD={w:4.2f}/{h:4.2f}/{d:4.2f}  "
            f"up_world={'Z' if d >= w and d >= h else ('X' if w >= d and w >= h else 'Y')}  "
            f"action={mode} (DRY)"
        )
    apply_rotation(mode)
    re_center()
    export_glb(glb)
    dims2, _ = world_aabb()
    w2, h2, d2 = dims2
    return (
        f"{glb.stem:32s} WxHxD={w:4.2f}/{h:4.2f}/{d:4.2f} -> "
        f"{w2:4.2f}/{h2:4.2f}/{d2:4.2f}  action={mode}"
    )


def main() -> None:
    root, dry = parse_args()
    files = collect(root)
    print(f"processing {len(files)} GLB(s) under {root}  dry_run={dry}")
    for f in files:
        try:
            print(process(f, dry))
        except Exception as e:
            print(f"{f.stem:32s} ERROR: {e}")


if __name__ == "__main__":
    main()