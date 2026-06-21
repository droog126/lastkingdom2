"""build_all_models.py — 一次性出 pretty/ 下全部 9 类 Blender 资产.

用法:
    F:\BLENDER\blender.exe --background --python tools\build_all_models.py

输出 (相对仓库根):
    assets\procedural\pretty\player_avatar.glb
    assets\procedural\pretty\monster_snake.glb
    assets\procedural\pretty\monster_frost_elf.glb
    assets\procedural\pretty\monster_sand_wurm.glb
    assets\procedural\pretty\monster_treant.glb
    assets\procedural\pretty\monster_aether_wraith.glb
    assets\procedural\pretty\cloud_puff.glb
    assets\procedural\pretty\tree.glb
    assets\procedural\pretty\rock_dark.glb
    assets\procedural\pretty\rock_mid.glb
    assets\procedural\pretty\rock_moss.glb
    assets\procedural\pretty\flower.glb
    assets\procedural\pretty\hill.glb
    assets\procedural\pretty\poi_pillar_red.glb
    assets\procedural\pretty\poi_pillar_cyan.glb
    assets\procedural\pretty\poi_pillar_pink.glb
    assets\procedural\pretty\poi_pillar_gold.glb
    assets\procedural\pretty\ground_disc_outer.glb
    assets\procedural\pretty\ground_disc_inner.glb
    assets\procedural\pretty\MANIFEST.json

设计原则:
- 每个 .glb ≤ 800 面 (低多边形)
- flat shading (除了球/气泡/云)
- 坐标: Blender Z-up, glTF 导出后 Bevy 端视作 Y-up
- 静态模型: 动画在 Rust 侧做 (animate_avatar / animate_monsters / animate_cloud_puffs)
- 关键尺寸 (与原 pretty/mod.rs 1:1 对齐):
    玩家 avatar 身高 ≈ 3.0m (head 顶到腿底)
    怪物 直径 ≈ 1.0m
    树 总高 ≈ 4.0m
    石头 0.4-0.85m
    花 高 0.55m
    山丘 5×3×5
    POI 标柱 高 2.4-3.8m + 球顶
    云 直径 3.2m 主球
    外圈圆盘 直径 12m, 内圈 5m
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

# 让 Blender 找到 models_lib
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from models_lib import (  # noqa: E402
    clear_scene,
    cube,
    cone,
    cylinder,
    export_glb,
    ico_sphere,
    mat,
    merge_into,
    uv_sphere,
    OUT_DIR,
    ROOT,
)


# ---------- 1. 玩家 avatar (10 件套, 矮壮小兵) ----------
def make_player_avatar() -> None:
    """10 件套: head / helmet / torso / shoulderL / shoulderR / armL / armR / belt / legL / legR.

    原 pretty/mod.rs 用的尺寸:
        head       0.7 x 0.7 x 0.7   @ y=+2.55  肤色
        helmet     0.5 x 0.18 x 0.5  @ y=+2.99  深灰
        torso      1.0 x 2.0 x 0.7   @ y=+1.0   红
        shoulderL  0.40 x 0.40 x 0.50 @ x=-0.62, y=+1.85  深灰
        shoulderR  同上              @ x=+0.62
        armL       0.35 x 1.4 x 0.35 @ x=-0.62, y=+1.05  深红
        armR       同上              @ x=+0.62
        belt       0.85 x 0.15 x 0.7 @ y=+0.07  金
        legL       0.35 x 1.2 x 0.35 @ x=-0.20, y=-0.6  深灰裤
        legR       同上              @ x=+0.20
    """
    clear_scene()
    skin = mat("avatar_skin", (0.98, 0.82, 0.68))
    helm = mat("avatar_helmet", (0.25, 0.27, 0.32))
    torso = mat("avatar_torso_red", (0.95, 0.30, 0.30))
    arm = mat("avatar_arm_dark_red", (0.78, 0.22, 0.22))
    belt = mat("avatar_belt_gold", (0.85, 0.65, 0.25), metallic=0.6)
    leg = mat("avatar_leg_dark", (0.30, 0.32, 0.38))
    accent = mat("avatar_buckle", (0.95, 0.80, 0.30), metallic=0.7)

    # 头 (微椭, 比 cube 圆一点)
    uv_sphere("head", (0.0, 2.55, 0.0), (0.40, 0.40, 0.40), skin, 12, 8)
    # 头盔冠 (低模锥台)
    cone("helmet_crown", (0.0, 2.97, 0.0), 0.30, 0.25, 0.16, helm, vertices=8)
    # 头盔侧面护片 (左右两小块)
    cube("helmet_side_l", (-0.27, 2.70, 0.0), (0.10, 0.20, 0.30), helm)
    cube("helmet_side_r", (0.27, 2.70, 0.0), (0.10, 0.20, 0.30), helm)
    # 躯干 (圆角化: 用一截短 cylinder + 顶/底 cap, 这里简化为 cube)
    cube("torso", (0.0, 1.0, 0.0), (1.0, 2.0, 0.7), torso)
    # 胸甲中央十字 (一个小红方块装饰)
    cube("chest_cross_h", (0.0, 1.20, 0.36), (0.30, 0.08, 0.02), accent)
    cube("chest_cross_v", (0.0, 1.20, 0.36), (0.08, 0.30, 0.02), accent)
    # 肩甲
    cube("shoulder_l", (-0.62, 1.85, 0.0), (0.40, 0.40, 0.50), helm)
    cube("shoulder_r", (0.62, 1.85, 0.0), (0.40, 0.40, 0.50), helm)
    # 肩章小球
    uv_sphere("pauldron_l", (-0.72, 1.95, 0.0), (0.10, 0.10, 0.10), accent, 8, 6)
    uv_sphere("pauldron_r", (0.72, 1.95, 0.0), (0.10, 0.10, 0.10), accent, 8, 6)
    # 臂
    cube("arm_l", (-0.62, 1.05, 0.0), (0.35, 1.4, 0.35), arm)
    cube("arm_r", (0.62, 1.05, 0.0), (0.35, 1.4, 0.35), arm)
    # 手 (球)
    uv_sphere("hand_l", (-0.62, 0.30, 0.0), (0.18, 0.16, 0.18), skin, 10, 6)
    uv_sphere("hand_r", (0.62, 0.30, 0.0), (0.18, 0.16, 0.18), skin, 10, 6)
    # 腰甲带
    cube("belt", (0.0, 0.07, 0.0), (0.85, 0.15, 0.7), belt)
    # 腰扣
    cube("belt_buckle", (0.0, 0.07, 0.36), (0.18, 0.18, 0.04), accent)
    # 腿
    cube("leg_l", (-0.20, -0.6, 0.0), (0.35, 1.2, 0.35), leg)
    cube("leg_r", (0.20, -0.6, 0.0), (0.35, 1.2, 0.35), leg)
    # 靴子
    cube("boot_l", (-0.20, -1.30, 0.05), (0.38, 0.20, 0.50), helm)
    cube("boot_r", (0.20, -1.30, 0.05), (0.38, 0.20, 0.50), helm)

    export_glb("player_avatar")


# ---------- 2. 怪物 5 种 ----------
def make_monster(name: str, base_color, glow_color, kind: str) -> None:
    clear_scene()
    body = mat(f"{name}_body", base_color, roughness=0.6,
               emissive=(glow_color[0] * 0.4, glow_color[1] * 0.4, glow_color[2] * 0.4))
    accent = mat(f"{name}_accent", glow_color, roughness=0.5,
                 emissive=(glow_color[0] * 0.6, glow_color[1] * 0.6, glow_color[2] * 0.6))
    eye = mat(f"{name}_eye", (0.04, 0.03, 0.02), roughness=0.4)

    if kind == "snake":
        # 蜿蜒蛇身: 6 段球, 沿 S 形排开 (segments=8/rings=5 = ~64 面/段 × 6 = 384)
        for i in range(6):
            t = i / 5.0
            x = (t - 0.5) * 0.7
            z = math.sin(t * 3.14) * 0.20
            scale = 0.18 if i in (0, 5) else 0.22
            uv_sphere(f"body_{i}", (x, 0.0, z), (scale, scale * 0.9, scale), body, 8, 5)
        # 蛇头 (最大, 稍微抬起)
        uv_sphere("head", (-0.45, 0.05, 0.0), (0.26, 0.22, 0.26), body, 10, 6)
        # 双眼
        uv_sphere("eye_l", (-0.55, 0.18, 0.10), (0.05, 0.05, 0.05), eye, 6, 4)
        uv_sphere("eye_r", (-0.55, 0.18, -0.10), (0.05, 0.05, 0.05), eye, 6, 4)
        # 信子 (小红尖)
        cone("tongue", (-0.65, 0.10, 0.0), 0.012, 0.0, 0.10, accent, vertices=6)

    elif kind == "frost_elf":
        # 蓝精灵: 锥形身体 + 头 + 帽尖
        cone("body", (0.0, 0.05, 0.0), 0.20, 0.42, 0.70, body, vertices=8)
        uv_sphere("head", (0.0, 0.55, 0.0), (0.30, 0.28, 0.28), body, 16, 10)
        # 尖帽
        cone("hat", (0.0, 0.95, 0.0), 0.22, 0.04, 0.55, accent, vertices=8)
        # 双眼
        uv_sphere("eye_l", (-0.10, 0.60, 0.24), (0.06, 0.06, 0.04), eye, 8, 6)
        uv_sphere("eye_r", (0.10, 0.60, 0.24), (0.06, 0.06, 0.04), eye, 8, 6)
        # 双手 (冰晶球)
        uv_sphere("hand_l", (-0.35, 0.10, 0.0), (0.10, 0.10, 0.10), accent, 10, 6)
        uv_sphere("hand_r", (0.35, 0.10, 0.0), (0.10, 0.10, 0.10), accent, 10, 6)

    elif kind == "sand_wurm":
        # 沙虫: 4 段递缩 + 大头 (低面数球)
        for i in range(4):
            t = i / 3.0
            x = (t - 0.5) * 0.5
            scale = 0.18 - t * 0.05
            uv_sphere(f"body_{i}", (x, 0.0, 0.0), (scale, scale * 0.85, scale), body, 8, 5)
        uv_sphere("head", (0.30, 0.05, 0.0), (0.32, 0.26, 0.28), body, 10, 6)
        # 触须 (左右两短锥)
        cone("tentacle_l", (-0.35, 0.05, 0.10), 0.025, 0.0, 0.18, accent, vertices=6)
        cone("tentacle_r", (-0.35, 0.05, -0.10), 0.025, 0.0, 0.18, accent, vertices=6)
        # 眼
        uv_sphere("eye_l", (0.40, 0.18, 0.10), (0.06, 0.06, 0.04), eye, 6, 4)
        uv_sphere("eye_r", (0.40, 0.18, -0.10), (0.06, 0.06, 0.04), eye, 6, 4)

    elif kind == "treant":
        # 树人: 圆木身体 + 头 + 双臂 (粗枝)
        cylinder("trunk", (0.0, 0.20, 0.0), 0.30, 0.60, body, vertices=10)
        uv_sphere("canopy", (0.0, 0.65, 0.0), (0.50, 0.40, 0.50), accent, 12, 8)
        # 粗枝双臂
        cylinder("arm_l", (-0.40, 0.45, 0.0), 0.08, 0.55, body, vertices=8)
        cylinder("arm_r", (0.40, 0.45, 0.0), 0.08, 0.55, body, vertices=8)
        # 手 (球)
        uv_sphere("hand_l", (-0.40, 0.15, 0.0), (0.13, 0.13, 0.13), accent, 10, 8)
        uv_sphere("hand_r", (0.40, 0.15, 0.0), (0.13, 0.13, 0.13), accent, 10, 8)
        # 双眼 (橙色发光)
        eye_glow = mat("treant_eye_glow", (1.0, 0.6, 0.1), roughness=0.3,
                       emissive=(1.0, 0.5, 0.05))
        uv_sphere("eye_l", (-0.10, 0.65, 0.40), (0.06, 0.06, 0.04), eye_glow, 8, 6)
        uv_sphere("eye_r", (0.10, 0.65, 0.40), (0.06, 0.06, 0.04), eye_glow, 8, 6)

    elif kind == "aether_wraith":
        # 幽冥: 倒锥身 + 飘带头 + 飘带 (左右两条细长方块)
        cone("body", (0.0, 0.10, 0.0), 0.42, 0.05, 0.85, body, vertices=8)
        uv_sphere("head", (0.0, 0.65, 0.0), (0.32, 0.34, 0.30), body, 16, 10)
        # 飘带 (两片薄方块, 像斗篷)
        cube("wisp_l", (-0.30, 0.20, 0.0), (0.05, 0.55, 0.10), accent)
        cube("wisp_r", (0.30, 0.20, 0.0), (0.05, 0.55, 0.10), accent)
        # 双眼 (紫发光)
        eye_glow = mat("wraith_eye_glow", (1.0, 0.3, 0.95), roughness=0.3,
                       emissive=(1.0, 0.25, 0.95))
        uv_sphere("eye_l", (-0.10, 0.70, 0.26), (0.06, 0.06, 0.04), eye_glow, 8, 6)
        uv_sphere("eye_r", (0.10, 0.70, 0.26), (0.06, 0.06, 0.04), eye_glow, 8, 6)

    export_glb(name)


# ---------- 3. 云朵 ----------
def make_cloud_puff() -> None:
    clear_scene()
    cloud = mat("cloud_white", (0.92, 0.95, 1.0), roughness=0.95, alpha=0.85)
    rim = mat("cloud_rim", (1.0, 1.0, 1.0), roughness=0.85, alpha=0.70)

    # 中心大球 (直径 3.2m, 原 pretty 用 1.6 半径 = 3.2 直径) — 低面数
    uv_sphere("cloud_main", (0.0, 0.0, 0.0), (1.6, 1.6, 1.6), cloud, 12, 7)
    uv_sphere("cloud_l", (-1.5, 0.2, 0.0), (1.1, 1.1, 1.1), cloud, 10, 6)
    uv_sphere("cloud_r", (1.6, -0.1, 0.5), (1.2, 1.2, 1.2), cloud, 10, 6)
    uv_sphere("cloud_top", (0.3, 1.0, -0.2), (0.9, 0.9, 0.9), rim, 8, 5)
    export_glb("cloud_puff")


# ---------- 4. 树 ----------
def make_tree() -> None:
    clear_scene()
    bark = mat("tree_bark", (0.45, 0.27, 0.10), roughness=0.95)
    leaf = mat("tree_leaf", (0.25, 0.55, 0.20), roughness=0.95)
    leaf_dark = mat("tree_leaf_dark", (0.15, 0.40, 0.15), roughness=0.95)

    # 树干: 2 段 (1.0 高 + 1.0 高, 半径 0.2)
    cylinder("trunk_low", (0.0, 0.5, 0.0), 0.20, 1.0, bark, vertices=8)
    cylinder("trunk_high", (0.0, 1.5, 0.0), 0.16, 1.0, bark, vertices=8)
    # 树冠: 3 团不同深浅的叶球, 半径 0.45-0.55
    uv_sphere("canopy_main", (0.0, 2.4, 0.0), (0.55, 0.50, 0.55), leaf, 12, 8)
    uv_sphere("canopy_l", (-0.40, 2.6, 0.20), (0.45, 0.45, 0.45), leaf_dark, 12, 8)
    uv_sphere("canopy_r", (0.40, 2.5, -0.20), (0.50, 0.45, 0.50), leaf, 12, 8)
    uv_sphere("canopy_top", (0.10, 3.0, 0.10), (0.40, 0.40, 0.40), leaf_dark, 12, 8)
    export_glb("tree")


# ---------- 5. 石头 ×3 色 ----------
def make_rock(name: str, color, scale_hint: float = 0.7) -> None:
    clear_scene()
    body = mat(f"rock_{name}", color, roughness=0.95)
    accent = mat(f"rock_{name}_highlight",
                 (min(1.0, color[0] + 0.1), min(1.0, color[1] + 0.1), min(1.0, color[2] + 0.1)),
                 roughness=0.9)
    # 主体 ico_sphere subdivision 1 ≈ 80 面, 扁平一下
    ico_sphere("rock_main", (0.0, 0.0, 0.0),
               (0.40 * scale_hint, 0.30 * scale_hint, 0.40 * scale_hint),
               body, subdivisions=1)
    # 顶部小尖
    ico_sphere("rock_top", (0.06, 0.20 * scale_hint, -0.04),
               (0.18 * scale_hint, 0.14 * scale_hint, 0.18 * scale_hint),
               accent, subdivisions=0)
    # 底座扁一点
    ico_sphere("rock_base", (-0.10, -0.12 * scale_hint, 0.08),
               (0.22 * scale_hint, 0.10 * scale_hint, 0.22 * scale_hint),
               body, subdivisions=0)
    export_glb(f"rock_{name}")


# ---------- 6. 花朵 (5 色变体, 一次出 1 个, 重复调用) ----------
def make_flower(color, idx: int) -> None:
    clear_scene()
    petal = mat(f"flower_petal_{idx}", color, roughness=0.65)
    stem = mat(f"flower_stem_{idx}", (0.30, 0.55, 0.20), roughness=0.9)
    center = mat(f"flower_center_{idx}", (1.0, 0.92, 0.40), roughness=0.7,
                 emissive=(0.4, 0.35, 0.10))

    # 茎
    cylinder("stem", (0.0, 0.20, 0.0), 0.025, 0.40, stem, vertices=6)
    # 两片小叶
    cube("leaf_l", (-0.10, 0.20, 0.0), (0.16, 0.04, 0.08), stem)
    cube("leaf_r", (0.10, 0.22, 0.0), (0.16, 0.04, 0.08), stem)
    # 花瓣 5 片
    for k in range(5):
        angle = k * 1.2566
        r = 0.18
        x = math.cos(angle) * r
        z = math.sin(angle) * r
        uv_sphere(f"petal_{k}", (x, 0.50, z), (0.10, 0.08, 0.10), petal, 10, 6)
    # 花心
    uv_sphere("flower_center", (0.0, 0.50, 0.0), (0.08, 0.06, 0.08), center, 10, 6)
    export_glb(f"flower_{idx}")


# ---------- 7. 远景山丘 ----------
def make_hill() -> None:
    clear_scene()
    body = mat("hill_green", (0.20, 0.42, 0.18), roughness=0.95)
    accent = mat("hill_green_dark", (0.10, 0.28, 0.10), roughness=0.95)

    # 5x3x5 的矮锥台 (原 pretty 用 Cuboid, 这里用 ico_sphere 拍扁, 更像山)
    ico_sphere("hill_main", (0.0, 0.0, 0.0), (2.5, 1.5, 2.5), body, subdivisions=1)
    # 顶部小尖
    ico_sphere("hill_peak", (0.30, 0.80, -0.20), (1.0, 0.7, 1.0), accent, subdivisions=1)
    export_glb("hill")


# ---------- 8. POI 标柱 ×4 色 ----------
def make_poi_pillar(name: str, base_color, glow_color) -> None:
    clear_scene()
    body = mat(f"poi_{name}_body", base_color, roughness=0.62,
               emissive=(glow_color[0] * 0.3, glow_color[1] * 0.3, glow_color[2] * 0.3))
    glow = mat(f"poi_{name}_glow", glow_color, roughness=0.38,
               emissive=(glow_color[0] * 1.15, glow_color[1] * 1.15, glow_color[2] * 1.15))

    # 标柱主体 (高 2.4-3.8, 用 2.8 平均)
    cube("pillar", (0.0, 0.0, 0.0), (0.45, 2.8, 0.45), body)
    # 底座
    cube("base", (0.0, -1.45, 0.0), (0.70, 0.10, 0.70), glow)
    # 顶端球
    uv_sphere("top", (0.0, 1.55, 0.0), (0.31, 0.31, 0.31), glow, 16, 10)
    # 顶端四角小尖刺
    for sx, sz in ((-0.18, -0.18), (0.18, -0.18), (-0.18, 0.18), (0.18, 0.18)):
        cone("spike", (sx, 1.85, sz), 0.04, 0.0, 0.18, glow, vertices=4)
    export_glb(f"poi_pillar_{name}")


# ---------- 9. 圆盘 (外/内) ----------
def make_ground_disc(name: str, radius: float, depth: float,
                     color, glow: tuple) -> None:
    clear_scene()
    body = mat(f"disc_{name}_body", color, roughness=0.95, alpha=0.85,
               emissive=(glow[0] * 0.5, glow[1] * 0.5, glow[2] * 0.5))
    # 6 边形粗 cylinder, 当低模圆盘
    cylinder("disc", (0.0, 0.0, 0.0), radius, depth, body, vertices=12)
    export_glb(f"ground_disc_{name}")


# ---------- main ----------
def main() -> None:
    print("=== building pretty/ models ===")

    print("[1/9] player_avatar")
    make_player_avatar()

    print("[2/9] monsters (5)")
    make_monster("monster_snake",
                 base_color=(0.50, 0.85, 0.20), glow_color=(0.70, 1.0, 0.30), kind="snake")
    make_monster("monster_frost_elf",
                 base_color=(0.30, 0.70, 0.95), glow_color=(0.50, 0.90, 1.0), kind="frost_elf")
    make_monster("monster_sand_wurm",
                 base_color=(0.95, 0.70, 0.20), glow_color=(1.0, 0.85, 0.30), kind="sand_wurm")
    make_monster("monster_treant",
                 base_color=(0.40, 0.25, 0.10), glow_color=(0.55, 0.35, 0.15), kind="treant")
    make_monster("monster_aether_wraith",
                 base_color=(0.70, 0.30, 0.85), glow_color=(0.95, 0.45, 1.0), kind="aether_wraith")

    print("[3/9] cloud_puff")
    make_cloud_puff()

    print("[4/9] tree")
    make_tree()

    print("[5/9] rocks (3)")
    make_rock("dark", (0.42, 0.42, 0.45), scale_hint=1.0)
    make_rock("mid",  (0.58, 0.55, 0.50), scale_hint=0.9)
    make_rock("moss", (0.50, 0.52, 0.48), scale_hint=0.8)

    print("[6/9] flowers (5)")
    flower_colors = [
        ("pink",   (0.98, 0.30, 0.55)),
        ("yellow", (1.00, 0.85, 0.20)),
        ("purple", (0.55, 0.30, 0.98)),
        ("orange", (1.00, 0.45, 0.20)),
        ("red",    (0.95, 0.30, 0.30)),
    ]
    for idx, (_n, c) in enumerate(flower_colors):
        make_flower(c, idx)

    print("[7/9] hill")
    make_hill()

    print("[8/9] poi_pillars (4)")
    make_poi_pillar("red",  (0.95, 0.20, 0.08), (1.00, 0.55, 0.18))
    make_poi_pillar("cyan", (0.05, 0.48, 0.50), (0.18, 0.90, 0.82))
    make_poi_pillar("pink", (0.72, 0.06, 0.25), (1.00, 0.18, 0.40))
    make_poi_pillar("gold", (0.82, 0.58, 0.14), (1.00, 0.80, 0.22))

    print("[9/9] ground_discs")
    make_ground_disc("outer", 6.0, 0.20,
                     color=(0.32, 0.48, 0.20), glow=(0.20, 0.40, 0.10))
    make_ground_disc("inner", 2.5, 0.20,
                     color=(0.55, 0.75, 0.30), glow=(0.30, 0.50, 0.15))

    # 写 MANIFEST.json
    manifest = build_manifest()
    manifest_path = OUT_DIR / "MANIFEST.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\nMANIFEST -> {manifest_path}")
    print(f"\n=== done. {len(list(OUT_DIR.glob('*.glb')))} .glb files in {OUT_DIR} ===")


def build_manifest() -> dict:
    """为后续 Rust 资源加载生成 manifest."""
    return {
        "version": 1,
        "format": "glb",
        "coordinate_system": {
            "src": "blender_z_up",
            "engine": "bevy_y_up",
            "note": "glTF exporter is Y-up by default; Bevy loads as Y-up. No transform needed.",
        },
        "poly_budget": 800,
        "shading": "flat (mesh non-sphere) / smooth (sphere)",
        "animation": "none (static glb, animation in Rust)",
        "assets": {
            "player_avatar": {
                "path": "procedural/pretty/player_avatar.glb",
                "approx_height_m": 3.0,
                "anchor": "feet (y=0)",
                "spawn_as": "single SceneRoot, then Rust positions child meshes by offset",
            },
            "monster_snake": {
                "path": "procedural/pretty/monster_snake.glb",
                "approx_size_m": 1.0,
                "anchor": "center",
                "replaces": "monsters[0] in pretty/mod.rs (green sphere)",
            },
            "monster_frost_elf": {
                "path": "procedural/pretty/monster_frost_elf.glb",
                "approx_size_m": 1.0,
                "anchor": "center",
                "replaces": "monsters[1] in pretty/mod.rs (blue sphere)",
            },
            "monster_sand_wurm": {
                "path": "procedural/pretty/monster_sand_wurm.glb",
                "approx_size_m": 1.0,
                "anchor": "center",
                "replaces": "monsters[2] in pretty/mod.rs (orange sphere)",
            },
            "monster_treant": {
                "path": "procedural/pretty/monster_treant.glb",
                "approx_size_m": 1.4,
                "anchor": "feet (y=0)",
                "replaces": "monsters[3] in pretty/mod.rs (brown sphere)",
            },
            "monster_aether_wraith": {
                "path": "procedural/pretty/monster_aether_wraith.glb",
                "approx_size_m": 1.4,
                "anchor": "center",
                "replaces": "monsters[4] in pretty/mod.rs (purple sphere)",
            },
            "cloud_puff": {
                "path": "procedural/pretty/cloud_puff.glb",
                "approx_size_m": 3.2,
                "anchor": "center",
                "replaces": "4 cloud_puff spheres in pretty/mod.rs",
            },
            "tree": {
                "path": "procedural/pretty/tree.glb",
                "approx_size_m": "3.2 (h)",
                "anchor": "feet (y=0)",
                "replaces": "8 trees (2x trunk + 2x canopy cubes) in pretty/mod.rs",
            },
            "rock_dark": {
                "path": "procedural/pretty/rock_dark.glb",
                "approx_size_m": 0.85,
                "anchor": "feet (y=0)",
                "replaces": "rocks i%3==0 in pretty/mod.rs",
            },
            "rock_mid": {
                "path": "procedural/pretty/rock_mid.glb",
                "approx_size_m": 0.75,
                "anchor": "feet (y=0)",
                "replaces": "rocks i%3==1 in pretty/mod.rs",
            },
            "rock_moss": {
                "path": "procedural/pretty/rock_moss.glb",
                "approx_size_m": 0.65,
                "anchor": "feet (y=0)",
                "replaces": "rocks i%3==2 in pretty/mod.rs",
            },
            "flower_0_pink":   {"path": "procedural/pretty/flower_0.glb", "approx_size_m": 0.55, "anchor": "feet (y=0)"},
            "flower_1_yellow": {"path": "procedural/pretty/flower_1.glb", "approx_size_m": 0.55, "anchor": "feet (y=0)"},
            "flower_2_purple": {"path": "procedural/pretty/flower_2.glb", "approx_size_m": 0.55, "anchor": "feet (y=0)"},
            "flower_3_orange": {"path": "procedural/pretty/flower_3.glb", "approx_size_m": 0.55, "anchor": "feet (y=0)"},
            "flower_4_red":    {"path": "procedural/pretty/flower_4.glb", "approx_size_m": 0.55, "anchor": "feet (y=0)"},
            "hill": {
                "path": "procedural/pretty/hill.glb",
                "approx_size_m": "5x3x5",
                "anchor": "feet (y=0)",
                "replaces": "4 hill Cuboids in pretty/mod.rs",
            },
            "poi_pillar_red":  {"path": "procedural/pretty/poi_pillar_red.glb",  "approx_size_m": 3.8, "anchor": "feet (y=0)"},
            "poi_pillar_cyan": {"path": "procedural/pretty/poi_pillar_cyan.glb", "approx_size_m": 2.8, "anchor": "feet (y=0)"},
            "poi_pillar_pink": {"path": "procedural/pretty/poi_pillar_pink.glb", "approx_size_m": 2.5, "anchor": "feet (y=0)"},
            "poi_pillar_gold": {"path": "procedural/pretty/poi_pillar_gold.glb", "approx_size_m": 2.4, "anchor": "feet (y=0)"},
            "ground_disc_outer": {
                "path": "procedural/pretty/ground_disc_outer.glb",
                "approx_size_m": 12.0,
                "anchor": "feet (y=0)",
                "replaces": "OuterDisc cylinder (r=6, h=0.2) in pretty/mod.rs",
            },
            "ground_disc_inner": {
                "path": "procedural/pretty/ground_disc_inner.glb",
                "approx_size_m": 5.0,
                "anchor": "feet (y=0)",
                "replaces": "InnerDisc cylinder (r=2.5, h=0.2) in pretty/mod.rs",
            },
        },
    }


if __name__ == "__main__":
    main()
