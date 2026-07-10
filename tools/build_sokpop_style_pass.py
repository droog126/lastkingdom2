from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from models_lib import (
    clear_scene,
    cone_y_up as cone,
    cube_y_up as cube,
    cylinder_y_up as cylinder,
    export_glb,
    ico_sphere_y_up as ico_sphere,
    mat,
    uv_sphere_y_up as uv_sphere,
)


def m(name, color, roughness=0.92, alpha=1.0):
    return mat(name, color, roughness=roughness, alpha=alpha)


def slab(name, loc, radius, depth, material, vertices=9, x_scale=1.0, y_scale=1.0, rot=0.0):
    obj = cylinder(name, loc, radius, depth, material, vertices=vertices)
    obj.scale.x *= x_scale
    obj.scale.y *= y_scale
    obj.rotation_euler.z = rot
    return obj


def make_villager() -> None:
    clear_scene()
    skin = m("villager_skin", (0.86, 0.62, 0.44))
    skin_shadow = m("villager_skin_shadow", (0.70, 0.45, 0.30))
    hair = m("villager_hair", (0.24, 0.16, 0.10))
    tunic = m("villager_tunic", (0.45, 0.58, 0.38))
    tunic_dark = m("villager_tunic_dark", (0.29, 0.38, 0.26))
    pants = m("villager_pants", (0.28, 0.24, 0.20))
    boot = m("villager_boots", (0.18, 0.13, 0.09))
    eye = m("villager_eye", (0.06, 0.04, 0.03))
    wood = m("villager_tool_wood", (0.45, 0.29, 0.14))
    iron = m("villager_tool_iron", (0.55, 0.55, 0.52))

    # Compact Sokpop-like character: visible neck, overlapping head/body,
    # chunky stance, and one readable prop.
    cube("body", (0.0, 0.58, 0.0), (0.28, 0.34, 0.18), tunic)
    cube("neck", (0.0, 0.83, 0.0), (0.09, 0.10, 0.08), skin_shadow)
    ico_sphere("head", (0.0, 1.00, 0.0), (0.24, 0.24, 0.22), skin, subdivisions=2)
    cube("hair_cap", (0.0, 1.17, -0.02), (0.24, 0.10, 0.21), hair)
    cube("hair_front", (0.0, 1.08, 0.18), (0.20, 0.07, 0.04), hair)
    cube("nose", (0.0, 0.98, 0.21), (0.045, 0.055, 0.035), skin_shadow)
    cube("eye_l", (-0.075, 1.03, 0.205), (0.035, 0.035, 0.018), eye)
    cube("eye_r", (0.075, 1.03, 0.205), (0.035, 0.035, 0.018), eye)

    cube("belt", (0.0, 0.43, 0.01), (0.31, 0.05, 0.19), tunic_dark)
    cube("leg_l", (-0.09, 0.22, 0.0), (0.10, 0.30, 0.11), pants)
    cube("leg_r", (0.09, 0.22, 0.0), (0.10, 0.30, 0.11), pants)
    cube("boot_l", (-0.09, 0.055, 0.04), (0.13, 0.08, 0.17), boot)
    cube("boot_r", (0.09, 0.055, 0.04), (0.13, 0.08, 0.17), boot)

    cube("arm_l", (-0.25, 0.58, 0.0), (0.075, 0.28, 0.09), tunic)
    cube("arm_r", (0.25, 0.58, 0.0), (0.075, 0.28, 0.09), tunic)
    ico_sphere("hand_l", (-0.25, 0.38, 0.015), (0.065, 0.065, 0.06), skin, subdivisions=1)
    ico_sphere("hand_r", (0.25, 0.38, 0.015), (0.065, 0.065, 0.06), skin, subdivisions=1)

    cube("tool_handle", (0.39, 0.42, 0.09), (0.035, 0.42, 0.035), wood)
    cube("tool_head", (0.42, 0.67, 0.09), (0.15, 0.055, 0.07), iron)
    export_glb("villager")


