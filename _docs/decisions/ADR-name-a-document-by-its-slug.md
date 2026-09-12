# Name a document by its slug

## Context and Problem Statement

The naming rule forbade a counter after a kind prefix and gave merge collision as the reason: two branches allocate the next number, and both files claim one identity. That reason does not stop at a kind prefix. Two shelves carried an ordinal on every chapter to encode a reading order, and a reader who met the rule first read the tree as inconsistent.

## Considered Options

- `slug every managed document and keep the order in the README` — chosen.
- `keep the ordinals on the chapter shelves` — rejected: an insertion renumbers every later file and breaks every link into them.
- `teach the CLI the reading order` — rejected: a parser with a stake in the README makes a second copy of the order to hold in step, for a convenience nobody asked for.
- `deliver the check as a gate` — rejected: the chapter shelves are canon subtrees no instance edits, and a project owner keeps the choice over documents `sdd` does not seed.

## Decision Outcome

Chosen option: `slug every managed document and keep the order in the README` — the merge-collision argument generalizes, so the rule does too. A reading order belongs in prose a reader reads, where an insertion costs one sentence and no file moves.

This repository enforces the rule with a cargo test under `tests/canon.rs`, under `ADR-split-gates-by-delivery-domain`. An instance receives the rule as a convention in its documentation block and its skills. The kind-prefixed set stays gated by `adr-filename-shape` and `ki-filename-shape`.

Enforced by `docs-foundations:a-kind-prefix-carries-a-slug` and `docs-foundations:a-document-directory-explains-itself`.

## Consequences

- Good: two branches can each add a chapter and both merge.
- Good: a chapter list from the CLI is an alphabetical index, and the README is the one place that states the order.
- Bad: `sdd method 00-model` stopped resolving, and the slug form is the only address.
- Bad: no command judges an instance's unprefixed documents.

## Status

Implemented

Enacted by `tests/canon.rs`, `method/README.md`, and `comparison-docs/README.md`.
