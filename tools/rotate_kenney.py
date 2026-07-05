"""Rotate Kenney GLB files so the longest axis becomes Y (height).

Reads each GLB under --root, computes the AABB of all mesh vertices in
world space, then rotates the GLB so the longest dim ends up on +Y and the
second-longest on +X. Re-exports in place.

Usage:
    blender --background --python tools/rotate_kenney.py -- --root assets/kenney/curated

The rotation is decided per file:
  - If H is already the longest: skip
  - If D is the longest: rotate -90 deg around X (Z becomes Y)
  - If W is the longest: rotate +90 deg around Z (X becomes Y)
  - If the GLB has no meshes: skip

If --dry-run is passed, only the proposed rotation is printed.
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
        print("usage: blender --background --python rotate_kenney.py -- --root <dir> [--dry-run]")
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


def import_glb(glb: Path) -> int:
    reset_scene()
    bpy.ops.import_scene.gltf(filepath=str(glb))
    return sum(1 for o in bpy.context.scene.objects if o.type == "MESH")


def world_aabb() -> tuple[tuple[float, float, float], tuple[float, float, float]]:
    mn = [math.inf, math.inf, math.inf]
    mx = [-math.inf, -math.inf, -math.inf]
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH":
            continue
        mw = obj.matrix_world
        for corner in obj.bound_box:
            world = mw @ Vector(corner)
            for axis in range(3):
                if world[axis] < mn[axis]:
                    mn[axis] = world[axis]
                if world[axis] > mx[axis]:
                    mx[axis] = world[axis]
    return (
        (mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]),
        (mn[0], mn[1], mn[2]),
    )


def decide_rotation(stem: str, dims: tuple[float, float, float]) -> tuple[str, str]:
    """Return (axis_letter, rotation_or_skip) describing the fix.

    Heuristic: rotate so the longest dim ends up on +Y. Per-stem overrides
    exist for boats (long axis must stay horizontal) and tiles/coins/hearts
    (short axis must be vertical = thickness).
    """
    w, h, d = dims
    # Treat two dims as "tied" if they are within 4% of the largest — many
    # Kenney models are roughly cubical (bear, players) and rotating them
    # either way doesn't matter visually, but skipping avoids floating-point
    # near-ties triggering unwanted rotations.
    largest = max(w, h, d)
    tied_with = lambda x: abs(x - largest) / max(largest, 1e-6) < 0.04
    # Boats: length must be horizontal. Treat W or D as length; keep whichever
    # is longest horizontal and rotate so H becomes the smallest.
    boat_stems = {
        "kenney_row_boat_small",
    }
    # Flat tiles / coins / hearts: H must be the smallest dim (thickness).
    flat_stems = {
        "kenney_block_grass_large",
        "kenney_block_snow_large",
        "kenney_dock_platform",
        "kenney_coin_gold",
        "kenney_heart",
        "kenney_resource_stone",
        "kenney_resource_wood",
        "kenney_campfire_pit",
        "kenney_workbench",
    }
    # Tents: always stand up — Kenney's tent frame is intentionally open so
    # the visible structure is what we want.
    standing_stems = {
        "kenney_tent",
        "kenney_pirate_flag",
        "kenney_palm_straight",
        "kenney_door_open",
    }
    # Helper: rotate so the axis named `from` ends up on Y.
    def rotate_to_h(from_axis: str) -> tuple[str, str]:
        if from_axis == "H":
            return ("H", "skip")
        if from_axis == "D":
            return ("D", "rotate_-90_X")
        return ("W", "rotate_+90_Z")

    # Helper: pick the "longest" axis with a tolerance for ties. Returns
    # the axis letter and the second-largest dim (for tie checks).
    def longest_axis_with_tie() -> tuple[str, float]:
        items = [("W", w), ("H", h), ("D", d)]
        items.sort(key=lambda kv: kv[1], reverse=True)
        top_axis, top_val = items[0]
        second_val = items[1][1]
        if (top_val - second_val) / max(top_val, 1e-6) < 0.04 and top_axis != "H":
            # Top two dims are tied; if H is among them, prefer keeping H
            # upright (no rotation). Otherwise pick the one that lands best.
            for ax, val in items[:2]:
                if ax == "H":
                    return ax, val
            # Tie between two non-H axes — fall back to whichever is "more
            # vertical" by convention. For characters W is wider so D=H-like.
            return top_axis, second_val
        return top_axis, second_val

    if stem in flat_stems:
        # Flat objects: H must be the smallest. Rotate so the smallest dim
        # ends up on Y.
        smallest = min(w, h, d)
        if smallest == h:
            return ("H", "skip")
        if smallest == w:
            return ("W", "rotate_+90_Z")
        return ("D", "rotate_-90_X")
    if stem in standing_stems:
        # Force upright (H = longest). Pick whichever dim is longest and bring
        # it onto Y.
        if h >= w and h >= d:
            return ("H", "skip")
        if d >= w:
            return ("D", "rotate_-90_X")
        return ("W", "rotate_+90_Z")
    if stem in boat_stems:
        # Boats: keep the longest dim on X (horizontal keel) and the smallest
        # dim on Y (height).
        if h <= w and h <= d:
            return ("H", "skip")
        smallest = min(w, h, d)
        if smallest == w:
            return ("W", "rotate_+90_Z")
        return ("D", "rotate_-90_X")
    # Default: rotate so the longest dim ends up on Y, unless the longest
    # is already H (with tie tolerance).
    longest, _ = longest_axis_with_tie()
    return rotate_to_h(longest)


def apply_rotation(mode: str) -> None:
    """Rotate every mesh object so the targeted axis lands on Y, then bake."""
    bpy.ops.object.select_all(action="DESELECT")
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
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


def re_center_to_origin() -> None:
    """Re-center so the AABB min is at (0,0,0) and bottom sits on Y=0."""
    dims, mn = world_aabb()
    tx, ty, tz = -mn[0], -mn[1], -mn[2]
    if abs(tx) < 1e-5 and abs(ty) < 1e-5 and abs(tz) < 1e-5:
        return
    bpy.ops.object.select_all(action="DESELECT")
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
            obj.select_set(True)
    bpy.context.view_layer.objects.active = bpy.context.selected_objects[0]
    bpy.ops.transform.translate(value=(tx, ty, tz))


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
    n = import_glb(glb)
    if n == 0:
        return f"{glb.stem:32s} NO_MESH"
    dims, mn = world_aabb()
    longest, mode = decide_rotation(glb.stem, dims)
    w, h, d = dims
    if mode == "skip":
        return (
            f"{glb.stem:32s} WxHxD={w:4.2f}/{h:4.2f}/{d:4.2f}  "
            f"longest={longest}  action=skip"
        )
    if dry:
        return (
            f"{glb.stem:32s} WxHxD={w:4.2f}/{h:4.2f}/{d:4.2f}  "
            f"longest={longest}  action={mode} (DRY)"
        )
    apply_rotation(mode)
    re_center_to_origin()
    export_glb(glb)
    dims2, _ = world_aabb()
    w2, h2, d2 = dims2
    return (
        f"{glb.stem:32s} WxHxD={w:4.2f}/{h:4.2f}/{d:4.2f} -> "
        f"{w2:4.2f}/{h2:4.2f}/{d2:4.2f}  longest={longest}  action={mode}"
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