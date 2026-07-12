

from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from models_lib import (
    clear_scene,
    cube_y_up as cube,
    cone_y_up as cone,
    cylinder_y_up as cylinder,
    export_glb,
    ico_sphere_y_up as ico_sphere,
    mat,
    merge_into,
    uv_sphere_y_up as uv_sphere,
    OUT_DIR,
)

def make_monster(name: str, base_color, glow_color, kind: str) -> None:
    clear_scene()
    body = mat(f"{name}_body", base_color, roughness=0.6,
               emissive=(glow_color[0] * 0.4, glow_color[1] * 0.4, glow_color[2] * 0.4))
    accent = mat(f"{name}_accent", glow_color, roughness=0.5,
                 emissive=(glow_color[0] * 0.6, glow_color[1] * 0.6, glow_color[2] * 0.6))
    eye = mat(f"{name}_eye", (0.04, 0.03, 0.02), roughness=0.4)

    if kind == "snake":

        for i in range(6):
            t = i / 5.0
            x = (t - 0.5) * 0.7
            z = math.sin(t * 3.14) * 0.20
            scale = 0.18 if i in (0, 5) else 0.22
            uv_sphere(f"body_{i}", (x, 0.0, z), (scale, scale * 0.9, scale), body, 8, 5)

        uv_sphere("head", (-0.45, 0.05, 0.0), (0.26, 0.22, 0.26), body, 10, 6)

        uv_sphere("eye_l", (-0.55, 0.18, 0.10), (0.05, 0.05, 0.05), eye, 6, 4)
        uv_sphere("eye_r", (-0.55, 0.18, -0.10), (0.05, 0.05, 0.05), eye, 6, 4)

        cone("tongue", (-0.65, 0.10, 0.0), 0.012, 0.0, 0.10, accent, vertices=6)

    elif kind == "frost_elf":

        cone("body", (0.0, 0.05, 0.0), 0.20, 0.42, 0.70, body, vertices=8)
        uv_sphere("head", (0.0, 0.55, 0.0), (0.30, 0.28, 0.28), body, 16, 10)

        cone("hat", (0.0, 0.95, 0.0), 0.22, 0.04, 0.55, accent, vertices=8)

        uv_sphere("eye_l", (-0.10, 0.60, 0.24), (0.06, 0.06, 0.04), eye, 8, 6)
        uv_sphere("eye_r", (0.10, 0.60, 0.24), (0.06, 0.06, 0.04), eye, 8, 6)

        uv_sphere("hand_l", (-0.35, 0.10, 0.0), (0.10, 0.10, 0.10), accent, 10, 6)
        uv_sphere("hand_r", (0.35, 0.10, 0.0), (0.10, 0.10, 0.10), accent, 10, 6)

    elif kind == "sand_wurm":

        for i in range(4):
            t = i / 3.0
            x = (t - 0.5) * 0.5
            scale = 0.18 - t * 0.05
            uv_sphere(f"body_{i}", (x, 0.0, 0.0), (scale, scale * 0.85, scale), body, 8, 5)
        uv_sphere("head", (0.30, 0.05, 0.0), (0.32, 0.26, 0.28), body, 10, 6)

        cone("tentacle_l", (-0.35, 0.05, 0.10), 0.025, 0.0, 0.18, accent, vertices=6)
        cone("tentacle_r", (-0.35, 0.05, -0.10), 0.025, 0.0, 0.18, accent, vertices=6)

        uv_sphere("eye_l", (0.40, 0.18, 0.10), (0.06, 0.06, 0.04), eye, 6, 4)
        uv_sphere("eye_r", (0.40, 0.18, -0.10), (0.06, 0.06, 0.04), eye, 6, 4)

    elif kind == "treant":

        cylinder("trunk", (0.0, 0.20, 0.0), 0.30, 0.60, body, vertices=10)
        uv_sphere("canopy", (0.0, 0.65, 0.0), (0.50, 0.40, 0.50), accent, 12, 8)

        cylinder("arm_l", (-0.40, 0.45, 0.0), 0.08, 0.55, body, vertices=8)
        cylinder("arm_r", (0.40, 0.45, 0.0), 0.08, 0.55, body, vertices=8)

        uv_sphere("hand_l", (-0.40, 0.15, 0.0), (0.13, 0.13, 0.13), accent, 10, 8)
        uv_sphere("hand_r", (0.40, 0.15, 0.0), (0.13, 0.13, 0.13), accent, 10, 8)

        eye_glow = mat("treant_eye_glow", (1.0, 0.6, 0.1), roughness=0.3,
                       emissive=(1.0, 0.5, 0.05))
        uv_sphere("eye_l", (-0.10, 0.65, 0.40), (0.06, 0.06, 0.04), eye_glow, 8, 6)
        uv_sphere("eye_r", (0.10, 0.65, 0.40), (0.06, 0.06, 0.04), eye_glow, 8, 6)

    elif kind == "aether_wraith":

        cone("body", (0.0, 0.10, 0.0), 0.42, 0.05, 0.85, body, vertices=8)
        uv_sphere("head", (0.0, 0.65, 0.0), (0.32, 0.34, 0.30), body, 16, 10)

        cube("wisp_l", (-0.30, 0.20, 0.0), (0.05, 0.55, 0.10), accent)
        cube("wisp_r", (0.30, 0.20, 0.0), (0.05, 0.55, 0.10), accent)

        eye_glow = mat("wraith_eye_glow", (1.0, 0.3, 0.95), roughness=0.3,
                       emissive=(1.0, 0.25, 0.95))
        uv_sphere("eye_l", (-0.10, 0.70, 0.26), (0.06, 0.06, 0.04), eye_glow, 8, 6)
        uv_sphere("eye_r", (0.10, 0.70, 0.26), (0.06, 0.06, 0.04), eye_glow, 8, 6)

    export_glb(name)

