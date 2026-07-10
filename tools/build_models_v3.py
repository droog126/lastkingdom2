"""Build 10 more visual models for lastkingdom2: forest/magic/decor variety.

Adds: mushroom_red, mushroom_brown, crystal_blue, crystal_pink, treasure_chest,
      boat, arch_stone, cart, tombstone, haystack.

Output: assets/procedural/pretty/

Run through `python tools/model_pipeline.py build --generator build_models_v3.py`.
"""
from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import models_lib

from models_lib import (  # noqa: E402
    clear_scene, cone, cube, cylinder, export_glb, ico_sphere, mat, uv_sphere,
)

_cube = cube
_cone = cone
_cylinder = cylinder
_ico_sphere = ico_sphere
_uv_sphere = uv_sphere


def _loc_y_up(loc):
    return (loc[0], loc[2], loc[1])


def _scale_y_up(scale):
    return (scale[0], scale[2], scale[1])


def cube(name, loc, scale, material):
    return _cube(name, _loc_y_up(loc), _scale_y_up(scale), material)


def cone(name, loc, radius1, radius2, depth, material, vertices=8):
    return _cone(name, _loc_y_up(loc), radius1, radius2, depth, material, vertices=vertices)


def cylinder(name, loc, radius, depth, material, vertices=24):
    return _cylinder(name, _loc_y_up(loc), radius, depth, material, vertices=vertices)


def ico_sphere(name, loc, scale, material, subdivisions=1):
    return _ico_sphere(name, _loc_y_up(loc), _scale_y_up(scale), material, subdivisions=subdivisions)


def uv_sphere(name, loc, scale, material, segments=16, rings=8):
    return _uv_sphere(name, _loc_y_up(loc), _scale_y_up(scale), material, segments=segments, rings=rings)


def glow_mat(name, color, glow_strength=0.4, roughness=0.6):
    return mat(name, color, roughness=max(roughness, 0.82))


def mat_only(name, color, roughness=0.9):
    return mat(name, color, roughness=roughness)


# ============================================================== FOREST FLORA


def make_mushroom_red() -> None:
    clear_scene()
    cap = glow_mat("mush_r_cap", (0.92, 0.18, 0.18), glow_strength=0.55, roughness=0.55)
    cap_dark = glow_mat("mush_r_cap_dark", (0.65, 0.10, 0.10), glow_strength=0.3, roughness=0.7)
    dot = mat("mush_r_dot", (0.97, 0.95, 0.85), roughness=0.7)
    stem = glow_mat("mush_r_stem", (0.95, 0.92, 0.78), glow_strength=0.35, roughness=0.75)
    stem_dark = glow_mat("mush_r_stem_dark", (0.70, 0.65, 0.50), glow_strength=0.18, roughness=0.85)
    grass = glow_mat("mush_grass", (0.32, 0.55, 0.20), glow_strength=0.25, roughness=0.85)

    cone("cap", (0.0, 0.45, 0.0), 0.35, 0.0, 0.40, cap, vertices=14)
    cylinder("cap_ring", (0.0, 0.27, 0.0), 0.36, 0.04, cap_dark, vertices=14)
    cylinder("stem", (0.0, 0.15, 0.0), 0.10, 0.28, stem, vertices=12)
    cylinder("stem_base", (0.0, 0.02, 0.0), 0.14, 0.05, stem_dark, vertices=12)

    for ang in [0.3, 1.4, 2.7, 3.9, 5.1]:
        a = ang
        uv_sphere(f"dot_{ang:.1f}", (math.cos(a) * 0.22, 0.55, math.sin(a) * 0.22),
                  (0.05, 0.05, 0.05), dot, 8, 6)
    uv_sphere("dot_top", (0.0, 0.70, 0.0), (0.06, 0.06, 0.06), dot, 8, 6)

    for px, pz in [(-0.18, 0.05), (0.15, -0.08), (0.22, 0.12)]:
        cube("grass", (px, 0.02, pz), (0.04, 0.10, 0.04), grass)

    export_glb("mushroom_red")


