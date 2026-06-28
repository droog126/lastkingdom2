# Engineering Baseline

This repo is a Bevy workspace with three runtime crates:

- `lk2-core`: shared game state, rules, protocol types, diagnostics, and pure logic.
- `lk2-client`: Bevy window, rendering, HUD, input, offline demo, screenshots, and client-side prediction.
- `lk2-server`: headless authority loop, networking, replication, and server-side PvP.

## Hard Boundaries

- `lk2-core` may define Bevy components/resources for shared state, but must not own rendering, window, asset loading, screenshots, input devices, or process orchestration.
- `lk2-client` owns presentation and local UX only. It may call shared simulation APIs, but it should not fork rule logic that belongs in `lk2-core`.
- `lk2-server` owns authority and network ingress. It must not depend on client rendering, UI, camera, or pretty asset code.
- `tools/` owns reproducible asset generation. Generated runtime output belongs in `screenshots/` or `run-logs/`, not the repo root.
- Root `loop.ps1` and `tdd.ps1` are compatibility wrappers and the supported developer entry points.
- Real script implementations live under `scripts/loop`, `scripts/dev`, `scripts/ci`, and `scripts/maintenance`.

## Current Architecture Debt

The codebase compiles, tests, and the closed-loop demo is currently healthy, but several modules are too large:

- `crates/client/src/render/mod.rs`: rendering, camera, input, indicators, and auto-demo movement are mixed.
- `crates/client/src/main.rs`: app construction, networking, diagnostics, screenshots, and local sim wiring are mixed.
- `crates/core/src/combat.rs`: combat data, rules, systems, and tests are in one file.

Do not add broad new behavior to these files without either:

- extracting a focused module first, or
- documenting why the change must stay local and keeping it small.

## Required Checks

Use the project wrappers:

```powershell
.\tdd.ps1 -Scope changed
.\tdd.ps1 -Scope audit
.\loop.ps1 -Offline -Seconds 12
```

`.\tdd.ps1 -Scope audit` includes `scripts/ci/architecture_audit.ps1`, which checks:

- no hard-coded local absolute paths in project scripts,
- no duplicate client objective setup registration,
- no runtime/build output in the repo root,
- known oversized Rust modules are reported as warnings.

## Next Refactor Order

1. Split `crates/client/src/main.rs` into `app.rs`, `network.rs`, `offline_loop.rs`, and `capture.rs`.
2. Split `crates/client/src/render/mod.rs` into camera, player input, terrain rendering, indicators, and auto-demo movement.
3. Split `crates/core/src/combat.rs` into data, rules, ECS systems, and tests.

Each extraction should be behavior-preserving and validated before adding new gameplay.