def make_cloud_puff() -> None:
    clear_scene()
    cloud = mat("cloud_white", (0.92, 0.95, 1.0), roughness=0.95, alpha=0.85)
    rim = mat("cloud_rim", (1.0, 1.0, 1.0), roughness=0.85, alpha=0.70)

    uv_sphere("cloud_main", (0.0, 0.0, 0.0), (1.6, 1.6, 1.6), cloud, 12, 7)
    uv_sphere("cloud_l", (-1.5, 0.2, 0.0), (1.1, 1.1, 1.1), cloud, 10, 6)
    uv_sphere("cloud_r", (1.6, -0.1, 0.5), (1.2, 1.2, 1.2), cloud, 10, 6)
    uv_sphere("cloud_top", (0.3, 1.0, -0.2), (0.9, 0.9, 0.9), rim, 8, 5)
    export_glb("cloud_puff")

def make_rock(name: str, color, scale_hint: float = 0.7) -> None:
    clear_scene()
    body = mat(f"rock_{name}", color, roughness=0.95)
    accent = mat(f"rock_{name}_highlight",
                 (min(1.0, color[0] + 0.1), min(1.0, color[1] + 0.1), min(1.0, color[2] + 0.1)),
                 roughness=0.9)

    ico_sphere("rock_main", (0.0, 0.0, 0.0),
               (0.40 * scale_hint, 0.30 * scale_hint, 0.40 * scale_hint),
               body, subdivisions=1)

    ico_sphere("rock_top", (0.06, 0.20 * scale_hint, -0.04),
               (0.18 * scale_hint, 0.14 * scale_hint, 0.18 * scale_hint),
               accent, subdivisions=1)

    ico_sphere("rock_base", (-0.10, -0.12 * scale_hint, 0.08),
               (0.22 * scale_hint, 0.10 * scale_hint, 0.22 * scale_hint),
               body, subdivisions=1)
    export_glb(f"rock_{name}")

def make_flower(color, idx: int) -> None:
    clear_scene()
    petal = mat(f"flower_petal_{idx}", color, roughness=0.65)
    stem = mat(f"flower_stem_{idx}", (0.30, 0.55, 0.20), roughness=0.9)
    center = mat(f"flower_center_{idx}", (1.0, 0.92, 0.40), roughness=0.7,
                 emissive=(0.4, 0.35, 0.10))

    cylinder("stem", (0.0, 0.20, 0.0), 0.025, 0.40, stem, vertices=6)

    cube("leaf_l", (-0.10, 0.20, 0.0), (0.16, 0.04, 0.08), stem)
    cube("leaf_r", (0.10, 0.22, 0.0), (0.16, 0.04, 0.08), stem)

    for k in range(5):
        angle = k * 1.2566
        r = 0.18
        x = math.cos(angle) * r
        z = math.sin(angle) * r
        uv_sphere(f"petal_{k}", (x, 0.50, z), (0.10, 0.08, 0.10), petal, 10, 6)

    uv_sphere("flower_center", (0.0, 0.50, 0.0), (0.08, 0.06, 0.08), center, 10, 6)
    export_glb(f"flower_{idx}")

