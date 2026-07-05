"""Read a GLB and print the actual mesh vertex range (no matrix_world)."""
import struct
import sys
import json
from pathlib import Path


def parse_glb(path):
    with open(path, "rb") as f:
        magic = f.read(4)
        version = struct.unpack("<I", f.read(4))[0]
        length = struct.unpack("<I", f.read(4))[0]
        while True:
            header = f.read(8)
            if len(header) < 8:
                break
            chunk_len, chunk_type = struct.unpack("<I4s", header)
            ct = chunk_type.decode("ascii", errors="ignore")
            chunk_data = f.read(chunk_len)
            if ct == "JSON":
                return chunk_data.decode("utf-8")
    return ""


glb = Path(sys.argv[-1])
json_str = parse_glb(glb)
data = json.loads(json_str)

print(f"GLB has {len(data.get('accessors', []))} accessors")
print(f"GLB has {len(data.get('meshes', []))} meshes")
print(f"GLB has {len(data.get('nodes', []))} nodes")
print()
print("Nodes (translation/rotation/scale/matrix):")
for i, node in enumerate(data.get("nodes", [])):
    print(
        f"  node[{i}] name={node.get('name','?')} "
        f"translation={node.get('translation')} "
        f"rotation={node.get('rotation')} "
        f"scale={node.get('scale')} "
        f"matrix={node.get('matrix')}"
    )

print()
print("Accessors with POSITION (componentType/bufferView min/max):")
for i, acc in enumerate(data.get("accessors", [])):
    if acc.get("type") == "VEC3":
        print(f"  acc[{i}] min={acc.get('min')} max={acc.get('max')}")