def make_mushroom_brown() -> None:
    clear_scene()
    cap = glow_mat("mush_b_cap", (0.55, 0.35, 0.18), glow_strength=0.30, roughness=0.85)
    cap_dark = glow_mat("mush_b_cap_dark", (0.32, 0.20, 0.10), glow_strength=0.15, roughness=0.9)
    stem = glow_mat("mush_b_stem", (0.85, 0.78, 0.62), glow_strength=0.30, roughness=0.80)
    stem_dark = glow_mat("mush_b_stem_dark", (0.62, 0.55, 0.40), glow_strength=0.18, roughness=0.85)
    moss = glow_mat("mush_b_moss", (0.30, 0.55, 0.20), glow_strength=0.30, roughness=0.85)

    cone("cap", (0.0, 0.42, 0.0), 0.30, 0.05, 0.30, cap, vertices=12)
    cylinder("cap_rim", (0.0, 0.30, 0.0), 0.30, 0.05, cap_dark, vertices=12)
    cylinder("stem", (0.0, 0.14, 0.0), 0.09, 0.26, stem, vertices=10)
    cylinder("stem_base", (0.0, 0.02, 0.0), 0.13, 0.05, stem_dark, vertices=10)

    for ang in [0.5, 2.0, 3.5, 4.8]:
        uv_sphere(f"moss_{ang:.1f}", (math.cos(ang) * 0.18, 0.45, math.sin(ang) * 0.18),
                  (0.05, 0.04, 0.05), moss, 6, 4)
    uv_sphere("moss_top", (0.0, 0.62, 0.0), (0.05, 0.04, 0.05), moss, 6, 4)

    export_glb("mushroom_brown")


# ============================================================== CRYSTALS


def make_crystal_blue() -> None:
    clear_scene()
    core = glow_mat("crystal_b_core", (0.45, 0.85, 1.0), glow_strength=1.6, roughness=0.2)
    mid = glow_mat("crystal_b_mid", (0.30, 0.65, 0.95), glow_strength=1.3, roughness=0.25)
    base = glow_mat("crystal_b_base", (0.55, 0.80, 1.0), glow_strength=0.7, roughness=0.4)
    rock = mat_only("crystal_b_rock", (0.45, 0.42, 0.40))

    cone("main", (0.0, 0.55, 0.0), 0.18, 0.0, 0.95, core, vertices=6)
    cone("left", (-0.20, 0.30, 0.05), 0.12, 0.0, 0.55, mid, vertices=6)
    cone("right", (0.18, 0.30, -0.05), 0.10, 0.0, 0.50, mid, vertices=6)
    cone("back", (0.0, 0.25, 0.18), 0.09, 0.0, 0.40, base, vertices=6)
    cone("front", (0.0, 0.20, -0.16), 0.07, 0.0, 0.30, base, vertices=6)

    ico_sphere("glow", (0.0, 0.65, 0.0), (0.10, 0.10, 0.10), core, subdivisions=1)
    uv_sphere("glow_sm", (-0.20, 0.40, 0.05), (0.05, 0.05, 0.05), mid, 8, 6)
    uv_sphere("glow_sm2", (0.18, 0.40, -0.05), (0.04, 0.04, 0.04), mid, 8, 6)

    ico_sphere("rock1", (-0.25, 0.05, 0.10), (0.13, 0.07, 0.13), rock, subdivisions=1)
    ico_sphere("rock2", (0.20, 0.05, -0.12), (0.11, 0.06, 0.11), rock, subdivisions=1)
    ico_sphere("rock3", (0.05, 0.04, 0.18), (0.10, 0.06, 0.10), rock, subdivisions=1)
    cylinder("base_disc", (0.0, 0.03, 0.0), 0.30, 0.04, base, vertices=12)

    export_glb("crystal_blue")


