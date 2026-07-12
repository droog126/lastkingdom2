from __future__ import annotations

import argparse
import math
import random
import sys
from pathlib import Path

import bpy

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from model_style import TEMP_PREVIEW_DIR
from models_lib import OUT_DIR, clear_scene, export_glb, mat, shade_flat


ROOT = Path(__file__).resolve().parents[1]
PREVIEW_PATH = TEMP_PREVIEW_DIR / "granular_flora.png"


def make_mat(name: str, color, roughness: float = 0.94, alpha: float = 1.0):
    return mat(name, color, roughness=roughness, alpha=alpha)


def block(name: str, loc, scale, material, rot=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc, rotation=rot)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    obj.data.materials.append(material)
    shade_flat(obj)
    return obj


def cylinder(name: str, loc, radius: float, depth: float, material, vertices: int = 6, rot=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=depth,
        location=loc,
        rotation=rot,
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(material)
    shade_flat(obj)
    return obj


def flower_disc(name: str, loc, radius: float, material, vertices: int = 6):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=0.018,
        location=loc,
        rotation=(math.radians(90.0), 0.0, 0.0),
    )
    obj = bpy.context.object
    obj.name = name
    obj.scale.y = 0.55
    obj.data.materials.append(material)
    shade_flat(obj)
    return obj


def scatter_leaf_blocks(
    rng: random.Random,
    prefix: str,
    center,
    count: int,
    radius,
    materials,
    size_range=(0.10, 0.18),
    vertical_bias: float = 0.0,
):
    cx, cy, cz = center
    rx, ry, rz = radius
    for i in range(count):
        theta = rng.random() * math.tau
        r = math.sqrt(rng.random())
        x = cx + math.cos(theta) * rx * r
        y = cy + math.sin(theta) * ry * r
        z = cz + (rng.random() * 2.0 - 1.0) * rz + vertical_bias * (1.0 - r)
        sx = rng.uniform(*size_range)
        sy = rng.uniform(size_range[0] * 0.45, size_range[1] * 0.75)
        sz = rng.uniform(size_range[0] * 0.75, size_range[1] * 1.15)
        block(
            f"{prefix}_leaf_{i:02d}",
            (x, y, z),
            (sx, sy, sz),
            materials[i % len(materials)],
            rot=(rng.uniform(-0.18, 0.18), rng.uniform(-0.12, 0.12), theta),
        )


def make_granular_round_tree() -> None:
    clear_scene()
    rng = random.Random(1101)
    bark = make_mat("granular_round_bark", (0.34, 0.22, 0.13))
    bark_light = make_mat("granular_round_bark_light", (0.47, 0.31, 0.17))
    leaf_a = make_mat("granular_round_leaf_a", (0.17, 0.38, 0.20))
    leaf_b = make_mat("granular_round_leaf_b", (0.27, 0.54, 0.27))
    leaf_c = make_mat("granular_round_leaf_c", (0.10, 0.27, 0.16))
    leaf_tip = make_mat("granular_round_leaf_tip", (0.36, 0.64, 0.30))

    cylinder("trunk_low", (0.0, 0.0, 0.62), 0.16, 1.24, bark, vertices=7)
    cylinder("trunk_high", (0.03, -0.01, 1.45), 0.12, 0.82, bark_light, vertices=7)
    for i, angle in enumerate((-0.65, 0.35, 1.25)):
        cylinder(
            f"branch_{i}",
            (math.cos(angle) * 0.20, math.sin(angle) * 0.20, 1.45 + i * 0.15),
            0.045,
            0.70,
            bark,
            vertices=5,
            rot=(math.radians(64.0), 0.0, angle),
        )

    scatter_leaf_blocks(rng, "round_core", (0.0, 0.0, 2.22), 68, (0.78, 0.60, 0.55), [leaf_a, leaf_b, leaf_c], (0.11, 0.20), 0.16)
    scatter_leaf_blocks(rng, "round_left", (-0.42, 0.06, 2.04), 26, (0.42, 0.34, 0.30), [leaf_c, leaf_a], (0.09, 0.16), 0.08)
    scatter_leaf_blocks(rng, "round_right", (0.44, -0.05, 2.12), 28, (0.44, 0.36, 0.34), [leaf_b, leaf_a], (0.09, 0.17), 0.10)
    scatter_leaf_blocks(rng, "round_top", (0.06, 0.02, 2.62), 20, (0.42, 0.35, 0.26), [leaf_tip, leaf_b], (0.08, 0.15), 0.12)
    export_glb("granular_round_tree")


