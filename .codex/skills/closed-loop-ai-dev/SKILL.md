---
name: closed-loop-ai-dev
description: Runtime observe-decide-act validation workflow for lastkingdom2. Use when the user or acceptance criteria require running or diagnosing just loop/xtask loop, offline auto-demo, runtime screenshots, observer health, progression artifacts, health.json, assertions.json, regression.json, diff.json, final_state.json, or decision.md. Do not trigger for every visual code edit or standalone model preview.
---

# Closed Loop AI Dev

## Contract

Use the loop to obtain runtime evidence: observe -> decide -> act -> build -> re-run. A completed loop iteration must have inspected artifacts and a `screenshots/iter_NN/decision.md` before another normal iteration starts.

Before running any command, confirm the task actually needs loop, screenshot, health, auto-demo, or runtime artifact evidence because the user asked for it, acceptance criteria require it, diagnosis cannot proceed without it, or the final answer will claim observed rendered behavior. If this skill was loaded only because a change is visible, graphical, Bevy-facing, or presentation-only, return to the primary implementation skill and do not run loop commands.

Standalone model previews belong to `$ai-modeling`; use this skill only after an asset is wired into gameplay or when the task explicitly requires loop evidence.

## Run

```sh
just loop
just health
```

To retain the latest loop PNG as a long-lived milestone without rerunning the loop:

```sh
just milestone
```

This copies only the primary PNG to the top-level `milestones/` directory. The archive is
Git-visible, survives `just clean-runs`, and does not modify `screenshots/`; continue to use the
source iteration's JSON and `decision.md` for health and simulation claims.

Direct form:

```sh
just xtask loop --offline --seconds 60
```

Build the affected executable first unless `xtask loop` is allowed to build it. Do not interpret stale output as evidence for a source change.

## Artifact Order

1. Read `health.json` for the primary verdict.
2. Read `regression.json` for newly failed or newly passed assertions.
3. If health is `PARTIAL` or `FAIL`, read `assertions.json` and copy concrete failure messages.
4. Read `final_state.json` and `diff.json` only to explain state, authority, or progression.
5. When present, read natural-world `event_count`, `region_tick`, `catch_up_remaining`, and
   `save_restored`; these fields are optional for backward compatibility.
6. Inspect the PNG when the claim depends on visibility, framing, HUD, terrain, player, or presentation.
7. Use `$screenshot-scoring` only when a numeric score, comparison, or scored `decision.md` is requested.
8. Complete `decision.md` from the generated template with result, evidence, problems, tests, and one next action.

If an artifact is absent, report it as missing instead of fabricating a score or state conclusion.

## Evidence Rules

- JSON entity existence is not proof of readable gameplay; require cause-and-effect state evidence and the relevant runtime view.
- A visually improved screenshot does not override a health regression.
- World-scale terrain and decor must remain fixed while the player moves.
- Camera scale changes require at least one view after movement.
- Compare simulation progress, frame count, and wall time when diagnosing early auto-demo exits.
- If a partial run still produced artifacts, write its decision before rerunning.
- After three non-improving iterations, stop parameter tuning and re-diagnose the system.

## Completion

Report the iteration path, health/regression verdict, inspected evidence, validation commands, and remaining failure. When a milestone was requested, also report its archive path. Do not modify unrelated code merely because the loop reveals a new issue; record it unless the user asked to fix it.
