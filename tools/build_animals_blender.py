from __future__ import annotations

"""Build the small farm and woodland animals used by the game.

The animals are intentionally low-poly, but their primary forms use rounded
volumes and explicit contact geometry so they remain readable in the game
camera.  Keep this file deterministic: asset names and scale contracts are
part of the runtime content API.
"""

import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import models_lib


def body_volume(name, loc, scale, material):
    return models_lib.ico_sphere(name, loc, scale, material, subdivisions=2)


def soft_volume(name, loc, scale, material):
    return models_lib.uv_sphere(name, loc, scale, material, segments=12, rings=6)


def leg(name, start, end, radius, material):
    models_lib.cylinder_between(name, start, end, radius, material, vertices=8)
    models_lib.ico_sphere(f"{name}_hoof", end, (radius * 1.35, radius * 1.15, radius * 0.55), material, 1)


def articulated_leg(name, start, joint, end, radius, material):
    models_lib.cylinder_between(f"{name}_upper", start, joint, radius, material, vertices=8)
    models_lib.cylinder_between(f"{name}_lower", joint, end, radius * 0.92, material, vertices=8)
    models_lib.ico_sphere(
        f"{name}_hoof", end, (radius * 1.35, radius * 1.15, radius * 0.55), material, 1
    )


def ear(name, side, base_x, base_z, width, height, material, inner=None):
    sign = float(side)
    points = [
        (base_x - width * 0.5, base_z),
        (base_x + width * 0.42, base_z),
        (base_x + width * 0.08, base_z + height),
    ]
    models_lib.prism(name, points, width * 0.42, material, y=sign * width * 0.22, bevel=0.018)
    if inner is not None:
        inner_points = [
            (base_x - width * 0.18, base_z + height * 0.10),
            (base_x + width * 0.18, base_z + height * 0.10),
            (base_x + width * 0.07, base_z + height * 0.72),
        ]
        models_lib.prism(
            f"{name}_inner",
            inner_points,
            width * 0.12,
            inner,
            y=sign * width * 0.46,
            bevel=0.008,
        )


def eye_pair(prefix, x, y, z, material, radius=0.045):
    for side in (-1.0, 1.0):
        soft_volume(f"{prefix}_eye_{'l' if side < 0 else 'r'}", (x, y + side * 0.27, z), (radius, radius * 0.55, radius), material)


def tail_tip(name, start, end, radius, material, tip_material=None):
    models_lib.cylinder_between(name, start, end, radius, material, vertices=8)
    if tip_material is not None:
        soft_volume(f"{name}_tip", end, (radius * 1.45, radius * 1.25, radius * 1.45), tip_material)


def build_pig():
    pink = models_lib.mat("pig_pink", (0.83, 0.39, 0.43), roughness=0.92)
    light = models_lib.mat("pig_light", (0.98, 0.66, 0.68), roughness=0.92)
    dark = models_lib.mat("pig_dark", (0.26, 0.08, 0.09), roughness=0.9)
    body_volume("pig_body", (0.0, 0.0, 0.55), (0.72, 0.50, 0.43), pink)
    body_volume("pig_chest", (0.42, 0.0, 0.60), (0.40, 0.44, 0.46), light)
    body_volume("pig_head", (0.78, 0.0, 0.78), (0.39, 0.38, 0.36), pink)
    soft_volume("pig_snout", (1.08, 0.0, 0.68), (0.22, 0.29, 0.17), light)
    for side in (-1, 1):
        ear("pig_ear", side, 0.70, 1.02, 0.24, 0.22, pink)
    eye_pair("pig", 0.98, 0.0, 0.87, dark, 0.045)
    for side in (-1, 1):
        leg(f"pig_leg_{side}_front", (0.37, side * 0.30, 0.50), (0.37, side * 0.30, 0.09), 0.095, pink)
        leg(f"pig_leg_{side}_back", (-0.40, side * 0.30, 0.48), (-0.40, side * 0.30, 0.09), 0.095, pink)
    tail_tip("pig_tail", (-0.68, 0.03, 0.70), (-0.88, 0.03, 0.88), 0.045, pink, light)
    models_lib.uv_sphere("pig_nostril_l", (1.20, -0.12, 0.70), (0.028, 0.018, 0.028), dark, 8, 4)
    models_lib.uv_sphere("pig_nostril_r", (1.20, 0.12, 0.70), (0.028, 0.018, 0.028), dark, 8, 4)


