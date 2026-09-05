---
name: sdd-migrate
description: Drives a repository from however it documents itself today to the current spec-driven-docs convention, with a full corpus sweep as the default. Use when asked to migrate a project onto spec-driven docs, convert an existing docs or specs convention, adopt sdd in a repository that already documents itself, upgrade an older instance, or reconcile a drifted one. Triggers include migrate to sdd, adopt spec-driven docs, convert docs conventions, docs migration, and sdd upgrade.
license: CC-BY-4.0
compatibility: Requires the sdd binary on PATH; install with cargo install spec-driven-docs or cargo binstall spec-driven-docs. Requires pre-commit for the closing gate run. Every sdd verb runs offline.
---

# sdd-migrate

Take a repository from however it documents itself today to the current spec-driven-docs convention. The default is the full corpus sweep: every durable fact the project has written down ends the migration in exactly one sdd home, and the old convention is retired. Narrow the scope only when the request narrows it. The procedure this skill drives is `sdd method 12-migration`; read it once per task and hold it, because every step below is one of its sections and the chapter owns the how.

## Before acting

Read two shared files before the first action of a task, in this order, and hold both for the whole task.

1. `~/.local/state/spec-driven-docs/skills/shared/pre-flight-gate.md` — run it whatever the request carries. It checks this host with `sdd doctor` and stops the task on what no plan can work around. No flag skips it.
2. `~/.local/state/spec-driven-docs/skills/shared/plan-gate.md` — it binds three phases: plan and present the plan for approval, validate that plan against every preview and read-only source phase 2 names, then execute it.

The two gates are why this skill is safe to run unattended: the sweep rewrites and retires files the project authored, nothing restores them but version control or the plan's own backups, and the pre-flight says whether this host carries the tools the sweep leans on.

When the request carries `--no-plan`, skip the plan gate's approval turn only. Still run the pre-flight, still state the ordered plan before acting, and still validate it as phase 2 directs.

## Detect

The pre-flight's last two steps already ran `sdd status --target . --json` and, at a target with no instance, `sdd assess --target . --json`; this skill routes on what they returned, and the chapter's first section defines the verdicts.

- `brownfield` with no instance is this skill's subject: the sweep, start to end.
- `greenfield` belongs to the sdd-setup skill, because there is nothing to migrate; hand off and stop.
- `needs-decision` is the operator's call, asked with `AskUserQuestion` and the report's evidence — the documents it found outside every recognized home — before any plan claims to know what they are.
- An instance routes by its status report, whatever the verdict says, and `alignment` decides first: `instance-newer` stops until a matching `sdd` is installed, because no verb below is safe from an older binary; `binary-newer` takes the sdd-setup skill's `## Upgrade`, which owns the dry run, the apply, and the atomic conflict refusal.
- An `aligned` instance routes by the report's two counts, in this order, and the branches do not overlap: `failures` above zero — managed drift, a missing adopted file, a broken integration block, a duplicate rule ID — takes the sdd-setup skill's `## Verify`, fixing what `sdd verify --target .` cites, smallest first, with managed files reverted or reconciled as `## Upgrade` states; then `adopted_drift` above zero is the project's own reconciliation, `DRIFT` lines this skill never reverts; only both at zero is reported and stops, because there is nothing to migrate.

No instance never means no documents. A target that documents itself without sdd is brownfield, and landing seeds over it without the sweep starts a second convention beside the first.

## The full corpus sweep

The chapter owns each step's how; this is what the skill decides at each one.

1. Inventory every place the project documents itself. The assess report's document inventory is the seed, not the boundary: it lists the extensions it recognizes, so a read-only sweep of every populated documentation root and every other source the report names — files of any extension, wiki exports, README files, contributor guides — completes the inventory before the checklist freezes. The inventory becomes the migration checklist, one entry per source in the shape the chapter's inventory section gives — checksum, disposition, destinations, approval, verification state.
2. Classify each entry's facts by the placement chapter's decision procedure, `sdd method 01-placement`. One source often splits across several fates; list each fate under its entry rather than averaging them.
3. Present the checklist as the plan. Ask every disposition question the inventory raises before approval, with `AskUserQuestion`, because an approved checklist is what makes the loop safe to run. The checklist lands under version control in the project's plan zone; provisional rewrites stage under `.draft/migration/`, and the ignore entry that keeps them out of version control is a gated step below.
4. Land the instance exactly as the sdd-setup skill's `## Land an instance` and `## Wire agent context` state it — the profile, the preview against the assess report's collisions, the apply, the documentation section in `AGENTS.md`. Nothing of that sequence is restated here.
5. Run the chapter's loop in verified batches: re-checksum, rewrite into the owning zone, verify the destinations, retire the source only under the chapter's four conditions, record completion last. Each destination is authored as the sdd-write-docs skill's `## Before writing` states — the adopted spec for its domain loaded, the matching template taken only where one exists; a plain reference page has none and follows its zone's format and placement rules.
6. Close on the chapter's evidence: the fresh inventory against the checklist, `sdd verify --target .`, `pre-commit run --all-files`, and the workshop emptied with the operator's approval. The checklist stays, checked and closed, as the record of what went where.

## What waits for the operator

Gate each of these: print the exact command or edit, say what it changes and why, wait, then re-observe before continuing.

- The `.draft/` ignore entry, where the target's ignore file lacks it — one line, before anything stages in the workshop.
- `sdd init --target "$PWD" --profile <profile> --apply` — the one landing write; its preview runs first, per the sdd-setup skill.
- Committing, or backing up where the plan says, an untracked or locally modified source before its entry retires anything: the chapter retires only bytes version control or the approved backup holds.
- Every retirement of a file the project authored — only where the approved checklist named it, only after its destinations verified.
- A `needs-decision` verdict, and every disposition question the inventory raises.
- Emptying `.draft/migration/` at the close: anything still in it is promoted or consciously dropped by the operator, never swept.

## When it goes wrong

- A gate failure citing a `domain:rule` ID routes to the sdd-write-docs skill's `## Gate triage`: read the rule with `sdd spec <domain>`, fix the document, re-run; never widen a budget or remove a gate to pass.
- A source whose checksum changed mid-sweep returns its entry to planning; nothing is written from a stale classification.
- A discovery — a document the inventory missed, a split the plan did not name, a deletion nobody approved — returns to planning and is never handled inline.
- An interrupted sweep resumes from the checklist: reload it, re-checksum from the first unchecked entry, and continue. Re-running a completed entry proves equivalence and reports it; it does not rewrite.
- An instance found older than the binary mid-sweep is its own entry through the sdd-setup skill's `## Upgrade`, not a step folded into another entry.

## Defaults

- The full corpus sweep is the default scope; a partial migration is an explicit narrowing, stated in the plan.
- No durable fact is lost: each one is rewritten into its owner, parked in `.draft/migration/`, or retired with the operator's approval.
- Prefer an sdd verb over hand-editing anything the instance manifest owns; everything under `.spec-driven-docs/` belongs to the canon.
- Report every step's outcome from observation, never from memory of what the command should do.
- Never widen an approved scope inline: a discovered document, an extra split, a version upgrade is its own entry, approved at its own size.
