from __future__ import annotations

import argparse
import math
import random
import sys
from pathlib import Path

import bpy
from mathutils import Vector

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from models_lib import OUT_DIR, clear_scene, export_glb, mat, shade_flat


PREVIEW_PATH = Path("D:/Temp/hoplite_legendaries_preview.png")


def material(name: str, color, *, metallic=0.0, roughness=0.72, glow=None):
    return mat(
        name,
        color,
        metallic=metallic,
        roughness=roughness,
        emissive=glow or (0.0, 0.0, 0.0),
    )


def bevel_box(name: str, loc, scale, material_slot, *, bevel=0.025, rotation=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc, rotation=rotation)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel > 0.0:
        modifier = obj.modifiers.new("small_cut_bevel", "BEVEL")
        modifier.width = bevel
        modifier.segments = 1
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    obj.data.materials.append(material_slot)
    shade_flat(obj)
    return obj


def prism(name: str, points, depth: float, material_slot, *, y=0.0, bevel=0.018):
    half = depth * 0.5
    vertices = [(x, y - half, z) for x, z in points] + [(x, y + half, z) for x, z in points]
    count = len(points)
    faces = [tuple(range(count)), tuple(range(count, count * 2))[::-1]]
    for index in range(count):
        nxt = (index + 1) % count
        faces.append((index, nxt, count + nxt, count + index))
    mesh = bpy.data.meshes.new(f"{name}_mesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(material_slot)
    if bevel > 0.0:
        modifier = obj.modifiers.new("edge_soften", "BEVEL")
        modifier.width = bevel
        modifier.segments = 1
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)
        bpy.ops.object.modifier_apply(modifier=modifier.name)
        obj.select_set(False)
    shade_flat(obj)
    return obj


def cylinder_between(name: str, start, end, radius: float, material_slot, *, vertices=8):
    start_vec = Vector(start)
    end_vec = Vector(end)
    direction = end_vec - start_vec
    midpoint = (start_vec + end_vec) * 0.5
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=direction.length,
        location=midpoint,
    )
    obj = bpy.context.object
    obj.name = name
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = Vector((0.0, 0.0, 1.0)).rotation_difference(direction.normalized())
    obj.data.materials.append(material_slot)
    shade_flat(obj)
    return obj


def energy_motes(rng: random.Random, prefix: str, center, radius, count: int, mats):
    cx, cy, cz = center
    rx, ry, rz = radius
    for index in range(count):
        angle = rng.random() * math.tau
        distance = rng.uniform(0.55, 1.0)
        size = rng.uniform(0.025, 0.055)
        bevel_box(
            f"{prefix}_mote_{index:02d}",
            (
                cx + math.cos(angle) * rx * distance,
                cy + rng.uniform(-ry, ry),
                cz + math.sin(angle) * rz * distance,
            ),
            (size, size * rng.uniform(0.55, 0.9), size * rng.uniform(0.65, 1.4)),
            mats[index % len(mats)],
            bevel=size * 0.18,
            rotation=(rng.uniform(-0.4, 0.4), rng.uniform(-0.4, 0.4), rng.uniform(-0.8, 0.8)),
        )


