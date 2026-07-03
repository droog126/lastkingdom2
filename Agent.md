# Agent.md

This repository uses project skills for detailed instructions. Keep this file as a routing table only. When a task matches a route, read the matching `SKILL.md` before acting.

## Skill Routing

- Use `$local-dev` for ordinary repository work: inspect files, make local code/docs/script changes, choose validation scope, handle git hygiene, or prepare branch/commit/PR work.
- Use `$tdd-iteration` for testable behavior changes: pure logic, rules, state machines, resources, drops, conservation, nations, monsters, animals, combat, protection periods, phase timing, CLI/protocol/network parsing, AI decisions, scenarios, tick observers, bug fixes, and regressions.
- Use `$bevy-gameplay-dev` for Bevy 0.18.1 client/server/gameplay work: `crates/client`, `crates/server`, gameplay systems in `crates/core`, render/input/HUD/camera, voxel rendering, networking behavior, scenarios, performance, and log-spam fixes.
- Use `$closed-loop-ai-dev` for visual/gameplay-experience iteration: screenshots, offline auto-demo, `loop.ps1`, `scripts/loop`, observer health, `health.json`, `assertions.json`, `diff.json`, `final_state.json`, PNG review, and `decision.md`.
- Use `$screenshot-scoring` for evidence-backed screenshot scoring: visual category scores, hard gates, previous/current comparison, `decision.md` score sections, and next visual iteration choice.
- Use `$ai-modeling` for 3D assets: Blender-generated GLBs, procedural models, `tools/build_*.py`, `assets/procedural/pretty`, `assets/procedural/eco`, animals, terrain buildings, `MANIFEST.json`, and poly-budget validation.

## Composition

- For gameplay rule changes, use `$tdd-iteration` plus `$bevy-gameplay-dev`.
- For visible client/gameplay changes, use `$bevy-gameplay-dev` plus `$closed-loop-ai-dev`.
- For loop screenshot review, use `$closed-loop-ai-dev` plus `$screenshot-scoring`.
- For generated models that appear in game, use `$ai-modeling` plus `$closed-loop-ai-dev`.
- For broad tasks, start with `$local-dev`, then add the more specific skill above.

## Skill Files

- `.codex/skills/local-dev/SKILL.md`
- `.codex/skills/tdd-iteration/SKILL.md`
- `.codex/skills/bevy-gameplay-dev/SKILL.md`
- `.codex/skills/closed-loop-ai-dev/SKILL.md`
- `.codex/skills/screenshot-scoring/SKILL.md`
- `.codex/skills/ai-modeling/SKILL.md`
