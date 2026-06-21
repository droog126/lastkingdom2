"""低多边形风格化资产构建工具库.

所有 pretty/ 下的 .glb 都通过这里生成. 风格统一: flat shading, ≤ 800 面/模型,
Bevy Y-up (Blender 默认 Z-up, glTF 导出时 Bevy 会自动转).

设计原则:
- 每个模型 1 个或几个 mesh, 不做复杂骨骼 (动画交给 Rust 侧)
- 坐标系: Blender 默认 Z-up, 导出 glTF 后 Bevy 端视作 Y-up
- 材质全部 bake 进 .glb (Principled BSDF + export_materials=EXPORT)
"""

from __future__ import annotations

import math
from pathlib import Path
from typing import Tuple

import bpy


ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "assets" / "procedural" / "pretty"


RGBA = Tuple[float, float, float, float]
RGB = Tuple[float, float, float]


def clear_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()


def mat(
    name: str,
    color: RGB,
    roughness: float = 0.8,
    alpha: float = 1.0,
    metallic: float = 0.0,
    emissive: RGB = (0.0, 0.0, 0.0),
) -> bpy.types.Material:
    material = bpy.data.materials.new(name)
    material.use_nodes = True
    bsdf = next(
        (n for n in material.node_tree.nodes if n.type == "BSDF_PRINCIPLED"),
        None,
    )
    if bsdf is None:
        bsdf = material.node_tree.nodes.new(type="ShaderNodeBsdfPrincipled")
    bsdf.inputs["Base Color"].default_value = (
        color[0],
        color[1],
        color[2],
        alpha,
    )
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    if alpha < 1.0:
        bsdf.inputs["Alpha"].default_value = alpha
        material.blend_method = "BLEND"
    if any(c > 0.0 for c in emissive):
        bsdf.inputs["Emission Color"].default_value = (
            emissive[0],
            emissive[1],
            emissive[2],
            1.0,
        )
        bsdf.inputs["Emission Strength"].default_value = 1.0
    return material


def shade_flat(obj: bpy.types.Object) -> None:
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.shade_flat()
    obj.select_set(False)


def shade_smooth(obj: bpy.types.Object) -> None:
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.shade_smooth()
    obj.select_set(False)


def _set_mat(obj: bpy.types.Object, material: bpy.types.Material) -> None:
    obj.data.materials.clear()
    obj.data.materials.append(material)


def cube(
    name: str,
    loc: Tuple[float, float, float],
    scale: Tuple[float, float, float],
    material: bpy.types.Material,
) -> bpy.types.Object:
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    _set_mat(obj, material)
    shade_flat(obj)
    return obj


def uv_sphere(
    name: str,
    loc: Tuple[float, float, float],
    scale: Tuple[float, float, float],
    material: bpy.types.Material,
    segments: int = 16,
    rings: int = 8,
) -> bpy.types.Object:
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments,
        ring_count=rings,
        radius=1.0,
        location=loc,
    )
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    _set_mat(obj, material)
    shade_smooth(obj)
    return obj


def cone(
    name: str,
    loc: Tuple[float, float, float],
    radius1: float,
    radius2: float,
    depth: float,
    material: bpy.types.Material,
    vertices: int = 8,
) -> bpy.types.Object:
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices,
        radius1=radius1,
        radius2=radius2,
        depth=depth,
        location=loc,
    )
    obj = bpy.context.object
    obj.name = name
    _set_mat(obj, material)
    shade_flat(obj)
    return obj


def cylinder(
    name: str,
    loc: Tuple[float, float, float],
    radius: float,
    depth: float,
    material: bpy.types.Material,
    vertices: int = 24,
) -> bpy.types.Object:
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=depth,
        location=loc,
    )
    obj = bpy.context.object
    obj.name = name
    _set_mat(obj, material)
    shade_flat(obj)
    return obj


def ico_sphere(
    name: str,
    loc: Tuple[float, float, float],
    scale: Tuple[float, float, float],
    material: bpy.types.Material,
    subdivisions: int = 1,
) -> bpy.types.Object:
    bpy.ops.mesh.primitive_ico_sphere_add(
        subdivisions=subdivisions,
        radius=1.0,
        location=loc,
    )
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    _set_mat(obj, material)
    shade_flat(obj)
    return obj


def merge_into(obj: bpy.types.Object, target_name: str) -> bpy.types.Object:
    """把所有 mesh 合并到 obj, 返回合并后的对象 (用于控制面数 ≤ 800)."""
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    for other in bpy.context.scene.objects:
        if other is not obj and other.type == "MESH":
            other.select_set(True)
    bpy.ops.object.join()
    out = bpy.context.object
    out.name = target_name
    return out


def export_glb(name: str) -> Path:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    path = OUT_DIR / f"{name}.glb"
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_materials="EXPORT",
    )
    print(f"exported {path} ({path.stat().st_size} bytes)")
    return path
