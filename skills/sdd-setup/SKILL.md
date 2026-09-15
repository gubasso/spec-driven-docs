---
name: sdd-setup
description: Lands, migrates, and keeps current a spec-driven-docs instance through the sdd CLI. Use when asked to install or set up spec-driven docs in a project that documents nothing, to migrate a project that already documents itself onto the convention, to take a landed instance to a chosen release, or to diagnose drift and sdd verify failures. Triggers include spec-driven-docs, sdd init, sdd stage, sdd status, sdd verify, sdd upgrade, migrate to sdd, adopt spec-driven docs, convert docs conventions, and docs governance instance.
license: CC-BY-4.0
compatibility: Requires the sdd binary on PATH; install with cargo install spec-driven-docs or cargo binstall spec-driven-docs. Requires pre-commit, which runs the delivered gates the landing wires into the project's hook configuration. Every sdd verb runs offline and lands the version of sdd that is installed.
---

# sdd-setup

One route for every arrival and every stay-current task. The installed binary renders its own candidate, the agent compares it against the target and prepares what the project owns, and the operator authorizes the landing. This skill routes: it names the chapter each answer lives in and restates none of it.

## Before acting

Read two files from this skill's own directory before the first action of a task, in this order, and hold both for the whole task. Each path is relative to the directory this `SKILL.md` sits in.

1. `references/pre-flight-gate.md`: run it whatever the request carries. It checks this host with `sdd doctor` and stops the task on what no plan can work around. No flag skips it.
2. `references/plan-gate.md`: it binds three phases: plan and present the plan for approval, validate that plan against every preview and read-only source phase 2 names, then execute it.

The two gates are why this skill is safe to run unattended: every verb below writes files into a repository or under the user's home, and a sweep rewrites and retires files the project authored. The pre-flight says whether this host can run those verbs at all, and the plan gate states which steps stay the operator's own.

When the request carries `--no-plan`, skip the plan gate's approval turn only. Still run the pre-flight, still state the ordered plan before acting, and still validate it as phase 2 directs.

## Route

| Question                                | Where it is answered                |
| --------------------------------------- | ----------------------------------- |
| What is this target, and what would run | the plan, step 2                    |
| Landing a target that documents nothing | `sdd docs migration`                |
| Moving a corpus already written down    | `sdd docs migration`                |
| Upgrading, drift, or a current instance | `sdd docs reconcile`                |
| Comparing a candidate against a target  | `sdd docs stage`                    |
| Where a fact belongs                    | `sdd docs placement`                |
| A failing gate, by its rule ID          | `sdd spec <domain>`                 |
| Anything this table does not answer     | `sdd docs`, then `sdd docs <topic>` |

Read a chapter with `sdd docs <topic>` or its shelf verb. Never copy one here: a chapter restated in a skill is a chapter that drifts at the first edit.

## 1. Observe

The pre-flight gate already ran `sdd status --target . --json`. Hold what it returned, including any invalid-instance diagnostic. It is routing evidence, not a verdict: the planner re-observes the same target, and a record nobody can read is not an absent one.

The report's `paths` section is where every path this skill needs comes from, so nothing below spells one. `paths.user` carries the roots under the user's home with the variable that moved each. `paths.active` is the landed instance, or `null`. `paths.candidates` carries one destination set per profile, and `paths.proposals` what this target offers for the docs scratch. Every entry names its source.

## 2. Acquire the version, then stage it

Acquisition is the operator's. No `sdd` verb resolves, fetches, or installs another release, so what lands is the version on `PATH`, and installing another one is a gated step the operator runs. Before updating from a release that stored plans, use the still-installed old binary to finish or abandon any active work, and record what stays behind as local evidence: the new binary carries no parser for it.

```bash
sdd stage --target . --json
```

The stage writes nothing into the target. It holds every destination the candidate would land under `artifacts/`, this version's own method, specs, templates and skills under `reference/`, and a receipt naming what it rendered. Keep it for the whole task.

## 3. Compare and prepare

