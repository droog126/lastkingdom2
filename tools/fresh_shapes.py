"""Fresh collection-shape primitives for the reference-inspired toy world."""

from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from models_lib import (  # noqa: E402
    bevel_box,
    clear_scene,
    cone,
    cone_between,
    cube,
    cylinder,
    cylinder_between,
    export_glb,
    mat,
    prism,
    smooth_organic,
    shade_smooth,
    uv_sphere,
)


PALETTE = {
    "meadow": (0.70, 0.76, 0.42),
    "grass": (0.25, 0.39, 0.24),
    "deep_green": (0.10, 0.25, 0.19),
    "leaf": (0.31, 0.48, 0.25),
    "leaf_light": (0.55, 0.65, 0.30),
    "wood": (0.44, 0.24, 0.13),
    "wood_light": (0.86, 0.57, 0.30),
    "skin": (0.34, 0.24, 0.18),
    "teal": (0.06, 0.64, 0.67),
    "coral": (0.91, 0.34, 0.20),
    "yellow": (0.98, 0.68, 0.20),
    "purple": (0.52, 0.32, 0.62),
    "ink": (0.08, 0.07, 0.06),
    "cream": (0.87, 0.73, 0.48),
    "peach": (0.95, 0.48, 0.35),
    "stone": (0.40, 0.42, 0.40),
    "water": (0.20, 0.56, 0.62),
    "snow": (0.88, 0.88, 0.76),
    "fish_blue": (0.10, 0.45, 0.90),
    "fish_cyan": (0.18, 0.68, 0.95),
    "fish_deep": (0.06, 0.25, 0.58),
    "wolf_gray": (0.38, 0.40, 0.43),
    "wolf_light": (0.66, 0.64, 0.58),
    "wolf_dark": (0.20, 0.22, 0.25),
    "wolf_muzzle": (0.50, 0.47, 0.42),
    "wolf_eye": (0.95, 0.68, 0.18),
}

_MATERIAL_CACHE = {}

def fresh_mat(name: str, role: str):
    key = (name, role)
    if key not in _MATERIAL_CACHE:
        _MATERIAL_CACHE[key] = mat(f"fresh_{name}_{role}", PALETTE[role], roughness=0.94)
    return _MATERIAL_CACHE[key]


def blob(name, loc, scale, material, segments=20, rings=12):
    return uv_sphere(name, loc, scale, material, segments=segments, rings=rings)


def box(name, loc, scale, material, rotation=(0.0, 0.0, 0.0), bevel=0.0):
    if bevel > 0.0:
        return bevel_box(name, loc, scale, material, rotation=rotation, bevel=bevel)
    return cube(name, loc, scale, material, rotation=rotation)


def post(name, loc, radius, depth, material, vertices=8):
    return cylinder(name, loc, radius, depth, material, vertices=vertices)


def line(name, start, end, radius, material, vertices=7):
    return cylinder_between(name, start, end, radius, material, vertices=vertices)


def readable_eye(
    name,
    loc,
    pupil_material,
    *,
    size=(0.045, 0.028, 0.045),
    socket_role="cream",
):
    """Build a camera-readable toy eye on the near-facing side of a head."""
    x, y, z = loc
    sx, sy, sz = size
    blob(
        f"{name}_socket",
        (x, y + sy * 0.28, z),
        (sx * 1.55, sy * 1.20, sz * 1.55),
        fresh_mat("readable_eye", socket_role),
        8,
        5,
    )
    blob(
        f"{name}_pupil",
        (x + sx * 0.10, y - sy * 0.72, z),
        (sx * 0.72, sy * 0.52, sz * 0.72),
        pupil_material,
        8,
        5,
    )
    blob(
        f"{name}_glint",
        (x + sx * 0.28, y - sy * 1.02, z + sz * 0.30),
        (sx * 0.20, sy * 0.16, sz * 0.20),
        fresh_mat("readable_eye", "snow"),
        6,
        4,
    )


def wedge(name, loc, scale, material, rotation=0.0):
    obj = cone(name, loc, 1.0, 0.0, 1.0, material, vertices=4)
    obj.scale = scale
    obj.rotation_euler.z = rotation
    return obj


def disc(name, loc, radius, depth, material, x_scale=1.0, z_scale=1.0):
    obj = cylinder(name, loc, radius, depth, material, vertices=12)
    obj.scale.x = x_scale
    obj.scale.y = z_scale
    return obj


def reset():
    _MATERIAL_CACHE.clear()
    clear_scene()


def finish(name: str, collection: str):
    return export_glb(name, collection=collection)


def make_person(name: str, shirt_role: str = "teal"):
    skin = fresh_mat(name, "skin")
    shirt = fresh_mat(name, shirt_role)
    pants = fresh_mat(name, "ink")
    # No basket, backpack, tool, or legacy gathering prop.
    blob("body", (0.0, 0.0, 0.86), (0.23, 0.17, 0.20), shirt)
    blob("head", (0.0, 0.0, 1.24), (0.16, 0.14, 0.16), skin)
    line("leg_l", (-0.08, 0.0, 0.55), (-0.13, 0.06, 0.08), 0.028, pants)
    line("leg_r", (0.08, 0.0, 0.55), (0.13, 0.06, 0.08), 0.028, pants)
    line("arm_l", (-0.20, 0.0, 0.88), (-0.32, -0.01, 0.58), 0.028, skin)
    line("arm_r", (0.20, 0.0, 0.88), (0.32, -0.01, 0.58), 0.028, skin)
    readable_eye(
        "eye_l",
        (-0.055, -0.13, 1.27),
        fresh_mat(name, "ink"),
        size=(0.026, 0.016, 0.026),
        socket_role="cream",
    )
    readable_eye(
        "eye_r",
        (0.055, -0.13, 1.27),
        fresh_mat(name, "ink"),
        size=(0.026, 0.016, 0.026),
        socket_role="cream",
    )


