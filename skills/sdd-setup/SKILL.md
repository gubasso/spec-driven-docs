---
name: sdd-setup
description: Lands and operates a spec-driven-docs instance in a project through the sdd CLI. Use when asked to install or set up spec-driven docs, detect whether a repository has an instance, verify or upgrade an installed instance, splice the documentation section into AGENTS.md, or diagnose sdd verify failures. Triggers include spec-driven-docs, sdd init, sdd status, sdd verify, sdd upgrade, and docs governance instance.
license: CC-BY-4.0
compatibility: Requires the sdd binary on PATH; install with cargo install spec-driven-docs or cargo binstall spec-driven-docs. Requires pre-commit, which runs the delivered gates the install wires into .pre-commit-config.yaml.
---

# sdd-setup

Operate a project's spec-driven-docs instance through the `sdd` CLI. The CLI is the whole interface: every binding rule is readable with `sdd spec <name>` and every method chapter with `sdd method <chapter>`.

## Before acting

Read two shared files before the first action of a task, in this order, and hold both for the whole task.

1. `~/.local/state/spec-driven-docs/skills/shared/pre-flight-gate.md`: run it whatever the request carries. It checks this host with `sdd doctor` and stops the task on what no plan can work around. No flag skips it.
2. `~/.local/state/spec-driven-docs/skills/shared/plan-gate.md`: it binds three phases: plan and present the plan for approval, validate that plan against every preview and read-only source phase 2 names, then execute it.

The two gates are why this skill is safe to run unattended: every verb below writes files into a repository or under the user's home. The pre-flight says whether this host can run those verbs at all, and the plan gate states which steps stay the operator's own.

When the request carries `--no-plan`, skip the plan gate's approval turn only. Still run the pre-flight, still state the ordered plan before acting, and still validate it as phase 2 directs.

## Detect

```bash
sdd status --target . --json
```

`"instance": false` means the repository has no instance. Any other result reports the installed profile, version alignment, and drift counts. `sdd status` exits 0 whether or not an instance exists.

With no instance, classify before landing: `sdd assess --target . --json` reads the evidence and answers one of three verdicts. Route by it:

- `greenfield`: land an instance, below.
- `brownfield`: the target already documents itself. Load the sdd-migrate skill and follow it, because landing seeds over a settled corpus starts a second convention beside the first.
- `needs-decision`: durable prose sits outside any recognized home. Present the report's evidence with `AskUserQuestion` and let the operator pick the route.

## Land an instance

Ask the three declaration questions below first, in the same plan turn as the profile. Their answers are flags on the one apply, so the sequence writes once.

1. Choose the profile: `codebase` keeps records under `docs/`, and `knowledge-base` keeps them under `_docs/`.
2. Preview: `sdd init --target "$PWD" --profile codebase`. A non-empty target defaults to a dry run and lists every destination.
3. Review the listed paths, then apply, carrying whichever declarations the operator gave: `sdd init --target "$PWD" --profile codebase --apply --plan-zone docs/plan --docs-scratch .docs-scratch --writing-style builtin`.
4. Confirm: `sdd verify --target "$PWD"` prints `OK spec-driven-docs <version>`.

The install seeds specs and templates the project owns from then on (adopted). It lands byte-exact configurations and agent skills the canon owns (managed), and splices one marked block into `.pre-commit-config.yaml`. It touches nothing outside its destinations and the markers.

## Declare what the gates judge

`.spec-driven-docs/config.yaml` states which paths no delivered gate judges and which filters a named gate takes. `sdd init` writes it once and never again, so it is the project's from the moment it lands.

- `reserved:` is a list of paths no gate judges. Use it for a region another tool renders or hashes.
- `gates:` names a gate and gives it `include` or `exclude` globs. A gate not named there takes its registry default.

Four layers compose, highest last: the registry default, the `gates:` entry, a `--include` or `--exclude` flag, then `reserved:`. Every `exclude` layer extends. A `gates:` `include` replaces the registry default rather than adding to it, because an include is a whitelist and extending one could only widen what the gate judges.

Edit the file, then run `sdd hooks --apply`. The managed block is rendered from the declaration, so nothing else reaches it and `sdd upgrade` returns early at the same version. `sdd verify` fails and names the gate whose wiring disagrees.

`sdd init --reserve <PATH>` records a reservation at install time. Pass it once per path. Ask the operator whether another tool owns a region of any file in the repository before running the install, because that is the case the mechanism exists for.

`sdd gate --explain <PATH>` answers which gates judge a path and names the pattern and layer that decided each answer. It is the first thing to run after a surprising result. It reports path eligibility only: pre-commit also applies each row's `types:`, which the command prints rather than folds in.

## Carry an inherited corpus

A budget gate fails a document over its cap. A project adopting the convention over documents written before it records each inherited violation in `.spec-driven-docs/debt.yaml`, per gate, per path, per dimension, under `sdd spec budget-debt`. A recorded ceiling is judged instead of the cap and only comes down. Nothing delivers the file, and a project inheriting a corpus records debt rather than raising a budget.

- `sdd debt baseline` previews every current violation as debt, and `--apply` writes it. It refuses where a debt file exists, because a baseline never widens one.
- `sdd debt migrate` converts the older flat list at `.spec-driven-docs/chapter-size-debt.txt` and removes it under `--apply`. It preserves every exemption and broadens nothing.
- `sdd debt tighten` lowers each ceiling to its measurement and clears each corrected exception under `--apply`. It never raises a ceiling.

A brownfield landing runs the install, then `sdd debt baseline`, then `sdd debt baseline --apply`, then its first commit. A gate failure naming `sdd debt tighten --apply` means a carried document shrank: run it and commit the file with the change. Where `sdd verify` notes that no local specification defines `budget-debt:a-recorded-dimension-only-shrinks`, the project's specs predate the mechanism: read the rule with `sdd spec budget-debt` and add it to the project's copy.

