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
- For regional simulation or authority migrations, read [regional-simulation.md](references/regional-simulation.md) before editing; preserve one owner for ecology/resources/scheduler and trace the full restore and projection path.

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

## Visual Rig Consumer Context

- Treat a procedural IK rig as a consumer-context feature, not only as a solver: trace the
  authoritative/presentation root, imported GLTF scene hierarchy, named child parts, binding
  marker, animation query, and recursive cleanup as one chain.
- Keep the physics or replicated root responsible for world-space position, heading, scale, and
  authoritative state. Keep IK-driven mesh nodes responsible for local-space pose only; do not let
  a root and a visual child write the same `Transform`.
- Make coordinate spaces explicit. Generate limb targets in body-local space, sample terrain in
  world space using the same-frame prospective body transform (especially the final yaw), then
  convert the contact point back to body-local space before solving or applying child poses. Never
  project feet through a stale root rotation during a turn.
- Bind imported parts recursively and idempotently. Do not mark a root bound until the expected
  named parts exist; GLTF scene instantiation is asynchronous, so the binding system must safely
  retry while the hierarchy is incomplete.
- Give each bound part its owner root and filter animation consumers by that owner. Make the
  schedule edge explicit: entity reconciliation/spawn -> hierarchy binding -> IK animation. When
  roots are removed, despawn their related children and ensure no binding component can outlive the
  owner.
- For every new rig variant, verify the asset node names, segment axis and source length against
  the generator/export output before tuning solver constants. A passing solver test cannot prove
  that the imported mesh uses the expected local axis or hierarchy.

## Workflow

1. Trace the affected plugin, startup registration, schedule, resources, and authority path.
2. For every new material/resource/system, trace the full consumer context: registration/plugin, producer startup system, component/query type, schedule order, and shader binding layout. Confirm the resource parameter is on the system that consumes it, not merely a neighboring startup system.
3. After changing a Query or system parameter, re-read the producer/insert site and every consumer registration. Repeated signatures make nearby-function edits easy to misapply.
4. Check schedule edges in both directions: producer -> projection -> observer/recorder, and cleanup/despawn -> consumer. Make current-tick snapshot dependencies explicit with `.after(...)`.
5. Change one to three strongly related issues without unrelated refactoring.
6. For deterministic rules, use `$tdd-iteration` for the pure behavior portion.
7. Batch related implementation work before validation. Do not rerun `cargo check`, tests, or loop after every small edit when the next edits are already known.
8. Run the narrowest affected test or `cargo check -p <crate>` at a meaningful milestone, before handoff, after risky schedule/API changes, or after a failed validation has been addressed.
9. For low-risk visual tuning, presentation-only parameter changes, or exploratory refactors, it is acceptable to defer validation and report that choice explicitly.
10. Distinguish implementation status from evidence status: do not report a completion percentage or claim a shader/material path works until the relevant compile check or runtime evidence has passed. If validation is deferred, label the result as unverified.
11. Add `$closed-loop-ai-dev` only when the user explicitly asks for loop/screenshot/runtime evidence, the acceptance criteria explicitly require it, or the final claim would say rendered behavior was actually observed.

## Networked Gameplay Contract

- Classify every feature as offline-only, online-only, or shared before editing. Do not call a client-local prototype server-capable until the core wire type, server-owned state, replication registration, client projection, and cleanup lifecycle all exist.
- For a networked entity, trace the full chain: protocol component/message -> server spawn and authority mutation -> replication target -> client query and presentation -> disconnect/death/despawn cleanup. A replicated boolean without an authoritative entity/state owner is not a complete feature.
- Keep client presentation derived from replicated state. Do not infer authoritative position, occupancy, speed, or interaction success from local input after the server path exists.
- Check usage context at both input surfaces: offline keybindings must be remappable and must not collide with existing interactions; online bindings and HUD hints must express the same player-facing action unless the difference is intentional and documented.
- Make offline and online acceptance behavior match for mountable actions, especially jump, movement speed, seat/pose, range checks, and respawn/disconnect behavior.
- Treat spawned GLTF roots as hierarchies. When removing runtime visuals, use recursive relationship cleanup and verify that repeated spawn/despawn cannot leave child entities behind.
- For vehicle or mount collision, distinguish a visual footprint approximation from authoritative physics. If using sampled clearance, rotate the footprint with the authoritative heading and document the approximation; do not describe it as a real collider.

### Networked physics integration

- Treat a physics integration as a full context chain: shared component registration and serialization
  -> server authority entity and mutation -> physics/network plugin order -> replication target ->
  client query -> presentation child -> despawn cleanup.
- Register Avian components such as `Position` and `Rotation` in the shared Lightyear protocol before
  querying them on the client. A workspace dependency declaration or `use ... as _` does not register
  a network component.
- Add the Lightyear physics plugin on both client and server when both participate in the path. Keep
  the existing authority model explicit: if the server still owns grid collision/movement, use Avian
  as a kinematic state bridge and do not claim full Avian-authoritative terrain physics.
- Never let a network/physics entity and its visual offset both write the same `Transform`. Keep the
  network entity physics-only and attach a child visual for seat offsets, bobbing, scale, or effects.
- Before calling a network integration complete, inspect registration, spawn, authority mutation,
  replication, client projection, and disconnect/death cleanup together. Report implementation status
  separately from compile/runtime evidence; do not invent a completion percentage without evidence.

### Networked terrain and chunk presentation

- Trace terrain end to end as one contract: authoritative world edit -> terrain revision -> full
  chunk snapshot/delta producer -> protocol registration -> locally owned client consumer -> mesh
  rebuild -> old Mesh handle cleanup. A client-side delta map is not terrain rendering until a
  replicated snapshot establishes the surrounding material state.
- Treat a delta revision as an ordered stream. Apply only the next revision; if a revision jumps,
  stop applying later deltas and wait for a full snapshot/resync. Never silently accept only the
  newest delta when intermediate edits may exist.
- Include the sampling contract in the snapshot: horizontal origin, border/neighbor width, and
  stride must be explicit or derived from shared protocol constants. Surface Nets, marching cubes,
  and similar extractors need at least one neighboring sample ring; treating every chunk edge as
  air creates artificial walls and visible seams.
- Cache full snapshots by `(terrain_revision, chunk_coordinate)` on the authority and assign the
  replicated component only when its value changes. Do not rebuild and mark a large `Vec<u8>`
  snapshot changed every fixed tick.
- Filter terrain snapshots and deltas by the locally controlled entity using replicated owner
  identity. `ControlledBy.is_some()` is insufficient in multiplayer because a remote player's
  snapshot can overwrite the local terrain projection.
- Register the presentation consumer explicitly: Startup must spawn the terrain and material
  owners; the Update system consuming the snapshot must run after replication projection; Mesh
  replacement must build new assets first, swap handles, then remove old generated handles. Keep
  ore/material overlays separate from authoritative block rules.
- For offline local terrain, avoid hard density masks that manufacture cylindrical or square walls.
  Sample the real neighboring volume first, then trim extracted triangles to the requested edit
  region if a local rebuild is necessary.

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
- For custom vertex materials, inspect main, prepass, deferred, and shadow consumers together; a vertex deformation is incomplete when only the forward path is changed.

## Completion

Report the behavior changed, tests or reason no stable test applies, the affected crate check, runtime evidence when required, and unresolved risk. Do not claim a visual/runtime result from compilation alone.
