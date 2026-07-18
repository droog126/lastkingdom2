"""Build every pretty asset from the new collection-shape language."""

from __future__ import annotations

import math
import os
import sys

HERE = os.path.dirname(__file__)
sys.path.insert(0, HERE)

from fresh_shapes import (  # noqa: E402
    PALETTE,
    blob,
    box,
    cone,
    disc,
    finish,
    fresh_mat,
    line,
    make_animal,
    make_cart,
    make_building,
    make_person,
    make_goose,
    make_wolf,
    make_prop,
    make_tree,
    make_tree_cluster,
    post,
    readable_eye,
    reset,
    wedge,
)
import models_lib  # noqa: F401, E402


ASSETS = (
    "monster_snake", "monster_frost_elf", "monster_sand_wurm", "monster_treant",
    "monster_aether_wraith", "cloud_puff", "rock_dark", "rock_mid", "rock_moss",
    "flower_0", "flower_1", "flower_2", "flower_3", "flower_4", "hill",
    "poi_pillar_red", "poi_pillar_cyan", "poi_pillar_pink", "poi_pillar_gold",
    "ground_disc_outer", "ground_disc_inner", "sokpop_gatherer", "fallen_stick",
    "sokpop_tree", "granular_round_tree", "granular_pine_tree", "forest_stone_spire",
    "granular_birch_tree", "granular_autumn_tree", "granular_willow_tree",
    "granular_wildflowers", "granular_reed_bank", "mushroom_red", "mushroom_brown",
    "crystal_blue", "crystal_pink", "treasure_chest", "boat", "arch_stone", "cart",
    "tombstone", "haystack", "palm", "resource_wood", "resource_stone", "cauldron",
    "cooking_station", "spit_roast", "campfire", "lantern_post", "crate", "barrel",
    "signpost", "market_stall", "bench", "fountain", "statue", "forge", "chapel", "pier",
    "tavern", "wolf", "bear", "villager", "ground_patch", "lake", "swamp", "beach",
    "mountain_snow", "volcano", "desert_dune", "cliff", "cave_entrance", "house_small",
    "watchtower", "windmill", "bridge_stone", "well", "barn", "fence", "shrine",
    "lighthouse", "hoplite_reaper_scythe", "hoplite_dragon_katana",
    "hoplite_golem_hammer", "hoplite_midas_sword", "hoplite_ender_dragon",
)


def make_flower(name: str, role: str):
    stem = fresh_mat(name, "grass")
    petal = fresh_mat(name, role)
    line("stem", (0, 0, 0.0), (0, 0, 0.38), 0.025, stem, 6)
    for index in range(5):
        angle = index * math.tau / 5.0
        blob(f"petal_{index}", (math.cos(angle) * 0.12, math.sin(angle) * 0.12, 0.42), (0.10, 0.07, 0.08), petal, 8, 4)
    blob("center", (0, 0, 0.44), (0.07, 0.06, 0.06), fresh_mat(name, "yellow"), 8, 4)


