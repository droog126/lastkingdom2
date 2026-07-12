

from __future__ import annotations

import math
import sys
from pathlib import Path

import bmesh
import bpy

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import models_lib


_MATERIAL_CACHE = {}


def new_object(name):
    
    me = bpy.data.meshes.new(name)
    ob = bpy.data.objects.new(name, me)
    bpy.context.collection.objects.link(ob)
    return ob

def add_box_bmesh(ob, loc, size):
    
    x, y, z = loc
    sx, sy, sz = size
    bm = bmesh.new()
    verts = [
        bm.verts.new((x,       y,       z)),
        bm.verts.new((x + sx,  y,       z)),
        bm.verts.new((x + sx,  y + sy,  z)),
        bm.verts.new((x,       y + sy,  z)),
        bm.verts.new((x,       y,       z + sz)),
        bm.verts.new((x + sx,  y,       z + sz)),
        bm.verts.new((x + sx,  y + sy,  z + sz)),
        bm.verts.new((x,       y + sy,  z + sz)),
    ]
    faces = [
        (0, 1, 2, 3),
        (5, 4, 7, 6),
        (0, 4, 5, 1),
        (2, 6, 7, 3),
        (0, 3, 7, 4),
        (1, 5, 6, 2),
    ]
    for face_verts in faces:
        bm.faces.new([verts[i] for i in face_verts])
    bm.to_mesh(ob.data)
    bm.free()

def set_color(ob, r, g, b, a=1.0):
    key = (r, g, b, a)
    material = _MATERIAL_CACHE.get(key)
    if material is None:
        suffix = "_".join(f"{round(component * 255):02x}" for component in key)
        material = models_lib.mat(f"animal_{suffix}", (r, g, b), roughness=0.9, alpha=a)
        _MATERIAL_CACHE[key] = material
    ob.data.materials.append(material)

def parent_to(parent, child, offset=(0, 0, 0)):
    
    child.parent = parent
    child.location = offset

def export_glb(name):
    

    roots = [o for o in bpy.data.objects if o.parent is None and o.name == name.title()]
    if not roots:
        print(f"  ✗ 未找到根对象 {name}")
        return
    root = roots[0]

    bpy.ops.object.select_all(action='DESELECT')
    for child in root.children_recursive:
        child.select_set(True)
    root.select_set(True)
    bpy.context.view_layer.objects.active = root

    bpy.ops.object.join()

    path = models_lib.export_glb(name, collection="animals")
    print(f"  ✓ {name}.glb  ({len(root.data.vertices)} vertices)")
    return path


def child_named(root, name):
    if root.name == name:
        return root
    return next((child for child in root.children_recursive if child.name == name), None)


def recolor_named(root, prefixes, color):
    for obj in [root, *root.children_recursive]:
        if obj.name.startswith(prefixes) and getattr(obj, "data", None) is not None:
            obj.data.materials.clear()
            set_color(obj, *color)


def build_pig():
    root = new_object("Pig")
    b = root

    body = new_object("Pig_Body")
    parent_to(b, body)
    add_box_bmesh(body, (-0.50, -0.30, 0.0), (1.00, 0.70, 0.60))
    add_box_bmesh(body, (-0.35, -0.20, 0.50), (0.70, 0.50, 0.20))
    set_color(body, 0.95, 0.70, 0.75)

    head = new_object("Pig_Head")
    parent_to(b, head, (0.40, 0.0, 0.15))
    add_box_bmesh(head, (0.0, -0.22, 0.0), (0.35, 0.45, 0.38))
    set_color(head, 0.95, 0.70, 0.75)

    nose = new_object("Pig_Nose")
    parent_to(head, nose, (0.32, -0.08, 0.08))
    add_box_bmesh(nose, (0.0, 0.0, 0.0), (0.10, 0.18, 0.18))
    set_color(nose, 0.85, 0.55, 0.60)

    for side, sign in [("L", -1), ("R", 1)]:
        ear = new_object(f"Pig_Ear_{side}")
        parent_to(head, ear, (0.05, sign * 0.17, 0.32))
        add_box_bmesh(ear, (0.0, 0.0, 0.0), (0.12, 0.10, 0.08))
        set_color(ear, 0.90, 0.65, 0.70)

    for i, (lx, lz) in enumerate([(-0.35,-0.22),(0.25,-0.22),(-0.35,0.12),(0.25,0.12)]):
        leg = new_object(f"Pig_Leg_{i}")
        parent_to(body, leg, (lx+0.05, lz, -0.35))
        add_box_bmesh(leg, (0.0, 0.0, 0.0), (0.18, 0.16, 0.35))
        set_color(leg, 0.90, 0.65, 0.70)

    tail = new_object("Pig_Tail")
    parent_to(body, tail, (-0.52, 0.05, 0.22))
    add_box_bmesh(tail, (0.0, 0.0, 0.0), (0.10, 0.08, 0.08))
    set_color(tail, 0.90, 0.65, 0.70)
    return root

