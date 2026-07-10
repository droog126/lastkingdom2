---
name: game-logic-audit
description: Evidence-backed, read-only audit workflow for lastkingdom2 game logic. Use when the user asks to inspect, review, or rank bugs in end-to-end rules, authority flow, simulation reachability, client/core/server divergence, scenario progression, conservation, combat, AI, or systems that compile but may not run. Do not use it as an implementation workflow unless the user separately asks for fixes.
---

# Game Logic Audit

## Contract

Audit before proposing changes. Do not edit code, assets, scenarios, or configuration unless the request explicitly includes implementation.

## Workflow

1. Define the audit boundary: targeted subsystem or repository-wide logic review.
2. Inspect `git status --short` and exclude unrelated user changes from conclusions.
3. Read `docs/architecture/engineering-baseline.md` and the relevant entry points.
4. Trace behavior end to end rather than reviewing isolated functions:
   - input or scenario trigger
   - client/server authority boundary
   - shared core rule or state transition
   - ECS schedule or caller reachability
   - replication, HUD, observer, or runtime evidence
5. Compare implementation with tests, scenario fixtures, assertions, and current runtime artifacts when available.
6. Run focused read-only checks only when they materially increase confidence.
7. Report findings by severity with exact file/line evidence, impact, and the missing or contradictory path.

## Audit Questions

- Can the system be reached from a registered startup, schedule, scenario, command, or network message?
- Is authority implemented once, or forked between offline client and server?
- Are state transitions complete at caps, empty inputs, phase boundaries, death, disconnect, and retry paths?
- Do resource and combat paths conserve values and share rule tables?
- Does replication expose authoritative state without clients inventing it locally?
- Do tests prove behavior, or only construct unused helpers?
- Do runtime artifacts demonstrate cause and effect rather than mere entity existence?

## Evidence Standard

- Treat a concrete failing path, unreachable registration, violated invariant, test contradiction, or runtime artifact as a finding.
- Label plausible but unproven concerns as questions, not bugs.
- Do not lower severity because code compiles, and do not raise severity because code looks unfamiliar.
- If no actionable findings remain, say so and name the inspected surfaces and validation limits.

## Handoff

For an explicitly requested fix, use `$tdd-iteration` for deterministic behavior and `$bevy-gameplay-dev` for ECS/runtime wiring. Keep the audit report separate from the implementation diff.
