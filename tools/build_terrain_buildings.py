"""Build terrain & building models for lastkingdom2 — v2 (bigger, more
topographic, explicit y=0 ground anchor).

Style: v6-balanced cute (candy colors + emissive + smooth spheres).
Output: assets/procedural/pretty/

Run:
    & "F:\\BLENDER\\blender-launcher.exe" --background --python tools\\build_terrain_buildings.py
"""
from __future__ import annotations

import json
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


# ================================================================= TERRAIN v2
# Anchor convention: every model places its footprint on y=0. The bottom of
# any cube/cone/cylinder is centered on the ground plane. This way
# follow_audit_ring's ground_top + 0 = model footprint sits on the local
# ground.


def make_mountain_snow() -> None:
    clear_scene()
    stone = glow_mat("mtn_stone", (0.55, 0.55, 0.65), glow_strength=0.18, roughness=0.95)
    stone_dark = glow_mat("mtn_stone_dark", (0.32, 0.30, 0.36), glow_strength=0.12, roughness=0.95)
    snow = glow_mat("mtn_snow", (1.0, 0.98, 1.0), glow_strength=0.55, roughness=0.5)
    snow_pink = glow_mat("mtn_snow_pink", (1.0, 0.85, 0.95), glow_strength=0.7, roughness=0.5)
    ice = mat_only("mtn_ice", (0.85, 0.95, 1.0))

    cone("base_main", (0.0, 1.00, 0.0), 2.10, 0.0, 2.00, stone, vertices=12)
    cone("base_l", (-1.30, 0.65, 0.40), 1.20, 0.0, 1.30, stone_dark, vertices=10)
    cone("base_r", (1.20, 0.55, -0.50), 1.20, 0.0, 1.10, stone, vertices=10)
    cone("base_back", (0.10, 0.55, -1.20), 1.15, 0.0, 1.10, stone_dark, vertices=10)
    cone("base_front", (-0.20, 0.50, 1.20), 1.10, 0.0, 1.00, stone, vertices=10)

    cone("mid_l", (-0.60, 1.80, 0.30), 0.75, 0.0, 1.20, snow, vertices=10)
    cone("mid_r", (0.65, 1.80, -0.20), 0.70, 0.0, 1.15, snow, vertices=10)
    cone("mid_back", (0.10, 1.85, -0.55), 0.65, 0.0, 1.10, snow, vertices=10)
    cone("mid_front", (-0.20, 1.80, 0.60), 0.65, 0.0, 1.10, snow, vertices=10)

    cone("peak", (0.0, 2.95, 0.0), 0.45, 0.0, 1.10, snow, vertices=10)
    uv_sphere("peak_glow", (0.0, 3.55, 0.0), (0.25, 0.18, 0.25), snow_pink, 12, 8)

    ico_sphere("float_l", (-1.05, 0.30, 0.85), (0.35, 0.25, 0.35), stone_dark, subdivisions=1)
    ico_sphere("float_r", (1.10, 0.20, 0.95), (0.30, 0.22, 0.30), stone, subdivisions=1)
    ico_sphere("float_back", (0.95, 0.25, -1.15), (0.32, 0.20, 0.32), stone_dark, subdivisions=1)

    cube("snow_lip_l", (-0.85, 2.30, 0.0), (0.50, 0.18, 0.95), snow)
    cube("snow_lip_r", (0.95, 2.30, 0.0), (0.50, 0.18, 0.95), snow)
    cube("snow_lip_front", (0.0, 2.30, 1.05), (0.95, 0.18, 0.40), snow)
    cube("snow_lip_back", (0.0, 2.30, -1.05), (0.95, 0.18, 0.40), snow)

    cone("ice_l", (-0.95, 0.20, 1.45), 0.10, 0.0, 0.40, ice, vertices=6)
    cone("ice_r", (1.05, 0.20, 1.50), 0.10, 0.0, 0.40, ice, vertices=6)

    export_glb("mountain_snow")


def make_volcano() -> None:
    clear_scene()
    rock = glow_mat("vol_rock", (0.40, 0.28, 0.25), glow_strength=0.18, roughness=0.95)
    rock_dark = glow_mat("vol_rock_dark", (0.20, 0.12, 0.10), glow_strength=0.10, roughness=0.95)
    lava = glow_mat("vol_lava", (1.0, 0.30, 0.05), glow_strength=1.2, roughness=0.5)
    lava_hot = glow_mat("vol_lava_hot", (1.0, 0.85, 0.20), glow_strength=1.3, roughness=0.3)
    smoke = mat("vol_smoke", (0.50, 0.45, 0.50), roughness=0.95, alpha=0.55)
    ash = mat_only("vol_ash", (0.20, 0.18, 0.18))

    cone("body_main", (0.0, 1.20, 0.0), 1.90, 0.0, 2.40, rock, vertices=14)
    cone("body_l", (-1.20, 0.80, 0.30), 1.10, 0.0, 1.60, rock_dark, vertices=10)
    cone("body_r", (1.15, 0.80, -0.30), 1.10, 0.0, 1.60, rock, vertices=10)
    cone("body_back", (0.0, 0.80, -1.15), 1.05, 0.0, 1.60, rock_dark, vertices=10)
    cone("body_front", (0.0, 0.80, 1.20), 1.05, 0.0, 1.60, rock, vertices=10)

    cone("crater_rim", (0.0, 2.45, 0.0), 0.95, 0.65, 0.30, rock_dark, vertices=12)
    cone("crater_lava", (0.0, 2.45, 0.0), 0.78, 0.55, 0.20, lava, vertices=12)
    uv_sphere("lava_pit", (0.0, 2.40, 0.0), (0.58, 0.18, 0.58), lava_hot, 14, 8)

    cone("stream1", (0.70, 0.30, 0.85), 0.20, 0.06, 0.85, lava, vertices=6)
    cone("stream2", (-0.85, 0.30, -0.70), 0.20, 0.06, 0.85, lava, vertices=6)
    cone("stream3", (0.85, 0.30, 0.0), 0.16, 0.05, 0.70, lava_hot, vertices=6)
    cone("stream4", (-0.30, 0.25, 0.95), 0.16, 0.05, 0.70, lava, vertices=6)

    uv_sphere("smoke1", (-0.30, 3.20, 0.20), (0.45, 0.45, 0.45), smoke, 12, 8)
    uv_sphere("smoke2", (0.40, 3.70, -0.15), (0.55, 0.50, 0.55), smoke, 12, 8)
    uv_sphere("smoke3", (0.05, 4.20, 0.10), (0.65, 0.55, 0.65), smoke, 12, 8)
    uv_sphere("smoke4", (-0.20, 4.65, 0.20), (0.50, 0.40, 0.50), smoke, 10, 7)

    ico_sphere("ash1", (1.85, 0.10, 1.10), (0.30, 0.18, 0.30), ash, subdivisions=1)
    ico_sphere("ash2", (-1.95, 0.10, -1.05), (0.32, 0.20, 0.32), ash, subdivisions=1)

    export_glb("volcano")


