"""build_v5_cute.py �?全部重做 27 �?.glb �?v5-cute 梦幻可爱 Q 版风�?

风格:
- Q 版比�?(头占�?1/3)
- 糖果�?(饱和�?0.85+)
- 强自发光 emissive 0.3-0.5
- smooth shading icosphere subdivision 2-3
- 球形/胶囊为主
- 大眼�?
- 超贴�?(y �?0.8m, 锚点 y=0)

替换: assets/procedural/pretty/*.glb + assets/procedural/eco/*.glb
"""
from __future__ import annotations
import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import models_lib
models_lib.OUT_DIR = Path(r"F:\rustProject\lastkingdom2\assets\procedural\pretty")

from models_lib import (  # noqa: E402
    clear_scene, cube, cone, cylinder, export_glb, ico_sphere, mat, uv_sphere,
)


# ===== 共用材质 (糖果�?+ �?emissive) =====
def glow_mat(name, color, glow_strength=0.4, roughness=0.5):
    """糖果�?+ 强自发光."""
    return mat(name, color, roughness=roughness, emissive=color)


# =================================================================
# eco/ 4 �?v5-cute 资产
# =================================================================
def make_rabbit_v5() -> None:
    """v5-cute 兔子: 圆球�?+ 长耳朵 + 球身 + 短腿 + 大眼�?+ 红鼻�?+ �?
    �?0.7m, 贴地, 0.3m �?"""
    clear_scene()
    fur = glow_mat("rabbit_fur_warm", (0.98, 0.85, 0.80), glow_strength=0.3)
    belly = glow_mat("rabbit_belly", (1.0, 0.95, 0.90), glow_strength=0.4)
    inner_ear = glow_mat("rabbit_inner_ear", (1.0, 0.70, 0.75), glow_strength=0.5)
    eye = glow_mat("rabbit_eye_dark", (0.10, 0.05, 0.08), glow_strength=0.2)
    eye_shine = glow_mat("rabbit_eye_shine", (1.0, 1.0, 1.0), glow_strength=0.8)
    nose = glow_mat("rabbit_nose_pink", (1.0, 0.55, 0.65), glow_strength=0.6)
    cheek = glow_mat("rabbit_cheek_pink", (1.0, 0.70, 0.75), glow_strength=0.6)

    # 球身 (0.3 radius, y 中心 0.25, 贴地)
    uv_sphere("body", (0.0, 0.25, 0.0), (0.30, 0.25, 0.28), 
    # 圆头 (0.28 radius, y 中心 0.55, 头上�?
    uv_sphere("head", (0.0, 0.55, 0.05), (0.28, 0.26, 0.26), 
    # 长耳朵 (椭球, �?z 方向拉长, y 中心 0.85)
    uv_sphere("ear_l", (-0.10, 0.85, 0.05), (0.06, 0.06, 0.22), fur, 12, 8)
    uv_sphere("ear_r", ( 0.10, 0.85, 0.05), (0.06, 0.06, 0.22), fur, 12, 8)
    # 内�?(粉红, 在耳朵�?
    uv_sphere("inner_ear_l", (-0.10, 0.85, 0.10), (0.035, 0.035, 0.18), inner_ear, 10, 6)
    uv_sphere("inner_ear_r", ( 0.10, 0.85, 0.10), (0.035, 0.035, 0.18), inner_ear, 10, 6)
    # 大眼�?(�?+ 白高�? 卡通大�?
    uv_sphere("eye_l", (-0.10, 0.58, 0.25), (0.05, 0.06, 0.04), eye, 12, 10)
    uv_sphere("eye_r", ( 0.10, 0.58, 0.25), (0.05, 0.06, 0.04), eye, 12, 10)
    # 眼睛高光 (小白�?
    uv_sphere("shine_l", (-0.085, 0.61, 0.28), (0.018, 0.022, 0.012), eye_shine, 8, 6)
    uv_sphere("shine_r", ( 0.115, 0.61, 0.28), (0.018, 0.022, 0.012), eye_shine, 8, 6)
    # 鼻子 (粉红三角)
    cone("nose", (0.0, 0.48, 0.27), 0.02, 0.0, 0.025, nose, vertices=8)
    # �?(小粉红弧, 2 个小�?
    uv_sphere("mouth_l", (-0.018, 0.45, 0.26), (0.012, 0.012, 0.012), nose, 8, 6)
    uv_sphere("mouth_r", ( 0.018, 0.45, 0.26), (0.012, 0.012, 0.012), nose, 8, 6)
    # 红脸�?(腮红)
    uv_sphere("cheek_l", (-0.18, 0.50, 0.18), (0.05, 0.04, 0.03), cheek, 10, 6)
    uv_sphere("cheek_r", ( 0.18, 0.50, 0.18), (0.05, 0.04, 0.03), cheek, 10, 6)
    # 短腿 (�?
    uv_sphere("leg_l", (-0.10, 0.05, 0.05), (0.06, 0.06, 0.08), fur, 10, 8)
    uv_sphere("leg_r", ( 0.10, 0.05, 0.05), (0.06, 0.06, 0.08), fur, 10, 8)
    # 小尾�?
    uv_sphere("tail", (0.0, 0.25, -0.25), (0.10, 0.10, 0.08), belly, 12, 8)
    export_glb_v5_eco("rabbit")


def make_berry_bush_v5() -> None:
    """v5-cute 浆果�? 1 棵圆球叶�?(中心, 0.5m 半径, �?
    + 周围散开 7 颗浆�?(�?�?�?�?�? 散在 0.5-0.8m 范围, 0.05m 半径)
    + 1 根矮�?(圆柱, 0.1m 半径, 0.3m �?
    高度 �?0.6m"""
    clear_scene()
    leaf = glow_mat("berry_leaf", (0.20, 0.85, 0.30), glow_strength=0.3)
    leaf_dark = glow_mat("berry_leaf_dark", (0.10, 0.60, 0.20), glow_strength=0.25)
    stem_brown = glow_mat("berry_stem", (0.50, 0.32, 0.20), glow_strength=0.15)
    # 5 种浆�?(强发�? 梦幻)
    fruit_red    = glow_mat("fruit_red",    (1.0, 0.20, 0.30), glow_strength=0.6)
    fruit_purple = glow_mat("fruit_purple", (0.70, 0.30, 0.95), glow_strength=0.6)
    fruit_blue   = glow_mat("fruit_blue",   (0.40, 0.65, 1.0), glow_strength=0.6)
    fruit_pink   = glow_mat("fruit_pink",   (1.0, 0.55, 0.80), glow_strength=0.6)
    fruit_gold   = glow_mat("fruit_gold",   (1.0, 0.85, 0.30), glow_strength=0.7)
    fruit_shine  = glow_mat("fruit_shine",  (1.0, 1.0, 1.0), glow_strength=0.9)
    fruit_colors = [fruit_red, fruit_purple, fruit_blue, fruit_pink, fruit_gold]

    # �?(0.05 半径, 0.30 �? 贴地 y=0~0.30)
    cylinder("stem", (0.0, 0.15, 0.0), 0.05, 0.30, stem_brown, vertices=8)
    # 叶丛 (中心 0.30 半径, y 0.40~0.60)
    uv_sphere("leaf_main", (0.0, 0.50, 0.0), (0.30, 0.20, 0.30), leaf, 14, 10)
    uv_sphere("leaf_dark_1", (-0.15, 0.45, 0.10), (0.18, 0.15, 0.18), leaf_dark, 12, 8)
    uv_sphere("leaf_dark_2", ( 0.18, 0.55, -0.10), (0.16, 0.13, 0.16), leaf_dark, 12, 8)

    # 散开 7 颗浆�?(围绕叶丛, 距离 0.4-0.7m, y 0.30~0.50 高度)
    fruit_positions = [
        (-0.45, 0.35,  0.10, 0),  # �?
        ( 0.50, 0.40, -0.15, 1),  # �?
        ( 0.05, 0.50,  0.50, 2),  # �?
        ( 0.10, 0.45, -0.55, 3),  # �?
        (-0.30, 0.30, -0.40, 4),  # 左后
        ( 0.30, 0.30,  0.40, 4),  # 右前
        (-0.20, 0.50, -0.30, 0),  # 顶左
    ]
    for i, (x, y, z, color_idx) in enumerate(fruit_positions):
        # 浆果主身
        uv_sphere(
            f"fruit_{i}",
            (x, y, z),
            (0.07, 0.07, 0.07),
            fruit_colors[color_idx],
            12, 10,
        )
        # 浆果高光
        uv_sphere(
            f"fruit_shine_{i}",
            (x - 0.020, y + 0.025, z + 0.030),
            (0.022, 0.022, 0.015),
            fruit_shine,
            8, 6,
        )
    export_glb_v5_eco("berry_bush")


def make_berry_fruit_v5() -> None:
    """v5-cute 单颗浆果: 强发光圆�?+ 高光, 0.08m 半径."""
    clear_scene()
    fruit_red    = glow_mat("fruit_red",    (1.0, 0.20, 0.30), glow_strength=0.7)
    fruit_purple = glow_mat("fruit_purple", (0.70, 0.30, 0.95), glow_strength=0.7)
    fruit_blue   = glow_mat("fruit_blue",   (0.40, 0.65, 1.0), glow_strength=0.7)
    fruit_pink   = glow_mat("fruit_pink",   (1.0, 0.55, 0.80), glow_strength=0.7)
    fruit_gold   = glow_mat("fruit_gold",   (1.0, 0.85, 0.30), glow_strength=0.8)
    shine = glow_mat("fruit_shine", (1.0, 1.0, 1.0), glow_strength=0.95)
    stem = glow_mat("fruit_stem", (0.40, 0.25, 0.15), glow_strength=0.2)

    # 5 颗浆果的 master .glb (5 个独�?mesh group, 跑起来按需�?
    # �?v5-cute 实际只放 1 �?+ �?+ 叶子, �?main.rs spawn 多次
    # 这里�?1 颗红浆果作为 base
    uv_sphere("berry", (0.0, 0.08, 0.0), (0.08, 0.08, 0.08), fruit_red, 14, 10)
    # 高光
    uv_sphere("shine", (-0.020, 0.10, 0.025), (0.025, 0.025, 0.015), shine, 8, 6)
    # �?
    cylinder("stem", (0.0, 0.18, 0.0), 0.008, 0.04, stem, vertices=6)
    # 叶子
    cone("leaf", (0.020, 0.18, 0.0), 0.012, 0.0, 0.020, stem, vertices=4)
    export_glb_v5_eco("berry_fruit")


def make_co2_bubble_v5() -> None:
    """v5-cute CO2 气泡: 半透明发光�? �?emissive."""
    clear_scene()
    bubble = mat("co2_bubble", (0.65, 0.95, 1.0), roughness=0.05, alpha=0.45)
    # �?emissive 通过 add 一�?inner sphere
    inner = mat("co2_inner", (0.85, 1.0, 1.0), roughness=0.1, emissive=(0.50, 0.85, 1.0))
    rim = glow_mat("co2_rim", (0.80, 0.95, 1.0), glow_strength=0.9)

    # 4 球拼 (大中�?
    uv_sphere("bubble_big", (0.0, 0.15, 0.0), (0.20, 0.20, 0.20), bubble, 14, 10)
    uv_sphere("bubble_mid", (0.18, 0.08, 0.15), (0.10, 0.10, 0.10), bubble, 12, 8)
    uv_sphere("bubble_small", (-0.15, 0.20, 0.20), (0.07, 0.07, 0.07), bubble, 10, 6)
    # 内核 (不透明强发�? 在大球中�?
    uv_sphere("inner", (0.0, 0.15, 0.0), (0.10, 0.10, 0.10), inner, 10, 8)
    # 高光�?
    uv_sphere("rim", (-0.08, 0.22, 0.10), (0.04, 0.04, 0.04), rim, 10, 8)
    export_glb_v5_eco("co2_bubble")


# =================================================================
# pretty/ 23 �?v5-cute 资产 (替换 v4-flat)
# =================================================================
def make_player_avatar_cute() -> None:
    """v5-cute 玩家: Q 版大圆头 + 大眼�?+ 球身 + 4 球手�?+ 红脸�?+ 微笑�?
    �?0.8m, 贴地, 0.5m �?"""
    clear_scene()
    skin = glow_mat("cute_skin_warm", (1.0, 0.85, 0.75), glow_strength=0.4)
    shirt_red = glow_mat("cute_shirt_red", (1.0, 0.40, 0.40), glow_strength=0.5)
    pants_blue = glow_mat("cute_pants_blue", (0.40, 0.55, 0.95), glow_strength=0.4)
    hair = glow_mat("cute_hair_brown", (0.45, 0.30, 0.20), glow_strength=0.3)
    eye = glow_mat("cute_eye_dark", (0.10, 0.05, 0.08), glow_strength=0.2)
    eye_shine = glow_mat("cute_eye_shine", (1.0, 1.0, 1.0), glow_strength=0.95)
    cheek = glow_mat("cute_cheek_pink", (1.0, 0.65, 0.70), glow_strength=0.6)
    mouth_dark = glow_mat("cute_mouth", (0.85, 0.30, 0.40), glow_strength=0.5)

    # 球腿 (Q 版短�?
    uv_sphere("leg_l", (-0.10, 0.10, 0.0), (0.10, 0.10, 0.10), pants_blue, 14, 10)
    uv_sphere("leg_r", ( 0.10, 0.10, 0.0), (0.10, 0.10, 0.10), pants_blue, 14, 10)
    # 球脚 (圆滚�? 比腿大一�? 像大脚丫)
    uv_sphere("foot_l", (-0.10, 0.05, 0.05), (0.13, 0.08, 0.16), pants_blue, 12, 8)
    uv_sphere("foot_r", ( 0.10, 0.05, 0.05), (0.13, 0.08, 0.16), pants_blue, 12, 8)
    # 球身 (Q 版圆�?
    uv_sphere("body", (0.0, 0.40, 0.0), (0.30, 0.28, 0.25), 
    # �?(大圆�? 占身 1/2)
    uv_sphere("head", (0.0, 0.70, 0.0), (0.30, 0.28, 0.28), 
    # 头发 (头顶一�?
    uv_sphere("hair", (0.0, 0.80, 0.0), (0.30, 0.18, 0.30), hair, 14, 10)
    # 头发刘海 (前面一�?
    uv_sphere("bangs", (0.0, 0.78, 0.18), (0.20, 0.10, 0.10), hair, 12, 8)
    # 大眼�?(Q �? 黑色大圆 + 白高�?
    uv_sphere("eye_l", (-0.10, 0.72, 0.22), (0.07, 0.09, 0.05), eye, 14, 12)
    uv_sphere("eye_r", ( 0.10, 0.72, 0.22), (0.07, 0.09, 0.05), eye, 14, 12)
    # 眼睛高光 (双高光更 Q)
    uv_sphere("shine_l_big", (-0.080, 0.76, 0.26), (0.030, 0.035, 0.018), eye_shine, 10, 8)
    uv_sphere("shine_l_small", (-0.115, 0.69, 0.255), (0.015, 0.018, 0.010), eye_shine, 8, 6)
    uv_sphere("shine_r_big", ( 0.120, 0.76, 0.26), (0.030, 0.035, 0.018), eye_shine, 10, 8)
    uv_sphere("shine_r_small", ( 0.085, 0.69, 0.255), (0.015, 0.018, 0.010), eye_shine, 8, 6)
    # 微笑�?(上挑月牙, 2 个小�?
    uv_sphere("mouth_l", (-0.025, 0.62, 0.27), (0.015, 0.012, 0.010), mouth_dark, 8, 6)
    uv_sphere("mouth_r", ( 0.025, 0.62, 0.27), (0.015, 0.012, 0.010), mouth_dark, 8, 6)
    # 红脸�?(腮红)
    uv_sphere("cheek_l", (-0.22, 0.66, 0.18), (0.07, 0.05, 0.03), cheek, 10, 6)
    uv_sphere("cheek_r", ( 0.22, 0.66, 0.18), (0.07, 0.05, 0.03), cheek, 10, 6)
    # 球手 (短圆�?
    uv_sphere("hand_l", (-0.30, 0.42, 0.0), (0.10, 0.10, 0.10), skin, 12, 10)
    uv_sphere("hand_r", ( 0.30, 0.42, 0.0), (0.10, 0.10, 0.10), skin, 12, 10)
    # 衣领 (胸前小三�?
    cone("collar", (0.0, 0.58, 0.20), 0.06, 0.0, 0.04, shirt_red, vertices=4)
    export_glb("player_avatar")


def make_monster_cute(name, body_color, glow_color, eye_color=(0.10, 0.05, 0.08), cheeks=True) -> None:
    """v5-cute 怪物: 圆胖球身 + 2 大眼�?+ �?+ �?�?(可�?."""
    clear_scene()
    body = glow_mat(f"{name}_body", body_color, glow_strength=0.5)
    glow = glow_mat(f"{name}_glow", glow_color, glow_strength=0.7)
    eye = glow_mat(f"{name}_eye", eye_color, glow_strength=0.2)
    eye_shine = glow_mat(f"{name}_shine", (1.0, 1.0, 1.0), glow_strength=0.9)
    cheek = glow_mat(f"{name}_cheek", (1.0, 0.60, 0.65), glow_strength=0.6)
    mouth = glow_mat(f"{name}_mouth", (0.70, 0.30, 0.40), glow_strength=0.4)

    # 球身 (�? 圆胖)
    uv_sphere("body", (0.0, 0.30, 0.0), (0.40, 0.32, 0.38), 
    # �?(�? 凸出上面)
    uv_sphere("head", (0.0, 0.45, 0.0), (0.30, 0.26, 0.30), 
    # 大眼�?
    uv_sphere("eye_l", (-0.13, 0.50, 0.22), (0.07, 0.09, 0.05), eye, 14, 12)
    uv_sphere("eye_r", ( 0.13, 0.50, 0.22), (0.07, 0.09, 0.05), eye, 14, 12)
    # 眼睛高光
    uv_sphere("shine_l", (-0.10, 0.54, 0.26), (0.025, 0.030, 0.015), eye_shine, 10, 8)
    uv_sphere("shine_r", ( 0.16, 0.54, 0.26), (0.025, 0.030, 0.015), eye_shine, 10, 8)
    # �?(微笑)
    uv_sphere("mouth", (0.0, 0.38, 0.30), (0.04, 0.025, 0.02), mouth, 10, 6)
    # 红脸�?
    if cheeks:
        uv_sphere("cheek_l", (-0.25, 0.42, 0.20), (0.07, 0.05, 0.04), cheek, 10, 6)
        uv_sphere("cheek_r", ( 0.25, 0.42, 0.20), (0.07, 0.05, 0.04), cheek, 10, 6)
    # �?�?(发光)
    cone("horn_l", (-0.10, 0.70, 0.0), 0.06, 0.0, 0.15, glow, vertices=6)
    cone("horn_r", ( 0.10, 0.70, 0.0), 0.06, 0.0, 0.15, glow, vertices=6)
    # 短腿
    uv_sphere("leg_l", (-0.15, 0.05, 0.0), (0.10, 0.08, 0.12), body, 12, 8)
    uv_sphere("leg_r", ( 0.15, 0.05, 0.0), (0.10, 0.08, 0.12), body, 12, 8)
    export_glb(name)


def make_cloud_puff_cute() -> None:
    clear_scene()
    cloud = glow_mat("cloud_puff_white", (1.0, 1.0, 1.0), glow_strength=0.5)
    cloud_pink = glow_mat("cloud_puff_pink", (1.0, 0.85, 0.95), glow_strength=0.55)
    cloud_blue = glow_mat("cloud_puff_blue", (0.85, 0.95, 1.0), glow_strength=0.55)
    # 4 球拼 (3 �?+ 1 �?+ 1 �?, 贴地 y=0.15~0.35
    uv_sphere("c1", (0.0, 0.25, 0.0), (0.30, 0.22, 0.30), 
    uv_sphere("c2", (-0.25, 0.20, 0.05), (0.22, 0.20, 0.22), cloud, 14, 10)
    uv_sphere("c3", ( 0.25, 0.18, 0.05), (0.24, 0.20, 0.24), cloud, 14, 10)
    uv_sphere("c4_pink", (0.0, 0.35, -0.10), (0.18, 0.15, 0.18), cloud_pink, 12, 8)
    uv_sphere("c5_blue", (0.15, 0.22, 0.20), (0.16, 0.14, 0.16), cloud_blue, 12, 8)
    export_glb("cloud_puff")


def make_tree_cute() -> None:
    clear_scene()
    bark = glow_mat("tree_bark", (0.70, 0.45, 0.30), glow_strength=0.2)
    leaf = glow_mat("tree_leaf", (0.35, 0.95, 0.40), glow_strength=0.4)
    leaf_pink = glow_mat("tree_leaf_pink", (1.0, 0.75, 0.85), glow_strength=0.5)
    # 圆叶 (Q �?
    uv_sphere("canopy_1", (0.0, 0.55, 0.0), (0.40, 0.32, 0.40), 
    uv_sphere("canopy_2", (-0.20, 0.60, 0.10), (0.25, 0.25, 0.25), leaf_pink, 14, 10)
    uv_sphere("canopy_3", ( 0.20, 0.60, -0.10), (0.25, 0.25, 0.25), leaf, 14, 10)
    # 圆干 (矮粗, 0.10 半径, 0.30 �?
    cylinder("trunk", (0.0, 0.15, 0.0), 0.10, 0.30, bark, vertices=10)
    # 圆根 (底座)
    uv_sphere("root", (0.0, 0.05, 0.0), (0.15, 0.08, 0.15), bark, 12, 8)
    export_glb("tree")


def make_rock_cute(name, color) -> None:
    clear_scene()
    body = glow_mat(f"rock_{name}", color, glow_strength=0.2)
    # 圆滚滚石 (3 球拼, 像糖�?
    uv_sphere("rock_1", (0.0, 0.10, 0.0), (0.18, 0.14, 0.18), body, 14, 10)
    uv_sphere("rock_2", (0.10, 0.15, 0.05), (0.10, 0.10, 0.10), body, 12, 8)
    uv_sphere("rock_3", (-0.08, 0.13, -0.05), (0.08, 0.08, 0.08), body, 10, 8)
    export_glb(f"rock_{name}")


def make_flower_cute(idx, color) -> None:
    clear_scene()
    petal = glow_mat(f"flower_petal_{idx}", color, glow_strength=0.55)
    center = glow_mat(f"flower_center_{idx}", (1.0, 0.95, 0.40), glow_strength=0.85)
    stem = glow_mat(f"flower_stem_{idx}", (0.35, 0.85, 0.40), glow_strength=0.4)
    leaf = glow_mat(f"flower_leaf_{idx}", (0.30, 0.75, 0.30), glow_strength=0.4)
    # �?
    cylinder("stem", (0.0, 0.20, 0.0), 0.012, 0.40, stem, vertices=6)
    # 1 片小�?
    cone("leaf", (0.04, 0.20, 0.0), 0.025, 0.0, 0.04, leaf, vertices=4)
    # 5 片圆花瓣 (围绕中心)
    for k in range(5):
        angle = k * 1.2566
        r = 0.10
        x = math.cos(angle) * r
        z = math.sin(angle) * r
        uv_sphere(f"petal_{k}", (x, 0.45, z), (0.05, 0.04, 0.05), petal, 12, 8)
    # 中心 (亮黄, 强发�?
    uv_sphere("center", (0.0, 0.45, 0.0), (0.04, 0.04, 0.04), center, 12, 10)
    export_glb(f"flower_{idx}")


def make_hill_cute() -> None:
    clear_scene()
    body = glow_mat("hill_green", (0.40, 0.85, 0.45), glow_strength=0.35)
    body_top = glow_mat("hill_green_dark", (0.30, 0.70, 0.40), glow_strength=0.3)
    # 圆山 (Q �? 像绿色棉花糖)
    uv_sphere("hill_main", (0.0, 0.10, 0.0), (0.60, 0.30, 0.60), 
    uv_sphere("hill_top", (0.10, 0.30, -0.10), (0.30, 0.18, 0.30), body_top, 14, 10)
    # 1 朵小花点缀
    uv_sphere("flower_dot", (0.30, 0.40, 0.20), (0.05, 0.04, 0.05),
              glow_mat("flower_dot_pink", (1.0, 0.65, 0.85), glow_strength=0.6), 10, 8)
    export_glb("hill")


def make_poi_pillar_cute(name, body_color, glow_color) -> None:
    clear_scene()
    body = glow_mat(f"poi_{name}_body", body_color, glow_strength=0.4)
    glow = glow_mat(f"poi_{name}_glow", glow_color, glow_strength=0.85)
    glow_inner = glow_mat(f"poi_{name}_inner", glow_color, glow_strength=1.0)
    # 标柱 (矮粗, 0.16 半径, 0.30 �? 强发�?
    cylinder("pillar", (0.0, 0.20, 0.0), 0.16, 0.40, body, vertices=10)
    # 底座 (圆球, 像宝石底)
    uv_sphere("base", (0.0, 0.04, 0.0), (0.22, 0.05, 0.22), body, 12, 8)
    # 顶端大光�?(梦幻光晕)
    uv_sphere("top_glow", (0.0, 0.55, 0.0), (0.18, 0.18, 0.18), 
    uv_sphere("top_inner", (0.0, 0.55, 0.0), (0.10, 0.10, 0.10), glow_inner, 12, 8)
    # 顶端小尖�?(装饰)
    for sx, sz in ((-0.10, -0.10), (0.10, -0.10), (-0.10, 0.10), (0.10, 0.10)):
        cone(f"spike_{sx}_{sz}", (sx, 0.65, sz), 0.04, 0.0, 0.08, glow, vertices=4)
    export_glb(f"poi_pillar_{name}")


def make_ground_disc_cute(name, color, glow) -> None:
    clear_scene()
    body = glow_mat(f"disc_{name}_body", color, glow_strength=0.45, roughness=0.6)
    body_emissive = glow_mat(f"disc_{name}_glow", glow, glow_strength=0.5)
    # 12 边形圆盘, 0.30 半径, 0.02 �?
    cylinder("disc", (0.0, 0.01, 0.0), 0.30, 0.02, body, vertices=12)
    # 内圈发光小环
    cylinder("disc_glow", (0.0, 0.025, 0.0), 0.15, 0.005, body_emissive, vertices=10)
    export_glb(f"ground_disc_{name}")


# ===== helper for eco 4 �?(�?eco/ 目录) =====
def export_glb_v5_eco(name):
    """导出�?eco/ 目录."""
    import models_lib
    eco_dir = Path(r"F:\rustProject\lastkingdom2\assets\procedural\eco")
    eco_dir.mkdir(parents=True, exist_ok=True)
    path = eco_dir / f"{name}.glb"
    bpy_path_fix = models_lib.OUT_DIR
    models_lib.OUT_DIR = eco_dir
    try:
        models_lib.export_glb(name)
    finally:
        models_lib.OUT_DIR = bpy_path_fix


# =================================================================
# main
# =================================================================
def main() -> None:
    print("=== building v5-cute (梦幻可爱 Q �? ===")

    # eco/ 4 �?
    print("\n--- eco/ ---")
    print("[1/4] rabbit (Q 版胖兔子, 0.7m)")
    make_rabbit_v5()
    print("[2/4] berry_bush (圆叶�?+ 7 颗散开浆果, 0.6m)")
    make_berry_bush_v5()
    print("[3/4] berry_fruit (单颗发光浆果, 0.2m)")
    make_berry_fruit_v5()
    print("[4/4] co2_bubble (发光气泡, 0.4m)")
    make_co2_bubble_v5()

    # pretty/ 23 �?
    print("\n--- pretty/ ---")
    print("[1/23] player_avatar (Q 版大圆头 + 大眼�? 0.8m)")
    make_player_avatar_cute()

    print("[2-6/23] monsters x5 (圆胖球身 + 大眼 + �?")
    make_monster_cute("monster_snake",     (0.55, 0.95, 0.30), (0.85, 1.0, 0.50))
    make_monster_cute("monster_frost_elf", (0.45, 0.80, 1.0),  (0.65, 0.95, 1.0))
    make_monster_cute("monster_sand_wurm", (1.0, 0.80, 0.30),  (1.0, 0.95, 0.50))
    make_monster_cute("monster_treant",    (0.50, 0.35, 0.20),  (0.65, 0.50, 0.30))
    make_monster_cute("monster_aether_wraith", (0.85, 0.50, 1.0),  (1.0, 0.70, 1.0))

    print("[7/23] cloud_puff (5 球拼, 3 �?1 �?1 �? 0.35m)")
    make_cloud_puff_cute()

    print("[8/23] tree (圆叶 + 圆干 + 圆根, 0.7m)")
    make_tree_cute()

    print("[9-11/23] rocks x3 (3 球糖�?")
    make_rock_cute("dark", (0.55, 0.55, 0.60))
    make_rock_cute("mid",  (0.70, 0.65, 0.55))
    make_rock_cute("moss", (0.55, 0.75, 0.55))

    print("[12-16/23] flowers x5 (5 圆瓣 + 中心 + �?+ �?")
    flower_colors = [
        (1.0, 0.50, 0.70),  # �?
        (1.0, 0.95, 0.40),  # �?
        (0.80, 0.55, 1.0),  # �?
        (1.0, 0.65, 0.30),  # �?
        (1.0, 0.40, 0.50),  # �?
    ]
    for i, c in enumerate(flower_colors):
        make_flower_cute(i, c)

    print("[17/23] hill (Q 版圆�?+ 小花点缀, 0.6m)")
    make_hill_cute()

    print("[18-21/23] poi_pillars x4 (发光�?+ 顶端光球 + 4 角尖�?")
    make_poi_pillar_cute("red",  (1.0, 0.30, 0.40), (1.0, 0.55, 0.20))
    make_poi_pillar_cute("cyan", (0.30, 0.65, 1.0),  (0.40, 0.95, 1.0))
    make_poi_pillar_cute("pink", (1.0, 0.50, 0.80), (1.0, 0.30, 0.65))
    make_poi_pillar_cute("gold", (1.0, 0.85, 0.30), (1.0, 0.95, 0.50))

    print("[22-23/23] ground_discs")
    make_ground_disc_cute("outer", (0.45, 0.85, 0.50), (0.65, 1.0, 0.40))
    make_ground_disc_cute("inner", (0.75, 0.95, 0.45), (0.95, 1.0, 0.30))

    # 更新 MANIFEST
    manifest = {
        "version": 5,
        "spec": "v5-cute 梦幻可爱 Q �?(糖果�?+ �?emissive + 大眼�?+ smooth 球形 + y �?0.8m 贴地)",
        "format": "glb",
        "assets": {
            "eco": ["rabbit", "berry_bush", "berry_fruit", "co2_bubble"],
            "pretty": [
                "player_avatar", "monster_snake", "monster_frost_elf", "monster_sand_wurm",
                "monster_treant", "monster_aether_wraith", "cloud_puff", "tree",
                "rock_dark", "rock_mid", "rock_moss", "flower_0", "flower_1", "flower_2",
                "flower_3", "flower_4", "hill", "poi_pillar_red", "poi_pillar_cyan",
                "poi_pillar_pink", "poi_pillar_gold", "ground_disc_outer", "ground_disc_inner",
            ],
        },
    }
    mp = Path(r"F:\rustProject\lastkingdom2\assets\procedural\pretty\MANIFEST.json")
    mp.write_text(json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8")
    eco_manifest = Path(r"F:\rustProject\lastkingdom2\assets\procedural\eco\MANIFEST.json")
    eco_manifest.write_text(json.dumps({"version": 5, "spec": "v5-cute 梦幻可爱"}, indent=2), encoding="utf-8")
    print(f"\nMANIFEST -> {mp}")
    print(f"ECO MANIFEST -> {eco_manifest}")
    print("=== done ===")


if __name__ == "__main__":
    main()
