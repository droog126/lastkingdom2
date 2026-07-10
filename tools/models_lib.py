from __future__ import annotations

import inspect
import math
import os
from pathlib import Path
from typing import Iterable, Sequence

import bpy
from mathutils import Vector

from model_catalog import owner_for, scale_contract_for
from model_style import (
    COLLECTION_DIRS,
    ROOT,
    STYLE_NAME,
    STYLE_VERSION,
    validate_rgb,
)


RGB = tuple[float, float, float]
Vec3Like = Sequence[float]


def output_dir(collection: str = "pretty") -> Path:
    try:
        production = COLLECTION_DIRS[collection]
    except KeyError as exc:
        raise ValueError(f"unknown model collection: {collection}") from exc
    staging_root = os.environ.get("LK2_MODEL_OUTPUT_ROOT")
    if not staging_root:
        return production
    root = Path(staging_root)
    relative = production.relative_to(ROOT / "assets")
    return root / relative


OUT_DIR = output_dir("pretty")


def clear_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for datablocks in (bpy.data.meshes, bpy.data.curves, bpy.data.cameras, bpy.data.lights):
        for datablock in list(datablocks):
            if datablock.users == 0:
                datablocks.remove(datablock)
    for material in list(bpy.data.materials):
        if material.users == 0:
            bpy.data.materials.remove(material)


def mat(
    name: str,
    color: RGB,
    roughness: float = 0.8,
    alpha: float = 1.0,
    metallic: float = 0.0,
    emissive: RGB = (0.0, 0.0, 0.0),
    emissive_strength: float = 1.0,
) -> bpy.types.Material:
    validate_rgb(color, f"{name} base color")
    validate_rgb(emissive, f"{name} emissive color")
    if not 0.0 <= roughness <= 1.0:
        raise ValueError(f"{name} roughness must be in [0, 1]")
    if not 0.0 <= metallic <= 1.0:
        raise ValueError(f"{name} metallic must be in [0, 1]")
    if not 0.0 < alpha <= 1.0:
        raise ValueError(f"{name} alpha must be in (0, 1]")

    material = bpy.data.materials.new(name=name)
    material.use_nodes = True
    nodes = material.node_tree.nodes
    bsdf = next((node for node in nodes if node.type == "BSDF_PRINCIPLED"), None)
    if bsdf is None:
        bsdf = nodes.new(type="ShaderNodeBsdfPrincipled")
    bsdf.inputs["Base Color"].default_value = (*color, alpha)
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    if alpha < 1.0:
        bsdf.inputs["Alpha"].default_value = alpha
        if hasattr(material, "surface_render_method"):
            material.surface_render_method = "DITHERED"
        elif hasattr(material, "blend_method"):
            material.blend_method = "BLEND"
    if any(component > 0.0 for component in emissive):
        emission_input = bsdf.inputs.get("Emission Color") or bsdf.inputs.get("Emission")
        if emission_input is not None:
            emission_input.default_value = (*emissive, 1.0)
        strength_input = bsdf.inputs.get("Emission Strength")
        if strength_input is not None:
            strength_input.default_value = max(0.0, emissive_strength)
    material["lk2_style"] = STYLE_NAME
    material["lk2_style_version"] = STYLE_VERSION
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
    loc: Vec3Like,
    scale: Vec3Like,
    material: bpy.types.Material,
    *,
    rotation: Vec3Like = (0.0, 0.0, 0.0),
) -> bpy.types.Object:
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc, rotation=rotation)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    _set_mat(obj, material)
    shade_flat(obj)
    return obj


def bevel_box(
    name: str,
    loc: Vec3Like,
    scale: Vec3Like,
    material: bpy.types.Material,
    *,
    bevel: float = 0.025,
    rotation: Vec3Like = (0.0, 0.0, 0.0),
) -> bpy.types.Object:
    obj = cube(name, loc, scale, material, rotation=rotation)
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel > 0.0:
        modifier = obj.modifiers.new("lk2_bevel", "BEVEL")
        modifier.width = bevel
        modifier.segments = 1
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    obj.select_set(False)
    return obj


def uv_sphere(
    name: str,
    loc: Vec3Like,
    scale: Vec3Like,
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
    loc: Vec3Like,
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
    loc: Vec3Like,
    radius: float,
    depth: float,
    material: bpy.types.Material,
    vertices: int = 12,
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
    loc: Vec3Like,
    scale: Vec3Like,
    material: bpy.types.Material,
    subdivisions: int = 1,
) -> bpy.types.Object:
    if not 1 <= subdivisions <= 3:
        raise ValueError("ico sphere subdivisions must be between 1 and 3")
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


def _y_up_location(value: Vec3Like) -> tuple[float, float, float]:
    return value[0], value[2], value[1]


def _y_up_scale(value: Vec3Like) -> tuple[float, float, float]:
    return value[0], value[2], value[1]


def cube_y_up(name, loc, scale, material):
    return cube(name, _y_up_location(loc), _y_up_scale(scale), material)


