---
name: ai-modeling
description: Reproducible 3D asset workflow for lastkingdom2. Use when creating, modifying, validating, previewing, or wiring Blender-generated GLBs, procedural models, tools/build_*.py generators, assets/procedural manifests, animals, terrain buildings, materials, scale, orientation, or poly budgets, including proving that the target gameplay path actually consumes the asset. Use closed-loop validation only after an asset is placed in the game.
---

# AI Modeling

## Contract

Generate committed 3D assets from deterministic scripts under `tools/`. Do not make an unrepeatable manual Blender edit and commit only the exported GLB.

## Visual Priorities

1. Establish silhouette, proportions, stance, contact points, and target-camera readability.
2. Ensure connected parts visibly connect and intersections look intentional.
3. Add one or two identity cues only after the base form reads correctly.
4. Use simple matte materials with clear role-based color separation.
5. Reject placeholder, debug, calibration, floating, or generic display-base geometry from production manifests.

Prefer a few readable primitives over many small details. Fix structural feedback directly instead of compensating with decoration or lighting.

## Workflow

1. Read `tools/AGENTS.md` and run `python tools/audit_model_generators.py`.
2. Find the asset's single owner in `tools/model_catalog.py`; do not add a competing generator.
3. Add reusable Blender operations to `tools/models_lib.py`, then update the deterministic owner script.
4. Build through `python tools/model_pipeline.py build --asset <stem>`.
5. Run `python tools/model_pipeline.py validate`, then sync manifests after filenames stabilize.
6. Produce a non-interactive preview and inspect the pixels for silhouette, scale, alignment, gaps, intersections, orientation, and materials.
7. Stabilize filenames and manifests before wiring repo-relative Bevy asset paths.
8. Before calling the asset wired, trace every requested runtime mode from its spawn/reconciliation system to the asset path. Treat catalog registration, manifest inclusion, preload lists, and model-preview visibility as discovery evidence only; they do not prove gameplay consumption. Search the actual consumer for a competing procedural or placeholder mesh and verify that it is not still selected for the target mode.
9. If the model is placed in gameplay, validate it in the target camera with `$closed-loop-ai-dev`.

If a retired asset can still be regenerated, update or disable its generator entry in the same change.

## Preview Commands

Use an auto-exiting command for agent validation:

```sh
just model-preview-shot <model-stem>
just model-preview-all-only <model-stem>
just model-optimize <collection>/<model-stem>
```

`just model-preview` is an interactive showroom and does not exit automatically. Run it only when the user wants an interactive window.

## Generation And Validation

Use `blender` from `PATH` or an explicitly configured local Blender executable:

```sh
blender --background --python tools/<generator>.py
python tools/audit_model_generators.py
python tools/validate_pretty_glbs.py
python tools/verify_poly_budget.py
python tools/model_pipeline.py validate
```

Run only commands applicable to the changed asset set. Do not commit temporary exports, preview screenshots, logs, Blender backups, `__pycache__`, or local absolute-path configuration unless explicitly requested.
