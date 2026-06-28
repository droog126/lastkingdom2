
import pygltflib

NAMES = [
    ("player_avatar", "pretty"),
    ("rabbit", "eco"),
    ("monster_snake", "eco"),
    ("tree", "pretty"),
    ("cloud_puff", "pretty"),
]

for name, sub in NAMES:
    p = rf"F:\rustProject\lastkingdom2\assets\procedural\{sub}\{name}.glb"
    try:
        glb = pygltflib.GLTF2.load(p)
        for m in glb.meshes:
            for prim in m.primitives:
                pos = glb.accessors[prim.attributes.POSITION]
                mn, mx = pos.min, pos.max
                if name == "player_avatar":
                    print(f"{name}: min={[round(x,2) for x in mn]} max={[round(x,2) for x in mx]}")
                else:
                    print(f"{name}: min={[round(x,2) for x in mn]} max={[round(x,2) for x in mx]}")
                    break
            if name != "player_avatar": break
    except Exception as e:
        print(f"{name}: ERR {e}")