def cone_y_up(name, loc, radius1, radius2, depth, material, vertices=8):
    return cone(name, _y_up_location(loc), radius1, radius2, depth, material, vertices)


def cylinder_y_up(name, loc, radius, depth, material, vertices=12):
    return cylinder(name, _y_up_location(loc), radius, depth, material, vertices)


def ico_sphere_y_up(name, loc, scale, material, subdivisions=1):
    return ico_sphere(
        name,
        _y_up_location(loc),
        _y_up_scale(scale),
        material,
        subdivisions,
    )


def uv_sphere_y_up(name, loc, scale, material, segments=16, rings=8):
    return uv_sphere(
        name,
        _y_up_location(loc),
        _y_up_scale(scale),
        material,
        segments,
        rings,
    )


def prism(
    name: str,
    points: Sequence[tuple[float, float]],
    depth: float,
    material: bpy.types.Material,
    *,
    y: float = 0.0,
    bevel: float = 0.018,
) -> bpy.types.Object:
    if len(points) < 3:
        raise ValueError("prism requires at least three points")
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
    _set_mat(obj, material)
    if bevel > 0.0:
        modifier = obj.modifiers.new("lk2_bevel", "BEVEL")
        modifier.width = bevel
        modifier.segments = 1
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)
        bpy.ops.object.modifier_apply(modifier=modifier.name)
        obj.select_set(False)
    shade_flat(obj)
    return obj


def cylinder_between(
    name: str,
    start: Vec3Like,
    end: Vec3Like,
    radius: float,
    material: bpy.types.Material,
    *,
    vertices: int = 8,
) -> bpy.types.Object:
    start_vec = Vector(start)
    end_vec = Vector(end)
    direction = end_vec - start_vec
    if direction.length <= 1e-6:
        raise ValueError(f"{name} endpoints must differ")
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=direction.length,
        location=(start_vec + end_vec) * 0.5,
    )
    obj = bpy.context.object
    obj.name = name
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = Vector((0.0, 0.0, 1.0)).rotation_difference(direction.normalized())
    _set_mat(obj, material)
    shade_flat(obj)
    return obj


def cone_between(
    name: str,
    start: Vec3Like,
    end: Vec3Like,
    radius: float,
    material: bpy.types.Material,
    *,
    vertices: int = 6,
) -> bpy.types.Object:
    start_vec = Vector(start)
    end_vec = Vector(end)
    direction = end_vec - start_vec
    if direction.length <= 1e-6:
        raise ValueError(f"{name} endpoints must differ")
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices,
        radius1=radius,
        radius2=0.0,
        depth=direction.length,
        location=(start_vec + end_vec) * 0.5,
    )
    obj = bpy.context.object
    obj.name = name
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = Vector((0.0, 0.0, 1.0)).rotation_difference(direction.normalized())
    _set_mat(obj, material)
    shade_flat(obj)
    return obj


def merge_into(obj: bpy.types.Object, target_name: str) -> bpy.types.Object:
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    for other in bpy.context.scene.objects:
        if other is not obj and other.type == "MESH":
            other.select_set(True)
    bpy.ops.object.join()
    output = bpy.context.object
    output.name = target_name
    return output


def _calling_generator() -> str:
    for frame in inspect.stack()[2:]:
        name = Path(frame.filename).name
        if name.startswith("build_") or name == "create_eco_models.py":
            return name
    return "<interactive>"


def _assert_owner(name: str, collection: str, generator: str) -> None:
    owner = owner_for(collection, name)
    if owner is None:
        raise RuntimeError(f"{collection}/{name} is not registered in model_catalog.py")
    if owner.script != generator:
        raise RuntimeError(
            f"{collection}/{name} is owned by {owner.script}; {generator} may not overwrite it"
        )


def _mesh_objects() -> list[bpy.types.Object]:
    return [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]


def _consolidate_static_meshes(name: str, objects: list[bpy.types.Object]) -> list[bpy.types.Object]:
    if len(objects) <= 1:
        if objects:
            objects[0].name = f"{name}_root"
            bpy.ops.object.select_all(action="DESELECT")
            objects[0].select_set(True)
            bpy.context.view_layer.objects.active = objects[0]
        root = objects[0] if objects else None
    else:
        bpy.ops.object.select_all(action="DESELECT")
        for obj in objects:
            obj.select_set(True)
        bpy.context.view_layer.objects.active = objects[0]
        bpy.ops.object.join()
        root = bpy.context.object
    if root is None:
        return []
    root.name = f"{name}_root"
    root.data.name = f"{name}_mesh"
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    return [root]


def _world_bounds(objects: Iterable[bpy.types.Object]) -> tuple[Vector, Vector]:
    minimum = Vector((math.inf, math.inf, math.inf))
    maximum = Vector((-math.inf, -math.inf, -math.inf))
    for obj in objects:
        for corner in obj.bound_box:
            point = obj.matrix_world @ Vector(corner)
            minimum.x = min(minimum.x, point.x)
            minimum.y = min(minimum.y, point.y)
            minimum.z = min(minimum.z, point.z)
            maximum.x = max(maximum.x, point.x)
            maximum.y = max(maximum.y, point.y)
            maximum.z = max(maximum.z, point.z)
    return minimum, maximum


