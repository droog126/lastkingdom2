

from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from models_lib import (
    clear_scene, cube, cone, cylinder, export_glb, ico_sphere,
    mat, merge_into, uv_sphere,
)

import models_lib

def make_player_avatar_v2() -> None:
    
    clear_scene()

    skin = mat("skin_warm", (0.98, 0.82, 0.68))
    hat_straw = mat("hat_straw", (0.78, 0.62, 0.32), roughness=0.95)
    hat_band = mat("hat_band", (0.45, 0.30, 0.18), roughness=0.85)
    eye_blue = mat("eye_blue", (0.25, 0.50, 0.85), emissive=(0.10, 0.25, 0.50))
    mouth = mat("mouth_dark", (0.55, 0.28, 0.22))
    nose = mat("nose_skin", (0.92, 0.75, 0.62))
    shirt_brown = mat("shirt_brown", (0.62, 0.42, 0.25), roughness=0.85)
    shirt_dark = mat("shirt_dark", (0.45, 0.28, 0.15))
    belt_rope = mat("belt_rope", (0.55, 0.42, 0.25), roughness=0.95)
    hip_grey = mat("hip_grey_linen", (0.62, 0.58, 0.50))
    arm_shirt = mat("arm_shirt", (0.58, 0.38, 0.22))
    arm_skin = mat("arm_skin_sunburnt", (0.92, 0.72, 0.55))
    pant_linen = mat("pant_linen", (0.55, 0.48, 0.35), roughness=0.90)
    pant_dark = mat("pant_dark_linen", (0.40, 0.35, 0.25))
    shoe_straw = mat("shoe_straw", (0.32, 0.22, 0.12), roughness=0.95)

    cube("head", (0.0, 2.55, 0.0), (0.70, 0.70, 0.70), skin)

    cube("eye_l", (-0.18, 2.62, 0.34), (0.10, 0.10, 0.04), eye_blue)
    cube("eye_r", ( 0.18, 2.62, 0.34), (0.10, 0.10, 0.04), eye_blue)

    cube("mouth", (0.0, 2.42, 0.35), (0.16, 0.04, 0.04), mouth)

    cone("nose", (0.0, 2.50, 0.36), 0.05, 0.0, 0.08, nose, vertices=4)

    cylinder("hat_top", (0.0, 2.97, 0.0), 0.25, 0.18, hat_straw, vertices=10)

    cylinder("hat_brim", (0.0, 2.84, 0.0), 0.45, 0.05, hat_straw, vertices=12)

    cylinder("hat_band", (0.0, 2.89, 0.0), 0.26, 0.04, hat_band, vertices=10)

    cube("shirt", (0.0, 1.20, 0.0), (0.95, 1.40, 0.55), shirt_brown)

    cube("collar", (0.0, 1.78, 0.10), (0.40, 0.10, 0.10), shirt_dark)

    cube("belt", (0.0, 0.40, 0.0), (0.85, 0.14, 0.55), belt_rope)

    cube("hip", (0.0, 0.10, 0.0), (0.85, 0.30, 0.55), hip_grey)

    for sx, label in ((-0.50, "l"), (0.50, "r")):
        cube(f"shoulder_{label}", (sx, 1.85, 0.0), (0.22, 0.20, 0.22), shirt_dark)

    for sx, label in ((-0.50, "l"), (0.50, "r")):

        cube(f"upper_arm_{label}", (sx, 1.45, 0.0), (0.22, 0.55, 0.22), arm_shirt)

        uv_sphere(f"elbow_{label}", (sx, 1.10, 0.0), (0.13, 0.13, 0.13), arm_skin, 6, 4)

        cube(f"forearm_{label}", (sx, 0.65, 0.0), (0.20, 0.75, 0.20), arm_skin)

        cube(f"fist_{label}", (sx, 0.20, 0.0), (0.22, 0.18, 0.22), arm_skin)

    for sx, label in ((-0.20, "l"), (0.20, "r")):

        cube(f"thigh_{label}", (sx, -0.20, 0.0), (0.30, 0.65, 0.30), pant_linen)

        uv_sphere(f"knee_{label}", (sx, -0.55, 0.05), (0.16, 0.16, 0.16), pant_dark, 6, 4)

        cube(f"shin_{label}", (sx, -0.95, 0.0), (0.28, 0.55, 0.28), pant_linen)

        cube(f"shoe_{label}", (sx, -1.35, 0.05), (0.30, 0.15, 0.45), shoe_straw)

    export_glb("player_avatar")

def make_sword() -> None:
    clear_scene()
    blade = mat("sword_blade", (0.85, 0.88, 0.92), metallic=0.95, roughness=0.15)
    handle = mat("sword_handle", (0.40, 0.20, 0.10), roughness=0.75)
    guard = mat("sword_guard", (0.85, 0.68, 0.20), metallic=0.85, roughness=0.30)
    pommel = mat("sword_pommel", (0.30, 0.30, 0.35), metallic=0.7, roughness=0.40)

    cube("blade",  (0.0, -0.70, 0.0), (0.10, 0.95, 0.04), blade)
    cube("ridge",  (0.0, -0.70, 0.02), (0.04, 0.95, 0.02), pommel)
    cube("guard",  (0.0, -0.20, 0.0), (0.32, 0.06, 0.10), guard)
    cube("handle", (0.0,  0.00, 0.0), (0.08, 0.32, 0.08), handle)
    uv_sphere("pommel_top", (0.0, 0.20, 0.0), (0.08, 0.08, 0.08), pommel, 8, 6)

    export_glb("sword")

if __name__ == "__main__":
    print("=== building sword only ===")
    print("player_avatar.glb is retired; use sokpop_gatherer.glb instead.")
    make_sword()
    print("=== done ===")