# Model Tooling Rules

These instructions apply to every Python file under `tools/`.

## Target Style

- Build one coherent Sokpop-like toy world: compact silhouettes, chunky connected forms, flat or deliberately faceted shading, restrained bevels, matte materials, and one readable accent.
- Use the Hoplite assets as the quality reference for silhouette, deterministic detail, material role separation, object naming, preview support, and reproducible export. Do not copy their dark fantasy palette onto unrelated assets.
- Judge assets at the in-game camera scale. Passing GLB structure and triangle checks is necessary but not sufficient.
- Prefer 3 to 7 material roles per prop: base, secondary, dark/support, light/edge, and at most two accents. Keep emissive geometry below roughly 10 percent of the visible surface.
- Keep connected parts visibly overlapping. Reject floating handles, disconnected limbs, hidden sockets, accidental intersections, generic display bases, and detail that disappears in the model preview.

## Geometry Contract

- Work in meters and use Blender Z-up. Export glTF Y-up through `models_lib.export_glb`.
- Put placeable assets on the ground plane and center their footprint near the origin. Weapons may use a deliberate grip/pommel origin.
- Use snake_case names for files, objects, meshes, and materials.
- Prefer 6 to 12 sided cylinders/cones, one-segment bevels, and ico spheres with subdivision 1 or 2. Use denser geometry only when the silhouette visibly needs it.
- Keep each production GLB below the category budget in `model_style.py`; the hard repository ceiling is 5000 triangles.
- Seed every randomized detail with `random.Random(<stable integer>)`. Never use global random state or time-based seeds.

## Generator Architecture

- Register every production asset under exactly one generator in `model_catalog.py` before exporting it.
- Use `models_lib.py` for scene cleanup, materials, primitives, and export. Add reusable helpers there instead of copying Blender operators into another generator.
- Do not call `bpy.ops.export_scene.gltf` from a generator. Do not hard-code absolute paths or mutate `models_lib.OUT_DIR`.
- Do not write `MANIFEST.json` from a generator. Run `python tools/sync_model_manifests.py` after filenames stabilize.
- Keep generators deterministic and import-safe. Put execution behind `main()` and `if __name__ == "__main__"`.
- Retired generators must fail with a replacement command. They must never silently overwrite current assets.
- Temporary exports and previews must use `LK2_MODEL_OUTPUT_ROOT` or `.tmp/model-previews/`, not production paths outside an intentional build.

## Required Workflow

1. Run `python tools/audit_model_generators.py` before editing to identify ownership and legacy conflicts.
2. Modify the canonical generator or shared helper. Do not add a second generator for an existing asset.
3. Build through `python tools/model_pipeline.py build --asset <stem>` or the owning Blender script.
4. Run `python tools/model_pipeline.py validate`.
5. Run `just model-preview-shot <stem>` for changed assets and inspect the PNG.
6. Run `python tools/sync_model_manifests.py` only after the output set is final.

Completion requires generator audit, GLB structure validation, poly-budget validation, and rendered visual inspection. A script that only exports successfully is incomplete.