def make_cart(name: str):
    """Build the playable cart with a clear local +X front silhouette."""
    wood = fresh_mat(name, "wood")
    wood_light = fresh_mat(name, "wood_light")
    dark = fresh_mat(name, "deep_green")
    iron = fresh_mat(name, "stone")
    cargo = fresh_mat(name, "yellow")
    cargo_light = fresh_mat(name, "cream")

    # A low underframe makes the wheel contact read clearly from the gameplay
    # camera, while the bed remains broad enough to carry the rider's cargo.
    box("underframe", (0.0, 0.0, 0.46), (1.42, 0.78, 0.12), dark, bevel=0.025)
    box("bed", (0.0, 0.0, 0.58), (1.55, 0.92, 0.16), wood, bevel=0.035)
    box("rail_near", (0.0, -0.47, 0.83), (1.58, 0.08, 0.28), wood_light, bevel=0.02)
    box("rail_far", (0.0, 0.47, 0.83), (1.58, 0.08, 0.28), wood_light, bevel=0.02)
    box("rail_back", (-0.76, 0.0, 0.83), (0.08, 0.92, 0.28), wood_light, bevel=0.02)
    box("rail_front", (0.76, 0.0, 0.75), (0.08, 0.92, 0.16), wood_light, bevel=0.02)

    for index, x in enumerate((-0.62, 0.62)):
        post(f"post_{index}", (x, 0.0, 0.86), 0.07, 0.72, dark, vertices=8)

    # Handles overlap the front rail so they remain connected at a three-quarter angle.
    box("handle_near", (1.02, -0.20, 0.72), (0.72, 0.08, 0.08), wood_light, bevel=0.02)
    box("handle_far", (1.02, 0.20, 0.72), (0.72, 0.08, 0.08), wood_light, bevel=0.02)
    box("handle_rise_near", (1.34, -0.20, 0.82), (0.08, 0.08, 0.24), wood_light, bevel=0.018)
    box("handle_rise_far", (1.34, 0.20, 0.82), (0.08, 0.08, 0.24), wood_light, bevel=0.018)
    box("handle_crossbar", (1.34, 0.0, 0.92), (0.10, 0.50, 0.08), iron, bevel=0.02)
    post("handle_cap", (1.38, 0.0, 0.92), 0.07, 0.12, iron, vertices=8)

    # Four visible wheel assemblies keep the cart grounded from either side.
    for side in (-1, 1):
        wheel_y = side * 0.54
        hub_y = side * 0.63
        for index, x in enumerate((-0.58, 0.58)):
            wheel = post(f"wheel_{side}_{index}", (x, wheel_y, 0.31), 0.32, 0.14, wood, vertices=14)
            wheel.rotation_euler.x = math.pi / 2.0
            hub = post(f"hub_{side}_{index}", (x, hub_y, 0.31), 0.10, 0.16, iron, vertices=8)
            hub.rotation_euler.x = math.pi / 2.0

    for index, x in enumerate((-0.58, 0.58)):
        box(f"axle_{index}", (x, 0.0, 0.31), (0.10, 1.18, 0.10), iron, bevel=0.018)

    blob("cargo_a", (-0.20, 0.0, 0.96), (0.30, 0.24, 0.20), cargo, 10, 6)
    blob("cargo_b", (0.32, 0.16, 0.98), (0.24, 0.20, 0.18), cargo_light, 10, 6)
    blob("cargo_c", (0.04, -0.20, 0.94), (0.22, 0.18, 0.16), cargo, 10, 6)


