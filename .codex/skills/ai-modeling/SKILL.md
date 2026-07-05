---
name: ai-modeling
description: Reproducible AI 3D modeling workflow for lastkingdom2 assets. Use when creating, modifying, validating, or wiring Blender-generated GLB assets, procedural models, asset manifests, tools/build_*.py scripts, assets/procedural/pretty, assets/procedural/eco, animals, terrain buildings, poly budgets, or Bevy asset-server model references.
---

# AI Modeling

## Rule

Create and modify 3D models through reproducible scripts under `tools/`. Do not make unrepeatable manual Blender edits and commit only the exported result.

## Style Guardrails

For the Sokpop-style target, keep models simple, grounded, and structurally readable before adding detail.

- Start with the primary silhouette and proportions: head/body/limb scale, contact points, centerline, and stance must read correctly from the game camera before adding accessories.
- Prefer a few clear primitive forms over many small decorative pieces. If a model needs many small parts to communicate the idea, simplify the idea first.
- Ensure connected body parts visibly connect or deliberately overlap. Avoid floating heads, exposed gaps, hidden sockets, or large primitives intersecting in ways that look like something is stuffed inside the model.
- Use short, chunky, toy-like forms for characters. Avoid long thin necks, dangling limbs, overly tall torsos, or realistic anatomy that fights the simple style.
- Add identity details only after the base form works: one clear prop, one color accent, or one readable accessory is better than layered small features.
- Keep colors matte, moderately saturated, and separated by role. Do not rely on bright lighting, tiny texture-like marks, or subtle shading to make a form readable.
- When iterating after screenshot feedback, fix the structural problem directly instead of compensating with unrelated details.
- Generate a preview render or use the model preview system after export, and self-check for scale, alignment, visible gaps, unwanted intersections, over-brightness, and whether the model still reads when viewed small.

## Blender

Cross-platform command shape:

```sh
blender --background --python tools/build_all_models.py
```

On this Windows workstation, Blender is available as `F:\BLENDER\blender-launcher.exe`; use it as a local substitute for `blender` when needed. Use the same command shape for project-specific generators such as `tools/build_models_v4.py`, `tools/build_animals_blender.py`, or a new focused script.

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

```sh
python tools/validate_pretty_glbs.py
python tools/verify_poly_budget.py
```

For visual/model placement changes, also use `$closed-loop-ai-dev`.

## Do Not Commit

- `__pycache__/`
- temporary export files
- Blender auto-backup files
- local absolute-path configuration
- generated screenshots or logs unless explicitly requested
