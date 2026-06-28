
import pygltflib, json
glb = pygltflib.GLTF2.load(r"F:\rustProject\lastkingdom2\assets\procedural\pretty\player_avatar.glb")
print("scenes:", [(i, s.name) for i, s in enumerate(glb.scenes or [])])
print("nodes:")
for i, n in enumerate(glb.nodes or []):
    print(f"  [{i}] name={n.name} translation={n.translation} rotation={n.rotation} scale={n.scale}")
print("mesh[0] primitives[0] POSITION min/max:",
      glb.accessors[glb.meshes[0].primitives[0].attributes.POSITION].min,
      glb.accessors[glb.meshes[0].primitives[0].attributes.POSITION].max)

if glb.scenes:
    root_node_idx = glb.scenes[0].nodes[0] if glb.scenes[0].nodes else None
    print(f"root node idx = {root_node_idx}, root.translation = {glb.nodes[root_node_idx].translation if root_node_idx is not None else None}")
