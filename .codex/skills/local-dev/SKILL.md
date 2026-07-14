---
name: local-dev
description: Fallback local-development workflow for lastkingdom2. Use for ordinary repository inspection, small code or configuration changes, validation selection, dirty-worktree handling, and git preparation when no more specific project skill covers the task. Do not load it automatically alongside a specific gameplay, TDD, loop, modeling, lifecycle, documentation, audit, or skill-authoring route.
---

# Local Dev

## Boundary

Use this as the primary fallback, not as a mandatory companion. If another project skill fully covers the request, follow that skill and apply only the universal worktree rules below.

## Workflow

1. Define the requested outcome and likely files before editing.
2. Run `git status --short`. Treat existing changes as user work.
3. Read the nearest implementation, tests, and active documentation.
4. Classify cleanup scope before deleting anything: source/config cleanup, generated artifacts,
   or both. Do not remove ignored caches or runtime outputs when the user asks for code cleanup.
5. For dependency cleanup, distinguish workspace-level candidates from crate dependencies. Confirm
   actual manifest consumers, feature use, `Cargo.lock`/`cargo tree` impact, and active-document
   intent before deleting a declaration; a missing source import alone is not proof of garbage.
6. Make the smallest coherent change without unrelated cleanup.
7. Run the narrowest validation that covers the changed surface.
8. Report the change, validation result, and remaining risk.

## Tooling

- Put durable workflow automation, state handling, and cross-platform command logic in Rust `xtask`.
- Use Python for Blender, asset generation, and focused one-off analysis, not loop orchestration.
- Keep `justfile` recipes as short aliases.
- Do not add root PowerShell workflow wrappers.

Prefer package-specific checks over workspace builds:

```sh
cargo check -p <crate>
cargo test -p <crate>
just test-changed
just fmt
just audit-skills
```

Do not enable Bevy dynamic linking by default on Windows; it can trigger large `bevy_dylib` linker failures.

## Worktree And Validation

- Never revert, delete, move, stage, or commit user changes unless explicitly requested.
- Scope diffs and formatting to files involved in the task.
- Do not commit runtime output, logs, screenshots, Blender backups, `__pycache__`, or local absolute-path configuration.
- If compilation is blocked, diagnose or narrow the check before making further speculative edits.
- Do not claim completion when the affected code was not validated; state the exact blocker instead.
- Before stopping a workspace process, inspect its command line and stop only a process clearly owned by the current task.
