# Engineering Baseline

This file is the active engineering baseline for humans. `AGENTS.md` routes agent work to skills, and `.codex/skills/*/SKILL.md` files carry the operational instructions. If active docs and skills drift, prefer current code and `xtask` behavior, then update the docs and skill wording together.

This repo is a Bevy 0.19 workspace with three runtime crates:

- `lk2-core`: shared game state, rules, protocol types, diagnostics, and pure logic.
- `lk2-client`: Bevy window, rendering, HUD, input, offline demo, screenshots, and client-side prediction.
- `lk2-server`: headless authority loop, networking, replication, and server-side PvP.

Current engine-family dependencies are Bevy 0.19, Avian3D 0.7, Lightyear 0.28,
Leafwing Input Manager 0.21, bevy-inspector-egui 0.37, bevy-tnua 0.32, and
bevy_panorbit_camera 0.35. Historical docs and archived drift reports may still
describe Bevy 0.18 or Lightyear 0.26; treat those as background unless active code
or this baseline points back to them.

## Hard Boundaries

- `lk2-core` may define Bevy components/resources for shared state, but must not own rendering, window, asset loading, screenshots, input devices, or process orchestration.
- `lk2-client` owns presentation and local UX only. It may call shared simulation APIs, but it should not fork rule logic that belongs in `lk2-core`.
- `lk2-server` owns authority and network ingress. It must not depend on client rendering, UI, camera, or pretty asset code.
- `tools/` owns reproducible asset generation. Generated runtime output belongs in `screenshots/` or `run-logs/`, not the repo root.
- `xtask/` owns durable workflow automation: loop orchestration, TDD scopes, health checks, screenshot/state assertions, scenarios, and audits.
- `justfile` owns short human-friendly command aliases.
- `scripts/` is not a workflow runtime. Blender/model scripts stay under `tools/`; loop automation stays in Rust.
- Root PowerShell workflow wrappers are legacy. Do not add new durable workflow logic outside `xtask/`.

## Offline Authority Boundary

Current offline demo mode is `client-offline authority`: the Bevy client owns the window and runs
shared `lk2-core` simulation APIs in-process so screenshots and HUD capture stay simple. Online
play remains `lk2-server` authority.

Do not add a second copy of gameplay rules to the client. If offline and online behavior need the
same rule, move that rule into `lk2-core` first and call it from both sides. The intended future
state is an in-process headless server for offline mode, but that should be a dedicated
networking/refactor task, not an incidental gameplay change.

## Current Architecture Debt

The codebase compiles, tests, and the closed-loop demo is currently healthy, but several modules are too large:

- `crates/client/src/render/mod.rs`: rendering, camera, input, indicators, and auto-demo movement are mixed.
- `crates/client/src/main.rs`: app construction, networking, diagnostics, screenshots, and local sim wiring are mixed.
- `crates/core/src/combat.rs`: combat data, rules, systems, and tests are in one file.

Do not add broad new behavior to these files without either:

- extracting a focused module first, or
- documenting why the change must stay local and keeping it small.

## Required Checks

Use the project task runner:

```sh
just test-changed
just audit-tdd
just loop
just health
```

`just loop` currently maps to:

```sh
just xtask loop --offline --seconds 60
```

`just xtask audit-architecture` checks:

- no hard-coded local absolute paths in project scripts,
- no duplicate client objective setup registration,
- no runtime/build output in the repo root,
- known oversized Rust modules are reported as warnings.

## Closed-Loop Artifacts

The current loop output shape is directory-based:

```text
screenshots/iter_NN/
  iter_NN.png
  final_state.json
  diff.json
  assertions.json
  health.json
  health.txt
  decision.template.md
  decision.md
```

Read `health.json` first. A new loop run should not start until the previous iteration has a `decision.md`.

## Next Refactor Order

1. Split `crates/client/src/main.rs` into `app.rs`, `network.rs`, `offline_loop.rs`, and `capture.rs`.
2. Split `crates/client/src/render/mod.rs` into camera, player input, terrain rendering, indicators, and auto-demo movement.
3. Split `crates/core/src/combat.rs` into data, rules, ECS systems, and tests.

Each extraction should be behavior-preserving and validated before adding new gameplay.