def make_tree(name: str, canopy_role: str = "leaf"):
    wood = fresh_mat(name, "wood")
    leaf = fresh_mat(name, canopy_role)
    dark = fresh_mat(name, "deep_green")

    if name == "granular_pine_tree":
        post("trunk", (0.0, 0.0, 1.45), 0.13, 2.90, wood, vertices=8)
        for index, (z, radius, height) in enumerate(
            ((1.65, 1.35, 1.05), (2.35, 1.08, 1.00), (3.05, 0.78, 0.92), (3.70, 0.46, 0.82))
        ):
            wedge(
                f"pine_tier_{index}",
                (0.0, 0.0, z),
                (radius, radius * 0.82, height),
                leaf if index % 2 else dark,
            )
        return

    if name == "granular_birch_tree":
        pale_trunk = fresh_mat(name, "cream")
        post("trunk", (0.0, 0.0, 1.55), 0.12, 3.10, pale_trunk, vertices=8)
        for index, x in enumerate((-0.40, 0.38)):
            line(f"branch_{index}", (0.0, 0.0, 2.25), (x, 0.02, 2.78), 0.07, pale_trunk, 7)
        blob("canopy_main", (0.0, 0.0, 3.05), (1.05, 0.72, 0.66), leaf, 16, 9)
        blob("canopy_left", (-0.70, 0.06, 2.82), (0.56, 0.45, 0.45), dark, 12, 8)
        blob("canopy_right", (0.68, -0.04, 3.02), (0.58, 0.46, 0.46), leaf, 12, 8)
        return

    if name == "granular_autumn_tree":
        autumn = fresh_mat(name, "coral")
        gold = fresh_mat(name, "yellow")
        post("trunk", (0.0, 0.0, 1.25), 0.14, 2.50, wood, vertices=8)
        for index, (x, y, z, scale, material) in enumerate(
            (
                (-0.72, 0.02, 2.42, (0.72, 0.55, 0.56), autumn),
                (0.0, 0.0, 2.72, (0.88, 0.64, 0.62), gold),
                (0.72, -0.03, 2.48, (0.70, 0.52, 0.56), autumn),
                (0.0, 0.04, 3.12, (0.56, 0.44, 0.48), gold),
            )
        ):
            blob(f"autumn_crown_{index}", (x, y, z), scale, material, 14, 8)
        return

    if name == "granular_willow_tree":
        post("trunk", (0.0, 0.0, 1.35), 0.15, 2.70, wood, vertices=8)
        blob("canopy_main", (0.0, 0.0, 2.72), (1.30, 0.82, 0.52), leaf, 16, 9)
        for index, (x, y, end_z) in enumerate(
            ((-0.92, -0.04, 1.55), (-0.48, -0.16, 1.38), (0.48, -0.12, 1.42), (0.92, 0.02, 1.62))
        ):
            line(f"willow_hang_{index}", (x * 0.72, y, 2.65), (x, y, end_z), 0.075, dark, 7)
            blob(f"willow_tip_{index}", (x, y, end_z - 0.10), (0.13, 0.11, 0.22), leaf, 10, 6)
        return

    if name == "palm":
        trunk_light = fresh_mat(name, "wood_light")
        post("trunk_base", (0.0, 0.0, 1.10), 0.15, 2.20, wood, vertices=8)
        line("trunk_top", (0.0, 0.0, 2.05), (0.16, 0.0, 3.25), 0.13, trunk_light, 8)
        crown = (0.16, 0.0, 3.25)
        for index, (x, y, z) in enumerate(
            ((-1.10, -0.05, 3.72), (-0.58, -0.25, 4.00), (0.0, -0.18, 4.12), (0.62, -0.14, 3.98), (1.10, 0.02, 3.66), (0.25, 0.30, 4.02))
        ):
            line(f"frond_{index}", crown, (x, y, z), 0.055, leaf, 7)
        blob("crown", crown, (0.22, 0.20, 0.20), dark, 10, 6)
        return

    if name == "granular_round_tree":
        post("trunk", (0.0, 0.0, 1.20), 0.13, 2.40, wood, vertices=8)
        for index, (x, y, z, scale) in enumerate(
            (
                (-0.78, 0.04, 2.42, (0.82, 0.60, 0.54)),
                (0.0, 0.0, 2.72, (0.98, 0.72, 0.62)),
                (0.78, -0.02, 2.42, (0.80, 0.58, 0.54)),
            )
        ):
            blob(f"round_crown_{index}", (x, y, z), scale, leaf if index != 0 else dark, 16, 9)
        return

    post("trunk", (0.0, 0.0, 1.15), 0.11, 2.3, wood)
    blob("canopy_main", (0.0, 0.0, 2.45), (1.30, 0.92, 0.56), leaf)
    blob("canopy_left", (-0.72, 0.08, 2.35), (0.78, 0.58, 0.42), dark)
    blob("canopy_right", (0.74, -0.10, 2.52), (0.76, 0.54, 0.40), leaf)


def make_animal(
    name: str,
    body_role: str,
    snout_role: str = "skin",
    *,
    body_loc=(0.0, 0.0, 0.62),
    body_scale=(0.46, 0.25, 0.28),
    head_loc=(0.38, -0.02, 0.88),
    head_scale=(0.28, 0.22, 0.25),
    snout_loc=(0.57, -0.18, 0.82),
    snout_scale=(0.15, 0.10, 0.10),
    leg_x=(-0.25, 0.25),
    leg_top=0.45,
    leg_radius=0.055,
    tail_start=(-0.40, 0.02, 0.72),
    tail_end=(-0.76, 0.04, 0.98),
    tail_radius=0.045,
    eye_loc=(0.46, -0.18, 0.98),
    eye_size=(0.045, 0.028, 0.045),
    belly_role="cream",
    leg_role=None,
    foot_role=None,
    foot_forward=0.08,
    tail_enabled=True,
):
    body = fresh_mat(name, body_role)
    snout = fresh_mat(name, snout_role)
    ink = fresh_mat(name, "ink")
    blob("body", body_loc, body_scale, body)
    blob("head", head_loc, head_scale, body)
    blob("snout", snout_loc, snout_scale, snout)
    blob(
        "belly",
        (body_loc[0] - 0.02, body_loc[1] - body_scale[1] * 0.84, body_loc[2] - 0.04),
        (body_scale[0] * 0.70, 0.038, body_scale[2] * 0.42),
        fresh_mat(name, belly_role),
        14,
        7,
    )
    leg_material = fresh_mat(name, leg_role or body_role)
    for index, x in enumerate(leg_x):
        line(f"leg_{index}", (x, 0.0, leg_top), (x, 0.0, 0.10), leg_radius, leg_material)
        if foot_role:
            foot_material = fresh_mat(name, foot_role)
            blob(
                f"foot_{index}",
                (x + foot_forward, -0.05, 0.07),
                (0.13, 0.10, 0.055),
                foot_material,
                8,
                5,
            )
            line(
                f"toe_{index}",
                (x + foot_forward * 0.75, -0.10, 0.055),
                (x + foot_forward + 0.16, -0.12, 0.05),
                0.024,
                foot_material,
                vertices=6,
            )
    readable_eye("eye", eye_loc, ink, size=eye_size)
    if tail_enabled:
        line("tail", tail_start, tail_end, tail_radius, body)