def make_desert_dune() -> None:
    clear_scene()
    sand = glow_mat("dune_sand", (1.0, 0.85, 0.50), glow_strength=0.40, roughness=0.95)
    sand_shadow = glow_mat("dune_sand_shadow", (0.80, 0.55, 0.25), glow_strength=0.20, roughness=0.95)
    cactus = glow_mat("dune_cactus", (0.25, 0.55, 0.25), glow_strength=0.4, roughness=0.9)
    cactus_dark = glow_mat("dune_cactus_dark", (0.15, 0.40, 0.15), glow_strength=0.3, roughness=0.9)
    cactus_flower = glow_mat("dune_cactus_flower", (1.0, 0.45, 0.70), glow_strength=0.9, roughness=0.5)
    rock_d = mat_only("dune_rock", (0.70, 0.50, 0.30))

    cone("dune_main", (0.0, 0.50, 0.0), 2.40, 0.20, 1.00, sand, vertices=16)
    cone("dune_l", (-1.65, 0.30, 0.45), 1.20, 0.10, 0.60, sand_shadow, vertices=14)
    cone("dune_r", (1.50, 0.35, -0.55), 1.35, 0.10, 0.70, sand, vertices=14)
    cone("dune_back", (-0.20, 0.30, -1.40), 1.20, 0.10, 0.60, sand_shadow, vertices=14)
    cone("dune_front", (0.20, 0.30, 1.30), 1.20, 0.10, 0.60, sand, vertices=14)

    cone("dune_peak1", (0.45, 1.05, -0.10), 0.50, 0.0, 0.40, sand, vertices=12)
    cone("dune_peak2", (-1.40, 0.75, 0.30), 0.40, 0.0, 0.35, sand_shadow, vertices=12)
    cone("dune_peak3", (1.35, 0.85, -0.30), 0.40, 0.0, 0.35, sand, vertices=12)

    cylinder("cactus_trunk", (1.10, 0.85, 1.20), 0.18, 1.70, cactus, vertices=10)
    cylinder("cactus_arm_l", (0.85, 0.95, 1.20), 0.10, 0.55, cactus_dark, vertices=8)
    cylinder("cactus_arm_r", (1.30, 1.00, 1.20), 0.10, 0.65, cactus, vertices=8)
    cone("cactus_arm_l_top", (0.85, 1.30, 1.20), 0.08, 0.0, 0.20, cactus_dark, vertices=6)
    cone("cactus_arm_r_top", (1.30, 1.35, 1.20), 0.08, 0.0, 0.20, cactus, vertices=6)

    uv_sphere("cactus_flower1", (0.85, 1.50, 1.20), (0.08, 0.08, 0.08), cactus_flower, 8, 6)
    uv_sphere("cactus_flower2", (1.10, 1.78, 1.20), (0.10, 0.10, 0.10), cactus_flower, 8, 6)
    uv_sphere("cactus_flower3", (1.30, 1.50, 1.20), (0.08, 0.08, 0.08), cactus_flower, 8, 6)

    cylinder("cactus2", (-1.45, 0.50, -1.30), 0.13, 1.00, cactus_dark, vertices=8)
    cylinder("cactus2_arm", (-1.65, 0.70, -1.30), 0.08, 0.40, cactus, vertices=6)
    uv_sphere("cactus2_flower", (-1.45, 1.10, -1.30), (0.06, 0.06, 0.06), cactus_flower, 8, 5)

    ico_sphere("rock1", (2.10, 0.15, 1.85), (0.25, 0.15, 0.25), rock_d, subdivisions=1)
    ico_sphere("rock2", (-2.05, 0.12, 1.65), (0.20, 0.12, 0.20), rock_d, subdivisions=1)

    export_glb("desert_dune")


def make_lake() -> None:
    clear_scene()
    water = glow_mat("lake_water", (0.30, 0.65, 0.95), glow_strength=0.6, roughness=0.3)
    water_deep = glow_mat("lake_water_deep", (0.10, 0.30, 0.65), glow_strength=0.4, roughness=0.4)
    grass = glow_mat("lake_grass", (0.45, 0.85, 0.40), glow_strength=0.4, roughness=0.95)
    grass_dark = glow_mat("lake_grass_dark", (0.25, 0.60, 0.25), glow_strength=0.3, roughness=0.95)
    reed = glow_mat("lake_reed", (0.65, 0.55, 0.25), glow_strength=0.3, roughness=0.9)
    lily = glow_mat("lake_lily", (0.95, 0.65, 0.80), glow_strength=0.55, roughness=0.7)

    cylinder("lake", (0.0, 0.04, 0.0), 1.80, 0.08, water, vertices=24)
    cylinder("lake_inner", (0.0, 0.02, 0.0), 1.20, 0.06, water_deep, vertices=20)
    cylinder("lake_ring", (0.0, 0.07, 0.0), 1.85, 0.02, water_deep, vertices=24)

    for i in range(12):
        a = (i / 12.0) * math.tau
        x = math.cos(a) * 2.05
        z = math.sin(a) * 2.05
        ico_sphere(f"grass_{i}", (x, 0.18, z), (0.30, 0.18, 0.30),
                   grass if i % 2 == 0 else grass_dark, subdivisions=1)

    for sx, sz in [(-1.75, 0.15), (1.80, -0.10), (-0.35, -1.85), (0.50, 1.75)]:
        cylinder(f"reed_stem_{sx}", (sx, 0.55, sz), 0.025, 1.00, reed, vertices=6)
        uv_sphere(f"reed_top_{sx}", (sx, 1.10, sz), (0.07, 0.18, 0.07), reed, 6, 5)

    for sx, sz in [(0.65, 0.65), (-0.55, 0.85), (0.85, -0.55), (-0.75, -0.40)]:
        uv_sphere(f"lily_pad_{sx}", (sx, 0.10, sz), (0.20, 0.04, 0.20), grass_dark, 10, 5)
        uv_sphere(f"lily_flower_{sx}", (sx, 0.13, sz), (0.07, 0.07, 0.07), lily, 8, 6)

    export_glb("lake")