def _normalize_asset_scale(name: str, collection: str, root: bpy.types.Object) -> None:
    contract = scale_contract_for(collection, name)
    if contract is None:
        raise RuntimeError(f"missing scale contract for {collection}/{name}")
    bpy.context.view_layer.update()
    minimum, maximum = _world_bounds([root])
    dimensions = maximum - minimum
    measured = (
        dimensions.z
        if contract.metric == "height"
        else max(dimensions.x, dimensions.y)
        if contract.target_height_meters is not None
        else max(dimensions)
    )
    if measured <= 1e-6 or not math.isfinite(measured):
        raise RuntimeError(f"cannot normalize invalid bounds for {collection}/{name}: {dimensions}")
    factor = contract.target_meters / measured
    root.scale = tuple(component * factor for component in root.scale)
    bpy.context.view_layer.objects.active = root
    root.select_set(True)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    bpy.context.view_layer.update()

    if contract.target_height_meters is not None:
        minimum, maximum = _world_bounds([root])
        current_height = maximum.z - minimum.z
        if current_height <= 1e-6:
            raise RuntimeError(f"cannot normalize zero height for {collection}/{name}")
        root.scale.z *= contract.target_height_meters / current_height
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
        bpy.context.view_layer.update()

    minimum, maximum = _world_bounds([root])
    center = (minimum + maximum) * 0.5
    root.location.x -= center.x
    root.location.y -= center.y
    root.location.z -= minimum.z if contract.grounded else center.z
    bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)
    bpy.context.view_layer.update()

    normalized_min, normalized_max = _world_bounds([root])
    normalized_dimensions = normalized_max - normalized_min
    normalized = (
        normalized_dimensions.z
        if contract.metric == "height"
        else max(normalized_dimensions.x, normalized_dimensions.y)
        if contract.target_height_meters is not None
        else max(normalized_dimensions)
    )
    if abs(normalized - contract.target_meters) > contract.target_meters * 0.005:
        raise RuntimeError(
            f"failed to normalize {collection}/{name}: expected {contract.target_meters}m, "
            f"measured {normalized:.4f}m"
        )
    if contract.target_height_meters is not None:
        if abs(normalized_dimensions.z - contract.target_height_meters) > contract.target_height_meters * 0.005:
            raise RuntimeError(
                f"failed to normalize {collection}/{name} height: expected "
                f"{contract.target_height_meters}m, measured {normalized_dimensions.z:.4f}m"
            )


def _assert_scene_contract(objects: Iterable[bpy.types.Object]) -> None:
    objects = list(objects)
    if not objects:
        raise RuntimeError("cannot export a scene without mesh objects")
    names: set[str] = set()
    for obj in objects:
        if obj.name in names:
            raise RuntimeError(f"duplicate object name: {obj.name}")
        names.add(obj.name)
        if not obj.data.materials:
            raise RuntimeError(f"mesh object has no material: {obj.name}")
        values = (*obj.location, *obj.scale, *obj.rotation_euler)
        if not all(math.isfinite(float(value)) for value in values):
            raise RuntimeError(f"mesh object has non-finite transform: {obj.name}")
        if any(abs(float(value)) <= 1e-8 for value in obj.scale):
            raise RuntimeError(f"mesh object has zero scale: {obj.name}")


def export_glb(name: str, *, collection: str = "pretty") -> Path | None:
    generator = _calling_generator()
    _assert_owner(name, collection, generator)
    selected_asset = os.environ.get("LK2_MODEL_ONLY")
    if selected_asset and selected_asset != name:
        print(f"skip {collection}/{name}; LK2_MODEL_ONLY={selected_asset}")
        return None

    objects = _consolidate_static_meshes(name, _mesh_objects())
    _assert_scene_contract(objects)
    _normalize_asset_scale(name, collection, objects[0])
    for obj in objects:
        obj["lk2_style"] = STYLE_NAME
        obj["lk2_style_version"] = STYLE_VERSION
        obj["lk2_generator"] = generator

    directory = output_dir(collection)
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / f"{name}.glb"
    temporary = directory / f".{name}.tmp.glb"
    temporary.unlink(missing_ok=True)

    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.export_scene.gltf(
        filepath=str(temporary),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_materials="EXPORT",
        export_animations=False,
        export_extras=True,
        export_yup=True,
    )
    if not temporary.is_file() or temporary.stat().st_size == 0:
        raise RuntimeError(f"Blender did not produce a valid temporary GLB for {name}")
    temporary.replace(path)
    print(f"exported {path} ({path.stat().st_size} bytes, style={STYLE_NAME}-v{STYLE_VERSION})")
    return path
