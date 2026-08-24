# Use the slug as a decision record's identifier

## Context and Problem Statement

Decision records were named `ADR-<number>-<slug>.md`, with the number allocated by scanning the
directory for the highest existing value. Work happens across several branches, worktrees, and
parallel agent sessions at once, so two records are routinely created against the same highest value.
Both are correct when written and collide on merge, and the resolution is a rename that breaks every
citation already made.

## Considered Options

- `slug as the identifier` — chosen.
- `sequential number` — rejected: allocation requires a coordinator that parallel branches do not have.
- `number assigned at merge` — rejected: every citation written before merge points at a name that
  changes.
- `ULID or UUID prefix` — rejected: solves distributed uniqueness at the cost of an unreadable
  filename.
- `date prefix` — deferred: revisit if chronological ordering in a file listing becomes a need the
  log itself cannot serve.

## Decision Outcome

Chosen option: `slug as the identifier` — a record is named `ADR-<slug>.md` and the
filename is immutable once merged.

Two branches choosing the same slug have collided on a subject rather than on a counter, which is a
conflict worth surfacing. Stability, the only load-bearing property the number had, comes from
declaring the filename immutable instead.

Enforced by `decision-records:filename-carries-no-digit`.

## Consequences

- Good: records can be created in parallel with no coordination.
- Good: a citation carries meaning without opening the file.
- Bad: a record whose title is later improved keeps a filename that no longer matches it.
- Bad: renaming to correct a genuine identity error requires repointing every live citation in the
  same change.

## Status

Accepted
