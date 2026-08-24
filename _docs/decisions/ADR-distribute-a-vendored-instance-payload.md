# Distribute a vendored instance payload

## Context and Problem Statement

Projects need documentation rules to remain operable when the network or canon checkout is unavailable.
Runtime fetching would make linting and verification depend on external state.

## Considered Options

- `vendored, hash-tracked payload` — chosen.
- `runtime network fetch` — rejected: routine checks would not be self-contained.
- `documentation links only` — rejected: rules would lack local enforcement.

## Decision Outcome

Chosen option: `vendored, hash-tracked payload` — installation copies managed gates, configurations,
templates, and the verifier locally while the manifest records their hashes.

Enforced by `distribution:manifest-identifies-every-owned-file` and
`distribution:instances-operate-offline`.

## Consequences

- Good: an installed project can lint, test, and verify without upstream access.
- Good: drift is deterministic.
- Bad: releases must carry explicit upgrade machinery.

## Status

Accepted
