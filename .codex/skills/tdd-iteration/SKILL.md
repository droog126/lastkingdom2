---
name: tdd-iteration
description: Test-driven development workflow for lastkingdom2. Use when changing pure logic, rules, state machines, resources, drops, transfers, conservation, nations, monsters, animals, combat, protection periods, phase timing, CLI/protocol/network parsing, AI decisions, scenarios, tick observers, bug fixes, regressions, or any behavior that can be tested before implementation.
---

# TDD Iteration

## Workflow

1. Reproduce or specify the behavior with a failing test first.
2. Confirm the failure is for the expected reason.
3. Implement the smallest change that makes the test pass.
4. Add boundary tests for the risky edge.
5. Run the narrowest relevant test scope.
6. Summarize the failing test, fix, and validation result.

Prefer tests in `crates/core` because they are fastest and most stable.

## Must Test

Add or update tests for:

- resource add/remove/drop/transfer/conservation
- nations, monsters, animals, combat, protection periods, phase timing
- CLI, protocol, and network parameter parsing
- AI decisions
- scenario progression
- tick observer invariant checks
- regressions for previously broken edge cases

## Test-Optional Cases

It is acceptable to skip test-first only for:

- pure visual parameter tuning
- temporary debug logging
- screenshot composition, lighting, or camera angle tweaks

Even for visual tasks, test any pure function, state transition, or data selection logic behind the visual.

## Commands

Choose by changed surface:

```powershell
.\tdd.ps1 -Scope core
.\tdd.ps1 -Scope client
.\tdd.ps1 -Scope server
.\tdd.ps1 -Scope changed
.\tdd.ps1 -Scope workspace
.\tdd.ps1 -Scope audit
.\tdd.ps1 -Scope fmt
```

Use `.\tdd.ps1 -Scope core` for pure core changes, `client` for client-only changes, `server` for server-only changes, and `workspace` for cross-crate or public API changes.

If the whole repo already has formatting drift, format only files touched in the current task and report the remaining `cargo fmt --check` issue.

## Constraints

- Do not mutate process environment variables in concurrent unit tests.
- Avoid `unwrap()` in production code unless a local invariant makes panic intentionally correct and obvious.
- Do not hide new warnings with broad `allow` attributes.
- Tests must not depend on wall-clock time, random iteration order, or local machine config.
