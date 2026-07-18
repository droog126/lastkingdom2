<!-- doc-status: current -->
# Engineering baseline

This is the human-readable engineering boundary. Code, Cargo metadata, `justfile`, and `xtask`
behavior take precedence; update this file and affected skills in the same change when a boundary or
workflow changes.

## Workspace

The runtime workspace has three crates:

- `lk2-core`: shared state, rules, protocol types, diagnostics, and pure simulation logic.
- `lk2-client`: focused Bevy playable scene, input, offline authority, screenshots, and previews.
- `lk2-server`: headless authority loop, networking, replication, and server-side PvP.

Engine-family dependencies are defined in the workspace `Cargo.toml`: Bevy 0.19, Avian3D 0.7,
Lightyear 0.28 with its Avian3D integration, Leafwing Input Manager 0.21, and bevy-tnua 0.32.
Do not duplicate dependency constraints in another current document.

## Ownership boundaries

- `lk2-core` may define shared Bevy components and resources, but does not own windows, rendering,
  asset loading, screenshots, input devices, or process orchestration.
- `lk2-client` owns presentation and local UX. Shared gameplay rules belong in `lk2-core` rather
  than client-only copies.
- `lk2-server` owns authority and network ingress. It must not depend on client rendering, UI,
  camera, or pretty-asset code.
- `lk2-client --ai-client` is a headless online client, not a second authority. It observes
  replicated player state and sends the existing validated gameplay commands through Lightyear.
- `tools/` owns reproducible asset generation.
- `xtask/` owns durable workflow automation, state files, JSON contracts, retries, audits, and
  cross-platform error handling.
- `justfile` owns short command aliases only.
- Generated runtime output belongs in `screenshots/` or `run-logs/`, not the repository root.

## Offline authority

Offline play is client-offline authority: the client owns the window and invokes shared `lk2-core`
simulation and combat APIs in-process. Online play now uses the focused client's Lightyear adapter
and keeps the server authoritative for movement and gameplay messages. The current online scene
covers connection, input, basic replicated player presentation, and a compact HUD/ecology overlay;
the online player path also carries Avian `Position`/`Rotation` through the Lightyear Avian
integration; the server player has a kinematic Avian body while the shared world-grid rules remain
the terrain authority. Robust multiplayer player state remains architecture debt. A shared rule
needed by both modes belongs in `lk2-core`.

## Known architecture debt

- `crates/client/src/game_scene.rs` still combines scene construction, offline authority,
  presentation, input, camera, and screenshot behavior.
- The focused client online scene currently covers Lightyear connection, Leafwing input, basic player
  position presentation, gameplay message ingress, a compact replicated HUD/ecology overlay, and
  basic auto-demo artifact production. Robust multiplayer player state and richer HUD/ecology
  presentation still need to be completed.
- `crates/core/src/combat.rs` still combines combat data, rules, ECS systems, and tests.

Keep changes to these files focused. Prefer behavior-preserving extraction before adding broad new
responsibilities.

## Required checks

Choose the narrowest check that covers a change:

```sh
just test-changed
just test-nextest
just coverage
just deps-unused
just snapshots
just audit-tdd
just audit-architecture
just audit-docs
just audit-skills
just loop
just loop-ai
just milestone
just health
```

The repository pins its Rust toolchain in `rust-toolchain.toml`. CI uses the crates.io sparse
index. Developers who need a regional registry mirror should configure it in their user-level
Cargo config rather than committing a workspace-wide source replacement.

The audits have distinct scopes:

- `audit-tdd`: separates default and experimental core tests, reports files without direct tests,
  and flags top-level modules with no runtime references outside their own implementation.
- `audit-architecture`: rejects PowerShell workflow files under `scripts/` and machine-local
  absolute paths inside those files.
- `audit-docs`: validates document status, current-document stale terms and absolute paths,
  documentation index coverage, and local Markdown links.
- `audit-skills`: validates project skill frontmatter, UI metadata, and AGENTS routing.
- `audit-visual`: checks selected world-visual ownership patterns in client code.

These are guardrails, not substitutes for package tests, builds, or runtime evidence when that
evidence is actually required. Visual, graphical, Bevy presentation-layer, or presentation-only
changes do not require `just loop` by default.
`just test-changed` automatically runs the documentation and skill audits when affected paths are
present in its changed-file set.

## Closed-loop artifacts

The artifact contract remains:

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
  ai_client.json
```

Read `health.json` first. A later loop must not begin until the preceding iteration has a completed
`decision.md`. A `PARTIAL` verdict requires explanation and follow-up; it is not equivalent to
`PASS`.

The focused client now has basic offline and online auto-demo producers for `iter_NN.png`,
`final_state.json`, and `diff.json`. Health, assertions, regression summaries, and decision
templates are still owned by `xtask`; a generated iteration is runtime evidence only after the
requested loop/health command has actually been run and inspected.

`just loop` is the default online closed loop: it starts the focused rendered client and a
headless AI client, and requires `ai_client.json` to prove connection, replicated observations,
movement, and decisions. `just loop-ai` is an explicit alias for the same workflow; use
`just loop-offline` for the offline-only loop.

`just milestone` copies the newest loop's primary PNG to a unique timestamped path under the
top-level `milestones/` directory. Unlike ordinary runtime output, this directory is Git-visible
and `just clean-runs` preserves it without special-casing `screenshots/`. The command archives
presentation evidence only; the unchanged source iteration remains the owner of health and
simulation evidence.

The rendered offline and online gameplay scenes also install the shared milestone capture plugin.
Pressing `F12` writes the current primary window directly to `milestones/`; preview-only modes and
closed-loop JSON producers remain separate.

Normal offline and online play aliases route through `xtask play` and archive client logs and error
summaries under `run-logs/`. `just play --online` builds the client and server, starts the server,
and connects the focused online scene. This is not yet closed-loop runtime evidence. On Windows,
normal play defaults to Vulkan and accepts `--gpu-backend=dx12` as an explicit comparison or fallback.

## Refactor order

1. Split offline authority and snapshot-to-presentation mapping out of `game_scene.rs`.
2. Expand the focused online adapter over the same snapshot presentation boundary, including richer
   HUD/ecology presentation and robust multiplayer player state.
3. Harden closed-loop artifact production after the focused runtime path is registered.

Validate every extraction before adding dependent gameplay behavior.