## Declare the two locations

Two locations belong to the project rather than to this framework. Ask for each with `AskUserQuestion`, in the plan turn, and pass the answer to `sdd init`. Mark no answer as recommended: each one has its own cost. These two questions are the only place that names a candidate path. Everywhere else the corpus names the variable.

The plan zone is where the planning tool writes entry documents. State that the gate reads it and that the project can keep none.

- a repository-relative path, such as `docs/plan`: under version control, so every clone carries it and the gate checks it.
- `untracked:<PATH>`: inside the repository but not committed, so the gate reports nothing and a reviewer holds the rule.
- `env`: the records live wherever `SDD_PLAN_ZONE` points, which suits a planning tool that owns a path under the user's home.
- `none`: the project keeps no plan zone, and the rule binds nothing.

The docs scratch holds material that is not a statement yet, and it stages a migration. State that it must stay out of version control.

- `.docs-scratch/` at the repository root, with the matching ignore entry.
- a directory beside the checkout, such as `../<project>.docs-scratch/`, which the repository never sees.
- a path the operator types.

Both flags are optional, and an omitted flag keeps what is recorded: a later `sdd init` or `sdd upgrade` that carries no flag changes neither value. `--plan-zone none` and `--docs-scratch none` are how a recorded value is cleared. `sdd status --json` reports both, plus whatever the two variables carry here.

## Select the writing source

The writing style is the project's to select, under `sdd spec writing-policy`. Ask with `AskUserQuestion`, in the plan turn, and pass the answer as `--writing-style`. Mark no answer as recommended: each one has its own cost.

- `builtin`: this convention's chapter, served offline by `sdd method writing-style`, with nothing to maintain.
- `project:<PATH>`: the project's own document, which the project keeps.
- `none`: no route and no conversion obligation, so an agent editing the project's documents follows whatever its own instructions say.

The selection lives in `.spec-driven-docs/config.yaml` under `writing_style`. To change it later, edit the file and run `sdd hooks --apply`, which rewrites the documentation block in `AGENTS.md` as well as the pre-commit block. Where `sdd verify` notes that no local specification defines `writing-policy:the-project-selects-one-source`, the project's specs predate the selection: read the rule with `sdd spec writing-policy` and add it to the project's copy.

## Land the variables

`SDD_PLAN_ZONE` and `SDD_DOCS_SCRATCH` override the recorded values. Neither is required, and the binary writes neither. Land them as a gated step the operator approves first.

1. Observe, read-only: an `.envrc` at the root, `direnv` on `PATH`, an existing `.env`, and what `.gitignore` already covers.
2. With direnv present, append the two `export` lines to `.envrc.local`, and add `.envrc.local` to `.gitignore` where it is absent.
3. Without direnv, write the two assignments to `.env`, and add `.env` to `.gitignore` where it is absent. State the caveat: nothing loads a plain `.env` on its own, so the operator must source it before a gate can read it.
4. With neither, print the two lines and stop.
5. Close the task with the followup: neither variable is required. A recorded `tracked` zone is checked whether or not `SDD_PLAN_ZONE` is set. The other three kinds are checked only where it is set.

## Wire agent context

`sdd init` manages this for you. The install writes a marker-delimited documentation block into the root `AGENTS.md` and records it as an integration block. The install creates the file when absent. The block routes an agent to the affected specs and to the selected writing source, `sdd method writing-style` by default, before the agent authors or edits prose. Every byte outside the markers is your own. Do not hand-copy the block: an edit inside the markers is a conflict `sdd upgrade` refuses, and an edit outside them survives.

## Verify

```bash
sdd verify --target .
```

`FAIL` lines mean managed drift or a broken record and exit 1. `DRIFT` lines mean adopted edits awaiting reconciliation and exit 0. A failure message cites a `domain:rule` ID. Read the binding sentence with `sdd spec <domain>`.

## Upgrade

```bash
sdd upgrade --target . --dry-run
sdd upgrade --target .
```

Install the newer `sdd` first. A locally edited managed file aborts the whole upgrade with every conflict listed in one run. Revert or reconcile, then re-run. Adopted files and content outside the markers survive upgrades.

An instance installed before 0.4.14 records manifest schema 2, and this binary reads 3. Every verb but `sdd upgrade` refuses that record and names the upgrade. Run the upgrade once. It migrates the record and keeps both declared locations, and it also migrates an instance whose canon version already matches this binary.

An adopted seed the canon stops shipping is left in place, because the project owns it from the moment it lands. An upgrade that stops seeding one names it in the release notes. Delete the file once nothing local cites its rules. `SPEC-distribution.md` is the first: it states the installer's obligations, which no project can meet or check, and `SPEC-instance.md` now carries what a project owes its own installation.

`sdd upgrade` prints a `removed managed file no longer owned:` line for each file the new payload no longer declares. It removes the immediate parent of a removed file when that removal empties it, and it walks no further up. After an upgrade that prints those lines, remove every directory under `.spec-driven-docs/` the removals left empty, deepest first. `find .spec-driven-docs -type d -empty -delete` does it, and it removes nothing that still holds a file.

## Defaults

- One instance per repository, at the repository root.
- Prefer `sdd status --json` for machine decisions. Its stdout is one JSON object.
- Leave managed files alone: everything under `.spec-driven-docs/` belongs to the canon, and `sdd verify` fails on any edit.
- An instance carries no skill file. The skills live at user scope, where `sdd skill install` puts them. A copy under the repository's `.claude/skills/` or `.agents/skills/` is a leftover from a version before that rule, and `sdd upgrade` removes it.
