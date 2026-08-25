# Keep release migrations actionable

## Context and Problem Statement

An upgrade may require reconciliation of living rule IDs and local integration. Readers need current instructions for crossing a version boundary without turning project documentation into a release diary.

## Considered Options

- `imperative migration guides` — chosen.
- `changelog entries` — rejected: summaries do not provide executable reconciliation.
- `automatic adopted-file rewrites` — rejected: living specs are locally owned.

## Decision Outcome

Chosen option: `imperative migration guides` — each version transition states applicability, preconditions, managed changes, rule-ID changes, local action, verification, and rollback.

## Consequences

- Good: upgrades have a bounded procedure.
- Good: adopted content remains under local judgment.
- Bad: a release with living-rule changes requires guide maintenance.

## Status

Accepted
