# A delivered gate parses no foreign grammar

## Context and Problem Statement

`suppression-names-its-case` cited two rules with different subjects. One resolved a `KI-` token this convention defines. The other judged whether a suppression stated a reason where the suppressing tool reads one. The second cost the parser: 1938 lines for one gate, against 7007 for the whole gate tree, modelling 20 suppression forms, 15 filename suffixes, 7 reason positions, and a per-language lexer.

The table had no end. It grew in v0.5.1, v0.6.4, and v0.6.5, and the module documentation still named three surfaces the scan could not reach. The half that cost nothing never fired here: this repository keeps no known-issue record.

Each language enforces the reason rule better, in its own toolchain. clippy ships `allow_attributes_without_reason`, ESLint has `eslint-comments/require-description`, and Ruff has `RUF100`.

## Considered Options

- `delete the reason rule and keep the citation check` — chosen.
- `keep the reason rule as an opt-in an adopter wires` — rejected: the lines stay and the table keeps growing, which trades a scope problem for a maintenance one.
- `extend the table again` — rejected on the evidence of three widenings in five releases.
- `keep a reduced form table` — rejected: any table that recognizes a suppression is the table this removes.

## Decision Outcome

Chosen option: `delete the reason rule and keep the citation check` — a delivered gate reads a token this convention defines and parses no grammar somebody else defines.

`release:a-delivered-gate-reads-what-the-convention-owns` bounds which paths a gate judges, and a project answers it. This bounds which syntax a gate judges, and no project can widen it. The rule keeps its ID under `docs-specs:rule-id-outlives-its-sentence`, so every existing citation resolves.

Enforced by `spec-to-code:a-suppression-names-its-case`.

## Consequences

- Good: roughly 1600 lines leave the tree, and no adopter's language is modelled here.
- Good: the scan reaches every file, including the ones no suffix table named.
- Bad: a suppression naming neither a case nor a reason goes unreported, and an adopter turns on its own linter instead.

## Status

Implemented

Supersedes [ADR-let-a-permanent-exception-state-its-reason](./ADR-let-a-permanent-exception-state-its-reason.md) and [ADR-read-a-permanent-reason-in-the-tools-own-idiom](./ADR-read-a-permanent-reason-in-the-tools-own-idiom.md). Enacted by `src/gates/suppression_names_its_case.rs`.
