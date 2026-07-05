# Wanguo Origins: Last Kingdom Diamond

Bevy 0.19 voxel simulation and rendering demo with a closed-loop AI iteration workflow.

The core workflow is: run the game, capture screenshots and state JSON, inspect the result, decide the next change, then build and run again. Read [AGENTS.md](./AGENTS.md) before making code changes.

## Quick Start

```sh
cargo build --workspace
export BEVY_DISABLE_ACCESSIBILITY=1
export RUST_LOG=info
cargo run -p lk2-client -- --offline
```

Closed-loop run:

```sh
just loop
```

Useful validation entry points:

```sh
just test-core
just test-changed
just test
just fmt
```

## Project Layout

- `crates/core/src/` - shared simulation, rules, protocol, world, AI, resources, combat, terrain
- `crates/client/src/` - Bevy client, rendering, HUD, input, screenshots, offline auto-demo
- `crates/server/src/` - headless server, self-check, authoritative simulation, UDP entry point
- `assets/` - art assets and generated GLB models
- `assets/procedural/pretty/` - generated visual models
- `assets/procedural/eco/` - generated ecology models
- `tools/` - Python scripts for Blender model generation and asset validation
- `scenarios/` - scenario JSON scripts
- `screenshots/` - closed-loop output, ignored by Git except archived material
- `docs/` - design notes, architecture plans, gameplay design, and archived imports
- `xtask/` - Rust task runner for loop, health, TDD, scenarios, and audits
- `justfile` - short human-friendly command aliases
- `AGENTS.md` - AI-agent operating manual
- `docs/architecture/engineering-baseline.md` - current engineering boundaries, audit gates, and refactor order

## Closed-Loop Output

Each loop writes an iteration directory like:

- `screenshots/iter_NN/iter_NN.png`
- `screenshots/iter_NN/final_state.json`
- `screenshots/iter_NN/diff.json`
- `screenshots/iter_NN/health.json`
- `screenshots/iter_NN/assertions.json`
- `screenshots/iter_NN/decision.template.md`
- `screenshots/iter_NN/decision.md` after AI review

Read `health.json` first. If the result is `PARTIAL` or `FAIL`, read `assertions.json`
next for the machine-readable failure reasons before opening the PNG. The next iteration
should not proceed without a completed `decision.md`.

## Modeling Workflow

Model generation must be reproducible from Python scripts and Blender. Use:

```sh
blender --background --python tools/build_all_models.py
blender --background --python tools/create_eco_models.py
```

On this Windows workstation the Blender launcher is `F:\BLENDER\blender-launcher.exe`; keep that
as local setup knowledge, not as a committed workflow dependency.

After generating models, validate assets:

```sh
python tools/validate_pretty_glbs.py
python tools/verify_poly_budget.py
```

Do not commit Python caches, Blender backup files, temporary exports, or local absolute-path config.

## Development Rules

- Use Bevy 0.19 APIs such as `Mesh3d` and `MeshMaterial3d`.
- Do not use deprecated `PbrBundle` or `MaterialMeshBundle`.
- Share mesh and material handles for repeated block types.
- Throttle logs in systems that run every tick.
- Prefer tests in `crates/core` for game rules and state transitions.
- Keep changes small and tied to the current task.

## Current Status Signals

The demo has visible terrain, player, HUD, monsters, ecology objects, resources, and observer state. The latest loop state should be judged from the newest `screenshots/iter_*` directory, not from stale README claims.