def make_hill() -> None:
    clear_scene()
    body = mat("hill_green", (0.20, 0.42, 0.18), roughness=0.95)
    accent = mat("hill_green_dark", (0.10, 0.28, 0.10), roughness=0.95)

    ico_sphere("hill_main", (0.0, 0.0, 0.0), (2.5, 1.5, 2.5), body, subdivisions=1)

    ico_sphere("hill_peak", (0.30, 0.80, -0.20), (1.0, 0.7, 1.0), accent, subdivisions=1)
    export_glb("hill")

def make_poi_pillar(name: str, base_color, glow_color) -> None:
    clear_scene()
    body = mat(f"poi_{name}_body", base_color, roughness=0.62,
               emissive=(glow_color[0] * 0.3, glow_color[1] * 0.3, glow_color[2] * 0.3))
    glow = mat(f"poi_{name}_glow", glow_color, roughness=0.38,
               emissive=glow_color)

    cube("pillar", (0.0, 0.0, 0.0), (0.45, 2.8, 0.45), body)

    cube("base", (0.0, -1.45, 0.0), (0.70, 0.10, 0.70), glow)

    uv_sphere("top", (0.0, 1.55, 0.0), (0.31, 0.31, 0.31), glow, 16, 10)

    for sx, sz in ((-0.18, -0.18), (0.18, -0.18), (-0.18, 0.18), (0.18, 0.18)):
        cone("spike", (sx, 1.85, sz), 0.04, 0.0, 0.18, glow, vertices=4)
    export_glb(f"poi_pillar_{name}")

def make_ground_disc(name: str, radius: float, depth: float,
                     color, glow: tuple) -> None:
    clear_scene()
    body = mat(f"disc_{name}_body", color, roughness=0.95, alpha=0.85,
               emissive=(glow[0] * 0.5, glow[1] * 0.5, glow[2] * 0.5))

    cylinder("disc", (0.0, 0.0, 0.0), radius, depth, body, vertices=12)
    export_glb(f"ground_disc_{name}")

def main() -> None:
    print("=== building pretty/ models ===")


    print("[1/7] monsters (5)")
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

    print("[2/7] cloud_puff")
    make_cloud_puff()

    print("[3/7] rocks (3)")
    make_rock("dark", (0.42, 0.42, 0.45), scale_hint=1.0)
    make_rock("mid",  (0.58, 0.55, 0.50), scale_hint=0.9)
    make_rock("moss", (0.50, 0.52, 0.48), scale_hint=0.8)

    print("[4/7] flowers (5)")
    flower_colors = [
        ("pink",   (0.98, 0.30, 0.55)),
        ("yellow", (1.00, 0.85, 0.20)),
        ("purple", (0.55, 0.30, 0.98)),
        ("orange", (1.00, 0.45, 0.20)),
        ("red",    (0.95, 0.30, 0.30)),
    ]
    for idx, (_n, c) in enumerate(flower_colors):
        make_flower(c, idx)

    print("[5/7] hill")
    make_hill()

    print("[6/7] poi_pillars (4)")
    make_poi_pillar("red",  (0.95, 0.20, 0.08), (1.00, 0.55, 0.18))
    make_poi_pillar("cyan", (0.05, 0.48, 0.50), (0.18, 0.90, 0.82))
    make_poi_pillar("pink", (0.72, 0.06, 0.25), (1.00, 0.18, 0.40))
    make_poi_pillar("gold", (0.82, 0.58, 0.14), (1.00, 0.80, 0.22))

    print("[7/7] ground_discs")
    make_ground_disc("outer", 6.0, 0.20,
                     color=(0.32, 0.48, 0.20), glow=(0.20, 0.40, 0.10))
    make_ground_disc("inner", 2.5, 0.20,
                     color=(0.55, 0.75, 0.30), glow=(0.30, 0.50, 0.15))

    print(f"\n=== done. {len(list(OUT_DIR.glob('*.glb')))} .glb files in {OUT_DIR} ===")

def build_manifest() -> dict:
    
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
