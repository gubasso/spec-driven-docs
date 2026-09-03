---
name: sdd-migrate
description: Drives a repository from however it documents itself today to the current spec-driven-docs convention, with a full greenfield sweep as the default. Use when asked to migrate a project onto spec-driven docs, convert an existing docs or specs convention, adopt sdd in a repository that already documents itself, upgrade an older instance, or reconcile a drifted one. Triggers include migrate to sdd, adopt spec-driven docs, convert docs conventions, docs migration, and sdd upgrade.
license: CC-BY-4.0
compatibility: Requires the sdd binary on PATH; install with cargo install spec-driven-docs or cargo binstall spec-driven-docs. Requires pre-commit for the closing gate run. Every sdd verb runs offline.
---

# sdd-migrate

Take a repository from however it documents itself today to the current spec-driven-docs convention. The default is the full sweep: every durable fact the project has written down ends the migration in exactly one sdd home, and the old convention is retired. Narrow the scope only when the request narrows it.

## Before acting

Read two shared files before the first action of a task, in this order, and hold both for the whole task.

1. `~/.local/state/spec-driven-docs/skills/shared/pre-flight-gate.md` — run it whatever the request carries. It checks this host with `sdd doctor` and stops the task on what no plan can work around. No flag skips it.
2. `~/.local/state/spec-driven-docs/skills/shared/plan-gate.md` — it binds three phases: plan and present the plan for approval, validate that plan against every preview and read-only source phase 2 names, then execute it.

The two gates are why this skill is safe to run unattended: the sweep rewrites and retires files the project authored, nothing restores them but version control, and the pre-flight says whether this host carries the tools the sweep leans on.

When the request carries `--no-plan`, skip the plan gate's approval turn only. Still run the pre-flight, still state the ordered plan before acting, and still validate it as phase 2 directs.

## Detect

`sdd status --target . --json` routes the work:

- `"instance": false`: run the greenfield sweep below.
- An instance older than the binary: follow the version path below.
- A current instance reporting drift: `sdd verify --target .`, then fix what it cites, smallest first.

## The greenfield sweep

1. Inventory every place the project documents itself: the docs root, README files, decision directories, scattered specs, contributor guides, wiki exports. List every file; the list is part of the plan the gate presents.
2. Classify each durable fact by the placement chapter's decision procedure — `sdd method 01-placement` serves it. What binds now becomes a spec; why-at-the-time becomes a decision record; task recipes become guides, started from `sdd template guide` and written to the adopted guides spec; exact values become reference; the rest is explanation, or not durable at all.
3. Land the instance: choose the profile — `codebase` keeps records under `docs/`, `knowledge-base` under `_docs/` — preview `sdd init --target "$PWD" --profile <profile>`, review the listed paths, then re-run with `--apply`, and wire the documentation section into the project's `AGENTS.md` exactly as the sdd-setup skill states it.
4. Rewrite, never move. Promotion into a zone is a rewrite into that zone's format and budgets, not a file move; old prose usually splits into a spec half stating what holds now and a decision half stating why it was chosen.
5. Retire the old convention. Remove each superseded file only after its facts are placed, and only when the approved plan named the removal; anything not yet placed goes to `.draft/` rather than being dropped.
6. Close: `sdd verify --target .`, then `pre-commit run --all-files` for the delivered gates.

## From a previous version

1. Install the newer `sdd` first, then `sdd upgrade --target . --dry-run` and read the report before anything writes.
2. `sdd upgrade --target .` applies. A locally edited managed file aborts the whole upgrade with every conflict listed in one run; reconcile, then re-run rather than working around the refusal.
3. Re-run `sdd verify --target .`; a failure message cites a `domain:rule` ID, and `sdd spec <domain>` states the sentence to satisfy.

## Defaults

- The full sweep is the default scope; a partial migration is an explicit narrowing, stated in the plan.
- No durable fact is lost: each one is rewritten into its owner, parked in `.draft/`, or retired with the operator's approval.
- Prefer an sdd verb over hand-editing anything the instance manifest owns; everything under `.spec-driven-docs/` belongs to the canon.
- Report every step's outcome from observation, never from memory of what the command should do.