def make_wolf(name: str):
    """Build the animated wolf with a compact, readable low-poly silhouette.

    The eight leg object names are part of the client animation contract. Each
    segment is a real, short cylinder so the runtime IK can resize it along its
    local Y axis without turning a cube into a long floating spike.
    """
    fur = fresh_mat(name, "wolf_gray")
    fur_light = fresh_mat(name, "wolf_light")
    fur_dark = fresh_mat(name, "wolf_dark")
    muzzle = fresh_mat(name, "wolf_muzzle")
    eye = fresh_mat(name, "wolf_eye")
    ink = fresh_mat(name, "ink")

    body = blob("body", (0.0, 0.0, 0.70), (0.60, 0.30, 0.36), fur, 16, 10)
    smooth_organic(body, subdivision_levels=1)
    blob("belly", (-0.06, -0.02, 0.57), (0.47, 0.275, 0.17), fur_light, 14, 8)
    blob("chest", (0.34, 0.0, 0.76), (0.29, 0.29, 0.33), fur, 14, 8)

    # Keep the head at the source low-poly resolution. The body gets the one
    # organic smoothing pass; a faceted head preserves the wolf's graphic toy
    # read and leaves healthy triangle budget for animated limbs.
    blob("head", (0.52, 0.0, 0.96), (0.30, 0.26, 0.28), fur, 16, 10)
    blob("snout", (0.78, -0.015, 0.88), (0.20, 0.18, 0.14), muzzle, 12, 8)
    blob("snout_tip", (0.95, -0.02, 0.87), (0.065, 0.135, 0.08), ink, 8, 6)

    for side, label in ((-1.0, "near"), (1.0, "far")):
        readable_eye(
            f"eye_{label}",
            (0.65, side * 0.225, 1.04),
            eye,
            size=(0.050, 0.030, 0.050),
            socket_role="cream",
        )

    for side, label in ((-0.16, "near"), (0.16, "far")):
        ear = wedge(f"ear_{label}", (0.46, side, 1.20), (0.15, 0.13, 0.23), fur_dark)
        ear.rotation_euler.y = math.radians(-16.0)

    # The tail is deliberately two connected, faceted pieces: it reads at the
    # target camera and never becomes a detached floating triangle.
    line("tail_base", (-0.48, 0.02, 0.72), (-0.76, 0.04, 0.98), 0.11, fur_dark, 8)
    cone_between("tail_tip", (-0.76, 0.04, 0.98), (-0.94, 0.05, 1.16), 0.105, fur_dark, vertices=8)

    # Keep the names and part count aligned with quadruped_part_kind() in the
    # client. The source length is 0.20 m, matching apply_visual_segment().
    for x, prefix in ((0.28, "front"), (-0.28, "hind")):
        for side, side_name in ((0.18, "left"), (-0.18, "right")):
            line(
                f"leg_{prefix}_{side_name}_upper",
                (x, side, 0.30),
                (x, side, 0.50),
                0.085 if prefix == "front" else 0.10,
                fur,
                8,
            )
            line(
                f"leg_{prefix}_{side_name}_lower",
                (x, side, 0.10),
                (x, side, 0.30),
                0.075 if prefix == "front" else 0.09,
                fur_dark,
                8,
            )
            blob(
                f"paw_{prefix}_{side_name}",
                (x + 0.03, side, 0.075),
                (0.13, 0.10, 0.07),
                fur_dark,
                10,
                6,
            )


def make_goose(name: str):
    """Build the character showcase as a readable, mischievous toy goose.

    The silhouette does the work here: a heavy pear-shaped body, raised tail,
    long neck, wedge-like orange beak, and broad webbed feet.  The forms are
    deliberately simple and matte so the asset remains legible in the game's
    isometric model preview instead of turning into a detailed bird sculpture.
    """
    feather = fresh_mat(name, "snow")
    feather_shadow = fresh_mat(name, "cream")
    bill = fresh_mat(name, "peach")
    feet = fresh_mat(name, "yellow")
    ink = fresh_mat(name, "ink")

    blob("body", (0.0, 0.0, 0.70), (0.62, 0.36, 0.43), feather, 20, 12)
    blob("neck", (0.34, 0.0, 1.14), (0.18, 0.17, 0.38), feather_shadow, 16, 10)
    blob("head", (0.50, 0.0, 1.55), (0.28, 0.23, 0.26), feather, 18, 10)

    beak = cone_between("beak", (0.66, -0.01, 1.53), (1.16, -0.01, 1.53), 0.15, bill, vertices=12)
    beak["lk2_surface_role"] = "organic"
    shade_smooth(beak)
    for side in (-1.0, 1.0):
        readable_eye(
            f"eye_{'near' if side < 0 else 'far'}",
            (0.60, side * 0.20, 1.63),
            ink,
            size=(0.050, 0.030, 0.050),
            socket_role="snow",
        )

    # Rounded wing volumes keep the silhouette soft; the overlap is broad
    # enough that the body reads as one toy form instead of a paper cutout.
    blob("wing_near", (0.02, -0.29, 0.78), (0.40, 0.13, 0.31), feather, 20, 12)
    blob("wing_far", (0.02, 0.29, 0.78), (0.40, 0.13, 0.31), feather, 20, 12)
    tail = blob("tail", (-0.51, 0.02, 0.90), (0.36, 0.16, 0.20), feather, 18, 10)
    tail.rotation_euler.y = math.radians(-28.0)

    for index, x in enumerate((-0.18, 0.18)):
        line(f"leg_{index}", (x, 0.0, 0.48), (x, 0.0, 0.12), 0.055, feet, 10)
        box(f"foot_{index}", (x + 0.08, -0.06, 0.07), (0.27, 0.22, 0.08), feet, bevel=0.045)
        line(f"toe_{index}", (x + 0.10, -0.14, 0.06), (x + 0.28, -0.23, 0.055), 0.028, feet, 8)