def make_crystal_pink() -> None:
    clear_scene()
    core = glow_mat("crystal_p_core", (1.0, 0.55, 0.85), glow_strength=1.6, roughness=0.2)
    mid = glow_mat("crystal_p_mid", (0.95, 0.35, 0.65), glow_strength=1.3, roughness=0.25)
    base = glow_mat("crystal_p_base", (1.0, 0.70, 0.90), glow_strength=0.7, roughness=0.4)
    rock = mat_only("crystal_p_rock", (0.50, 0.42, 0.45))

    cone("main", (0.0, 0.50, 0.0), 0.16, 0.0, 0.85, core, vertices=6)
    cone("left", (-0.18, 0.28, -0.04), 0.11, 0.0, 0.50, mid, vertices=6)
    cone("right", (0.16, 0.28, 0.06), 0.09, 0.0, 0.45, mid, vertices=6)
    cone("back", (-0.04, 0.22, 0.16), 0.08, 0.0, 0.38, base, vertices=6)
    cone("front", (0.04, 0.18, -0.16), 0.07, 0.0, 0.30, base, vertices=6)

    ico_sphere("glow", (0.0, 0.60, 0.0), (0.09, 0.09, 0.09), core, subdivisions=1)

    ico_sphere("rock1", (-0.22, 0.04, -0.10), (0.12, 0.06, 0.12), rock, subdivisions=1)
    ico_sphere("rock2", (0.18, 0.05, 0.12), (0.10, 0.05, 0.10), rock, subdivisions=1)
    cylinder("base_disc", (0.0, 0.02, 0.0), 0.28, 0.04, base, vertices=12)

    export_glb("crystal_pink")


# ============================================================== TREASURE


def make_treasure_chest() -> None:
    clear_scene()
    wood = glow_mat("tc_wood", (0.55, 0.32, 0.15), glow_strength=0.25, roughness=0.85)
    wood_dark = glow_mat("tc_wood_dark", (0.32, 0.18, 0.08), glow_strength=0.12, roughness=0.9)
    gold = glow_mat("tc_gold", (1.0, 0.80, 0.30), glow_strength=1.2, roughness=0.3)
    gold_dark = glow_mat("tc_gold_dark", (0.80, 0.55, 0.15), glow_strength=0.7, roughness=0.4)
    gem = glow_mat("tc_gem", (0.45, 1.0, 0.85), glow_strength=1.6, roughness=0.2)
    inside = glow_mat("tc_inside", (0.30, 0.18, 0.08), glow_strength=0.10, roughness=0.95)

    cube("body", (0.0, 0.30, 0.0), (1.00, 0.60, 0.65), wood)
    cube("body_dark", (0.0, 0.30, -0.30), (1.00, 0.60, 0.10), wood_dark)
    cube("rim_top", (0.0, 0.60, 0.0), (1.05, 0.04, 0.70), wood_dark)
    cube("rim_bot", (0.0, 0.02, 0.0), (1.05, 0.04, 0.70), wood_dark)

    cube("lid_outer", (0.0, 0.85, -0.05), (1.02, 0.40, 0.65), wood)
    cube("lid_inner", (0.0, 0.85, 0.27), (0.95, 0.40, 0.05), inside)
    cube("lid_rim", (0.0, 0.65, 0.0), (1.06, 0.04, 0.70), wood_dark)

    for px in [-0.50, 0.50]:
        cube("strap_v", (px, 0.45, 0.0), (0.08, 0.90, 0.05), gold_dark)
        cube("strap_top_v", (px, 0.85, 0.0), (0.08, 0.40, 0.05), gold_dark)
    for py in [0.20, 0.50, 0.80]:
        cube("strap_h", (0.0, py, 0.32), (1.00, 0.05, 0.04), gold_dark)

    cube("lock_plate", (0.0, 0.55, 0.34), (0.20, 0.22, 0.02), gold)
    cube("keyhole", (0.0, 0.55, 0.36), (0.06, 0.10, 0.02), wood_dark)
    cube("corner_tl", (-0.48, 0.60, 0.32), (0.10, 0.06, 0.06), gold)
    cube("corner_tr", (0.48, 0.60, 0.32), (0.10, 0.06, 0.06), gold)
    cube("corner_tl_b", (-0.48, 0.60, -0.30), (0.10, 0.06, 0.06), gold_dark)
    cube("corner_tr_b", (0.48, 0.60, -0.30), (0.10, 0.06, 0.06), gold_dark)

    cone("lid_handle_l", (-0.20, 1.05, -0.05), 0.04, 0.0, 0.10, gold, vertices=6)
    cone("lid_handle_r", (0.20, 1.05, -0.05), 0.04, 0.0, 0.10, gold, vertices=6)

    uv_sphere("gem1", (0.0, 0.78, 0.10), (0.07, 0.05, 0.07), gem, 8, 6)
    uv_sphere("gem2", (-0.20, 0.74, 0.18), (0.05, 0.04, 0.05), gem, 8, 6)
    uv_sphere("gem3", (0.22, 0.76, 0.16), (0.06, 0.04, 0.06), gem, 8, 6)
    uv_sphere("coin1", (-0.30, 0.70, 0.10), (0.05, 0.05, 0.05),
              glow_mat("tc_coin", (1.0, 0.85, 0.30), glow_strength=0.9), 8, 4)
    uv_sphere("coin2", (0.30, 0.68, 0.08), (0.05, 0.05, 0.05),
              glow_mat("tc_coin2", (1.0, 0.85, 0.30), glow_strength=0.9), 8, 4)

    export_glb("treasure_chest")


