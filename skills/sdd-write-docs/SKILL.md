---
name: sdd-write-docs
description: Authors documentation governed by spec-driven-docs, covering current specs, immutable decision records, agent digests, comparison documents, known-issue records, and step-by-step guides. Use when writing or editing docs in a repository that carries a .spec-driven-docs/ directory, when a pre-commit documentation gate fails citing a domain:rule ID, or when a task mentions SPEC files, ADRs, EARS requirements, sdd method, sdd spec, or sdd template.
license: CC-BY-4.0
compatibility: Requires the sdd binary on PATH; install with cargo install spec-driven-docs or cargo binstall spec-driven-docs.
---

# sdd-write-docs

Author documentation by the spec-driven-docs method. The binding rules live in the instance's adopted specs, seeded from the binary's canon baseline; read the adopted spec first, and the CLI where a domain is not adopted, because the adopted text is what the instance owns.

## Before acting

Read two shared files before the first action of a task, in this order, and hold both for the whole task.

1. `~/.local/state/spec-driven-docs/skills/shared/pre-flight-gate.md` — run it whatever the request carries. It checks this host with `sdd doctor` and stops the task on what no plan can work around. No flag skips it.
2. `~/.local/state/spec-driven-docs/skills/shared/plan-gate.md` — it binds three phases: plan and present the plan for approval, validate that plan against every preview and read-only source phase 2 names, then execute it.

The two gates are why this skill is safe to run unattended: every change below rewrites a documentation zone the instance governs, and the pre-flight says whether the host and the installed skills are ones this binary answers for.

When the request carries `--no-plan`, skip the plan gate's approval turn only. Still run the pre-flight, still state the ordered plan before acting, and still validate it as phase 2 directs.

## Route to the rules

| Need                                | Command                                                    |
| ----------------------------------- | ---------------------------------------------------------- |
| List method chapters                | `sdd method --list`                                        |
| Read a chapter                      | `sdd method <chapter>`                                     |
| List rule domains                   | `sdd spec --list`                                          |
| Read the binding rules for a domain | adopted `specs/SPEC-<domain>.md`; else `sdd spec <domain>` |
| List templates                      | `sdd template --list`                                      |
| Start a new document                | `sdd template <name>`                                      |
| List the known-issue cases          | `sdd ki list`                                              |

## Before writing

1. Run `sdd status --target . --json`; confirm the repository is an instance and note `docs_root`.
2. Load the adopted spec for each domain the change touches, from `specs/` under `docs_root`: `SPEC-docs-format.md` always, `SPEC-docs-specs.md` for SPEC files, `SPEC-decision-records.md` for ADRs, `SPEC-comparison-docs.md` for comparisons, `SPEC-known-issues.md` for known-issue records, `SPEC-guides.md` for step-by-step guides. `sdd spec <domain>` prints the canon baseline where a domain is not adopted.
3. Start from the matching template: `sdd template spec`, `sdd template adr`, `sdd template comparison`, `sdd template agents-digest`, `sdd template guide`, or `sdd template known-issue`.

## Format defaults

- One source line per paragraph or list item; no hard-wrapped prose.
- Headings, lists, tables, fenced code with a language, inline code, and links; no bold or italic text.
- State the present; decision records are the only history-bearing document class.
- A new requirement carries five parts: a title, a `domain:rule` ID, one EARS statement, a GIVEN/WHEN/THEN scenario, and a `Verify:` line naming a live command.

## Gate triage

A failing gate or `sdd verify` message cites a `domain:rule` ID. Read the rule in the adopted `specs/SPEC-<domain>.md` under `docs_root`, or with `sdd spec <domain>` where the instance has not adopted that domain, fix the document to satisfy the cited sentence, and re-run the failing command. A budget is never widened and a gate is never removed to admit a document.

## Handoff

```bash
sdd verify --target .
pre-commit run --all-files
```

Both pass before the change is done.
