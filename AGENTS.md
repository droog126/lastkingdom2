# AGENTS.md

This file routes agent work to project skills. Keep operational detail in the matching `SKILL.md`.

## Source Of Truth

- Use current code and `xtask` output to establish what the repository does now.
- Use the relevant skill and `docs/architecture/engineering-baseline.md` to establish intended engineering behavior.
- When implementation and intended behavior disagree, report the conflict instead of automatically rewriting documentation to match a likely bug.
- Update only active docs and skill contracts directly affected by the current change. Treat `docs/archive/` and imported design notes as historical unless an active document points to them.

## Skill Selection

- Choose the smallest sufficient set: one primary skill and, only when needed, one validation companion.
- Use `$local-dev` as the fallback for ordinary repository work. Do not add it automatically when a more specific skill already covers the task.
- Treat audit and review requests as read-only unless the user also asks for implementation.
- Read every selected skill before acting.

## Skill Routing

- Use `$local-dev` for ordinary code/docs/tooling work, validation selection, worktree hygiene, and git preparation when no more specific route applies.
- Use `$tdd-iteration` for deterministic behavior that can be specified with a focused test before implementation: rules, state machines, parsing, invariants, scenarios, AI decisions, and regressions.
- Use `$bevy-gameplay-dev` for Bevy 0.19 client/server/runtime work: ECS systems, rendering, input, HUD, camera, networking, simulation wiring, scenarios, performance, and logging.
- Use `$bevy-resource-lifecycle` for runtime asset or GPU lifetime symptoms such as out-of-memory, invalid textures, repeated generated asset allocation, render rebuild leaks, or generated assets surviving entity cleanup.
- Use `$closed-loop-ai-dev` when the task explicitly requires running or diagnosing the observe-decide-act loop, auto-demo, runtime screenshots, health artifacts, or `decision.md`.
- Use `$screenshot-scoring` only for quantitative loop-screenshot scoring, previous/current comparison, or the score section of `decision.md`; ordinary PNG inspection does not require it.
- Use `$ai-modeling` for reproducible Blender/GLB generation, asset manifests, model previews, poly budgets, and wiring generated models.
- Use `$game-logic-audit` for evidence-backed, read-only audits of end-to-end game rules, authority flow, unreachable systems, or client/core/server divergence.
- Use `$docs-governance` for documentation classification, current/reference/proposal boundaries, stale facts, broken links, command/dependency/artifact-contract guidance, and `audit-docs` failures.
- Use `$skill-sedimentation` when the user explicitly asks to codify lessons, update skill routing, create/update a skill, or prevent a repeated agent failure through durable instructions.

## Composition

- Gameplay rule implementation: `$tdd-iteration`; add `$bevy-gameplay-dev` only when ECS scheduling or runtime wiring is part of the change.
- Visible Bevy/runtime implementation: `$bevy-gameplay-dev`; add `$closed-loop-ai-dev` when runtime evidence is required for acceptance.
- Resource lifetime bug: `$bevy-resource-lifecycle`; add `$closed-loop-ai-dev` only when reproduction or validation needs loop/runtime evidence.
- Loop scoring: `$closed-loop-ai-dev` plus `$screenshot-scoring` only when a numeric score or scored `decision.md` is required.
- Generated asset: `$ai-modeling`; add `$closed-loop-ai-dev` only after the asset is wired into the game and must be judged in the target camera.
- Logic audit: `$game-logic-audit` alone for the report; use the implementation route in a later or explicitly combined fix task.
- Documentation change: `$docs-governance`; add `$tdd-iteration` only when changing the `xtask` documentation audit behavior.
- Skill changes: `$skill-sedimentation` plus the system `$skill-creator` skill.

## Repository Constraints

- Framework: Bevy 0.19. Visual target: small, readable, colorful, toy-like Sokpop-style presentation.
- Put durable automation in Rust `xtask`; use Python for Blender/assets and focused analysis; keep `justfile` to short aliases.
- Do not add root PowerShell workflow scripts or Python loop orchestration.
- For public internet access, use SOCKS5 `127.0.0.1:7890` when the local proxy is reachable. If it is unavailable, report that fact rather than repeatedly retrying through a dead proxy.

## Skill Files

- `.codex/skills/local-dev/SKILL.md`
- `.codex/skills/tdd-iteration/SKILL.md`
- `.codex/skills/bevy-gameplay-dev/SKILL.md`
- `.codex/skills/bevy-resource-lifecycle/SKILL.md`
- `.codex/skills/closed-loop-ai-dev/SKILL.md`
- `.codex/skills/screenshot-scoring/SKILL.md`
- `.codex/skills/ai-modeling/SKILL.md`
- `.codex/skills/game-logic-audit/SKILL.md`
- `.codex/skills/docs-governance/SKILL.md`
- `.codex/skills/skill-sedimentation/SKILL.md`