# ============================================================== BOAT


def make_boat() -> None:
    clear_scene()
    wood = glow_mat("boat_wood", (0.55, 0.32, 0.15), glow_strength=0.25, roughness=0.85)
    wood_dark = glow_mat("boat_wood_dark", (0.32, 0.18, 0.08), glow_strength=0.12, roughness=0.9)
    rope = glow_mat("boat_rope", (0.85, 0.70, 0.40), glow_strength=0.30, roughness=0.85)
    sail = glow_mat("boat_sail", (0.92, 0.88, 0.78), glow_strength=0.5, roughness=0.7)
    sail_red = glow_mat("boat_sail_red", (0.85, 0.25, 0.25), glow_strength=0.5, roughness=0.7)
    barrel = glow_mat("boat_barrel", (0.50, 0.30, 0.15), glow_strength=0.2, roughness=0.85)
    lantern = glow_mat("boat_lantern", (1.0, 0.75, 0.30), glow_strength=1.5, roughness=0.3)

    cube("hull_bottom", (0.0, 0.04, 0.0), (1.40, 0.10, 0.60), wood_dark)
    cube("hull_side_l", (0.0, 0.18, 0.32), (1.40, 0.30, 0.04), wood)
    cube("hull_side_r", (0.0, 0.18, -0.32), (1.40, 0.30, 0.04), wood)
    cube("hull_back", (-0.70, 0.18, 0.0), (0.04, 0.30, 0.64), wood)
    cube("hull_front", (0.70, 0.18, 0.0), (0.04, 0.30, 0.64), wood)
    cube("rim_top", (0.0, 0.32, 0.0), (1.45, 0.04, 0.70), wood_dark)

    cube("seat_back", (-0.40, 0.45, 0.0), (0.30, 0.05, 0.60), wood_dark)
    cube("seat_front", (0.40, 0.45, 0.0), (0.30, 0.05, 0.60), wood_dark)
    cube("plank_mid", (0.0, 0.34, 0.0), (0.20, 0.02, 0.62), wood_dark)

    cube("mast", (0.0, 1.20, 0.0), (0.10, 1.80, 0.10), wood_dark)
    cube("boom", (0.0, 1.50, 0.0), (1.10, 0.06, 0.06), wood_dark)
    cube("sail_main", (0.0, 1.50, 0.05), (1.00, 1.00, 0.04), sail)
    cube("sail_strip", (0.0, 1.50, 0.08), (0.85, 0.06, 0.02), sail_red)

    cube("bow", (0.70, 0.50, 0.0), (0.20, 0.40, 0.10), wood)
    cube("bow_top", (0.85, 0.72, 0.0), (0.04, 0.04, 0.08), wood_dark)
    cone("bow_tip", (0.95, 0.62, 0.0), 0.04, 0.0, 0.20, wood_dark, vertices=4)

    cube("post_l", (-0.55, 0.55, 0.28), (0.06, 0.60, 0.06), wood_dark)
    cube("post_r", (-0.55, 0.55, -0.28), (0.06, 0.60, 0.06), wood_dark)
    cube("lantern_body", (-0.55, 0.85, 0.0), (0.18, 0.18, 0.04), wood_dark)
    cube("lantern_glow", (-0.55, 0.85, 0.04), (0.14, 0.14, 0.02), lantern)
    cone("lantern_roof", (-0.55, 1.00, 0.0), 0.10, 0.0, 0.10, wood_dark, vertices=4)

    cylinder("barrel", (0.40, 0.50, 0.0), 0.10, 0.26, barrel, vertices=10)
    cylinder("barrel_hoop1", (0.40, 0.40, 0.0), 0.105, 0.02, wood_dark, vertices=10)
    cylinder("barrel_hoop2", (0.40, 0.60, 0.0), 0.105, 0.02, wood_dark, vertices=10)

    for px in [-0.50, -0.20, 0.20, 0.50]:
        cylinder("tie", (px, 0.36, 0.32), 0.012, 0.10, rope, vertices=6)

    export_glb("boat")


