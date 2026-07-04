# AGENTS.md

This repository uses project skills for detailed instructions. Keep this file as a routing table only. When a task matches a route, read the matching `SKILL.md` before acting.

## Synchronization Policy

- `AGENTS.md` is the routing table, not the detailed handbook.
- `.codex/skills/*/SKILL.md` files are the operational instructions for agents.
- `docs/architecture/engineering-baseline.md` is the human-readable engineering baseline.
- `docs/STARTING.md` is the human-readable run guide.
- If these sources drift, prefer current code and `xtask` behavior first, then the relevant `SKILL.md`, then active docs. Update all touched sources in the same change.
- Historical material under `docs/archive/` and imported design notes may mention old paths; do not treat them as current workflow instructions unless an active doc points there.

## Technology Direction

- Framework: Bevy 0.18.1.
- Visual target: Sokpop-style small, readable, colorful, toy-like game presentation with clear silhouettes and simple shapes.
- First choice for core automation: Rust `xtask`, especially closed-loop orchestration, health checks, screenshot/state assertions, test runners, state files, JSON contracts, cross-platform command logic, and robust error handling.
- Second choice: Python only for Blender, asset generation, and focused one-off analysis outside the loop workflow.
- Third choice: `justfile`, only for short command aliases. Do not put complex branching, state handling, or loop logic in `justfile`.
- No PowerShell scripts in root. Loop automation logic lives in `xtask/` with `justfile` as the user-facing command entry. Python is not used for loop workflow code.

## Skill Routing

- Use `$local-dev` for ordinary repository work: inspect files, make local code/docs/script changes, choose validation scope, handle git hygiene, or prepare branch/commit/PR work.
- Use `$tdd-iteration` for testable behavior changes: pure logic, rules, state machines, resources, drops, conservation, nations, monsters, animals, combat, protection periods, phase timing, CLI/protocol/network parsing, AI decisions, scenarios, tick observers, bug fixes, and regressions.
- Use `$bevy-gameplay-dev` for Bevy 0.18.1 client/server/gameplay work: `crates/client`, `crates/server`, gameplay systems in `crates/core`, render/input/HUD/camera, voxel rendering, networking behavior, scenarios, performance, and log-spam fixes.
- Use `$closed-loop-ai-dev` for visual/gameplay-experience iteration: screenshots, offline auto-demo, `cargo run -q -p xtask -- loop` (or `just loop`), observer health, `health.json`, `assertions.json`, `diff.json`, `final_state.json`, PNG review, and `decision.md`.
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
