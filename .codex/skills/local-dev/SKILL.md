---
name: local-dev
description: Local development workflow for the lastkingdom2 Rust/Bevy workspace. Use when Codex needs to inspect the repo, make ordinary code changes, run local build/dev commands, respect existing user changes, choose validation scope, or prepare branch/PR/commit work that is not specifically closed-loop, TDD-first, Blender modeling, or Bevy gameplay focused.
---

# Local Dev

## Workflow

1. Define the task boundary before editing: what problem is being solved, which files are likely relevant, and what result counts as done.
2. Inspect the current worktree first. Treat all existing modified, deleted, or untracked files as user work unless you created them in this turn.
3. Read the nearest relevant code, tests, scripts, and docs before choosing an implementation.
4. Make the smallest coherent change. Do not mix unrelated refactors, cleanup, or visual polish into the task.
5. Pick the narrowest validation command that covers the changed surface.
6. Summarize what changed, how it was validated, and what risk remains.

## Repo Map

- `crates/core/src/`: shared simulation, data, protocol, AI, scenario, monsters, nations, resources.
- `crates/client/src/main.rs`: Bevy client entry, HUD, screenshot, offline demo.
- `crates/client/src/render/`: render, camera, smooth mesh, auto-demo presentation.
- `crates/server/src/main.rs`: headless server, self-check, authority simulation, UDP listen.
- `scenarios/`: scenario JSON scripts.
- `screenshots/`: closed-loop output.
- `docs/`: design notes and plans.
- `assets/`: art and 3D models.
- `xtask/`: Rust task runner for loop, health, TDD scopes, scenarios, and audits.
- `justfile`: short cross-platform aliases for the Rust task runner.

Active docs:

- `AGENTS.md`: project skill routing and synchronization policy.
- `docs/architecture/engineering-baseline.md`: current engineering boundaries and automation ownership.
- `docs/STARTING.md`: current run guide.
- `docs/README.md`: docs map.

Legacy note: do not route work through removed `minecraft_bevy`, `launchers/`, root PowerShell workflow wrappers, or `scripts/` workflow-runtime paths.

## Tooling Choices

Prefer durable automation in this order:

1. Rust `xtask`: use for core closed-loop orchestration, test runners, state files, JSON contracts, cross-platform command logic, and robust error handling.
2. Python: use for Blender, asset generation, and focused one-off analysis. Do not add Python as the loop workflow runtime.
3. `justfile`: use only for short command aliases. Do not put complex branching, state files, retries, or loop control in `justfile`.

Do not add PowerShell workflow wrappers. Durable workflow logic belongs in Rust `xtask`.

## Commands

Install/build:

```sh
cargo build --workspace
```

Start offline client:

```sh
export BEVY_DISABLE_ACCESSIBILITY=1
export RUST_LOG=info
cargo run -p lk2-client -- --offline
```

Common validation entry points:

```sh
just test-changed
just test
just audit-tdd
just fmt
just clippy
```

Closed-loop entry points:

```sh
just loop
just health
cargo run -q -p xtask -- loop --offline --seconds 60
cargo run -q -p xtask -- health
```

Use dev dynamic linking only for local client/server development when the repo scripts expect it. Do not use it for release or CI validation.

## Worktree Rules

- Never revert files you did not change unless explicitly asked.
- Do not delete or move generated-looking files unless the task is to clean them and the target is confirmed.
- Do not commit `target/`, `screenshots/iter_*.png`, `*.log`, `__pycache__/`, Blender backups, or local absolute-path config.
- Check `git status --short` before and after edits when preparing a final summary.

## Quality Bar

- Keep changes small and explainable in one sentence.
- Use existing abstractions and local patterns.
- Do not introduce duplicate rule tables.
- Do not rely on local environment variables, current time, or random ordering to pass tests.
- Throttle per-tick logs with a local counter or equivalent dedupe.
- Return clear errors; do not silently ignore failure.
