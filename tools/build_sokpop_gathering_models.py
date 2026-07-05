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

    skin = mat("skin_peach", (0.98, 0.78, 0.58), roughness=0.9)
    hair = mat("hair_chestnut", (0.34, 0.18, 0.09), roughness=0.95)
    shirt = mat("shirt_leaf_green", (0.23, 0.58, 0.32), roughness=0.9)
    scarf = mat("scarf_tomato", (0.86, 0.22, 0.16), roughness=0.85)
    trousers = mat("trousers_denim_blue", (0.18, 0.34, 0.58), roughness=0.9)
    boots = mat("boots_warm_brown", (0.28, 0.16, 0.08), roughness=0.95)
    eye = mat("eye_ink", (0.03, 0.04, 0.05), roughness=0.8)
    cheek = mat("cheek_coral", (0.95, 0.43, 0.36), roughness=0.9)

    # Forward is negative Y to match the existing player asset convention.
    cube("body", (0.0, 0.0, 1.16), (0.42, 0.30, 0.52), shirt)
    cube("neck", (0.0, 0.0, 1.76), (0.16, 0.14, 0.12), skin)
    uv_sphere("head", (0.0, -0.01, 2.12), (0.38, 0.34, 0.38), skin, 12, 6)
    cube("hair_cap", (0.0, 0.03, 2.38), (0.40, 0.30, 0.13), hair)
    cube("hair_side_l", (-0.31, -0.02, 2.12), (0.09, 0.23, 0.28), hair)
    cube("hair_side_r", (0.31, -0.02, 2.12), (0.09, 0.23, 0.28), hair)

    cube("eye_l", (-0.13, -0.34, 2.14), (0.045, 0.025, 0.055), eye)
    cube("eye_r", (0.13, -0.34, 2.14), (0.045, 0.025, 0.055), eye)
    cube("cheek_l", (-0.22, -0.335, 2.02), (0.06, 0.018, 0.035), cheek)
    cube("cheek_r", (0.22, -0.335, 2.02), (0.06, 0.018, 0.035), cheek)
    cube("smile", (0.0, -0.345, 1.96), (0.11, 0.018, 0.025), eye)

    cube("scarf_front", (0.0, -0.31, 1.68), (0.32, 0.045, 0.08), scarf)
    cube("scarf_tail", (0.18, -0.34, 1.46), (0.09, 0.04, 0.25), scarf)

    rotate(cylinder("upper_arm_l", (-0.42, -0.02, 1.34), 0.085, 0.50, shirt, vertices=8), 0.0, 0.18, -0.32)
    rotate(cylinder("upper_arm_r", (0.42, -0.02, 1.34), 0.085, 0.50, shirt, vertices=8), 0.0, -0.18, 0.32)
    rotate(cylinder("forearm_l", (-0.53, -0.05, 0.94), 0.075, 0.44, skin, vertices=8), 0.0, 0.22, -0.18)
    rotate(cylinder("forearm_r", (0.53, -0.05, 0.94), 0.075, 0.44, skin, vertices=8), 0.0, -0.22, 0.18)
    uv_sphere("hand_l", (-0.57, -0.08, 0.69), (0.10, 0.09, 0.10), skin, 8, 4)
    uv_sphere("hand_r", (0.57, -0.08, 0.69), (0.10, 0.09, 0.10), skin, 8, 4)

    cube("hip", (0.0, 0.0, 0.58), (0.38, 0.28, 0.18), trousers)
    rotate(cylinder("leg_l", (-0.16, 0.0, 0.15), 0.105, 0.70, trousers, vertices=8), 0.0, 0.08, 0.04)
    rotate(cylinder("leg_r", (0.16, 0.0, 0.15), 0.105, 0.70, trousers, vertices=8), 0.0, -0.08, -0.04)
    cube("boot_l", (-0.17, -0.08, -0.24), (0.14, 0.23, 0.10), boots)
    cube("boot_r", (0.17, -0.08, -0.24), (0.14, 0.23, 0.10), boots)

    export_glb("sokpop_gatherer")


def make_fallen_stick() -> None:
    clear_scene()

    bark = mat("bark_warm", (0.45, 0.25, 0.10), roughness=0.98)
    cut = mat("fresh_cut_wood", (0.77, 0.55, 0.31), roughness=0.95)
    moss = mat("moss_tip", (0.26, 0.48, 0.19), roughness=0.98)

    main = rotate(cylinder("main_branch", (0.0, 0.0, 0.10), 0.065, 1.20, bark, vertices=7), 0.0, math.radians(74), math.radians(18))
    side_a = rotate(cylinder("fork_a", (0.22, 0.06, 0.18), 0.038, 0.48, bark, vertices=7), math.radians(20), math.radians(58), math.radians(-38))
    side_b = rotate(cylinder("fork_b", (-0.18, -0.02, 0.16), 0.032, 0.38, bark, vertices=7), math.radians(-18), math.radians(62), math.radians(42))
    _ = (main, side_a, side_b)

    rotate(cylinder("cut_end_a", (-0.58, -0.19, 0.03), 0.067, 0.018, cut, vertices=7), 0.0, math.radians(74), math.radians(18))
    rotate(cylinder("cut_end_b", (0.58, 0.19, 0.17), 0.060, 0.018, cut, vertices=7), 0.0, math.radians(74), math.radians(18))
    uv_sphere("knot", (-0.08, -0.02, 0.17), (0.085, 0.055, 0.045), bark, 8, 4)
    uv_sphere("moss_patch", (0.05, 0.02, 0.22), (0.06, 0.035, 0.025), moss, 8, 4)

    export_glb("fallen_stick")


def main() -> int:
    print("=== building sokpop gathering models ===")
    make_sokpop_gatherer()
    make_fallen_stick()
    print("=== done ===")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
