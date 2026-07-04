

from __future__ import annotations
import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import models_lib

from models_lib import (
    clear_scene, cube, cone, cylinder, export_glb, ico_sphere, mat, uv_sphere,
)

def make_player_avatar_flat() -> None:
    
    clear_scene()
    body_brown = mat("body_brown", (0.62, 0.42, 0.25))
    skin = mat("skin_warm", (0.95, 0.78, 0.65))
    hat_straw = mat("hat_straw", (0.78, 0.62, 0.32))
    eye_blue = mat("eye_blue", (0.25, 0.50, 0.85))
    pants = mat("pants_grey", (0.45, 0.40, 0.32))

    cube("feet", (0.0, 0.05, 0.0), (0.4, 0.10, 0.4), pants)

    cube("torso", (0.0, 0.20, 0.0), (0.5, 0.20, 0.3), body_brown)

    cube("head", (0.0, 0.375, 0.0), (0.30, 0.15, 0.25), skin)

    cube("eye_l", (-0.08, 0.385, 0.13), (0.05, 0.05, 0.04), eye_blue)
    cube("eye_r", ( 0.08, 0.385, 0.13), (0.05, 0.05, 0.04), eye_blue)

    cylinder("hat", (0.0, 0.50, 0.0), 0.20, 0.05, hat_straw, vertices=10)
    export_glb("player_avatar")

def make_monster_flat(name: str, base_color, glow_color) -> None:
    
    clear_scene()
    body = mat(f"{name}_body", base_color, emissive=(glow_color[0]*0.3, glow_color[1]*0.3, glow_color[2]*0.3))
    eye = mat(f"{name}_eye", (0.04, 0.03, 0.02))

    uv_sphere("body", (0.0, 0.30, 0.0), (0.30, 0.18, 0.30), body, 10, 6)

    cube("eye_l", (-0.10, 0.35, 0.20), (0.04, 0.04, 0.04), eye)
    cube("eye_r", ( 0.10, 0.35, 0.20), (0.04, 0.04, 0.04), eye)

    cone("spike", (0.0, 0.50, 0.0), 0.06, 0.0, 0.05, body, vertices=4)
    export_glb(name)

def make_cloud_puff_flat() -> None:
    clear_scene()
    cloud = mat("cloud_white", (0.92, 0.95, 1.0), alpha=0.85)

    uv_sphere("c1", (0.0, 0.20, 0.0), (0.40, 0.20, 0.40), cloud, 10, 6)
    uv_sphere("c2", (-0.30, 0.20, 0.0), (0.30, 0.18, 0.30), cloud, 10, 6)
    uv_sphere("c3", ( 0.30, 0.18, 0.0), (0.30, 0.18, 0.30), cloud, 10, 6)
    uv_sphere("c4", (0.0, 0.30, -0.20), (0.30, 0.15, 0.20), cloud, 10, 6)
    export_glb("cloud_puff")

def make_tree_flat() -> None:
    clear_scene()
    bark = mat("tree_bark", (0.45, 0.27, 0.10))
    leaf = mat("tree_leaf", (0.25, 0.55, 0.20))
    leaf_dark = mat("tree_leaf_dark", (0.15, 0.40, 0.15))

    cube("trunk", (0.0, 0.15, 0.0), (0.20, 0.30, 0.20), bark)

    uv_sphere("canopy_low", (0.0, 0.40, 0.0), (0.35, 0.18, 0.35), leaf, 10, 6)
    uv_sphere("canopy_top", (0.05, 0.50, 0.05), (0.25, 0.12, 0.25), leaf_dark, 10, 6)
    export_glb("tree")

def make_rock_flat(name: str, color) -> None:
    clear_scene()
    body = mat(f"rock_{name}", color)

    ico_sphere("main", (0.0, 0.10, 0.0), (0.20, 0.10, 0.20), body, subdivisions=0)
    ico_sphere("top", (0.05, 0.15, -0.03), (0.08, 0.05, 0.08), body, subdivisions=0)
    export_glb(f"rock_{name}")

def make_flower_flat(idx: int, color) -> None:
    clear_scene()
    petal = mat(f"flower_petal_{idx}", color)
    stem = mat(f"flower_stem_{idx}", (0.30, 0.55, 0.20))

    cube("stem", (0.0, 0.10, 0.0), (0.02, 0.20, 0.02), stem)

    uv_sphere("p1", (-0.05, 0.28, 0.0), (0.06, 0.04, 0.06), petal, 8, 5)
    uv_sphere("p2", ( 0.05, 0.30, 0.0), (0.06, 0.04, 0.06), petal, 8, 5)
    uv_sphere("p3", (0.0, 0.32, 0.05), (0.06, 0.04, 0.06), petal, 8, 5)
    uv_sphere("center", (0.0, 0.32, 0.0), (0.03, 0.03, 0.03), mat(f"fc_{idx}", (1.0, 0.92, 0.40)), 8, 5)
    export_glb(f"flower_{idx}")

def make_hill_flat() -> None:
    clear_scene()
    body = mat("hill_green", (0.20, 0.42, 0.18))

    ico_sphere("hill", (0.0, 0.15, 0.0), (0.60, 0.15, 0.60), body, subdivisions=0)
    export_glb("hill")

