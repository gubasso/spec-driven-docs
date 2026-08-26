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
- The delivered gate set is declared once, in the registry in `src/gates.rs`; the managed block an instance receives and the published `.pre-commit-hooks.yaml` are both rendered from it by `sdd hooks`, and a cargo test holds the published file equal to the render.
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
- State what is true now. Decision records are the only history-bearing document class.
- Keep exploratory material in `.draft/`; promotion is a rewrite into the owning zone.

## Executable artifacts

- Rust follows the exobrain CLI conventions: clap derive in `src/cli/`, one handler per subcommand in `src/commands/`, typed errors with a tested exit-code matrix in `src/error.rs`.
- Every gate change lands with its unit tests, and every failure message a gate prints cites a rule ID that a spec defines; the registry test holds the citable set to the specs.
- Run `just check` before handoff. It lints, tests, and installs into a scratch target.
- Run `just manifest` after editing anything the canon manifest records: `.markdownlint/`, `_docs/specs/`, the recorded templates, or the managed pre-commit block.
- `Cargo.toml` is the release source of truth. Write Conventional Commits; release-plz derives the version, the changelog, and the tag. Never author a tag: `_docs/guides/release.md` owns the sequence.
- Manage dependencies through cargo (`cargo add`, `cargo remove`, `cargo update`); never hand-edit versions in `Cargo.toml`.

## Routing

- Method-specific routing: `method/AGENTS.md`.
- Comparison-document routing: `comparison-docs/AGENTS.md`.
- Distribution and ownership: `_docs/specs/SPEC-distribution.md`.
- Format and budgets: `_docs/specs/SPEC-docs-format.md`.
- Decision records: `_docs/specs/SPEC-decision-records.md`.
- Cutting a release: `_docs/guides/release.md`.