def make_reaper_scythe() -> None:
    clear_scene()
    rng = random.Random(4501)
    black = material("reaper_black_iron", (0.035, 0.028, 0.045), metallic=0.72, roughness=0.28)
    steel = material("reaper_bone_steel", (0.72, 0.65, 0.72), metallic=0.62, roughness=0.32)
    violet = material("reaper_violet", (0.35, 0.055, 0.45), metallic=0.35, roughness=0.30, glow=(0.20, 0.01, 0.34))
    magenta = material("reaper_soul_glow", (0.72, 0.08, 0.80), roughness=0.24, glow=(0.62, 0.02, 0.75))

    handle_start = (-0.34, 0.0, 0.10)
    handle_end = (0.13, 0.0, 2.12)
    cylinder_between("reaper_handle", handle_start, handle_end, 0.075, black, vertices=8)
    for index, t in enumerate((0.08, 0.27, 0.47, 0.68, 0.86)):
        point = Vector(handle_start).lerp(Vector(handle_end), t)
        bevel_box(
            f"reaper_grip_wrap_{index}",
            point,
            (0.13, 0.12, 0.055),
            violet if index % 2 else steel,
            bevel=0.02,
            rotation=(0.0, math.radians(-13.0), math.radians(-13.0)),
        )

    bevel_box("reaper_head_core", (0.16, 0.0, 2.20), (0.28, 0.16, 0.28), black, bevel=0.045, rotation=(0.0, 0.0, -0.12))
    bevel_box("reaper_head_gem", (0.16, -0.175, 2.20), (0.13, 0.026, 0.13), magenta, bevel=0.026)
    bevel_box("reaper_counterweight", (0.47, 0.0, 2.08), (0.25, 0.11, 0.10), violet, bevel=0.025, rotation=(0.0, 0.0, -0.22))

    outer_blade = [
        (0.13, 2.32), (-0.10, 2.45), (-0.48, 2.66), (-0.92, 2.83),
        (-1.34, 2.84), (-1.55, 2.70), (-1.45, 2.50), (-1.18, 2.60),
        (-0.80, 2.58), (-0.42, 2.42), (-0.08, 2.22),
    ]
    inner_blade = [
        (-0.04, 2.39), (-0.42, 2.60), (-0.84, 2.75), (-1.23, 2.76),
        (-1.40, 2.68), (-1.37, 2.59), (-1.14, 2.66), (-0.79, 2.65),
        (-0.45, 2.52), (-0.14, 2.34),
    ]
    prism("reaper_outer_blade", outer_blade, 0.14, black, y=0.0, bevel=0.024)
    prism("reaper_inner_blade", inner_blade, 0.045, magenta, y=-0.095, bevel=0.012)
    prism("reaper_bone_edge", [(-1.49, 2.70), (-1.34, 2.84), (-0.92, 2.83), (-0.98, 2.76), (-1.29, 2.75)], 0.17, steel, bevel=0.014)
    energy_motes(rng, "reaper", (-0.55, -0.02, 2.42), (1.16, 0.20, 0.58), 18, [violet, magenta])
    export_glb("hoplite_reaper_scythe")


