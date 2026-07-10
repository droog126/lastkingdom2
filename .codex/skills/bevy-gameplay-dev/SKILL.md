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
4. Run the narrowest affected test or `cargo check -p <crate>`.
5. Add `$closed-loop-ai-dev` only when acceptance requires runtime, auto-demo, screenshot, movement, or health evidence.

## Runtime Checks

- Validate camera and movement changes after movement, not only at spawn.
- Require both state evidence and visual evidence when the task claims visible gameplay state.
- Inspect old presentation layers before changing correct simulation logic to compensate for hidden or occluded behavior.
- Validate auto-demo timing against the simulation progress metric used by health checks, not frame count alone.
- Treat compile failures, panics, state desync, conservation breaks, and authority mismatch as higher priority than polish.

## Completion

Report the behavior changed, tests or reason no stable test applies, the affected crate check, runtime evidence when required, and unresolved risk. Do not claim a visual/runtime result from compilation alone.
