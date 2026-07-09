---
name: skill-sedimentation
description: Convert repeated agent failures, bug-prone workflows, sedimentation/"沉淀" requests, postmortems, recurring validation gaps, or "next time do not repeat this mistake" feedback into reusable Codex skills. Use when the user asks to codify lessons, write routing rules, create a new problem-solving skill, update AGENTS.md skill routing, or turn a concrete mistake such as Bevy resource leaks, missing runtime validation, log spam, stale builds, or dirty-worktree confusion into durable project instructions.
---

# Skill Sedimentation

## Purpose

Turn a concrete failure pattern into durable routing plus an operational skill. Prefer updating an existing skill when the lesson is narrow; create a new skill when the pattern is reusable, cross-cutting, and needs its own trigger surface.

## Trigger Logic

Trigger this skill when the current request or recent conversation has at least one hard trigger or a score of 3 or more from the weighted signals below.

Hard triggers:

- The user explicitly asks to "sediment", "沉淀", "codify", "write a skill", "create a skill", "update skill routing", "turn this into a reusable rule", "do not repeat this next time", or equivalent wording.
- The user complains about repeated agent mistakes, missing validation, poor process memory, or asks why the same class of bug keeps happening.
- A completed or failed task reveals a reusable workflow gap that should change future agent behavior, not only the current code.

Weighted signals:

- +2: the same failure mode happened more than once, or the user says it is recurring.
- +2: the issue crosses more than one existing route, such as Bevy plus closed-loop validation.
- +2: the failure was caused by process, validation, or routing, not by a single typo.
- +1: the evidence includes stable trigger text, log patterns, file paths, commands, or code smells.
- +1: the fix requires a checklist or required command sequence future agents should follow.
- +1: the current skill descriptions do not already mention the trigger clearly.

Do not trigger:

- The user only asks to fix one concrete bug and does not ask for process codification.
- The lesson is a temporary workaround, local environment detail, or one-off fact.
- The right change is simply adding a test or editing code under an already-triggered skill.

When triggered, first produce a route classification:

```text
sedimentation_trigger:
- evidence: <user phrase/log/file/failed validation>
- failure_mode: <process gap>
- route: update existing skill | create new skill
- target_skill: <skill-name>
- composition: <other skills to combine>
```

Then perform the workflow below. Keep the classification concise; it is a working note, not a postmortem document.

## Workflow

1. Reconstruct the failure from concrete evidence:
   - user feedback
   - prior assistant actions
   - failing logs, commands, screenshots, diffs, or tests
   - files touched and validation skipped or failed
2. Classify the lesson before editing:
   - domain: local development, TDD/rules, Bevy gameplay/rendering, closed-loop screenshots, screenshot scoring, asset/model generation, automation, git hygiene, or meta-skill creation
   - failure mode: missing first-read, wrong API assumption, resource lifecycle leak, incomplete validation, stale binary, log spam, visual evidence gap, dirty-worktree confusion, or unsafe workflow
   - trigger signals: exact user phrases, log patterns, file paths, commands, or code smells that should activate the new or updated skill
3. Decide update vs create:
   - Update an existing skill when the lesson belongs cleanly to one existing route.
   - Create a new skill when at least two existing routes need the same guardrail, the user explicitly asks for a new skill, or the lesson needs its own intent recognition.
   - Do not create a skill for one-off facts, temporary workarounds, or broad advice without an executable workflow.
4. Implement the route:
   - Add or update `.codex/skills/<skill-name>/SKILL.md`.
   - Add `agents/openai.yaml` with a short display name, short description, and default prompt.
   - Update `AGENTS.md` routing and composition rules when this is a project-level skill.
   - Update active docs only if the new skill changes human-facing workflow. Do not edit archive docs.
5. Validate:
   - Run the skill validator from the system `skill-creator` skill when available:
     `python C:\Users\98185\.codex\skills\.system\skill-creator\scripts\quick_validate.py <skill-dir>`
   - Read the generated `SKILL.md` and verify the frontmatter description contains all trigger conditions.
   - Check `AGENTS.md` mentions the new skill path when project routing is required.

## New Skill Contract

Every new problem-solving skill must include:

- Frontmatter with only `name` and `description`.
- A description that names the task and the trigger conditions. The body is too late for intent recognition.
- A compact workflow with required first reads, implementation rules, validation commands, and completion criteria.
- Concrete log/code/user-message trigger examples when relevant.
- Clear composition guidance if the skill should be combined with existing skills.

Avoid:

- generic "be careful" advice
- README or extra docs inside the skill
- broad refactors while creating the skill
- scripts unless the workflow repeats deterministic operations that agents would otherwise rewrite

## Project Routing Pattern

For this repository, use these routing edits in `AGENTS.md`:

- Add the skill under `Skill Routing` with a one-line "Use `$skill-name` for ..." rule.
- Add composition rules only when the skill must be combined with another skill.
- Add the path under `Skill Files`.
- Keep `AGENTS.md` a routing table. Put operational details in the skill itself.

## Skill Naming

Use short lowercase hyphen-case names. Prefer names that identify the failure class:

- `bevy-resource-lifecycle`
- `runtime-validation`
- `loop-artifact-triage`
- `git-worktree-hygiene`
- `log-spam-control`

## Completion

Finish only after:

- the skill file exists or the existing skill has been updated
- route classification is reflected in `AGENTS.md` when project-level routing is needed
- validation has passed, or the validation blocker is stated with the exact command and error
- the final response names the created/updated skill and the trigger it now handles
