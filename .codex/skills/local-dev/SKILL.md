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

## Rust Build Performance

Rust/Bevy builds can be slow in this workspace because Bevy, rendering, physics, networking, ECS generics, derive macros, and linking create a large dependency graph. On Windows/MSVC, linking large Bevy binaries is often a bottleneck, and `bevy/dynamic_linking` can fail with `bevy_dylib` linker limits such as `LNK1189`.

Prefer the narrowest command that matches the change:

- Use `cargo check -p <crate>` for type validation when a full binary is not needed.
- Use package-specific commands such as `cargo check -p lk2-server` or `cargo check -p lk2-client` instead of workspace-wide builds.
- Avoid switching feature sets repeatedly in the same target directory; toggling Bevy or Lightyear features can invalidate large parts of the cache.
- Keep server builds headless and avoid enabling client render features unless the task needs them.
- Do not enable `dev-dynamic-linking` by default on Windows; it may compile `bevy_dylib` and hit MSVC linker object limits.

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

## Recent Lessons

- Treat a dirty worktree as normal in this repo. Scope status/diffs to the files involved in the task and ignore unrelated asset/tool churn unless it blocks the change.
- When a loop run uses `--skip-build`, build the affected binary first. Otherwise screenshots may come from an old executable even though source changes and tests look correct.
- On Windows, `lk2-client.exe` can be locked by stale `lk2-client`, `cargo`, or model-preview processes. Check command lines before stopping anything, and only clear processes that are clearly from this workspace/task.
