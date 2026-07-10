---
name: tdd-iteration
description: Test-first workflow for deterministic lastkingdom2 behavior. Use when implementing or fixing rules, state machines, resource conservation, combat, phases, parsing, protocol values, AI decisions, scenarios, observer invariants, or a regression that can be reproduced with a focused automated test. Do not use for purely visual tuning or runtime-only investigation with no stable test seam.
---

# TDD Iteration

## Workflow

1. Add the narrowest test that specifies the requested behavior.
2. Run it and confirm it fails for the intended reason.
3. Implement the smallest behavior change that passes it.
4. Add a boundary or regression case for the risky edge.
5. Run the focused test, then the narrowest affected package scope.
6. Report the red test, implementation, and green validation.

Prefer pure tests in `crates/core` when the behavior belongs to shared simulation. Do not move presentation or authority logic into core merely to make it easier to test.

## Required Coverage

Test deterministic changes to:

- resource production, consumption, transfer, caps, drops, and conservation
- nations, monsters, animals, combat, protection periods, and phase timing
- CLI, scenario, protocol, and network-value parsing
- AI decisions and scenario progression
- observer invariants and previously broken edge cases

For Bevy scheduling or runtime wiring, keep the pure rule test here and use `$bevy-gameplay-dev` only for the runtime-specific portion.

## Commands

```sh
just test-core
just test-client
just test-server
just test-changed
just test
just audit-tdd
```

Use the smallest command that covers the change. If repository-wide formatting already drifts, format only touched files and report the unrelated drift.

## Test Integrity

- Do not depend on wall-clock timing, random iteration order, mutable process environment, or local machine configuration.
- Avoid broad warning suppressions and production `unwrap()` without an intentional invariant.
- Do not invent a brittle test solely to satisfy test-first; report when the behavior has no stable test seam and use the relevant runtime validation route.