def build_sheep():
    root = new_object("Sheep")

    body = new_object("Sheep_Body")
    parent_to(root, body)
    add_box_bmesh(body, (-0.50, -0.35, 0.05), (1.00, 0.70, 0.55))
    add_box_bmesh(body, (-0.45, -0.30, 0.00), (0.90, 0.60, 0.40))
    add_box_bmesh(body, (-0.30, -0.25, 0.55), (0.60, 0.50, 0.20))
    set_color(body, 0.95, 0.95, 0.92)

    head = new_object("Sheep_Head")
    parent_to(root, head, (0.45, 0.0, 0.22))
    add_box_bmesh(head, (0.0, -0.18, 0.0), (0.35, 0.36, 0.34))
    set_color(head, 0.15, 0.12, 0.10)

    nose = new_object("Sheep_Nose")
    parent_to(head, nose, (0.30, -0.05, 0.10))
    add_box_bmesh(nose, (0.0, 0.0, 0.0), (0.08, 0.12, 0.10))
    set_color(nose, 0.20, 0.15, 0.12)

    for side, sign in [("L", -1), ("R", 1)]:
        ear = new_object(f"Sheep_Ear_{side}")
        parent_to(head, ear, (0.05, sign*0.14, 0.28))
        add_box_bmesh(ear, (0.0, 0.0, 0.0), (0.12, 0.12, 0.06))
        set_color(ear, 0.15, 0.12, 0.10)

        horn = new_object(f"Sheep_Horn_{side}")
        parent_to(head, horn, (0.0, sign*0.12, 0.30))
        add_box_bmesh(horn, (0.0, 0.0, 0.0), (0.08, 0.08, 0.20))
        set_color(horn, 0.85, 0.80, 0.65)

    for i, (lx, lz) in enumerate([(-0.35,-0.25),(0.25,-0.25),(-0.35,0.15),(0.25,0.15)]):
        leg = new_object(f"Sheep_Leg_{i}")
        parent_to(body, leg, (lx+0.05, lz, -0.40))
        add_box_bmesh(leg, (0.0, 0.0, 0.0), (0.16, 0.16, 0.40))
        set_color(leg, 0.15, 0.12, 0.10)

    tail = new_object("Sheep_Tail")
    parent_to(body, tail, (-0.55, 0.05, 0.25))
    add_box_bmesh(tail, (0.0, 0.0, 0.0), (0.12, 0.14, 0.14))
    set_color(tail, 0.95, 0.95, 0.92)
    return root

