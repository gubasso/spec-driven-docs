# License the canon under CC BY

## Context and Problem Statement

The repository combines documentation with executable maintenance artifacts. A single license keeps
the initial distribution unambiguous while the project establishes its release surface.

## Considered Options

- `CC BY 4.0 throughout` — chosen.
- `CC BY for prose and MIT for code` — deferred: revisit when executable reuse needs a software-specific license.
- `Apache-2.0 throughout` — rejected: the knowledge product is documentation-first.

## Decision Outcome

Chosen option: `CC BY 4.0 throughout` — the license field and repository license use the same identifier.

## Consequences

- Good: every shipped artifact has one clear license.
- Bad: CC BY is less conventional for shell and Python code.

## Status

Superseded by [ADR-split-the-license-by-product](./ADR-split-the-license-by-product.md).
