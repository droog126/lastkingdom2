# docs/

Documentation is grouped by purpose.

- Root `AGENTS.md` routes agent work to project skills.
- `.codex/skills/*/SKILL.md` contains detailed agent operating rules.
- `architecture/engineering-baseline.md` is the active engineering baseline.
- `STARTING.md` is the current run guide.

- `architecture/` - engineering boundaries, system architecture, refactor plans.
- `plans/` - near-term and feature-specific implementation plans.
- `design/` - gameplay, world, content, and product design notes.
- `notes/` - working notes, TDD backlog, starting guide, imported practical notes.
- `archive/` - historical imports, drift reports, and material kept for reference.

Start with:

- `../AGENTS.md`
- `architecture/engineering-baseline.md`
- `STARTING.md`
- `plans/closed-loop-iteration.md`
- `notes/tdd.md`

When old plans mention `loop.ps1`, root PowerShell workflow scripts, `minecraft_bevy`, `launchers/`, or `scripts/` as the workflow runtime, treat that as historical unless an active skill or baseline file says otherwise. Current durable workflow automation lives in Rust `xtask`, with `justfile` as the short command layer.