def make_ground_patch() -> None:
    clear_scene()
    grass = m("ground_patch_grass", (0.31, 0.50, 0.28))
    grass_light = m("ground_patch_grass_light", (0.42, 0.61, 0.34))
    grass_dark = m("ground_patch_grass_dark", (0.20, 0.34, 0.20))
    dirt = m("ground_patch_dirt", (0.34, 0.25, 0.16))
    stone = m("ground_patch_stone", (0.42, 0.42, 0.38))

    slab("soil_bank", (0.0, -0.045, 0.0), 1.75, 0.09, dirt, vertices=9, x_scale=1.18, y_scale=0.88, rot=0.18)
    slab("grass_mass", (-0.08, 0.02, 0.02), 1.55, 0.055, grass, vertices=8, x_scale=1.18, y_scale=0.82, rot=-0.08)
    slab("raised_knoll", (-0.52, 0.09, -0.18), 0.58, 0.09, grass_light, vertices=7, x_scale=1.15, y_scale=0.72, rot=0.35)
    slab("worn_path", (0.46, 0.12, 0.22), 0.50, 0.025, dirt, vertices=6, x_scale=1.75, y_scale=0.34, rot=-0.38)
    slab("bare_dirt", (0.90, 0.11, -0.48), 0.32, 0.025, dirt, vertices=6, x_scale=1.10, y_scale=0.70, rot=0.20)
    for i, (x, z, sx, sz) in enumerate([(-1.05, 0.48, 0.26, 0.16), (0.12, -0.88, 0.22, 0.14), (1.06, 0.28, 0.20, 0.12)]):
        cube(f"grass_tuft_{i}", (x, 0.16, z), (sx, 0.16, sz), grass_dark)
    for i, (x, z, sx) in enumerate([(-0.78, -0.58, 0.11), (0.78, 0.62, 0.09), (0.10, 0.62, 0.08)]):
        ico_sphere(f"embedded_stone_{i}", (x, 0.15, z), (sx, 0.055, sx * 0.78), stone, subdivisions=1)
    export_glb("ground_patch")


def make_lake() -> None:
    clear_scene()
    grass = m("lake_bank_grass", (0.30, 0.49, 0.29))
    grass_dark = m("lake_bank_dark", (0.20, 0.33, 0.20))
    dirt = m("lake_bank_dirt", (0.34, 0.25, 0.16))
    water = m("lake_water", (0.18, 0.40, 0.56), roughness=0.78, alpha=0.86)
    water_deep = m("lake_deep_water", (0.12, 0.29, 0.42), roughness=0.80, alpha=0.88)
    reed = m("lake_reed", (0.48, 0.43, 0.20))
    lily = m("lake_lily", (0.25, 0.48, 0.25))

    slab("outer_bank", (0.0, -0.04, 0.0), 1.55, 0.10, dirt, vertices=10, x_scale=1.22, y_scale=0.82, rot=0.12)
    slab("grass_bank", (-0.08, 0.03, -0.03), 1.38, 0.06, grass, vertices=9, x_scale=1.18, y_scale=0.78, rot=-0.06)
    slab("water_outer", (0.08, 0.095, -0.02), 1.04, 0.035, water, vertices=12, x_scale=1.25, y_scale=0.68, rot=0.16)
    slab("water_deep", (0.12, 0.125, -0.04), 0.58, 0.025, water_deep, vertices=9, x_scale=1.35, y_scale=0.62, rot=-0.20)
    for i, (x, z, h) in enumerate([(-0.98, 0.38, 0.28), (-0.82, 0.58, 0.20), (1.05, -0.35, 0.26), (0.76, -0.58, 0.18)]):
        cube(f"reed_{i}", (x, 0.20 + h * 0.15, z), (0.026, h, 0.026), reed)
    cube("bank_tuft_a", (-1.18, -0.005, -0.22), (0.22, 0.14, 0.14), grass_dark)
    cube("bank_tuft_b", (0.98, 0.00, 0.32), (0.24, 0.12, 0.16), grass_dark)
    cube("lily_pad_a", (0.18, 0.15, 0.34), (0.18, 0.018, 0.12), lily)
    cube("lily_pad_b", (-0.42, 0.15, -0.20), (0.14, 0.018, 0.10), lily)
    export_glb("lake")