def make_poi_pillar_flat(name: str, base_color, glow_color) -> None:
    clear_scene()
    body = mat(f"poi_{name}_body", base_color, emissive=(glow_color[0]*0.3, glow_color[1]*0.3, glow_color[2]*0.3))
    glow = mat(f"poi_{name}_glow", glow_color, emissive=(glow_color[0]*1.0, glow_color[1]*1.0, glow_color[2]*1.0))

    cube("pillar", (0.0, 0.20, 0.0), (0.20, 0.40, 0.20), body)
    uv_sphere("top", (0.0, 0.45, 0.0), (0.10, 0.06, 0.10), glow, 10, 6)

    for sx, sz in ((-0.10, -0.10), (0.10, -0.10), (-0.10, 0.10), (0.10, 0.10)):
        cone(f"sp_{sx}_{sz}", (sx, 0.05, sz), 0.02, 0.0, 0.05, glow, vertices=4)
    export_glb(f"poi_pillar_{name}")

def make_ground_disc_flat(name: str, radius: float, color, glow) -> None:
    clear_scene()
    body = mat(f"disc_{name}_body", color, alpha=0.85, emissive=(glow[0]*0.5, glow[1]*0.5, glow[2]*0.5))

    cylinder("disc", (0.0, 0.025, 0.0), radius, 0.05, body, vertices=8)
    export_glb(f"ground_disc_{name}")

def main() -> None:
    print("=== building pretty/ v4-flat (超平坦贴地) ===")

    print("[1/9] player_avatar (≤0.5m 蘑菇人)")
    make_player_avatar_flat()

    print("[2/9] monsters x5 (扁球身 ≤0.5m)")
    make_monster_flat("monster_snake",     (0.50, 0.85, 0.20), (0.70, 1.0, 0.30))
    make_monster_flat("monster_frost_elf", (0.30, 0.70, 0.95), (0.50, 0.90, 1.0))
    make_monster_flat("monster_sand_wurm", (0.95, 0.70, 0.20), (1.0, 0.85, 0.30))
    make_monster_flat("monster_treant",    (0.40, 0.25, 0.10), (0.55, 0.35, 0.15))
    make_monster_flat("monster_aether_wraith", (0.70, 0.30, 0.85), (0.95, 0.45, 1.0))

    print("[3/9] cloud_puff (贴地 ≤0.35m)")
    make_cloud_puff_flat()

    print("[4/9] tree (矮粗 + 双球冠 ≤0.50m)")
    make_tree_flat()

    print("[5/9] rocks x3 (扁球 ≤0.20m)")
    make_rock_flat("dark", (0.42, 0.42, 0.45))
    make_rock_flat("mid",  (0.58, 0.55, 0.50))
    make_rock_flat("moss", (0.50, 0.52, 0.48))

    print("[6/9] flowers x5 (扁小花 ≤0.35m)")
    flower_colors = [
        (0.98, 0.30, 0.55),
        (1.00, 0.85, 0.20),
        (0.55, 0.30, 0.98),
        (1.00, 0.45, 0.20),
        (0.95, 0.30, 0.30),
    ]
    for i, c in enumerate(flower_colors):
        make_flower_flat(i, c)

    print("[7/9] hill (扁山 ≤0.30m)")
    make_hill_flat()

    print("[8/9] poi_pillars x4 (矮标柱 ≤0.50m)")
    make_poi_pillar_flat("red",  (0.95, 0.20, 0.08), (1.00, 0.55, 0.18))
    make_poi_pillar_flat("cyan", (0.05, 0.48, 0.50), (0.18, 0.90, 0.82))
    make_poi_pillar_flat("pink", (0.72, 0.06, 0.25), (1.00, 0.18, 0.40))
    make_poi_pillar_flat("gold", (0.82, 0.58, 0.14), (1.00, 0.80, 0.22))

    print("[9/9] ground_discs (扁圆盘)")
    make_ground_disc_flat("outer", 1.0, (0.32, 0.48, 0.20), (0.20, 0.40, 0.10))
    make_ground_disc_flat("inner", 0.4, (0.55, 0.75, 0.30), (0.30, 0.50, 0.15))

    manifest = {
        "version": 4,
        "spec": "flat (all y ≤ 0.5m, 贴地, no fall/no upside-down)",
        "format": "glb",
        "assets": [
            "player_avatar", "monster_snake", "monster_frost_elf", "monster_sand_wurm",
            "monster_treant", "monster_aether_wraith", "cloud_puff", "tree",
            "rock_dark", "rock_mid", "rock_moss", "flower_0", "flower_1", "flower_2",
            "flower_3", "flower_4", "hill", "poi_pillar_red", "poi_pillar_cyan",
            "poi_pillar_pink", "poi_pillar_gold", "ground_disc_outer", "ground_disc_inner",
        ],
    }
    mp = models_lib.OUT_DIR / "MANIFEST.json"
    mp.write_text(json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\nMANIFEST -> {mp}")
    print(f"=== done. {len(list(models_lib.OUT_DIR.glob('*.glb')))} glbs ===")

if __name__ == "__main__":
    main()