def make_mushroom(name: str):
    """Build a compact mushroom with a readable cap profile from every view.

    The old asset used a pointed four-sided wedge for the cap.  That made the
    silhouette read as a cone from the front and left the side view without a
    convincing overhanging rim.  A flattened low-poly dome plus a darker,
    overlapping underside keeps the cap organic while preserving the toy-like
    faceting and a small triangle budget.
    """
    red = name == "mushroom_red"
    cap_role = "coral" if red else "wood"
    cap_shadow_role = "cream"
    spot_role = "cream" if red else "yellow"
    stem_role = "cream"

    if red:
        cap_center = 0.61
        cap_depth = 0.24
        cap_radius = 0.44
        cap_top_radius = 0.28
        cap_top_center = 0.73
        cap_top_scale = (0.28, 0.28, 0.10)
        rim_center = 0.49
        rim_radius = 0.39
        rim_depth = 0.055
        stem_center = 0.36
        stem_depth = 0.34
        base_center = 0.16
        base_scale = (0.15, 0.15, 0.14)
        spot_size = 0.055
    else:
        cap_center = 0.46
        cap_depth = 0.18
        cap_radius = 0.35
        cap_top_radius = 0.22
        cap_top_center = 0.54
        cap_top_scale = (0.22, 0.22, 0.08)
        rim_center = 0.36
        rim_radius = 0.31
        rim_depth = 0.05
        stem_center = 0.31
        stem_depth = 0.28
        base_center = 0.14
        base_scale = (0.13, 0.13, 0.12)
        spot_size = 0.045

    cap = fresh_mat(name, cap_role)
    cap_shadow = fresh_mat(name, cap_shadow_role)
    stem = fresh_mat(name, stem_role)
    spots = fresh_mat(name, spot_role)

    # A faceted truncated cone establishes the overhanging silhouette.  The
    # small flattened dome overlaps its top to remove the hard flat plateau,
    # while the pale rim keeps the underside readable instead of becoming a
    # black floating band under the preview key light.
    cone("cap_profile", (0.0, 0.0, cap_center), cap_radius, cap_top_radius, cap_depth, cap, vertices=12)
    blob("cap_top", (0.0, 0.0, cap_top_center), cap_top_scale, cap, 12, 6)
    post("cap_under_rim", (0.0, 0.0, rim_center), rim_radius, rim_depth, cap_shadow, vertices=12)
    # A gently tapered stem reads as an organic support instead of a straight
    # pole, and its wider lower profile catches a little more of the key light.
    cone(
        "stem_profile",
        (0.0, 0.0, stem_center),
        0.14 if red else 0.125,
        0.105 if red else 0.095,
        stem_depth,
        stem,
        vertices=10,
    )
    blob("stem_base", (0.0, 0.0, base_center), base_scale, stem, 10, 6)

    # Place spots on the dome surface rather than as a floating crown.  Their
    # slight Z overlap makes the attachment stable from front, side, and top.
    spots_layout = ((0.00, 0.00), (-0.16, 0.05), (0.15, 0.08), (-0.08, -0.15), (0.18, -0.12))
    for index, (x, y) in enumerate(spots_layout if red else spots_layout[1:]):
        radial = math.sqrt(x * x + y * y)
        normalized = min(1.0, radial / max(cap_top_radius, 0.001))
        surface_z = cap_top_center + cap_top_scale[2] * math.sqrt(max(0.0, 1.0 - normalized * normalized))
        blob(
            f"cap_spot_{index}",
            (x, y, surface_z + spot_size * 0.02),
            (spot_size * 1.08, spot_size * 1.08, spot_size * 0.08),
            spots,
            8,
            4,
        )


def make_resource_wood(name: str):
    """Build the legacy wood resource as a reproducible fresh prop."""
    bark = fresh_mat(name, "wood")
    cut = fresh_mat(name, "cream")
    moss = fresh_mat(name, "leaf")
    logs = (
        (0.00, 0.00, 0.12, 0.10),
        (0.00, 0.16, 0.27, -0.08),
        (0.00, 0.32, 0.42, 0.05),
    )
    for index, (x, y, z, tilt) in enumerate(logs):
        rotation = (0.0, tilt, 0.0)
        box(f"log_{index}", (x, y, z), (0.96, 0.20, 0.18), bark, rotation=rotation, bevel=0.025)
        end_x = x + 0.49 * math.cos(tilt)
        end_z = z + 0.49 * math.sin(tilt)
        box(
            f"cut_{index}",
            (end_x, y, end_z),
            (0.035, 0.14, 0.13),
            cut,
            rotation=rotation,
            bevel=0.012,
        )
    box("moss_patch", (0.02, -0.115, 0.24), (0.22, 0.035, 0.08), moss, bevel=0.012)


