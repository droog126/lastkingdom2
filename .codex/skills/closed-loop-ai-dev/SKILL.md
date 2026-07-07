---
name: closed-loop-ai-dev
description: Closed-loop AI iteration workflow for lastkingdom2. Use when tasks affect visuals, gameplay feel, client experience, offline auto-demo behavior, screenshots, scenario progression, observer health, xtask loop, or when the user asks for AI self-iteration, observe-decide-act, loop runs, screenshot review, health.json analysis, or decision.md.
---

# Closed Loop AI Dev

## Contract

The loop is observe -> decide -> act -> build -> re-run. A loop is not complete until the run artifacts are inspected and `screenshots/iter_NN/decision.md` records the result.

`xtask loop` enforces this: before starting a new iteration, the previous `screenshots/iter_NN/decision.md` must exist. After a run it writes `decision.template.md` for the latest iteration.

Use this skill for visual, gameplay, client UX, offline demo, scenario, screenshot, and automated observation changes. Use `$screenshot-scoring` whenever inspecting PNGs or writing the score section of `decision.md`.

For model or visual asset work, do not treat successful generation, file validation, or build success as visual completion. Inspect the rendered preview or target runtime view before calling the iteration done.

## Run

```sh
just loop
```

Direct form:

```sh
just xtask loop --offline --seconds 60
```

Quickly find latest iterations:

```sh
just health
```

## Artifact Order

Always inspect in this order:

1. Read `screenshots/iter_NN/health.json` first. It is the primary verdict and is far cheaper than PNG inspection.
2. If `health.json` is `PARTIAL` or `FAIL`, read `assertions.json` next and locate the failing assertion.
3. Read `final_state.json` and `diff.json` only when needed to explain state or progression.
4. Open the PNG only when health or assertions point to visual readability, framing, black screen, bad image, hidden player, HUD, or terrain issues.
5. Use `$screenshot-scoring` to assign evidence-backed category scores.
6. Write or update `decision.md`.

For asset-only iterations, use the same observe -> decide -> act discipline with a rendered preview as the visual artifact. Record concrete visual problems such as placeholder-looking geometry, poor material readability, wrong orientation, disconnected parts, or production manifests containing non-production assets.

## Health Meaning

- `PASS`: screenshot readable, final state parseable, tick threshold reached, observer invariants pass, auto-demo made basic progress.
- `PARTIAL`: process ran but did not meet the full loop contract, usually tick/progression shortfall.
- `FAIL`: hard failure such as black screen, bad image, missing state, observer error, invariant failure, out of bounds, or unreadable visual.

## Timing And Executables

- A source change is not present in the loop if validation runs stale build output. Build the affected executable or artifact first after gameplay or presentation changes.
- If health is `PARTIAL` only because progress is below the completion threshold, inspect whether auto-demo completion or another exit path stops the app before the authoritative progress metric reaches the threshold.
- Do not tune auto-demo duration by frame count alone. Compare the state progress metric, frame count, and wall time; the health contract is based on simulation progress.
- When a loop exits early but still writes PNG/state, write `decision.md` for that partial iteration before rerunning.

## State Vs Pixels

- Do not call gameplay done because JSON contains entities. For core gameplay systems, require state proof, state deltas, and a screenshot or runtime view that makes the loop inspectable.
- For stateful loops, look for cause-and-effect evidence such as inputs changing, outputs respecting limits, and expected stop or cap conditions being reached.
- For camera or movement-facing visual changes, verify that world-scale visuals stay fixed in world space while the player moves. Treat any player-following continent, fake ground sheet, underlay, or foot disc in normal gameplay as a failure unless the task explicitly asked for a debug marker.
- For top-down or far camera changes, inspect at least one runtime view or screenshot after movement; compile checks alone do not catch scale/framing bugs.

## Decision Template

```markdown
# iter_NN decision

task: <current goal>
result: pass / partial / fail

score:
- sky: X/10
- player: X/10
- terrain: X/10
- decor: X/10
- hud: X/10
- gameplay: X/10
- total: X.X/10

vs_prev:
- visual: improved / same / worse, <reason>
- state: <key delta from diff.json>

problems:
- <specific issue 1>
- <specific issue 2>
- <specific issue 3>

tests:
- <commands run>
- <results>

next:
- <single highest-value next action>
```

If three consecutive iterations do not improve, stop the current direction and re-diagnose instead of continuing parameter tweaks.

## Scoring Focus

- Sky: not black or white; sky is readable.
- Player: visible, with orientation and height cues.
- Terrain: voxel terrain is legible and spawn is standable.
- Decor: props and environment elements have visible layering and read as intentional in-world assets rather than debug or calibration objects.
- HUD: readable and not blocking critical view.
- Gameplay: auto-demo moves; resources and actor state changes are explainable; no out-of-bounds, voxel spam, or log spam.
