# Apply only a stored plan

## Context and Problem Statement

An apply that recomputed its intent would act on whatever the tree says when it runs, which need not be what the operator approved. The window between reading a preview and running the write is where a target changes: another process lands something, a file is edited, a release moves. The old verbs closed that window by not opening it: they previewed and applied in one invocation.

## Considered Options

- `store the plan and apply exactly that one` — chosen.
- `recompute and hope nothing moved` — rejected: the apply would do something nobody saw and call it the plan.
- `recompute and re-prompt on a difference` — rejected: an unattended run answers its own prompt, which is the same thing with a step in it.
- `hold the plan in memory` — rejected: an approval that cannot outlive the process cannot be given by somebody who left the room.

## Decision Outcome

Chosen option: `store the plan and apply exactly that one`. A plan is written under its fingerprint, owner-only and outside the target, with every byte it will write. The apply reads it, takes the target lock, re-observes, recomputes the fingerprint, and compares. A difference is a refusal naming every field and destination that moved. Nothing is staged before that passes.

A plan gate is a prompt and a revalidation is a program. The difference matters exactly when an agent is wrong in a way that looks like being right: a prompt asks it to check, and a program checks.

Execution reuses the transaction the skill installer proved: stage beside each destination, journal before the first replacement, replace one file at a time, and leave a journal the next invocation resolves.

Enforced by `reconcile:a-plan-is-stored-and-applied-by-its-id`, `reconcile:an-apply-refuses-a-plan-whose-inputs-moved`, and `reconcile:one-writer-holds-a-target`.

## Consequences

- Good: what ran is exactly what was read, or nothing ran.
- Good: an approval outlives the session that gave it.
- Bad: the tool keeps target-derived bytes outside the target.
- Bad: a plan expires, so a slow approval plans again.

## Status

Superseded by [ADR-render-the-candidate-this-binary-carries](./ADR-render-the-candidate-this-binary-carries.md)