def build_cow():
    root = new_object("Cow")

    body = new_object("Cow_Body")
    parent_to(root, body)
    add_box_bmesh(body, (-0.60, -0.38, 0.05), (1.20, 0.75, 0.65))
    set_color(body, 0.95, 0.95, 0.95)

    for i, (loc, sz) in enumerate([
        ((-0.30, -0.10, 0.65), (0.30, 0.25, 0.06)),
        (( 0.10, -0.30, 0.65), (0.20, 0.18, 0.06)),
        ((-0.50,  0.10, 0.65), (0.20, 0.20, 0.06)),
        (( 0.20,  0.05, 0.65), (0.15, 0.15, 0.06)),
    ]):
        spot = new_object(f"Cow_Spot_{i}")
        parent_to(body, spot, loc)
        add_box_bmesh(spot, (0.0, 0.0, 0.0), sz)
        set_color(spot, 0.12, 0.10, 0.08)

    head = new_object("Cow_Head")
    parent_to(root, head, (0.55, 0.0, 0.28))
    add_box_bmesh(head, (0.0, -0.22, 0.0), (0.40, 0.44, 0.40))
    set_color(head, 0.95, 0.95, 0.95)

    snout = new_object("Cow_Snout")
    parent_to(head, snout, (0.36, -0.10, 0.10))
    add_box_bmesh(snout, (0.0, 0.0, 0.0), (0.10, 0.22, 0.20))
    set_color(snout, 0.90, 0.70, 0.72)

    for side, sign in [("L", -1), ("R", 1)]:
        eye = new_object(f"Cow_Eye_{side}")
        parent_to(head, eye, (0.32, sign*0.14, 0.35))
        add_box_bmesh(eye, (0.0, 0.0, 0.0), (0.06, 0.06, 0.06))
        set_color(eye, 0.08, 0.06, 0.05)

        ear = new_object(f"Cow_Ear_{side}")
        parent_to(head, ear, (0.05, sign*0.16, 0.35))
        add_box_bmesh(ear, (0.0, 0.0, 0.0), (0.14, 0.12, 0.08))
        set_color(ear, 0.95, 0.95, 0.95)

        horn = new_object(f"Cow_Horn_{side}")
        parent_to(head, horn, (0.0, sign*0.20, 0.38))
        add_box_bmesh(horn, (0.0, 0.0, 0.0), (0.08, 0.08, 0.20))
        set_color(horn, 0.82, 0.78, 0.60)

    for i, (lx, lz) in enumerate([(-0.42,-0.28),(0.30,-0.28),(-0.42,0.18),(0.30,0.18)]):
        leg = new_object(f"Cow_Leg_{i}")
        parent_to(body, leg, (lx+0.05, lz, -0.50))
        add_box_bmesh(leg, (0.0, 0.0, 0.0), (0.18, 0.18, 0.50))
        set_color(leg, 0.95, 0.95, 0.95)

    udder = new_object("Cow_Udder")
    parent_to(body, udder, (-0.15, -0.08, -0.05))
    add_box_bmesh(udder, (0.0, 0.0, 0.0), (0.30, 0.20, 0.15))
    set_color(udder, 0.88, 0.65, 0.68)

    tail = new_object("Cow_Tail")
    parent_to(body, tail, (-0.65, 0.0, 0.30))
    add_box_bmesh(tail, (0.0, 0.0, 0.0), (0.08, 0.08, 0.40))
    set_color(tail, 0.95, 0.95, 0.95)
    return root

def build_chicken():
    root = new_object("Chicken")

    body = new_object("Chicken_Body")
    parent_to(root, body)
    add_box_bmesh(body, (-0.28, -0.22, 0.0), (0.56, 0.44, 0.42))
    set_color(body, 0.95, 0.90, 0.78)

    for side, sign in [("L", -1), ("R", 1)]:
        wing = new_object(f"Chicken_Wing_{side}")
        parent_to(body, wing, (-0.05, sign*0.22, 0.05))
        add_box_bmesh(wing, (0.0, 0.0, 0.0), (0.35, 0.08, 0.32))
        set_color(wing, 0.85, 0.78, 0.60)

    head = new_object("Chicken_Head")
    parent_to(root, head, (0.22, 0.0, 0.18))
    add_box_bmesh(head, (0.0, -0.16, 0.0), (0.28, 0.32, 0.32))
    set_color(head, 0.95, 0.88, 0.70)

    for i, pos in enumerate([(0.0,0.14,0.28),(0.10,0.14,0.28),(0.05,0.14,0.36)]):
        comb = new_object(f"Chicken_Comb_{i}")
        parent_to(head, comb, pos)
        add_box_bmesh(comb, (0.0, 0.0, 0.0), (0.12, 0.08, 0.12))
        set_color(comb, 0.85, 0.10, 0.10)

    wattle = new_object("Chicken_Wattle")
    parent_to(head, wattle, (0.22, -0.02, 0.08))
    add_box_bmesh(wattle, (0.0, 0.0, 0.0), (0.10, 0.12, 0.10))
    set_color(wattle, 0.80, 0.10, 0.10)

    beak = new_object("Chicken_Beak")
    parent_to(head, beak, (0.24, -0.04, 0.12))
    add_box_bmesh(beak, (0.0, 0.0, 0.0), (0.14, 0.14, 0.12))
    set_color(beak, 0.95, 0.82, 0.10)

    for side, sign in [("L", -1), ("R", 1)]:
        eye = new_object(f"Chicken_Eye_{side}")
        parent_to(head, eye, (0.20, sign*0.10, 0.24))
        add_box_bmesh(eye, (0.0, 0.0, 0.0), (0.05, 0.05, 0.05))
        set_color(eye, 0.05, 0.05, 0.05)

    for side, sign in [("L", -1), ("R", 1)]:
        leg = new_object(f"Chicken_Leg_{side}")
        parent_to(body, leg, (sign*0.12, -0.06, -0.28))
        add_box_bmesh(leg, (0.0, 0.0, 0.0), (0.10, 0.12, 0.28))
        set_color(leg, 0.88, 0.72, 0.20)
        for j in range(3):
            toe = new_object(f"Chicken_Toe_{side}_{j}")
            parent_to(leg, toe, (j*0.04-0.04, 0.02, -0.22))
            add_box_bmesh(toe, (0.0, 0.0, 0.0), (0.04, 0.08, 0.10))
            set_color(toe, 0.85, 0.68, 0.15)

    return root

