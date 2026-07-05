"""Targeted rotation for Kenney models based on visual inspection.

Each Kenney GLB gets a hand-picked rotation to stand it up correctly. The
table was derived from looking at the model_preview screenshots and the
world AABB.

Usage:
    blender --background --python tools/rotate_kenney_v3.py -- [--dry-run]
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
        print("usage: blender --background --python rotate_kenney_v3.py -- [--dry-run]")
        sys.exit(2)
    argv = argv[argv.index("--") + 1 :]
    dry = False
    for a in argv:
        if a == "--dry-run":
            dry = True
    root = Path(r"F:\rustProject\lastkingdom2\assets\kenney\curated").resolve()
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


# Per-stem rotation plan derived from screenshot review. Modes:
#   "rotate_-90_X"  - swap Z and Y (Z becomes Y)
#   "rotate_+90_X"  - swap Z and -Y (Z becomes -Y)
#   "rotate_-90_Z"  - swap X and Y (X becomes Y)
#   "rotate_+90_Z"  - swap -X and Y (-X becomes Y)
#   "skip"          - already correct
PLAN: dict[str, str] = {
    # Animals: lying/standing cube poses. Rotate so the longest dim aligns
    # with Y (animals should stand tall).
    "kenney_bear": "rotate_-90_X",  # WxHxD=1.25/1.50/1.50, want tallest on Y
    "kenney_bunny": "rotate_-90_X",
    "kenney_cow": "rotate_-90_X",
    "kenney_deer": "rotate_-90_X",
    "kenney_fox": "skip",  # WxHxD=1.25/2.31/1.69, H is already tallest
    # Characters: already Y-up (H=2.00 ≈ 2m tall humans).
    "kenney_player_female_b": "skip",
    "kenney_player_male_b": "skip",
    "kenney_villager_female_a": "skip",
    "kenney_villager_male_a": "skip",
    # Props / coastal & pirate
    "kenney_cannon": "skip",  # already tall on Y
    "kenney_dock_platform": "rotate_-90_X",  # WxHxD=2.50/2.51/1.31, want Z=1.31 → Y for flat tile
    "kenney_palm_straight": "rotate_-90_X",
    "kenney_pirate_flag": "rotate_-90_X",  # WxHxD=1.34/0.40/2.10, want Z=2.10 → Y for vertical pole
    "kenney_row_boat_small": "rotate_-90_X",  # WxHxD=2.75/2.37/0.85, want Z=0.85 → Y for flat boat
    "kenney_ship_wreck": "skip",  # already tall
    # Survival props
    "kenney_campfire_pit": "rotate_-90_X",  # WxHxD=0.28/0.27/0.11, want Z=0.11 → Y (flat)
    "kenney_resource_stone": "rotate_-90_X",  # WxHxD=0.17/0.14/0.11, want Z=0.11 → Y
    "kenney_resource_wood": "rotate_+90_Z",  # WxHxD=0.21/0.09/0.06, want W=0.21 → Y (wood pile)
    "kenney_tent": "rotate_+90_Z",  # WxHxD=0.56/0.56/0.49, want X=0.56 → Y (tent up)
    "kenney_tool_axe": "rotate_-90_X",
    "kenney_tool_pickaxe": "rotate_-90_X",
    "kenney_workbench": "rotate_+90_Z",  # WxHxD=0.33/0.30/0.29, X longest, want X → Y
    # Terrain blocks — should be flat tiles. WxHxD=2.08/2.08/1.00, want Z=1.00 → Y.
    "kenney_block_grass_large": "rotate_-90_X",
    "kenney_block_snow_large": "rotate_-90_X",
    "kenney_coin_gold": "skip",  # WxHxD=0.40/0.17/0.40, H=0.17 already smallest (flat coin)
    "kenney_door_open": "rotate_-90_X",  # WxHxD=0.60/0.20/1.00, want Z=1.00 → Y
    "kenney_heart": "rotate_+90_Z",  # WxHxD=0.41/0.12/0.38, want X=0.41 → Y
}


def apply_rotation(mode: str) -> None:
    bpy.ops.object.select_all(action="DESELECT")
    for obj in collect_meshes():
        obj.select_set(True)
    if not bpy.context.selected_objects:
        return
    bpy.context.view_layer.objects.active = bpy.context.selected_objects[0]
    if mode == "rotate_-90_X":
        bpy.ops.transform.rotate(value=math.radians(-90), orient_axis="X")
    elif mode == "rotate_+90_X":
        bpy.ops.transform.rotate(value=math.radians(90), orient_axis="X")
    elif mode == "rotate_-90_Z":
        bpy.ops.transform.rotate(value=math.radians(-90), orient_axis="Z")
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
    dims_before, _ = world_aabb()
    mode = PLAN.get(glb.stem, "skip")
    w, h, d = dims_before
    if mode == "skip":
        return f"{glb.stem:32s} WxHxD={w:4.2f}/{h:4.2f}/{d:4.2f}  action=skip"
    if dry:
        return f"{glb.stem:32s} WxHxD={w:4.2f}/{h:4.2f}/{d:4.2f}  action={mode} (DRY)"
    apply_rotation(mode)
    re_center()
    export_glb(glb)
    dims_after, _ = world_aabb()
    w2, h2, d2 = dims_after
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