def make_monster(name: str):
    body_role = {"monster_snake": "leaf", "monster_frost_elf": "water", "monster_sand_wurm": "yellow", "monster_treant": "wood", "monster_aether_wraith": "purple"}[name]
    body = fresh_mat(name, body_role)
    accent = fresh_mat(name, "coral")
    if name == "monster_snake":
        for i in range(6):
            blob(f"coil_{i}", ((i - 2.5) * 0.18, math.sin(i) * 0.08, 0.20 + abs(math.sin(i)) * 0.06), (0.20, 0.15, 0.18), body, 10, 5)
        blob("head", (0.48, -0.02, 0.35), (0.28, 0.21, 0.22), accent, 10, 5)
        readable_eye("eye", (0.60, -0.19, 0.43), accent, size=(0.045, 0.030, 0.045), socket_role="cream")
    elif name == "monster_sand_wurm":
        for i in range(4):
            blob(f"segment_{i}", ((i - 1.5) * 0.24, 0, 0.30), (0.28, 0.20, 0.20), body, 10, 5)
        wedge("jaw", (0.50, -0.04, 0.52), (0.30, 0.20, 0.28), accent)
        readable_eye("eye", (0.62, -0.19, 0.62), accent, size=(0.045, 0.030, 0.045), socket_role="cream")
    else:
        blob("body", (0, 0, 0.75), (0.40, 0.26, 0.50), body)
        blob("head", (0, -0.02, 1.40), (0.28, 0.23, 0.25), body)
        line("arm_l", (-0.34, 0, 0.85), (-0.55, 0, 0.45), 0.07, body)
        line("arm_r", (0.34, 0, 0.85), (0.55, 0, 0.45), 0.07, body)
        readable_eye("eye_l", (-0.08, -0.22, 1.45), accent, size=(0.050, 0.030, 0.050), socket_role="cream")
        readable_eye("eye_r", (0.08, -0.22, 1.45), accent, size=(0.050, 0.030, 0.050), socket_role="cream")


