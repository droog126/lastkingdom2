
import struct
import sys
import json
from pathlib import Path

def triangle_count(glb_path: Path) -> tuple[int, int]:
    d = glb_path.read_bytes()
    if d[:4] != b"glTF":
        return (0, 0)
    jl = struct.unpack_from("<I", d, 12)[0]
    g = json.loads(d[20 : 20 + jl].decode('utf-8', errors='replace'))
    t = 0
    v = 0
    for m in g.get('meshes', []):
        for p in m.get('primitives', []):
            if p.get('indices') is not None:
                t += g['accessors'][p['indices']]['count'] // 3
            if p.get('attributes', {}).get('POSITION') is not None:
                v += g['accessors'][p['attributes']['POSITION']]['count']
    return t, v

def main() -> int:
    out_dir = Path(sys.argv[1] if len(sys.argv) > 1 else
                   r"F:\rustProject\lastkingdom2\assets\procedural\pretty")
    budget = 5000
    worst = 0
    fails = []
    print(f"v5-cute 预算: {budget} 面/模型")
    for p in sorted(out_dir.glob("*.glb")):
        t, v = triangle_count(p)
        if t > worst:
            worst = t
        if t > budget:
            fails.append((p.name, t))
        status = "OK " if t <= budget else "OVER"
        print(f"  {p.name:35s}  tris={t:5d}  verts={v:5d}  {status}")
    print(f"\nworst = {worst} tris, budget = {budget}")
    if fails:
        print(f"FAIL ({len(fails)} over budget):")
        for n, t in fails:
            print(f"  {n}: {t}")
        return 1
    print("PASS — all within budget")
    return 0

if __name__ == "__main__":
    sys.exit(main())
