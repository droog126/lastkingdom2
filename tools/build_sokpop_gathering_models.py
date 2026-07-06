from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from models_lib import clear_scene, cube, cylinder, export_glb, mat, uv_sphere


def rotate(obj, x: float = 0.0, y: float = 0.0, z: float = 0.0):
    obj.rotation_euler = (x, y, z)
    return obj


def make_sokpop_gatherer() -> None:
    clear_scene()

    skin = mat("skin_simple_peach", (0.88, 0.63, 0.43), roughness=0.95)
    shirt = mat("shirt_simple_green", (0.22, 0.52, 0.30), roughness=0.96)
    shirt_dark = mat("shirt_simple_green_shadow", (0.13, 0.34, 0.21), roughness=0.96)
    trousers = mat("pants_simple_blue", (0.20, 0.36, 0.55), roughness=0.96)
    boot = mat("boot_simple_brown", (0.22, 0.13, 0.07), roughness=0.98)
    eye = mat("eye_simple_dark", (0.025, 0.025, 0.022), roughness=0.9)
    cheek = mat("cheek_muted_red", (0.76, 0.35, 0.30), roughness=0.95)
    basket = mat("basket_simple_reed", (0.55, 0.36, 0.16), roughness=0.99)
    basket_dark = mat("basket_simple_dark", (0.30, 0.18, 0.08), roughness=0.99)
    twig = mat("stick_simple_bark", (0.36, 0.20, 0.09), roughness=0.99)

    # Forward is negative Y. Neck overlaps both torso and head, so the simple
    # toy-like stack reads as connected instead of floating or crushed.
    cube("torso", (0.0, 0.0, 0.76), (0.42, 0.31, 0.66), shirt)
    cube("pants_block", (0.0, -0.005, 0.39), (0.36, 0.28, 0.34), trousers)
    cylinder("neck", (0.0, -0.015, 1.13), 0.135, 0.25, skin, vertices=8)
    cube("collar", (0.0, -0.02, 1.08), (0.28, 0.22, 0.07), shirt_dark)
    uv_sphere("head", (0.0, -0.015, 1.46), (0.35, 0.315, 0.33), skin, 10, 5)

    cube("eye_l", (-0.11, -0.315, 1.46), (0.038, 0.018, 0.048), eye)
    cube("eye_r", (0.11, -0.315, 1.46), (0.038, 0.018, 0.048), eye)
    cube("cheek_l", (-0.205, -0.312, 1.37), (0.040, 0.014, 0.030), cheek)
    cube("cheek_r", (0.205, -0.312, 1.37), (0.040, 0.014, 0.030), cheek)

    rotate(cylinder("arm_l", (-0.39, -0.02, 0.63), 0.070, 0.40, skin, vertices=7), 0.0, math.radians(8), math.radians(-8))
    rotate(cylinder("arm_r", (0.39, -0.02, 0.63), 0.070, 0.40, skin, vertices=7), 0.0, math.radians(-8), math.radians(8))
    uv_sphere("hand_l", (-0.42, -0.05, 0.38), (0.072, 0.064, 0.072), skin, 8, 4)
    uv_sphere("hand_r", (0.42, -0.05, 0.38), (0.072, 0.064, 0.072), skin, 8, 4)

    rotate(cylinder("leg_l", (-0.13, 0.0, 0.06), 0.080, 0.72, trousers, vertices=7), 0.0, 0.0, 0.0)
    rotate(cylinder("leg_r", (0.13, 0.0, 0.06), 0.080, 0.72, trousers, vertices=7), 0.0, 0.0, 0.0)
    cube("boot_l", (-0.13, -0.10, -0.35), (0.16, 0.30, 0.12), boot)
    cube("boot_r", (0.13, -0.10, -0.35), (0.16, 0.30, 0.12), boot)

    cube("basket", (0.0, 0.31, 0.68), (0.34, 0.10, 0.30), basket)
    cube("basket_rim", (0.0, 0.38, 0.87), (0.38, 0.035, 0.050), basket_dark)
    rotate(cylinder("stick", (0.56, -0.12, 0.42), 0.025, 0.54, twig, vertices=6), math.radians(70), math.radians(8), math.radians(25))

    export_glb("sokpop_gatherer")


def make_fallen_stick() -> None:
    clear_scene()

    bark = mat("bark_warm", (0.45, 0.25, 0.10), roughness=0.98)
    cut = mat("fresh_cut_wood", (0.77, 0.55, 0.31), roughness=0.95)
    moss = mat("moss_tip", (0.26, 0.48, 0.19), roughness=0.98)
    shadow = mat("soft_ground_shadow", (0.13, 0.09, 0.05), roughness=1.0, alpha=0.58)
    leaf = mat("attached_leaf", (0.34, 0.67, 0.20), roughness=0.98)

    cube("pickup_shadow", (0.0, 0.0, -0.015), (0.70, 0.26, 0.018), shadow)
    main = rotate(cylinder("main_branch", (0.0, 0.0, 0.10), 0.065, 1.20, bark, vertices=7), 0.0, math.radians(74), math.radians(18))
    side_a = rotate(cylinder("fork_a", (0.22, 0.06, 0.18), 0.038, 0.48, bark, vertices=7), math.radians(20), math.radians(58), math.radians(-38))
    side_b = rotate(cylinder("fork_b", (-0.18, -0.02, 0.16), 0.032, 0.38, bark, vertices=7), math.radians(-18), math.radians(62), math.radians(42))
    _ = (main, side_a, side_b)

    rotate(cylinder("cut_end_a", (-0.58, -0.19, 0.03), 0.067, 0.018, cut, vertices=7), 0.0, math.radians(74), math.radians(18))
    rotate(cylinder("cut_end_b", (0.58, 0.19, 0.17), 0.060, 0.018, cut, vertices=7), 0.0, math.radians(74), math.radians(18))
    uv_sphere("knot", (-0.08, -0.02, 0.17), (0.085, 0.055, 0.045), bark, 8, 4)
    uv_sphere("moss_patch", (0.05, 0.02, 0.22), (0.06, 0.035, 0.025), moss, 8, 4)
    cube("leaf_flag_a", (0.33, -0.05, 0.30), (0.10, 0.025, 0.055), leaf)
    cube("leaf_flag_b", (-0.28, 0.08, 0.23), (0.08, 0.022, 0.045), leaf)

    export_glb("fallen_stick")