def make_swamp() -> None:
    clear_scene()
    water = glow_mat("swamp_water", (0.20, 0.35, 0.25), glow_strength=0.4, roughness=0.6)
    water_dark = glow_mat("swamp_water_dark", (0.08, 0.18, 0.12), glow_strength=0.2, roughness=0.7)
    moss = glow_mat("swamp_moss", (0.20, 0.55, 0.18), glow_strength=0.4, roughness=0.95)
    wood = glow_mat("swamp_wood", (0.35, 0.22, 0.12), glow_strength=0.15, roughness=0.95)
    wood_dark = glow_mat("swamp_wood_dark", (0.20, 0.12, 0.06), glow_strength=0.1, roughness=0.95)
    mush_cap = glow_mat("swamp_mushroom_cap", (0.95, 0.40, 0.55), glow_strength=0.8, roughness=0.6)
    mush_stem = glow_mat("swamp_mushroom_stem", (0.95, 0.88, 0.78), glow_strength=0.4, roughness=0.7)

    cylinder("swamp_pool", (0.0, 0.04, 0.0), 1.65, 0.08, water, vertices=22)
    cylinder("swamp_deep", (0.15, 0.02, -0.15), 1.00, 0.06, water_dark, vertices=18)

    cylinder("stump1", (-1.30, 0.45, 0.95), 0.18, 0.90, wood, vertices=10)
    cone("stump1_top", (-1.30, 0.95, 0.95), 0.20, 0.18, 0.10, wood_dark, vertices=8)
    cylinder("branch1a", (-1.55, 0.95, 0.95), 0.06, 0.55, wood_dark, vertices=6)
    cylinder("branch1b", (-1.30, 1.20, 0.70), 0.05, 0.40, wood, vertices=6)

    cylinder("stump2", (1.45, 0.55, -0.85), 0.22, 1.10, wood_dark, vertices=10)
    cone("stump2_top", (1.45, 1.15, -0.85), 0.24, 0.22, 0.10, wood, vertices=8)
    cylinder("branch2a", (1.65, 1.20, -0.65), 0.06, 0.55, wood_dark, vertices=6)

    cylinder("stump3", (0.20, 0.30, 1.60), 0.14, 0.60, wood_dark, vertices=8)

    for i, (x, z, scale) in enumerate([
        (0.85, 1.10, 0.30), (-0.95, -1.20, 0.32), (1.30, 0.85, 0.28),
        (-1.10, 1.30, 0.30), (1.60, 1.20, 0.26),
    ]):
        ico_sphere(f"moss_{i}", (x, 0.22, z), (scale, 0.18, scale * 0.9), moss, subdivisions=1)

    cylinder("mush_stem1", (0.0, 0.20, 1.20), 0.05, 0.40, mush_stem, vertices=6)
    cone("mush_cap1", (0.0, 0.45, 1.20), 0.18, 0.0, 0.18, mush_cap, vertices=10)
    uv_sphere("mush_glow1", (0.0, 0.55, 1.20), (0.10, 0.06, 0.10), mush_cap, 10, 6)

    cylinder("mush_stem2", (-0.65, 0.18, 0.50), 0.04, 0.36, mush_stem, vertices=6)
    cone("mush_cap2", (-0.65, 0.40, 0.50), 0.13, 0.0, 0.13, mush_cap, vertices=10)
    uv_sphere("mush_glow2", (-0.65, 0.48, 0.50), (0.08, 0.05, 0.08), mush_cap, 10, 5)

    export_glb("swamp")


def make_cliff() -> None:
    clear_scene()
    stone = glow_mat("cliff_stone", (0.65, 0.58, 0.50), glow_strength=0.22, roughness=0.95)
    stone_mid = glow_mat("cliff_stone_mid", (0.45, 0.40, 0.35), glow_strength=0.15, roughness=0.95)
    stone_dark = glow_mat("cliff_stone_dark", (0.28, 0.24, 0.22), glow_strength=0.10, roughness=0.95)
    stone_red = glow_mat("cliff_stone_red", (0.75, 0.45, 0.35), glow_strength=0.25, roughness=0.95)
    moss = glow_mat("cliff_moss", (0.30, 0.70, 0.30), glow_strength=0.45, roughness=0.9)
    grass = glow_mat("cliff_grass", (0.50, 0.85, 0.40), glow_strength=0.4, roughness=0.95)

    cube("layer1", (0.0, 0.25, 0.0), (2.40, 0.50, 1.80), stone_dark)
    cube("layer2", (0.0, 0.70, 0.0), (2.20, 0.40, 1.60), stone_mid)
    cube("layer3", (0.0, 1.10, 0.0), (2.00, 0.40, 1.40), stone_red)
    cube("layer4", (0.0, 1.50, 0.0), (1.70, 0.40, 1.20), stone_mid)
    cube("layer5", (0.0, 1.90, 0.0), (1.40, 0.40, 1.00), stone)
    cube("layer6", (0.0, 2.30, 0.0), (1.00, 0.40, 0.70), stone_mid)

    cube("ledge_l", (-0.70, 2.55, 0.0), (0.50, 0.10, 0.85), stone)
    cube("ledge_r", (0.70, 2.55, 0.10), (0.45, 0.10, 0.75), stone_mid)

    cube("protrude_l", (-1.30, 0.50, 0.30), (0.30, 0.20, 0.30), stone)
    cube("protrude_r", (1.35, 0.90, -0.30), (0.30, 0.20, 0.30), stone_dark)
    cube("protrude_front", (0.0, 0.40, 0.95), (0.45, 0.20, 0.15), stone_mid)

    for x, z, sx, sz in [
        (-0.95, 0.55, 0.30, 0.25), (1.05, 0.45, 0.28, 0.22),
        (-0.55, -0.55, 0.25, 0.20), (0.75, -0.65, 0.30, 0.25),
        (0.0, 0.85, 0.22, 0.18), (0.0, -0.85, 0.28, 0.22),
    ]:
        uv_sphere("moss_patch", (x, 1.85, z), (sx, 0.10, sz), moss, 10, 6)

    uv_sphere("grass_tuft1", (-0.45, 2.65, 0.30), (0.18, 0.14, 0.18), grass, 10, 6)
    uv_sphere("grass_tuft2", (0.55, 2.65, 0.10), (0.20, 0.14, 0.20), grass, 10, 6)
    uv_sphere("grass_tuft3", (0.10, 2.70, -0.30), (0.16, 0.12, 0.16), grass, 10, 6)

    cone("peak", (0.0, 2.65, 0.0), 0.20, 0.0, 0.30, stone, vertices=6)

    export_glb("cliff")


