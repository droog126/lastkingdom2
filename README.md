<!-- doc-status: current -->
# Wanguo Origins: Last Kingdom Diamond

Bevy 0.19 playable forest MVP with shared simulation and combat rules, a focused offline client,
and a separate server authority crate under migration.

Read [AGENTS.md](AGENTS.md) before changing the repository. Human-facing documentation starts at
[docs/README.md](docs/README.md), and the run guide is [docs/STARTING.md](docs/STARTING.md).

## Quick start

```sh
just build
just offline
just export
```

The normal play aliases route through `xtask play`, which builds when needed and archives runtime
errors under `run-logs/`.

`just export` writes a stable content package to the current directory. It includes the
human-readable `content.md` catalog, complete registry JSON, active-content and planned-content
JSON/CSV views, recipes, and a manifest. `just export-content` remains an alias. Use
`just export --out=PATH` to choose another output directory.

`just loop` runs the online closed loop with a focused rendered client and a second client. The
direct `just codex-client` command enters the complete Living Forest main scene and replaces human
input with Codex decisions. Use `just loop-offline` when an offline-only run is needed; `just loop-codex` is the
explicit Codex loop, while `just loop-ai` remains the deterministic local fallback.
The Codex path uses the local `codex exec` command with read-only sandboxing; authenticate it with
`codex login` first, or override the executable with `LK2_CODEX_BIN`.

After a loop, run `just milestone` to copy its latest primary PNG into the top-level
`milestones/` directory. Milestone captures are Git-visible and survive `just clean-runs`;
ordinary `screenshots/iter_NN/` output remains temporary and unchanged.
While playing the offline or online scene, press `F12` to capture the current game window directly
to the same `milestones/` directory without touching loop artifacts.

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
- `crates/client/src/`: focused Bevy scene, input, offline authority, screenshots, and previews.
- `crates/server/src/`: headless authority simulation, networking, and server PvP.
- `xtask/`: Rust task runner for development, testing, audits, play, health, and loop orchestration.
- `justfile`: short human-friendly aliases for `xtask` and Cargo.
- `assets/`: runtime art and generated models.
- `tools/`: reproducible Blender/model generation and focused asset analysis.
- `scenarios/`: scenario JSON scripts.
- `screenshots/`: temporary closed-loop output.
- `milestones/`: durable, Git-visible copies of selected loop screenshots.
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

To prepare a model for AI visual optimization, render its front, side, and top views and
write a task bundle under screenshots/model_optimize/:

```powershell
just model-optimize animals/rabbit
```

Use an assets-relative path when a stem exists in more than one collection.

Use the project `$ai-modeling` skill for asset generation or manifest changes. Do not commit local
launcher paths, Python caches, Blender backups, temporary exports, or generated run logs.

## Documentation contract

Only files marked `doc-status: current` are maintained as operational facts. Reference and proposal
documents may preserve old designs, so revalidate them against code before implementation. Run
`just audit-docs` after documentation, dependency, command, or artifact-contract changes.
