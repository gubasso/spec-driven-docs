---
name: sdd-migrate
description: Drives a repository from however it documents itself today to the current spec-driven-docs convention, with a full corpus sweep as the default. Use when asked to migrate a project onto spec-driven docs, convert an existing docs or specs convention, adopt sdd in a repository that already documents itself, upgrade an older instance, or reconcile a drifted one. Triggers include migrate to sdd, adopt spec-driven docs, convert docs conventions, docs migration, and sdd upgrade.
license: CC-BY-4.0
compatibility: Requires the sdd binary on PATH; install with cargo install spec-driven-docs or cargo binstall spec-driven-docs. Requires pre-commit for the closing gate run. Every sdd verb runs offline.
---

# sdd-migrate

Take a repository from however it documents itself today to the current spec-driven-docs convention. The default is the full corpus sweep: every durable fact the project has written down ends the migration in exactly one sdd home, and the old convention is retired. Narrow the scope only when the request narrows it. The procedure this skill drives is `sdd method 12-migration`; read it once per task and hold it.

## Before acting

Read two shared files before the first action of a task, in this order, and hold both for the whole task.

1. `~/.local/state/spec-driven-docs/skills/shared/pre-flight-gate.md` — run it whatever the request carries. It checks this host with `sdd doctor` and stops the task on what no plan can work around. No flag skips it.
2. `~/.local/state/spec-driven-docs/skills/shared/plan-gate.md` — it binds three phases: plan and present the plan for approval, validate that plan against every preview and read-only source phase 2 names, then execute it.

The two gates are why this skill is safe to run unattended: the sweep rewrites and retires files the project authored, nothing restores them but version control or the plan's own backups, and the pre-flight says whether this host carries the tools the sweep leans on.

When the request carries `--no-plan`, skip the plan gate's approval turn only. Still run the pre-flight, still state the ordered plan before acting, and still validate it as phase 2 directs.

## Detect

Two read-only reports route the work:

- `sdd status --target . --json` — an instance current with the binary and reporting drift takes `sdd verify --target .`, then fixes what it cites, smallest first; an instance older than the binary takes the version path below.
- `sdd assess --target . --json` — with no instance, the classification routes: `brownfield` is this skill's subject, the sweep below; `greenfield` belongs to the sdd-setup skill, because there is nothing to migrate; `needs-decision` is the operator's call, asked with the report's evidence via `AskUserQuestion` before any plan claims to know.

No instance never means no documents. A target that documents itself without sdd is brownfield, and landing seeds over it without the sweep starts a second convention beside the first.

## The full corpus sweep

The chapter owns the procedure; this is the order of work.

1. Inventory every place the project documents itself — the assess report's document inventory, plus README files, wiki exports, and contributor guides it lists. The inventory becomes the migration checklist: one entry per source with its checksum, disposition (retire, merge, split, rewrite), destinations, approval reference, and verification state, exactly as `sdd method 12-migration` shapes it.
2. Classify each entry's facts by the placement chapter's decision procedure — `sdd method 01-placement` serves it. What binds now becomes a spec; why-at-the-time becomes a decision record; task recipes become guides, started from `sdd template guide`; exact values become reference. One source often splits across those fates; list each under its entry.
3. Present the checklist as the plan. Every disposition question the inventory raises is asked before approval, because an approved checklist is what makes the loop safe to run; the checklist lands under version control in the project's plan zone, and provisional rewrites stage under `.draft/migration/` — confirm version control ignores `.draft/` first, and where it does not, the one-line ignore entry is part of the approved plan.
4. Land the instance: choose the profile — `codebase` keeps records under `docs/`, `knowledge-base` under `_docs/` — preview `sdd init --target "$PWD" --profile <profile>`, review the listed paths against the assess report's collisions, then re-run with `--apply`, and wire the documentation section into the project's `AGENTS.md` exactly as the sdd-setup skill states it.
5. Run the loop in verified batches: re-checksum the source (changed means the entry returns to planning), rewrite into the owning zone — never move a file — verify the destinations, retire the source only after they verify, only where the approved plan named the removal, and only while version control or the approved backup holds the checksum-recorded bytes — an untracked or modified source is committed, or backed up where the plan says, first. A discovered document or an unplanned split returns to planning; it is never handled inline.
6. Close on evidence: a fresh inventory finds nothing untracked and no retired path still present, `sdd verify --target .` passes, `pre-commit run --all-files` passes, and `.draft/migration/` is emptied with the operator's approval. The checklist stays, checked and closed, as the record of what went where.

## From a previous version

1. Install the newer `sdd` first, then `sdd upgrade --target . --dry-run` and read the report before anything writes.
2. `sdd upgrade --target .` applies. A locally edited managed file aborts the whole upgrade with every conflict listed in one run; reconcile, then re-run rather than working around the refusal.
3. Re-run `sdd verify --target .`; a failure message cites a `domain:rule` ID, and `sdd spec <domain>` states the sentence to satisfy.

## Defaults

- The full corpus sweep is the default scope; a partial migration is an explicit narrowing, stated in the plan.
- No durable fact is lost: each one is rewritten into its owner, parked in `.draft/migration/`, or retired with the operator's approval.
- Prefer an sdd verb over hand-editing anything the instance manifest owns; everything under `.spec-driven-docs/` belongs to the canon.
- Report every step's outcome from observation, never from memory of what the command should do.