def build_rabbit():
    root = new_object("Rabbit")

    body = new_object("Rabbit_Body")
    parent_to(root, body)
    add_box_bmesh(body, (-0.38, -0.28, 0.0), (0.76, 0.56, 0.52))
    set_color(body, 0.88, 0.86, 0.84)

    head = new_object("Rabbit_Head")
    parent_to(root, head, (0.32, 0.0, 0.18))
    add_box_bmesh(head, (0.0, -0.20, 0.0), (0.36, 0.40, 0.38))
    set_color(head, 0.88, 0.86, 0.84)

    nose = new_object("Rabbit_Nose")
    parent_to(head, nose, (0.32, -0.04, 0.12))
    add_box_bmesh(nose, (0.0, 0.0, 0.0), (0.08, 0.10, 0.10))
    set_color(nose, 0.95, 0.72, 0.75)

    for side, sign in [("L", -1), ("R", 1)]:
        eye = new_object(f"Rabbit_Eye_{side}")
        parent_to(head, eye, (0.26, sign*0.10, 0.30))
        add_box_bmesh(eye, (0.0, 0.0, 0.0), (0.06, 0.06, 0.06))
        set_color(eye, 0.55, 0.25, 0.25)

        ear = new_object(f"Rabbit_Ear_{side}")
        parent_to(head, ear, (0.05, sign*0.16, 0.34))
        add_box_bmesh(ear, (0.0, 0.0, 0.0), (0.12, 0.08, 0.55))
        set_color(ear, 0.88, 0.86, 0.84)

        inner = new_object(f"Rabbit_EarInner_{side}")
        parent_to(ear, inner, (0.02, 0.02, 0.08))
        add_box_bmesh(inner, (0.0, 0.0, 0.0), (0.08, 0.04, 0.42))
        set_color(inner, 0.92, 0.68, 0.72)

    for side, sign in [("L", -1), ("R", 1)]:
        fl = new_object(f"Rabbit_FrontLeg_{side}")
        parent_to(body, fl, (sign*0.22, -0.08, -0.25))
        add_box_bmesh(fl, (0.0, 0.0, 0.0), (0.16, 0.16, 0.25))
        set_color(fl, 0.85, 0.82, 0.80)

        hl = new_object(f"Rabbit_HindLeg_{side}")
        parent_to(body, hl, (sign*0.28, -0.08, -0.42))
        add_box_bmesh(hl, (0.0, 0.0, 0.0), (0.18, 0.18, 0.42))
        set_color(hl, 0.85, 0.82, 0.80)

    tail = new_object("Rabbit_Tail")
    parent_to(body, tail, (-0.42, 0.02, 0.18))
    add_box_bmesh(tail, (0.0, 0.0, 0.0), (0.16, 0.14, 0.14))
    set_color(tail, 0.95, 0.95, 0.95)
    return root

