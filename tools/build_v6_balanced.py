
from __future__ import annotations
import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import models_lib

from models_lib import (
    clear_scene, cube, cone, cylinder, export_glb, ico_sphere, mat, uv_sphere,
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

def glow_mat(name, color, glow_strength=0.4, roughness=0.5):
    return mat(name, color, roughness=max(roughness, 0.82))

    clear_scene()
    skin = glow_mat("cute_skin_warm", (1.0, 0.85, 0.75), glow_strength=0.4)
    shirt_red = glow_mat("cute_shirt_red", (1.0, 0.45, 0.45), glow_strength=0.5)
    pants_blue = glow_mat("cute_pants_blue", (0.45, 0.55, 0.95), glow_strength=0.4)
    hair = glow_mat("cute_hair_brown", (0.45, 0.30, 0.20), glow_strength=0.3)
    eye = glow_mat("cute_eye_dark", (0.10, 0.05, 0.08), glow_strength=0.2)
    eye_shine = glow_mat("cute_eye_shine", (1.0, 1.0, 1.0), glow_strength=0.95)
    cheek = glow_mat("cute_cheek_pink", (1.0, 0.65, 0.70), glow_strength=0.6)
    mouth_dark = glow_mat("cute_mouth", (0.85, 0.30, 0.40), glow_strength=0.5)

    uv_sphere("head", (0.0, 0.32, 0.0), (0.14, 0.13, 0.13), skin, 12, 9)

    uv_sphere("hair", (0.0, 0.38, 0.0), (0.14, 0.10, 0.14), hair, 12, 9)

    uv_sphere("bangs", (0.0, 0.36, 0.08), (0.10, 0.06, 0.06), hair, 10, 8)

    uv_sphere("eye_l", (-0.05, 0.33, 0.10), (0.030, 0.040, 0.025), eye, 12, 10)
    uv_sphere("eye_r", ( 0.05, 0.33, 0.10), (0.030, 0.040, 0.025), eye, 12, 10)

    uv_sphere("shine_l_big", (-0.040, 0.350, 0.125), (0.014, 0.016, 0.008), eye_shine, 8, 6)
    uv_sphere("shine_l_small", (-0.055, 0.318, 0.122), (0.008, 0.010, 0.005), eye_shine, 8, 6)
    uv_sphere("shine_r_big", ( 0.060, 0.350, 0.125), (0.014, 0.016, 0.008), eye_shine, 8, 6)
    uv_sphere("shine_r_small", ( 0.045, 0.318, 0.122), (0.008, 0.010, 0.005), eye_shine, 8, 6)

    uv_sphere("mouth", (0.0, 0.28, 0.13), (0.020, 0.015, 0.010), mouth_dark, 8, 6)

    uv_sphere("cheek_l", (-0.11, 0.30, 0.08), (0.040, 0.030, 0.020), cheek, 10, 6)
    uv_sphere("cheek_r", ( 0.11, 0.30, 0.08), (0.040, 0.030, 0.020), cheek, 10, 6)

    uv_sphere("body", (0.0, 0.18, 0.0), (0.16, 0.14, 0.13), shirt_red, 12, 9)

    cone("collar", (0.0, 0.28, 0.10), 0.04, 0.0, 0.025, shirt_red, vertices=4)

    uv_sphere("leg_l", (-0.05, 0.05, 0.0), (0.05, 0.05, 0.05), pants_blue, 10, 8)
    uv_sphere("leg_r", ( 0.05, 0.05, 0.0), (0.05, 0.05, 0.05), pants_blue, 10, 8)

    uv_sphere("foot_l", (-0.05, 0.025, 0.025), (0.07, 0.04, 0.085), pants_blue, 10, 8)
    uv_sphere("foot_r", ( 0.05, 0.025, 0.025), (0.07, 0.04, 0.085), pants_blue, 10, 8)

    uv_sphere("hand_l", (-0.16, 0.20, 0.0), (0.05, 0.05, 0.05), skin, 10, 8)
    uv_sphere("hand_r", ( 0.16, 0.20, 0.0), (0.05, 0.05, 0.05), skin, 10, 8)

def make_monster_v6(name, body_color, horn_color, eye_color=(0.05, 0.02, 0.04)) -> None:
    clear_scene()
    body = glow_mat(f"{name}_body", body_color, roughness=0.92)
    dark = glow_mat(f"{name}_dark", tuple(max(c * 0.55, 0.05) for c in body_color), roughness=0.95)
    accent = glow_mat(f"{name}_accent", horn_color, roughness=0.9)
    eye = glow_mat(f"{name}_eye", eye_color, roughness=0.9)

    if name == "monster_snake":
        cube("body", (0.0, 0.22, 0.0), (0.60, 0.18, 0.18), body)
        cube("head", (0.44, 0.28, 0.0), (0.22, 0.22, 0.22), body)
        cube("tail", (-0.42, 0.24, 0.0), (0.22, 0.12, 0.12), dark)
        cube("eye_l", (0.56, 0.32, 0.09), (0.035, 0.035, 0.025), eye)
        cube("eye_r", (0.56, 0.32, -0.09), (0.035, 0.035, 0.025), eye)
        cube("stripe_1", (-0.18, 0.37, 0.0), (0.045, 0.035, 0.20), accent)
        cube("stripe_2", (0.08, 0.37, 0.0), (0.045, 0.035, 0.20), accent)
    elif name == "monster_frost_elf":
        cube("body", (0.0, 0.34, 0.0), (0.24, 0.42, 0.18), body)
        uv_sphere("head", (0.0, 0.72, 0.0), (0.18, 0.16, 0.17), body, 8, 5)
        cone("hat", (0.0, 0.98, 0.0), 0.18, 0.0, 0.30, accent, vertices=5)
        cube("arm_l", (-0.22, 0.44, 0.0), (0.08, 0.28, 0.08), dark)
        cube("arm_r", (0.22, 0.44, 0.0), (0.08, 0.28, 0.08), dark)
        cube("eye_l", (-0.06, 0.75, 0.145), (0.030, 0.030, 0.018), eye)
        cube("eye_r", (0.06, 0.75, 0.145), (0.030, 0.030, 0.018), eye)
    elif name == "monster_sand_wurm":
        for i, x in enumerate([-0.42, -0.20, 0.02, 0.24]):
            uv_sphere(f"segment_{i}", (x, 0.18 + i * 0.02, 0.0), (0.18, 0.14, 0.16), body if i % 2 else dark, 8, 5)
        cube("head", (0.48, 0.28, 0.0), (0.22, 0.20, 0.20), body)
        cube("jaw", (0.58, 0.18, 0.0), (0.10, 0.08, 0.18), dark)
        cube("eye_l", (0.58, 0.33, 0.08), (0.030, 0.030, 0.020), eye)
        cube("eye_r", (0.58, 0.33, -0.08), (0.030, 0.030, 0.020), eye)
    elif name == "monster_treant":
        cube("trunk", (0.0, 0.38, 0.0), (0.24, 0.55, 0.20), body)
        cube("crown", (0.0, 0.78, 0.0), (0.40, 0.24, 0.34), accent)
        cube("root_l", (-0.22, 0.10, 0.08), (0.22, 0.08, 0.08), dark)
        cube("root_r", (0.22, 0.10, -0.08), (0.22, 0.08, 0.08), dark)
        cube("branch_l", (-0.26, 0.56, 0.0), (0.24, 0.08, 0.08), dark)
        cube("branch_r", (0.26, 0.60, 0.0), (0.24, 0.08, 0.08), dark)
        cube("eye_l", (-0.06, 0.48, 0.105), (0.030, 0.030, 0.018), eye)
        cube("eye_r", (0.06, 0.48, 0.105), (0.030, 0.030, 0.018), eye)
    else:
        cube("cloak", (0.0, 0.36, 0.0), (0.34, 0.54, 0.22), body)
        cone("hood", (0.0, 0.78, 0.0), 0.28, 0.12, 0.34, dark, vertices=6)
        cube("face_void", (0.0, 0.74, 0.145), (0.16, 0.12, 0.025), eye)
        cube("wisp_l", (-0.28, 0.30, 0.0), (0.16, 0.10, 0.08), accent)
        cube("wisp_r", (0.28, 0.42, 0.0), (0.16, 0.10, 0.08), accent)
    export_glb(name)

def make_cloud_puff_v6() -> None:
    clear_scene()
    cloud = glow_mat("cloud_puff_white", (1.0, 1.0, 1.0), glow_strength=0.5)
    cloud_pink = glow_mat("cloud_puff_pink", (1.0, 0.85, 0.95), glow_strength=0.55)
    cloud_blue = glow_mat("cloud_puff_blue", (0.85, 0.95, 1.0), glow_strength=0.55)
    uv_sphere("c1", (0.0, 0.30, 0.0), (0.36, 0.26, 0.36), cloud, 12, 9)
    uv_sphere("c2", (-0.30, 0.24, 0.05), (0.28, 0.24, 0.28), cloud, 12, 9)
    uv_sphere("c3", ( 0.30, 0.22, 0.05), (0.30, 0.24, 0.30), cloud, 12, 9)
    uv_sphere("c4_pink", (0.0, 0.42, -0.12), (0.22, 0.18, 0.22), cloud_pink, 12, 8)
    uv_sphere("c5_blue", (0.18, 0.26, 0.24), (0.20, 0.16, 0.20), cloud_blue, 12, 8)
    export_glb("cloud_puff")

def make_tree_v6() -> None:
    clear_scene()
    bark = glow_mat("tree_bark", (0.42, 0.25, 0.12), roughness=0.96)
    leaf = glow_mat("tree_leaf", (0.22, 0.58, 0.24), roughness=0.96)
    leaf_dark = glow_mat("tree_leaf_dark", (0.12, 0.38, 0.18), roughness=0.96)

    cylinder("trunk", (0.0, 0.42, 0.0), 0.14, 0.84, bark, vertices=7)
    cube("root_l", (-0.16, 0.08, 0.05), (0.22, 0.08, 0.08), bark)
    cube("root_r", (0.16, 0.08, -0.05), (0.22, 0.08, 0.08), bark)
    uv_sphere("canopy_center", (0.0, 1.10, 0.0), (0.46, 0.36, 0.42), leaf, 9, 5)
    uv_sphere("canopy_l", (-0.28, 0.98, 0.02), (0.28, 0.24, 0.26), leaf_dark, 8, 5)
    uv_sphere("canopy_r", (0.26, 1.02, -0.02), (0.28, 0.24, 0.26), leaf, 8, 5)
    export_glb("tree")

def make_rock_v6(name, color) -> None:
    clear_scene()
    body = glow_mat(f"rock_{name}", color, roughness=0.98)
    dark = glow_mat(f"rock_{name}_dark", tuple(max(c * 0.70, 0.08) for c in color), roughness=0.98)

    ico_sphere("main", (0.0, 0.14, 0.0), (0.28, 0.18, 0.22), body, subdivisions=1)
    ico_sphere("side", (0.18, 0.11, -0.05), (0.15, 0.11, 0.12), dark, subdivisions=1)
    cube("flat_base", (0.0, 0.025, 0.0), (0.38, 0.05, 0.26), dark)
    export_glb(f"rock_{name}")

def make_flower_v6(idx, color) -> None:
    clear_scene()
    petal = glow_mat(f"flower_petal_{idx}", color, glow_strength=0.55)
    center = glow_mat(f"flower_center_{idx}", (1.0, 0.95, 0.40), glow_strength=0.85)
    stem = glow_mat(f"flower_stem_{idx}", (0.35, 0.85, 0.40), glow_strength=0.4)

    cylinder("stem", (0.0, 0.25, 0.0), 0.015, 0.50, stem, vertices=6)

    cone("leaf", (0.05, 0.25, 0.0), 0.030, 0.0, 0.05, stem, vertices=4)

    for k in range(5):
        angle = k * 1.2566
        r = 0.14
        x = math.cos(angle) * r
        z = math.sin(angle) * r
        uv_sphere(f"petal_{k}", (x, 0.56, z), (0.07, 0.05, 0.07), petal, 12, 8)

    uv_sphere("center", (0.0, 0.56, 0.0), (0.05, 0.05, 0.05), center, 12, 10)
    export_glb(f"flower_{idx}")

def make_hill_v6() -> None:
    clear_scene()
    body = glow_mat("hill_grass", (0.32, 0.62, 0.30), roughness=0.98)
    dirt = glow_mat("hill_dirt", (0.42, 0.27, 0.14), roughness=0.98)
    grass_dark = glow_mat("hill_grass_shadow", (0.20, 0.42, 0.22), roughness=0.98)

    cone("mound", (0.0, 0.20, 0.0), 0.78, 0.18, 0.40, body, vertices=9)
    cube("front_cut", (0.0, 0.08, 0.42), (0.58, 0.12, 0.08), dirt)
    cube("ridge", (-0.08, 0.38, -0.04), (0.46, 0.06, 0.08), grass_dark)
    export_glb("hill")

def make_poi_pillar_v6(name, body_color, glow_color) -> None:
    clear_scene()
    stone = glow_mat(f"poi_{name}_stone", (0.46, 0.42, 0.34), roughness=0.98)
    stone_dark = glow_mat(f"poi_{name}_stone_dark", (0.28, 0.25, 0.20), roughness=0.98)
    accent = glow_mat(f"poi_{name}_accent", body_color, roughness=0.92)

    cube("base", (0.0, 0.10, 0.0), (0.38, 0.20, 0.34), stone_dark)
    cube("shaft", (0.0, 0.42, 0.0), (0.22, 0.52, 0.20), stone)
    cube("cap", (0.0, 0.72, 0.0), (0.34, 0.12, 0.30), stone_dark)
    cube("rune", (0.0, 0.43, 0.105), (0.12, 0.18, 0.018), accent)
    cone("marker", (0.0, 0.88, 0.0), 0.10, 0.0, 0.16, accent, vertices=4)
    export_glb(f"poi_pillar_{name}")

def make_ground_disc_v6(name, color, glow) -> None:
    clear_scene()
    body = glow_mat(f"disc_{name}_soil", (0.30, 0.40, 0.22), roughness=0.98)
    rim = glow_mat(f"disc_{name}_rim", (0.18, 0.28, 0.15), roughness=0.98)
    grass = glow_mat(f"disc_{name}_grass", color, roughness=0.98)
    radius = 0.48 if name == "outer" else 0.30
    cylinder("soil_patch", (0.0, 0.025, 0.0), radius, 0.05, body, vertices=12)
    cylinder("grass_top", (0.0, 0.060, 0.0), radius * 0.84, 0.03, grass, vertices=12)
    cube("edge_notch", (radius * 0.35, 0.080, radius * 0.55), (radius * 0.22, 0.035, radius * 0.08), rim)
    export_glb(f"ground_disc_{name}")

def make_rabbit_v6() -> None:
    
    clear_scene()
    fur = glow_mat("rabbit_fur_warm", (1.0, 0.92, 0.85), glow_strength=0.3)
    inner_ear = glow_mat("rabbit_inner_ear", (1.0, 0.75, 0.80), glow_strength=0.5)
    eye = glow_mat("rabbit_eye_dark", (0.05, 0.02, 0.04), glow_strength=0.2)
    eye_shine = glow_mat("rabbit_eye_shine", (1.0, 1.0, 1.0), glow_strength=0.95)
    cheek = glow_mat("rabbit_cheek_pink", (1.0, 0.70, 0.75), glow_strength=0.6)

    uv_sphere("body", (0.0, 0.32, 0.0), (0.40, 0.34, 0.36), fur, 12, 9)

    uv_sphere("head", (0.0, 0.70, 0.05), (0.36, 0.32, 0.34), fur, 12, 9)

    uv_sphere("ear_l", (-0.13, 1.05, 0.05), (0.08, 0.08, 0.28), fur, 10, 6)
    uv_sphere("ear_r", ( 0.13, 1.05, 0.05), (0.08, 0.08, 0.28), fur, 10, 6)

    uv_sphere("inner_ear_l", (-0.13, 1.05, 0.12), (0.045, 0.045, 0.22), inner_ear, 8, 5)
    uv_sphere("inner_ear_r", ( 0.13, 1.05, 0.12), (0.045, 0.045, 0.22), inner_ear, 8, 5)

    uv_sphere("eye_l", (-0.13, 0.74, 0.30), (0.07, 0.09, 0.05), eye, 12, 10)
    uv_sphere("eye_r", ( 0.13, 0.74, 0.30), (0.07, 0.09, 0.05), eye, 12, 10)

    uv_sphere("shine_l", (-0.10, 0.78, 0.34), (0.025, 0.030, 0.015), eye_shine, 8, 6)
    uv_sphere("shine_r", ( 0.16, 0.78, 0.34), (0.025, 0.030, 0.015), eye_shine, 8, 6)

    cone("nose", (0.0, 0.62, 0.33), 0.025, 0.0, 0.03, glow_mat("rabbit_nose", (1.0, 0.55, 0.65), glow_strength=0.6), vertices=8)

    uv_sphere("cheek_l", (-0.22, 0.62, 0.22), (0.06, 0.05, 0.03), cheek, 10, 6)
    uv_sphere("cheek_r", ( 0.22, 0.62, 0.22), (0.06, 0.05, 0.03), cheek, 10, 6)

    uv_sphere("leg_l", (-0.12, 0.05, 0.05), (0.08, 0.08, 0.10), fur, 10, 8)
    uv_sphere("leg_r", ( 0.12, 0.05, 0.05), (0.08, 0.08, 0.10), fur, 10, 8)

    uv_sphere("tail", (0.0, 0.30, -0.30), (0.13, 0.13, 0.10), glow_mat("rabbit_tail", (1.0, 0.98, 0.95), glow_strength=0.5), 10, 6)
    export_glb_v6_eco("rabbit")

def make_berry_bush_v6() -> None:
    
    clear_scene()
    leaf = glow_mat("berry_leaf", (0.20, 0.85, 0.30), glow_strength=0.3)
    leaf_dark = glow_mat("berry_leaf_dark", (0.10, 0.60, 0.20), glow_strength=0.25)
    stem_brown = glow_mat("berry_stem", (0.50, 0.32, 0.20), glow_strength=0.15)
    fruit_red    = glow_mat("fruit_red",    (1.0, 0.20, 0.30), glow_strength=0.6)
    fruit_purple = glow_mat("fruit_purple", (0.70, 0.30, 0.95), glow_strength=0.6)
    fruit_blue   = glow_mat("fruit_blue",   (0.40, 0.65, 1.0), glow_strength=0.6)
    fruit_pink   = glow_mat("fruit_pink",   (1.0, 0.55, 0.80), glow_strength=0.6)
    fruit_gold   = glow_mat("fruit_gold",   (1.0, 0.85, 0.30), glow_strength=0.7)
    fruit_shine  = glow_mat("fruit_shine",  (1.0, 1.0, 1.0), glow_strength=0.9)
    fruit_colors = [fruit_red, fruit_purple, fruit_blue, fruit_pink, fruit_gold]

    cylinder("stem", (0.0, 0.18, 0.0), 0.06, 0.36, stem_brown, vertices=8)

    uv_sphere("leaf_main", (0.0, 0.60, 0.0), (0.40, 0.26, 0.40), leaf, 12, 9)
    uv_sphere("leaf_dark_1", (-0.20, 0.55, 0.12), (0.22, 0.18, 0.22), leaf_dark, 12, 8)
    uv_sphere("leaf_dark_2", ( 0.22, 0.65, -0.12), (0.20, 0.16, 0.20), leaf_dark, 12, 8)

    fruit_positions = [
        (-0.55, 0.42,  0.12, 0),
        ( 0.60, 0.48, -0.18, 1),
        ( 0.05, 0.60,  0.60, 2),
        ( 0.12, 0.55, -0.65, 3),
        (-0.36, 0.36, -0.48, 4),
        ( 0.36, 0.36,  0.48, 4),
        (-0.24, 0.60, -0.36, 0),
    ]
    for i, (x, y, z, color_idx) in enumerate(fruit_positions):
        uv_sphere(f"fruit_{i}", (x, y, z), (0.09, 0.09, 0.09), fruit_colors[color_idx], 12, 10)
        uv_sphere(f"fruit_shine_{i}", (x - 0.025, y + 0.030, z + 0.035), (0.028, 0.028, 0.018), fruit_shine, 8, 6)
    export_glb_v6_eco("berry_bush")

def make_berry_fruit_v6() -> None:
    clear_scene()
    fruit_red = glow_mat("fruit_red", (1.0, 0.20, 0.30), glow_strength=0.7)
    shine = glow_mat("fruit_shine", (1.0, 1.0, 1.0), glow_strength=0.95)
    stem = glow_mat("fruit_stem", (0.40, 0.25, 0.15), glow_strength=0.2)

    uv_sphere("berry", (0.0, 0.10, 0.0), (0.10, 0.10, 0.10), fruit_red, 12, 10)
    uv_sphere("shine", (-0.025, 0.13, 0.030), (0.030, 0.030, 0.018), shine, 8, 6)
    cylinder("stem", (0.0, 0.22, 0.0), 0.010, 0.05, stem, vertices=6)
    cone("leaf", (0.025, 0.22, 0.0), 0.015, 0.0, 0.025, stem, vertices=4)
    export_glb_v6_eco("berry_fruit")

def make_co2_bubble_v6() -> None:
    clear_scene()
    bubble = mat("co2_bubble", (0.65, 0.95, 1.0), roughness=0.05, alpha=0.45)
    inner = mat("co2_inner", (0.85, 1.0, 1.0), roughness=0.1, emissive=(0.50, 0.85, 1.0))
    rim = glow_mat("co2_rim", (0.80, 0.95, 1.0), glow_strength=0.9)

    uv_sphere("bubble_big", (0.0, 0.18, 0.0), (0.25, 0.25, 0.25), bubble, 12, 9)
    uv_sphere("bubble_mid", (0.22, 0.10, 0.18), (0.12, 0.12, 0.12), bubble, 10, 8)
    uv_sphere("bubble_small", (-0.18, 0.25, 0.24), (0.09, 0.09, 0.09), bubble, 10, 6)
    uv_sphere("inner", (0.0, 0.18, 0.0), (0.12, 0.12, 0.12), inner, 10, 8)
    uv_sphere("rim", (-0.10, 0.26, 0.12), (0.05, 0.05, 0.05), rim, 10, 8)
    export_glb_v6_eco("co2_bubble")

def export_glb_v6_eco(name):
    import models_lib
    eco_dir = models_lib.ROOT / "assets" / "procedural" / "eco"
    eco_dir.mkdir(parents=True, exist_ok=True)
    bpy_path_fix = models_lib.OUT_DIR
    models_lib.OUT_DIR = eco_dir
    try:
        models_lib.export_glb(name)
    finally:
        models_lib.OUT_DIR = bpy_path_fix

def main() -> None:
    print("=== building v6-balanced (1-3 色 + 比例人小他大) ===")

    print("\n--- eco/ ---")
    print("[1/4] rabbit (0.9m, 2 色 暖白+粉)")
    make_rabbit_v6()
    print("[2/4] berry_bush (0.8m, 叶绿 + 5 散开浆果)")
    make_berry_bush_v6()
    print("[3/4] berry_fruit (0.22m, 红+高光)")
    make_berry_fruit_v6()
    print("[4/4] co2_bubble (0.50m, 半透明蓝发光)")
    make_co2_bubble_v6()

    print("\n--- pretty/ ---")

    print("[2-6/23] monsters x5 (0.55m, 3 色)")
    make_monster_v6("monster_snake",     (0.55, 0.95, 0.30), (0.95, 1.0, 0.30))
    make_monster_v6("monster_frost_elf", (0.45, 0.80, 1.0),  (0.70, 0.95, 1.0))
    make_monster_v6("monster_sand_wurm", (1.0, 0.80, 0.30),  (1.0, 0.95, 0.40))
    make_monster_v6("monster_treant",    (0.50, 0.35, 0.20),  (0.70, 0.55, 0.25))
    make_monster_v6("monster_aether_wraith", (0.85, 0.50, 1.0),  (1.0, 0.75, 1.0))

    print("[7/23] cloud_puff (0.45m, 3 色 白+粉+蓝)")
    make_cloud_puff_v6()

    print("[8/23] tree (1.0m, 2 色 叶绿+深绿+棕干)")
    make_tree_v6()

    print("[9-11/23] rocks x3 (0.28m, 1 色)")
    make_rock_v6("dark", (0.55, 0.55, 0.60))
    make_rock_v6("mid",  (0.70, 0.65, 0.55))
    make_rock_v6("moss", (0.55, 0.75, 0.55))

    print("[12-16/23] flowers x5 (0.7m, 2 色 花瓣+中心黄)")
    flower_colors = [
        (1.0, 0.50, 0.70),
        (1.0, 0.95, 0.40),
        (0.80, 0.55, 1.0),
        (1.0, 0.65, 0.30),
        (1.0, 0.40, 0.50),
    ]
    for i, c in enumerate(flower_colors):
        make_flower_v6(i, c)

    print("[17/23] hill (0.9m, 2 色 主绿+深绿+小花)")
    make_hill_v6()

    print("[18-21/23] poi_pillars x4 (0.75m, 2 色 柱+顶发光)")
    make_poi_pillar_v6("red",  (1.0, 0.30, 0.40), (1.0, 0.55, 0.20))
    make_poi_pillar_v6("cyan", (0.30, 0.65, 1.0),  (0.40, 0.95, 1.0))
    make_poi_pillar_v6("pink", (1.0, 0.50, 0.80), (1.0, 0.30, 0.65))
    make_poi_pillar_v6("gold", (1.0, 0.85, 0.30), (1.0, 0.95, 0.50))

    print("[22-23/23] ground_discs (0.50m 外 + 0.30m 内)")
    make_ground_disc_v6("outer", (0.45, 0.85, 0.50), (0.65, 1.0, 0.40))
    make_ground_disc_v6("inner", (0.75, 0.95, 0.45), (0.95, 1.0, 0.30))

    manifest = {
        "version": 6,
        "spec": "v6-balanced: 1-3 色配色 + 比例人小他大",
        "format": "glb",
        "scale": {
            "monster": "0.55m 直径 (放大)",
            "tree": "1.0m 高 (放大)",
            "rock": "0.28m 直径 (放大)",
            "flower": "0.7m 高 (放大)",
            "hill": "0.9m 宽 (放大)",
            "poi_pillar": "0.75m 高 (放大)",
            "ground_disc_outer": "0.50m (放大)",
            "ground_disc_inner": "0.30m (放大)",
            "rabbit": "0.9m 高 (放大)",
            "berry_bush": "0.8m 高 (放大, 散开)",
            "berry_fruit": "0.22m (放大)",
            "co2_bubble": "0.50m (放大)",
        },
    }
    mp = models_lib.ROOT / "assets" / "procedural" / "pretty" / "MANIFEST.json"
    mp.write_text(json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8")
    eco_manifest = models_lib.ROOT / "assets" / "procedural" / "eco" / "MANIFEST.json"
    eco_manifest.write_text(json.dumps({"version": 6, "spec": "v6-balanced"}, indent=2), encoding="utf-8")
    print(f"\nMANIFEST -> {mp}")
    print("=== done ===")

if __name__ == "__main__":
    main()