def make_tree_cluster(name: str, roles: tuple[str, ...]):
    if name == "granular_reed_bank":
        stem = fresh_mat(name, "grass")
        leaf = fresh_mat(name, "leaf_light")
        for index, x in enumerate((-0.55, -0.28, 0.0, 0.30, 0.58)):
            height = 1.0 + (index % 3) * 0.18
            line(f"reed_{index}", (x, 0.0, 0.0), (x + 0.12, 0.0, height), 0.035, stem, 6)
            line(f"reed_leaf_{index}", (x + 0.12, 0.0, height * 0.58), (x + 0.28, -0.02, height * 0.78), 0.025, leaf, 6)
        return

    if name == "granular_wildflowers":
        stem = fresh_mat(name, "grass")
        for index, (x, y, z, role) in enumerate(
            ((-0.42, 0.0, 0.38, "coral"), (-0.20, -0.02, 0.52, "yellow"), (0.08, 0.02, 0.45, "purple"), (0.34, 0.0, 0.58, "peach"))
        ):
            line(f"flower_stem_{index}", (x, y, 0.0), (x, y, z), 0.018, stem, 6)
            blob(f"flower_head_{index}", (x, y, z), (0.11, 0.08, 0.09), fresh_mat(name, role), 8, 4)
        return

    wood = fresh_mat(name, "wood")
    for i, role in enumerate(roles):
        x = (i - (len(roles) - 1) * 0.5) * 0.45
        post(f"trunk_{i}", (x, 0.0, 0.65), 0.07, 1.3, wood)
        blob(f"crown_{i}", (x, 0.0, 1.45 + (i % 2) * 0.18), (0.48, 0.36, 0.34), fresh_mat(name, role))


