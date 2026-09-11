# A project declares what its gates read

## Context and Problem Statement

`ADR-a-delivered-gate-reads-what-the-convention-owns` diagnosed two real failures and fixed them in the wrong layer. It chose one boundary for every gate, when what the project knew was which region another tool owned. Its rule exempted any gate resolving its own scope, 19 of the 30 rows, so it constrained 3. It also anchored `no-personal-path` to the documentation root, taking the breadth that is a leak check's whole value.

## Considered Options

- `let the project declare the paths each gate reads` — chosen.
- `hand-edit the managed block` — rejected: an edited block aborts the upgrade, so this needs a partial structural hash or a per-hook merge.
- `a filter language beyond globs` — rejected: globs answer every observed case, and a grammar is a surface to version.
- `per-rule filtering inside one gate` — rejected: the unit is the gate, and a gate needing two scopes is two gates.
- `make the rendered selection equal the matcher` — rejected: one side is a Rust byte regex and the other Python `re.search`, and a superset gives the same safety with no converter.

## Decision Outcome

Chosen option: `let the project declare the paths each gate reads` — the observed failure was a project knowing something this convention cannot know, so the answer belongs where that knowledge is.

Every row declares the subject paths it judges, and one matcher applies the declaration to every route a subject path takes. Support paths stay open: a filter reaching them would be an off switch rather than a scope control.

Enforced by `release:a-delivered-gate-reads-what-the-convention-owns`, `instance:the-project-declares-what-its-gates-judge`, and `instance:the-managed-block-agrees-with-the-declaration`.

## Consequences

- Good: `no-personal-path` judges the whole project again, and a project that needs a path exempt reserves it.
- Good: the registry is silent about no gate, and `sdd gate --explain` names the pattern that decided.
- Bad: an instance carries one more file, and editing it means running `sdd hooks --apply`.

## Status

Implemented

Supersedes `ADR-a-delivered-gate-reads-what-the-convention-owns`, unedited. Enacted by `src/domain/path_filter.rs` and `src/domain/instance_config.rs`.