def make_terrain(name: str):
    if name == "mountain_snow":
        stone = fresh_mat(name, "stone")
        snow = fresh_mat(name, "snow")
        wedge("mountain_base", (0.0, 0.0, 1.55), (2.25, 1.70, 2.55), stone)
        wedge("mountain_ridge", (-0.42, -0.08, 2.50), (1.30, 1.08, 2.00), stone)
        wedge("snow_cap", (-0.48, -0.10, 3.32), (0.88, 0.76, 1.20), snow)
        line("snow_slope", (-0.72, -0.82, 2.85), (-0.35, -0.84, 3.54), 0.11, snow, 6)
    elif name == "volcano":
        stone = fresh_mat(name, "stone")
        lava = fresh_mat(name, "coral")
        wedge("volcano", (0.0, 0.0, 1.65), (2.05, 1.62, 2.25), stone)
        disc("crater", (0.0, 0.0, 3.12), 0.52, 0.08, lava, 1.0, 0.82)
        line("lava_flow", (0.18, -0.78, 2.88), (0.48, -0.86, 1.08), 0.10, lava, 7)
        blob("lava_glow", (0.0, -0.02, 3.20), (0.28, 0.22, 0.08), lava, 10, 4)
    elif name == "desert_dune":
        sand = fresh_mat(name, "yellow")
        blob("dune_front", (0.0, -0.15, 0.62), (2.20, 1.20, 0.64), sand)
        blob("dune_back", (-0.72, 0.28, 0.98), (1.35, 0.82, 0.58), fresh_mat(name, "cream"), 16, 9)
        line("dune_ridge", (-1.20, -0.92, 0.78), (0.95, -0.92, 1.02), 0.07, sand, 7)
    elif name == "cliff":
        stone = fresh_mat(name, "stone")
        box("cliff_face", (0.0, 0.0, 1.25), (3.0, 1.8, 2.50), stone, bevel=0.08)
        box("lower_ledge", (0.0, -0.70, 0.48), (3.18, 0.32, 0.24), stone, bevel=0.05)
        blob("cliff_grass", (0.0, -0.05, 2.58), (2.82, 1.62, 0.26), fresh_mat(name, "grass"))
        line("grass_edge", (-2.1, -0.88, 2.72), (1.85, -0.88, 2.72), 0.06, fresh_mat(name, "leaf_light"), 7)
    elif name == "cave_entrance":
        stone = fresh_mat(name, "stone")
        # Build the silhouette around an open mouth. The previous version
        # placed a full blob in front of the dark patch, so the imported asset
        # read as a giant gray boulder instead of an entrance.
        box("left_jamb", (-0.92, -0.20, 0.72), (0.55, 0.78, 1.25), stone, bevel=0.08)
        box("right_jamb", (0.92, -0.20, 0.72), (0.55, 0.78, 1.25), stone, bevel=0.08)
        box("arch_cap", (0.0, -0.10, 1.55), (2.25, 0.90, 0.62), stone, bevel=0.10)
        box("cave_dark", (0.0, -0.60, 0.78), (1.18, 0.06, 1.25), fresh_mat(name, "ink"), bevel=0.03)
        box("entrance_lip", (0.0, -0.68, 0.04), (1.35, 0.28, 0.16), stone, bevel=0.05)
        for index, x in enumerate((-0.68, 0.68)):
            wedge(f"cave_tooth_{index}", (x, -0.52, 1.00), (0.24, 0.38, 0.45), stone)
    elif name == "forest_stone_spire":
        stone = fresh_mat(name, "stone")
        moss = fresh_mat(name, "grass")
        # Keep the landmark as one continuous silhouette. Two separately
        # scaled cones looked like floating upper/lower shards after GLB
        # export because their tips and bases did not overlap in world space.
        wedge("spire_body", (0.0, 0.0, 3.5), (1.55, 1.15, 7.0), stone)
        blob("moss_patch", (-0.72, -0.74, 2.20), (0.46, 0.10, 0.24), moss, 10, 5)
    elif name in {"lake", "swamp", "beach", "ground_patch"}:
        disc("land", (0, 0, 0.05), 2.1, 0.12, fresh_mat(name, "grass"), 1.2, 0.75)
        role = "water" if name in {"lake", "swamp"} else "cream"
        disc("center", (0.05, -0.04, 0.13), 1.35, 0.08, fresh_mat(name, role), 1.25, 0.65)
        if name == "swamp":
            blob("mud", (0.45, 0.1, 0.20), (0.48, 0.28, 0.12), fresh_mat(name, "wood"))
            for index, x in enumerate((-0.65, 0.10, 0.72)):
                line(f"swamp_reed_{index}", (x, 0.0, 0.18), (x + 0.12, 0.0, 0.72 + index * 0.08), 0.025, fresh_mat(name, "grass"), 6)
        elif name == "lake":
            blob("island", (-0.55, 0.02, 0.20), (0.42, 0.30, 0.08), fresh_mat(name, "grass"), 10, 4)
        elif name == "beach":
            for index, x in enumerate((-0.62, 0.0, 0.64)):
                blob(f"beach_stone_{index}", (x, -0.20, 0.22), (0.16, 0.10, 0.09), fresh_mat(name, "stone"), 8, 5)
    else:
        disc("ground", (0, 0, 0.05), 2.2, 0.12, fresh_mat(name, "grass"), 1.2, 0.75)


def make_weapon(name: str):
    metal = fresh_mat(name, "stone")
    accent = fresh_mat(name, "coral" if "reaper" in name else "yellow")
    line("handle", (0, 0, 0), (0, 0, 1.1), 0.07, fresh_mat(name, "wood"), 7)
    wedge("blade", (0, 0, 1.42), (0.24, 0.08, 0.55), metal)
    blob("pommel", (0, 0, 0.02), (0.13, 0.10, 0.13), accent, 8, 4)


