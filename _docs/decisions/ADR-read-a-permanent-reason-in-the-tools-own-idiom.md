# Read a permanent reason in the tool's own idiom

## Context and Problem Statement

The permanent-exception rule accepted one spelling: an `sdd: permanent` marker. A generated artifact carries suppressions this repository must not edit. The release-kit payload renders `# zizmor: ignore[dangerous-triggers] <reason>` into a workflow, naming the analyzer, the finding, the site, and the reason. The gate rejected it, because the reason wore the wrong brand.

## Considered Options

- `a reason position the suppressing tool defines, with the marker as the fallback` — chosen.
- `a path exclusion for generated files` — rejected: it hides every present and future suppression in the file, which is the wrong granularity for executable repository policy.
- `a change in the generator` — rejected: it puts this convention's marker inside a tool that must stay unaware of it.
- `an external suppression registry` — rejected: it duplicates a rationale the artifact already carries, and it needs stable identities and stale-entry detection to be safe.

## Decision Outcome

Chosen option: `a reason position the suppressing tool defines, with the marker as the fallback` — a suppression states its permanent reason where its own tool reads one, and carries the `sdd: permanent` marker where the tool defines no such position.

Only a position the tool defines counts. Nearby prose is not a reason, so an unrelated comment closes nothing. A case named inside a tool's reason stays a known issue, and the marker written beside a case is still the conflict it was.

Enforced by `spec-to-code:a-permanent-exception-states-its-reason`, which [ADR-let-a-permanent-exception-state-its-reason](./ADR-let-a-permanent-exception-state-its-reason.md) introduced.

## Consequences

- Good: a generated artifact satisfies the rule in its own idiom, with no path exception and no byte of this convention inside it.
- Good: the accepted forms follow what linters already converged on, so authors write the idiom they know.
- Bad: each form's reason channel is a parser this gate now owns, and a tool that changes its syntax breaks one.

## Status

Implemented

Enacted by `src/gates/suppression_names_its_case.rs`.
