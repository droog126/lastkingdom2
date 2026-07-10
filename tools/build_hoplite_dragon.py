from __future__ import annotations

import argparse
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from model_style import TEMP_PREVIEW_DIR
from models_lib import (
    OUT_DIR,
    bevel_box,
    clear_scene,
    cone_between,
    cylinder_between,
    export_glb,
    ico_sphere,
    mat,
    shade_flat,
)


PREVIEW_PATH = TEMP_PREVIEW_DIR / "hoplite_dragon.png"


def material(name: str, color, *, metallic=0.0, roughness=0.72, glow=None):
    return mat(
        name,
        color,
        metallic=metallic,
        roughness=roughness,
        emissive=glow or (0.0, 0.0, 0.0),
    )


def wing_panel(name: str, side: float, membrane, bone):
    root = Vector((0.34 * side, 0.10, 1.72))
    elbow = Vector((1.35 * side, 0.18, 2.36))
    tip = Vector((2.75 * side, 0.56, 2.02))
    rear = Vector((1.62 * side, 1.18, 1.18))
    inner = Vector((0.48 * side, 0.78, 1.30))
    points = [root, elbow, tip, rear, inner]
    thickness = 0.035
    vertices = [(p.x, p.y - thickness, p.z) for p in points] + [
        (p.x, p.y + thickness, p.z) for p in points
    ]
    faces = [
        (0, 1, 2, 3, 4),
        (9, 8, 7, 6, 5),
        (0, 5, 6, 1),
        (1, 6, 7, 2),
        (2, 7, 8, 3),
        (3, 8, 9, 4),
        (4, 9, 5, 0),
    ]
    mesh = bpy.data.meshes.new(f"{name}_mesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(membrane)
    shade_flat(obj)
    cylinder_between(f"{name}_leading_bone", root, elbow, 0.075, bone)
    cylinder_between(f"{name}_outer_bone", elbow, tip, 0.060, bone)
    cylinder_between(f"{name}_rear_bone", root, rear, 0.050, bone)


def build_dragon() -> None:
    clear_scene()
    hide = material("dragon_obsidian_hide", (0.025, 0.028, 0.045), metallic=0.36, roughness=0.42)
    plates = material("dragon_violet_plates", (0.17, 0.045, 0.29), metallic=0.28, roughness=0.34)
    membrane = material("dragon_wing_membrane", (0.25, 0.035, 0.38), roughness=0.50, glow=(0.10, 0.01, 0.18))
    bone = material("dragon_bone", (0.63, 0.58, 0.72), metallic=0.18, roughness=0.52)
    eye = material("dragon_ender_eye", (0.30, 1.00, 0.24), roughness=0.20, glow=(0.18, 0.90, 0.10))
    core = material("dragon_heart_glow", (0.82, 0.04, 0.68), roughness=0.18, glow=(0.72, 0.02, 0.52))

    body = ico_sphere("dragon_body", (0.0, 0.12, 1.28), (0.68, 0.98, 0.56), hide, subdivisions=2)
    body.rotation_euler.z = math.radians(3.0)
    bevel_box("dragon_chest_plate", (0.0, -0.48, 1.28), (0.54, 0.20, 0.46), plates, bevel=0.07)
    bevel_box("dragon_heart", (0.0, -0.60, 1.30), (0.20, 0.035, 0.22), core, bevel=0.04, rotation=(0.0, 0.0, math.radians(45.0)))

    cylinder_between("dragon_neck_low", (0.0, -0.38, 1.45), (0.0, -0.92, 1.68), 0.30, hide)
    cylinder_between("dragon_neck_high", (0.0, -0.82, 1.66), (0.0, -1.27, 1.82), 0.25, plates)
    bevel_box("dragon_head", (0.0, -1.48, 1.85), (0.72, 0.72, 0.48), hide, bevel=0.08)
    bevel_box("dragon_snout", (0.0, -1.90, 1.76), (0.58, 0.42, 0.28), plates, bevel=0.055)
    bevel_box("dragon_left_eye", (-0.22, -1.865, 1.94), (0.13, 0.035, 0.10), eye, bevel=0.025)
    bevel_box("dragon_right_eye", (0.22, -1.865, 1.94), (0.13, 0.035, 0.10), eye, bevel=0.025)
    bevel_box("dragon_mouth", (0.0, -2.12, 1.70), (0.34, 0.025, 0.045), core, bevel=0.012)
    cone_between("dragon_left_horn", (-0.22, -1.23, 2.06), (-0.52, -0.62, 2.46), 0.13, bone)
    cone_between("dragon_right_horn", (0.22, -1.23, 2.06), (0.52, -0.62, 2.46), 0.13, bone)

    for side in (-1.0, 1.0):
        wing_panel("dragon_left_wing" if side < 0 else "dragon_right_wing", side, membrane, bone)
        hip = (0.46 * side, 0.38, 1.18)
        knee = (0.66 * side, 0.42, 0.66)
        foot = (0.72 * side, 0.08, 0.25)
        cylinder_between(f"dragon_leg_{side}_upper", hip, knee, 0.13, hide)
        cylinder_between(f"dragon_leg_{side}_lower", knee, foot, 0.10, plates)
        for claw_index, dx in enumerate((-0.10, 0.0, 0.10)):
            claw_start = (foot[0] + dx, foot[1], foot[2])
            claw_end = (foot[0] + dx + 0.035 * side, foot[1] - 0.28, 0.10)
            cone_between(f"dragon_claw_{side}_{claw_index}", claw_start, claw_end, 0.045, bone, vertices=5)

    tail_points = [
        (0.0, 0.72, 1.25),
        (0.08, 1.25, 1.15),
        (-0.06, 1.80, 0.98),
        (0.10, 2.28, 0.78),
        (0.0, 2.72, 0.62),
    ]
    radii = [0.25, 0.20, 0.15, 0.10]
    for index, (start, end, radius) in enumerate(zip(tail_points, tail_points[1:], radii)):
        cylinder_between(f"dragon_tail_{index}", start, end, radius, hide if index % 2 == 0 else plates)
    cone_between("dragon_tail_tip", tail_points[-1], (0.0, 3.22, 0.50), 0.12, bone)

    for index, y in enumerate((-0.20, 0.18, 0.58, 1.00, 1.42, 1.82)):
        height = 2.02 - index * 0.18
        cone_between(
            f"dragon_spine_{index}",
            (0.0, y, height - 0.08),
            (0.0, y + 0.02, height + 0.25),
            max(0.045, 0.10 - index * 0.008),
            bone,
            vertices=5,
        )

    export_glb("hoplite_ender_dragon")


def point_camera(camera, target) -> None:
    camera.rotation_euler = (Vector(target) - camera.location).to_track_quat("-Z", "Y").to_euler()


def render_preview(path: Path) -> None:
    clear_scene()
    ground_mat = material("preview_ground", (0.055, 0.065, 0.075), roughness=0.94)
    bevel_box("preview_ground", (0.0, 0.25, -0.07), (7.0, 7.0, 0.12), ground_mat, bevel=0.05)
    bpy.ops.import_scene.gltf(filepath=str(OUT_DIR / "hoplite_ender_dragon.glb"))
    bpy.ops.object.light_add(type="AREA", location=(-3.5, -4.5, 6.5))
    bpy.context.object.data.energy = 900.0
    bpy.context.object.data.size = 5.0
    bpy.ops.object.light_add(type="AREA", location=(4.0, 1.0, 4.0))
    bpy.context.object.data.energy = 650.0
    bpy.context.object.data.color = (0.50, 0.30, 1.0)
    bpy.context.object.data.size = 4.0
    bpy.ops.object.camera_add(location=(5.6, -7.6, 4.7))
    camera = bpy.context.object
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 6.6
    point_camera(camera, (0.0, 0.15, 1.25))
    bpy.context.scene.camera = camera
    scene = bpy.context.scene
    engines = {item.identifier for item in bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items}
    scene.render.engine = "BLENDER_EEVEE_NEXT" if "BLENDER_EEVEE_NEXT" in engines else "BLENDER_EEVEE"
    scene.render.resolution_x = 1400
    scene.render.resolution_y = 1000
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.world.color = (0.012, 0.016, 0.025)
    scene.view_settings.look = "AgX - Medium High Contrast"
    path.parent.mkdir(parents=True, exist_ok=True)
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    print(f"preview -> {path}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--preview", action="store_true")
    parser.add_argument("--preview-path", default=str(PREVIEW_PATH))
    script_args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    args = parser.parse_args(script_args)
    build_dragon()
    if args.preview:
        render_preview(Path(args.preview_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