# ============================================================== ARCH


def make_arch_stone() -> None:
    clear_scene()
    stone = glow_mat("arch_stone", (0.85, 0.82, 0.72), glow_strength=0.40, roughness=0.85)
    stone_dark = glow_mat("arch_stone_dark", (0.50, 0.46, 0.40), glow_strength=0.20, roughness=0.9)
    moss = glow_mat("arch_moss", (0.30, 0.55, 0.20), glow_strength=0.35, roughness=0.85)

    cube("base_l", (-0.85, 0.50, 0.0), (0.50, 1.00, 0.50), stone)
    cube("base_r", (0.85, 0.50, 0.0), (0.50, 1.00, 0.50), stone)
    cube("base_shadow_l", (-0.85, 0.50, -0.20), (0.50, 1.00, 0.15), stone_dark)
    cube("base_shadow_r", (0.85, 0.50, -0.20), (0.50, 1.00, 0.15), stone_dark)

    cube("block2_l", (-0.85, 1.10, 0.0), (0.55, 0.20, 0.55), stone_dark)
    cube("block2_r", (0.85, 1.10, 0.0), (0.55, 0.20, 0.55), stone_dark)

    cube("top_horizontal", (0.0, 1.55, 0.0), (2.20, 0.30, 0.55), stone)
    cube("top_shadow", (0.0, 1.55, -0.20), (2.20, 0.30, 0.15), stone_dark)

    cube("cap_l", (-0.85, 1.85, 0.0), (0.55, 0.20, 0.60), stone_dark)
    cube("cap_r", (0.85, 1.85, 0.0), (0.55, 0.20, 0.60), stone_dark)
    cube("cap_top", (0.0, 2.00, 0.0), (2.30, 0.10, 0.65), stone)

    cube("keystone", (0.0, 1.60, 0.30), (0.30, 0.40, 0.06), stone_dark)
    cube("keystone_dot", (0.0, 1.60, 0.34), (0.10, 0.10, 0.02), stone)

    cube("foundation_l", (-0.85, 0.05, 0.0), (0.70, 0.10, 0.70), stone_dark)
    cube("foundation_r", (0.85, 0.05, 0.0), (0.70, 0.10, 0.70), stone_dark)

    for px, pz in [(-0.95, 0.28), (0.95, -0.28), (-0.95, -0.28), (0.95, 0.28)]:
        uv_sphere("moss", (px, 0.95, pz), (0.10, 0.06, 0.10), moss, 8, 5)

    export_glb("arch_stone")


# ============================================================== CART