def make_cave_entrance() -> None:
    clear_scene()
    stone = glow_mat("cave_stone", (0.50, 0.46, 0.42), glow_strength=0.18, roughness=0.95)
    stone_dark = glow_mat("cave_stone_dark", (0.28, 0.24, 0.22), glow_strength=0.10, roughness=0.95)
    moss = glow_mat("cave_moss", (0.30, 0.70, 0.30), glow_strength=0.45, roughness=0.9)
    glow = glow_mat("cave_inner_glow", (1.0, 0.55, 0.18), glow_strength=1.4, roughness=0.3)
    glow_hot = glow_mat("cave_inner_glow_hot", (1.0, 0.85, 0.30), glow_strength=1.6, roughness=0.2)
    crystal = glow_mat("cave_crystal", (0.55, 0.85, 1.0), glow_strength=1.2, roughness=0.2)
    crystal_pink = glow_mat("cave_crystal_pink", (1.0, 0.55, 0.85), glow_strength=1.1, roughness=0.2)

    cube("base", (0.0, 0.10, 0.0), (2.20, 0.20, 1.80), stone_dark)

    cube("arch_l", (-0.85, 0.75, 0.0), (0.50, 1.10, 0.95), stone)
    cube("arch_r", (0.85, 0.75, 0.0), (0.50, 1.10, 0.95), stone)
    cube("arch_top", (0.0, 1.55, 0.0), (1.90, 0.50, 0.95), stone)

    cube("arch_top_l", (-0.70, 1.95, 0.0), (0.40, 0.30, 0.85), stone_dark)
    cube("arch_top_r", (0.70, 1.95, 0.0), (0.40, 0.30, 0.85), stone_dark)

    cone("opening", (0.0, 0.70, 0.30), 0.55, 0.40, 1.10, stone_dark, vertices=10)
    cone("opening_inner", (0.0, 0.70, 0.30), 0.40, 0.30, 0.85,
         mat("cave_void", (0.02, 0.01, 0.01), roughness=0.95), vertices=10)

    uv_sphere("inner_glow", (0.0, 0.65, 0.20), (0.25, 0.35, 0.10), glow, 10, 8)
    uv_sphere("inner_glow_hot", (0.0, 0.65, 0.05), (0.10, 0.18, 0.06), glow_hot, 10, 8)

    cone("crystal_l", (-0.55, 0.40, 0.65), 0.08, 0.0, 0.45, crystal, vertices=6)
    cone("crystal_r", (0.55, 0.40, 0.65), 0.10, 0.0, 0.55, crystal, vertices=6)
    cone("crystal_mid", (0.0, 0.35, 0.70), 0.08, 0.0, 0.40, crystal_pink, vertices=6)
    cone("crystal_back_l", (-0.30, 0.25, 0.75), 0.06, 0.0, 0.30, crystal_pink, vertices=6)

    for x, z, scale in [
        (-1.20, 0.55, 0.25), (1.30, 0.65, 0.28), (-1.05, -0.65, 0.30),
        (1.10, -0.70, 0.28), (0.0, -0.95, 0.32),
    ]:
        ico_sphere("moss", (x, 0.20, z), (scale, 0.18, scale * 0.9), moss, subdivisions=1)

    ico_sphere("stone1", (-1.55, 0.30, 0.50), (0.30, 0.20, 0.30), stone_dark, subdivisions=1)
    ico_sphere("stone2", (1.55, 0.30, 0.55), (0.32, 0.20, 0.32), stone, subdivisions=1)

    export_glb("cave_entrance")


def make_beach() -> None:
    clear_scene()
    sand = glow_mat("beach_sand", (1.0, 0.92, 0.65), glow_strength=0.5, roughness=0.95)
    sand_wet = glow_mat("beach_sand_wet", (0.80, 0.70, 0.45), glow_strength=0.3, roughness=0.95)
    water = mat("beach_water", (0.30, 0.65, 0.95), roughness=0.3, alpha=0.7)
    foam = glow_mat("beach_foam", (0.95, 1.0, 1.0), glow_strength=0.7, roughness=0.6)
    shell = glow_mat("beach_shell", (1.0, 0.60, 0.65), glow_strength=0.7, roughness=0.5)
    palm_trunk = glow_mat("beach_palm_trunk", (0.45, 0.28, 0.15), glow_strength=0.2, roughness=0.95)
    palm_leaf = glow_mat("beach_palm_leaf", (0.30, 0.75, 0.30), glow_strength=0.4, roughness=0.95)

    cylinder("sand_dry", (0.0, 0.06, 0.85), 1.85, 0.12, sand, vertices=24)
    cylinder("sand_wet", (0.0, 0.05, -0.10), 1.75, 0.10, sand_wet, vertices=22)
    cylinder("water", (0.0, 0.04, -1.20), 1.55, 0.08, water, vertices=20)

    for i in range(14):
        a = (i / 14.0) * math.tau
        x = math.cos(a) * 1.60
        z = math.sin(a) * 1.60 - 0.80
        uv_sphere(f"foam_{i}", (x, 0.11, z), (0.10, 0.05, 0.10), foam, 8, 5)

    cylinder("palm", (-0.90, 1.00, 0.95), 0.10, 2.00, palm_trunk, vertices=8)

    for i, y in enumerate([1.00, 1.30, 1.60, 1.85]):
        cone(f"palm_lean_{i}", (-0.90 + i * 0.05, y, 0.95 + i * 0.03),
             0.14, 0.07, 0.24, palm_trunk, vertices=6)

    for ang in [0.0, 1.2, 2.4, 3.6, 4.8]:
        x = -0.90 + math.cos(ang) * 0.45
        z = 0.95 + math.sin(ang) * 0.45
        cone(f"palm_leaf_{ang}", (x, 2.05, z), 0.25, 0.0, 0.12, palm_leaf, vertices=4)

    cone("palm_top", (-0.90, 2.25, 0.95), 0.15, 0.0, 0.12, palm_leaf, vertices=6)
    cone("coconut1", (-1.05, 1.95, 1.05), 0.06, 0.0, 0.08,
         glow_mat("beach_coconut", (0.55, 0.35, 0.20), glow_strength=0.3), vertices=6)
    cone("coconut2", (-0.75, 1.85, 1.10), 0.06, 0.0, 0.08,
         glow_mat("beach_coconut", (0.55, 0.35, 0.20), glow_strength=0.3), vertices=6)

    cone("shell1", (0.85, 0.18, 0.70), 0.16, 0.0, 0.12, shell, vertices=8)
    uv_sphere("shell1_dot", (0.85, 0.28, 0.70), (0.06, 0.06, 0.06),
              glow_mat("beach_shell_dot", (1.0, 1.0, 0.85), glow_strength=0.95), 8, 5)
    cone("shell2", (1.20, 0.15, 0.30), 0.13, 0.0, 0.10, shell, vertices=6)

    export_glb("beach")


# =============================================================== BUILDINGS v2