def build_deer():
    root = new_object("Deer")

    body = new_object("Deer_Body")
    parent_to(root, body)
    add_box_bmesh(body, (-0.62, -0.34, 0.10), (1.24, 0.68, 0.72))
    set_color(body, 0.58, 0.32, 0.14)

    chest = new_object("Deer_Chest")
    parent_to(body, chest, (0.34, 0.0, 0.34))
    add_box_bmesh(chest, (0.0, -0.25, 0.0), (0.34, 0.50, 0.56))
    set_color(chest, 0.70, 0.42, 0.20)

    head = new_object("Deer_Head")
    parent_to(root, head, (0.62, 0.0, 0.52))
    add_box_bmesh(head, (0.0, -0.22, 0.0), (0.40, 0.44, 0.42))
    set_color(head, 0.62, 0.35, 0.15)

    snout = new_object("Deer_Snout")
    parent_to(head, snout, (0.34, -0.08, 0.06))
    add_box_bmesh(snout, (0.0, 0.0, 0.0), (0.16, 0.24, 0.18))
    set_color(snout, 0.32, 0.16, 0.08)

    for side, sign in (("L", -1), ("R", 1)):
        eye = new_object(f"Deer_Eye_{side}")
        parent_to(head, eye, (0.28, sign * 0.18, 0.30))
        add_box_bmesh(eye, (0.0, 0.0, 0.0), (0.06, 0.06, 0.06))
        set_color(eye, 0.04, 0.03, 0.02)

        ear = new_object(f"Deer_Ear_{side}")
        parent_to(head, ear, (0.02, sign * 0.19, 0.38))
        add_box_bmesh(ear, (0.0, 0.0, 0.0), (0.18, 0.10, 0.10))
        set_color(ear, 0.48, 0.23, 0.10)

        antler = new_object(f"Deer_Antler_{side}")
        parent_to(head, antler, (-0.02, sign * 0.14, 0.46))
        add_box_bmesh(antler, (0.0, 0.0, 0.0), (0.07, 0.07, 0.42))
        set_color(antler, 0.78, 0.64, 0.42)
        for branch, offset in enumerate((0.12, 0.25)):
            tine = new_object(f"Deer_Antler_{side}_{branch}")
            parent_to(antler, tine, (0.05, sign * 0.02, offset))
            add_box_bmesh(tine, (0.0, 0.0, 0.0), (0.16, 0.06, 0.06))
            set_color(tine, 0.78, 0.64, 0.42)

    for index, (lx, ly) in enumerate(((-0.42, -0.24), (0.34, -0.24), (-0.42, 0.18), (0.34, 0.18))):
        leg = new_object(f"Deer_Leg_{index}")
        parent_to(body, leg, (lx, ly, -0.42))
        add_box_bmesh(leg, (0.0, 0.0, 0.0), (0.16, 0.16, 0.54))
        set_color(leg, 0.48, 0.24, 0.10)
        hoof = new_object(f"Deer_Hoof_{index}")
        parent_to(leg, hoof, (0.03, -0.02, -0.08))
        add_box_bmesh(hoof, (0.0, 0.0, 0.0), (0.18, 0.18, 0.08))
        set_color(hoof, 0.10, 0.07, 0.04)

    tail = new_object("Deer_Tail")
    parent_to(body, tail, (-0.66, 0.0, 0.48))
    add_box_bmesh(tail, (0.0, 0.0, 0.0), (0.12, 0.14, 0.22))
    set_color(tail, 0.92, 0.84, 0.64)
    return root

def build_fox():
    root = new_object("Fox")

    body = new_object("Fox_Body")
    parent_to(root, body)
    add_box_bmesh(body, (-0.48, -0.30, 0.10), (0.96, 0.60, 0.58))
    set_color(body, 0.82, 0.28, 0.08)

    belly = new_object("Fox_Belly")
    parent_to(body, belly, (0.08, -0.31, 0.16))
    add_box_bmesh(belly, (0.0, 0.0, 0.0), (0.58, 0.05, 0.34))
    set_color(belly, 0.98, 0.78, 0.48)

    head = new_object("Fox_Head")
    parent_to(root, head, (0.44, 0.0, 0.40))
    add_box_bmesh(head, (0.0, -0.20, 0.0), (0.38, 0.40, 0.38))
    set_color(head, 0.82, 0.28, 0.08)

    muzzle = new_object("Fox_Muzzle")
    parent_to(head, muzzle, (0.32, -0.06, 0.04))
    add_box_bmesh(muzzle, (0.0, 0.0, 0.0), (0.16, 0.22, 0.16))
    set_color(muzzle, 0.98, 0.78, 0.50)

    nose = new_object("Fox_Nose")
    parent_to(muzzle, nose, (0.14, -0.02, 0.08))
    add_box_bmesh(nose, (0.0, 0.0, 0.0), (0.06, 0.10, 0.08))
    set_color(nose, 0.08, 0.04, 0.03)

    for side, sign in (("L", -1), ("R", 1)):
        eye = new_object(f"Fox_Eye_{side}")
        parent_to(head, eye, (0.26, sign * 0.15, 0.28))
        add_box_bmesh(eye, (0.0, 0.0, 0.0), (0.06, 0.06, 0.06))
        set_color(eye, 0.04, 0.03, 0.02)

        ear = new_object(f"Fox_Ear_{side}")
        parent_to(head, ear, (-0.02, sign * 0.15, 0.40))
        add_box_bmesh(ear, (0.0, 0.0, 0.0), (0.16, 0.12, 0.30))
        set_color(ear, 0.72, 0.18, 0.06)

        ear_tip = new_object(f"Fox_EarTip_{side}")
        parent_to(ear, ear_tip, (0.02, 0.0, 0.28))
        add_box_bmesh(ear_tip, (0.0, 0.0, 0.0), (0.12, 0.10, 0.12))
        set_color(ear_tip, 0.12, 0.05, 0.03)

    for index, (lx, ly) in enumerate(((-0.30, -0.20), (0.24, -0.20), (-0.30, 0.16), (0.24, 0.16))):
        leg = new_object(f"Fox_Leg_{index}")
        parent_to(body, leg, (lx, ly, -0.36))
        add_box_bmesh(leg, (0.0, 0.0, 0.0), (0.14, 0.14, 0.40))
        set_color(leg, 0.82, 0.28, 0.08)

    tail = new_object("Fox_Tail")
    parent_to(body, tail, (-0.56, 0.02, 0.34))
    add_box_bmesh(tail, (0.0, 0.0, 0.0), (0.52, 0.20, 0.24))
    set_color(tail, 0.78, 0.24, 0.07)
    tail_tip = new_object("Fox_TailTip")
    parent_to(tail, tail_tip, (-0.20, 0.0, 0.02))
    add_box_bmesh(tail_tip, (0.0, 0.0, 0.0), (0.20, 0.22, 0.26))
    set_color(tail_tip, 0.98, 0.88, 0.68)
    return root