def make_building(name: str, role: str = "cream", roof_role: str = "peach"):
    stone = fresh_mat(name, "stone")
    wood = fresh_mat(name, "wood")
    light_wood = fresh_mat(name, "wood_light")
    roof = fresh_mat(name, roof_role)
    signal = fresh_mat(name, "coral")
    water = fresh_mat(name, "water")

    if name == "watchtower":
        # A squat tower with a projecting lookout reads much better than the
        # shared house silhouette at the isometric target camera.
        post("tower", (0.0, 0.0, 2.0), 0.62, 4.0, stone, vertices=10)
        disc("lookout_floor", (0.0, 0.0, 3.55), 0.90, 0.20, wood, 1.0, 1.0)
        for index, angle in enumerate((0.0, math.pi * 0.5, math.pi, math.pi * 1.5)):
            line(
                f"lookout_rail_{index}",
                (math.cos(angle) * 0.72, math.sin(angle) * 0.72, 3.90),
                (math.cos(angle + math.pi * 0.5) * 0.72, math.sin(angle + math.pi * 0.5) * 0.72, 3.90),
                0.055,
                light_wood,
                6,
            )
        wedge("roof", (0.0, 0.0, 4.55), (0.92, 0.92, 0.72), roof)
        blob("signal", (0.0, 0.0, 4.95), (0.18, 0.18, 0.18), signal, 8, 5)
        return

    if name == "windmill":
        post("tower", (0.0, 0.0, 2.05), 0.78, 4.10, stone, vertices=8)
        wedge("cap", (0.0, 0.0, 4.35), (0.95, 0.95, 0.60), roof)
        hub = blob("hub", (0.0, -0.82, 2.65), (0.18, 0.12, 0.18), light_wood, 8, 5)
        hub.rotation_euler.x = math.pi * 0.5
        for index, angle in enumerate((0.0, math.pi * 0.5, math.pi, math.pi * 1.5)):
            blade = box(
                f"blade_{index}",
                (math.cos(angle) * 0.52, -0.84, 2.65 + math.sin(angle) * 0.52),
                (0.10, 0.05, 0.56),
                light_wood,
                rotation=(0.0, 0.0, angle),
                bevel=0.025,
            )
            blade.rotation_euler.y = math.pi * 0.5
        return

    if name == "lighthouse":
        post("tower", (0.0, 0.0, 3.0), 0.72, 6.0, stone, vertices=10)
        disc("balcony", (0.0, 0.0, 5.55), 0.98, 0.18, light_wood)
        post("lantern_room", (0.0, 0.0, 6.05), 0.42, 0.82, water, vertices=8)
        wedge("lantern_roof", (0.0, 0.0, 6.55), (0.66, 0.66, 0.38), roof)
        blob("beacon", (0.0, 0.0, 6.22), (0.20, 0.20, 0.20), signal, 8, 5)
        return

    if name == "shrine":
        box("platform", (0.0, 0.0, 0.18), (1.25, 1.05, 0.24), stone, bevel=0.05)
        for index, x in enumerate((-0.72, 0.72)):
            post(f"pillar_{index}", (x, 0.0, 1.25), 0.14, 2.10, stone, vertices=8)
        box("altar", (0.0, 0.0, 0.72), (0.62, 0.45, 0.26), light_wood, bevel=0.04)
        wedge("roof", (0.0, 0.0, 2.38), (1.05, 0.82, 0.46), roof)
        blob("relic", (0.0, -0.12, 1.02), (0.18, 0.14, 0.24), signal, 8, 5)
        return

    if name == "well":
        disc("basin", (0.0, 0.0, 0.20), 0.78, 0.34, stone, 1.0, 1.0)
        disc("water", (0.0, 0.0, 0.39), 0.53, 0.06, water, 1.0, 1.0)
        for index, x in enumerate((-0.62, 0.62)):
            post(f"roof_post_{index}", (x, 0.0, 1.10), 0.10, 1.50, wood, vertices=8)
        box("roof_beam", (0.0, 0.0, 1.78), (0.82, 0.18, 0.12), wood, bevel=0.03)
        wedge("roof", (0.0, 0.0, 2.05), (0.96, 0.70, 0.40), roof)
        line("winch", (-0.48, -0.12, 1.46), (0.48, -0.12, 1.46), 0.05, light_wood, 8)
        return

    if name == "forge":
        box("base", (0.0, 0.0, 0.45), (1.15, 0.92, 0.45), stone, bevel=0.05)
        box("hearth", (0.0, -0.48, 0.85), (0.58, 0.14, 0.52), wood, bevel=0.04)
        blob("hot_coal", (0.0, -0.64, 0.94), (0.28, 0.035, 0.16), signal, 8, 4)
        post("chimney", (0.38, 0.16, 1.75), 0.22, 2.20, stone, vertices=8)
        box("anvil", (-0.52, -0.18, 0.94), (0.28, 0.22, 0.12), light_wood, bevel=0.03)
        line("anvil_leg", (-0.52, -0.18, 0.82), (-0.52, -0.18, 0.48), 0.08, stone, 6)
        return

    if name == "bridge_stone":
        box("deck", (0.0, 0.0, 0.48), (3.50, 1.05, 0.22), stone, bevel=0.06)
        for side, label in ((-0.90, "near"), (0.90, "far")):
            line(f"rail_{label}", (-3.15, side, 0.95), (3.15, side, 0.95), 0.10, light_wood, 8)
            for index, x in enumerate((-2.7, 0.0, 2.7)):
                post(f"rail_post_{label}_{index}", (x, side, 0.70), 0.07, 0.72, stone, vertices=8)
        return

    if name == "pier":
        box("deck", (0.0, 0.0, 0.34), (3.30, 0.95, 0.18), light_wood, bevel=0.04)
        for index, x in enumerate((-2.65, 0.0, 2.65)):
            for side in (-0.62, 0.62):
                post(f"pier_post_{index}_{side}", (x, side, -0.18), 0.09, 1.10, wood, vertices=8)
        box("end_cap", (3.25, 0.0, 0.55), (0.18, 1.08, 0.18), stone, bevel=0.03)
        return

    if name == "fence":
        for index, x in enumerate((-1.0, 0.0, 1.0)):
            post(f"fence_post_{index}", (x, 0.0, 0.72), 0.09, 1.44, wood, vertices=8)
        box("rail_top", (0.0, 0.0, 1.08), (1.05, 0.08, 0.10), light_wood, bevel=0.025)
        box("rail_bottom", (0.0, 0.0, 0.52), (1.05, 0.08, 0.10), light_wood, bevel=0.025)
        return

    body = fresh_mat(name, role)
    roof_mat = fresh_mat(name, roof_role)
    box("body", (0.0, 0.0, 0.75), (1.35, 1.05, 1.35), body)
    wedge("roof", (0.0, 0.0, 1.65), (0.95, 0.82, 0.65), roof_mat, math.pi * 0.25)
    box("door", (0.0, -0.55, 0.48), (0.24, 0.04, 0.56), wood)
    blob("window", (0.37, -0.56, 0.92), (0.15, 0.03, 0.15), fresh_mat(name, "teal"), 8, 4)