Read the target's tree, its record, the Git history of what the candidate would touch, the stage, and only the topics the work needs. Then classify each destination by its provenance: the record and the history agreeing is attribution, and a landing refreshes it; one of the two missing is reduced confidence, presented to the operator; neither is bounded best effort, said plainly and never converted into an automatic overwrite.

Prepare what the project owns now, while nothing is written: its documents, its declaration entries, the ignore lines the scratch needs. A landing refuses before its first write when a destination holds bytes no record accounts for, and this step is what prevents that.

## 4. Land

Present the ordered verbs, the destinations each writes, and the steps that stay the operator's. Then run the landing as a gated step.

```bash
sdd init --target . --profile <profile> --apply
sdd upgrade --target .
```

`sdd init` serves a first landing and `sdd upgrade` an installed instance; each refuses the classification it does not serve and names what does. Both render the candidate afresh, so no staged byte reaches production. A landing that stops partway names the destinations holding candidate bytes; run it again rather than repairing by hand.

## 5. Verify, then clean

Compare the real diff against the stage, destination by destination. Then run the project's checks and `sdd verify --target .`, which prints `OK spec-driven-docs <version>`. Report every step from observation. The stage is still there, which is the point: a failure is diagnosed against it rather than against memory. Removing it is the last gated step, and only with the operator's explicit authority:

```bash
sdd stage clean <path>
```

Inactive state an older release left is shown with the evidence that no operation is still active, and removed only by exact path under that same authority.

## A setup or migration classification

Load `sdd docs migration`. It owns the classification, the inventory, the loop, and the close, and every step below is one of its sections.

With a sweep, author the migration checklist into the location the operator names, in the shape the chapter's inventory section gives, and drive the chapter's loop. A read-only sweep of every populated documentation root completes the inventory before the checklist freezes, covering files of any extension, wiki exports, README files, and contributor guides.

With an incremental scope, land the instance, record the inherited budget violations with `sdd debt baseline --apply`, and close the task naming the two standing rules: recorded debt tightens as a document shrinks, and prose converts the next time somebody edits it. Author no checklist, because nothing retires.

A numbered document, `01-intro.md` or `ADR-0007-thing.md`, is renamed to a slug drawn from its subject inside the migration, never as a follow-up. The rename repairs every inbound link in the same change. Where the corpus has a reading order, that order moves into the directory's `README.md` as prose before the numbers come off.

## An upgrade, drift, or current classification

Load `sdd docs reconcile`. It owns the three reconciliations, and none of them is a single verb. Managed drift blocks the plan and carries the three-way comparison to present. Adopted drift is kept and its baseline moves, which is the ownership working. A retired rule ID is re-pointed by the operator, and the citation gate fails until it is.

Install the newer `sdd` before landing a newer release, because the binary lands only itself. An instance ahead of this binary stops the task: no verb here is safe from an older engine.

## An invalid classification

A record or declaration nobody can read is a stop. `sdd status --target . --json` carries the evidence, nothing is written, and the operator resolves it. Never rewrite an unreadable record as an absent one, and never land seeds over it.

## Wire agent context

`sdd init` manages this for you, and so does an upgrade. The landing writes a marker-delimited documentation block into the root agent digest `paths.active.destinations.agents_digest` names and records it as an integration block, creating the file when absent. The block routes an agent to the affected specs and to the selected writing source, `sdd method writing-style` by default, before the agent authors or edits prose. Every byte outside the markers is your own. Do not hand-copy the block: an edit inside the markers is a conflict the next plan refuses, and an edit outside them survives.

## Declare what the gates judge

The declaration `paths.active.destinations.declaration` names states which paths no delivered gate judges and which filters a named gate takes. The landing writes it once and never again, so it is the project's from the moment it lands. `reserved:` lists paths no gate judges, for a region another tool renders or hashes. `gates:` names a gate and gives it `include` or `exclude` globs, and a gate absent there takes its registry default.

Four layers compose, highest last: the registry default, the `gates:` entry, a `--include` or `--exclude` flag, then `reserved:`. Every `exclude` layer extends. A `gates:` `include` replaces the registry default rather than adding to it, because extending a whitelist could only widen what the gate judges.