def make_dragon_katana() -> None:
    clear_scene()
    rng = random.Random(4502)
    black = material("katana_obsidian", (0.025, 0.022, 0.040), metallic=0.58, roughness=0.25)
    silver = material("katana_edge", (0.82, 0.77, 0.90), metallic=0.82, roughness=0.20)
    purple = material("katana_dragon_energy", (0.36, 0.02, 0.62), metallic=0.38, roughness=0.22, glow=(0.31, 0.01, 0.66))
    pink = material("katana_hot_edge", (0.92, 0.10, 0.68), roughness=0.18, glow=(0.88, 0.02, 0.58))
    green = material("katana_dragon_eye", (0.26, 0.92, 0.20), roughness=0.20, glow=(0.18, 0.80, 0.08))

    cylinder_between("katana_handle", (0.0, 0.0, 0.05), (0.0, 0.0, 0.84), 0.085, black, vertices=8)
    for index, z in enumerate((0.14, 0.30, 0.46, 0.62, 0.78)):
        bevel_box(f"katana_wrap_{index}", (0.0, -0.075, z), (0.105, 0.035, 0.045), purple if index % 2 else silver, bevel=0.016, rotation=(0.0, 0.0, (-1) ** index * 0.28))
    bevel_box("katana_pommel", (0.0, 0.0, 0.04), (0.12, 0.12, 0.10), silver, bevel=0.025)
    bevel_box("katana_guard", (0.0, 0.0, 0.91), (0.72, 0.16, 0.10), black, bevel=0.035)
    bevel_box("katana_guard_left_tip", (-0.38, 0.0, 0.94), (0.22, 0.16, 0.10), silver, bevel=0.025, rotation=(0.0, 0.0, math.radians(20.0)))
    bevel_box("katana_guard_right_tip", (0.38, 0.0, 0.94), (0.22, 0.16, 0.10), silver, bevel=0.025, rotation=(0.0, 0.0, math.radians(-20.0)))
    bevel_box("katana_eye_frame", (0.0, -0.095, 0.91), (0.20, 0.035, 0.20), black, bevel=0.022, rotation=(0.0, 0.0, math.radians(45.0)))
    prism("katana_dragon_eye", [(-0.08, 0.91), (0.0, 1.00), (0.08, 0.91), (0.0, 0.82)], 0.035, green, y=-0.125, bevel=0.008)
    prism("katana_eye_slit", [(-0.018, 0.96), (0.018, 0.96), (0.018, 0.86), (-0.018, 0.86)], 0.045, black, y=-0.15, bevel=0.004,)

    outer = [(-0.18, 1.00), (0.19, 1.00), (0.25, 2.82), (0.04, 3.22), (-0.19, 2.88)]
    core = [(-0.105, 1.07), (0.09, 1.07), (0.14, 2.78), (0.035, 3.03), (-0.11, 2.82)]
    edge = [(-0.19, 1.00), (-0.11, 1.07), (-0.11, 2.82), (0.035, 3.03), (0.04, 3.22), (-0.19, 2.88)]
    prism("katana_outer_blade", outer, 0.13, black, bevel=0.022)
    prism("katana_purple_core", core, 0.045, purple, y=-0.092, bevel=0.010)
    prism("katana_silver_edge", edge, 0.16, silver, y=0.01, bevel=0.010)
    prism("katana_hot_edge", [(-0.145, 1.10), (-0.105, 1.10), (-0.105, 2.78), (0.02, 3.05), (-0.02, 3.11), (-0.15, 2.84)], 0.025, pink, y=-0.13, bevel=0.006)
    energy_motes(rng, "katana", (0.02, -0.02, 2.10), (0.52, 0.18, 1.20), 16, [purple, pink, green])
    export_glb("hoplite_dragon_katana")


def make_golem_hammer() -> None:
    clear_scene()
    rng = random.Random(4503)
    iron = material("hammer_iron", (0.63, 0.67, 0.65), metallic=0.72, roughness=0.34)
    bright = material("hammer_bright_edge", (0.84, 0.88, 0.84), metallic=0.80, roughness=0.24)
    dark = material("hammer_lodestone", (0.11, 0.12, 0.12), metallic=0.65, roughness=0.38)
    wood = material("hammer_blaze_handle", (0.34, 0.12, 0.07), roughness=0.66)
    red = material("hammer_golem_core", (0.82, 0.04, 0.025), roughness=0.24, glow=(0.80, 0.025, 0.01))

    cylinder_between("hammer_handle", (0.0, 0.0, 0.04), (0.0, 0.0, 1.76), 0.105, wood, vertices=8)
    for index, z in enumerate((0.15, 0.39, 0.63, 0.87, 1.11, 1.35, 1.58)):
        bevel_box(f"hammer_handle_band_{index}", (0.0, 0.0, z), (0.135, 0.135, 0.055), dark if index % 2 else iron, bevel=0.018)
    bevel_box("hammer_pommel", (0.0, 0.0, 0.04), (0.18, 0.18, 0.12), bright, bevel=0.032)

    bevel_box("hammer_core", (0.0, 0.0, 2.02), (0.46, 0.34, 0.45), dark, bevel=0.06)
    bevel_box("hammer_left_head", (-0.50, 0.0, 2.02), (0.34, 0.40, 0.58), iron, bevel=0.055)
    bevel_box("hammer_right_head", (0.50, 0.0, 2.02), (0.34, 0.40, 0.58), iron, bevel=0.055)
    bevel_box("hammer_left_face", (-0.72, 0.0, 2.02), (0.10, 0.47, 0.66), bright, bevel=0.035)
    bevel_box("hammer_right_face", (0.72, 0.0, 2.02), (0.10, 0.47, 0.66), bright, bevel=0.035)
    bevel_box("hammer_brow", (0.0, -0.19, 2.20), (0.28, 0.035, 0.10), iron, bevel=0.018)
    bevel_box("hammer_eye_left", (-0.13, -0.195, 2.08), (0.07, 0.028, 0.07), red, bevel=0.015)
    bevel_box("hammer_eye_right", (0.13, -0.195, 2.08), (0.07, 0.028, 0.07), red, bevel=0.015)
    for index, x in enumerate((-0.55, -0.35, 0.35, 0.55)):
        bevel_box(f"hammer_red_rune_{index}", (x, -0.225, 1.89 + 0.10 * (index % 2)), (0.035, 0.018, 0.18), red, bevel=0.008, rotation=(0.0, 0.0, (-1) ** index * 0.25))
    energy_motes(rng, "hammer", (0.0, -0.02, 1.92), (1.05, 0.25, 0.78), 14, [iron, red])
    export_glb("hoplite_golem_hammer")


