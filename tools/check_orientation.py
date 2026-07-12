
import pygltflib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

NAMES = [
    ("sokpop_gatherer", "pretty"),
    ("rabbit", "eco"),
    ("monster_snake", "pretty"),
    ("sokpop_tree", "pretty"),
    ("cloud_puff", "pretty"),
]

for name, sub in NAMES:
    p = ROOT / "assets" / "procedural" / sub / f"{name}.glb"
    try:
        glb = pygltflib.GLTF2.load(str(p))
        for m in glb.meshes:
            for prim in m.primitives:
                pos = glb.accessors[prim.attributes.POSITION]
                mn, mx = pos.min, pos.max
                if name == "sokpop_gatherer":
                    print(f"{name}: min={[round(x,2) for x in mn]} max={[round(x,2) for x in mx]}")
                else:
                    print(f"{name}: min={[round(x,2) for x in mn]} max={[round(x,2) for x in mx]}")
                    break
            if name != "sokpop_gatherer": break
    except Exception as e:
        print(f"{name}: ERR {e}")