def make_house_small() -> None:
    clear_scene()
    wall = glow_mat("house_wall", (0.95, 0.85, 0.60), glow_strength=0.40, roughness=0.85)
    wall_shadow = glow_mat("house_wall_shadow", (0.70, 0.58, 0.40), glow_strength=0.25, roughness=0.9)
    roof = glow_mat("house_roof", (0.85, 0.25, 0.25), glow_strength=0.5, roughness=0.7)
    roof_dark = glow_mat("house_roof_dark", (0.55, 0.15, 0.15), glow_strength=0.3, roughness=0.7)
    door = glow_mat("house_door", (0.45, 0.25, 0.12), glow_strength=0.3, roughness=0.85)
    window_glow = glow_mat("house_window", (1.0, 0.85, 0.40), glow_strength=1.2, roughness=0.4)
    chimney = glow_mat("house_chimney", (0.45, 0.40, 0.40), glow_strength=0.2, roughness=0.95)
    smoke = mat("house_smoke", (0.85, 0.85, 0.90), roughness=0.95, alpha=0.55)

    cube("body", (0.0, 0.85, 0.0), (1.60, 1.70, 1.30), wall)
    cube("body_shadow", (0.0, 0.85, -0.35), (1.60, 1.70, 0.30), wall_shadow)

    cube("door", (0.0, 0.45, 0.66), (0.30, 0.85, 0.04), door)
    uv_sphere("door_handle", (0.10, 0.50, 0.69), (0.04, 0.04, 0.04),
              glow_mat("door_handle", (0.95, 0.80, 0.30), glow_strength=0.7), 8, 5)

    cube("window_l", (-0.50, 1.05, 0.66), (0.32, 0.32, 0.04), window_glow)
    cube("window_r", (0.50, 1.05, 0.66), (0.32, 0.32, 0.04), window_glow)
    cube("window_cross_l_h", (-0.50, 1.05, 0.69), (0.32, 0.04, 0.02),
         mat("house_window_x", (0.30, 0.18, 0.10), roughness=0.85))
    cube("window_cross_l_v", (-0.50, 1.05, 0.69), (0.04, 0.32, 0.02),
         mat("house_window_x", (0.30, 0.18, 0.10), roughness=0.85))
    cube("window_cross_r_h", (0.50, 1.05, 0.69), (0.32, 0.04, 0.02),
         mat("house_window_x", (0.30, 0.18, 0.10), roughness=0.85))
    cube("window_cross_r_v", (0.50, 1.05, 0.69), (0.04, 0.32, 0.02),
         mat("house_window_x", (0.30, 0.18, 0.10), roughness=0.85))

    cone("roof_main", (0.0, 2.20, 0.0), 1.10, 0.0, 1.00, roof, vertices=4)
    cube("roof_skirting", (0.0, 1.78, 0.0), (1.70, 0.10, 1.40), roof_dark)

    cube("chimney", (0.55, 2.20, -0.20), (0.20, 0.65, 0.20), chimney)
    cube("chimney_top", (0.55, 2.55, -0.20), (0.26, 0.06, 0.26), chimney)
    uv_sphere("smoke1", (0.55, 2.85, -0.20), (0.18, 0.18, 0.18), smoke, 10, 7)
    uv_sphere("smoke2", (0.65, 3.15, -0.20), (0.22, 0.22, 0.22), smoke, 10, 7)
    uv_sphere("smoke3", (0.50, 3.45, -0.20), (0.26, 0.26, 0.26), smoke, 10, 7)

    cube("base_step", (0.0, 0.07, 0.78), (0.55, 0.14, 0.18), wall_shadow)
    cube("base_foundation", (0.0, 0.07, 0.0), (1.70, 0.14, 1.40), wall_shadow)

    export_glb("house_small")


def make_watchtower() -> None:
    clear_scene()
    stone = glow_mat("tower_stone", (0.60, 0.55, 0.50), glow_strength=0.25, roughness=0.9)
    stone_dark = glow_mat("tower_stone_dark", (0.40, 0.35, 0.30), glow_strength=0.15, roughness=0.95)
    wood = glow_mat("tower_wood", (0.50, 0.30, 0.15), glow_strength=0.25, roughness=0.9)
    flag = glow_mat("tower_flag", (0.95, 0.30, 0.40), glow_strength=0.7, roughness=0.6)
    flag_glow = glow_mat("tower_flag_glow", (1.0, 0.65, 0.30), glow_strength=1.0, roughness=0.4)
    roof = glow_mat("tower_roof", (0.45, 0.30, 0.20), glow_strength=0.25, roughness=0.85)
    door = glow_mat("tower_door", (0.40, 0.22, 0.10), glow_strength=0.25, roughness=0.9)

    cube("base1", (0.0, 0.30, 0.0), (1.05, 0.60, 1.05), stone_dark)
    cube("base2", (0.0, 0.90, 0.0), (0.95, 0.60, 0.95), stone)
    cube("mid1", (0.0, 1.50, 0.0), (0.85, 0.60, 0.85), stone_dark)
    cube("mid2", (0.0, 2.10, 0.0), (0.75, 0.60, 0.75), stone)
    cube("mid3", (0.0, 2.70, 0.0), (0.65, 0.60, 0.65), stone_dark)

    cube("window_n", (0.0, 1.20, 0.43), (0.30, 0.40, 0.04),
         mat("tower_window", (0.05, 0.05, 0.10), roughness=0.7))
    cube("window_s", (0.0, 1.80, -0.43), (0.30, 0.40, 0.04),
         mat("tower_window", (0.05, 0.05, 0.10), roughness=0.7))

    cube("door", (0.0, 0.30, 0.50), (0.30, 0.55, 0.04), door)
    uv_sphere("door_handle", (0.10, 0.30, 0.53), (0.03, 0.03, 0.03),
              glow_mat("tower_door_handle", (0.85, 0.65, 0.20), glow_strength=0.7), 8, 5)

    cylinder("platform", (0.0, 3.10, 0.0), 0.65, 0.10, wood, vertices=12)

    for ang in [0.0, 1.57, 3.14, 4.71]:
        x = math.cos(ang) * 0.55
        z = math.sin(ang) * 0.55
        cube("post", (x, 3.35, z), (0.10, 0.55, 0.10), wood)

    cube("rail_top_n", (0.0, 3.65, 0.55), (1.20, 0.06, 0.06), wood)
    cube("rail_top_s", (0.0, 3.65, -0.55), (1.20, 0.06, 0.06), wood)
    cube("rail_top_e", (0.55, 3.65, 0.0), (0.06, 0.06, 1.20), wood)
    cube("rail_top_w", (-0.55, 3.65, 0.0), (0.06, 0.06, 1.20), wood)

    cylinder("flag_pole", (0.0, 4.10, 0.0), 0.04, 0.95, wood, vertices=6)
    cube("flag_cloth", (0.30, 4.20, 0.0), (0.55, 0.32, 0.02), flag)
    cube("flag_emblem", (0.36, 4.20, 0.03), (0.16, 0.16, 0.02), flag_glow)

    cone("tower_cap", (0.0, 3.95, 0.0), 0.55, 0.0, 0.55, roof, vertices=8)
    cone("tower_cap2", (0.0, 4.40, 0.0), 0.18, 0.0, 0.35, roof, vertices=6)

    export_glb("watchtower")


def make_windmill() -> None:
    clear_scene()
    stone = glow_mat("mill_stone", (0.90, 0.82, 0.65), glow_strength=0.4, roughness=0.9)
    stone_dark = glow_mat("mill_stone_dark", (0.55, 0.45, 0.35), glow_strength=0.2, roughness=0.9)
    roof = glow_mat("mill_roof", (0.55, 0.30, 0.20), glow_strength=0.3, roughness=0.85)
    wood = glow_mat("mill_wood", (0.45, 0.28, 0.15), glow_strength=0.2, roughness=0.9)
    sail = glow_mat("mill_sail", (0.95, 0.92, 0.80), glow_strength=0.55, roughness=0.85)
    sail_frame = glow_mat("mill_sail_frame", (0.50, 0.30, 0.15), glow_strength=0.2, roughness=0.9)
    window = glow_mat("mill_window", (1.0, 0.80, 0.40), glow_strength=1.0, roughness=0.4)

    cylinder("tower_base", (0.0, 0.90, 0.0), 0.70, 1.80, stone, vertices=14)
    cylinder("tower_mid", (0.0, 1.85, 0.0), 0.55, 0.40, stone_dark, vertices=14)
    cylinder("tower_top", (0.0, 2.15, 0.0), 0.50, 0.30, stone, vertices=14)

    cone("roof", (0.0, 2.75, 0.0), 0.55, 0.0, 0.85, roof, vertices=10)

    cube("door", (0.0, 0.45, 0.55), (0.26, 0.65, 0.04), wood)
    cube("window_l", (-0.30, 1.20, 0.34), (0.26, 0.26, 0.04), window)
    cube("window_r", (0.30, 1.20, 0.34), (0.26, 0.26, 0.04), window)
    cube("window_back", (0.0, 1.80, -0.30), (0.22, 0.22, 0.04), window)

    cylinder("shaft", (0.0, 1.95, 0.65), 0.06, 0.40, wood, vertices=8)
    cube("shaft_x", (0.0, 2.10, 0.85), (1.30, 0.06, 0.06), sail_frame)
    cube("shaft_y", (0.0, 2.10, 0.85), (0.06, 1.30, 0.06), sail_frame)

    for ang in [0.0, 1.5708, 3.1416, 4.7124]:
        cx = math.cos(ang) * 0.45
        cy = 2.10 + math.sin(ang) * 0.45
        cube(f"sail_{ang}", (cx, cy, 0.95), (0.50, 0.06, 0.04), sail)

    cube("sail_top_l", (-0.30, 2.40, 0.95), (0.06, 0.50, 0.04), sail)
    cube("sail_top_r", (0.30, 2.40, 0.95), (0.06, 0.50, 0.04), sail)
    cube("sail_bot_l", (-0.30, 1.80, 0.95), (0.06, 0.50, 0.04), sail)
    cube("sail_bot_r", (0.30, 1.80, 0.95), (0.06, 0.50, 0.04), sail)

    export_glb("windmill")


