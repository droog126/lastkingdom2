---
name: screenshot-scoring
description: Game screenshot scoring rubric for lastkingdom2 closed-loop AI iteration. Use when Codex needs to inspect loop PNGs, grade visual quality, compare current and previous screenshots, write score sections in decision.md, decide whether an iteration improved, diagnose black/blank/unreadable scenes, or choose the next visual/gameplay presentation change from screenshot evidence.
---

# Screenshot Scoring

## Purpose

Score screenshots as evidence, not vibes. Every number must be justified by visible pixels or by paired loop artifacts when state/progression matters.

Use this skill after `$closed-loop-ai-dev` identifies that PNG review is needed, and whenever writing the score section of `decision.md`.

## Required Inputs

Minimum:

- current screenshot PNG path
- current `health.json`

Preferred:

- current `assertions.json`
- current `final_state.json`
- current `diff.json`
- previous comparable screenshot and `diff.json`
- current task goal

If a required visual cannot be inspected, score only artifact-backed categories and mark visual categories as blocked with the reason.

## Scoring Workflow

1. Check hard gates before scoring.
2. Inspect the whole screenshot at normal view.
3. Zoom or crop only to confirm uncertain details.
4. Score each category from 0 to 10 using the anchors below.
5. Give one sentence of concrete evidence for every category.
6. Compare against the previous iteration if available.
7. Choose exactly one highest-value next change.

Do not give high scores for intent, code changes, generated assets, or expected behavior that is not visible in the screenshot.

## Hard Gates

Apply these caps before category scoring:

- Black screen, white screen, missing/invalid PNG, or camera inside geometry: total score max 2.0.
- No game world visible: total score max 2.5.
- World visible but player/camera target is lost and HUD cannot establish state: total score max 4.0.
- HUD or overlay covers the main scene enough to prevent visual judgment: total score max 5.0.
- `health.json` is `FAIL` for non-visual invariant/state reasons: gameplay score max 4 even if the screenshot looks good.
- Repeated severe log/assertion spam related to rendering, out-of-bounds, or voxel count: gameplay score max 5.

## Category Rubric

Score each category as an integer unless a half point is genuinely useful.

### Sky

- 0: no sky visible, black/white void, or broken clear color.
- 2: sky visible but flat, harsh, clipped, or visually confusing.
- 5: readable sky/background with acceptable contrast.
- 8: attractive sky/light balance that supports depth and scene readability.
- 10: polished sky, lighting, and horizon that strongly improve composition.

Check horizon, background color, fog, lighting direction, overexposure, and underexposure.

### Player

- 0: player absent or impossible to locate.
- 2: player barely visible, clipped, hidden, or indistinguishable from terrain.
- 5: player visible but lacks readable orientation, scale, or height cue.
- 8: player is clear, well framed, and has orientation/height context.
- 10: player is visually expressive, readable at a glance, and compositionally placed.

Check silhouette, contrast, camera framing, occlusion, feet/contact with ground, direction cue, head/body alignment, neck or overlap, and whether limbs look connected rather than floating or stuffed into the body.

### Terrain

- 0: terrain absent, broken, or camera is inside it.
- 2: terrain visible but noisy, flat, impossible to navigate, or out-of-bounds looking.
- 5: terrain is readable and spawn seems standable.
- 8: terrain has clear shapes, height variation, paths/landmarks, and depth.
- 10: terrain is polished, varied, navigable, and supports the intended fantasy.

Check terrain readability, spawn platform, cliffs/holes, edges, pathing, and scale. For environment props or model previews, penalize generic display-base presentation when the asset should read as an in-world object; prefer a convincing footprint, layered forms, and local detail that reads at target scale.

### Decor

- 0: no decor or all props invisible.
- 2: decor exists but is tiny, floating, clipped, repetitive, or visually noisy.
- 5: some trees/water/animals/monsters/props are visible and identifiable.
- 8: decor creates layered composition and makes the world feel inhabited.
- 10: decor is varied, well placed, thematic, and reinforces gameplay readability.

Check props, vegetation, water, actors, buildings, object scale, repetition, material assignment, and whether any asset still reads as placeholder, sample, debug, or calibration geometry instead of intentional content.
For ecology screenshots, verify variety is visually inspectable, not only counted in HUD/state. Penalize large foreground animals, blocks, trees, signs, or demo props that hide the player or the animal/plant/resource band.

### HUD

- 0: HUD missing when expected, unreadable, or corrupted.
- 2: HUD present but text overlaps, clips, or blocks important scene content.
- 5: HUD readable enough to understand basic state.
- 8: HUD is clean, stable, and does not obstruct important visuals.
- 10: HUD is polished, concise, legible, and supports the screenshot narrative.

Check text readability, overlap, contrast, anchoring, clipping, and important state visibility.

### Gameplay

Use screenshot plus `health.json`, `final_state.json`, and `diff.json`.

- 0: no progress, hard failure, invalid state, or out-of-bounds.
- 2: program alive but no understandable action or state.
- 5: basic auto-demo progress or resource/actor state is explainable.
- 8: action, resources, actors, and world state form a clear loop.
- 10: screenshot and artifacts show compelling, stable, self-explanatory gameplay.

Check movement, actor visibility, monster/animal/player state, resource changes, scenario progress, and observer assertions.
For finite ecology/resource loops, score higher only when artifacts show an explainable conversion chain, such as fruit eaten, food produced, regrowth consuming an input pool, and production stopping or capping when inputs are exhausted.

## Total Score

Compute:

```text
total = (sky + player + terrain + decor + hud + gameplay) / 6
```

Then apply hard-gate caps. Round to one decimal.

## Improvement Judgment

Compare to the previous comparable iteration:

- `improved`: total rises by at least 0.5, or the task-specific target category rises by at least 1 with no hard regression.
- `same`: total changes by less than 0.5 and no category changes by more than 1.
- `worse`: total drops by at least 0.5, a hard gate appears, or a key category drops by 2 or more.

Never mark `improved` if the screenshot looks better but `health.json` regressed to `FAIL`, unless the decision explicitly says visual improved while state worsened.

## Output Format

Use this format inside `decision.md`:

```markdown
score:
- sky: X/10 - <visible evidence>
- player: X/10 - <visible evidence>
- terrain: X/10 - <visible evidence>
- decor: X/10 - <visible evidence>
- hud: X/10 - <visible evidence>
- gameplay: X/10 - <artifact or visible evidence>
- total: X.X/10

vs_prev:
- visual: improved / same / worse - <specific comparison>
- state: <key delta from diff.json or "not compared">

next:
- <one change that targets the lowest important score or removes a hard gate>
```

## Decision Rules

- Fix hard gates before polishing.
- For generated assets, fix structural readability before adding decorative detail.
- If a screenshot or preview shows placeholder assets, poor material readability, wrong orientation, disconnected parts, or generic display-base presentation, score that as a concrete visual regression even when the asset is technically valid.
- If player is below 5, prioritize camera/player visibility over decor.
- If terrain is below 5, prioritize spawn framing, navigation readability, or camera placement.
- If HUD is below 5, prioritize legibility and non-overlap.
- If gameplay is below 5 with visual scores above 5, inspect state artifacts before changing visuals.
- If decor is below 5 but terrain/player/HUD are good, add or reposition visible props, animals, buildings, or water.
- If sky is below 5 and the scene is otherwise readable, tune lighting/fog/background after gameplay readability is stable.
- If state proves entities exist but the screenshot reads as "only berries" or "only one creature", prioritize camera corridor, spawn placement, scale, or occlusion fixes before adding more content.
