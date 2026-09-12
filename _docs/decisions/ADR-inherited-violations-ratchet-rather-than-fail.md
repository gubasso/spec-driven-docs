# Inherited violations ratchet rather than fail

## Context and Problem Statement

A project adopting this convention brings documents written before it. Every budget rule stated its cap with no exception, so an inherited corpus failed its first commit on files nobody touched. The one relief, a flat list the chapter gate read, was documented nowhere and skipped a listed file before counting it, so a listed chapter could grow without limit.

## Considered Options

- `record each violation per gate, path, and dimension, and let the record only shrink` — chosen.
- `fail on install day` — rejected: adoption becomes a rewrite of the corpus, the cost the method exists to avoid.
- `baseline inside sdd init` — rejected: installation is a payload write with its own preview, and accepting debt is a policy decision that deserves a second one.
- `keep a list that skips a file, or warn on a stale ceiling` — rejected: either one permits regrowth.
- `raise the budget` — rejected: `docs-format:every-budget-carries-a-gate` forbids it, and a cap that admits one document admits the next.
- `one verb that baselines or migrates by the state of the filesystem` — rejected: a request for a format conversion could accept violations instead.

## Decision Outcome

Chosen option: `record each violation per gate, path, and dimension, and let the record only shrink` — the record carries what the project inherited and nothing it adds afterwards.

Nothing delivers the file. `sdd debt baseline --apply` creates it once, `sdd debt migrate --apply` converts the flat list, and `sdd debt tighten --apply` lowers it. `baseline` refuses where a file exists, so a second inherited corpus has no widening command: the documents are fixed. That refusal is deliberate.

Enforced by `budget-debt:a-recorded-dimension-only-shrinks` and `budget-debt:debt-is-created-by-an-explicit-act`.

## Consequences

- Good: a brownfield landing commits green after one baseline, and no carried document can grow.
- Good: every ceiling is reachable only downward.
- Bad: shrinking a carried document meets one failure naming the tightening.
- Bad: an instance carrying the flat list runs one migration.

## Status

Implemented

Enacted by `src/domain/debt.rs`, `src/gates/budget.rs`, and `src/commands/debt.rs`.