def build_deer_fawn():
    root = build_deer()
    root.name = "Deer_Fawn"
    for obj in list(root.children_recursive):
        if obj.name.startswith("Deer_Antler"):
            bpy.data.objects.remove(obj, do_unlink=True)
    recolor_named(root, ("Deer_Body", "Deer_Chest", "Deer_Head"), (0.76, 0.48, 0.24))
    recolor_named(root, ("Deer_Snout",), (0.42, 0.23, 0.12))
    recolor_named(root, ("Deer_Leg",), (0.58, 0.30, 0.14))

    body = child_named(root, "Deer_Body")
    if body is not None:
        for index, (x, y, z) in enumerate(((-0.30, -0.35, 0.62), (-0.02, -0.35, 0.68), (0.26, -0.35, 0.54), (-0.18, 0.34, 0.50))):
            spot = new_object(f"Deer_FawnSpot_{index}")
            parent_to(body, spot, (x, y, z))
            add_box_bmesh(spot, (0.0, 0.0, 0.0), (0.12, 0.035, 0.10))
            set_color(spot, 0.92, 0.76, 0.48)
    return root


def build_fox_silver():
    root = build_fox()
    root.name = "Fox_Silver"
    recolor_named(root, ("Fox_Body", "Fox_Head", "Fox_Ear", "Fox_Tail"), (0.42, 0.47, 0.52))
    recolor_named(root, ("Fox_Belly", "Fox_Muzzle", "Fox_TailTip"), (0.86, 0.88, 0.84))
    recolor_named(root, ("Fox_EarTip",), (0.18, 0.20, 0.23))
    return root


def build_rabbit_brown():
    root = build_rabbit()
    root.name = "Rabbit_Brown"
    recolor_named(root, ("Rabbit_Body", "Rabbit_Head", "Rabbit_FrontLeg", "Rabbit_HindLeg"), (0.58, 0.36, 0.20))
    recolor_named(root, ("Rabbit_Tail",), (0.82, 0.70, 0.56))
    recolor_named(root, ("Rabbit_EarInner",), (0.78, 0.46, 0.42))
    return root


if __name__ == "__main__":
    print("=" * 50)
    print("开始生成动物模型...")
    print(f"output: {models_lib.output_dir('animals')}")

    animals = [
        ("pig",     build_pig),
        ("sheep",   build_sheep),
        ("cow",     build_cow),
        ("chicken", build_chicken),
        ("rabbit",  build_rabbit),
        ("rabbit_brown", build_rabbit_brown),
        ("deer",    build_deer),
        ("deer_fawn", build_deer_fawn),
        ("fox",     build_fox),
        ("fox_silver", build_fox_silver),
    ]

    for name, builder in animals:
        print(f"\n[生成] {name} ...")
        try:
            models_lib.clear_scene()
            _MATERIAL_CACHE.clear()
            root = builder()
            bpy.ops.object.select_all(action='DESELECT')
            root.select_set(True)
            bpy.context.view_layer.objects.active = root
            export_glb(name)
        except Exception as e:
            import traceback
            print(f"  ✗ 错误: {e}")
            traceback.print_exc()

    print("\n✅ 全部完成！")
