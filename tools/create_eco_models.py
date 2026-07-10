from __future__ import annotations

import math
import sys
from pathlib import Path

import bpy

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import models_lib
from models_lib import clear_scene, cone, cube, mat, uv_sphere


def export_glb(name: str):
    return models_lib.export_glb(name, collection="eco")

def make_rabbit():
    clear_scene()
    fur = mat("warm_tan_fur", (0.72, 0.58, 0.42), 0.9)
    belly = mat("soft_belly_white", (1.0, 0.98, 0.92), 0.9)
    inner = mat("inner_ear_pink", (1.0, 0.46, 0.58), 0.85)
    ear_tip = mat("pink_ear_tip", (1.0, 0.30, 0.46), 0.8)
    eye = mat("black_eye", (0.03, 0.025, 0.02), 0.65)
    eye_patch = mat("soft_gray_eye_patch", (0.30, 0.32, 0.34), 0.85)
    nose = mat("rose_nose", (0.8, 0.25, 0.32), 0.7)
    whisker = mat("dark_whisker", (0.10, 0.08, 0.06), 0.75)

    uv_sphere("body", (0.0, 0.02, 0.42), (0.50, 0.70, 0.36), fur, 16, 8)
    uv_sphere("belly_patch", (0.0, -0.48, 0.43), (0.34, 0.12, 0.26), belly, 12, 6)
    uv_sphere("head", (0.0, -0.66, 0.82), (0.38, 0.34, 0.32), fur, 16, 8)
    uv_sphere("muzzle", (0.0, -0.92, 0.74), (0.20, 0.12, 0.12), belly, 12, 6)
    uv_sphere("tail", (0.0, 0.72, 0.47), (0.18, 0.18, 0.18), belly, 10, 5)

    uv_sphere("left_cheek", (-0.11, -0.94, 0.70), (0.12, 0.08, 0.08), belly, 8, 4)
    uv_sphere("right_cheek", (0.11, -0.94, 0.70), (0.12, 0.08, 0.08), belly, 8, 4)

    for side in (-1.0, 1.0):
        ear = uv_sphere(
            f"ear_{side}",
            (side * 0.18, -0.63, 1.30),
            (0.095, 0.055, 0.48),
            fur,
            12,
            6,
        )
        ear.rotation_euler[1] = math.radians(side * 7.0)
        inner_ear = uv_sphere(
            f"inner_ear_{side}",
            (side * 0.18, -0.685, 1.30),
            (0.050, 0.014, 0.36),
            inner,
            8,
            4,
        )
        inner_ear.rotation_euler[1] = ear.rotation_euler[1]
        uv_sphere(
            f"ear_tip_{side}",
            (side * 0.19, -0.67, 1.70),
            (0.072, 0.032, 0.080),
            ear_tip,
            8,
            4,
        )
        uv_sphere(
            f"eye_patch_{side}",
            (side * 0.17, -0.985, 0.895),
            (0.085, 0.018, 0.075),
            eye_patch,
            8,
            4,
        )
        uv_sphere(f"eye_{side}", (side * 0.18, -0.96, 0.90), (0.050, 0.030, 0.050), eye, 8, 4)
        uv_sphere(
            f"side_eye_{side}",
            (side * 0.31, -0.74, 0.88),
            (0.032, 0.028, 0.032),
            eye,
            8,
            4,
        )
        uv_sphere(
            f"front_paw_{side}",
            (side * 0.22, -0.36, 0.15),
            (0.12, 0.18, 0.08),
            fur,
            8,
            4,
        )
        uv_sphere(
            f"hind_paw_{side}",
            (side * 0.30, 0.34, 0.13),
            (0.15, 0.24, 0.09),
            fur,
            8,
            4,
        )
        for row, z in enumerate((0.70, 0.75, 0.80)):
            whisk = cube(
                f"whisker_{side}_{row}",
                (side * 0.26, -1.02, z),
                (0.18, 0.010, 0.006),
                whisker,
            )
            whisk.rotation_euler[2] = math.radians(side * (row - 1) * 8.0)

    uv_sphere("nose", (0.0, -1.03, 0.78), (0.070, 0.040, 0.045), nose, 8, 4)
    export_glb("rabbit")

