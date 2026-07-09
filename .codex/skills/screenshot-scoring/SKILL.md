---
name: screenshot-scoring
description: Game screenshot scoring rubric for lastkingdom2 closed-loop AI iteration. Use when Codex needs to inspect loop PNGs, grade visual quality, compare current and previous screenshots, write score sections in decision.md, decide whether an iteration improved, diagnose black/blank/unreadable scenes, or choose the next visual/gameplay presentation change from screenshot evidence.
---

# Screenshot Scoring

## Purpose

Score screenshots as evidence, not vibes. Every number must be justified by visible pixels or by paired loop artifacts when state/progression matters.

Use this skill after `$closed-loop-ai-dev` identifies that PNG review is needed, and whenever writing the score section of `decision.md`.

## Required Inputs

Required:

- current screenshot PNG path
- current `health.json` (must include `assertions.failed`, `assertions.hard_failed`, `assertions.partial_failed`, `verdict`, and the `stderr` block: `deserialize_invalid_count`, `out_of_bounds_count`, `voxel_overflow_count`, `files_scanned`)
- current `regression.json` (must include `newly_failed`, `still_failing`, `newly_passed` — list every newly-failed assertion in the verdict)
- `.harness/KNOWN_ISSUES.md` "Open" section (must cross-check: did this iter fix any open issue?)

Preferred:

- current `assertions.json` (full assertion list with id + severity + actual + expected)
- current `final_state.json` (must include `camera.mode`, `camera.first_person_eye`, `monsters.current`, `nations.total_nations`, `network_command.move_world_sent`, `role`)
- current `diff.json` (`resource_deltas` for sim-motion evidence)
- previous comparable screenshot, `health.json`, and `diff.json`
- current task goal (from `decision.md` task: line or `task:` field)

If a required visual cannot be inspected, score only artifact-backed categories and mark visual categories as blocked with the reason.

## Scoring Workflow

1. **Read `health.json` first** — note `verdict` (PASS / PARTIAL / FAIL), `assertions.failed`/`hard_failed`/`partial_failed`, and the `stderr` block (any nonzero `deserialize_invalid_count` is a hard gate even if overall verdict is PASS).
2. **Read `regression.json`** — `newly_failed` list is the most important signal: anything that passed before but failed this iter is the priority finding. List every id in the verdict.
3. **Read `assertions.json`** — for any failed assertion, copy the exact `message` into the decision `problems:` section.
4. **Cross-check `KNOWN_ISSUES.md` Open** — for each open ISSUE-NNN, mark "verified fixed" / "still open" / "not addressed" in the decision `carryover:` section.
5. Check hard gates (next section) before scoring.
6. Inspect the whole screenshot at normal view.
7. Zoom or crop only to confirm uncertain details.
8. Score each category from 0 to 10 using the anchors below.
9. Give one sentence of concrete evidence for every category.
10. Compare against the previous iteration if available.
11. Choose exactly one highest-value next change.

Do not give high scores for intent, code changes, generated assets, or expected behavior that is not visible in the screenshot.

## Hard Gates

Apply these caps before category scoring:

- Black screen, white screen, missing/invalid PNG, or camera inside geometry: total score max 2.0.
- No game world visible: total score max 2.5.
- World visible but player/camera target is lost and HUD cannot establish state: total score max 4.0.
- HUD or overlay covers the main scene enough to prevent visual judgment: total score max 5.0.
- `health.json` is `FAIL` for non-visual invariant/state reasons: gameplay score max 4 even if the screenshot looks good.
- `health.json` is `PARTIAL`: gameplay score max 6 even if visuals look clean.
- Repeated severe log/assertion spam related to rendering, out-of-bounds, or voxel count: gameplay score max 5. Quantified thresholds (from `health.json::stderr` block, populated by `xtask/src/health.rs::scan_stderr_logs`):
  - `stderr.deserialize_invalid_count >= 1` is a hard gate (lightyear protocol registry mismatch floods logs at ~5ms cadence and blocks real packets — iter_199 root cause).
  - `stderr.out_of_bounds_count >= 5` is a hard gate (MoveTo retry / bounds clamping missing).
  - `stderr.voxel_overflow_count >= 5` is a hard gate (render warn throttle missing).
  - `stderr.files_scanned == 0` is a partial gate (no `.err.log` found at all means harness cannot detect runtime errors — run loop or fix log redirect).
- `assertions.json` has any `severity == "fail"` and `ok == false`: gameplay score max 4. Read `regression.json::newly_failed` to see what regressed this iter; list every newly-failed assertion by id in the decision verdict.
- `assertions.json` has any `severity == "partial"` and `ok == false`: gameplay score max 6.
- Network regression (online mode only): `network_command.move_world_sent == 0` after 30s+ is a hard gate (WASD pipeline dead — iter_200 root cause B).
- First-person task alignment: `decision.md` task says "first-person" but `final_state.json::camera.mode != "FirstPerson"` is a hard gate (iter_200 mode was ThirdPerson despite first-person task).
- Pretty landmark Y regression: `camera.first_person_eye[1] > 18` (or current `FIRST_PERSON_EYE_MAX_Y` constant) is a hard gate for any iter whose task mentions landmarks / kenney / sokpop decor — they spawn at eye-Y, not 19m in the air (iter_200 root cause C).

If a hard gate fires, fix the gate cause before scoring categories. Do not give a category above the cap.

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

Check silhouette, contrast, camera framing, occlusion, contact with ground, direction cue, major-part alignment, and whether connected parts look intentionally attached.

### Terrain

- 0: terrain absent, broken, or camera is inside it.
- 2: terrain visible but noisy, flat, impossible to navigate, or out-of-bounds looking.
- 5: terrain is readable and spawn seems standable.
- 8: terrain has clear shapes, height variation, paths/landmarks, and depth.
- 10: terrain is polished, varied, navigable, and supports the intended fantasy.

Check terrain readability, usable surfaces, edges, pathing, and scale. For environment props or model previews, penalize generic display-base presentation when the asset should read as an in-world object; prefer a convincing footprint, layered forms, and local detail that reads at target scale.

### Decor

- 0: no decor or all props invisible.
- 2: decor exists but is tiny, floating, clipped, repetitive, or visually noisy.
- 5: some environmental elements, actors, structures, or props are visible and identifiable.
- 8: decor creates layered composition and makes the world feel inhabited.
- 10: decor is varied, well placed, thematic, and reinforces gameplay readability.

Check props, environment elements, actors, structures, object scale, repetition, material assignment, and whether any asset still reads as placeholder, sample, debug, or calibration geometry instead of intentional content.
For runtime screenshots, verify important variety is visually inspectable, not only counted in UI or state. Penalize foreground or oversized elements that hide the focal subject or relevant interaction area.

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

Check movement, actor visibility, state changes, scenario progress, and observer assertions.
For stateful loops, score higher only when artifacts show an explainable chain: inputs change, outputs are produced within expected limits, and stop or cap conditions are honored.

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
- If decor is below 5 but terrain/player/HUD are good, add or reposition visible content that reinforces scene readability.
- If sky is below 5 and the scene is otherwise readable, tune lighting/fog/background after gameplay readability is stable.
- If state proves content exists but the screenshot does not make it visually inspectable, prioritize camera framing, placement, scale, or occlusion fixes before adding more content.