def build_sheep():
    wool = models_lib.mat("sheep_wool", (0.90, 0.88, 0.80), roughness=1.0)
    wool_light = models_lib.mat("sheep_wool_light", (1.0, 0.98, 0.88), roughness=1.0)
    face = models_lib.mat("sheep_face", (0.18, 0.13, 0.12), roughness=0.94)
    horn = models_lib.mat("sheep_horn", (0.66, 0.48, 0.28), roughness=0.96)
    body_volume("sheep_body", (-0.02, 0.0, 0.62), (0.72, 0.50, 0.47), wool)
    body_volume("sheep_wool_front", (0.38, 0.0, 0.68), (0.42, 0.45, 0.48), wool_light)
    body_volume("sheep_head", (0.82, 0.0, 0.85), (0.34, 0.32, 0.34), face)
    soft_volume("sheep_muzzle", (1.08, 0.0, 0.78), (0.15, 0.22, 0.16), face)
    eye_pair("sheep", 0.98, 0.0, 0.95, horn, 0.034)
    for side in (-1, 1):
        ear("sheep_ear", side, 0.78, 1.10, 0.22, 0.16, face)
        models_lib.cone_between(
            f"sheep_horn_{side}",
            (0.72, side * 0.22, 1.05),
            (0.57, side * 0.30, 1.27),
            0.075,
            horn,
            vertices=7,
        )
        leg(f"sheep_leg_{side}_front", (0.38, side * 0.29, 0.55), (0.38, side * 0.29, 0.08), 0.10, face)
        leg(f"sheep_leg_{side}_back", (-0.42, side * 0.29, 0.52), (-0.42, side * 0.29, 0.08), 0.10, face)
    soft_volume("sheep_tail", (-0.69, 0.0, 0.76), (0.16, 0.14, 0.16), wool_light)


def build_cow():
    white = models_lib.mat("cow_cream", (0.92, 0.89, 0.80), roughness=0.96)
    brown = models_lib.mat("cow_patch", (0.22, 0.10, 0.06), roughness=0.96)
    muzzle = models_lib.mat("cow_muzzle", (0.82, 0.45, 0.40), roughness=0.94)
    horn = models_lib.mat("cow_horn", (0.74, 0.58, 0.34), roughness=0.96)
    body_volume("cow_body", (0.0, 0.0, 0.82), (0.86, 0.56, 0.55), white)
    body_volume("cow_chest", (0.48, 0.0, 0.86), (0.46, 0.52, 0.58), white)
    for index, (x, y, z, sx, sy, sz) in enumerate(
        ((-0.34, -0.50, 0.88, 0.22, 0.07, 0.24), (0.10, 0.47, 0.98, 0.27, 0.08, 0.18), (-0.18, -0.45, 0.54, 0.18, 0.07, 0.16))
    ):
        soft_volume(f"cow_spot_{index}", (x, y, z), (sx, sy, sz), brown)
    body_volume("cow_head", (0.98, 0.0, 1.04), (0.42, 0.40, 0.42), white)
    soft_volume("cow_muzzle", (1.30, 0.0, 0.90), (0.24, 0.30, 0.18), muzzle)
    eye_pair("cow", 1.18, 0.0, 1.15, brown, 0.043)
    for side in (-1, 1):
        ear("cow_ear", side, 0.88, 1.38, 0.26, 0.18, brown)
        models_lib.cone_between(
            f"cow_horn_{side}",
            (0.82, side * 0.24, 1.35),
            (0.72, side * 0.30, 1.62),
            0.07,
            horn,
            vertices=7,
        )
        leg(f"cow_leg_{side}_front", (0.48, side * 0.36, 0.68), (0.48, side * 0.36, 0.08), 0.115, brown)
        leg(f"cow_leg_{side}_back", (-0.50, side * 0.36, 0.66), (-0.50, side * 0.36, 0.08), 0.115, brown)
    tail_tip("cow_tail", (-0.82, 0.0, 1.02), (-1.02, 0.02, 1.34), 0.045, brown, brown)


