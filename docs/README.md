<!-- doc-status: current -->
# Documentation map

Documents are classified by the first-line `doc-status` marker:

- `current`: maintained workflow or engineering fact; checked by `just audit-docs`.
- `reference`: useful background that may not match the current code.
- `proposal`: an uncommitted plan; revalidate it before implementation.
- `docs/archive/`: historical imports and completed migrations; not audited as current guidance.

## Current documents

- [Run guide](STARTING.md)
- [Engineering baseline](architecture/engineering-baseline.md)
- [TDD workflow](notes/tdd.md)
- [Closed-loop contract](plans/closed-loop-iteration.md)

Agent routing lives in [AGENTS.md](../AGENTS.md), with operational instructions under
[project skills](../.codex/skills/). Code, Cargo metadata, `justfile`, and `xtask` behavior take
precedence when a current document drifts.

## Reference documents

- [Architecture snapshot](architecture/architecture.md)
- [Game architecture snapshot](architecture/game.md)
- [Server target architecture](architecture/server.md)
- [Content design](design/content.md)
- [Gameplay design](design/gameplay-v1.md)
- [Imported ECS implementation proposal](design/kimi-gameplay.md)
- [Product and architecture vision](design/overview.md)
- [Developer notes](notes/dev-notes.md)
- [Blender export notes](notes/document-notes/blender-export-and-character-workflow.md)

## Proposals

- [Architecture evolution plan](architecture/architecture_plan_v2.md)
- [Four-agent natural-world refactor plan](plans/natural-world-parallel-refactor.md)

## Maintenance

Run these after changing documentation, project skills, commands, dependencies, or artifact
contracts:

```sh
just audit-docs
just audit-skills
```

`just test-changed` invokes the relevant documentation and skill audits automatically when its
changed-file set touches these contracts.

Every non-archived Markdown file under `docs/` must be listed here and have one status marker.
Current documents must not contain machine-local absolute paths or legacy workflow facts. Local
Markdown links must resolve.
