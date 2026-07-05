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
- finite ecology/resource conversions, especially "does not grow when the required source pool is empty" and "does not consume when the destination pool is full"
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

```sh
just test-core
just test-client
just test-server
just test-changed
just test
just audit-tdd
just fmt
```

Use `just test-core` for pure core changes, `just test-client` for client-only changes, `just test-server` for server-only changes, and `just test` for cross-crate or public API changes.

If the whole repo already has formatting drift, format only files touched in the current task and report the remaining `cargo fmt --check` issue.

## Constraints

- Do not mutate process environment variables in concurrent unit tests.
- Avoid `unwrap()` in production code unless a local invariant makes panic intentionally correct and obvious.
- Do not hide new warnings with broad `allow` attributes.
- Tests must not depend on wall-clock time, random iteration order, or local machine config.
- For demo ecology, test both catalog variety and default placement when visibility matters. "Entity exists in state" is not enough if the gameplay proof depends on initial camera readability.