def make_granular_pine_tree() -> None:
    clear_scene()
    rng = random.Random(1207)
    bark = make_mat("granular_pine_bark", (0.30, 0.19, 0.11))
    leaf_dark = make_mat("granular_pine_leaf_dark", (0.08, 0.22, 0.16))
    leaf_mid = make_mat("granular_pine_leaf_mid", (0.14, 0.34, 0.22))
    leaf_light = make_mat("granular_pine_leaf_light", (0.22, 0.46, 0.27))

    cylinder("trunk", (0.0, 0.0, 1.10), 0.12, 2.20, bark, vertices=7)
    levels = [
        (0.0, 0.0, 0.88, 0.92, 42),
        (0.0, 0.0, 1.32, 0.78, 34),
        (0.0, 0.0, 1.76, 0.60, 28),
        (0.0, 0.0, 2.16, 0.42, 20),
        (0.0, 0.0, 2.48, 0.26, 12),
    ]
    for level, (cx, cy, cz, radius, count) in enumerate(levels):
        for i in range(count):
            theta = rng.random() * math.tau
            dist = radius * math.sqrt(rng.random())
            x = cx + math.cos(theta) * dist
            y = cy + math.sin(theta) * dist
            z = cz + rng.uniform(-0.08, 0.08) - dist * 0.16
            length = rng.uniform(0.14, 0.25) * (1.05 - level * 0.08)
            block(
                f"pine_needles_{level}_{i:02d}",
                (x, y, z),
                (length, 0.050, rng.uniform(0.08, 0.14)),
                [leaf_dark, leaf_mid, leaf_light][(i + level) % 3],
                rot=(rng.uniform(-0.25, 0.25), rng.uniform(-0.18, 0.18), theta),
            )
    export_glb("granular_pine_tree")


def make_forest_stone_spire() -> None:
    clear_scene()
    stone = make_mat("forest_spire_stone", (0.82, 0.84, 0.80), roughness=0.88)
    stone_light = make_mat("forest_spire_stone_light", (0.94, 0.95, 0.91), roughness=0.84)
    stone_shadow = make_mat("forest_spire_stone_shadow", (0.58, 0.62, 0.60), roughness=0.92)
    moss = make_mat("forest_spire_moss", (0.35, 0.48, 0.28), roughness=0.98)

    # A readable stepped monolith: broad broken foot, tapered shaft, and a
    # narrow cap that remains legible between tall trees.
    block("spire_foot", (0.0, 0.0, 0.28), (2.20, 1.72, 0.56), stone_shadow, rot=(0.0, 0.0, -0.08))
    block("spire_foot_slab", (0.10, -0.04, 0.62), (1.72, 1.34, 0.36), stone, rot=(0.0, 0.0, 0.05))
    block("spire_lower_shoulder", (-0.04, 0.02, 1.18), (1.25, 1.02, 0.74), stone_light, rot=(0.0, 0.0, -0.06))
    block("spire_lower_shaft", (0.02, 0.02, 2.55), (0.86, 0.74, 2.15), stone, rot=(0.0, 0.0, 0.025))
    block("spire_upper_shaft", (-0.04, 0.02, 4.38), (0.60, 0.56, 1.55), stone_light, rot=(0.0, 0.0, -0.035))
    block("spire_cap", (0.0, 0.0, 5.78), (0.38, 0.36, 1.25), stone, rot=(0.0, 0.0, 0.02))
    block("spire_broken_left", (-0.86, 0.12, 0.92), (0.54, 0.46, 0.48), stone, rot=(0.07, -0.12, -0.24))
    block("spire_broken_right", (0.82, -0.10, 0.78), (0.46, 0.40, 0.36), stone_light, rot=(-0.05, 0.10, 0.20))
    block("spire_moss_patch", (-0.72, -0.88, 0.72), (0.42, 0.035, 0.20), moss, rot=(0.0, 0.0, -0.15))
    block("spire_shadow_patch", (0.30, -0.76, 2.08), (0.22, 0.035, 0.68), stone_shadow, rot=(0.02, 0.0, 0.06))
    export_glb("forest_stone_spire")


