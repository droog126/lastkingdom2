---
name: closed-loop-ai-dev
description: Closed-loop AI iteration workflow for lastkingdom2. Use when tasks affect visuals, gameplay feel, client experience, offline auto-demo behavior, screenshots, scenario progression, observer health, xtask loop, or when the user asks for AI self-iteration, observe-decide-act, loop runs, screenshot review, health.json analysis, or decision.md.
---

# Closed Loop AI Dev

## Contract

The loop is observe -> decide -> act -> build -> re-run. A loop is not complete until the run artifacts are inspected and `screenshots/iter_NN/decision.md` records the result.

`xtask loop` enforces this: before starting a new iteration, the previous `screenshots/iter_NN/decision.md` must exist. After a run it writes `decision.template.md` for the latest iteration.

Use this skill for visual, gameplay, client UX, offline demo, scenario, screenshot, and automated observation changes. Use `$screenshot-scoring` whenever inspecting PNGs or writing the score section of `decision.md`.

## Run

```sh
just loop
```

Direct form:

```sh
cargo run -q -p xtask -- loop --offline --seconds 60
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

## Health Meaning

- `PASS`: screenshot readable, final state parseable, tick threshold reached, observer invariants pass, auto-demo made basic progress.
- `PARTIAL`: process ran but did not meet the full loop contract, usually tick/progression shortfall.
- `FAIL`: hard failure such as black screen, bad image, missing state, observer error, invariant failure, out of bounds, or unreadable visual.

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
- Decor: trees, water, animals, monsters, or props have visible layering.
- HUD: readable and not blocking critical view.
- Gameplay: auto-demo moves; resources and actor state changes are explainable; no out-of-bounds, voxel spam, or log spam.
