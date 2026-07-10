---
name: docs-governance
description: Maintain and audit lastkingdom2 documentation classification, current facts, links, commands, dependency claims, and runtime artifact contracts. Use when editing README or docs files, changing Cargo/justfile/xtask workflows that affect human guidance, reviewing stale or contradictory documentation, moving material between current/reference/proposal/archive status, or fixing `audit-docs` failures.
---

# Docs Governance

Keep operational facts small, verifiable, and distinct from design history.

## First reads

1. Read `docs/README.md` for the document classification and complete index.
2. Read the nearest code, `Cargo.toml`, `justfile`, or `xtask` implementation that owns the fact.
3. Read the current guide being changed and any affected domain skill.
4. Check `git status --short`; preserve unrelated user work.

Use evidence in this order: current code and `xtask`, Cargo/justfile configuration, applicable
project skill, current document, then reference/proposal material.

## Status contract

Every non-archived Markdown file under `docs/` starts with exactly one marker:

```text
<!-- doc-status: current -->
<!-- doc-status: reference -->
<!-- doc-status: proposal -->
```

- `current`: maintained operational fact. Keep commands, paths, versions, and artifact names exact.
- `reference`: background or design material that may differ from code. Add a visible warning when
  readers could mistake it for current behavior.
- `proposal`: uncommitted work. Do not mark phases complete without implementation evidence.
- `docs/archive/`: historical material; do not modernize it merely to satisfy current guidance.

Only these human documents are required to remain current: root `README.md`, `docs/README.md`,
`docs/STARTING.md`, `docs/architecture/engineering-baseline.md`, `docs/notes/tdd.md`, and
`docs/plans/closed-loop-iteration.md`.

## Editing rules

- Update `docs/README.md` whenever a non-archived document is added, removed, renamed, or
  reclassified. Use resolvable Markdown links.
- Do not copy changing test counts, latest health verdicts, FPS claims, local paths, or temporary
  roadmap state into current documents.
- Do not duplicate dependency constraints outside Cargo. A current document may summarize the
  workspace versions but must identify Cargo as the owner.
- When commands change, update `justfile`, `xtask` help, current run docs, and affected skills in
  one change.
- When loop files or JSON contracts change, update `STARTING.md`, the closed-loop contract, and
  `$closed-loop-ai-dev` together.
- Preserve useful old detail by reclassifying it as reference/proposal instead of silently
  presenting it as current.
- Keep machine-local absolute paths out of current documentation.

## Validation

Run:

```sh
just audit-docs
just audit-skills
```

`just test-changed` also invokes these audits when changed paths touch documentation contracts,
project skills, Cargo, `justfile`, or `xtask`.

For `xtask` audit behavior changes, use `$tdd-iteration`, add a failing unit test first, then run:

```sh
cargo test -p xtask
just fmt
```

An `audit-docs` failure is fixed by correcting the owning fact or classification, not by weakening
the check solely to accept stale guidance.

## Completion

Finish only when document status and index coverage are complete, local links resolve, current
facts match their owners, affected skills are synchronized, and the documentation and skill audits
pass.