def make_cart() -> None:
    clear_scene()
    wood = glow_mat("cart_wood", (0.55, 0.30, 0.15), glow_strength=0.25, roughness=0.85)
    wood_dark = glow_mat("cart_wood_dark", (0.32, 0.18, 0.08), glow_strength=0.12, roughness=0.9)
    iron = mat_only("cart_iron", (0.35, 0.35, 0.38))
    wheel = glow_mat("cart_wheel", (0.45, 0.30, 0.18), glow_strength=0.20, roughness=0.85)
    hay = glow_mat("cart_hay", (0.85, 0.70, 0.30), glow_strength=0.40, roughness=0.85)
    pumpkin = glow_mat("cart_pumpkin", (0.95, 0.45, 0.15), glow_strength=0.45, roughness=0.7)
    pumpkin_dark = glow_mat("cart_pumpkin_dark", (0.55, 0.25, 0.08), glow_strength=0.20, roughness=0.8)

    cube("bed_bottom", (0.0, 0.42, 0.0), (1.30, 0.06, 0.85), wood_dark)
    cube("bed_side_l", (0.0, 0.55, 0.42), (1.30, 0.20, 0.04), wood)
    cube("bed_side_r", (0.0, 0.55, -0.42), (1.30, 0.20, 0.04), wood)
    cube("bed_front", (-0.65, 0.55, 0.0), (0.04, 0.20, 0.85), wood)
    cube("bed_back", (0.65, 0.55, 0.0), (0.04, 0.20, 0.85), wood)

    cube("post_fl", (-0.60, 0.85, 0.40), (0.10, 0.55, 0.10), wood_dark)
    cube("post_fr", (-0.60, 0.85, -0.40), (0.10, 0.55, 0.10), wood_dark)
    cube("post_bl", (0.60, 0.85, 0.40), (0.10, 0.55, 0.10), wood_dark)
    cube("post_br", (0.60, 0.85, -0.40), (0.10, 0.55, 0.10), wood_dark)

    cube("handle_l", (0.85, 0.70, 0.15), (0.40, 0.06, 0.06), wood_dark)
    cube("handle_r", (0.85, 0.70, -0.15), (0.40, 0.06, 0.06), wood_dark)
    cube("handle_top", (0.85, 0.95, 0.0), (0.40, 0.06, 0.36), wood_dark)
    cube("handle_knob", (0.85, 0.95, 0.0), (0.10, 0.10, 0.10), iron)

    for px in [-0.50, 0.50]:
        cylinder(f"wheel_{px}", (px, 0.22, 0.50), 0.22, 0.06, wheel, vertices=14)
        cylinder(f"wheel_{px}_in", (px, 0.22, 0.50), 0.05, 0.10, iron, vertices=8)
        cylinder(f"wheel_inner_{px}", (px, 0.22, -0.50), 0.22, 0.06, wheel, vertices=14)
        cylinder(f"wheel_inner_{px}_in", (px, 0.22, -0.50), 0.05, 0.10, iron, vertices=8)

    cube("axle", (0.0, 0.22, 0.0), (1.30, 0.06, 1.10), iron)

    cube("hay1", (-0.30, 0.72, 0.05), (0.40, 0.20, 0.40), hay)
    cube("hay2", (0.25, 0.78, -0.10), (0.45, 0.18, 0.40), hay)
    cube("hay3", (0.0, 0.85, 0.20), (0.50, 0.18, 0.35), hay)

    uv_sphere("pumpkin1", (0.45, 0.78, 0.20), (0.13, 0.12, 0.13), pumpkin, 10, 6)
    uv_sphere("pumpkin2", (-0.45, 0.80, -0.15), (0.12, 0.11, 0.12), pumpkin, 10, 6)
    cone("stem1", (0.45, 0.88, 0.20), 0.02, 0.0, 0.05, pumpkin_dark, vertices=6)
    cone("stem2", (-0.45, 0.90, -0.15), 0.02, 0.0, 0.05, pumpkin_dark, vertices=6)

    export_glb("cart")


# ============================================================== TOMBSTONE


def make_tombstone() -> None:
    clear_scene()
    stone = glow_mat("tomb_stone", (0.78, 0.78, 0.75), glow_strength=0.30, roughness=0.9)
    stone_dark = glow_mat("tomb_stone_dark", (0.50, 0.50, 0.48), glow_strength=0.15, roughness=0.92)
    moss = glow_mat("tomb_moss", (0.30, 0.55, 0.20), glow_strength=0.35, roughness=0.85)
    flower_w = glow_mat("tomb_flower_w", (1.0, 0.95, 0.85), glow_strength=0.55, roughness=0.6)
    flower_p = glow_mat("tomb_flower_p", (1.0, 0.55, 0.75), glow_strength=0.55, roughness=0.6)

    cube("base", (0.0, 0.10, 0.0), (0.90, 0.20, 0.50), stone_dark)
    cube("base_top", (0.0, 0.20, 0.0), (0.95, 0.04, 0.55), stone)
    cube("plinth", (0.0, 0.40, 0.0), (0.55, 0.20, 0.30), stone)

    cube("stone_lower", (0.0, 0.65, 0.0), (0.50, 0.30, 0.18), stone)
    cube("stone_main", (0.0, 0.95, 0.0), (0.46, 0.50, 0.16), stone)
    cube("stone_shadow", (0.0, 0.95, -0.07), (0.46, 0.50, 0.05), stone_dark)

    cube("cross_v", (0.0, 1.30, 0.0), (0.06, 0.50, 0.06), stone)
    cube("cross_h", (0.0, 1.40, 0.0), (0.30, 0.06, 0.06), stone)
    cube("cross_shadow_v", (0.0, 1.30, -0.03), (0.06, 0.50, 0.02), stone_dark)
    cube("cross_shadow_h", (0.0, 1.40, -0.03), (0.30, 0.06, 0.02), stone_dark)

    cube("nameplate", (0.0, 0.85, 0.09), (0.30, 0.18, 0.02), stone_dark)
    cube("nameplate_text", (0.0, 0.85, 0.105), (0.22, 0.08, 0.01),
          mat("tomb_text", (0.85, 0.78, 0.55), roughness=0.7))

    for px, pz, c in [(-0.30, 0.22, moss), (0.30, -0.22, moss), (-0.20, -0.18, moss), (0.20, 0.20, moss)]:
        uv_sphere(f"moss_{px}_{pz}", (px, 0.22, pz), (0.06, 0.04, 0.06), c, 6, 4)

    for ang, c in [(0.0, flower_w), (1.6, flower_p), (3.2, flower_w)]:
        x = math.cos(ang) * 0.32
        z = math.sin(ang) * 0.22
        uv_sphere(f"flower_{ang}", (x, 0.22, z), (0.04, 0.04, 0.04), c, 6, 5)
        cube(f"stem_{ang}", (x, 0.18, z), (0.012, 0.10, 0.012), moss)

    cube("crack", (-0.10, 0.85, 0.085), (0.02, 0.30, 0.005), stone_dark)
    cube("crack2", (0.10, 0.85, 0.085), (0.02, 0.20, 0.005), stone_dark)

    export_glb("tombstone")