def make_boss():
    body = fresh_mat("dragon", "purple")
    accent = fresh_mat("dragon", "coral")
    eye = fresh_mat("dragon", "yellow")

    # Keep the semantic coordinates in the runtime convention: X lateral,
    # Y height, and -Z forward. Blender's Z-up source uses +Y for that same
    # forward direction; export_yup maps Blender +Y to glTF -Z.
    point = lambda x, forward, height: (x, -forward, height)

    blob("body", point(0.0, 0.0, 1.0), (0.60, 1.10, 0.60), body)
    blob("chest", point(0.0, -0.56, 1.05), (0.50, 0.55, 0.50), accent, 14, 8)
    blob("neck", point(0.0, -0.78, 1.30), (0.38, 0.40, 0.42), body, 14, 8)
    blob("head", point(0.0, -1.06, 1.58), (0.48, 0.42, 0.40), body)
    blob("muzzle", point(0.0, -1.40, 1.46), (0.30, 0.30, 0.20), accent, 10, 6)
    for side in (-1, 1):
        blob(
            f"dragon_eye_{side}",
            point(side * 0.31, -1.30, 1.70),
            (0.075, 0.055, 0.075),
            eye,
            8,
            5,
        )
        line(
            f"horn_{side}",
            point(side * 0.24, -1.00, 1.84),
            point(side * 0.32, -0.94, 2.16),
            0.085,
            accent,
            7,
        )
    line("jaw", point(0.0, -1.30, 1.34), point(0.0, -1.62, 1.34), 0.07, accent, 7)

    # Two overlapping cone sections make the tail taper and keep its root
    # visibly connected to the body.
    models_lib.cone_between(
        "tail_base", point(0.0, 0.62, 1.0), point(0.0, 1.35, 1.20), 0.24, body, vertices=8
    )
    models_lib.cone_between(
        "tail_tip", point(0.0, 1.30, 1.20), point(0.0, 2.05, 1.52), 0.16, body, vertices=7
    )
    blob("tail_spike", point(0.0, 2.06, 1.52), (0.18, 0.18, 0.18), accent, 8, 5)

    leg_layout = (
        ("front_left", -0.44, -0.34),
        ("front_right", 0.44, -0.34),
        ("hind_left", -0.52, 0.42),
        ("hind_right", 0.52, 0.42),
    )
    for label, side, forward in leg_layout:
        line(
            f"dragon_leg_{label}_upper",
            point(side, forward, 0.82),
            point(side * 1.04, forward, 0.45),
            0.12,
            body,
            8,
        )
        line(
            f"dragon_leg_{label}_lower",
            point(side * 1.04, forward, 0.45),
            point(side * 1.10, forward, 0.14),
            0.10,
            body,
            8,
        )
        blob(
            f"claw_{label}",
            point(side * 1.10, forward - 0.04, 0.12),
            (0.22, 0.16, 0.08),
            accent,
            8,
            5,
        )

    for side in (-1, 1):
        label = "left" if side < 0 else "right"
        blob(f"wing_root_{label}", point(side * 0.46, 0.0, 1.38), (0.32, 0.32, 0.30), body, 12, 7)
        wedge(
            f"dragon_wing_{label}_upper",
            point(side * 0.86, 0.05, 1.62),
            (0.72, 0.24, 0.56),
            accent,
            side * 0.25,
        )
        wedge(
            f"dragon_wing_{label}_lower",
            point(side * 1.42, 0.05, 1.84),
            (0.58, 0.18, 0.42),
            accent,
            side * 0.18,
        )
        line(
            f"wing_edge_{label}",
            point(side * 0.50, 0.04, 1.48),
            point(side * 1.55, 0.06, 1.98),
            0.055,
            body,
            7,
        )