def make_bridge_stone() -> None:
    clear_scene()
    stone = glow_mat("bridge_stone", (0.70, 0.62, 0.55), glow_strength=0.3, roughness=0.9)
    stone_dark = glow_mat("bridge_stone_dark", (0.40, 0.35, 0.30), glow_strength=0.15, roughness=0.95)
    wood = glow_mat("bridge_wood", (0.55, 0.32, 0.18), glow_strength=0.2, roughness=0.9)
    water = mat("bridge_water", (0.30, 0.55, 0.85), roughness=0.3, alpha=0.7)
    moss = glow_mat("bridge_moss", (0.30, 0.65, 0.25), glow_strength=0.4, roughness=0.95)

    cube("deck", (0.0, 0.95, 0.0), (3.60, 0.18, 1.10), stone)

    for sx in [-1.40, 0.0, 1.40]:
        cube("arch_leg", (sx, 0.50, 0.0), (0.30, 1.00, 0.80), stone_dark)

    cube("arch_top", (0.0, 0.10, 0.0), (2.80, 0.18, 0.55), stone_dark)

    cylinder("water", (0.0, 0.10, 0.0), 1.55, 0.08, water, vertices=18)

    for sx, sz in [(-1.65, 0.55), (-1.65, -0.55), (1.65, 0.55), (1.65, -0.55)]:
        cube("post", (sx, 1.45, sz), (0.10, 0.95, 0.10), wood)
    cube("rail_top_n", (0.0, 1.95, 0.55), (3.40, 0.06, 0.06), wood)
    cube("rail_top_s", (0.0, 1.95, -0.55), (3.40, 0.06, 0.06), wood)

    for sx in [-1.20, -0.40, 0.40, 1.20]:
        cube("spindle_n", (sx, 1.70, 0.55), (0.06, 0.55, 0.06), wood)
        cube("spindle_s", (sx, 1.70, -0.55), (0.06, 0.55, 0.06), wood)

    cube("step_l", (-1.85, 0.45, 0.0), (0.40, 0.10, 1.30), stone)
    cube("step_r", (1.85, 0.45, 0.0), (0.40, 0.10, 1.30), stone)

    for x, z in [(-1.70, 0.0), (1.70, 0.0), (0.0, 0.65), (0.0, -0.65),
                 (-0.85, 0.70), (0.85, -0.70)]:
        uv_sphere("moss", (x, 0.18, z), (0.16, 0.08, 0.16), moss, 8, 5)

    export_glb("bridge_stone")


def make_well() -> None:
    clear_scene()
    stone = glow_mat("well_stone", (0.55, 0.50, 0.45), glow_strength=0.25, roughness=0.9)
    stone_dark = glow_mat("well_stone_dark", (0.35, 0.30, 0.25), glow_strength=0.15, roughness=0.95)
    wood = glow_mat("well_wood", (0.50, 0.30, 0.15), glow_strength=0.2, roughness=0.9)
    roof = glow_mat("well_roof", (0.75, 0.30, 0.20), glow_strength=0.4, roughness=0.8)
    water = glow_mat("well_water", (0.20, 0.45, 0.75), glow_strength=0.7, roughness=0.3)
    bucket = glow_mat("well_bucket", (0.65, 0.40, 0.20), glow_strength=0.3, roughness=0.85)
    moss = glow_mat("well_moss", (0.30, 0.65, 0.25), glow_strength=0.4, roughness=0.95)

    cylinder("well_base", (0.0, 0.40, 0.0), 0.80, 0.80, stone, vertices=16)
    cylinder("well_top", (0.0, 0.84, 0.0), 0.75, 0.10, stone_dark, vertices=16)

    cylinder("water_pool", (0.0, 0.70, 0.0), 0.65, 0.10, water, vertices=16)

    cube("post_l", (-0.55, 1.55, 0.0), (0.14, 1.55, 0.14), wood)
    cube("post_r", (0.55, 1.55, 0.0), (0.14, 1.55, 0.14), wood)

    cube("beam_top", (0.0, 2.35, 0.0), (1.40, 0.14, 0.16), wood)
    cube("beam_x", (0.0, 2.00, 0.0), (0.14, 0.14, 0.95), wood)

    cone("roof_main", (0.0, 2.65, 0.0), 0.85, 0.0, 0.50, roof, vertices=4)
    cube("roof_eave", (0.0, 2.40, 0.0), (1.40, 0.08, 1.10), roof)

    cylinder("rope", (-0.25, 1.85, 0.0), 0.02, 0.85, wood, vertices=6)
    cube("bucket_body", (-0.25, 1.30, 0.0), (0.20, 0.18, 0.20), bucket)
    cone("bucket_handle", (-0.25, 1.46, 0.0), 0.10, 0.0, 0.08, wood, vertices=4)
    cylinder("bucket_rim", (-0.25, 1.39, 0.0), 0.12, 0.02, wood, vertices=8)

    for x, z, scale in [(-0.95, 0.65, 0.20), (0.95, 0.55, 0.22),
                        (-0.90, -0.70, 0.18), (0.85, -0.85, 0.20)]:
        ico_sphere("moss", (x, 0.15, z), (scale, 0.10, scale * 0.9), moss, subdivisions=1)

    export_glb("well")


