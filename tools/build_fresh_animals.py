"""Build farm and forest animals from broad, readable collection shapes."""

from __future__ import annotations

import os
import sys
import math

sys.path.insert(0, os.path.dirname(__file__))

from fresh_shapes import blob, finish, fresh_mat, line, make_animal, readable_eye, reset  # noqa: E402
import models_lib  # noqa: F401, E402


ASSETS = (
    "pig", "sheep", "cow", "chicken", "rabbit", "rabbit_brown",
    "deer", "deer_fawn", "fox", "fox_silver", "fish",
)


def make_asset(name: str):
    reset()
    if name == "pig":
        make_animal(
            name, "coral", "peach",
            body_loc=(0.0, 0.0, 0.58), body_scale=(0.58, 0.31, 0.32),
            head_loc=(0.43, -0.02, 0.72), head_scale=(0.34, 0.27, 0.28),
            snout_loc=(0.68, -0.20, 0.68), snout_scale=(0.20, 0.14, 0.14),
            leg_x=(-0.30, 0.30), leg_top=0.40, leg_radius=0.07,
            tail_start=(-0.52, 0.02, 0.68), tail_end=(-0.78, 0.04, 0.84), tail_radius=0.06,
            eye_loc=(0.56, -0.24, 0.83), eye_size=(0.050, 0.032, 0.050), belly_role="peach",
        )
        line("ear_l", (0.30, 0.0, 0.94), (0.22, 0.01, 1.12), 0.07, fresh_mat(name, "peach"))
        line("ear_r", (0.50, 0.0, 0.94), (0.59, 0.01, 1.10), 0.07, fresh_mat(name, "peach"))
        blob("tail_curl", (-0.80, 0.04, 0.84), (0.10, 0.07, 0.10), fresh_mat(name, "peach"), 8, 4)
    elif name == "sheep":
        make_animal(
            name, "snow", "ink",
            body_scale=(0.54, 0.32, 0.35), head_loc=(0.39, -0.02, 0.88),
            head_scale=(0.24, 0.21, 0.25), snout_loc=(0.56, -0.17, 0.83),
            snout_scale=(0.13, 0.10, 0.11), leg_x=(-0.28, 0.28), leg_top=0.45,
            eye_loc=(0.48, -0.19, 1.01), eye_size=(0.045, 0.030, 0.045),
        )
        blob("wool_top", (0, 0, 0.91), (0.56, 0.34, 0.38), fresh_mat(name, "snow"))
        line("ear_l", (0.28, 0.0, 1.04), (0.16, 0.01, 1.14), 0.055, fresh_mat(name, "ink"))
        line("ear_r", (0.48, 0.0, 1.04), (0.60, 0.01, 1.14), 0.055, fresh_mat(name, "ink"))
    elif name == "cow":
        make_animal(
            name, "snow", "ink",
            body_scale=(0.70, 0.36, 0.38), head_loc=(0.45, -0.02, 0.94),
            head_scale=(0.34, 0.27, 0.30), snout_loc=(0.68, -0.19, 0.88),
            snout_scale=(0.20, 0.14, 0.14), leg_x=(-0.36, 0.36), leg_top=0.50,
            leg_radius=0.07, tail_start=(-0.58, 0.02, 0.72), tail_end=(-0.72, 0.02, 0.30),
            tail_radius=0.065, eye_loc=(0.56, -0.24, 1.09), eye_size=(0.055, 0.034, 0.055),
        )
        for i, x in enumerate((-0.18, 0.20)):
            blob(f"patch_{i}", (x, -0.34, 0.78), (0.18, 0.045, 0.16), fresh_mat(name, "ink"), 8, 4)
        blob("tail_tuft", (-0.73, 0.02, 0.22), (0.11, 0.08, 0.15), fresh_mat(name, "ink"), 8, 5)
        line("horn_l", (0.34, 0.0, 1.18), (0.20, 0.01, 1.36), 0.045, fresh_mat(name, "ink"))
        line("horn_r", (0.54, 0.0, 1.18), (0.68, 0.01, 1.36), 0.045, fresh_mat(name, "ink"))
    elif name == "chicken":
        make_animal(
            name, "snow", "yellow",
            body_loc=(0.0, 0.0, 0.62), body_scale=(0.38, 0.26, 0.42),
            head_loc=(0.33, -0.02, 1.04), head_scale=(0.22, 0.20, 0.23),
            snout_loc=(0.51, -0.19, 0.99), snout_scale=(0.13, 0.08, 0.08),
            leg_x=(-0.14, 0.14), leg_top=0.35, leg_radius=0.04,
            tail_start=(-0.28, 0.02, 0.78), tail_end=(-0.56, 0.04, 1.04), tail_radius=0.05,
            eye_loc=(0.40, -0.20, 1.12), eye_size=(0.040, 0.026, 0.040), belly_role="yellow",
            leg_role="yellow", foot_role="yellow", foot_forward=0.10, tail_enabled=False,
        )
        blob("neck", (0.25, 0.0, 0.86), (0.17, 0.17, 0.24), fresh_mat(name, "snow"), 12, 8)
        coral = fresh_mat(name, "coral")
        for index, (x, z, scale) in enumerate(
            (
                (0.28, 1.23, (0.075, 0.065, 0.085)),
                (0.37, 1.27, (0.085, 0.070, 0.105)),
                (0.45, 1.23, (0.070, 0.060, 0.080)),
            )
        ):
            blob(f"comb_{index}", (x, -0.05, z), scale, coral, 8, 4)
        blob("wattle", (0.47, -0.18, 0.91), (0.060, 0.040, 0.090), coral, 8, 5)
        models_lib.cone_between(
            "beak",
            (0.49, -0.19, 1.00),
            (0.69, -0.20, 0.98),
            0.060,
            fresh_mat(name, "yellow"),
            vertices=6,
        )
        near_wing = blob(
            "wing_l", (0.02, -0.25, 0.82), (0.25, 0.065, 0.19),
            fresh_mat(name, "cream"), 12, 6,
        )
        near_wing.rotation_euler.y = math.radians(-14.0)
        far_wing = blob(
            "wing_r", (0.02, 0.25, 0.82), (0.25, 0.065, 0.19),
            fresh_mat(name, "cream"), 12, 6,
        )
        far_wing.rotation_euler.y = math.radians(-14.0)
        tail_material = fresh_mat(name, "snow")
        blob("tail_base", (-0.25, 0.0, 0.80), (0.16, 0.18, 0.18), tail_material, 10, 6)
        for index, (y, tip_x, tip_z, radius) in enumerate(
            (
                (-0.10, -0.62, 1.06, 0.095),
                (0.0, -0.72, 1.20, 0.105),
                (0.10, -0.62, 1.06, 0.095),
            )
        ):
            feather = models_lib.cone_between(
                f"tail_feather_{index}",
                (-0.23, y, 0.78),
                (tip_x, y * 1.15, tip_z),
                radius,
                tail_material,
                vertices=6,
            )
            # Flatten the low-poly cone across its local width so it reads as
            # a feather blade instead of three rigid spikes.
            feather.scale.y = 0.52
    elif name in {"rabbit", "rabbit_brown"}:
        make_animal(
            name, "wood" if name.endswith("brown") else "snow", "peach",
            body_scale=(0.36, 0.24, 0.29), head_loc=(0.34, -0.02, 0.84),
            head_scale=(0.25, 0.21, 0.27), snout_loc=(0.53, -0.17, 0.80),
            snout_scale=(0.12, 0.09, 0.09), leg_x=(-0.19, 0.19), leg_top=0.40,
            eye_loc=(0.43, -0.18, 0.96), eye_size=(0.048, 0.030, 0.048),
        )
        line("ear_l", (0.26, 0, 0.98), (0.20, 0.01, 1.42), 0.07, fresh_mat(name, "peach"))
        line("ear_r", (0.48, 0, 0.98), (0.54, 0.01, 1.42), 0.07, fresh_mat(name, "peach"))
        blob("tail_ball", (-0.42, 0.03, 0.75), (0.13, 0.12, 0.13), fresh_mat(name, "cream"), 10, 6)
    elif name in {"deer", "deer_fawn"}:
        make_animal(
            name, "yellow" if name.endswith("fawn") else "wood", "cream",
            body_scale=(0.53, 0.28, 0.30), head_loc=(0.43, -0.01, 1.02),
            head_scale=(0.25, 0.20, 0.25), snout_loc=(0.65, -0.16, 0.96),
            snout_scale=(0.14, 0.09, 0.10), leg_x=(-0.27, 0.27), leg_top=0.52,
            leg_radius=0.045, tail_start=(-0.46, 0.02, 0.76), tail_end=(-0.70, 0.04, 0.96),
            eye_loc=(0.50, -0.17, 1.13), eye_size=(0.045, 0.028, 0.045),
        )
        neck = fresh_mat(name, "wood")
        # Keep the neck and antler roots deliberately overlapped with the
        # generic head volume. The previous high antler roots floated above
        # the head in the target camera and read as detached sticks.
        line("neck", (0.24, 0, 0.78), (0.35, 0, 1.20), 0.13, neck)
        line("antler_l", (0.34, 0, 1.12), (0.23, 0, 1.52), 0.045, neck)
        line("antler_r", (0.46, 0, 1.12), (0.57, 0, 1.52), 0.045, neck)
    elif name in {"fox", "fox_silver"}:
        fur = "stone" if name.endswith("silver") else "coral"
        make_animal(
            name, fur, "cream",
            body_scale=(0.56, 0.25, 0.28), head_loc=(0.45, -0.02, 0.90),
            head_scale=(0.25, 0.20, 0.25), snout_loc=(0.69, -0.16, 0.83),
            snout_scale=(0.18, 0.10, 0.11), leg_x=(-0.28, 0.28), leg_top=0.47,
            leg_radius=0.05, tail_start=(-0.48, 0.02, 0.74), tail_end=(-0.92, 0.04, 1.04),
            tail_radius=0.10, eye_loc=(0.55, -0.17, 1.02), eye_size=(0.048, 0.030, 0.048),
        )
        ear = fresh_mat(name, "stone")
        line("ear_l", (0.32, 0, 1.05), (0.22, 0.01, 1.42), 0.075, ear)
        line("ear_r", (0.51, 0, 1.05), (0.62, 0.01, 1.42), 0.075, ear)
        blob("tail_tip", (-0.96, 0.04, 1.08), (0.15, 0.11, 0.16), fresh_mat(name, "cream"), 10, 6)
    elif name == "fish":
        body = fresh_mat(name, "fish_blue")
        fin = fresh_mat(name, "fish_cyan")
        deep = fresh_mat(name, "fish_deep")
        ink = fresh_mat(name, "ink")
        # One continuous volume avoids the visible seam caused by overlapping
        # body/head spheres. A light subdivision pass keeps the surface soft.
        body_obj = blob("body", (0.02, 0.0, 0.58), (0.72, 0.34, 0.36), body, 20, 12)
        models_lib.smooth_organic(body_obj, subdivision_levels=1)
        # Thin, wide fan lobes keep a readable fish-tail silhouette while
        # the rounded profile and bevel remove the old hard triangle edges.
        models_lib.soft_fin(
            "tail_upper",
            [
                (-0.46, 0.63), (-0.62, 0.78), (-0.86, 0.94),
                (-1.08, 0.94), (-1.17, 0.84), (-1.08, 0.69),
                (-0.86, 0.56), (-0.62, 0.52),
            ],
            0.11,
            fin,
            y=0.0,
            bevel=0.035,
            bevel_segments=2,
        )
        models_lib.soft_fin(
            "tail_lower",
            [
                (-0.46, 0.47), (-0.62, 0.32), (-0.86, 0.16),
                (-1.08, 0.16), (-1.17, 0.26), (-1.08, 0.41),
                (-0.86, 0.54), (-0.62, 0.58),
            ],
            0.11,
            fin,
            y=0.0,
            bevel=0.035,
            bevel_segments=2,
        )
        blob("tail_joint", (-0.54, 0.0, 0.55), (0.20, 0.18, 0.15), fin, 12, 6)
        models_lib.soft_fin(
            "dorsal_fin",
            [(-0.18, 0.84), (-0.04, 1.05), (0.13, 1.12), (0.25, 0.84), (0.15, 0.80)],
            0.11,
            fin,
            y=0.0,
            bevel=0.030,
            bevel_segments=3,
        )
        models_lib.soft_fin(
            "belly_fin",
            [(-0.10, 0.33), (0.04, 0.11), (0.20, 0.08), (0.34, 0.18), (0.28, 0.36)],
            0.10,
            fin,
            y=0.02,
            bevel=0.030,
            bevel_segments=3,
        )
        models_lib.soft_fin(
            "pectoral_fin",
            [(0.02, 0.63), (0.20, 0.57), (0.36, 0.43), (0.40, 0.35), (0.28, 0.50)],
            0.08,
            deep,
            y=-0.31,
            bevel=0.025,
            bevel_segments=3,
        )
        blob("gill", (0.25, -0.30, 0.50), (0.10, 0.018, 0.13), deep, 8, 4)
        readable_eye(
            "eye",
            (0.48, -0.28, 0.70),
            ink,
            size=(0.065, 0.035, 0.065),
            socket_role="snow",
        )
    finish(name, "animals")


def main():
    for name in ASSETS:
        make_asset(name)


if __name__ == "__main__":
    main()
