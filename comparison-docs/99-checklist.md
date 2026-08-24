# 99 — Checklist

What must pass before a comparison document merges. Each line names the chapter that owns the rule,
so a failure is read at its source rather than argued at the checklist.

## The claim

- Every verdict describes a run the author performed, not a feature list they read
  ([00](./00-model.md)).
- Every capability not run carries `❓ untested`, not an inferred verdict ([00](./00-model.md)).
- The method was identical for every subject in its row ([00](./00-model.md)).
- The document includes the rows where its own subject loses ([00](./00-model.md)).
- No row is phrased so only one subject's design can satisfy it ([00](./00-model.md)).

## The table

- No table exceeds five columns, or six with every heading and verdict under twelve characters
  ([01](./01-table-shape.md)).
- Each section heading names the question its table answers ([01](./01-table-shape.md)).
- No two subjects share a column unless every verdict in that table matches ([01](./01-table-shape.md)).
- Every table is introduced by a complete sentence ([01](./01-table-shape.md)).
- The layout was checked at 320 CSS pixels and 200 percent zoom ([01](./01-table-shape.md)).

## The verdicts

- Every cell holds one verdict from the fixed vocabulary, unchanged ([02](./02-verdicts.md)).
- Every symbol is followed by its word ([02](./02-verdicts.md)).
- No cell is empty and none holds a bare dash ([02](./02-verdicts.md)).
- A legend sits immediately above the first table and lists only the verdicts used
  ([02](./02-verdicts.md)).

## The references

- Every row label links to its method ([03](./03-references.md)).
- Every qualified verdict links to its evidence, and unqualified verdicts are plain text
  ([03](./03-references.md)).
- No cell carries two references ([03](./03-references.md)).
- Every anchor resolves to a real heading, and no heading is duplicated within its section
  ([03](./03-references.md)).

## The evidence

- Every method is a numbered list naming what to record, with no expected outcome
  ([04](./04-scenarios.md)).
- Every evidence section names a version and a date ([04](./04-scenarios.md)).
- Every scenario runs from a state the reader can reach ([04](./04-scenarios.md)).
- Any verdict resting on timing or intermittency states its run count ([04](./04-scenarios.md)).

## The dates

- Every table carries a `Verified:` line with the run date and every subject version
  ([05](./05-freshness.md)).
- Nothing was carried across a subject's major version without re-running ([05](./05-freshness.md)).
- The re-verification cadence is recorded where the project tracks perishable facts
  ([05](./05-freshness.md)).

## The gates

- All five commands in [06 — Gates](./06-gates.md) pass.
- The escaped-pipe check runs ahead of the markdown formatter in the hook order
  ([06](./06-gates.md)).
