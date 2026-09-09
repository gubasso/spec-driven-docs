# Date the last check without judging its age

## Context and Problem Statement

A known-issue record stated the condition that removes its workaround and never stated when anyone last tested that condition. The retirement rule kept a workaround from having no stated exit. It did not keep one from outliving its bug, because nothing said whether the upstream was read last week or a year ago. A pinned tool already does better: a tracking entry carries a check source and a date beside it.

## Considered Options

- `a checked: date the gate reads for presence and shape` — chosen.
- `a checked: date with a cadence the gate enforces` — rejected: an old date over an upstream that has not moved is an accurate record, so failing it teaches people to touch the date rather than to read the upstream.
- `one entry per record in the tracking registry` — rejected: a registry entry per case is the file every branch edits, which the slug case id exists to avoid, and it holds a second copy of what the record states.

## Decision Outcome

Chosen option: `a checked: date the gate reads for presence and shape` — a masked or monitoring record carries the date its upstream state was last confirmed, and every other state carries none. The gate judges presence, the ISO shape, and a date that is not in the future. Whether the observation is recent enough is review's business.

The date is an observation rather than a third axis. Nothing decides from it, and no vocabulary bounds it, so the two axes that a record carries stay two.

Enforced by `known-issues:a-record-records-its-last-check`.

## Consequences

- Good: a reader tells a live watch from an abandoned one without reading the upstream.
- Good: the gate stays offline and reads current bytes alone, as every delivered gate does.
- Bad: a record written before this decision fails until the date is added, and the date is only as honest as the person who wrote it.

## Status

Implemented

Enacted by `src/gates/ki_checked_date.rs` and the `checked:` key in `templates/TEMPLATE-known-issue.md`.
