<!-- doc-status: current -->
# Wanguo Origins: Last Kingdom Diamond

Bevy 0.19 voxel simulation and rendering demo with a three-crate client/server workspace and a
Rust-driven closed-loop iteration workflow.

Read [AGENTS.md](AGENTS.md) before changing the repository. Human-facing documentation starts at
[docs/README.md](docs/README.md), and the run guide is [docs/STARTING.md](docs/STARTING.md).

## Quick start

```sh
just build
just offline
```

The normal play aliases route through `xtask play`, which builds when needed and archives runtime
errors under `run-logs/`.

Closed-loop run:

```sh
just loop
```

Useful validation entry points:

```sh
just test-core
just test-changed
just test
just audit-tdd
just audit-docs
just audit-skills
just fmt
```

## Project layout

- `crates/core/src/`: shared simulation, rules, protocol, world, AI, resources, and combat.
- `crates/client/src/`: Bevy client, rendering, HUD, input, screenshots, and offline demo.
- `crates/server/src/`: headless authority simulation, networking, and server PvP.
- `xtask/`: Rust task runner for development, testing, audits, play, health, and loop orchestration.
- `justfile`: short human-friendly aliases for `xtask` and Cargo.
- `assets/`: runtime art and generated models.
- `tools/`: reproducible Blender/model generation and focused asset analysis.
- `scenarios/`: scenario JSON scripts.
- `screenshots/`: closed-loop output.
- `run-logs/`: play/build logs and extracted error archives.
- `docs/`: current guides, reference material, proposals, and archives.
- `.codex/skills/`: project-specific agent workflows.

## Closed-loop output

Each loop writes `screenshots/iter_NN/` with the primary evidence below:

- `iter_NN.png`
- `final_state.json`
- `diff.json`
- `assertions.json`
- `health.json` and `health.txt`
- `error_logs.json` and `error_logs.txt`
- `perception_manifest.json`
- `regression.json`
- `decision.template.md`
- `decision.md` after review

Read `health.json` first. For `PARTIAL` or `FAIL`, inspect `assertions.json` before diagnosing from
the PNG. A later iteration is gated on the previous iteration having `decision.md`.

## Modeling workflow

Model generation is reproducible through Blender scripts under `tools/`:

```sh
blender --background --python tools/build_all_models.py
blender --background --python tools/create_eco_models.py
python tools/validate_pretty_glbs.py
python tools/verify_poly_budget.py
```

Use the project `$ai-modeling` skill for asset generation or manifest changes. Do not commit local
launcher paths, Python caches, Blender backups, temporary exports, or generated run logs.

## Documentation contract

Only files marked `doc-status: current` are maintained as operational facts. Reference and proposal
documents may preserve old designs, so revalidate them against code before implementation. Run
`just audit-docs` after documentation, dependency, command, or artifact-contract changes.
