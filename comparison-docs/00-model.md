# 00 — Model

A comparison document answers "should I use this instead of that" with observations rather than
claims. The matrix is an index; every verdict in it points at a scenario someone ran. This chapter
fixes what each layer owns and what wins when two of them disagree.

## Three layers

```text
               holds                           changes              read
matrix         one verdict per pair            when a run changes   always, first
method         the scenario, once per row      when the test does   when a reader doubts
evidence       what one subject actually did   per subject, dated   on demand
```

- The matrix MUST NOT hold a qualification. A cell that needs a sentence links to one.
- The method MUST be identical for every subject in its row.
- The evidence MUST name the version tested and the date it was tested.

A row where each subject was probed differently compares nothing, and it is the failure that hides
best: every cell looks filled.

## A verdict is an observation

- A verdict MUST describe a run the author performed.
- A verdict MUST NOT be derived from a subject's feature list, changelog, or marketing page.
- A capability the author has not run MUST carry the unverified state, never an inferred one.

Precedence when sources disagree: the observed run, then the subject's own documentation, then
silence. Silence is not a negative result. A subject that documents nothing about a capability and
was never tested is unverified, not unsupported.

A matrix assembled by reading documentation is cheap to write, impossible to re-verify, and wrong
within a release. The scenario is the artifact that survives; the table is a view over it.

## Row selection is the honest part

- A document MUST include the rows where its own subject loses.
- A row MUST NOT be phrased so only one subject's design can satisfy it.

A matrix whose subject wins every row is evidence about how the rows were chosen. Take the rows from
what a reader actually has to decide. A reader who finds one honest loss trusts the other twenty
rows; a reader who finds none trusts nothing.

## Subject first

- The document's own project MUST occupy the first data column.
- The alternatives MAY differ between tables within one document.

The subject anchors the reader's eye at the left edge where scanning starts, and holding it fixed
across tables lets the alternatives rotate by theme without the reader relearning the layout.

## What this genre is not

- A capability list for one project. That is reference material; it needs no columns.
- A migration guide. That is a procedure, and [10 — Procedures](../method/10-procedures.md) owns its
  shape.
- An argument. A document that argues belongs beside the matrix, not inside it, so a reader can
  check the observations without reading the case built on them.
