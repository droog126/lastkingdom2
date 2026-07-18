"""Build the reference sword asset."""

from __future__ import annotations

import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from models_lib import (  # noqa: E402
    bevel_box,
    clear_scene,
    cylinder,
    export_glb,
    mat,
    prism,
    uv_sphere,
)


def make_sword():
    clear_scene()
    blade = mat("reference_sword_blade", (0.50, 0.58, 0.62), roughness=0.70, metallic=0.12)
    edge = mat("reference_sword_edge", (0.86, 0.90, 0.88), roughness=0.62, metallic=0.15)
    grip = mat("reference_sword_grip", (0.27, 0.13, 0.07), roughness=0.88)
    guard = mat("reference_sword_guard", (0.76, 0.48, 0.16), roughness=0.70, metallic=0.10)
    cylinder("grip", (0.0, 0.0, 0.31), 0.055, 0.52, grip, vertices=24)
    bevel_box("guard", (0.0, 0.0, 0.64), (0.56, 0.13, 0.10), guard, bevel=0.05)
    prism(
        "blade",
        [(-0.16, 0.70), (0.16, 0.70), (0.12, 1.45), (0.0, 1.78), (-0.12, 1.45)],
        0.10,
        blade,
        y=0.0,
        bevel=0.025,
    )
    prism(
        "blade_edge",
        [(0.07, 0.76), (0.12, 0.76), (0.08, 1.43), (0.0, 1.68)],
        0.026,
        edge,
        y=-0.064,
        bevel=0.01,
    )
    uv_sphere("pommel", (0.0, 0.0, 0.04), (0.09, 0.09, 0.09), guard, 24, 12)
    export_glb("sword", collection="pretty")


def main():
    selected_asset = os.environ.get("LK2_MODEL_ONLY")
    if not selected_asset or selected_asset == "sword":
        make_sword()


if __name__ == "__main__":
    main()
