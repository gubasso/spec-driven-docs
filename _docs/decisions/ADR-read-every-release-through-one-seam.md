# Read every release through one seam

## Context and Problem Statement

Every landing verb read the payload compiled into the binary and nothing else. The installer computed a target's whole state from the embedded assets, the upgrader reinstalled through that function, and the verifier held a record to the same constants. So the only release the engine could describe was itself. Answering a question about another release meant obtaining that release's own executable, which a plan cannot do.

## Considered Options

- `a content seam plus a separate resolver` — chosen.
- `keep reading the embedded payload` — rejected: it is the coupling, and every question about another release stays unanswerable.
- `one interface that resolves and serves bytes` — rejected: an apply that can resolve can resolve twice, and the second answer need not be the one the plan described.
- `teach the boundary to parse the projection` — rejected: the transport would then be a second place to change whenever the projection grows.

## Decision Outcome

Chosen option: `a content seam plus a separate resolver`. `ReleaseBundle` answers with a manifest and with blobs by digest, and says nothing about where the bytes came from. `ReleaseResolver` turns a selector into exactly one verified bundle, once, and freezes the exact version, the registry checksum, and a digest over the content. Three bundles exist: the embedded one, a crate fetched from the registry and verified against the index checksum, and a directory for tests.

Resolution reaches the network and content access never does. That split is what lets a plan carry an identity an apply cannot quietly re-resolve.

Enforced by `bundle:a-release-is-read-through-one-seam` and `bundle:resolution-happens-once-and-writes-only-the-cache`.

## Consequences

- Good: the engine can describe a release it was not built as, which every later verb needs.
- Good: the whole suite exercises the boundary, because the ordinary path goes through it.
- Bad: one network read now exists in a tool that had none, with the archive as untrusted input.
- Bad: every landing verb gained a parameter, and a caller must say which release it means.

## Status

Superseded by [ADR-render-the-candidate-this-binary-carries](./ADR-render-the-candidate-this-binary-carries.md)