def make_granular_birch_tree() -> None:
    clear_scene()
    rng = random.Random(1309)
    bark = make_mat("granular_birch_bark", (0.78, 0.73, 0.58))
    bark_dark = make_mat("granular_birch_bark_dark", (0.18, 0.15, 0.11))
    leaf_a = make_mat("granular_birch_leaf_a", (0.30, 0.58, 0.22))
    leaf_b = make_mat("granular_birch_leaf_b", (0.48, 0.72, 0.25))
    leaf_c = make_mat("granular_birch_leaf_c", (0.16, 0.39, 0.19))

    cylinder("trunk_low", (0.0, 0.0, 0.68), 0.14, 1.36, bark, vertices=7)
    cylinder("trunk_high", (0.02, 0.0, 1.55), 0.10, 0.92, bark, vertices=7)
    for index, (x, z, width) in enumerate(((-0.01, 0.44, 0.11), (0.02, 0.92, 0.08), (-0.01, 1.38, 0.10))):
        block(f"bark_mark_{index}", (x, -0.145, z), (width, 0.022, 0.035), bark_dark)
    for index, angle in enumerate((-0.75, 0.25, 1.15, 2.5)):
        cylinder(
            f"branch_{index}",
            (math.cos(angle) * 0.18, math.sin(angle) * 0.18, 1.52 + index * 0.12),
            0.040,
            0.62,
            bark,
            vertices=5,
            rot=(math.radians(64.0), 0.0, angle),
        )

    scatter_leaf_blocks(rng, "birch_core", (0.0, 0.0, 2.22), 54, (0.68, 0.52, 0.48), [leaf_a, leaf_b, leaf_c], (0.10, 0.18), 0.10)
    scatter_leaf_blocks(rng, "birch_left", (-0.38, 0.04, 2.04), 20, (0.34, 0.28, 0.28), [leaf_b, leaf_a], (0.08, 0.14), 0.06)
    scatter_leaf_blocks(rng, "birch_right", (0.40, -0.02, 2.10), 22, (0.36, 0.30, 0.30), [leaf_a, leaf_c], (0.08, 0.15), 0.08)
    export_glb("granular_birch_tree")


def make_granular_autumn_tree() -> None:
    clear_scene()
    rng = random.Random(1411)
    bark = make_mat("granular_autumn_bark", (0.36, 0.19, 0.09))
    bark_light = make_mat("granular_autumn_bark_light", (0.54, 0.29, 0.11))
    leaf_red = make_mat("granular_autumn_leaf_red", (0.72, 0.18, 0.08))
    leaf_orange = make_mat("granular_autumn_leaf_orange", (0.95, 0.39, 0.08))
    leaf_gold = make_mat("granular_autumn_leaf_gold", (0.96, 0.68, 0.12))
    leaf_brown = make_mat("granular_autumn_leaf_brown", (0.48, 0.25, 0.08))

    cylinder("trunk_low", (0.0, 0.0, 0.66), 0.17, 1.32, bark, vertices=7)
    cylinder("trunk_high", (-0.02, 0.0, 1.46), 0.12, 0.72, bark_light, vertices=7)
    for index, angle in enumerate((-0.85, 0.10, 0.95, 2.35)):
        cylinder(
            f"branch_{index}",
            (math.cos(angle) * 0.22, math.sin(angle) * 0.22, 1.42 + index * 0.13),
            0.050,
            0.72,
            bark,
            vertices=6,
            rot=(math.radians(62.0), 0.0, angle),
        )

    scatter_leaf_blocks(rng, "autumn_core", (0.0, 0.0, 2.18), 66, (0.82, 0.62, 0.54), [leaf_orange, leaf_gold, leaf_red, leaf_brown], (0.10, 0.19), 0.14)
    scatter_leaf_blocks(rng, "autumn_left", (-0.48, 0.05, 2.02), 24, (0.42, 0.34, 0.32), [leaf_red, leaf_orange, leaf_brown], (0.09, 0.16), 0.06)
    scatter_leaf_blocks(rng, "autumn_right", (0.48, -0.05, 2.08), 25, (0.44, 0.36, 0.34), [leaf_gold, leaf_orange, leaf_red], (0.09, 0.17), 0.08)
    scatter_leaf_blocks(rng, "autumn_top", (0.05, 0.02, 2.60), 18, (0.40, 0.34, 0.26), [leaf_gold, leaf_orange], (0.08, 0.15), 0.10)
    export_glb("granular_autumn_tree")


