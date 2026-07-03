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
- `loop.ps1`, `tdd.ps1`: root compatibility wrappers.
- `scripts/loop/`, `scripts/dev/`, `scripts/ci/`, `scripts/maintenance/`: project scripts.

Legacy note: do not route work through removed `minecraft_bevy` or `launchers/` paths.

## Commands

Install/build:

```powershell
cargo build --workspace
```

Start offline client:

```powershell
$env:BEVY_DISABLE_ACCESSIBILITY="1"
$env:RUST_LOG="info"
cargo run -p lk2-client -- --offline
```

Common validation entry points:

```powershell
.\tdd.ps1 -Scope changed
.\tdd.ps1 -Scope workspace
.\tdd.ps1 -Scope audit
cargo fmt
cargo clippy --workspace
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
