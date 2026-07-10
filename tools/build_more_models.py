"""Build 15 more models for lastkingdom2: 8 decor props, 4 building
variants, 3 creatures. v2 cute style consistent with previous batches.

Output: assets/procedural/pretty/

Run through `python tools/model_pipeline.py build --generator build_more_models.py`.
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


# ================================================================== DECOR
# Anchor convention: y=0 is footprint. Models are 0.5-2.5m tall.


def make_campfire() -> None:
    clear_scene()
    wood = glow_mat("cfire_wood", (0.50, 0.28, 0.12), glow_strength=0.15, roughness=0.95)
    wood_dark = glow_mat("cfire_wood_dark", (0.28, 0.15, 0.06), glow_strength=0.08, roughness=0.95)
    stone_g = mat_only("cfire_stone", (0.55, 0.55, 0.55))
    flame = glow_mat("cfire_flame", (1.0, 0.45, 0.10), glow_strength=1.6, roughness=0.3)
    flame_hot = glow_mat("cfire_flame_hot", (1.0, 0.85, 0.30), glow_strength=1.8, roughness=0.2)
    coal = glow_mat("cfire_coal", (0.85, 0.20, 0.05), glow_strength=1.2, roughness=0.6)

    for sx, sz, scale in [
        (0.35, 0.0, 0.95), (-0.30, 0.10, 0.90), (0.10, -0.35, 0.95),
        (-0.10, 0.30, 0.90), (0.0, 0.0, 1.10),
    ]:
        ang = math.atan2(sz, sx)
        cube("log", (sx, 0.10, sz), (scale, 0.18, 0.18), wood)
        cone("log_cap", (sx + math.cos(ang) * 0.5, 0.10, sz + math.sin(ang) * 0.5),
             0.10, 0.0, 0.10, wood_dark, vertices=6)

    for ang in [0.0, 1.0, 2.1, 3.14, 4.2, 5.24]:
        r = 0.55
        sx = math.cos(ang) * r
        sz = math.sin(ang) * r
        ico_sphere("stone", (sx, 0.10, sz), (0.18, 0.12, 0.18), stone_g, subdivisions=1)

    uv_sphere("coal1", (0.0, 0.20, 0.0), (0.18, 0.10, 0.18), coal, 10, 6)
    uv_sphere("coal2", (0.10, 0.22, 0.10), (0.12, 0.08, 0.12), coal, 10, 5)

    cone("flame_outer", (0.0, 0.60, 0.0), 0.30, 0.0, 0.90, flame, vertices=8)
    cone("flame_inner", (0.0, 0.55, 0.0), 0.18, 0.0, 0.70, flame_hot, vertices=8)
    uv_sphere("flame_glow", (0.0, 0.30, 0.0), (0.15, 0.12, 0.15), flame_hot, 10, 6)

    for ang in [0.5, 2.6, 4.7]:
        r = 0.25
        sx = math.cos(ang) * r
        sz = math.sin(ang) * r
        uv_sphere("spark", (sx, 0.90 + (ang % 2) * 0.2, sz), (0.04, 0.04, 0.04), flame_hot, 6, 4)

    export_glb("campfire")


def make_lantern_post() -> None:
    clear_scene()
    wood = glow_mat("lp_wood", (0.40, 0.25, 0.12), glow_strength=0.2, roughness=0.9)
    wood_dark = glow_mat("lp_wood_dark", (0.25, 0.15, 0.08), glow_strength=0.1, roughness=0.9)
    iron = mat_only("lp_iron", (0.30, 0.30, 0.32))
    glass = glow_mat("lp_glass", (1.0, 0.85, 0.45), glow_strength=1.6, roughness=0.2)
    flame = glow_mat("lp_flame", (1.0, 0.70, 0.25), glow_strength=1.8, roughness=0.2)

    cube("base", (0.0, 0.10, 0.0), (0.45, 0.20, 0.45), wood_dark)
    cube("post_low", (0.0, 0.55, 0.0), (0.18, 0.70, 0.18), wood)
    cube("post_mid", (0.0, 1.40, 0.0), (0.16, 0.40, 0.16), wood)
    cube("post_high", (0.0, 1.95, 0.0), (0.14, 0.30, 0.14), wood)

    cube("arm", (0.30, 1.95, 0.0), (0.45, 0.08, 0.08), wood)

    cube("lantern_top", (0.55, 2.10, 0.0), (0.30, 0.10, 0.30), iron)
    cube("lantern_base", (0.55, 1.90, 0.0), (0.28, 0.08, 0.28), iron)
    cube("glass_n", (0.55, 2.00, 0.16), (0.26, 0.30, 0.02), glass)
    cube("glass_s", (0.55, 2.00, -0.16), (0.26, 0.30, 0.02), glass)
    cube("glass_e", (0.69, 2.00, 0.0), (0.02, 0.30, 0.26), glass)
    cube("glass_w", (0.41, 2.00, 0.0), (0.02, 0.30, 0.26), glass)
    cone("lantern_roof", (0.55, 2.30, 0.0), 0.22, 0.0, 0.15, iron, vertices=4)

    cone("flame", (0.55, 2.02, 0.0), 0.05, 0.0, 0.10, flame, vertices=6)
    uv_sphere("flame_core", (0.55, 1.98, 0.0), (0.04, 0.06, 0.04), flame, 8, 5)

    cube("chain", (0.55, 2.18, 0.0), (0.04, 0.04, 0.04), iron)
    cube("chain2", (0.55, 2.13, 0.0), (0.04, 0.04, 0.04), iron)

    export_glb("lantern_post")


def make_crate() -> None:
    clear_scene()
    wood = glow_mat("crate_wood", (0.65, 0.45, 0.25), glow_strength=0.25, roughness=0.9)
    wood_dark = glow_mat("crate_wood_dark", (0.40, 0.25, 0.12), glow_strength=0.12, roughness=0.95)
    iron = mat_only("crate_iron", (0.30, 0.30, 0.32))
    mark = glow_mat("crate_mark", (0.95, 0.70, 0.25), glow_strength=0.5, roughness=0.7)

    cube("body", (0.0, 0.40, 0.0), (0.70, 0.80, 0.70), wood)
    cube("top", (0.0, 0.82, 0.0), (0.74, 0.04, 0.74), wood_dark)
    cube("bot", (0.0, -0.02, 0.0), (0.74, 0.04, 0.74), wood_dark)

    for px, pz in [(-0.20, -0.20), (0.20, -0.20), (-0.20, 0.20), (0.20, 0.20)]:
        cube("strap_v", (px, 0.40, pz), (0.10, 0.85, 0.04), iron)

    for py in [0.20, 0.55]:
        cube("strap_h", (0.0, py, 0.20), (0.65, 0.05, 0.04), iron)
        cube("strap_h_b", (0.0, py, -0.20), (0.65, 0.05, 0.04), iron)

    cube("mark", (0.0, 0.40, 0.36), (0.18, 0.18, 0.02), mark)

    cube("lid_loose", (0.18, 0.95, 0.0), (0.50, 0.05, 0.50), wood_dark)
    cube("lid_loose_top", (0.18, 0.99, 0.0), (0.52, 0.03, 0.52), wood_dark)

    export_glb("crate")


def make_barrel() -> None:
    clear_scene()
    wood = glow_mat("barr_wood", (0.55, 0.35, 0.18), glow_strength=0.2, roughness=0.9)
    wood_dark = glow_mat("barr_wood_dark", (0.30, 0.18, 0.08), glow_strength=0.1, roughness=0.95)
    iron = mat_only("barr_iron", (0.35, 0.35, 0.38))
    liquid = glow_mat("barr_liquid", (0.40, 0.25, 0.10), glow_strength=0.5, roughness=0.5)

    cylinder("body", (0.0, 0.50, 0.0), 0.32, 1.00, wood, vertices=14)
    cylinder("top", (0.0, 1.02, 0.0), 0.34, 0.06, wood_dark, vertices=14)
    cylinder("bot", (0.0, -0.02, 0.0), 0.34, 0.06, wood_dark, vertices=14)

    for py in [0.15, 0.45, 0.80]:
        cylinder("hoop", (0.0, py, 0.0), 0.36, 0.05, iron, vertices=14)

    for ang in [0.0, 1.6, 3.14, 4.7]:
        x = math.cos(ang) * 0.34
        z = math.sin(ang) * 0.34
        cube("hoop_vert", (x, 0.50, z), (0.04, 1.00, 0.04), iron)

    cylinder("liquid", (0.0, 0.90, 0.0), 0.28, 0.04, liquid, vertices=12)
    cube("lid_loose", (0.10, 1.10, 0.10), (0.30, 0.05, 0.30), wood_dark)

    export_glb("barrel")


def make_signpost() -> None:
    clear_scene()
    wood = glow_mat("sign_wood", (0.55, 0.32, 0.15), glow_strength=0.2, roughness=0.9)
    wood_dark = glow_mat("sign_wood_dark", (0.30, 0.18, 0.08), glow_strength=0.1, roughness=0.95)
    rope = glow_mat("sign_rope", (0.85, 0.70, 0.40), glow_strength=0.3, roughness=0.85)
    text_color = glow_mat("sign_text", (0.95, 0.85, 0.45), glow_strength=0.6, roughness=0.6)

    cube("post", (0.0, 0.85, 0.0), (0.16, 1.70, 0.16), wood)
    cube("post_dark", (0.0, 0.05, 0.0), (0.22, 0.10, 0.22), wood_dark)

    cube("sign_l", (-0.40, 1.30, 0.0), (0.70, 0.32, 0.05), wood)
    cube("sign_l_face", (-0.40, 1.30, 0.03), (0.60, 0.22, 0.02),
         mat("sign_face", (0.95, 0.90, 0.75), roughness=0.7))
    cube("sign_text_l", (-0.40, 1.32, 0.05), (0.40, 0.10, 0.02), text_color)

    cube("sign_r", (0.40, 1.50, 0.0), (0.60, 0.32, 0.05), wood)
    cube("sign_r_face", (0.40, 1.50, 0.03), (0.50, 0.22, 0.02),
         mat("sign_face2", (0.95, 0.90, 0.75), roughness=0.7))
    cube("sign_text_r", (0.40, 1.52, 0.05), (0.30, 0.08, 0.02), text_color)

    for px in [-0.50, -0.30]:
        cylinder("chain", (px, 1.42, 0.04), 0.015, 0.18, rope, vertices=6)
    for px in [0.30, 0.50]:
        cylinder("chain2", (px, 1.62, 0.04), 0.015, 0.18, rope, vertices=6)

    cone("post_cap", (0.0, 1.78, 0.0), 0.12, 0.0, 0.12, wood_dark, vertices=4)

    export_glb("signpost")


def make_market_stall() -> None:
    clear_scene()
    wood = glow_mat("ms_wood", (0.55, 0.30, 0.15), glow_strength=0.2, roughness=0.9)
    wood_dark = glow_mat("ms_wood_dark", (0.30, 0.18, 0.08), glow_strength=0.1, roughness=0.95)
    cloth_red = glow_mat("ms_cloth_red", (0.85, 0.25, 0.25), glow_strength=0.5, roughness=0.8)
    cloth_strip = glow_mat("ms_cloth_strip", (0.95, 0.75, 0.30), glow_strength=0.5, roughness=0.8)
    apple = glow_mat("ms_apple", (0.95, 0.20, 0.20), glow_strength=0.5, roughness=0.5)
    bread = glow_mat("ms_bread", (0.85, 0.60, 0.30), glow_strength=0.4, roughness=0.7)
    melon = glow_mat("ms_melon", (0.30, 0.75, 0.30), glow_strength=0.4, roughness=0.5)

    cube("table", (0.0, 0.80, 0.0), (1.40, 0.10, 0.80), wood)
    cube("leg_fl", (-0.65, 0.40, -0.35), (0.10, 0.80, 0.10), wood_dark)
    cube("leg_fr", (0.65, 0.40, -0.35), (0.10, 0.80, 0.10), wood_dark)
    cube("leg_bl", (-0.65, 0.40, 0.35), (0.10, 0.80, 0.10), wood_dark)
    cube("leg_br", (0.65, 0.40, 0.35), (0.10, 0.80, 0.10), wood_dark)
    cube("cross_x", (0.0, 0.40, 0.0), (1.30, 0.05, 0.05), wood_dark)
    cube("cross_z", (0.0, 0.40, 0.0), (0.05, 0.05, 0.70), wood_dark)

    cube("post_l", (-0.55, 1.50, -0.20), (0.08, 1.50, 0.08), wood)
    cube("post_r", (0.55, 1.50, -0.20), (0.08, 1.50, 0.08), wood)
    cube("beam", (0.0, 2.20, -0.20), (1.30, 0.10, 0.10), wood)

    cone("canopy", (0.0, 2.45, 0.0), 0.95, 0.0, 0.50, cloth_red, vertices=4)
    cube("canopy_strip", (0.0, 2.40, 0.0), (1.30, 0.06, 0.06), cloth_strip)
    cube("canopy_strip2", (0.0, 2.20, 0.20), (1.30, 0.06, 0.06), cloth_strip)

    for i, (x, z, c) in enumerate([
        (-0.45, 0.20, apple), (0.0, 0.15, apple), (0.40, 0.20, apple),
        (-0.40, 0.0, bread), (0.30, 0.05, bread),
        (0.0, 0.30, melon), (0.30, 0.30, apple),
    ]):
        uv_sphere(f"fruit_{i}", (x, 0.95, z), (0.08, 0.08, 0.08), c, 10, 6)

    cube("basket", (-0.45, 0.95, 0.25), (0.30, 0.18, 0.18), wood_dark)
    cube("basket2", (0.30, 0.95, 0.25), (0.30, 0.18, 0.18), wood_dark)

    export_glb("market_stall")


def make_bench() -> None:
    clear_scene()
    wood = glow_mat("bench_wood", (0.55, 0.30, 0.15), glow_strength=0.2, roughness=0.9)
    wood_dark = glow_mat("bench_wood_dark", (0.30, 0.18, 0.08), glow_strength=0.1, roughness=0.95)
    iron = mat_only("bench_iron", (0.40, 0.40, 0.45))

    cube("seat", (0.0, 0.45, 0.0), (1.40, 0.08, 0.40), wood)
    cube("back", (0.0, 0.85, -0.20), (1.40, 0.55, 0.06), wood)

    for px in [-0.55, 0.55]:
        cube("leg_l", (px, 0.22, 0.0), (0.08, 0.45, 0.40), wood_dark)
        cube("leg_top", (px, 0.42, 0.15), (0.12, 0.10, 0.06), wood_dark)

    for py in [0.95, 1.15, 1.35]:
        cube("back_rail", (0.0, py, -0.20), (1.30, 0.04, 0.04), wood_dark)

    for px in [-0.60, -0.20, 0.20, 0.60]:
        cube("bolt", (px, 0.45, 0.21), (0.04, 0.04, 0.04), iron)
        cube("bolt_b", (px, 0.45, -0.20), (0.04, 0.04, 0.04), iron)

    export_glb("bench")


def make_fountain() -> None:
    clear_scene()
    stone = glow_mat("fount_stone", (0.85, 0.82, 0.75), glow_strength=0.4, roughness=0.85)
    stone_dark = glow_mat("fount_stone_dark", (0.55, 0.50, 0.45), glow_strength=0.2, roughness=0.9)
    water = glow_mat("fount_water", (0.45, 0.75, 1.0), glow_strength=0.6, roughness=0.3)
    water_glow = glow_mat("fount_water_glow", (0.70, 0.95, 1.0), glow_strength=0.9, roughness=0.2)
    moss = glow_mat("fount_moss", (0.30, 0.70, 0.30), glow_strength=0.45, roughness=0.9)

    cylinder("pool_outer", (0.0, 0.10, 0.0), 0.95, 0.20, stone_dark, vertices=20)
    cylinder("pool_inner", (0.0, 0.12, 0.0), 0.78, 0.18, stone, vertices=20)
    cylinder("water_pool", (0.0, 0.20, 0.0), 0.70, 0.06, water, vertices=20)

    cylinder("pillar", (0.0, 0.50, 0.0), 0.22, 0.80, stone, vertices=14)
    cylinder("pillar_top", (0.0, 0.92, 0.0), 0.30, 0.10, stone_dark, vertices=14)
    cylinder("pillar_top2", (0.0, 1.02, 0.0), 0.26, 0.10, stone, vertices=14)

    cylinder("upper_basin", (0.0, 1.18, 0.0), 0.45, 0.10, stone_dark, vertices=16)
    cylinder("upper_water", (0.0, 1.20, 0.0), 0.40, 0.06, water_glow, vertices=16)

    cylinder("spout_low", (0.0, 0.85, 0.0), 0.05, 0.20, stone_dark, vertices=6)
    uv_sphere("water_top", (0.0, 1.40, 0.0), (0.10, 0.10, 0.10), water_glow, 10, 6)

    cone("water_splash", (0.0, 1.55, 0.0), 0.20, 0.05, 0.40, water_glow, vertices=10)
    cone("water_splash2", (0.0, 1.45, 0.0), 0.15, 0.03, 0.30, water, vertices=8)

    for ang in [0.0, 1.57, 3.14, 4.71]:
        x = math.cos(ang) * 0.78
        z = math.sin(ang) * 0.78
        ico_sphere("moss", (x, 0.30, z), (0.16, 0.10, 0.16), moss, subdivisions=1)

    cube("rim_low", (0.0, 0.22, 0.0), (1.80, 0.04, 0.04), stone_dark)
    cube("rim_high", (0.0, 0.22, 0.0), (0.04, 0.04, 1.80), stone_dark)

    export_glb("fountain")


def make_statue() -> None:
    clear_scene()
    stone = glow_mat("stat_stone", (0.92, 0.88, 0.78), glow_strength=0.5, roughness=0.85)
    stone_dark = glow_mat("stat_stone_dark", (0.55, 0.50, 0.42), glow_strength=0.2, roughness=0.9)
    moss = glow_mat("stat_moss", (0.30, 0.70, 0.30), glow_strength=0.45, roughness=0.9)
    gem = glow_mat("stat_gem", (0.55, 0.85, 1.0), glow_strength=1.4, roughness=0.2)
    sword = glow_mat("stat_sword", (0.85, 0.82, 0.95), glow_strength=0.7, roughness=0.3)

    cube("base", (0.0, 0.20, 0.0), (1.10, 0.40, 1.10), stone_dark)
    cube("base2", (0.0, 0.50, 0.0), (0.95, 0.20, 0.95), stone)
    cube("base3", (0.0, 0.62, 0.0), (0.85, 0.06, 0.85), stone_dark)

    cube("legs", (0.0, 0.95, 0.0), (0.50, 0.55, 0.40), stone)

    cube("torso", (0.0, 1.55, 0.0), (0.55, 0.60, 0.35), stone)

    cube("shoulder_l", (-0.30, 1.75, 0.0), (0.20, 0.18, 0.30), stone)
    cube("shoulder_r", (0.30, 1.75, 0.0), (0.20, 0.18, 0.30), stone)

    cube("arm_l", (-0.32, 1.40, 0.10), (0.14, 0.65, 0.14), stone)
    cube("arm_r", (0.32, 1.40, 0.10), (0.14, 0.65, 0.14), stone)

    cube("hand_l", (-0.32, 1.00, 0.18), (0.18, 0.20, 0.20), stone)
    cube("hand_r", (0.32, 1.00, 0.18), (0.18, 0.20, 0.20), stone)

    uv_sphere("head", (0.0, 2.05, 0.0), (0.20, 0.24, 0.22), stone, 12, 10)
    cone("hat", (0.0, 2.30, 0.0), 0.20, 0.0, 0.30, stone_dark, vertices=8)

    cube("sword_hilt", (0.32, 1.15, 0.18), (0.06, 0.10, 0.06), stone_dark)
    cube("sword_blade", (0.32, 1.55, 0.18), (0.05, 0.70, 0.04), sword)
    cube("sword_tip", (0.32, 1.95, 0.18), (0.04, 0.06, 0.04), sword)

    uv_sphere("orb", (0.0, 2.55, 0.0), (0.08, 0.08, 0.08), gem, 10, 6)
    cone("orb_spike", (0.0, 2.70, 0.0), 0.05, 0.0, 0.15, gem, vertices=6)

    for x, z in [(-0.55, 0.55), (0.55, 0.55), (-0.55, -0.55), (0.55, -0.55)]:
        ico_sphere("moss", (x, 0.42, z), (0.14, 0.08, 0.14), moss, subdivisions=1)

    export_glb("statue")


# ============================================================== BUILDINGS v3


def make_forge() -> None:
    clear_scene()
    brick = glow_mat("forge_brick", (0.65, 0.32, 0.22), glow_strength=0.3, roughness=0.9)
    brick_dark = glow_mat("forge_brick_dark", (0.42, 0.20, 0.12), glow_strength=0.15, roughness=0.95)
    wood = glow_mat("forge_wood", (0.50, 0.28, 0.12), glow_strength=0.2, roughness=0.9)
    iron = mat_only("forge_iron", (0.30, 0.30, 0.32))
    coal = glow_mat("forge_coal", (0.85, 0.20, 0.05), glow_strength=1.2, roughness=0.6)
    coal_hot = glow_mat("forge_coal_hot", (1.0, 0.55, 0.10), glow_strength=1.6, roughness=0.3)
    anvil = mat_only("forge_anvil", (0.25, 0.25, 0.28))
    smoke = mat("forge_smoke", (0.45, 0.40, 0.40), roughness=0.95, alpha=0.55)

    cube("body", (0.0, 0.90, 0.0), (1.80, 1.80, 1.50), brick)
    cube("body_shadow", (0.0, 0.90, -0.40), (1.80, 1.80, 0.30), brick_dark)

    cube("furnace", (0.0, 0.40, 0.65), (0.70, 0.50, 0.20), brick_dark)
    cube("furnace_mouth", (0.0, 0.45, 0.76), (0.40, 0.30, 0.02),
         glow_mat("forge_furnace_glow", (1.0, 0.50, 0.10), glow_strength=1.6, roughness=0.3))
    uv_sphere("coal_glow", (0.0, 0.45, 0.72), (0.15, 0.10, 0.04), coal_hot, 10, 6)
    uv_sphere("coal2", (0.0, 0.55, 0.72), (0.10, 0.06, 0.02), coal, 10, 5)

    cube("chimney", (0.55, 2.10, 0.0), (0.30, 1.20, 0.30), brick_dark)
    cube("chimney_top", (0.55, 2.75, 0.0), (0.36, 0.06, 0.36), brick)
    uv_sphere("smoke1", (0.55, 3.10, 0.0), (0.30, 0.30, 0.30), smoke, 10, 7)
    uv_sphere("smoke2", (0.65, 3.50, -0.05), (0.40, 0.40, 0.40), smoke, 10, 7)
    uv_sphere("smoke3", (0.50, 3.90, 0.05), (0.50, 0.50, 0.50), smoke, 10, 7)

    cone("roof_main", (0.0, 2.30, 0.0), 1.20, 0.0, 0.80, wood, vertices=4)
    cube("roof_eave", (0.0, 1.92, 0.0), (2.00, 0.10, 1.65), brick_dark)

    cube("door", (-0.50, 0.30, 0.76), (0.40, 0.60, 0.04), wood)
    cube("window1", (0.45, 1.30, 0.76), (0.40, 0.40, 0.04),
         glow_mat("forge_window", (1.0, 0.65, 0.30), glow_strength=1.4, roughness=0.4))
    cube("window1_x_h", (0.45, 1.30, 0.78), (0.40, 0.04, 0.02), wood)
    cube("window1_x_v", (0.45, 1.30, 0.78), (0.04, 0.40, 0.02), wood)

    cube("anvil_base", (0.55, 0.20, 0.95), (0.40, 0.10, 0.30), wood)
    cube("anvil_top", (0.55, 0.40, 0.95), (0.50, 0.16, 0.18), anvil)
    cube("anvil_horn", (0.85, 0.42, 0.95), (0.10, 0.10, 0.06), anvil)

    cube("hammer", (0.0, 0.45, 0.90), (0.08, 0.18, 0.08), iron)
    cube("hammer_head", (0.0, 0.60, 0.90), (0.20, 0.08, 0.10), iron)

    cylinder("water_barrel", (-0.85, 0.30, 0.85), 0.18, 0.55,
             glow_mat("forge_barrel", (0.50, 0.30, 0.18), glow_strength=0.3, roughness=0.85), vertices=12)
    cylinder("water_top", (-0.85, 0.60, 0.85), 0.20, 0.04,
             glow_mat("forge_water", (0.30, 0.55, 0.85), glow_strength=0.4), vertices=10)

    export_glb("forge")


def make_chapel() -> None:
    clear_scene()
    stone = glow_mat("chap_stone", (0.95, 0.92, 0.85), glow_strength=0.55, roughness=0.85)
    stone_dark = glow_mat("chap_stone_dark", (0.55, 0.50, 0.42), glow_strength=0.2, roughness=0.9)
    roof = glow_mat("chap_roof", (0.30, 0.45, 0.65), glow_strength=0.3, roughness=0.85)
    wood = glow_mat("chap_wood", (0.50, 0.28, 0.12), glow_strength=0.2, roughness=0.9)
    glass = glow_mat("chap_glass", (1.0, 0.85, 0.40), glow_strength=1.4, roughness=0.3)
    cross = glow_mat("chap_cross", (0.95, 0.80, 0.30), glow_strength=0.9, roughness=0.4)

    cube("body", (0.0, 0.80, 0.0), (1.40, 1.60, 1.20), stone)
    cube("body_shadow", (0.0, 0.80, -0.30), (1.40, 1.60, 0.20), stone_dark)

    cube("door", (0.0, 0.45, 0.62), (0.30, 0.85, 0.04), wood)
    cube("door_arch", (0.0, 0.95, 0.62), (0.45, 0.30, 0.04), stone_dark)
    cube("door_panel_l", (-0.12, 0.45, 0.64), (0.12, 0.85, 0.02),
         glow_mat("chap_door_panel", (0.55, 0.30, 0.15), glow_strength=0.2, roughness=0.85))
    cube("door_panel_r", (0.12, 0.45, 0.64), (0.12, 0.85, 0.02),
         glow_mat("chap_door_panel", (0.55, 0.30, 0.15), glow_strength=0.2, roughness=0.85))
    cube("door_handle", (-0.06, 0.50, 0.66), (0.02, 0.08, 0.02),
         glow_mat("chap_door_handle", (0.85, 0.65, 0.20), glow_strength=0.7))

    cube("window_l", (-0.50, 1.20, 0.62), (0.30, 0.60, 0.04), glass)
    cube("window_r", (0.50, 1.20, 0.62), (0.30, 0.60, 0.04), glass)
    cube("win_x_l_h", (-0.50, 1.20, 0.64), (0.30, 0.04, 0.02), wood)
    cube("win_x_l_v", (-0.50, 1.20, 0.64), (0.04, 0.60, 0.02), wood)
    cube("win_x_r_h", (0.50, 1.20, 0.64), (0.30, 0.04, 0.02), wood)
    cube("win_x_r_v", (0.50, 1.20, 0.64), (0.04, 0.60, 0.02), wood)

    cone("roof_main", (0.0, 2.20, 0.0), 0.95, 0.0, 0.80, roof, vertices=4)
    cube("roof_eave", (0.0, 1.80, 0.0), (1.55, 0.06, 1.30), stone_dark)

    cube("tower", (0.0, 2.65, -0.20), (0.40, 0.90, 0.40), stone)
    cube("tower_top", (0.0, 3.20, -0.20), (0.50, 0.10, 0.50), stone_dark)
    cone("tower_roof", (0.0, 3.55, -0.20), 0.30, 0.0, 0.40, roof, vertices=4)

    cube("cross_v", (0.0, 3.95, -0.20), (0.06, 0.50, 0.06), cross)
    cube("cross_h", (0.0, 4.05, -0.20), (0.30, 0.06, 0.06), cross)

    cube("roof_cross_l", (-0.65, 2.10, 0.0), (0.10, 0.10, 1.10), stone_dark)
    cube("roof_cross_r", (0.65, 2.10, 0.0), (0.10, 0.10, 1.10), stone_dark)

    cube("base_step", (0.0, 0.10, 0.40), (0.50, 0.20, 0.30), stone_dark)
    cube("foundation", (0.0, 0.05, 0.0), (1.55, 0.10, 1.30), stone_dark)

    export_glb("chapel")


def make_pier() -> None:
    clear_scene()
    wood = glow_mat("pier_wood", (0.55, 0.30, 0.15), glow_strength=0.2, roughness=0.9)
    wood_dark = glow_mat("pier_wood_dark", (0.28, 0.15, 0.06), glow_strength=0.1, roughness=0.95)
    rope = glow_mat("pier_rope", (0.85, 0.70, 0.40), glow_strength=0.3, roughness=0.85)
    water = mat("pier_water", (0.30, 0.55, 0.85), roughness=0.3, alpha=0.7)
    barrel = glow_mat("pier_barrel", (0.50, 0.30, 0.15), glow_strength=0.2, roughness=0.85)
    lantern = glow_mat("pier_lantern", (1.0, 0.75, 0.30), glow_strength=1.5, roughness=0.3)

    cube("deck", (0.0, 0.20, 0.0), (2.50, 0.10, 1.20), wood)
    cube("deck_top", (0.0, 0.22, 0.0), (2.55, 0.04, 1.25), wood_dark)

    for px in [-1.05, -0.35, 0.35, 1.05]:
        cube("plank_under", (px, 0.12, 0.0), (0.15, 0.10, 1.20), wood_dark)

    for px in [-1.20, 0.0, 1.20]:
        for pz in [-0.40, 0.40]:
            cube("pile", (px, -0.60, pz), (0.12, 1.40, 0.12), wood_dark)

    cube("cross_x1", (0.0, -0.30, 0.0), (2.30, 0.10, 0.10), wood_dark)
    cube("cross_x2", (0.0, -0.70, 0.0), (2.30, 0.08, 0.08), wood_dark)
    cube("cross_z1", (-1.20, -0.30, 0.0), (0.10, 0.10, 0.80), wood_dark)
    cube("cross_z2", (1.20, -0.30, 0.0), (0.10, 0.10, 0.80), wood_dark)

    cylinder("water", (0.0, -0.20, 0.0), 1.20, 0.05, water, vertices=14)

    for px in [-0.90, 0.90]:
        cylinder("barrel", (px, 0.40, 0.30), 0.16, 0.40, barrel, vertices=10)
        cylinder("barrel_top", (px, 0.62, 0.30), 0.17, 0.03, wood_dark, vertices=10)

    cube("post_l", (-1.10, 0.65, -0.50), (0.10, 0.80, 0.10), wood)
    cube("post_r", (1.10, 0.65, -0.50), (0.10, 0.80, 0.10), wood)
    cube("beam", (0.0, 1.05, -0.50), (2.40, 0.10, 0.10), wood)

    for px in [-1.10, 1.10]:
        cube("lantern_body", (px, 0.95, -0.50), (0.12, 0.16, 0.12), wood_dark)
        cube("lantern_glow", (px, 0.95, -0.50), (0.10, 0.12, 0.10), lantern)
        cone("lantern_roof", (px, 1.10, -0.50), 0.10, 0.0, 0.08, wood_dark, vertices=4)

    for px in [-0.95, 0.0, 0.95]:
        cylinder("tie_rope", (px, 0.30, -0.55), 0.015, 0.50, rope, vertices=6)

    export_glb("pier")


def make_tavern() -> None:
    clear_scene()
    wall = glow_mat("tavern_wall", (0.75, 0.55, 0.35), glow_strength=0.3, roughness=0.9)
    wall_shadow = glow_mat("tavern_wall_shadow", (0.50, 0.35, 0.20), glow_strength=0.15, roughness=0.95)
    roof = glow_mat("tavern_roof", (0.55, 0.20, 0.18), glow_strength=0.4, roughness=0.8)
    roof_dark = glow_mat("tavern_roof_dark", (0.35, 0.10, 0.08), glow_strength=0.2, roughness=0.85)
    wood = glow_mat("tavern_wood", (0.45, 0.25, 0.10), glow_strength=0.2, roughness=0.9)
    window_glow = glow_mat("tavern_window", (1.0, 0.80, 0.40), glow_strength=1.4, roughness=0.4)
    sign = glow_mat("tavern_sign", (0.85, 0.65, 0.20), glow_strength=0.9, roughness=0.5)
    chimney = glow_mat("tavern_chimney", (0.50, 0.40, 0.40), glow_strength=0.2, roughness=0.95)
    smoke = mat("tavern_smoke", (0.85, 0.85, 0.90), roughness=0.95, alpha=0.55)

    cube("body", (0.0, 0.90, 0.0), (2.00, 1.80, 1.50), wall)
    cube("body_shadow", (0.0, 0.90, -0.40), (2.00, 1.80, 0.30), wall_shadow)

    cube("body2", (0.0, 2.40, -0.10), (1.80, 0.50, 1.40), wall)
    cube("body2_shadow", (0.0, 2.40, -0.50), (1.80, 0.50, 0.30), wall_shadow)

    cone("roof_main", (0.0, 2.10, 0.0), 1.40, 0.0, 0.90, roof, vertices=4)
    cube("roof_eave", (0.0, 1.65, 0.0), (2.20, 0.10, 1.65), roof_dark)
    cone("roof_top", (0.0, 3.20, -0.10), 1.25, 0.0, 0.70, roof, vertices=4)
    cube("roof_top_eave", (0.0, 2.80, -0.10), (2.00, 0.08, 1.50), roof_dark)

    cube("door", (0.0, 0.45, 0.76), (0.36, 0.85, 0.04), wood)
    cube("door_handle", (0.10, 0.50, 0.79), (0.04, 0.10, 0.04),
         glow_mat("tavern_handle", (0.85, 0.65, 0.20), glow_strength=0.7))
    cube("door_panel_l", (-0.10, 0.45, 0.78), (0.14, 0.85, 0.02),
         glow_mat("tavern_door_panel", (0.50, 0.28, 0.10), glow_strength=0.2, roughness=0.85))
    cube("door_panel_r", (0.10, 0.45, 0.78), (0.14, 0.85, 0.02),
         glow_mat("tavern_door_panel", (0.50, 0.28, 0.10), glow_strength=0.2, roughness=0.85))

    cube("window_fl", (-0.55, 1.20, 0.76), (0.32, 0.32, 0.04), window_glow)
    cube("window_fr", (0.55, 1.20, 0.76), (0.32, 0.32, 0.04), window_glow)
    cube("window_x_fl_h", (-0.55, 1.20, 0.78), (0.32, 0.04, 0.02), wood)
    cube("window_x_fl_v", (-0.55, 1.20, 0.78), (0.04, 0.32, 0.02), wood)
    cube("window_x_fr_h", (0.55, 1.20, 0.78), (0.32, 0.04, 0.02), wood)
    cube("window_x_fr_v", (0.55, 1.20, 0.78), (0.04, 0.32, 0.02), wood)

    cube("window2_l", (-0.65, 2.50, -0.10), (0.30, 0.30, 0.04), window_glow)
    cube("window2_r", (0.65, 2.50, -0.10), (0.30, 0.30, 0.04), window_glow)
    cube("window2_x_l_h", (-0.65, 2.50, -0.08), (0.30, 0.04, 0.02), wood)
    cube("window2_x_l_v", (-0.65, 2.50, -0.08), (0.04, 0.30, 0.02), wood)
    cube("window2_x_r_h", (0.65, 2.50, -0.08), (0.30, 0.04, 0.02), wood)
    cube("window2_x_r_v", (0.65, 2.50, -0.08), (0.04, 0.30, 0.02), wood)

    cube("chimney", (-0.65, 2.85, -0.10), (0.22, 0.80, 0.22), chimney)
    cube("chimney_top", (-0.65, 3.30, -0.10), (0.30, 0.06, 0.30), chimney)
    uv_sphere("smoke1", (-0.65, 3.60, -0.10), (0.20, 0.20, 0.20), smoke, 10, 7)
    uv_sphere("smoke2", (-0.60, 3.90, -0.10), (0.26, 0.26, 0.26), smoke, 10, 7)

    cube("post_l", (-0.90, 1.85, 0.85), (0.10, 1.20, 0.10), wood)
    cube("post_r", (0.90, 1.85, 0.85), (0.10, 1.20, 0.10), wood)
    cube("beam_sign", (0.0, 2.45, 0.85), (1.50, 0.10, 0.10), wood)
    cube("sign_board", (0.0, 2.30, 0.95), (1.10, 0.50, 0.05), sign)
    cube("sign_text", (0.0, 2.30, 0.98), (0.90, 0.30, 0.02),
         glow_mat("tavern_sign_text", (0.95, 0.55, 0.20), glow_strength=0.9))
    cube("sign_bottom", (0.0, 1.90, 0.88), (0.50, 0.20, 0.04), wood)

    for px in [-0.85, 0.85]:
        cube("post_base", (px, 0.05, 0.95), (0.20, 0.10, 0.20), wood)

    cube("foundation", (0.0, 0.05, 0.0), (2.10, 0.10, 1.60), wall_shadow)

    export_glb("tavern")


# ================================================================= CREATURES


def make_villager() -> None:
    clear_scene()
    skin = glow_mat("v_skin", (1.0, 0.85, 0.70), glow_strength=0.4, roughness=0.7)
    shirt = glow_mat("v_shirt", (0.45, 0.65, 0.35), glow_strength=0.4, roughness=0.8)
    shirt_dark = glow_mat("v_shirt_dark", (0.30, 0.45, 0.25), glow_strength=0.25, roughness=0.85)
    pants = glow_mat("v_pants", (0.40, 0.30, 0.20), glow_strength=0.3, roughness=0.85)
    hair = glow_mat("v_hair", (0.30, 0.18, 0.08), glow_strength=0.2, roughness=0.9)
    eye = glow_mat("v_eye", (0.10, 0.05, 0.04), glow_strength=0.3, roughness=0.4)
    apron = glow_mat("v_apron", (0.85, 0.75, 0.50), glow_strength=0.5, roughness=0.7)

    uv_sphere("head", (0.0, 1.35, 0.0), (0.18, 0.18, 0.18), skin, 12, 10)
    uv_sphere("hair_top", (0.0, 1.45, 0.0), (0.19, 0.10, 0.19), hair, 12, 8)
    cube("hair_bangs", (0.0, 1.42, 0.10), (0.20, 0.06, 0.06), hair)

    uv_sphere("eye_l", (-0.07, 1.36, 0.16), (0.025, 0.03, 0.02), eye, 8, 6)
    uv_sphere("eye_r", (0.07, 1.36, 0.16), (0.025, 0.03, 0.02), eye, 8, 6)
    uv_sphere("nose", (0.0, 1.30, 0.18), (0.02, 0.02, 0.02),
              glow_mat("v_nose", (0.95, 0.75, 0.65), glow_strength=0.3), 6, 5)
    uv_sphere("mouth", (0.0, 1.27, 0.17), (0.04, 0.015, 0.01),
              glow_mat("v_mouth", (0.85, 0.40, 0.45), glow_strength=0.4), 6, 4)

    cube("torso", (0.0, 0.90, 0.0), (0.32, 0.55, 0.22), shirt)
    cube("shirt_collar", (0.0, 1.18, 0.0), (0.18, 0.05, 0.18), shirt_dark)

    cube("arm_l", (-0.20, 0.85, 0.0), (0.08, 0.50, 0.10), shirt)
    cube("arm_r", (0.20, 0.85, 0.0), (0.08, 0.50, 0.10), shirt)
    uv_sphere("hand_l", (-0.20, 0.55, 0.0), (0.06, 0.08, 0.06), skin, 8, 6)
    uv_sphere("hand_r", (0.20, 0.55, 0.0), (0.06, 0.08, 0.06), skin, 8, 6)

    cube("leg_l", (-0.08, 0.30, 0.0), (0.10, 0.55, 0.12), pants)
    cube("leg_r", (0.08, 0.30, 0.0), (0.10, 0.55, 0.12), pants)
    cube("boot_l", (-0.08, 0.05, 0.03), (0.12, 0.10, 0.16),
         glow_mat("v_boot", (0.30, 0.20, 0.10), glow_strength=0.2, roughness=0.9))
    cube("boot_r", (0.08, 0.05, 0.03), (0.12, 0.10, 0.16),
         glow_mat("v_boot", (0.30, 0.20, 0.10), glow_strength=0.2, roughness=0.9))

    cube("apron_front", (0.0, 0.85, 0.12), (0.30, 0.45, 0.02), apron)
    cube("apron_strap_l", (-0.13, 1.12, 0.10), (0.04, 0.20, 0.04), apron)
    cube("apron_strap_r", (0.13, 1.12, 0.10), (0.04, 0.20, 0.04), apron)

    cube("basket", (0.30, 0.30, 0.10), (0.16, 0.20, 0.20),
         glow_mat("v_basket", (0.55, 0.35, 0.18), glow_strength=0.3, roughness=0.85))
    cube("basket_rim", (0.30, 0.42, 0.10), (0.18, 0.04, 0.22),
         glow_mat("v_basket_rim", (0.45, 0.28, 0.12), glow_strength=0.2, roughness=0.9))

    raise RuntimeError("villager is owned by build_sokpop_style_pass.py")


def make_wolf() -> None:
    clear_scene()
    fur = glow_mat("wolf_fur", (0.45, 0.45, 0.50), glow_strength=0.25, roughness=0.85)
    fur_dark = glow_mat("wolf_fur_dark", (0.25, 0.25, 0.30), glow_strength=0.15, roughness=0.9)
    belly = glow_mat("wolf_belly", (0.85, 0.82, 0.75), glow_strength=0.4, roughness=0.85)
    eye = glow_mat("wolf_eye", (0.95, 0.75, 0.10), glow_strength=0.7, roughness=0.4)
    nose = glow_mat("wolf_nose", (0.10, 0.05, 0.05), glow_strength=0.2, roughness=0.6)

    cube("body", (0.0, 0.50, 0.0), (0.85, 0.45, 0.35), fur)
    cube("body_belly", (0.0, 0.42, 0.0), (0.65, 0.18, 0.30), belly)
    cube("chest", (0.45, 0.55, 0.0), (0.20, 0.35, 0.30), fur_dark)

    uv_sphere("head", (0.55, 0.62, 0.0), (0.20, 0.20, 0.20), fur, 12, 10)
    cube("snout", (0.78, 0.55, 0.0), (0.18, 0.16, 0.16), fur_dark)
    cube("snout_tip", (0.88, 0.55, 0.0), (0.06, 0.06, 0.06), nose)
    cone("ear_l", (0.45, 0.85, 0.10), 0.05, 0.0, 0.12, fur, vertices=4)
    cone("ear_r", (0.45, 0.85, -0.10), 0.05, 0.0, 0.12, fur, vertices=4)

    uv_sphere("eye_l", (0.62, 0.68, 0.16), (0.04, 0.04, 0.04), eye, 8, 6)
    uv_sphere("eye_r", (0.62, 0.68, -0.16), (0.04, 0.04, 0.04), eye, 8, 6)

    cylinder("tail_base", (-0.50, 0.65, 0.0), 0.10, 0.40, fur, vertices=8)
    cone("tail_tip", (-0.70, 0.90, 0.0), 0.08, 0.0, 0.18, fur_dark, vertices=6)

    for px in [0.30, -0.20]:
        cube(f"leg_fl_{px}", (px, 0.20, 0.18), (0.10, 0.40, 0.10), fur)
        cube(f"leg_bl_{px}", (px, 0.20, -0.18), (0.10, 0.40, 0.10), fur)
        uv_sphere(f"paw_fl_{px}", (px, 0.02, 0.18), (0.07, 0.04, 0.10), fur_dark, 8, 5)
        uv_sphere(f"paw_bl_{px}", (px, 0.02, -0.18), (0.07, 0.04, 0.10), fur_dark, 8, 5)

    cube("leg_fr", (0.30, 0.20, 0.0), (0.10, 0.40, 0.10), fur)
    cube("leg_br", (-0.20, 0.20, 0.0), (0.10, 0.40, 0.10), fur)
    cube("paw_fr", (0.30, 0.02, 0.0), (0.12, 0.04, 0.10), fur_dark)
    cube("paw_br", (-0.20, 0.02, 0.0), (0.12, 0.04, 0.10), fur_dark)

    for ang in [-0.15, 0.15]:
        cube("stripe", (0.55, 0.78, ang), (0.16, 0.02, 0.04), fur_dark)

    export_glb("wolf")


def make_bear() -> None:
    clear_scene()
    fur = glow_mat("bear_fur", (0.45, 0.28, 0.18), glow_strength=0.3, roughness=0.85)
    fur_dark = glow_mat("bear_fur_dark", (0.30, 0.18, 0.10), glow_strength=0.15, roughness=0.9)
    belly = glow_mat("bear_belly", (0.75, 0.55, 0.40), glow_strength=0.4, roughness=0.85)
    muzzle = glow_mat("bear_muzzle", (0.85, 0.70, 0.55), glow_strength=0.4, roughness=0.8)
    eye = glow_mat("bear_eye", (0.10, 0.05, 0.02), glow_strength=0.3, roughness=0.5)
    nose = glow_mat("bear_nose", (0.10, 0.04, 0.04), glow_strength=0.2, roughness=0.6)

    uv_sphere("body_main", (0.0, 0.75, 0.0), (0.55, 0.50, 0.45), fur, 14, 10)
    uv_sphere("body_chest", (0.25, 0.60, 0.0), (0.32, 0.32, 0.30), belly, 12, 8)
    uv_sphere("body_belly", (-0.10, 0.50, 0.0), (0.32, 0.20, 0.30), belly, 12, 8)

    uv_sphere("head", (0.55, 0.95, 0.0), (0.30, 0.30, 0.30), fur, 14, 10)
    uv_sphere("muzzle", (0.85, 0.80, 0.0), (0.18, 0.15, 0.18), muzzle, 12, 8)
    uv_sphere("nose_tip", (0.96, 0.85, 0.0), (0.05, 0.05, 0.05), nose, 8, 6)

    uv_sphere("eye_l", (0.65, 1.00, 0.18), (0.04, 0.05, 0.04), eye, 8, 6)
    uv_sphere("eye_r", (0.65, 1.00, -0.18), (0.04, 0.05, 0.04), eye, 8, 6)

    cone("ear_l", (0.45, 1.22, 0.18), 0.07, 0.0, 0.10, fur_dark, vertices=4)
    cone("ear_r", (0.45, 1.22, -0.18), 0.07, 0.0, 0.10, fur_dark, vertices=4)

    cube("leg_fl", (0.25, 0.20, 0.28), (0.18, 0.40, 0.18), fur)
    cube("leg_fr", (0.25, 0.20, -0.28), (0.18, 0.40, 0.18), fur)
    cube("leg_bl", (-0.20, 0.20, 0.28), (0.18, 0.40, 0.18), fur)
    cube("leg_br", (-0.20, 0.20, -0.28), (0.18, 0.40, 0.18), fur)
    uv_sphere("paw_fl", (0.25, 0.02, 0.32), (0.10, 0.05, 0.12), fur_dark, 8, 5)
    uv_sphere("paw_fr", (0.25, 0.02, -0.32), (0.10, 0.05, 0.12), fur_dark, 8, 5)
    uv_sphere("paw_bl", (-0.20, 0.02, 0.32), (0.10, 0.05, 0.12), fur_dark, 8, 5)
    uv_sphere("paw_br", (-0.20, 0.02, -0.32), (0.10, 0.05, 0.12), fur_dark, 8, 5)

    uv_sphere("tail", (-0.55, 0.65, 0.0), (0.10, 0.10, 0.10), fur_dark, 10, 6)

    cube("claw_l1", (0.34, 0.05, 0.42), (0.04, 0.05, 0.05), fur_dark)
    cube("claw_l2", (0.25, 0.05, 0.42), (0.04, 0.05, 0.05), fur_dark)
    cube("claw_l3", (0.16, 0.05, 0.42), (0.04, 0.05, 0.05), fur_dark)

    export_glb("bear")


# --------------------------------------------------------------------- MAIN


def main() -> None:
    out_dir = models_lib.OUT_DIR
    out_dir.mkdir(parents=True, exist_ok=True)
    print(f"=== building 15 more models -> {out_dir} ===")

    print("\n--- decor (8) ---")
    print("[1/15] campfire")
    make_campfire()
    print("[2/15] lantern_post")
    make_lantern_post()
    print("[3/15] crate")
    make_crate()
    print("[4/15] barrel")
    make_barrel()
    print("[5/15] signpost")
    make_signpost()
    print("[6/15] market_stall")
    make_market_stall()
    print("[7/15] bench")
    make_bench()
    print("[8/15] fountain")
    make_fountain()
    print("[9/15] statue")
    make_statue()

    print("\n--- buildings v3 (4) ---")
    print("[10/15] forge")
    make_forge()
    print("[11/15] chapel")
    make_chapel()
    print("[12/15] pier")
    make_pier()
    print("[13/15] tavern")
    make_tavern()

    print("\n--- creatures (3) ---")
    print("[14/15] wolf + bear")
    make_wolf()
    make_bear()

    print("\n=== done: 16 new .glb files ===")


if __name__ == "__main__":
    main()
