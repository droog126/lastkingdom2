---
name: ai-modeling
description: Reproducible AI 3D modeling workflow for lastkingdom2 assets. Use when creating, modifying, validating, or wiring Blender-generated GLB assets, procedural models, asset manifests, tools/build_*.py scripts, assets/procedural/pretty, assets/procedural/eco, animals, terrain buildings, poly budgets, or Bevy asset-server model references.
---

# AI Modeling

## Rule

Create and modify 3D models through reproducible scripts under `tools/`. Do not make unrepeatable manual Blender edits and commit only the exported result.

## Blender

Launcher:

```powershell
& "F:\BLENDER\blender-launcher.exe" --background --python tools\build_all_models.py
```

Use the same command shape for project-specific generators such as `tools\build_models_v4.py`, `tools\build_animals_blender.py`, or a new focused script.

## Asset Placement

- Prefer new generated models in `assets/procedural/pretty/` or `assets/procedural/eco/`.
- Put generation scripts in `tools/`.
- Update the corresponding `MANIFEST.json` when adding, replacing, or renaming generated assets.
- Reference models in Rust using Bevy asset-server repo-relative paths, never local absolute paths.

## Workflow

1. Inspect existing nearby model scripts and manifests.
2. Add or update a deterministic Python generator in `tools/`.
3. Generate assets with Blender in background mode.
4. Validate exported GLBs and poly budgets.
5. Wire assets in code only after filenames and manifest entries are stable.
6. Run a closed-loop iteration when the change affects visible game output.

## Validation

Run these after generation when they apply:

```powershell
python tools\validate_pretty_glbs.py
python tools\verify_poly_budget.py
```

For visual/model placement changes, also use `$closed-loop-ai-dev`.

## Do Not Commit

- `__pycache__/`
- temporary export files
- Blender auto-backup files
- local absolute-path configuration
- generated screenshots or logs unless explicitly requested
