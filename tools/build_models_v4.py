"""Build kitchen/cooking models for lastkingdom2: cauldron, cooking_station,
spit_roast. v4 kitchen theme matching the existing style.

Output: assets/procedural/pretty/

Run through `python tools/model_pipeline.py build --generator build_models_v4.py`.
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


# ============================================================== CAULDRON


def make_cauldron() -> None:
    clear_scene()
    iron = mat_only("cauldron_iron", (0.18, 0.18, 0.20), roughness=0.5)
    iron_dark = mat_only("cauldron_iron_dark", (0.10, 0.10, 0.12), roughness=0.6)
    iron_rust = glow_mat("cauldron_iron_rust", (0.45, 0.25, 0.12), glow_strength=0.20, roughness=0.7)
    soup = glow_mat("cauldron_soup", (0.75, 0.50, 0.20), glow_strength=0.7, roughness=0.4)
    soup_bubble = glow_mat("cauldron_soup_bubble", (0.95, 0.75, 0.30), glow_strength=1.4, roughness=0.3)
    steam = mat("cauldron_steam", (0.85, 0.85, 0.88), roughness=0.95, alpha=0.55)
    wood = glow_mat("cauldron_wood", (0.45, 0.28, 0.12), glow_strength=0.20, roughness=0.9)
    coal = glow_mat("cauldron_coal", (0.85, 0.20, 0.05), glow_strength=1.2, roughness=0.6)
    coal_hot = glow_mat("cauldron_coal_hot", (1.0, 0.55, 0.10), glow_strength=1.6, roughness=0.3)

    cylinder("pot_outer", (0.0, 0.55, 0.0), 0.50, 0.80, iron, vertices=14)
    cylinder("pot_inner", (0.0, 0.55, 0.0), 0.45, 0.80, iron_dark, vertices=14)
    cylinder("rim", (0.0, 0.92, 0.0), 0.52, 0.06, iron, vertices=14)
    cylinder("rim_under", (0.0, 0.20, 0.0), 0.52, 0.06, iron, vertices=14)

    cylinder("soup", (0.0, 0.86, 0.0), 0.44, 0.08, soup, vertices=14)

    for ang in [0.0, 1.05, 2.1, 3.14, 4.2, 5.24]:
        x = math.cos(ang) * 0.52
        z = math.sin(ang) * 0.52
        cube("rivet", (x, 0.92, z), (0.04, 0.04, 0.04), iron_dark)
        cube("rivet_low", (x, 0.20, z), (0.04, 0.04, 0.04), iron_dark)

    cylinder("handle_l", (-0.55, 0.65, 0.0), 0.05, 0.04,
             glow_mat("cauldron_handle", (0.30, 0.18, 0.08), glow_strength=0.2, roughness=0.85), vertices=8)
    cylinder("handle_r", (0.55, 0.65, 0.0), 0.05, 0.04,
             glow_mat("cauldron_handle2", (0.30, 0.18, 0.08), glow_strength=0.2, roughness=0.85), vertices=8)

    cube("leg_fl", (-0.40, 0.10, 0.40), (0.10, 0.20, 0.10), iron_dark)
    cube("leg_fr", (0.40, 0.10, 0.40), (0.10, 0.20, 0.10), iron_dark)
    cube("leg_bl", (-0.40, 0.10, -0.40), (0.10, 0.20, 0.10), iron_dark)
    cube("leg_br", (0.40, 0.10, -0.40), (0.10, 0.20, 0.10), iron_dark)

    for px, pz in [(-0.30, 0.30), (0.30, 0.30), (-0.30, -0.30), (0.30, -0.30)]:
        uv_sphere("coal_glow", (px, 0.08, pz), (0.10, 0.08, 0.10), coal_hot, 8, 6)
        uv_sphere("coal", (px + 0.05, 0.06, pz - 0.05), (0.07, 0.06, 0.07), coal, 8, 5)

    for px, pz in [(-0.20, 0.0), (0.10, 0.15), (0.30, -0.10)]:
        uv_sphere("bubble", (px, 0.90, pz), (0.04, 0.03, 0.04), soup_bubble, 8, 5)

    uv_sphere("steam1", (-0.10, 1.30, 0.05), (0.18, 0.18, 0.18), steam, 10, 7)
    uv_sphere("steam2", (0.15, 1.65, -0.10), (0.22, 0.22, 0.22), steam, 10, 7)
    uv_sphere("steam3", (0.0, 1.95, 0.10), (0.28, 0.28, 0.28), steam, 10, 7)
    uv_sphere("steam4", (-0.15, 2.25, -0.05), (0.32, 0.32, 0.32), steam, 10, 7)

    cube("ladder_base", (-0.70, 0.05, 0.0), (0.10, 0.10, 0.10), wood)
    cube("ladder_pole1", (-0.70, 0.55, 0.0), (0.06, 1.10, 0.06), wood)
    cube("ladder_pole2", (-0.55, 0.55, 0.0), (0.06, 1.10, 0.06), wood)
    for py in [0.20, 0.40, 0.60, 0.80, 1.00]:
        cube(f"ladder_rung_{py}", (-0.62, py, 0.0), (0.18, 0.04, 0.04), wood)

    cube("log_a", (-0.40, 0.18, 0.55), (0.50, 0.08, 0.08), wood)
    cube("log_b", (0.30, 0.18, -0.55), (0.40, 0.08, 0.08), wood)

    cube("spoon_handle", (0.65, 1.05, 0.0), (0.04, 0.60, 0.04), wood)
    cube("spoon_bowl", (0.65, 0.72, 0.0), (0.18, 0.10, 0.18),
         glow_mat("cauldron_spoon_bowl", (0.55, 0.35, 0.18), glow_strength=0.3, roughness=0.7))

    export_glb("cauldron")


# ============================================================== COOKING STATION


def make_cooking_station() -> None:
    clear_scene()
    wood = glow_mat("cs_wood", (0.55, 0.30, 0.15), glow_strength=0.25, roughness=0.85)
    wood_dark = glow_mat("cs_wood_dark", (0.32, 0.18, 0.08), glow_strength=0.12, roughness=0.9)
    iron = mat_only("cs_iron", (0.30, 0.30, 0.32))
    bread = glow_mat("cs_bread", (0.85, 0.60, 0.30), glow_strength=0.40, roughness=0.7)
    bread_dark = glow_mat("cs_bread_dark", (0.65, 0.45, 0.20), glow_strength=0.25, roughness=0.8)
    meat = glow_mat("cs_meat", (0.75, 0.30, 0.20), glow_strength=0.40, roughness=0.7)
    apple = glow_mat("cs_apple", (0.92, 0.20, 0.18), glow_strength=0.50, roughness=0.5)
    cheese = glow_mat("cs_cheese", (0.95, 0.80, 0.30), glow_strength=0.45, roughness=0.6)
    cloth = glow_mat("cs_cloth", (0.90, 0.85, 0.75), glow_strength=0.40, roughness=0.8)
    cloth_red = glow_mat("cs_cloth_red", (0.85, 0.25, 0.25), glow_strength=0.5, roughness=0.8)

    cube("table_top", (0.0, 0.85, 0.0), (1.60, 0.10, 0.85), wood)
    cube("table_top_dark", (0.0, 0.80, -0.30), (1.60, 0.05, 0.20), wood_dark)

    cube("leg_fl", (-0.70, 0.40, 0.40), (0.10, 0.80, 0.10), wood_dark)
    cube("leg_fr", (0.70, 0.40, 0.40), (0.10, 0.80, 0.10), wood_dark)
    cube("leg_bl", (-0.70, 0.40, -0.40), (0.10, 0.80, 0.10), wood_dark)
    cube("leg_br", (0.70, 0.40, -0.40), (0.10, 0.80, 0.10), wood_dark)

    cube("cross_x", (0.0, 0.20, 0.0), (1.50, 0.06, 0.06), wood_dark)
    cube("cross_z", (0.0, 0.20, 0.0), (0.06, 0.06, 0.85), wood_dark)

    cube("bread1", (-0.50, 1.00, 0.20), (0.18, 0.16, 0.28), bread)
    cube("bread2", (-0.55, 1.04, -0.15), (0.20, 0.18, 0.30), bread_dark)
    cube("bread3", (-0.30, 1.05, -0.05), (0.16, 0.14, 0.24), bread)

    cube("cutting_board", (0.30, 0.96, 0.0), (0.45, 0.04, 0.30),
         glow_mat("cs_board", (0.75, 0.55, 0.30), glow_strength=0.30, roughness=0.7))

    cube("meat1", (0.20, 1.04, 0.05), (0.20, 0.08, 0.18), meat)
    cube("meat2", (0.35, 1.04, -0.10), (0.18, 0.07, 0.15), meat)

    cube("cheese_wheel", (0.50, 1.04, 0.30), (0.20, 0.20, 0.20), cheese)
    cube("cheese_cut", (0.55, 1.07, 0.20), (0.10, 0.10, 0.10), cheese)

    for px, pz in [(-0.10, 0.32), (0.10, 0.28), (0.0, 0.35), (-0.05, 0.20)]:
        uv_sphere("apple", (px, 1.02, pz), (0.06, 0.06, 0.06), apple, 8, 6)

    cube("knife_handle", (0.40, 1.02, -0.25), (0.04, 0.04, 0.16), wood_dark)
    cube("knife_blade", (0.40, 1.04, -0.18), (0.04, 0.02, 0.20),
         glow_mat("cs_blade", (0.85, 0.85, 0.95), glow_strength=0.6, roughness=0.2))

    cube("cloth_n", (0.0, 0.92, 0.42), (1.40, 0.04, 0.06), cloth)
    cube("cloth_s", (0.0, 0.92, -0.42), (1.40, 0.04, 0.06), cloth_red)
    cube("cloth_strip", (-0.30, 0.92, 0.42), (0.20, 0.04, 0.04), cloth_red)

    cube("post_l", (-0.70, 1.30, -0.40), (0.08, 0.80, 0.08), wood_dark)
    cube("post_r", (0.70, 1.30, -0.40), (0.08, 0.80, 0.08), wood_dark)
    cube("beam", (0.0, 1.75, -0.40), (1.55, 0.08, 0.08), wood_dark)
    cone("canopy", (0.0, 1.95, -0.40), 0.90, 0.0, 0.45, cloth_red, vertices=4)

    cube("bucket", (0.85, 0.50, 0.50), (0.20, 0.20, 0.20), iron)
    cube("bucket_rim", (0.85, 0.62, 0.50), (0.22, 0.04, 0.22), iron)
    cube("bucket_handle", (0.85, 0.65, 0.50), (0.04, 0.04, 0.20), iron)

    export_glb("cooking_station")


# ============================================================== SPIT ROAST


def make_spit_roast() -> None:
    clear_scene()
    wood = glow_mat("sr_wood", (0.55, 0.30, 0.15), glow_strength=0.25, roughness=0.85)
    wood_dark = glow_mat("sr_wood_dark", (0.32, 0.18, 0.08), glow_strength=0.12, roughness=0.9)
    iron = mat_only("sr_iron", (0.30, 0.30, 0.32))
    meat = glow_mat("sr_meat", (0.75, 0.30, 0.20), glow_strength=0.45, roughness=0.7)
    meat_cooked = glow_mat("sr_meat_cooked", (0.60, 0.30, 0.15), glow_strength=0.55, roughness=0.6)
    meat_burnt = mat_only("sr_meat_burnt", (0.30, 0.15, 0.08))
    coal = glow_mat("sr_coal", (0.85, 0.20, 0.05), glow_strength=1.2, roughness=0.6)
    coal_hot = glow_mat("sr_coal_hot", (1.0, 0.55, 0.10), glow_strength=1.6, roughness=0.3)
    flame = glow_mat("sr_flame", (1.0, 0.50, 0.10), glow_strength=1.7, roughness=0.3)
    smoke = mat("sr_smoke", (0.40, 0.38, 0.36), roughness=0.95, alpha=0.55)
    rope = glow_mat("sr_rope", (0.75, 0.62, 0.35), glow_strength=0.25, roughness=0.85)

    cube("post_fl", (-0.65, 0.85, 0.0), (0.14, 1.70, 0.14), wood_dark)
    cube("post_fr", (0.65, 0.85, 0.0), (0.14, 1.70, 0.14), wood_dark)
    cube("post_bl", (-0.65, 0.85, -0.60), (0.14, 1.70, 0.14), wood_dark)
    cube("post_br", (0.65, 0.85, -0.60), (0.14, 1.70, 0.14), wood_dark)

    cube("cross_top", (0.0, 2.55, -0.30), (1.50, 0.12, 0.12), wood)
    cube("cross_back", (-0.65, 2.55, -0.30), (0.12, 0.12, 0.60), wood)

    cube("spit", (0.0, 1.85, -0.30), (2.10, 0.06, 0.06),
         glow_mat("sr_spit", (0.65, 0.45, 0.25), glow_strength=0.4, roughness=0.6))

    for i, x in enumerate([-0.55, -0.30, -0.05, 0.20, 0.45, 0.70]):
        cube(f"meat_{i}", (x, 1.85, -0.30), (0.18, 0.18, 0.22),
             meat if i % 2 == 0 else meat_cooked)

    cube("rope_l", (-0.65, 2.20, -0.30), (0.04, 0.40, 0.04), rope)
    cube("rope_r", (0.65, 2.20, -0.30), (0.04, 0.40, 0.04), rope)

    for px, pz in [(-0.45, -0.30), (0.0, -0.45), (0.45, -0.30), (-0.20, -0.10), (0.20, -0.10)]:
        uv_sphere(f"coal_glow_{px}_{pz}", (px, 0.55, pz), (0.12, 0.08, 0.12), coal_hot, 8, 6)
        uv_sphere(f"coal_{px}_{pz}", (px + 0.05, 0.60, pz + 0.05), (0.08, 0.06, 0.08), coal, 8, 5)

    for ang in [0.0, 0.8, 1.7, 2.6, 3.5, 4.5, 5.4]:
        r = 0.25 + (ang % 2) * 0.10
        h = 0.30 + (ang % 3) * 0.15
        cone(f"flame_{ang:.1f}", (math.cos(ang) * 0.30, 0.85 + h * 0.4, math.sin(ang) * 0.30 - 0.20),
             r * 0.5, 0.0, h, flame, vertices=8)

    cube("log_a", (-0.40, 0.45, 0.05), (0.50, 0.10, 0.10), wood)
    cube("log_b", (0.40, 0.45, 0.05), (0.40, 0.10, 0.10), wood_dark)
    cube("log_c", (0.0, 0.40, -0.50), (0.45, 0.10, 0.10), wood)

    uv_sphere("smoke1", (0.0, 2.80, -0.30), (0.20, 0.20, 0.20), smoke, 10, 7)
    uv_sphere("smoke2", (0.10, 3.10, -0.40), (0.25, 0.25, 0.25), smoke, 10, 7)
    uv_sphere("smoke3", (-0.10, 3.40, -0.20), (0.30, 0.30, 0.30), smoke, 10, 7)

    cube("drip_a", (-0.30, 1.55, -0.30), (0.04, 0.08, 0.04),
         glow_mat("sr_drip", (0.85, 0.30, 0.15), glow_strength=0.7))
    cube("drip_b", (0.20, 1.50, -0.30), (0.04, 0.06, 0.04),
         glow_mat("sr_drip2", (0.85, 0.30, 0.15), glow_strength=0.7))

    export_glb("spit_roast")


# ============================================================== GROUND PATCH


def make_ground_patch() -> None:
    """A large grass disc 12m radius to overlay the chunky marching-cubes terrain."""
    clear_scene()
    grass = glow_mat("gp_grass", (0.42, 0.62, 0.28), glow_strength=0.25, roughness=0.9)
    grass_dark = glow_mat("gp_grass_dark", (0.30, 0.48, 0.20), glow_strength=0.18, roughness=0.92)
    grass_pale = glow_mat("gp_grass_pale", (0.55, 0.72, 0.35), glow_strength=0.28, roughness=0.85)
    dirt = mat_only("gp_dirt", (0.55, 0.45, 0.30), roughness=0.95)
    flower_w = glow_mat("gp_flower_w", (1.0, 0.95, 0.85), glow_strength=0.55, roughness=0.6)
    flower_y = glow_mat("gp_flower_y", (1.0, 0.85, 0.30), glow_strength=0.55, roughness=0.6)
    flower_p = glow_mat("gp_flower_p", (1.0, 0.55, 0.75), glow_strength=0.55, roughness=0.6)

    cylinder("main_disc", (0.0, -0.05, 0.0), 12.0, 0.10, grass, vertices=64)
    cylinder("ring1", (0.0, 0.0, 0.0), 11.0, 0.02, grass_dark, vertices=64)
    cylinder("ring2", (0.0, 0.0, 0.0), 8.5, 0.02, grass_pale, vertices=64)
    cylinder("ring3", (0.0, 0.0, 0.0), 5.5, 0.02, grass_dark, vertices=48)
    cylinder("ring4", (0.0, 0.0, 0.0), 3.0, 0.02, grass_pale, vertices=32)

    for ang_deg in range(0, 360, 24):
        ang = math.radians(ang_deg)
        for r in [2.0, 5.5, 9.5]:
            x = math.cos(ang) * r
            z = math.sin(ang) * r
            uv_sphere(f"tuft_{ang_deg}_{r}", (x, 0.05, z), (0.12, 0.04, 0.12), grass_dark, 4, 3)

    for ang_deg in range(15, 360, 45):
        ang = math.radians(ang_deg)
        r = 3.5 + (ang_deg % 7) * 0.6
        x = math.cos(ang) * r
        z = math.sin(ang) * r
        flower = flower_w if (ang_deg % 3 == 0) else (flower_y if (ang_deg % 3 == 1) else flower_p)
        uv_sphere(f"flower_{ang_deg}", (x, 0.05, z), (0.06, 0.04, 0.06), flower, 4, 3)

    for ang_deg in [25, 110, 200, 285]:
        ang = math.radians(ang_deg)
        for r in [4.5, 7.5, 10.5]:
            x = math.cos(ang) * r
            z = math.sin(ang) * r
            cube(f"dirt_{ang_deg}_{r}", (x, 0.05, z), (0.40, 0.02, 0.40), dirt)

    for ang_deg in range(0, 360, 45):
        ang = math.radians(ang_deg)
        x = math.cos(ang) * 11.5
        z = math.sin(ang) * 11.5
        uv_sphere(f"edge_tuft_{ang_deg}", (x, 0.05, z), (0.20, 0.10, 0.20), grass_dark, 5, 3)

    raise RuntimeError("ground_patch is owned by build_sokpop_style_pass.py")


# ============================================================== MAIN


def main() -> None:
    print("[v4] start building kitchen + ground patch models...")
    make_cauldron()
    make_cooking_station()
    make_spit_roast()
    print("[v4] done.")


if __name__ == "__main__":
    main()
