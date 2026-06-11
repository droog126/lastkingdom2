# AGENTS.md

Bevy 0.18.1 voxel game demo ("万国起源：最后一国 钻石版") with **closed-loop AI iteration** — the game runs itself, captures screenshots, AI reads results and decides the next change. No human in the loop per iteration.

## Setup commands

- Install deps: `cargo build --workspace` (Rust 1.75+, edition 2024)
- Start dev: `$env:BEVY_DISABLE_ACCESSIBILITY="1"; $env:RUST_LOG="info"; cargo run -p lk2-client -- --offline` (auto-demo / offline loop)
- Build: `cargo build --workspace` (~22 min cold from scratch, ~1 s incremental)
- Test: `cargo test --workspace`
- Lint: `cargo clippy --workspace`
- Format: `cargo fmt` (style in `rustfmt.toml`)

## Closed-loop iteration (核心)

The project's defining workflow. **Read [`Agent.md`](./Agent.md) before changing anything** — it documents the 4-phase observe → decide → act → build → re-run loop and lists required code infrastructure.

Quickstart:

```powershell
.\loop.ps1                           # build + 12s run + capture iter_NN.png + state_NN.json
Get-ChildItem screenshots\iter_*\iter_*.png | Sort LastWriteTime -Descending | Select -First 3
```

## Project layout

- `crates/client/src/main.rs` — Bevy client entry, HUD, screenshot, offline demo, client-side render/input
- `crates/server/src/main.rs` — headless server entry, self-check, authority sim, UDP listen
- `crates/core/src/` — shared sim/data/protocol modules (`world`, `ai`, `scenario`, `monster`, `nation`, `resource`, ...)
- `scenarios/` — scenario JSON files (test scripts)
- `screenshots/` — output of `loop.ps1` (PNG + state JSON)
- `document/` — design notes (Blender export workflow, etc.)
- `assets/` — art / 3D models
- `loop.ps1`, `run_scenario.ps1` — closed-loop drivers
- `Agent.md` — the project's AI-agent operations manual (read this first)

Legacy note: the old root `minecraft_bevy` package and `launchers/` wrappers were removed. Do not route new work through them.

## Code style

- `rustfmt.toml` — `max_width = 100`, `comment_width = 100`, `tab_spaces = 4`, `use_field_init_shorthand = true`, `newline_style = "Unix"`
- Bevy 0.18.1 patterns: use `Mesh3d` / `MeshMaterial3d` components, NOT the deprecated `PbrBundle` / `MaterialMeshBundle`
- Share `Handle<Mesh>` / `Handle<StandardMaterial>` across blocks of the same type (GPU state changes are expensive)
- `Cargo.toml` pins `compt = ">=1.9, <1.10"` — broccoli 0.6 does NOT compile with compt 1.10. Do not bump it
- `info!` / `warn!` in a system that runs every tick should be throttled (use `Local<u32>` to dedupe per tick)

## Testing instructions

- Unit tests: `cargo test --workspace` (cargo's built-in test framework)
- Visual / scenario validation: `loop.ps1`, then read the latest `iter_NN.png` + `state_NN.json`
- All scenarios must complete or fail with a clear reason — never dead-loop on `OUT OF BOUNDS` or spam `体素过多` warnings
- All tests + a fresh `loop.ps1` run must pass before opening a PR

## PR & commit conventions

- Branch from `master` (current dev branch); CI builds from `main` after merge
- Commit message: conventional commits (`feat:` / `fix:` / `docs:` / `refactor:` / `chore:`)
- Open PR via `gh pr create` once local `cargo build` + `loop.ps1` are green

## Security

- No secrets in the repo. Network code exists for local multiplayer, but no auth/secrets flow
- Do not commit `target/`, `screenshots/iter_*.png`, `*.log` — most are already in `.gitignore`; double-check with `git status` before pushing
