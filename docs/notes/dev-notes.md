<!-- doc-status: reference -->
# lastkingdom2 开发文档

本文保留为开发速记参考，不再承担日常入口。当前入口见 `docs/README.md` 和 `docs/STARTING.md`。

## 项目结构

```text
lastkingdom2/
├── crates/
│   ├── core/        # lk2-core: sim, data, protocol, AI, scenario
│   ├── client/      # lk2-client: Bevy render, input, HUD, screenshot
│   └── server/      # lk2-server: headless authority sim and UDP server
├── assets/          # art and generated runtime assets
├── scenarios/       # scenario JSON scripts
├── screenshots/     # loop output and ignored runtime data
├── run-logs/        # xtask build/run logs
├── docs/
│   ├── architecture/
│   ├── design/
│   ├── notes/
│   ├── plans/
│   └── archive/
├── tools/           # Blender/Python asset generation scripts
├── xtask/           # Rust automation entry point
└── justfile         # short command aliases
```

`document/`, `launchers/`, root PowerShell orchestration, and `scripts/` as a workflow runtime are legacy layouts. Durable workflow logic goes in `xtask/`.

## 常用命令

```powershell
just build
just test-changed
just test
just audit-tdd
just loop
just health
```

The real workflow implementation lives in `xtask/`. `justfile` is only a short alias layer.

```powershell
cargo run -q -p xtask -- loop --offline --seconds 60
cargo run -q -p xtask -- health
cargo run -q -p xtask -- scenario --json scenarios\iter07_flat_spawn.json
cargo run -q -p xtask -- dev build
cargo run -q -p xtask -- audit-architecture
```

## 开发边界

Put deterministic game rules in `crates/core`. Put rendering, camera, HUD, input, screenshots, and local client prediction in `crates/client`. Put authority-only simulation, server networking, and server self-checks in `crates/server`.

If a change touches gameplay, resources, AI, scenarios, networking, or state transitions, add or update a focused test first where practical. Prefer `crates/core` tests because they are fastest and avoid Bevy runtime cost.

## 闭环验证

After visual or gameplay changes, run:

```powershell
just loop
```

Then read the latest `screenshots/iter_NN/health.json` before opening the PNG. `health.json` tells whether the framebuffer/state is meaningful (`PASS`, `PARTIAL`, or `FAIL`). A loop iteration is not complete until its `decision.md` records what changed, what the screenshot/state showed, and the next step.

## 文档地图

- `docs/STARTING.md` - current run guide
- `docs/architecture/engineering-baseline.md` - current engineering rules and audit gates
- `docs/architecture/` - architecture notes and historical architecture plans
- `docs/design/` - gameplay/content/voxel design notes
- `docs/plans/` - active and historical implementation plans
- `docs/notes/` - day-to-day notes and TDD references
- `docs/archive/` - imported or legacy material kept for reference only