def make_asset(name: str):
    reset()
    if name == "sokpop_gatherer":
        make_goose(name)
    elif name == "villager":
        make_person(name, "coral")
    elif name == "cart":
        make_cart(name)
    elif name == "wolf":
        make_wolf(name)
    elif name == "bear":
        make_animal(name, "wood")
    elif name in {"sokpop_tree", "granular_round_tree", "granular_pine_tree", "granular_birch_tree", "granular_willow_tree", "granular_autumn_tree", "palm"}:
        make_tree(name, "leaf_light" if "autumn" not in name else "coral")
    elif name in {"granular_wildflowers", "granular_reed_bank"}:
        make_tree_cluster(name, ("grass", "leaf_light", "yellow"))
    elif name.startswith("monster_"):
        make_monster(name)
    elif name.startswith("flower_"):
        make_flower(name, ("coral", "yellow", "purple", "peach", "coral")[int(name[-1])])
    elif name in {"mushroom_red", "mushroom_brown"}:
        make_mushroom(name)
    elif name == "resource_wood":
        make_resource_wood(name)
    elif name.startswith("rock_") or name in {"resource_stone", "tombstone"}:
        make_prop(name, "tombstone" if name == "tombstone" else "rock")
    elif name in {"ground_disc_outer", "ground_disc_inner", "hill", "mountain_snow", "volcano", "desert_dune", "cliff", "cave_entrance", "forest_stone_spire", "lake", "swamp", "beach", "ground_patch"}:
        make_terrain(name)
    elif name in {"house_small", "barn", "chapel", "tavern"}:
        from fresh_shapes import make_medieval_house
        make_medieval_house(name)
    elif name in {"watchtower", "windmill", "lighthouse", "shrine", "well", "forge", "bridge_stone", "pier", "fence"}:
        make_building(name, "stone", "wood_light")
    elif name in {"sword", "hoplite_reaper_scythe", "hoplite_dragon_katana", "hoplite_golem_hammer", "hoplite_midas_sword"}:
        make_weapon(name)
    elif name == "hoplite_ender_dragon":
        make_boss()
    elif name == "cloud_puff":
        cloud = fresh_mat(name, "snow")
        cloud_shadow = fresh_mat(name, "stone")
        # A readable airborne cloud, not a gray three-ball placeholder.
        blob("cloud_base", (0.0, 0.0, 0.0), (1.45, 0.72, 0.48), cloud_shadow)
        for i, (x, z, sx) in enumerate(((-0.86, 0.15, 0.76), (0.0, 0.34, 0.98), (0.86, 0.13, 0.72))):
            blob(f"cloud_puff_{i}", (x, 0.0, z), (sx, 0.70, 0.56), cloud)
    elif name.startswith("poi_pillar_"):
        post("pillar", (0, 0, 1.3), 0.24, 2.6, fresh_mat(name, "stone"), 8)
        blob("signal", (0, 0, 2.7), (0.30, 0.25, 0.30), fresh_mat(name, "coral"), 8, 5)
    elif name in {"crystal_blue", "crystal_pink"}:
        stem = fresh_mat(name, "cream")
        role = "coral" if "pink" in name else "water"
        post("stem", (0, 0, 0.45), 0.14, 0.90, stem, 8)
        wedge("cap", (0, 0, 0.95), (0.50, 0.40, 0.35), fresh_mat(name, role))
    elif name in {"treasure_chest", "crate", "barrel"}:
        make_prop(name, "chest" if name == "treasure_chest" else "crate")
    elif name == "signpost":
        make_prop(name, "sign")
    elif name in {
        "campfire", "fountain", "cauldron", "cooking_station", "spit_roast",
        "statue", "market_stall", "bench", "lantern_post", "boat", "arch_stone",
        "haystack",
    }:
        make_prop(name, name)
    else:
        make_prop(name, "generic")
    finish(name, "pretty")


def main():
    for name in ASSETS:
        make_asset(name)


if __name__ == "__main__":
    main()