def make_swamp() -> None:
    clear_scene()
    mud = m("swamp_mud", (0.23, 0.20, 0.14))
    mud_light = m("swamp_mud_light", (0.32, 0.27, 0.18))
    moss = m("swamp_moss", (0.20, 0.38, 0.20))
    water = m("swamp_water", (0.12, 0.25, 0.18), roughness=0.82, alpha=0.82)
    wood = m("swamp_deadwood", (0.28, 0.20, 0.12))
    cap = m("swamp_mushroom_cap", (0.62, 0.30, 0.34))
    stem = m("swamp_mushroom_stem", (0.64, 0.55, 0.42))

    slab("mud_island", (0.0, -0.04, 0.0), 1.42, 0.10, mud, vertices=9, x_scale=1.18, y_scale=0.82, rot=-0.18)
    slab("shallow_water", (0.16, 0.06, -0.10), 0.98, 0.04, water, vertices=10, x_scale=1.30, y_scale=0.62, rot=0.20)
    slab("mud_bar", (-0.56, 0.11, 0.40), 0.38, 0.04, mud_light, vertices=6, x_scale=1.55, y_scale=0.45, rot=-0.35)
    for i, (x, z, sx, sz) in enumerate([(-0.78, 0.50, 0.30, 0.14), (0.75, -0.40, 0.26, 0.12), (0.22, 0.68, 0.22, 0.10)]):
        cube(f"moss_patch_{i}", (x, 0.13, z), (sx, 0.035, sz), moss)
    cube("fallen_log", (-0.12, 0.20, 0.46), (0.82, 0.11, 0.11), wood)
    cube("log_end", (0.34, 0.20, 0.46), (0.07, 0.14, 0.14), wood)
    cube("stump", (0.72, 0.23, 0.20), (0.13, 0.34, 0.13), wood)
    for i, (x, z, s) in enumerate([(-0.55, -0.40, 1.0), (-0.35, -0.56, 0.75), (0.52, -0.58, 0.65)]):
        cylinder(f"mush_stem_{i}", (x, 0.22 * s, z), 0.035 * s, 0.28 * s, stem, vertices=6)
        cone(f"mush_cap_{i}", (x, 0.40 * s, z), 0.11 * s, 0.035 * s, 0.10 * s, cap, vertices=7)
    export_glb("swamp")


def make_beach() -> None:
    clear_scene()
    sand = m("beach_sand", (0.66, 0.56, 0.36))
    sand_dark = m("beach_sand_shadow", (0.48, 0.40, 0.26))
    wet = m("beach_wet_sand", (0.43, 0.37, 0.26))
    water = m("beach_water", (0.16, 0.40, 0.54), roughness=0.80, alpha=0.84)
    foam = m("beach_foam", (0.78, 0.77, 0.65))
    shell = m("beach_shell", (0.70, 0.40, 0.36))
    trunk = m("beach_palm_trunk", (0.38, 0.24, 0.13))
    leaf = m("beach_palm_leaf", (0.25, 0.48, 0.22))

    slab("sand_bank", (0.0, -0.04, 0.0), 1.42, 0.10, sand_dark, vertices=9, x_scale=1.20, y_scale=0.82, rot=0.22)
    slab("sand_top", (-0.04, 0.03, -0.06), 1.24, 0.06, sand, vertices=8, x_scale=1.24, y_scale=0.75, rot=-0.12)
    slab("wet_sand", (-0.30, 0.09, 0.38), 0.58, 0.025, wet, vertices=6, x_scale=1.85, y_scale=0.35, rot=-0.03)
    slab("water_edge", (-0.34, 0.12, 0.64), 0.54, 0.035, water, vertices=6, x_scale=2.10, y_scale=0.36, rot=0.02)
    for i, x in enumerate([-0.82, -0.30, 0.26]):
        cube(f"foam_{i}", (x, 0.155, 0.48), (0.24, 0.018, 0.035), foam)
    cylinder("palm_trunk", (0.62, 0.35, -0.38), 0.065, 0.70, trunk, vertices=6)
    cube("palm_shadow", (0.46, 0.105, -0.44), (0.36, 0.025, 0.13), sand_dark)
    for i, ang in enumerate([0.0, 1.35, 2.6, 3.9, 5.1]):
        cube(
            f"palm_leaf_{i}",
            (0.62 + math.cos(ang) * 0.18, 0.76, -0.38 + math.sin(ang) * 0.18),
            (0.34, 0.045, 0.10),
            leaf,
        )
    ico_sphere("shell_a", (-0.50, 0.13, -0.32), (0.06, 0.035, 0.045), shell, subdivisions=1)
    ico_sphere("shell_b", (0.05, 0.13, -0.50), (0.05, 0.030, 0.040), foam, subdivisions=1)
    export_glb("beach")


def main() -> int:
    print("building focused Sokpop style pass")
    make_villager()
    make_ground_patch()
    make_lake()
    make_swamp()
    make_beach()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