def make_sokpop_tree() -> None:
    clear_scene()

    bark = mat("tree_bark_warm", (0.46, 0.25, 0.11), roughness=0.98)
    bark_dark = mat("tree_bark_shadow", (0.28, 0.14, 0.07), roughness=0.98)
    leaf = mat("leaf_round_green", (0.20, 0.58, 0.22), roughness=0.96)
    leaf_light = mat("leaf_sunlit_green", (0.42, 0.76, 0.27), roughness=0.96)
    leaf_dark = mat("leaf_deep_green", (0.12, 0.38, 0.18), roughness=0.96)
    apple = mat("tiny_apple_red", (0.86, 0.16, 0.12), roughness=0.9)
    leaf_tip = mat("leaf_yellow_tip", (0.72, 0.82, 0.26), roughness=0.96)
    hole = mat("tree_hole_dark", (0.10, 0.06, 0.03), roughness=1.0)
    drop_hint = mat("drop_hint_branch", (0.50, 0.29, 0.12), roughness=0.98)

    cylinder("trunk", (0.0, 0.0, 0.80), 0.20, 1.60, bark, vertices=8)
    cube("trunk_flat_front", (0.0, -0.185, 0.88), (0.12, 0.025, 0.72), bark_dark)
    uv_sphere("tree_hole", (0.0, -0.205, 0.92), (0.075, 0.020, 0.105), hole, 8, 4)
    rotate(cylinder("root_l", (-0.18, -0.05, 0.13), 0.055, 0.48, bark, vertices=7), 0.0, math.radians(78), math.radians(-18))
    rotate(cylinder("root_r", (0.18, -0.02, 0.13), 0.055, 0.48, bark, vertices=7), 0.0, math.radians(78), math.radians(18))
    rotate(cylinder("root_back", (0.0, 0.20, 0.13), 0.045, 0.42, bark, vertices=7), math.radians(78), 0.0, 0.0)

    rotate(cylinder("branch_l", (-0.28, -0.02, 1.48), 0.065, 0.62, bark, vertices=7), 0.0, math.radians(58), math.radians(-36))
    rotate(cylinder("branch_r", (0.30, 0.02, 1.56), 0.060, 0.58, bark, vertices=7), 0.0, math.radians(55), math.radians(38))
    rotate(cylinder("branch_back", (0.02, 0.22, 1.62), 0.052, 0.46, bark, vertices=7), math.radians(58), 0.0, math.radians(6))

    uv_sphere("canopy_center", (0.0, 0.0, 2.18), (0.78, 0.68, 0.62), leaf, 10, 5)
    uv_sphere("canopy_left", (-0.52, -0.06, 2.02), (0.48, 0.42, 0.42), leaf_dark, 10, 5)
    uv_sphere("canopy_right", (0.48, 0.05, 2.08), (0.50, 0.43, 0.45), leaf_light, 10, 5)
    uv_sphere("canopy_top", (0.02, 0.02, 2.62), (0.50, 0.44, 0.38), leaf_light, 10, 5)
    uv_sphere("canopy_front", (0.02, -0.42, 2.10), (0.42, 0.30, 0.38), leaf, 10, 5)
    cube("leaf_tip_l", (-0.72, -0.15, 2.15), (0.16, 0.07, 0.08), leaf_tip)
    cube("leaf_tip_r", (0.70, -0.08, 2.28), (0.15, 0.07, 0.08), leaf_tip)
    cube("leaf_tip_top", (0.05, -0.08, 2.90), (0.14, 0.07, 0.07), leaf_tip)

    uv_sphere("apple_l", (-0.35, -0.36, 1.93), (0.055, 0.050, 0.055), apple, 8, 4)
    uv_sphere("apple_r", (0.34, -0.30, 2.20), (0.050, 0.045, 0.050), apple, 8, 4)
    uv_sphere("apple_top", (0.08, -0.22, 2.50), (0.045, 0.040, 0.045), apple, 8, 4)
    rotate(cylinder("loose_branch_under_canopy", (0.26, -0.36, 1.76), 0.030, 0.46, drop_hint, vertices=7), math.radians(78), math.radians(18), math.radians(-18))

    export_glb("sokpop_tree")


def main() -> int:
    print("=== building sokpop gathering models ===")
    make_sokpop_gatherer()
    make_sokpop_tree()
    make_fallen_stick()
    print("=== done ===")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
