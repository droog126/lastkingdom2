from __future__ import annotations

import math
from itertools import product
from dataclasses import dataclass
from pathlib import Path

from pygltflib import GLTF2

from model_style import (
    HARD_TRIANGLE_LIMIT,
    MAX_MATERIAL_COUNT,
    MAX_NODE_COUNT,
    ScaleContract,
    budget_for,
)


@dataclass(frozen=True)
class GlbStats:
    path: Path
    meshes: int
    nodes: int
    materials: int
    triangles: int
    vertices: int
    bounds_min: tuple[float, float, float]
    bounds_max: tuple[float, float, float]
    dimensions: tuple[float, float, float]


@dataclass(frozen=True)
class ValidationResult:
    stats: GlbStats | None
    errors: tuple[str, ...]
    warnings: tuple[str, ...]

    @property
    def ok(self) -> bool:
        return not self.errors


def _triangle_count(index_count: int, mode: int | None) -> int:
    mode = 4 if mode is None else mode
    if mode == 4:
        return index_count // 3
    if mode in (5, 6):
        return max(0, index_count - 2)
    return 0


def _identity() -> list[list[float]]:
    return [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]


def _multiply(left: list[list[float]], right: list[list[float]]) -> list[list[float]]:
    return [
        [sum(left[row][index] * right[index][column] for index in range(4)) for column in range(4)]
        for row in range(4)
    ]


def _node_matrix(node) -> list[list[float]]:
    if node.matrix:
        return [[float(node.matrix[column * 4 + row]) for column in range(4)] for row in range(4)]
    tx, ty, tz = node.translation or (0.0, 0.0, 0.0)
    sx, sy, sz = node.scale or (1.0, 1.0, 1.0)
    x, y, z, w = node.rotation or (0.0, 0.0, 0.0, 1.0)
    rotation = [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w)],
        [2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w)],
        [2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y)],
    ]
    return [
        [rotation[0][0] * sx, rotation[0][1] * sy, rotation[0][2] * sz, tx],
        [rotation[1][0] * sx, rotation[1][1] * sy, rotation[1][2] * sz, ty],
        [rotation[2][0] * sx, rotation[2][1] * sy, rotation[2][2] * sz, tz],
        [0.0, 0.0, 0.0, 1.0],
    ]


def _transform_point(matrix: list[list[float]], point: tuple[float, float, float]) -> tuple[float, float, float]:
    x, y, z = point
    return tuple(
        matrix[row][0] * x + matrix[row][1] * y + matrix[row][2] * z + matrix[row][3]
        for row in range(3)
    )


def _world_bounds(glb: GLTF2) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
    minimum = [math.inf, math.inf, math.inf]
    maximum = [-math.inf, -math.inf, -math.inf]
    nodes = glb.nodes or []
    child_nodes = {child for node in nodes for child in (node.children or [])}
    roots = []
    if glb.scenes:
        scene_index = glb.scene or 0
        roots = list(glb.scenes[scene_index].nodes or [])
    if not roots:
        roots = [index for index in range(len(nodes)) if index not in child_nodes]

    visited: set[int] = set()

    def visit(index: int, parent_matrix: list[list[float]]) -> None:
        if index < 0 or index >= len(nodes):
            return
        visited.add(index)
        node = nodes[index]
        world = _multiply(parent_matrix, _node_matrix(node))
        if node.mesh is not None and 0 <= node.mesh < len(glb.meshes or []):
            for primitive in glb.meshes[node.mesh].primitives or []:
                position_index = getattr(primitive.attributes, "POSITION", None)
                if position_index is None or position_index >= len(glb.accessors or []):
                    continue
                accessor = glb.accessors[position_index]
                if accessor.min is None or accessor.max is None:
                    continue
                for corner in product(*zip(accessor.min, accessor.max)):
                    point = _transform_point(world, tuple(float(value) for value in corner))
                    for axis in range(3):
                        minimum[axis] = min(minimum[axis], point[axis])
                        maximum[axis] = max(maximum[axis], point[axis])
        for child in node.children or []:
            visit(child, world)

    for root in roots:
        visit(root, _identity())
    for index in range(len(nodes)):
        if index not in visited:
            visit(index, _identity())
    if not all(math.isfinite(value) for value in (*minimum, *maximum)):
        return (0.0, 0.0, 0.0), (0.0, 0.0, 0.0)
    return tuple(minimum), tuple(maximum)


