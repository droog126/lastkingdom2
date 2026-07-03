---
name: bevy-gameplay-dev
description: Bevy 0.18.1 gameplay, client, server, render, HUD, simulation, networking, and scenario development rules for lastkingdom2. Use when editing crates/client, crates/server, crates/core gameplay systems, Bevy rendering/input/HUD/camera code, voxel rendering, multiplayer protocol behavior, scenario logic, or performance/logging issues.
---

# Bevy Gameplay Dev

## Scope

Use this skill for gameplay-facing and engine-facing changes. Combine it with `$tdd-iteration` for testable rules and `$closed-loop-ai-dev` for visual, auto-demo, or player-experience changes.

## Bevy 0.18.1 Rules

- Use `Mesh3d` and `MeshMaterial3d`; do not add deprecated `PbrBundle` or `MaterialMeshBundle`.
- Share `Handle<Mesh>` and `Handle<StandardMaterial>` for repeated block/model types.
- Avoid naming conflicts with Bevy `World`; use `World as GameWorld` when needed.
- Throttle per-tick `info!` and `warn!` logs with `Local<u32>` or equivalent.
- Keep visual state tied to simulation state unless the task is explicitly presentation-only.

## Dependency Guardrails

- `Cargo.toml` pins `compt = ">=1.9, <1.10"` because broccoli 0.6 does not compile with compt 1.10. Do not bump it.
- Rust edition is 2024 and the repo expects Rust 1.75+.

## Priority Order

1. Compile failures, test failures, panic, dead loop, out-of-bounds, data corruption.
2. Rule errors, state desync, resource conservation breaks, network/client prediction mismatch.
3. Player-visible issues: invisible entities, stuck movement, wrong HUD, no feedback.
4. Performance issues: too many voxels, repeated per-tick heavy work, log spam.
5. Visual and experience enhancements.

Handle only one to three strongly related issues per task. Record additional findings instead of mixing them into the current patch.

## Common Pitfalls

- `cargo build` without a package/feature can be slow for Bevy iteration.
- Dev client/server builds may need `--features dev-dynamic-linking`; release/CI must not.
- Do not directly mutate `LK2_PORT` or similar environment variables inside concurrent tests.
- `MatchClock.wall_secs` changing does not guarantee `phase` has refreshed.
- Protection-period rules must consider both attacker and target.
- Resource drops and manual kills should share rule logic, not duplicate matches.
- A better screenshot with unchanged `diff.json` may be presentation-only.
- Changed state with unchanged screenshot often means player, camera, or entity visibility is wrong.

## Completion

A gameplay/client task is complete only when:

- the change is explained clearly
- relevant tests or a reason for no tests are provided
- validation commands passed or failures are reported concretely
- closed-loop artifacts were inspected when visuals/gameplay changed
- unresolved risks are listed