def make_medieval_house(name: str):
    """A readable medieval building silhouette for the village set."""
    is_barn = name == "barn"
    is_tavern = name == "tavern"
    is_chapel = name == "chapel"
    width, depth, wall_height = {
        "house_small": (4.8, 3.7, 3.20),
        "tavern": (6.4, 4.8, 3.65),
        "barn": (7.2, 5.0, 3.80),
        "chapel": (5.0, 5.8, 4.30),
    }[name]
    wall = fresh_mat(name, "wood" if is_barn else "cream")
    roof = fresh_mat(name, "coral" if is_tavern else "peach")
    dark = fresh_mat(name, "deep_green")
    glass = fresh_mat(name, "teal")
    foundation = fresh_mat(name, "stone")
    box("foundation", (0.0, 0.0, 0.16), (width + 0.18, depth + 0.18, 0.32), foundation)
    box("walls", (0.0, 0.0, 0.32 + wall_height * 0.5), (width, depth, wall_height), wall)
    # Heavy timber framing makes the medieval construction readable even
    # after the asset is normalized for the target camera.
    for x in (-width * 0.40, width * 0.40):
        box(f"front_post_{x}", (x, -depth * 0.525, 0.32 + wall_height * 0.5),
            (0.18, 0.10, wall_height + 0.08), foundation, bevel=0.025)
    box("front_beam", (0.0, -depth * 0.53, 0.32 + wall_height * 0.82),
        (width * 0.88, 0.11, 0.18), foundation, bevel=0.025)
    roof_base = 0.32 + wall_height
    roof_points = [(-width * 0.62, roof_base), (0.0, roof_base + 0.92), (width * 0.62, roof_base)]
    prism("gabled_roof", roof_points, depth + 0.28, roof, y=0.0, bevel=0.035)
    box("front_door", (0.0, -depth * 0.5 - 0.035, 0.90), (0.58, 0.07, 1.45), dark, bevel=0.025)
    if is_barn:
        box("barn_crossbar", (0.0, -depth * 0.52, 1.40), (0.92, 0.08, 0.10), foundation, bevel=0.02)
        box("barn_crossbar_diag", (0.0, -depth * 0.525, 1.12), (0.10, 0.08, 0.82), foundation, bevel=0.02, rotation=(0.0, 0.0, math.radians(38)))
    else:
        for index, x in enumerate((-width * 0.28, width * 0.28)):
            box(f"window_{index}", (x, -depth * 0.52, 1.65), (0.58, 0.06, 0.52), glass, bevel=0.025)
            box(f"window_bar_{index}", (x, -depth * 0.56, 1.65), (0.07, 0.02, 0.52), foundation, bevel=0.01)
    if is_chapel:
        wedge("chapel_spire", (0.0, 0.0, roof_base + 1.06), (0.34, 0.34, 0.72), roof)
        box("chapel_cross", (0.0, -depth * 0.03, roof_base + 1.52), (0.08, 0.08, 0.36), foundation)
    else:
        box("chimney", (width * 0.28, depth * 0.10, roof_base + 0.38), (0.25, 0.25, 0.88), foundation, bevel=0.025)
    if is_tavern:
        box("tavern_sign_post", (width * 0.48, -depth * 0.53, 0.85), (0.08, 0.08, 1.0), foundation)
        box("tavern_sign", (width * 0.48, -depth * 0.57, 1.34), (0.56, 0.08, 0.34), roof, bevel=0.025)


