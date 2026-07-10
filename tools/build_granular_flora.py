from __future__ import annotations

import argparse
import math
import random
import sys
from pathlib import Path

import bpy

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from models_lib import OUT_DIR, clear_scene, export_glb, mat, shade_flat


ROOT = Path(__file__).resolve().parents[1]
PREVIEW_PATH = Path("D:/Temp/granular_flora_preview.png")


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
