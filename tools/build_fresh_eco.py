"""Build the ecology props from the fresh collection-shape language."""

from __future__ import annotations

import math
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

from fresh_shapes import blob, finish, fresh_mat, line, readable_eye, reset  # noqa: E402
import models_lib  # noqa: F401, E402


ASSETS = ("rabbit", "berry_bush", "berry_fruit", "co2_bubble")


def make_asset(name: str):
    reset()
    if name == "rabbit":
        skin = fresh_mat(name, "snow")
        blob("body", (0, 0, 0.40), (0.38, 0.24, 0.28), skin)
        blob("head", (0.25, -0.02, 0.70), (0.22, 0.18, 0.20), skin)
        blob("belly", (-0.02, -0.21, 0.37), (0.27, 0.035, 0.13), fresh_mat(name, "cream"), 12, 6)
        readable_eye("eye", (0.31, -0.17, 0.78), fresh_mat(name, "ink"), size=(0.045, 0.030, 0.045), socket_role="cream")
        line("ear_l", (0.18, 0, 0.82), (0.10, 0, 1.25), 0.06, skin)
        line("ear_r", (0.32, 0, 0.82), (0.40, 0, 1.25), 0.06, skin)
    elif name == "berry_bush":
        leaf = fresh_mat(name, "leaf")
        fruit = fresh_mat(name, "coral")
        for i in range(5):
            angle = i * math.tau / 5.0
            blob(f"leaf_{i}", (math.cos(angle) * 0.35, math.sin(angle) * 0.25, 0.42 + (i % 2) * 0.15), (0.45, 0.34, 0.40), leaf)
            blob(f"berry_{i}", (math.cos(angle) * 0.45, math.sin(angle) * 0.30, 0.74), (0.07, 0.07, 0.07), fruit, 8, 4)
    elif name == "berry_fruit":
        blob("fruit", (0, 0, 0.14), (0.14, 0.12, 0.14), fresh_mat(name, "coral"), 10, 5)
        blob("leaf", (0.08, 0, 0.29), (0.12, 0.05, 0.04), fresh_mat(name, "leaf"), 8, 4)
    else:
        bubble = fresh_mat(name, "water")
        blob("bubble", (0, 0, 0.45), (0.36, 0.36, 0.36), bubble, 12, 6)
        blob("bubble_core", (-0.10, -0.25, 0.58), (0.08, 0.05, 0.08), fresh_mat(name, "snow"), 8, 4)
    finish(name, "eco")


def main():
    for name in ASSETS:
        make_asset(name)


if __name__ == "__main__":
    main()