def make_midas_sword() -> None:
    clear_scene()
    rng = random.Random(4504)
    gold = material("midas_gold", (0.95, 0.56, 0.06), metallic=0.78, roughness=0.24)
    bright = material("midas_bright_gold", (1.00, 0.83, 0.24), metallic=0.72, roughness=0.20, glow=(0.58, 0.22, 0.01))
    pale = material("midas_edge", (1.00, 0.92, 0.54), metallic=0.64, roughness=0.20)
    emerald = material("midas_emerald", (0.04, 0.66, 0.42), metallic=0.24, roughness=0.22, glow=(0.01, 0.48, 0.20))
    leather = material("midas_grip", (0.26, 0.12, 0.045), roughness=0.72)

    cylinder_between("midas_handle", (0.0, 0.0, 0.05), (0.0, 0.0, 0.82), 0.085, leather, vertices=8)
    for index, z in enumerate((0.15, 0.31, 0.47, 0.63, 0.79)):
        bevel_box(f"midas_grip_band_{index}", (0.0, -0.07, z), (0.095, 0.035, 0.035), gold, bevel=0.012, rotation=(0.0, 0.0, (-1) ** index * 0.30))
    bevel_box("midas_pommel", (0.0, 0.0, 0.04), (0.13, 0.13, 0.12), gold, bevel=0.028)
    bevel_box("midas_guard", (0.0, 0.0, 0.90), (0.88, 0.16, 0.11), gold, bevel=0.035)
    bevel_box("midas_guard_left_tip", (-0.43, 0.0, 0.94), (0.24, 0.16, 0.11), bright, bevel=0.028, rotation=(0.0, 0.0, math.radians(18.0)))
    bevel_box("midas_guard_right_tip", (0.43, 0.0, 0.94), (0.24, 0.16, 0.11), bright, bevel=0.028, rotation=(0.0, 0.0, math.radians(-18.0)))
    bevel_box("midas_guard_left_gem", (-0.39, -0.095, 0.93), (0.10, 0.025, 0.10), emerald, bevel=0.018, rotation=(0.0, 0.0, math.radians(45.0)))
    bevel_box("midas_guard_right_gem", (0.39, -0.095, 0.93), (0.10, 0.025, 0.10), emerald, bevel=0.018, rotation=(0.0, 0.0, math.radians(45.0)))

    outer = [(-0.22, 0.98), (0.22, 0.98), (0.28, 2.58), (0.0, 3.02), (-0.28, 2.58)]
    core = [(-0.13, 1.06), (0.13, 1.06), (0.17, 2.54), (0.0, 2.84), (-0.17, 2.54)]
    left_edge = [(-0.22, 0.98), (-0.13, 1.06), (-0.17, 2.54), (0.0, 2.84), (0.0, 3.02), (-0.28, 2.58)]
    prism("midas_outer_blade", outer, 0.15, gold, bevel=0.022)
    prism("midas_blade_core", core, 0.045, bright, y=-0.10, bevel=0.010)
    prism("midas_pale_edge", left_edge, 0.17, pale, y=0.01, bevel=0.010)
    prism("midas_center_rune", [(-0.045, 1.14), (0.045, 1.14), (0.055, 2.47), (0.0, 2.62), (-0.055, 2.47)], 0.025, emerald, y=-0.135, bevel=0.006)
    energy_motes(rng, "midas", (0.0, -0.02, 1.95), (0.84, 0.20, 1.08), 22, [gold, bright, emerald])
    export_glb("hoplite_midas_sword")


