# Hold the canon record with a canon check

## Context and Problem Statement

A spec was reworded without regenerating the instance record, so the recorded digest described bytes that no longer existed. The change passed a green hook run.

`sdd verify` saw it. It printed `DRIFT adopted file requires reconciliation` and exited 0, so the hook wired to it reported a pass. That is the verifier working as specified: managed drift fails, adopted drift is a note, and the split is what the ownership classes mean. An adopted file is seeded once and owned locally, and editing one is why a project adopts the framework.

This repository is the exception. Its record is generated from its tree by `sdd self-manifest` rather than owned against a baseline, so a difference here is not ownership exercised; it is a step skipped.

## Considered Options

- `add a canon-side check` — chosen.
- `make the verifier exit non-zero on adopted drift` — rejected: every instance would go red on its first edit to its own specs, which is the normal case and the point of adopting.
- `have the hook judge the verifier's output` — rejected: it relocates the same wrong rule one layer up, failing instances for the same edit.

## Decision Outcome

Chosen option: `add a canon-side check`. `release:the-canon-record-describes-its-tree` holds this repository's committed record to the files the same commit carries, verified by a cargo test that never ships. The verifier is unchanged, so what an instance experiences is unchanged.

A second gap surfaced alongside it: this repository's managed block is hand-maintained rather than installed, and nothing held it to the registry. The same suite now checks it wires every gate.

## Consequences

- Good: the obligation stated in the agent instructions fails a check instead of resting on the author remembering it.
- Good: a gate added to the registry cannot silently go unrun in the repository that authors it.
- Bad: two more canon-only invariants an instance cannot inherit, widening the gap between how this repository is checked and how one it installs into is.

## Status

Accepted
