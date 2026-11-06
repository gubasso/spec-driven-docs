# A spec allows one Unenforced section

## Context and Problem Statement

The method said a spec carries a title, `## Purpose`, and `## Requirements`, and no other section. The lint shape could not hold that: its trailing `+` admits any heading after the requirements. Two shipped specs carried their own unenforced table, while the gates chapter kept one central table of 31 rules. Adopters copied the per-spec table under other names, and added diagnostics and background sections beside it. The rule and the practice disagreed.

## Considered Options

- `one optional ## Unenforced section after the requirements` — chosen.
- `full freedom after the requirements` — rejected: nothing stops the next `## Background`, and a reader no longer knows where a spec ends.
- `no trailing section, and one central unenforced list` — rejected: the list lives far from the rules it qualifies, and two shipped specs already showed the per-spec list working.

## Decision Outcome

Chosen option: `one optional ## Unenforced section after the requirements`. Its table lists the rules of that spec that no command decides, or decides only in part, with what a reviewer confirms. A diagnostic message is a fact and goes to reference, and any other trailing section moves to reference, explanation, or a rule. A delivered gate holds the shape, because MD043 cannot express "at most one optional named heading". The gate warns for one release before it fails, so adopters can rename first.

Enforced by `docs-specs:a-spec-follows-the-heading-shape` and `docs-specs:unenforced-rules-are-declared`.

## Consequences

- Good: each unenforced rule has one list, beside the rule it qualifies.
- Good: one named section is the whole allowance, so a stray section is a finding.
- Bad: every adopter that renamed or added trailing sections must move them, and an upgrade keeps adopted bytes, so a changed shipped spec reaches an adopter only by a port.

## Status

Implemented
