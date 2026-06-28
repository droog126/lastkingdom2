# scripts/

Project automation is grouped by responsibility. Run commands from the repo root unless noted.

## Public Entry Points

Use the root wrappers for normal work:

```powershell
.\tdd.ps1 -Scope changed
.\loop.ps1 -Offline -Seconds 12
```

The root wrappers delegate to the implementation scripts below so old commands keep working.

## Layout

- `scripts/loop/` - closed-loop game runs, scenarios, visual health checks.
- `scripts/dev/` - local developer convenience commands.
- `scripts/ci/` - audit scripts used by `.\tdd.ps1 -Scope audit`.
- `scripts/maintenance/` - one-off migration and repository inspection helpers.

## Common Commands

```powershell
.\tdd.ps1 -Scope core
.\tdd.ps1 -Scope changed
.\tdd.ps1 -Scope audit
.\loop.ps1 -Offline -Seconds 12
powershell -File scripts\loop\run_scenario.ps1 -Json scenarios\iter07_flat_spawn.json
powershell -File scripts\maintenance\check_lock.ps1
```

## Rules

- Scripts must locate the repo root relative to `$PSScriptRoot`; no local absolute paths.
- Runtime logs go to `run-logs/`.
- Closed-loop visual output goes to `screenshots/`.
- Root scripts should remain thin compatibility wrappers.
