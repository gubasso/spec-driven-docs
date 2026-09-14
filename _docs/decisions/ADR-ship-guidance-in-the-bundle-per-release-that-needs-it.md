# Ship guidance in the bundle, per release that needs it

## Context and Problem Statement

[ADR-retire-the-migration-guide-chain](./ADR-retire-the-migration-guide-chain.md) removed the per-release briefing, and its reasons hold: a mandatory file per release is ceremony, and one missing link stranded every older instance. It recorded one bad consequence, and this is the bill. An operator behind reads one changelog and works out which paragraphs reach their target.

## Considered Options

- `one file per release that needs one, with a ledger` — chosen.
- `keep the changelog` — rejected: prose for a different reader, and parsing it would make its wording an API.
- `a mandatory file per release` — rejected: the chain the retired record removed, which strands an interval when one file goes missing.
- `an optional file with no ledger` — rejected: an absence would mean both nothing to do and somebody forgot.

## Decision Outcome

Chosen option: `one file per release that needs one, with a ledger`. `guidance/index.toml` carries one entry per release from the capability floor, selecting a file or `none`. A prose file exists only where a release asks something, so no missing file strands an interval and a missing entry fails at release time.

A changelog is written for a reader deciding whether to upgrade. A plan is read by somebody who already decided. The two answer different questions, and the bundle owes the second.

A step is data: an identifier, a kind, whether it breaks, the destinations it reaches, who takes it, and the body it points at. The plan filters steps against the destinations the target has. A breaking step is a decision the operator accepts with the body in view. The identifier derives from the release and the slug, so a reworded body moves no identity.

Enforced by `release:every-release-declares-what-it-asks`.

## Consequences

- Good: an operator behind gets the steps that reach their target.
- Good: the ledger cannot strand an interval, which retired the chain.
- Bad: a release has one more thing to declare.
- Bad: the history is whole, so the payload grows a line a release.

## Status

Accepted
