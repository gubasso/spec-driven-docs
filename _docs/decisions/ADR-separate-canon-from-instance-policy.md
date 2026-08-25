# Separate canon from instance policy

## Context and Problem Statement

Reusable method gates must not encode a consumer's planning zones, retired names, or local debt. Combining them would make upgrades overwrite policy the instance owns.

## Considered Options

- `managed canon with local overlays` — chosen.
- `one fully managed configuration` — rejected: it would erase project policy.
- `copy without ownership metadata` — rejected: drift and safe upgrades would be ambiguous.

## Decision Outcome

Chosen option: `managed canon with local overlays` — managed files project shared machinery, adopted files become local, and marker blocks reserve only their delimited integration.

Enforced by `distribution:manifest-identifies-every-owned-file` and `distribution:initialization-preserves-project-content`.

## Consequences

- Good: reusable gates stay neutral.
- Good: projects keep local rules and debt.
- Bad: some upgrades require explicit reconciliation.

## Status

Accepted