def make_barn() -> None:
    clear_scene()
    wall = glow_mat("barn_wall", (0.85, 0.30, 0.25), glow_strength=0.4, roughness=0.85)
    wall_shadow = glow_mat("barn_wall_shadow", (0.60, 0.20, 0.18), glow_strength=0.2, roughness=0.9)
    roof = glow_mat("barn_roof", (0.30, 0.30, 0.30), glow_strength=0.2, roughness=0.85)
    trim = glow_mat("barn_trim", (0.95, 0.95, 0.90), glow_strength=0.7, roughness=0.6)
    door = glow_mat("barn_door", (0.55, 0.32, 0.18), glow_strength=0.3, roughness=0.85)
    hay = glow_mat("barn_hay", (0.95, 0.80, 0.35), glow_strength=0.5, roughness=0.9)

    cube("body", (0.0, 1.00, 0.0), (2.20, 2.00, 1.60), wall)
    cube("body_shadow", (0.0, 1.00, -0.40), (2.20, 2.00, 0.30), wall_shadow)

    cone("roof", (0.0, 2.55, 0.0), 1.45, 0.0, 1.10, roof, vertices=4)
    cube("roof_eave", (0.0, 2.00, 0.0), (2.40, 0.14, 1.75), roof)

    cube("trim_top", (0.0, 2.00, 0.81), (2.22, 0.10, 0.04), trim)
    cube("trim_bot", (0.0, 0.05, 0.81), (2.22, 0.10, 0.04), trim)
    cube("trim_x", (1.11, 1.00, 0.81), (0.04, 2.00, 0.04), trim)
    cube("trim_x2", (-1.11, 1.00, 0.81), (0.04, 2.00, 0.04), trim)

    cube("door_l", (-0.40, 0.75, 0.82), (0.65, 1.40, 0.04), door)
    cube("door_r", (0.40, 0.75, 0.82), (0.65, 1.40, 0.04), door)
    cube("door_x", (0.0, 1.50, 0.84), (1.30, 0.06, 0.02), trim)
    cube("door_y", (0.0, 0.75, 0.84), (1.30, 0.06, 0.02), trim)
    cube("door_z", (0.0, 0.30, 0.84), (0.06, 0.80, 0.02), trim)

    for x, z, sx, sz, sy in [
        (0.50, -0.55, 0.32, 0.22, 0.18), (-0.40, -0.50, 0.28, 0.20, 0.18),
        (0.10, -0.70, 0.25, 0.18, 0.16),
    ]:
        ico_sphere("hay", (x, 2.20 + sy, z), (sx, sy, sz), hay, subdivisions=1)

    cube("loft_door", (0.0, 2.35, 0.82), (0.40, 0.40, 0.04), door)

    cube("foundation", (0.0, 0.05, 0.0), (2.30, 0.10, 1.70), wall_shadow)

    export_glb("barn")


def make_fence() -> None:
    clear_scene()
    wood = glow_mat("fence_wood", (0.65, 0.45, 0.25), glow_strength=0.3, roughness=0.9)
    wood_dark = glow_mat("fence_wood_dark", (0.45, 0.28, 0.15), glow_strength=0.15, roughness=0.95)
    grass = glow_mat("fence_grass", (0.50, 0.85, 0.40), glow_strength=0.4, roughness=0.95)
    flower_red = glow_mat("fence_flower_red", (1.0, 0.30, 0.30), glow_strength=0.7, roughness=0.6)
    flower_pink = glow_mat("fence_flower_pink", (1.0, 0.55, 0.75), glow_strength=0.7, roughness=0.6)
    flower_yellow = glow_mat("fence_flower_yellow", (1.0, 0.85, 0.30), glow_strength=0.8, roughness=0.6)

    for px in [-1.10, -0.55, 0.0, 0.55, 1.10]:
        cube("post", (px, 0.55, 0.0), (0.12, 1.10, 0.12), wood)
        cone("post_cap", (px, 1.15, 0.0), 0.08, 0.0, 0.10, wood_dark, vertices=4)

    cube("rail_top", (0.0, 0.95, 0.0), (2.40, 0.08, 0.06), wood)
    cube("rail_mid", (0.0, 0.55, 0.0), (2.40, 0.08, 0.06), wood)
    cube("rail_bot", (0.0, 0.18, 0.0), (2.40, 0.08, 0.06), wood)

    for gx in [-1.30, -0.70, -0.10, 0.50, 1.20]:
        ico_sphere("grass", (gx, 0.10, 0.30), (0.18, 0.10, 0.18), grass, subdivisions=1)
        ico_sphere("grass2", (gx + 0.20, 0.10, -0.30), (0.16, 0.10, 0.16), grass, subdivisions=1)

    flower_specs = [
        (-0.80, 0.20, flower_red), (0.20, 0.20, flower_pink),
        (0.90, 0.20, flower_yellow), (-0.30, 0.20, flower_red),
    ]
    for i, (fx, fz, fc) in enumerate(flower_specs):
        cylinder(f"flower_stem_{i}", (fx, 0.25, fz), 0.018, 0.50,
                 glow_mat(f"fence_flower_stem_{i}", (0.30, 0.65, 0.30), glow_strength=0.3),
                 vertices=6)
        cone(f"flower_leaf_{i}", (fx + 0.04, 0.35, fz), 0.04, 0.0, 0.10,
             glow_mat(f"fence_flower_leaf_{i}", (0.30, 0.65, 0.30), glow_strength=0.3), vertices=4)
        uv_sphere(f"flower_{i}", (fx, 0.55, fz), (0.07, 0.07, 0.07), fc, 8, 6)

    export_glb("fence")


def make_shrine() -> None:
    clear_scene()
    stone = glow_mat("shrine_stone", (0.90, 0.85, 0.75), glow_strength=0.5, roughness=0.85)
    stone_dark = glow_mat("shrine_stone_dark", (0.55, 0.48, 0.40), glow_strength=0.2, roughness=0.9)
    roof = glow_mat("shrine_roof", (0.75, 0.25, 0.20), glow_strength=0.5, roughness=0.75)
    wood = glow_mat("shrine_wood", (0.55, 0.30, 0.15), glow_strength=0.25, roughness=0.9)
    candle = glow_mat("shrine_candle", (1.0, 0.85, 0.50), glow_strength=1.4, roughness=0.3)
    flame = glow_mat("shrine_flame", (1.0, 0.65, 0.20), glow_strength=1.5, roughness=0.2)
    statue = glow_mat("shrine_statue", (0.95, 0.85, 0.65), glow_strength=0.6, roughness=0.6)

    cube("base", (0.0, 0.18, 0.0), (1.30, 0.36, 1.00), stone_dark)
    cube("altar", (0.0, 0.45, 0.0), (0.95, 0.30, 0.80), stone)
    cube("altar_top", (0.0, 0.62, 0.0), (0.90, 0.06, 0.75), stone)

    for px, pz in [(-0.55, -0.40), (0.55, -0.40), (-0.55, 0.40), (0.55, 0.40)]:
        cube("pillar", (px, 1.10, pz), (0.14, 1.20, 0.14), stone)
        cube("pillar_base", (px, 0.45, pz), (0.22, 0.06, 0.22), stone_dark)
        cube("pillar_top", (px, 1.75, pz), (0.22, 0.08, 0.22), stone)

    cube("beam_top", (0.0, 1.85, 0.0), (1.50, 0.18, 1.00), wood)

    cone("roof_main", (0.0, 2.30, 0.0), 1.05, 0.0, 0.70, roof, vertices=4)
    cube("roof_eave", (0.0, 1.95, 0.0), (1.60, 0.06, 1.10), roof)
    cube("roof_curve_l", (-0.65, 2.05, 0.0), (0.30, 0.10, 1.10), roof)
    cube("roof_curve_r", (0.65, 2.05, 0.0), (0.30, 0.10, 1.10), roof)

    cube("finial", (0.0, 2.80, 0.0), (0.16, 0.20, 0.16), wood)
    cone("finial_top", (0.0, 3.00, 0.0), 0.08, 0.0, 0.22, wood, vertices=4)
    uv_sphere("finial_orb", (0.0, 3.15, 0.0), (0.06, 0.06, 0.06), wood, 8, 6)

    cone("statue", (0.0, 0.95, 0.0), 0.22, 0.10, 0.55, statue, vertices=8)
    uv_sphere("statue_head", (0.0, 1.40, 0.0), (0.14, 0.16, 0.12), statue, 10, 8)
    cube("statue_base", (0.0, 0.70, 0.0), (0.30, 0.05, 0.30), stone_dark)

    for sx in [-0.32, 0.32]:
        cylinder("candle", (sx, 0.78, 0.0), 0.07, 0.18, candle, vertices=8)
        cone("flame", (sx, 0.95, 0.0), 0.04, 0.0, 0.10, flame, vertices=6)
        cube("candle_drip", (sx, 0.92, 0.0), (0.10, 0.04, 0.10),
             glow_mat("shrine_candle_drip", (0.95, 0.90, 0.75), glow_strength=0.6))

    export_glb("shrine")


