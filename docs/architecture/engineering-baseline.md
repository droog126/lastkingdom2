<!-- doc-status: current -->
# Engineering baseline

This is the human-readable engineering boundary. Code, Cargo metadata, `justfile`, and `xtask`
behavior take precedence; update this file and affected skills in the same change when a boundary or
workflow changes.

## Workspace

The runtime workspace has three crates:

- `lk2-core`: shared state, rules, protocol types, diagnostics, and pure simulation logic.
- `lk2-client`: Bevy window, rendering, HUD, input, offline demo, screenshots, and prediction.
- `lk2-server`: headless authority loop, networking, replication, and server-side PvP.

Engine-family dependencies are defined in the workspace `Cargo.toml`: Bevy 0.19, Avian3D 0.7,
Lightyear 0.28, Leafwing Input Manager 0.21, bevy-inspector-egui 0.37, bevy-tnua 0.32, and
bevy_panorbit_camera 0.35. Do not duplicate dependency constraints in another current document.

## Ownership boundaries

- `lk2-core` may define shared Bevy components and resources, but does not own windows, rendering,
  asset loading, screenshots, input devices, or process orchestration.
- `lk2-client` owns presentation and local UX. Shared gameplay rules belong in `lk2-core` rather
  than client-only copies.
- `lk2-server` owns authority and network ingress. It must not depend on client rendering, UI,
  camera, or pretty-asset code.
- `tools/` owns reproducible asset generation.
- `xtask/` owns durable workflow automation, state files, JSON contracts, retries, audits, and
  cross-platform error handling.
- `justfile` owns short command aliases only.
- Generated runtime output belongs in `screenshots/` or `run-logs/`, not the repository root.

## Offline authority

Offline demo mode is client-offline authority: the client owns the window and invokes shared
`lk2-core` simulation APIs in-process. Online play remains `lk2-server` authority. A shared rule
needed by both modes must move into `lk2-core` before integration.

## Known architecture debt

- `crates/client/src/main.rs` still mixes app construction, networking, diagnostics, and local
  simulation wiring. Screenshot/capture logic has already been extracted to `capture.rs`.
- `crates/client/src/render/mod.rs` still mixes camera, player input, terrain rendering,
  indicators, and auto-demo movement despite several focused helper modules.
- `crates/core/src/combat.rs` still combines combat data, rules, ECS systems, and tests.

Keep changes to these files focused. Prefer behavior-preserving extraction before adding broad new
responsibilities.

## Required checks

Choose the narrowest check that covers a change:

```sh
just test-changed
just audit-tdd
just audit-architecture
just audit-docs
just audit-skills
just loop
just health
```

The audits have distinct scopes:

- `audit-tdd`: separates default and experimental core tests, reports files without direct tests,
  and flags top-level modules with no runtime references outside their own implementation.
- `audit-architecture`: rejects PowerShell workflow files under `scripts/` and machine-local
  absolute paths inside those files.
- `audit-docs`: validates document status, current-document stale terms and absolute paths,
  documentation index coverage, and local Markdown links.
- `audit-skills`: validates project skill frontmatter, UI metadata, and AGENTS routing.
- `audit-visual`: checks selected world-visual ownership patterns in client code.

These are guardrails, not substitutes for package tests, builds, or runtime evidence.
`just test-changed` automatically runs the documentation and skill audits when affected paths are
present in its changed-file set.

## Closed-loop artifacts

`just loop` maps to `just xtask loop --offline --seconds 60` and writes:

```text
screenshots/iter_NN/
  iter_NN.png
  final_state.json
  diff.json
  assertions.json
  health.json
  health.txt
  error_logs.json
  error_logs.txt
  perception_manifest.json
  regression.json
  decision.template.md
  decision.md
```

Read `health.json` first. A later loop must not begin until the preceding iteration has a completed
`decision.md`. A `PARTIAL` verdict requires explanation and follow-up; it is not equivalent to
`PASS`.

Normal play aliases route through `xtask play`. They archive client logs and error summaries under
`run-logs/`; online play also archives server logs. On Windows, normal play defaults to Vulkan and
accepts `--gpu-backend=dx12` as an explicit comparison or fallback.

## Refactor order

1. Continue splitting client app construction, networking, and offline wiring out of `main.rs`
   while retaining `capture.rs` as the capture owner.
2. Split render camera, input, terrain, indicators, and auto-demo movement into focused modules.
3. Split combat data, rules, ECS systems, and tests without changing behavior.

Validate every extraction before adding dependent gameplay behavior.
