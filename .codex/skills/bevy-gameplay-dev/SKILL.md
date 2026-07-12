---
name: bevy-gameplay-dev
description: Bevy 0.19 runtime development workflow for lastkingdom2. Use when changing client or server ECS systems, rendering, input, HUD, camera, voxel presentation, simulation wiring, networking, replication, scenarios, performance, or logging in crates/client, crates/server, or Bevy-facing crates/core code. Add TDD or closed-loop validation only when that evidence is needed.
---

# Bevy Gameplay Dev

## Direction

- Build on Bevy 0.19 and keep presentation small, readable, colorful, toy-like, and state-driven.
- Prefer clear silhouettes and immediate state readability over density or realism.
- Keep shared rules in `lk2-core`, authority in `lk2-server`, and presentation/input in `lk2-client`.
- Make offline and online modes call the same core rule step; clients must not invent authoritative state.

## Player-Facing UI

- Use Simplified Chinese for all player-visible HUD, inventory, menu, button, resource, status, and shortcut text unless the user explicitly requests another language.
- Keep newly added UI labels and interaction feedback consistently Chinese; internal Rust identifiers and developer logs may remain English.
- Before handoff, scan changed UI code for accidental English display strings and verify that remappable shortcut hints are generated from the current bindings.

## Bevy Guardrails

- Use `Mesh3d` and `MeshMaterial3d`, not removed bundle APIs.
- Spawn GLTF scenes with `WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path)))`.
- Use `DirectionalLight.shadow_maps_enabled` and Bevy 0.19 `TextFont` APIs.
- Share mesh and material handles for repeated content.
- Avoid Bevy `World` naming conflicts by importing the game type as `GameWorld` when needed.
- Throttle per-tick logs and repeated expensive work.
- Keep large world visuals fixed in world space and derived from world state. Player-following ground sheets, continents, or underlays belong only in explicit debug/preview modes that default off.
- Avoid coplanar terrain overlays and z-fighting; prefer terrain material variation or an explicit decal/depth strategy.
- Keep the existing `compt = ">=1.9, <1.10"` compatibility pin unless a dedicated dependency task proves a safe upgrade.

## Workflow

1. Trace the affected plugin, startup registration, schedule, resources, and authority path.
2. Change one to three strongly related issues without unrelated refactoring.
3. For deterministic rules, use `$tdd-iteration` for the pure behavior portion.
4. Batch related implementation work before validation. Do not rerun `cargo check`, tests, or loop after every small edit when the next edits are already known.
5. Run the narrowest affected test or `cargo check -p <crate>` at a meaningful milestone, before handoff, after risky schedule/API changes, or after a failed validation has been addressed.
6. For low-risk visual tuning, presentation-only parameter changes, or exploratory refactors, it is acceptable to defer validation and report that choice explicitly.
7. Add `$closed-loop-ai-dev` only when the user explicitly asks for loop/screenshot/runtime evidence, the acceptance criteria explicitly require it, or the final claim would say rendered behavior was actually observed.

## Validation Choice

- No validation yet: while doing a batch of related visual/layout/parameter edits and more edits are already planned.
- `cargo check -p lk2-client`: before handoff when touching Bevy APIs, ECS queries/components, schedules, imports, resources, or module wiring.
- Focused tests: when changing deterministic math, parsing, state transitions, protocol values, or pure helper behavior.
- Runtime screenshot/loop: only when explicitly requested, required by acceptance criteria, or needed to support a final claim about observed rendered output, camera framing, movement after spawn, asset visibility, HUD readability, or auto-demo artifacts.
- Do not run loop just because a change is visible, graphical, Bevy-facing, or presentation-only.
- Do not repeat the same failed validation until code or configuration has changed in response to the failure.
- In the final report, say which validation tier was chosen and why; if validation was deferred, state the risk plainly.

## Runtime Checks

- Validate camera and movement changes after movement, not only at spawn, only when runtime validation is selected.
- Require both state evidence and visual evidence only when reporting that visible gameplay state was actually verified.
- Prefer one useful runtime check over repeated compile checks during visual iteration; use screenshots/loop evidence only when the claim depends on actual rendered output.
- Inspect old presentation layers before changing correct simulation logic to compensate for hidden or occluded behavior.
- Validate auto-demo timing against the simulation progress metric used by health checks, not frame count alone.
- Treat compile failures, panics, state desync, conservation breaks, and authority mismatch as higher priority than polish.

## Completion

Report the behavior changed, tests or reason no stable test applies, the affected crate check, runtime evidence when required, and unresolved risk. Do not claim a visual/runtime result from compilation alone.