# ============================================================== HAYSTACK


def make_haystack() -> None:
    clear_scene()
    hay = glow_mat("hay_main", (0.85, 0.70, 0.30), glow_strength=0.40, roughness=0.85)
    hay_dark = glow_mat("hay_dark", (0.60, 0.48, 0.18), glow_strength=0.20, roughness=0.9)
    hay_light = glow_mat("hay_light", (0.95, 0.82, 0.45), glow_strength=0.50, roughness=0.8)
    rope = glow_mat("hay_rope", (0.55, 0.40, 0.20), glow_strength=0.20, roughness=0.9)

    cube("base_wide", (0.0, 0.20, 0.0), (1.60, 0.40, 1.40), hay)
    cube("base_mid", (0.0, 0.45, 0.0), (1.40, 0.20, 1.20), hay_dark)
    cube("mid_wide", (0.0, 0.65, 0.0), (1.20, 0.30, 1.00), hay)
    cube("mid_mid", (0.0, 0.90, 0.0), (0.95, 0.20, 0.80), hay_dark)
    cube("upper_wide", (0.0, 1.10, 0.0), (0.75, 0.20, 0.60), hay)
    cube("upper_mid", (0.0, 1.30, 0.0), (0.55, 0.18, 0.45), hay_dark)
    cone("top", (0.0, 1.55, 0.0), 0.40, 0.0, 0.40, hay_light, vertices=12)

    for px, pz, c in [
        (-0.70, 0.55, hay_dark), (0.65, 0.60, hay_dark),
        (-0.55, -0.60, hay_light), (0.60, -0.55, hay_light),
        (0.0, 0.70, hay_light), (0.0, -0.70, hay_light),
    ]:
        cube(f"tuft_{px}_{pz}", (px, 0.95 + (abs(pz) * 0.1), pz), (0.12, 0.06, 0.12), c)

    cube("rope_h1", (0.0, 0.50, 0.0), (1.65, 0.025, 0.025), rope)
    cube("rope_h2", (0.0, 0.85, 0.0), (1.30, 0.025, 0.025), rope)
    cube("rope_v1", (0.0, 0.50, 0.72), (0.025, 0.80, 0.025), rope)
    cube("rope_v2", (0.0, 0.50, -0.72), (0.025, 0.80, 0.025), rope)

    export_glb("haystack")


# ============================================================== MAIN


def main() -> None:
    print("[v3] start building 10 new models...")
    make_mushroom_red()
    make_mushroom_brown()
    make_crystal_blue()
    make_crystal_pink()
    make_treasure_chest()
    make_boat()
    make_arch_stone()
    make_cart()
    make_tombstone()
    make_haystack()
    print("[v3] done.")


if __name__ == "__main__":
    main()