def build_chicken():
    feather = models_lib.mat("chicken_feather", (0.88, 0.68, 0.38), roughness=0.94)
    feather_light = models_lib.mat("chicken_feather_light", (1.0, 0.86, 0.53), roughness=0.94)
    wing = models_lib.mat("chicken_wing", (0.67, 0.38, 0.16), roughness=0.95)
    red = models_lib.mat("chicken_comb", (0.74, 0.08, 0.07), roughness=0.92)
    beak = models_lib.mat("chicken_beak", (0.94, 0.57, 0.08), roughness=0.92)
    dark = models_lib.mat("chicken_eye", (0.04, 0.025, 0.02), roughness=0.9)

    body_volume("chicken_body", (0.0, 0.0, 0.42), (0.40, 0.32, 0.38), feather)
    body_volume("chicken_breast", (0.24, -0.02, 0.48), (0.28, 0.30, 0.34), feather_light)
    for side in (-1, 1):
        models_lib.ico_sphere(f"chicken_wing_{side}", (-0.02, side * 0.30, 0.44), (0.28, 0.10, 0.28), wing, 1)
        leg(f"chicken_leg_{side}", (0.06, side * 0.14, 0.24), (0.06, side * 0.14, 0.06), 0.035, beak)
        for toe in (-1, 0, 1):
            models_lib.cylinder_between(
                f"chicken_toe_{side}_{toe}",
                (0.10, side * 0.14, 0.07),
                (0.20, side * 0.14 + toe * 0.08, 0.045),
                0.018,
                beak,
                vertices=6,
            )
    body_volume("chicken_head", (0.40, 0.0, 0.72), (0.25, 0.24, 0.26), feather_light)
    for index, (x, z) in enumerate(((0.32, 0.97), (0.43, 1.02), (0.54, 0.97))):
        models_lib.ico_sphere(f"chicken_comb_{index}", (x, 0.0, z), (0.08, 0.08, 0.10), red, 1)
    models_lib.cone_between("chicken_beak", (0.60, 0.0, 0.72), (0.78, 0.0, 0.70), 0.09, beak, vertices=6)
    eye_pair("chicken", 0.53, 0.0, 0.80, dark, 0.035)
    soft_volume("chicken_wattle", (0.56, 0.0, 0.57), (0.08, 0.07, 0.10), red)


def build_rabbit(brown=False):
    fur = models_lib.mat("rabbit_brown_fur" if brown else "rabbit_fur", (0.54, 0.31, 0.16) if brown else (0.76, 0.73, 0.68), roughness=0.97)
    fur_light = models_lib.mat("rabbit_brown_light" if brown else "rabbit_light", (0.72, 0.48, 0.27) if brown else (0.92, 0.87, 0.82), roughness=0.97)
    inner = models_lib.mat("rabbit_brown_inner" if brown else "rabbit_inner", (0.70, 0.35, 0.30) if brown else (0.88, 0.52, 0.55), roughness=0.94)
    eye = models_lib.mat("rabbit_eye", (0.16, 0.045, 0.04), roughness=0.9)

    stem = "rabbit_brown" if brown else "rabbit"
    body_volume(f"{stem}_body", (-0.04, 0.0, 0.52), (0.62, 0.44, 0.48), fur)
    body_volume(f"{stem}_hind_quarter", (-0.38, 0.0, 0.62), (0.38, 0.42, 0.45), fur_light)
    body_volume(f"{stem}_head", (0.47, 0.0, 0.82), (0.34, 0.34, 0.35), fur)
    soft_volume(f"{stem}_muzzle", (0.72, 0.0, 0.73), (0.16, 0.22, 0.14), fur_light)
    eye_pair(stem, 0.65, 0.0, 0.91, eye, 0.038)
    for side in (-1, 1):
        ear(f"{stem}_ear", side, 0.38, 1.06, 0.22, 0.62, fur, inner)
        articulated_leg(
            f"{stem}_front_leg_{side}",
            (0.30, side * 0.24, 0.38),
            (0.30, side * 0.24, 0.21),
            (0.30, side * 0.24, 0.08),
            0.075,
            fur,
        )
        articulated_leg(
            f"{stem}_hind_leg_{side}",
            (-0.40, side * 0.28, 0.46),
            (-0.40, side * 0.28, 0.25),
            (-0.40, side * 0.28, 0.08),
            0.105,
            fur_light,
        )
    soft_volume(f"{stem}_tail", (-0.72, 0.0, 0.72), (0.18, 0.18, 0.18), fur_light)