def add_preview_lights() -> None:
    bpy.ops.object.light_add(type="AREA", location=(-3.5, -4.0, 6.0), rotation=(math.radians(28.0), 0.0, math.radians(-25.0)))
    bpy.context.object.data.energy = 780.0
    bpy.context.object.data.size = 5.0
    bpy.ops.object.light_add(type="AREA", location=(4.0, -2.0, 3.5), rotation=(math.radians(65.0), 0.0, math.radians(140.0)))
    bpy.context.object.data.energy = 520.0
    bpy.context.object.data.color = (0.42, 0.58, 1.0)
    bpy.context.object.data.size = 4.0


def point_camera(camera, target) -> None:
    camera.rotation_euler = (Vector(target) - camera.location).to_track_quat("-Z", "Y").to_euler()


def render_preview(path: Path) -> None:
    clear_scene()
    backdrop = material("preview_backdrop", (0.055, 0.062, 0.073), roughness=0.92)
    bevel_box("preview_floor", (0.0, 0.45, -0.12), (6.7, 2.8, 0.10), backdrop, bevel=0.04)
    placements = [
        ("hoplite_reaper_scythe.glb", -4.55, -0.08),
        ("hoplite_dragon_katana.glb", -1.55, 0.08),
        ("hoplite_golem_hammer.glb", 1.55, 0.02),
        ("hoplite_midas_sword.glb", 4.55, -0.08),
    ]
    for filename, x, tilt in placements:
        before = set(bpy.context.scene.objects)
        bpy.ops.import_scene.gltf(filepath=str(OUT_DIR / filename))
        imported = [obj for obj in bpy.context.scene.objects if obj not in before]
        for obj in imported:
            obj.location.x += x
            obj.rotation_euler.y += tilt

    add_preview_lights()
    bpy.ops.object.camera_add(location=(0.0, -13.5, 4.2))
    camera = bpy.context.object
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 12.4
    point_camera(camera, (0.0, 0.0, 1.42))
    bpy.context.scene.camera = camera
    scene = bpy.context.scene
    engines = {item.identifier for item in bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items}
    scene.render.engine = "BLENDER_EEVEE_NEXT" if "BLENDER_EEVEE_NEXT" in engines else "BLENDER_EEVEE"
    scene.render.resolution_x = 1680
    scene.render.resolution_y = 900
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = False
    scene.world.color = (0.018, 0.022, 0.032)
    scene.view_settings.look = "AgX - Medium High Contrast"
    path.parent.mkdir(parents=True, exist_ok=True)
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    print(f"preview -> {path}")


def build_assets() -> None:
    make_reaper_scythe()
    make_dragon_katana()
    make_golem_hammer()
    make_midas_sword()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--preview", action="store_true")
    parser.add_argument("--preview-path", default=str(PREVIEW_PATH))
    script_args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    args = parser.parse_args(script_args)
    build_assets()
    if args.preview:
        render_preview(Path(args.preview_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