def inspect_glb(path: Path) -> GlbStats:
    glb = GLTF2.load(str(path))
    triangles = 0
    vertices = 0
    for mesh in glb.meshes or []:
        for primitive in mesh.primitives or []:
            position_index = getattr(primitive.attributes, "POSITION", None)
            if position_index is not None:
                vertices += glb.accessors[position_index].count
            if primitive.indices is not None:
                triangles += _triangle_count(glb.accessors[primitive.indices].count, primitive.mode)
    bounds_min, bounds_max = _world_bounds(glb)
    dimensions = tuple(bounds_max[index] - bounds_min[index] for index in range(3))
    return GlbStats(
        path=path,
        meshes=len(glb.meshes or []),
        nodes=len(glb.nodes or []),
        materials=len(glb.materials or []),
        triangles=triangles,
        vertices=vertices,
        bounds_min=bounds_min,
        bounds_max=bounds_max,
        dimensions=dimensions,
    )


def validate_glb(
    path: Path,
    category: str,
    scale_contract: ScaleContract | None = None,
) -> ValidationResult:
    errors: list[str] = []
    warnings: list[str] = []
    try:
        glb = GLTF2.load(str(path))
    except Exception as exc:
        return ValidationResult(None, (f"GLTF2.load failed: {exc}",), ())

    if not glb.meshes:
        errors.append("no meshes")
    if not glb.nodes:
        errors.append("no nodes")
    if not glb.materials:
        errors.append("no materials")

    triangles = 0
    vertices = 0
    for mesh_index, mesh in enumerate(glb.meshes or []):
        if not mesh.primitives:
            errors.append(f"mesh[{mesh_index}] has no primitives")
        for primitive_index, primitive in enumerate(mesh.primitives or []):
            label = f"mesh[{mesh_index}].primitive[{primitive_index}]"
            position_index = getattr(primitive.attributes, "POSITION", None)
            if position_index is None:
                errors.append(f"{label} missing POSITION")
            elif position_index >= len(glb.accessors or []):
                errors.append(f"{label} POSITION accessor out of range")
            else:
                accessor = glb.accessors[position_index]
                vertices += accessor.count
                if accessor.type != "VEC3":
                    errors.append(f"{label} POSITION must be VEC3")
                if accessor.min is None or accessor.max is None:
                    errors.append(f"{label} POSITION missing min/max")
                elif not all(math.isfinite(float(value)) for value in (*accessor.min, *accessor.max)):
                    errors.append(f"{label} POSITION bounds are not finite")
            if primitive.indices is None:
                errors.append(f"{label} is not indexed")
            elif primitive.indices >= len(glb.accessors or []):
                errors.append(f"{label} index accessor out of range")
            else:
                index_accessor = glb.accessors[primitive.indices]
                if index_accessor.type != "SCALAR":
                    errors.append(f"{label} indices must be SCALAR")
                triangles += _triangle_count(index_accessor.count, primitive.mode)
            if primitive.material is None:
                errors.append(f"{label} has no material")

    budget = budget_for(category)
    if triangles > min(budget.triangles, HARD_TRIANGLE_LIMIT):
        errors.append(f"{triangles} triangles exceeds {category} budget {budget.triangles}")
    if len(glb.nodes or []) > MAX_NODE_COUNT:
        errors.append(f"{len(glb.nodes or [])} nodes exceeds hard limit {MAX_NODE_COUNT}")
    elif len(glb.nodes or []) > budget.recommended_nodes:
        warnings.append(
            f"{len(glb.nodes or [])} nodes exceeds {category} recommendation {budget.recommended_nodes}"
        )
    if len(glb.materials or []) > MAX_MATERIAL_COUNT:
        errors.append(f"{len(glb.materials or [])} materials exceeds hard limit {MAX_MATERIAL_COUNT}")
    elif len(glb.materials or []) > budget.recommended_materials:
        warnings.append(
            f"{len(glb.materials or [])} materials exceeds {category} recommendation "
            f"{budget.recommended_materials}"
        )

    bounds_min, bounds_max = _world_bounds(glb)
    dimensions = tuple(bounds_max[index] - bounds_min[index] for index in range(3))
    if scale_contract is not None:
        measured = (
            dimensions[1]
            if scale_contract.metric == "height"
            else max(dimensions)
        )
        allowed = scale_contract.target_meters * scale_contract.tolerance
        if abs(measured - scale_contract.target_meters) > allowed:
            errors.append(
                f"world {scale_contract.metric} {measured:.3f}m outside "
                f"{scale_contract.target_meters:.3f}m +/- {allowed:.3f}m"
            )
        ground_tolerance = max(0.02, scale_contract.target_meters * 0.01)
        if scale_contract.grounded and abs(bounds_min[1]) > ground_tolerance:
            errors.append(
                f"ground anchor y={bounds_min[1]:.3f}m exceeds tolerance {ground_tolerance:.3f}m"
            )

    stats = GlbStats(
        path=path,
        meshes=len(glb.meshes or []),
        nodes=len(glb.nodes or []),
        materials=len(glb.materials or []),
        triangles=triangles,
        vertices=vertices,
        bounds_min=bounds_min,
        bounds_max=bounds_max,
        dimensions=dimensions,
    )
    return ValidationResult(stats, tuple(errors), tuple(warnings))
