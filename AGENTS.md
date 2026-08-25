# AGENTS

This repository is the canonical knowledge product for spec-driven documentation. `README.md`
routes readers; this file routes agents to the rules that bind their work.

## Before acting

- Load each `_docs/specs/SPEC-<domain>.md` affected by the work.
- Apply stated rules and cite their `<domain>:<rule>` IDs in reports and failures.
- Do not load `_docs/decisions/` unless someone asks why a rule exists.
- Update the owning spec in the same change as behavior.

## Ownership boundaries

- `method/`, `comparison-docs/`, `templates/`, and `reference/` are canon product files.
- `gates/instance/`, `.markdownlint/`, scripts, and profiles are managed instance payload;
  `instance/gates.json` declares that delivered set and both deliveries render from it.
- `gates/canon/` never leaves this repository: it holds the gates for invariants only the canon
  has (ADR-split-gates-by-delivery-domain).
- `_docs/specs/`, `_docs/decisions/`, marker-delimited integrations, and instance debt are local
  overlays after installation.
- Keep each durable fact in one owner and link to it elsewhere.
- `LICENSE` splits terms on the same boundary: CC BY 4.0 for the method, MIT for the payload.

## Authoring

- Keep the root digest at or below 100 lines and subtree digests at or below 150 lines.
- Keep chapters at or below 200 lines, catalogs and specs at or below 300 lines, and decision records
  at or below 350 words.
- Use headings, lists, tables, fenced blocks with a language, inline code, and links. Use no bold or
  italic text.
- Keep `README.md` and every `AGENTS.md` as indexes or digests, never rule dumps or filesystem
  inventories.
- State what is true now. Decision records are the only history-bearing document class.
- Keep exploratory material in `.draft/`; promotion is a rewrite into the owning zone.

## Executable artifacts

- A shipped executable requires a pre-commit gate, an accept and reject control under `just test`,
  and every dependency in `flake.nix`.
- Run `just check` before handoff. It checks without formatting.
- Run `just manifest` after editing anything under `gates/`, `.markdownlint/`, `scripts/verify.sh`,
  or the managed pre-commit block; the instance manifest records their hashes.
- `VERSION` is the release source of truth. Never author a tag: `just release` derives it, and
  `_docs/guides/release.md` owns the sequence.
- Every failure message a gate prints cites a rule ID that a spec defines.
- Never edit a generated dogfood template independently of its canonical copy under `templates/`.

## Routing

- Method-specific routing: `method/AGENTS.md`.
- Comparison-document routing: `comparison-docs/AGENTS.md`.
- Distribution and ownership: `_docs/specs/SPEC-distribution.md`.
- Format and budgets: `_docs/specs/SPEC-docs-format.md`.
- Decision records: `_docs/specs/SPEC-decision-records.md`.
- Cutting a release: `_docs/guides/release.md`.