Edit the file, then run `sdd hooks --apply`, which renders the managed block from the declaration. `sdd verify` fails and names the gate whose wiring disagrees. `sdd gate --explain <PATH>` names the pattern and layer behind each answer, and it is the first thing to run after a surprising result. Ask the operator whether another tool owns a region of any file before the landing, because that is the case reservations exist for.

## Land the variables

The variable that `paths.active.docs_scratch` names overrides the recorded value. It is not required, and the binary does not write it. Land it as a gated step the operator approves first.

1. Observe, read-only: an `.envrc` at the root, `direnv` on `PATH`, an existing `.env`, and what `.gitignore` already covers.
2. With direnv present, append the `export` line to `.envrc.local`, and add `.envrc.local` to `.gitignore` where it is absent.
3. Without direnv, write the assignment to `.env`, and add `.env` to `.gitignore` where it is absent. State the caveat: nothing loads a plain `.env` on its own, so the operator sources it before a gate can read it.
4. With neither, print the line and stop.

## Offer the freshness wire

Where the target pins `sdd` through a manager, the task does not close until the operator has answered about the freshness wire. Run this after the variables land and before the result is read.

1. Observe, read-only: `sdd self-depend status --target . --json`. Hold `wired` and `envrc_sync`. Where `wired` is `null`, there is no pin to keep fresh, and where `envrc_sync` is `true`, the wire is landed: in either case say nothing and continue.
2. Where `wired` names a manager and `envrc_sync` is `false`, ask with `AskUserQuestion` whether the project lands the line `sdd self-depend sync --apply || true` in its `.envrc`. State what the line does: on each directory entry it moves the pin to the latest release through that manager, it attempts at most one bump a day, and it leaves a diff for the operator to review and commit.
3. On yes, landing the line is a gated step: print `sdd self-depend add --target . --manager <wired>`, say what it changes, wait, then re-observe with the status verb until `envrc_sync` reads `true`.
4. On no, the landing is complete without the wire. Record the answer in the close of the task, so the question reads as asked and answered rather than skipped.

## What waits for the operator

Gate each of these: print the exact command or edit, say what it changes and why, wait, then re-observe before continuing.

- The ignore entry for the docs scratch, where it sits inside the repository and the ignore file lacks it.
- The landing itself, `sdd init --apply` or `sdd upgrade`, named with the destinations it writes, and `sdd stage clean <path>` last of all, only after every check passed.
- Committing or backing up an untracked or modified source before its entry retires anything.
- Every retirement of a file the project authored: only where the approved checklist named it, only after its destinations verified.
- Every disposition question the inventory raises, and every decision the plan declares.
- Emptying the docs scratch's migration directory at the close: anything still in it is promoted or consciously dropped, never swept.

## When it goes wrong

- A landing that refuses because a destination holds bytes no record accounts for is working as designed. Read what it names, compare it against the stage, and let the operator decide. Never force it.
- A gate failure citing a `domain:rule` ID routes to the sdd-write-docs skill's gate triage. Read the rule with `sdd spec <domain>`, fix the document, re-run. Never widen a budget or remove a gate to pass.
- A landing that stopped partway is finished by running it again, and the stage says what the result should be.
- A discovery is a document the inventory missed, a split nobody named, or a deletion nobody approved. It returns to planning and is never handled inline. An interrupted sweep resumes from the checklist: reload it, re-checksum from the first unchecked entry, and continue.

## Defaults

- One instance per repository, at the repository root.
- Every write comes from a plan the operator approved, and the landing renders that plan's candidate afresh.
- Prefer `sdd status --json` and `sdd stage --json` for machine decisions. Each prints one JSON object.
- Leave managed files alone: everything under the instance directory `paths.active.destinations.instance_dir` names belongs to the canon.
- An instance carries no skill file. The skills live at user scope, in the roots `paths.user.agent_roots` names, where `sdd skill install` puts them.
- Never widen an approved scope inline: a discovered document, an extra split, a version move is its own plan, approved at its own size.
