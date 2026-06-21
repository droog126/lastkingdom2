"""build_player_v2.py — 重做 player_avatar.glb 为 v2 中世纪小骑士.

新增 batch 2 资产:
- player_avatar.glb (v2): 头+眼+嘴+鼻+金盔+羽饰+肩甲+披风+分节臂腿+剑
- sword.glb: 独立武器 (柄+刃+格+首)

跑法: F:\\BLENDER\\blender.exe --background --python tools\\build_player_v2.py
输出: assets\\procedural\\pretty\\player_avatar.glb (覆盖)
      assets\\procedural\\pretty\\sword.glb (新)
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from models_lib import (  # noqa: E402
    clear_scene, cube, cone, cylinder, export_glb, ico_sphere,
    mat, merge_into, uv_sphere,
)

# 输出覆盖原 player_avatar.glb
import models_lib
models_lib.OUT_DIR = Path(r"F:\rustProject\lastkingdom2\assets\procedural\pretty")


# ---------- v2 玩家: 平民 (草帽+布衣+麻裤+草鞋) ----------
def make_player_avatar_v2() -> None:
    """v2 玩家 — 平民 (拓荒者/村民). 站高 ≈ 3.0m, 脚底 y=0.

    设计: 暖色大地色, 无金属/无披风/无武器, 朴素感.
    部件清单 (按 z 顺序从前到后):
        头 (肉色 0.7³) @ y=+2.55
        双眼 (2 蓝小点)   @ y=+2.65
        嘴 (小红条)       @ y=+2.40
        鼻 (小三角)       @ y=+2.50
        草帽顶 (棕色圆台) @ y=+2.95
        草帽檐 (宽扁方)   @ y=+2.83
        棕色布衫 (衣)     @ y=+1.20
        麻绳腰带 (浅棕)   @ y=+0.40
        灰布护腰           @ y=+0.10
        双肩布垫 (小方)   @ y=+1.85
        双上臂 (布衫色)   @ y=+1.45
        双肘 (肤色球)     @ y=+1.10
        双前臂 (浅肤)     @ y=+0.65
        双拳 (肤色方)     @ y=+0.20
        双大腿 (麻裤)     @ y=-0.20
        双膝 (灰布球)     @ y=-0.55
        双小腿 (麻裤)     @ y=-1.05
        双草鞋 (深棕方)   @ y=-1.40
    """
    clear_scene()

    # ---- 材质 ----
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

    # ---- 头 ----
    cube("head", (0.0, 2.55, 0.0), (0.70, 0.70, 0.70), skin)
    # 双眼 (蓝点, 平民朴素)
    cube("eye_l", (-0.18, 2.62, 0.34), (0.10, 0.10, 0.04), eye_blue)
    cube("eye_r", ( 0.18, 2.62, 0.34), (0.10, 0.10, 0.04), eye_blue)
    # 嘴 (暖红条, 微笑感)
    cube("mouth", (0.0, 2.42, 0.35), (0.16, 0.04, 0.04), mouth)
    # 鼻 (小三角)
    cone("nose", (0.0, 2.50, 0.36), 0.05, 0.0, 0.08, nose, vertices=4)

    # ---- 草帽 (圆台 + 宽檐) ----
    # 帽顶 (小圆台, 用 cylinder 半径 0.25 高 0.18)
    cylinder("hat_top", (0.0, 2.97, 0.0), 0.25, 0.18, hat_straw, vertices=10)
    # 帽檐 (宽扁方块, 略外伸)
    cylinder("hat_brim", (0.0, 2.84, 0.0), 0.45, 0.05, hat_straw, vertices=12)
    # 帽带 (深棕细环)
    cylinder("hat_band", (0.0, 2.89, 0.0), 0.26, 0.04, hat_band, vertices=10)

    # ---- 躯干 (棕色布衫) ----
    cube("shirt", (0.0, 1.20, 0.0), (0.95, 1.40, 0.55), shirt_brown)
    # 衫领口 (小 v 字 — 用深棕方块在脖子下面)
    cube("collar", (0.0, 1.78, 0.10), (0.40, 0.10, 0.10), shirt_dark)
    # 麻绳腰带
    cube("belt", (0.0, 0.40, 0.0), (0.85, 0.14, 0.55), belt_rope)
    # 麻布护腰
    cube("hip", (0.0, 0.10, 0.0), (0.85, 0.30, 0.55), hip_grey)

    # ---- 肩 (小方块布垫, 平民不穿甲) ----
    for sx, label in ((-0.50, "l"), (0.50, "r")):
        cube(f"shoulder_{label}", (sx, 1.85, 0.0), (0.22, 0.20, 0.22), shirt_dark)

    # ---- 双臂 (5 件套: 上臂 + 肘 + 前臂 + 拳) ----
    for sx, label in ((-0.50, "l"), (0.50, "r")):
        # 上臂 (布衫色)
        cube(f"upper_arm_{label}", (sx, 1.45, 0.0), (0.22, 0.55, 0.22), arm_shirt)
        # 肘 (肤色球, segments 6/4)
        uv_sphere(f"elbow_{label}", (sx, 1.10, 0.0), (0.13, 0.13, 0.13), arm_skin, 6, 4)
        # 前臂 (晒黑肤色, 卷袖感)
        cube(f"forearm_{label}", (sx, 0.65, 0.0), (0.20, 0.75, 0.20), arm_skin)
        # 拳
        cube(f"fist_{label}", (sx, 0.20, 0.0), (0.22, 0.18, 0.22), arm_skin)

    # ---- 双腿 (5 件套: 大腿 + 膝 + 小腿 + 草鞋) ----
    for sx, label in ((-0.20, "l"), (0.20, "r")):
        # 大腿 (麻裤)
        cube(f"thigh_{label}", (sx, -0.20, 0.0), (0.30, 0.65, 0.30), pant_linen)
        # 膝 (灰布球, segments 6/4)
        uv_sphere(f"knee_{label}", (sx, -0.55, 0.05), (0.16, 0.16, 0.16), pant_dark, 6, 4)
        # 小腿
        cube(f"shin_{label}", (sx, -0.95, 0.0), (0.28, 0.55, 0.28), pant_linen)
        # 草鞋 (深棕, 矮平)
        cube(f"shoe_{label}", (sx, -1.35, 0.05), (0.30, 0.15, 0.45), shoe_straw)

    export_glb("player_avatar")


# ---------- 独立 sword.glb (留作替换/调试用) ----------
def make_sword() -> None:
    clear_scene()
    blade = mat("sword_blade", (0.85, 0.88, 0.92), metallic=0.95, roughness=0.15)
    handle = mat("sword_handle", (0.40, 0.20, 0.10), roughness=0.75)
    guard = mat("sword_guard", (0.85, 0.68, 0.20), metallic=0.85, roughness=0.30)
    pommel = mat("sword_pommel", (0.30, 0.30, 0.35), metallic=0.7, roughness=0.40)

    # 剑总长 1.4m, 锚点 y=0 是剑柄顶部
    cube("blade",  (0.0, -0.70, 0.0), (0.10, 0.95, 0.04), blade)
    cube("ridge",  (0.0, -0.70, 0.02), (0.04, 0.95, 0.02), pommel)
    cube("guard",  (0.0, -0.20, 0.0), (0.32, 0.06, 0.10), guard)
    cube("handle", (0.0,  0.00, 0.0), (0.08, 0.32, 0.08), handle)
    uv_sphere("pommel_top", (0.0, 0.20, 0.0), (0.08, 0.08, 0.08), pommel, 8, 6)

    export_glb("sword")


if __name__ == "__main__":
    print("=== building v2 player + sword ===")
    print("[1/2] player_avatar (v2 knight)")
    make_player_avatar_v2()
    print("[2/2] sword (独立武器)")
    make_sword()
    print("=== done ===")
