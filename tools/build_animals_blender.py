

import bpy
import math
import os
import bmesh

OUTPUT_DIR = "F:/rustProject/lastkingdom2/assets/animals"

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
    
    mat = bpy.data.materials.new(name=f"Mat_{ob.name}")

    nt = mat.node_tree

    for n in list(nt.nodes):
        nt.nodes.remove(n)
    bsdf = nt.nodes.new("ShaderNodeBsdfPrincipled")
    bsdf.location = (0, 0)
    bsdf.inputs["Base Color"].default_value = (r, g, b, a)
    bsdf.inputs["Roughness"].default_value = 0.9
    bsdf.inputs["Specular IOR Level"].default_value = 0.0
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    out.location = (200, 0)
    nt.links.new(bsdf.outputs["BSDF"], out.inputs["Surface"])
    ob.data.materials.append(mat)

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

    path = os.path.join(OUTPUT_DIR, f"{name}.glb")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=path,
        export_format='GLB',
        use_selection=True,
        export_materials='EXPORT',
    )
    print(f"  ✓ {name}.glb  ({len(root.data.vertices)} vertices)")
    return path

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

if __name__ == "__main__":
    print("=" * 50)
    print("开始生成动物模型...")
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    print(f"输出目录: {OUTPUT_DIR}")

    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete()
    for me in list(bpy.data.meshes):
        bpy.data.meshes.remove(me)
    for mat in list(bpy.data.materials):
        bpy.data.materials.remove(mat)

    animals = [
        ("pig",     build_pig),
        ("sheep",   build_sheep),
        ("cow",     build_cow),
        ("chicken", build_chicken),
        ("rabbit",  build_rabbit),
    ]

    for name, builder in animals:
        print(f"\n[生成] {name} ...")
        try:
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