def make_prop(name: str, kind: str):
    wood = fresh_mat(name, "wood")
    light = fresh_mat(name, "wood_light")
    accent = fresh_mat(name, "coral")
    stone = fresh_mat(name, "stone")
    if kind == "campfire":
        for i in range(5):
            a = i * math.pi / 5
            line(f"log_{i}", (math.cos(a) * 0.28, math.sin(a) * 0.28, 0.12), (-math.cos(a) * 0.28, -math.sin(a) * 0.28, 0.12), 0.06, wood)
        blob("flame", (0.0, 0.0, 0.52), (0.20, 0.16, 0.38), accent, 8, 5)
    elif kind in {"rock", "resource_stone", "tombstone"}:
        blob("stone", (0.0, 0.0, 0.35), (0.48, 0.35, 0.32), stone)
        if kind == "tombstone":
            box("marker", (0.0, 0.0, 0.68), (0.30, 0.12, 0.48), stone)
    elif kind == "crate":
        box("container", (0.0, 0.0, 0.42), (0.72, 0.62, 0.72), wood, bevel=0.05)
        box("cross_near_h", (0.0, -0.65, 0.42), (0.62, 0.035, 0.08), light, bevel=0.015)
        box("cross_near_v", (0.0, -0.66, 0.42), (0.08, 0.035, 0.62), light, bevel=0.015)
    elif kind == "barrel":
        post("body", (0.0, 0.0, 0.48), 0.48, 0.96, wood, vertices=12)
        disc("top", (0.0, 0.0, 0.98), 0.38, 0.05, light, 1.0, 1.0)
        disc("bottom", (0.0, 0.0, 0.02), 0.38, 0.05, light, 1.0, 1.0)
        for index, z in enumerate((0.24, 0.72)):
            box(f"band_{index}", (0.0, -0.46, z), (0.43, 0.035, 0.06), light, bevel=0.015)
    elif kind == "chest":
        box("lower", (0.0, 0.0, 0.36), (0.76, 0.62, 0.52), wood, bevel=0.05)
        box("lid", (0.0, 0.0, 0.82), (0.80, 0.66, 0.14), light, rotation=(math.radians(-8.0), 0.0, 0.0), bevel=0.045)
        box("lock", (0.0, -0.68, 0.55), (0.11, 0.035, 0.14), accent, bevel=0.02)
    elif kind == "cauldron":
        disc("bowl", (0.0, 0.0, 0.45), 0.56, 0.34, stone, 1.0, 1.0)
        disc("liquid", (0.0, -0.01, 0.64), 0.42, 0.05, fresh_mat(name, "water"), 1.0, 1.0)
        for index, x in enumerate((-0.38, 0.38)):
            line(f"leg_{index}", (x, 0.0, 0.36), (x * 1.12, 0.0, 0.05), 0.07, stone, 7)
        line("rim", (-0.48, 0.0, 0.70), (0.48, 0.0, 0.70), 0.055, light, 8)
    elif kind == "cooking_station":
        box("table", (0.0, 0.0, 0.54), (0.90, 0.58, 0.16), wood, bevel=0.04)
        for index, x in enumerate((-0.68, 0.68)):
            line(f"leg_{index}", (x, 0.0, 0.45), (x, 0.0, 0.08), 0.08, wood, 7)
        post("pot", (0.0, -0.02, 0.92), 0.24, 0.30, stone, vertices=10)
        blob("food", (0.0, -0.02, 1.10), (0.15, 0.15, 0.08), accent, 8, 4)
        line("rack", (-0.72, 0.0, 1.28), (0.72, 0.0, 1.28), 0.045, light, 7)
    elif kind == "spit_roast":
        for index, x in enumerate((-0.62, 0.62)):
            post(f"stand_{index}", (x, 0.0, 0.72), 0.08, 1.44, wood, vertices=8)
        line("spit", (-0.78, 0.0, 1.15), (0.78, 0.0, 1.15), 0.045, stone, 7)
        roast = blob("roast", (0.0, 0.0, 1.15), (0.46, 0.22, 0.25), accent, 12, 8)
        smooth_organic(roast, subdivision_levels=1)
        for index, x in enumerate((-0.26, 0.26)):
            line(f"roast_mark_{index}", (x, -0.23, 1.15), (x, -0.25, 1.30), 0.025, light, 6)
    elif kind == "market_stall":
        box("counter", (0.0, -0.02, 0.62), (1.05, 0.58, 0.16), wood, bevel=0.035)
        for index, x in enumerate((-0.84, 0.84)):
            post(f"stall_post_{index}", (x, 0.0, 1.10), 0.07, 2.20, wood, vertices=8)
        box("awning", (0.0, 0.0, 2.10), (1.10, 0.70, 0.12), accent, bevel=0.04)
        for index, x in enumerate((-0.58, 0.0, 0.58)):
            blob(f"goods_{index}", (x, -0.42, 0.88), (0.17, 0.15, 0.16), light, 8, 5)
    elif kind == "bench":
        box("seat", (0.0, 0.0, 0.62), (0.92, 0.28, 0.12), light, bevel=0.035)
        box("back", (0.0, 0.20, 1.02), (0.92, 0.10, 0.40), wood, bevel=0.03)
        for index, x in enumerate((-0.68, 0.68)):
            line(f"leg_{index}", (x, 0.0, 0.52), (x, 0.0, 0.10), 0.07, stone, 7)
    elif kind == "lantern_post":
        post("post", (0.0, 0.0, 1.15), 0.07, 2.30, wood, vertices=8)
        box("lantern_frame", (0.0, -0.02, 2.28), (0.28, 0.24, 0.34), stone, bevel=0.035)
        blob("lantern_glow", (0.0, -0.27, 2.28), (0.13, 0.025, 0.16), accent, 8, 5)
        wedge("lantern_cap", (0.0, -0.02, 2.66), (0.38, 0.32, 0.16), light)
    elif kind == "boat":
        wedge("hull", (0.0, 0.0, 0.38), (1.30, 0.62, 0.38), wood)
        box("rim_near", (0.0, -0.54, 0.62), (1.10, 0.06, 0.08), light, bevel=0.02)
        box("rim_far", (0.0, 0.54, 0.62), (1.10, 0.06, 0.08), light, bevel=0.02)
        box("seat", (0.0, 0.0, 0.70), (0.52, 0.48, 0.08), light, bevel=0.02)
        post("mast", (-0.50, 0.0, 1.15), 0.045, 1.10, wood, vertices=7)
        wedge("sail", (-0.28, 0.0, 1.35), (0.42, 0.04, 0.52), fresh_mat(name, "cream"))
    elif kind == "arch_stone":
        for index, x in enumerate((-0.86, 0.86)):
            box(f"pillar_{index}", (x, 0.0, 0.82), (0.32, 0.60, 0.82), stone, bevel=0.05)
        box("lintel", (0.0, 0.0, 1.72), (1.18, 0.62, 0.25), stone, bevel=0.05)
        wedge("keystone", (0.0, -0.34, 1.88), (0.28, 0.12, 0.24), light)
    elif kind == "haystack":
        blob("base", (0.0, 0.0, 0.55), (0.70, 0.52, 0.55), light, 12, 8)
        blob("top", (0.0, 0.0, 1.05), (0.42, 0.34, 0.38), light, 12, 8)
        line("tie", (-0.42, -0.48, 0.62), (0.42, -0.48, 0.62), 0.035, wood, 7)
    elif kind == "statue":
        box("pedestal", (0.0, 0.0, 0.28), (0.42, 0.42, 0.28), stone, bevel=0.04)
        blob("body", (0.0, 0.0, 0.78), (0.25, 0.20, 0.40), stone, 12, 8)
        blob("head", (0.0, 0.0, 1.24), (0.18, 0.16, 0.18), light, 10, 6)
        line("arm_l", (-0.18, 0.0, 0.88), (-0.42, -0.02, 0.70), 0.055, stone, 7)
        line("arm_r", (0.18, 0.0, 0.88), (0.42, -0.02, 0.70), 0.055, stone, 7)
    elif kind == "sign":
        post("post", (0.0, 0.0, 0.62), 0.035, 1.24, wood, 6)
        box("board", (0.0, -0.03, 1.13), (0.62, 0.06, 0.32), light)
        box("mark", (0.0, -0.07, 1.13), (0.36, 0.02, 0.035), fresh_mat(name, "deep_green"))
    elif kind == "fountain":
        disc("basin", (0.0, 0.0, 0.18), 0.60, 0.28, stone)
        post("water", (0.0, 0.0, 0.62), 0.14, 0.85, fresh_mat(name, "water"), 8)
    else:
        box("base", (0.0, 0.0, 0.30), (0.90, 0.72, 0.48), wood)
        blob("accent", (0.0, -0.38, 0.65), (0.22, 0.04, 0.18), accent, 8, 4)