def make_lighthouse() -> None:
    clear_scene()
    white = glow_mat("lh_white", (0.95, 0.95, 1.0), glow_strength=0.55, roughness=0.75)
    red = glow_mat("lh_red", (0.90, 0.30, 0.30), glow_strength=0.5, roughness=0.75)
    dark = glow_mat("lh_dark", (0.30, 0.30, 0.35), glow_strength=0.15, roughness=0.9)
    light_glow = glow_mat("lh_light", (1.0, 0.95, 0.55), glow_strength=1.6, roughness=0.2)
    light_glow_warm = glow_mat("lh_light_warm", (1.0, 0.70, 0.30), glow_strength=1.5, roughness=0.2)
    base = glow_mat("lh_base", (0.55, 0.50, 0.45), glow_strength=0.25, roughness=0.9)
    door = glow_mat("lh_door", (0.40, 0.25, 0.15), glow_strength=0.3, roughness=0.85)

    cylinder("base", (0.0, 0.15, 0.0), 0.70, 0.30, base, vertices=16)
    cube("base_block", (0.0, 0.35, 0.0), (1.30, 0.10, 1.30), dark)
    cube("base_step", (0.0, 0.22, 0.0), (1.00, 0.10, 1.00), dark)

    cylinder("seg1_white", (0.0, 0.65, 0.0), 0.50, 0.50, white, vertices=16)
    cylinder("seg2_red", (0.0, 1.20, 0.0), 0.46, 0.50, red, vertices=16)
    cylinder("seg3_white", (0.0, 1.75, 0.0), 0.42, 0.50, white, vertices=16)
    cylinder("seg4_red", (0.0, 2.30, 0.0), 0.38, 0.50, red, vertices=16)
    cylinder("seg5_white", (0.0, 2.80, 0.0), 0.34, 0.40, white, vertices=16)
    cylinder("seg6_red_top", (0.0, 3.20, 0.0), 0.30, 0.30, red, vertices=16)

    cube("door", (0.0, 0.50, 0.42), (0.20, 0.45, 0.04), door)
    cube("window1", (0.0, 1.20, 0.36), (0.20, 0.20, 0.04), light_glow)
    cube("window2", (0.0, 1.80, 0.32), (0.18, 0.18, 0.04), light_glow)
    cube("window3", (0.0, 2.30, 0.28), (0.16, 0.16, 0.04), light_glow)
    cube("window4", (0.0, 2.80, 0.24), (0.14, 0.14, 0.04), light_glow)

    cylinder("lamp_room", (0.0, 3.55, 0.0), 0.34, 0.30, dark, vertices=14)
    cylinder("lamp_glass", (0.0, 3.55, 0.0), 0.30, 0.28, light_glow, vertices=14)

    uv_sphere("lamp_core", (0.0, 3.55, 0.0), (0.16, 0.16, 0.16), light_glow_warm, 12, 8)
    uv_sphere("lamp_halo", (0.0, 3.55, 0.0), (0.30, 0.30, 0.30),
              glow_mat("lh_lamp_halo", (1.0, 0.85, 0.50), glow_strength=0.7, roughness=0.5), 12, 8)

    cone("roof", (0.0, 3.85, 0.0), 0.36, 0.0, 0.35, dark, vertices=10)
    cylinder("spire", (0.0, 4.20, 0.0), 0.04, 0.45, dark, vertices=6)
    uv_sphere("spire_ball", (0.0, 4.50, 0.0), (0.06, 0.06, 0.06), dark, 8, 5)

    for ang in [0.0, 1.57, 3.14, 4.71]:
        x = math.cos(ang) * 0.80
        z = math.sin(ang) * 0.80
        cube("base_strut", (x, 0.05, z), (0.08, 0.08, 0.08), dark)

    export_glb("lighthouse")


# --------------------------------------------------------------------- MAIN


def main() -> None:
    out_dir = models_lib.OUT_DIR
    out_dir.mkdir(parents=True, exist_ok=True)
    print(f"=== building terrain & buildings v2 -> {out_dir} ===")

    print("\n--- terrain (8) ---")
    print("[1/8] mountain_snow")
    make_mountain_snow()
    print("[2/8] volcano")
    make_volcano()
    print("[3/8] desert_dune")
    make_desert_dune()
    print("[4/8] lake")
    make_lake()
    print("[5/8] swamp")
    make_swamp()
    print("[6/8] cliff")
    make_cliff()
    print("[7/8] cave_entrance")
    make_cave_entrance()
    print("[8/8] beach")
    make_beach()

    print("\n--- buildings (9) ---")
    print("[1/9] house_small")
    make_house_small()
    print("[2/9] watchtower")
    make_watchtower()
    print("[3/9] windmill")
    make_windmill()
    print("[4/9] bridge_stone")
    make_bridge_stone()
    print("[5/9] well")
    make_well()
    print("[6/9] barn")
    make_barn()
    print("[7/9] fence")
    make_fence()
    print("[8/9] shrine")
    make_shrine()
    print("[9/9] lighthouse")
    make_lighthouse()

    new_assets = {
        "terrain": [
            "mountain_snow", "volcano", "desert_dune", "lake", "swamp",
            "cliff", "cave_entrance", "beach",
        ],
        "buildings": [
            "house_small", "watchtower", "windmill", "bridge_stone", "well",
            "barn", "fence", "shrine", "lighthouse",
        ],
    }
    print(f"\n=== generated {sum(len(v) for v in new_assets.values())} new .glb files ===")
    print(json.dumps(new_assets, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
