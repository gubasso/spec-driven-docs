# Checklist

What must pass before a comparison document merges. Each line names the chapter that owns the rule, so a failure is read at its source rather than argued at the checklist.

## The claim

- Every verdict describes a run the author performed, not a feature list they read ([model](./model.md)).
- Every capability not run carries `❓ untested`, not an inferred verdict ([model](./model.md)).
- The method was identical for every subject in its row ([model](./model.md)).
- The document includes the rows where its own subject loses ([model](./model.md)).
- No row is phrased so only one subject's design can satisfy it ([model](./model.md)).

## The table

- No table exceeds five columns, or six with every heading and verdict under twelve characters ([table shape](./table-shape.md)).
- Each section heading names the question its table answers ([table shape](./table-shape.md)).
- No two subjects share a column unless every verdict in that table matches ([table shape](./table-shape.md)).
- Every table is introduced by a complete sentence ([table shape](./table-shape.md)).
- The layout was checked at 320 CSS pixels and 200 percent zoom ([table shape](./table-shape.md)).

## The verdicts

- Every cell holds one verdict from the fixed vocabulary, unchanged ([verdicts](./verdicts.md)).
- Every symbol is followed by its word ([verdicts](./verdicts.md)).
- No cell is empty and none holds a bare dash ([verdicts](./verdicts.md)).
- A legend sits immediately above the first table and lists only the verdicts used ([verdicts](./verdicts.md)).

## The references

- Every row label links to its method ([references](./references.md)).
- Every qualified verdict links to its evidence, and unqualified verdicts are plain text ([references](./references.md)).
- No cell carries two references ([references](./references.md)).
- Every anchor resolves to a real heading, and no heading is duplicated within its section ([references](./references.md)).

## The evidence

- Every method is a numbered list naming what to record, with no expected outcome ([scenarios](./scenarios.md)).
- Every evidence section names a version and a date ([scenarios](./scenarios.md)).
- Every scenario runs from a state the reader can reach ([scenarios](./scenarios.md)).
- Any verdict resting on timing or intermittency states its run count ([scenarios](./scenarios.md)).

## The dates

- Every table carries a `Verified:` line with the run date and every subject version ([freshness](./freshness.md)).
- Nothing was carried across a subject's major version without re-running ([freshness](./freshness.md)).
- The re-verification cadence is recorded where the project tracks perishable facts ([freshness](./freshness.md)).

## The gates

- All five commands in [gates](./gates.md) pass.
- The escaped-pipe check runs ahead of the markdown formatter in the hook order ([gates](./gates.md)).