def make_granular_willow_tree() -> None:
    clear_scene()
    rng = random.Random(1523)
    bark = make_mat("granular_willow_bark", (0.35, 0.25, 0.14))
    bark_light = make_mat("granular_willow_bark_light", (0.55, 0.39, 0.19))
    leaf_a = make_mat("granular_willow_leaf_a", (0.24, 0.54, 0.22))
    leaf_b = make_mat("granular_willow_leaf_b", (0.46, 0.70, 0.25))
    leaf_c = make_mat("granular_willow_leaf_c", (0.13, 0.35, 0.18))

    cylinder("trunk_low", (0.0, 0.0, 0.58), 0.20, 1.16, bark, vertices=8)
    cylinder("trunk_high", (0.0, 0.0, 1.42), 0.14, 0.84, bark_light, vertices=7)
    for index, angle in enumerate((-1.00, -0.35, 0.35, 1.00, 2.40, 3.00)):
        cylinder(
            f"branch_{index}",
            (math.cos(angle) * 0.24, math.sin(angle) * 0.24, 1.55),
            0.042,
            0.78,
            bark,
            vertices=5,
            rot=(math.radians(70.0), 0.0, angle),
        )

    scatter_leaf_blocks(rng, "willow_core", (0.0, 0.0, 2.20), 70, (0.68, 0.52, 0.34), [leaf_a, leaf_b, leaf_c], (0.09, 0.17), 0.08)
    scatter_leaf_blocks(rng, "willow_left", (-0.62, 0.0, 1.96), 46, (0.30, 0.25, 0.62), [leaf_a, leaf_b], (0.08, 0.16), -0.12)
    scatter_leaf_blocks(rng, "willow_right", (0.62, 0.02, 1.98), 46, (0.30, 0.25, 0.64), [leaf_b, leaf_a, leaf_c], (0.08, 0.16), -0.14)
    scatter_leaf_blocks(rng, "willow_front", (0.0, -0.42, 1.98), 34, (0.36, 0.20, 0.52), [leaf_a, leaf_c], (0.08, 0.15), -0.10)
    export_glb("granular_willow_tree")


def make_granular_wildflowers() -> None:
    clear_scene()
    rng = random.Random(2219)
    grass_a = make_mat("granular_flower_grass_a", (0.29, 0.49, 0.27))
    grass_b = make_mat("granular_flower_grass_b", (0.17, 0.34, 0.20))
    stem = make_mat("granular_flower_stem", (0.38, 0.57, 0.27))
    yellow = make_mat("granular_flower_yellow", (0.94, 0.75, 0.26))
    white = make_mat("granular_flower_white", (0.86, 0.86, 0.70))
    peach = make_mat("granular_flower_peach", (0.88, 0.42, 0.30))
    violet = make_mat("granular_flower_violet", (0.54, 0.42, 0.82))
    dirt = make_mat("granular_flower_soil", (0.33, 0.26, 0.16))

    block("soft_soil_patch", (0.0, 0.0, 0.02), (1.35, 0.82, 0.045), dirt, rot=(0.0, 0.0, 0.10))
    for i in range(78):
        theta = rng.random() * math.tau
        radius = math.sqrt(rng.random())
        x = math.cos(theta) * 1.16 * radius
        y = math.sin(theta) * 0.70 * radius
        h = rng.uniform(0.18, 0.54)
        block(
            f"grass_blade_{i:02d}",
            (x, y, h * 0.5 + 0.04),
            (rng.uniform(0.020, 0.035), rng.uniform(0.018, 0.032), h),
            grass_a if i % 3 else grass_b,
            rot=(rng.uniform(-0.16, 0.16), rng.uniform(-0.12, 0.12), theta),
        )
    flower_mats = [yellow, white, peach, violet]
    for i in range(22):
        theta = rng.random() * math.tau
        radius = math.sqrt(rng.random())
        x = math.cos(theta) * 1.05 * radius
        y = math.sin(theta) * 0.62 * radius
        h = rng.uniform(0.28, 0.62)
        block(f"flower_stem_{i:02d}", (x, y, h * 0.5 + 0.05), (0.018, 0.018, h), stem, rot=(0.0, 0.0, rng.uniform(-0.3, 0.3)))
        flower_disc(f"flower_head_{i:02d}", (x, y, h + 0.08), rng.uniform(0.035, 0.065), flower_mats[i % len(flower_mats)], vertices=5)
    export_glb("granular_wildflowers")


