# Track upstream dependencies with an offline gate and an explicit network check

## Context and Problem Statement

The vendored SimpleEnglish surface ages: upstream cuts new releases. An instance must verify and gate offline, so a freshness check cannot reach the network on every commit. A stale dependency must still surface, and a person must decide whether to accept an upstream move rather than have automation apply it.

## Considered Options

- `split offline freshness from an explicit network comparison` — chosen.
- `fetch upstream during verify` — rejected: it breaks the offline guarantee and gates routine work on a network.
- `auto-update the vendored surface on a schedule` — rejected: an upstream move can break the integration boundary, so a human reviews the diff first.

## Decision Outcome

Chosen option: `split offline freshness from an explicit network comparison` — the tracking registry records each perishable source with a cadence, and a pre-commit gate fails an overdue or dangling entry offline. One explicit command, `sdd track check`, compares a pinned Git revision to its upstream over the network and writes nothing. Movement is a finding, not an update.

Enforced by `tracking:an-overdue-entry-blocks`, `tracking:a-declared-dependent-exists`, and `tracking:an-upstream-check-does-not-edit-the-tree`, with `distribution:instances-operate-offline` kept intact.

## Consequences

- Good: routine verification stays offline, and a stale dependency blocks with an actionable recovery path.
- Good: a moved upstream is reviewed and vendored deliberately, never applied by a timer.
- Bad: a person must run the network check and act on it, because no automation advances the pinned revision.

## Status

Implemented

Enacted by `src/domain/tracking.rs`, `src/services/tracking.rs`, `src/gates/tracking_registry.rs`, and the `sdd track` command.
