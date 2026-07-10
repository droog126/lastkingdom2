---
name: screenshot-scoring
description: Quantitative scoring rubric for lastkingdom2 closed-loop gameplay screenshots. Use when the user asks for numeric visual scores, previous/current comparison, an improvement verdict, hard-gate evaluation, or the score section of a loop decision.md. Do not use for ordinary PNG inspection, standalone GLB/model previews, or implementation work.
---

# Screenshot Scoring

## Contract

Score visible and artifact-backed evidence, not intent or expected behavior. This is a read-only evaluation workflow: identify gates and recommend the next change, but do not modify code unless the user separately asks for implementation.

## Inputs

Required:

- current loop screenshot
- current task goal
- current `health.json`

Use when available:

- `regression.json` and `assertions.json`
- `final_state.json` and `diff.json`
- previous comparable screenshot and artifacts
- `.harness/KNOWN_ISSUES.md` when writing the carryover section of `decision.md`

If the screenshot cannot be inspected, mark visual categories blocked. If health is missing, score pixels only and mark gameplay blocked. Do not invent missing evidence.

## Workflow

1. Read health and regression artifacts before viewing pixels.
2. Record every newly failed assertion and any hard failure reported by current automation.
3. Inspect the full screenshot at normal scale; zoom only to verify uncertain details.
4. Apply hard gates, then score each category with one evidence sentence.
5. Compare against the previous comparable iteration when available.
6. Select exactly one highest-value next action.

Machine thresholds belong in `xtask` health/assertions. Use their current verdicts rather than copying historical iteration-specific constants into this rubric.

## Hard Gates

- Invalid, black, white, or camera-inside-geometry image: total maximum 2.0.
- No game world visible: total maximum 2.5.
- Lost player/camera target with no readable HUD state: total maximum 4.0.
- Overlay prevents scene judgment: total maximum 5.0.
- `health.json` `FAIL`: gameplay maximum 4.
- `health.json` `PARTIAL`: gameplay maximum 6.
- A failed assertion with fail severity: gameplay maximum 4; partial severity: maximum 6.
- Severe current rendering, protocol, out-of-bounds, or voxel log gates: gameplay maximum 5.

Apply the cap and continue scoring so the report still explains visible strengths and weaknesses.

## Category Anchors

Use integers unless a half point is necessary:

| Score | Meaning |
| --- | --- |
| 0 | absent, broken, or impossible to judge |
| 2 | present but severely unreadable or misleading |
| 5 | functional and understandable |
| 8 | clear, attractive, and supports gameplay readability |
| 10 | polished, expressive, and immediately legible |

Score:

- `sky`: exposure, horizon, fog, lighting, and background contrast
- `player`: silhouette, orientation, grounding, contrast, framing, and occlusion
- `terrain`: standability, navigation, scale, edges, height variation, and depth
- `decor`: identifiable in-world props, layering, variety, placement, and repetition
- `hud`: legibility, overlap, anchoring, concision, and obstruction
- `gameplay`: health plus explainable movement, state changes, actors, and progression

Compute `total = (sky + player + terrain + decor + hud + gameplay) / 6`, round to one decimal, then apply the total hard-gate cap.

## Comparison

- `improved`: total rises by at least 0.5, or the task category rises by at least 1 with no hard regression.
- `same`: total changes by less than 0.5 and no category changes by more than 1.
- `worse`: total drops by at least 0.5, a hard gate appears, or a key category drops by 2 or more.

Never call the iteration improved when health regressed to `FAIL`; separate visual improvement from state regression.

## Output

```markdown
score:
- sky: X/10 - <visible evidence>
- player: X/10 - <visible evidence>
- terrain: X/10 - <visible evidence>
- decor: X/10 - <visible evidence>
- hud: X/10 - <visible evidence>
- gameplay: X/10 - <artifact and visible evidence>
- total: X.X/10

vs_prev:
- visual: improved / same / worse - <specific comparison or not compared>
- state: <artifact delta or not compared>

next:
- <one evidence-backed action>
```