def build_deer(fawn=False):
    coat = models_lib.mat("deer_fawn_coat" if fawn else "deer_coat", (0.70, 0.42, 0.20) if fawn else (0.48, 0.24, 0.10), roughness=0.96)
    chest = models_lib.mat("deer_fawn_chest" if fawn else "deer_chest", (0.84, 0.62, 0.34) if fawn else (0.62, 0.34, 0.14), roughness=0.96)
    dark = models_lib.mat("deer_dark", (0.18, 0.09, 0.045), roughness=0.96)
    antler = models_lib.mat("deer_antler", (0.68, 0.50, 0.28), roughness=0.98)
    spot = models_lib.mat("deer_spot", (0.92, 0.75, 0.45), roughness=0.96)

    stem = "deer_fawn" if fawn else "deer"
    body_volume(f"{stem}_body", (-0.06, 0.0, 0.90), (0.82, 0.48, 0.58), coat)
    body_volume(f"{stem}_chest", (0.46, 0.0, 1.02), (0.42, 0.46, 0.70), chest)
    models_lib.cylinder_between(f"{stem}_neck", (0.50, 0.0, 1.12), (0.72, 0.0, 1.53), 0.22, coat, vertices=8)
    body_volume(f"{stem}_head", (0.84, 0.0, 1.66), (0.36, 0.34, 0.34), coat)
    soft_volume(f"{stem}_snout", (1.13, 0.0, 1.56), (0.22, 0.25, 0.17), dark)
    eye_pair(stem, 1.02, 0.0, 1.76, dark, 0.038)
    for side in (-1, 1):
        ear(f"{stem}_ear", side, 0.76, 1.87, 0.24, 0.24, coat)
        if not fawn:
            models_lib.cylinder_between(f"{stem}_antler_{side}_main", (0.74, side * 0.20, 1.87), (0.61, side * 0.28, 2.30), 0.055, antler, vertices=7)
            models_lib.cylinder_between(f"{stem}_antler_{side}_branch", (0.66, side * 0.25, 2.10), (0.46, side * 0.32, 2.22), 0.042, antler, vertices=7)
        leg(f"{stem}_front_leg_{side}", (0.40, side * 0.31, 0.70), (0.40, side * 0.31, 0.08), 0.085, dark)
        leg(f"{stem}_back_leg_{side}", (-0.46, side * 0.31, 0.67), (-0.46, side * 0.31, 0.08), 0.09, dark)
    tail_tip(f"{stem}_tail", (-0.80, 0.0, 1.05), (-1.02, 0.0, 1.32), 0.05, coat, chest)
    if fawn:
        for index, (x, y, z) in enumerate(((-0.32, -0.45, 1.10), (0.0, -0.45, 1.18), (0.28, -0.43, 1.06), (-0.18, 0.43, 1.02))):
            soft_volume(f"{stem}_spot_{index}", (x, y, z), (0.10, 0.035, 0.09), spot)


def build_fox(silver=False):
    orange = models_lib.mat("fox_silver_coat" if silver else "fox_coat", (0.38, 0.43, 0.49) if silver else (0.78, 0.22, 0.045), roughness=0.95)
    cream = models_lib.mat("fox_silver_cream" if silver else "fox_cream", (0.88, 0.90, 0.88) if silver else (0.98, 0.73, 0.38), roughness=0.95)
    black = models_lib.mat("fox_black", (0.07, 0.035, 0.025), roughness=0.92)
    ear_dark = models_lib.mat("fox_silver_ear" if silver else "fox_ear", (0.16, 0.18, 0.22) if silver else (0.44, 0.08, 0.025), roughness=0.95)

    stem = "fox_silver" if silver else "fox"
    body_volume(f"{stem}_body", (-0.04, 0.0, 0.62), (0.75, 0.44, 0.48), orange)
    body_volume(f"{stem}_chest", (0.42, 0.0, 0.68), (0.40, 0.42, 0.50), cream)
    body_volume(f"{stem}_head", (0.78, 0.0, 0.86), (0.35, 0.34, 0.36), orange)
    soft_volume(f"{stem}_muzzle", (1.08, 0.0, 0.76), (0.26, 0.25, 0.17), cream)
    soft_volume(f"{stem}_nose", (1.28, 0.0, 0.77), (0.07, 0.10, 0.07), black)
    eye_pair(stem, 1.00, 0.0, 0.96, black, 0.042)
    for side in (-1, 1):
        ear(f"{stem}_ear", side, 0.70, 1.08, 0.26, 0.48, orange, ear_dark)
        leg(f"{stem}_front_leg_{side}", (0.34, side * 0.27, 0.46), (0.34, side * 0.27, 0.08), 0.075, orange)
        leg(f"{stem}_back_leg_{side}", (-0.42, side * 0.27, 0.44), (-0.42, side * 0.27, 0.08), 0.08, orange)
    models_lib.cylinder_between(f"{stem}_tail_base", (-0.64, 0.0, 0.78), (-0.98, 0.02, 0.98), 0.16, orange, vertices=8)
    tail_tip(f"{stem}_tail_tip", (-0.93, 0.02, 0.96), (-1.20, 0.04, 0.92), 0.13, orange, cream)


BUILDERS = {
    "pig": build_pig,
    "sheep": build_sheep,
    "cow": build_cow,
    "chicken": build_chicken,
    "rabbit": lambda: build_rabbit(False),
    "rabbit_brown": lambda: build_rabbit(True),
    "deer": lambda: build_deer(False),
    "deer_fawn": lambda: build_deer(True),
    "fox": lambda: build_fox(False),
    "fox_silver": lambda: build_fox(True),
}


def main() -> int:
    selected = os.environ.get("LK2_MODEL_ONLY")
    names = [selected] if selected else list(BUILDERS)
    for name in names:
        if name not in BUILDERS:
            raise SystemExit(f"unknown animal asset: {name}")
        models_lib.clear_scene()
        BUILDERS[name]()
        models_lib.export_glb(name, collection="animals")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