def make_berry_bush():
    clear_scene()
    leaf = mat("leaf_green", (0.10, 0.48, 0.18), 0.95)
    dark_leaf = mat("dark_leaf_green", (0.05, 0.30, 0.12), 0.95)
    branch = mat("branch_brown", (0.36, 0.22, 0.12), 0.9)
    fruit = mat("ripe_red_clusters", (0.92, 0.04, 0.06), 0.65)
    shine = mat("berry_shine", (1.0, 0.38, 0.38), 0.55)

    cone("trunk", (0.0, 0.0, 0.28), 0.12, 0.07, 0.56, branch, 8)
    for i in range(11):
        angle = i / 11.0 * math.tau
        radius = 0.20 + (i % 3) * 0.10
        x = math.cos(angle) * radius
        y = math.sin(angle) * radius
        uv_sphere(
            f"leaf_cluster_{i}",
            (x, y, 0.48 + (i % 4) * 0.08),
            (0.30, 0.24, 0.22),
            leaf if i % 2 == 0 else dark_leaf,
            10,
            5,
        )
    berry_positions = [
        (-0.30, -0.28, 0.62),
        (-0.12, -0.42, 0.78),
        (0.10, -0.42, 0.66),
        (0.28, -0.28, 0.82),
        (-0.34, 0.06, 0.74),
        (0.34, 0.03, 0.60),
        (-0.05, 0.28, 0.88),
        (0.18, 0.22, 0.72),
        (0.00, -0.08, 0.96),
    ]
    for i, pos in enumerate(berry_positions):
        uv_sphere(f"fixed_red_berry_{i}", pos, (0.075, 0.075, 0.075), fruit, 8, 4)
        uv_sphere(
            f"fixed_red_berry_highlight_{i}",
            (pos[0] - 0.020, pos[1] - 0.028, pos[2] + 0.026),
            (0.018, 0.018, 0.014),
            shine,
            8,
            4,
        )
    export_glb("berry_bush")

def make_berry_fruit():
    clear_scene()
    fruit = mat("ripe_red", (0.88, 0.06, 0.08), 0.65)
    highlight = mat("fruit_highlight", (1.0, 0.35, 0.35), 0.5)
    leaf = mat("tiny_leaf", (0.10, 0.48, 0.18), 0.9)
    uv_sphere("berry", (0.0, 0.0, 0.16), (0.18, 0.18, 0.18), fruit, 20, 10)
    uv_sphere("highlight", (-0.050, -0.065, 0.23), (0.038, 0.032, 0.026), highlight, 10, 5)
    cone("stem", (0.0, 0.0, 0.38), 0.020, 0.008, 0.18, leaf, 6)
    export_glb("berry_fruit")

def make_co2_bubble():
    clear_scene()
    bubble = mat("co2_translucent_blue", (0.35, 0.78, 1.0), 0.2, alpha=0.35)
    rim = mat("co2_bright_rim", (0.78, 0.94, 1.0), 0.1, alpha=0.55)
    uv_sphere("co2_bubble_big", (0.0, 0.0, 0.0), (0.30, 0.30, 0.30), bubble, 24, 12)
    uv_sphere("co2_bubble_mid", (0.27, -0.08, 0.25), (0.18, 0.18, 0.18), bubble, 20, 10)
    uv_sphere("co2_bubble_small", (-0.22, 0.08, 0.38), (0.13, 0.13, 0.13), bubble, 18, 9)
    uv_sphere("co2_highlight", (-0.09, -0.18, 0.14), (0.045, 0.035, 0.035), rim, 10, 5)
    export_glb("co2_bubble")

if __name__ == "__main__":
    make_rabbit()
    make_berry_bush()
    make_berry_fruit()
    make_co2_bubble()
