"""build_player_v3.py — 重做 player_avatar.glb v3, 用 z 当 height (Blender Z-up).

修复: v2 用 y 当 height, 但 Blender Z-up, 高度应该是 z.
所有 cube/sphere 位置从 (x, y, z) 改成 (x, z, y), 即交换 y 和 z.
cylinder/cone 不变 (depth 默认沿 z).
"""
from __future__ import annotations
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import models_lib
models_lib.OUT_DIR = Path(r"F:\rustProject\lastkingdom2\assets\procedural\pretty")

from models_lib import clear_scene, cube, cone, cylinder, export_glb, ico_sphere, mat, merge_into, uv_sphere


# 工具: 旧坐标 (x, y_blender, z_blender) → 新坐标 (x, y_bevy, z_bevy)
# Blender Z-up → glTF Y-up: y_blender=前后 → z_bevy, z_blender=高度 → y_bevy
def pos(x, y_b, z_b):
    """旧 (x, y_blender, z_blender) → (x, y_bevy, z_bevy).
    y_bevy = z_blender (height)
    z_bevy = y_blender (前后, 翻转以让 -y (前面) → +z (前))
    """
    return (x, z_b, y_b)


# ---------- v3 玩家: 平民 (草帽+布衣+麻裤+草鞋) ----------
def make_player_avatar_v3() -> None:
    clear_scene()

    # 材质
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

    # ---- 头 (h=0.7, 中心 height=2.90) ----
    # Blender Z-up, 转换到 glTF Y-up: 想要 glTF head 中心 (0, 2.90, 0)
    # 反推 Blender: x=0, y_b = -z_gl = 0, z_b = y_gl = 2.90 → Blender (0, 0, 2.90)
    cube("head", (0.0, 0.0, 2.90), (0.7, 0.7, 0.7), skin)
    # 双眼: 想要 glTF (x=±0.18, y=2.65, z=0.36)
    # 反推 Blender: y_b = -z_gl = -0.36, z_b = y_gl = 2.65
    cube("eye_l", (-0.18, -0.36, 2.65), (0.10, 0.10, 0.04), eye_blue)
    cube("eye_r", ( 0.18, -0.36, 2.65), (0.10, 0.10, 0.04), eye_blue)
    # 嘴: glTF (0, 2.45, 0.36)
    cube("mouth", (0.0, -0.36, 2.45), (0.16, 0.04, 0.16), mouth)
    # 鼻: glTF (0, 2.50, 0.36)
    cone("nose", (0.0, -0.36, 2.50), 0.05, 0.0, 0.08, nose, vertices=4)

    # ---- 草帽 (cylinder 沿 z 立, depth=0.18=0.18m 高) ----
    # 帽顶: glTF (0, 2.97, 0) → Blender (0, 0, 2.97)
    cylinder("hat_top", (0.0, 0.0, 2.97), 0.25, 0.18, hat_straw, vertices=10)
    cylinder("hat_brim", (0.0, 0.0, 2.84), 0.45, 0.05, hat_straw, vertices=12)
    cylinder("hat_band", (0.0, 0.0, 2.89), 0.26, 0.04, hat_band, vertices=10)

    # ---- 躯干 (shirt) ----
    # glTF (0, 1.20, 0) → Blender (0, 0, 1.20)
    # scale: glTF (x, y=height, z=前后) → Blender scale 写 (x, z, y) 因为 Blender
    # 把 y 当前后, z 当 height.
    cube("shirt", (0.0, 0.0, 1.20), (0.95, 1.40, 0.55), shirt_brown)
    cube("collar", (0.0, -0.10, 1.78), (0.40, 0.10, 0.10), shirt_dark)
    cube("belt", (0.0, 0.0, 0.40), (0.85, 0.14, 0.55), belt_rope)
    cube("hip", (0.0, 0.0, 0.10), (0.85, 0.30, 0.55), hip_grey)

    # ---- 肩 ----
    for sx, label in ((-0.50, "l"), (0.50, "r")):
        cube(f"shoulder_{label}", (sx, 0.0, 1.85), (0.22, 0.20, 0.22), shirt_dark)

    # ---- 双臂 ----
    for sx, label in ((-0.50, "l"), (0.50, "r")):
        # 上臂: scale v2 (0.22, 0.55, 0.22) - 0.55 height → v3 scale (0.22, 0.22, 0.55) z 高度
        cube(f"upper_arm_{label}", (sx, 0.0, 1.45), (0.22, 0.22, 0.55), arm_shirt)
        # 肘 (sphere 各向同性, 改位置即可)
        uv_sphere(f"elbow_{label}", (sx, 0.0, 1.10), (0.13, 0.13, 0.13), arm_skin, 6, 4)
        # 前臂: v2 (0.20, 0.75, 0.20) - 0.75 height → v3 (0.20, 0.20, 0.75) z 高度
        cube(f"forearm_{label}", (sx, 0.0, 0.65), (0.20, 0.20, 0.75), arm_skin)
        # 拳: v2 (0.22, 0.18, 0.22) - 各向异性
        cube(f"fist_{label}", (sx, 0.0, 0.20), (0.22, 0.18, 0.22), arm_skin)

    # ---- 双腿 ----
    for sx, label in ((-0.20, "l"), (0.20, "r")):
        # 大腿: v2 (0.30, 0.65, 0.30) - 0.65 height → v3 (0.30, 0.30, 0.65) z 高度
        cube(f"thigh_{label}", (sx, 0.0, -0.20), (0.30, 0.30, 0.65), pant_linen)
        # 膝
        uv_sphere(f"knee_{label}", (sx, -0.05, -0.55), (0.16, 0.16, 0.16), pant_dark, 6, 4)
        # 小腿: v2 (0.28, 0.55, 0.28) → v3 (0.28, 0.28, 0.55) z 高度
        cube(f"shin_{label}", (sx, 0.0, -0.95), (0.28, 0.28, 0.55), pant_linen)
        # 草鞋: v2 (0.30, 0.15, 0.45) - 0.15 height, 0.45 前后 → v3 (0.30, 0.45, 0.15) y/z 反
        cube(f"shoe_{label}", (sx, -0.05, -1.35), (0.30, 0.45, 0.15), shoe_straw)

    export_glb("player_avatar")


if __name__ == "__main__":
    print("=== building v3 player (Z-up correct) ===")
    make_player_avatar_v3()
    print("=== done ===")