def make_granular_reed_bank() -> None:
    clear_scene()
    rng = random.Random(3313)
    mud = make_mat("granular_reed_mud", (0.28, 0.24, 0.15))
    grass = make_mat("granular_reed_grass", (0.20, 0.39, 0.25))
    reed = make_mat("granular_reed_stalk", (0.48, 0.43, 0.23))
    reed_tip = make_mat("granular_reed_tip", (0.58, 0.40, 0.23))
    water = make_mat("granular_reed_water", (0.12, 0.30, 0.32), roughness=0.72, alpha=0.70)

    block("mud_bank", (0.0, -0.05, 0.02), (1.45, 0.42, 0.05), mud, rot=(0.0, 0.0, -0.08))
    block("shallow_water_edge", (0.0, 0.36, 0.055), (1.52, 0.33, 0.028), water, rot=(0.0, 0.0, 0.02))
    for i in range(66):
        x = rng.uniform(-1.22, 1.22)
        y = rng.uniform(-0.38, 0.24)
        h = rng.uniform(0.28, 0.82)
        material = reed if i % 4 else grass
        block(
            f"reed_stalk_{i:02d}",
            (x, y, h * 0.5 + 0.05),
            (rng.uniform(0.018, 0.032), rng.uniform(0.018, 0.032), h),
            material,
            rot=(rng.uniform(-0.12, 0.12), rng.uniform(-0.10, 0.10), rng.uniform(-0.18, 0.18)),
        )
        if i % 3 == 0:
            block(
                f"reed_seed_{i:02d}",
                (x, y, h + 0.10),
                (0.038, 0.038, 0.12),
                reed_tip,
                rot=(rng.uniform(-0.08, 0.08), rng.uniform(-0.06, 0.06), rng.random() * math.tau),
            )
    export_glb("granular_reed_bank")


def add_preview_lights() -> None:
    bpy.ops.object.light_add(type="SUN", location=(0.0, -4.0, 5.0), rotation=(math.radians(45.0), 0.0, math.radians(35.0)))
    sun = bpy.context.object
    sun.name = "preview_sun"
    sun.data.energy = 1.8
    bpy.ops.object.light_add(type="AREA", location=(0.0, -3.2, 3.0))
    area = bpy.context.object
    area.name = "preview_softbox"
    area.data.energy = 180.0
    area.data.size = 5.5


def render_preview(path: Path = PREVIEW_PATH) -> Path:
    clear_scene()
    ground = make_mat("preview_ground", (0.28, 0.40, 0.25))
    block("preview_ground", (0.0, 0.0, -0.035), (5.2, 3.0, 0.04), ground)

    placements = [
        ("granular_round_tree.glb", (-1.75, 0.45, 0.0)),
        ("granular_pine_tree.glb", (0.85, 0.40, 0.0)),
        ("forest_stone_spire.glb", (0.05, 1.35, 0.0)),
        ("granular_wildflowers.glb", (-0.55, -1.15, 0.0)),
        ("granular_reed_bank.glb", (1.55, -1.05, 0.0)),
    ]
    for filename, offset in placements:
        before = set(bpy.context.scene.objects)
        bpy.ops.import_scene.gltf(filepath=str(OUT_DIR / filename))
        imported = [obj for obj in bpy.context.scene.objects if obj not in before]
        for obj in imported:
            obj.location.x += offset[0]
            obj.location.y += offset[1]
            obj.location.z += offset[2]

    add_preview_lights()
    bpy.ops.object.camera_add(location=(0.0, -7.1, 3.15), rotation=(math.radians(64.0), 0.0, 0.0))
    bpy.context.scene.camera = bpy.context.object
    bpy.context.scene.render.engine = "BLENDER_EEVEE_NEXT" if "BLENDER_EEVEE_NEXT" in {item.identifier for item in bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items} else "BLENDER_EEVEE"
    bpy.context.scene.render.resolution_x = 1400
    bpy.context.scene.render.resolution_y = 900
    bpy.context.scene.view_settings.view_transform = "Filmic"
    bpy.context.scene.view_settings.look = "Medium High Contrast"
    bpy.context.scene.view_settings.exposure = -0.35
    path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    print(f"preview -> {path}")
    return path


def build_assets() -> None:
    make_granular_round_tree()
    make_granular_pine_tree()
    make_forest_stone_spire()
    make_granular_birch_tree()
    make_granular_autumn_tree()
    make_granular_willow_tree()
    make_granular_wildflowers()
    make_granular_reed_bank()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--preview", action="store_true", help="also render a contact-sheet style preview png")
    parser.add_argument("--preview-path", default=str(PREVIEW_PATH))
    script_args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    args = parser.parse_args(script_args)

    build_assets()
    if args.preview:
        render_preview(Path(args.preview_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
