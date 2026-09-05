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

1. `~/.local/state/spec-driven-docs/skills/shared/pre-flight-gate.md` — run it whatever the request carries. It checks this host with `sdd doctor` and stops the task on what no plan can work around. No flag skips it.
2. `~/.local/state/spec-driven-docs/skills/shared/plan-gate.md` — it binds three phases: plan and present the plan for approval, validate that plan against every preview and read-only source phase 2 names, then execute it.

The two gates are why this skill is safe to run unattended: every verb below writes files into a repository or under the user's home, the pre-flight says whether this host can run those verbs at all, and the plan gate states which steps stay the operator's own.

When the request carries `--no-plan`, skip the plan gate's approval turn only. Still run the pre-flight, still state the ordered plan before acting, and still validate it as phase 2 directs.

## Detect

```bash
sdd status --target . --json
```

`"instance": false` means the repository has no instance; any other result reports the installed profile, version alignment, and drift counts. `sdd status` exits 0 whether or not an instance exists.

With no instance, classify before landing: `sdd assess --target . --json` reads the evidence and answers one of three verdicts. Route by it:

- `greenfield`: land an instance, below.
- `brownfield`: the target already documents itself; load the sdd-migrate skill and follow it — landing seeds over a settled corpus starts a second convention beside the first.
- `needs-decision`: durable prose sits outside any recognized home; present the report's evidence with `AskUserQuestion` and let the operator pick the route.

## Land an instance

1. Choose the profile: `codebase` keeps records under `docs/`; `knowledge-base` keeps them under `_docs/`.
2. Preview: `sdd init --target "$PWD" --profile codebase`. A non-empty target defaults to a dry run and lists every destination.
3. Review the listed paths, then apply: `sdd init --target "$PWD" --profile codebase --apply`.
4. Confirm: `sdd verify --target "$PWD"` prints `OK spec-driven-docs <version>`.
5. Declare the plan zone: in the landed `SPEC-spec-to-code.md`, retarget the verification command of `spec-to-code:a-spec-change-is-typed` at the directory the project's planning tool writes its work records to. Remove the requirement instead when the project keeps no plan zone. This is the only seeded value that is not portable, because the planning tool owns the record and this framework names none.

The install seeds specs and templates the project owns from then on (adopted), lands byte-exact configurations and agent skills the canon owns (managed), and splices one marked block into `.pre-commit-config.yaml`. It touches nothing outside its destinations and the markers.

## Wire agent context

`sdd init` manages this for you. The install writes a marker-delimited documentation block into the root `AGENTS.md`, creating the file when absent, and records it as an integration block. The block routes an agent to the affected specs and names SimpleEnglish `Plain` as the default mode for technical text, loaded from `specs/SPEC-simple-english.md` and the vendored `.spec-driven-docs/upstreams/simpleenglish/skills/simple-english/SKILL.md`. Every byte outside the markers is your own. Do not hand-copy the block: an edit inside the markers is a conflict `sdd upgrade` refuses, and an edit outside them survives.

## Verify

```bash
sdd verify --target .
```

`FAIL` lines mean managed drift or a broken record and exit 1; `DRIFT` lines mean adopted edits awaiting reconciliation and exit 0. A failure message cites a `domain:rule` ID; read the binding sentence with `sdd spec <domain>`.

## Upgrade

```bash
sdd upgrade --target . --dry-run
sdd upgrade --target .
```

Install the newer `sdd` first. A locally edited managed file aborts the whole upgrade with every conflict listed in one run; revert or reconcile, then re-run. Adopted files and content outside the markers survive upgrades.

An adopted seed the canon stops shipping is left in place, because the project owns it from the moment it lands. An upgrade that stops seeding one names it in the release notes; delete the file once nothing local cites its rules. `SPEC-distribution.md` is the first: it states the installer's obligations, which no project can meet or check, and `SPEC-instance.md` now carries what a project owes its own installation.

## Defaults

- One instance per repository, at the repository root.
- Prefer `sdd status --json` for machine decisions; stdout is one JSON object.
- Leave managed files alone: everything under `.spec-driven-docs/` belongs to the canon, and `sdd verify` fails on any edit.
- An instance carries no skill file. The skills live at user scope, where `sdd skill install` puts them; a copy under the repository's `.claude/skills/` or `.agents/skills/` is a leftover from a version before that rule, and `sdd upgrade` removes it.
