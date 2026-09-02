# AGENTS

This repository is the canonical knowledge product for spec-driven documentation. `README.md` routes readers; this file routes agents to the rules that bind their work.

## Before acting

- Load each `_docs/specs/SPEC-<domain>.md` affected by the work.
- Apply stated rules and cite their `<domain>:<rule>` IDs in reports and failures.
- Do not load `_docs/decisions/` unless someone asks why a rule exists.
- Update the owning spec in the same change as behavior.
- Update a skill in the same change as the behavior it describes.

## Ownership boundaries

- `method/`, `comparison-docs/`, `templates/`, `reference/`, and `skills/` are canon product files.
- `src/` is the distribution: the `sdd` binary embeds the payload — spec seeds, templates, `.markdownlint/` configurations, `instance/snippets/`, `skills/`, and `method/` — at compile time from these authored paths, so canon and binary cannot drift.
- `skill-shared/` is what every skill shares, installed once to `~/.local/state/spec-driven-docs/skills/shared/` and named there by absolute path: the two agent roots make no relative path reach one file from both. The plan gate every skill routes to lives there.
- The delivered gate set is declared once, in the registry in `src/gates.rs`; the managed block an instance receives is rendered from it at install time and committed nowhere, so there is no copy to hold equal. This repository publishes no `.pre-commit-hooks.yaml`: the gates serve instances, not repositories that reference them remotely.
- This repository is an instance of itself, and the one whose block no installer wrote: the managed region of its own `.pre-commit-config.yaml` is maintained by hand, so a new gate is wired there in the same change, and the release checks hold that region to the registry.
- Checks of invariants only this repository has — the license split, version alignment — are cargo tests under `tests/`, never delivered (ADR-split-gates-by-delivery-domain).
- `_docs/specs/`, `_docs/decisions/`, marker-delimited integrations, and instance debt are local overlays after installation.
- Keep each durable fact in one owner and link to it elsewhere.
- `LICENSE` splits terms on the same boundary: CC BY 4.0 for the method, MIT for the payload.

## Authoring

- Keep the root digest at or below 100 lines and subtree digests at or below 150 lines.
- Keep chapters at or below 200 lines, catalogs and specs at or below 300 lines, and decision records at or below 350 words.
- Use headings, lists, tables, fenced blocks with a language, inline code, and links. Use no bold or italic text.
- Keep prose unwrapped: one source line per paragraph or list item.
- Keep `README.md` and every `AGENTS.md` as indexes or digests, never rule dumps or filesystem inventories.
- Write step-by-step guides to `_docs/specs/SPEC-guides.md`: every step carries its check, a manual step enumerates every field and value, and upstream facts carry dated citations in the reference zone.
- State what is true now. Decision records are the only history-bearing document class.
- Keep exploratory material in `.draft/`; promotion is a rewrite into the owning zone.

## Executable artifacts

- Rust follows the exobrain CLI conventions: clap derive in `src/cli/`, one handler per subcommand in `src/commands/`, typed errors with a tested exit-code matrix in `src/error.rs`.
- Every gate change lands with its unit tests, and every failure message a gate prints cites a rule ID that a spec defines; the registry test holds the citable set to the specs.
- Run `just check` before handoff. It lints, tests, and installs into a scratch target.
- Run `just manifest` after editing anything the canon manifest records: `.markdownlint/`, `_docs/specs/`, the recorded templates, or the managed pre-commit block. `sdd verify` reports the omission as a note rather than a failure, because an instance owns its adopted files; here `release:the-canon-record-describes-its-tree` fails instead.
- `Cargo.toml` is the release source of truth. Write Conventional Commits; release-plz derives the version, the changelog, and the tag. Never author a tag: `_docs/guides/release.md` owns the sequence.
- Manage dependencies through cargo (`cargo add`, `cargo remove`, `cargo update`); never hand-edit versions in `Cargo.toml`.

## Routing

- Method-specific routing: `method/AGENTS.md`.
- Comparison-document routing: `comparison-docs/AGENTS.md`.
- Distribution and ownership: `_docs/specs/SPEC-distribution.md`.
- Format and budgets: `_docs/specs/SPEC-docs-format.md`.
- Decision records: `_docs/specs/SPEC-decision-records.md`.
- Cutting a release: `_docs/guides/release.md`.
- Step-by-step guides: `_docs/specs/SPEC-guides.md`.

<!-- BEGIN release-kit -->

## Releases

- This repository runs the release-kit convention; `rk method invariants` states what must stay true.
- An agent here guides and never drives: it reads this convention, tells the operator which step comes next, and takes no git or forge action — creating, switching or deleting a branch, creating or removing a worktree, committing, pushing, tagging, opening or updating or merging a pull request — unless the operator's request named that action. A request to change code authorizes the file changes alone.
- Work reaches the trunk only through a squash-merged pull request from a short-lived branch — `<type>/<slug>` mirroring the squash title's type, or the forge-minted `<issue-id>-<slug>`. Nothing is committed on `master`.
- This project works in worktrees: every code-changing branch lives in its linked worktree (`rk worktree add <branch>` creates or adopts it beside the checkout), the main checkout commits nothing, and `rk worktree prune` retires a merged worktree. One branch, one writer.
- The request's title becomes the trunk's commit message, so it MUST be a scoped Conventional Commit; the body carries the context and lands with it: it names no internal planning artifact and carries no agent attribution — the landed rk-message hook, the forge's body check, and the observed body source hold it.
- Every commit follows the same scoped convention; the landed commit-msg hook enforces it, and the scopes this project accepts are `release,gates,skills,skill-shared,distribution,specs,docs,guides,method,comparison-docs,templates,reference,instance,deps,ci,lint`.
- Never author a tag, and never hand-edit a generated artifact workflow.
- Run `rk status` before changing anything under `.github/workflows/` or `.gitlab-ci.yml`, or any file `.release-kit/manifest.json` names.
- The full method is `rk method --list`; the recovery paths are `rk method recovery`.

<!-- END release-kit -->
