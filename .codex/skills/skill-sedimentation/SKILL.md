---
name: skill-sedimentation
description: Audit and improve reusable Codex instructions for lastkingdom2. Use when the user explicitly asks to sediment/沉淀 lessons, codify a repeated failure, create or update a skill, change AGENTS.md routing, optimize agent rules, or make future agents stop repeating a documented workflow mistake. Do not mutate skills merely because an ordinary task failed.
---

# Skill Sedimentation

## Authorization Boundary

- If the user explicitly requests skill or routing changes, implement them.
- If an ordinary task only reveals a possible reusable lesson, report a `sedimentation_candidate` without editing skills or `AGENTS.md`.
- Do not turn a one-off workaround, local machine detail, or isolated typo into a project skill.

## Classification

Before editing, record a concise working classification:

```text
sedimentation_trigger:
- evidence: <repeated failure, explicit request, or stable artifact>
- failure_mode: <routing, process, validation, or domain gap>
- route: update existing skill | complete existing draft | create new skill
- target_skill: <name>
```

Prefer updating an existing skill. Create a new skill only when the user explicitly requests it or when repeated evidence shows a distinct workflow with stable triggers that cannot fit an existing route.

## Workflow

1. Reconstruct the problem from user feedback, logs, diffs, skipped validation, or repeated outcomes.
2. Identify the smallest instruction change that prevents the same class of failure.
3. Keep `AGENTS.md` to routing and composition; keep operational detail in `SKILL.md`.
4. Ensure frontmatter describes both capability and all trigger conditions.
5. Update `agents/openai.yaml` so its prompt and UI text match the skill.
6. Update active human documentation only when the human-facing workflow changes.
7. Run `just audit-skills` and the narrowest tests for any changed automation.

Use the system `$skill-creator` skill for skill structure and metadata rules.

## Guardrails

- Select one primary skill and at most one validation companion in routing examples.
- Keep historical incidents out of always-loaded skill bodies when a test, `xtask` assertion, or optional reference can carry them.
- Do not place unfinished templates or TODO scaffolds under `.codex/skills/`.
- Keep review/audit skills read-only unless their description explicitly covers implementation and the user requested it.
- Do not duplicate machine-checkable thresholds in prose; make automation the source of truth.

## Completion

Finish only when the route, skill body, metadata, and active docs agree; `just audit-skills` passes; and validation blockers are reported exactly.
