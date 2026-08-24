# Make precedence phase-dependent

## Context and Problem Statement

Precedence ran one way: code beats spec beats decision record. The project now writes specs before
the code that satisfies them, so at the moment work begins the spec is the only statement of the
agreement and the code is, by definition, wrong. A single precedence order forces a false choice:
either the failing state of the code discredits the spec, or every greenfield spec needs a
disclaimer explaining why it does not describe the running system.

## Considered Options

- `phase-dependent precedence` — chosen.
- `code always wins` — rejected: makes a spec written first indefensible, since the absent behavior
  would count as evidence against the agreement.
- `spec always wins` — rejected: turns every stale spec into a standing order to break working code.
- `per-project inversion declared in prose` — rejected: every adopting project would restate the
  exception, and the restatements would drift.
- `status field marking spec-first requirements` — rejected: stored state that the verification
  command already derives, and a second copy that rots.

## Decision Outcome

Chosen option: `phase-dependent precedence` — for observed behavior the code wins; for agreed
behavior the spec wins. The marker separating the two is mechanical: a rule is agreed behavior while
an open entry document in the plan zone cites its ID, and observed behavior otherwise.

A failing verification command on an agreed rule is therefore an unimplemented requirement, not a
broken spec, and requirement state is derived by running the command rather than stored anywhere.

Enforced by `spec-to-code:a-spec-may-lead-its-code` and
`spec-to-code:an-entry-document-cites-rule-ids`.

## Consequences

- Good: a spec can be merged before its implementation without a disclaimer or a status field.
- Good: which document wins any disagreement is decided by a grep for the rule ID, not by judgment.
- Bad: an entry document left open after its work stalls keeps its rules in agreed mode, so stale
  plan entries now distort precedence and must be closed or abandoned explicitly.

## Status

Accepted
