# Comparison Docs

A format for the document that answers "should I use this instead of that". The matrix indexes verdicts; each verdict points at a scenario that was run and dated. The separation is what lets a reader check a claim instead of believing it, and what lets the document be refreshed a year later by someone who did not write it.

## Problem

Two failures produce most comparison pages. The first is a matrix built by reading everyone's documentation: fast to write, impossible to re-verify, wrong within a release, and indistinguishable from an honest one. The second is a matrix of bare emoji: scannable and unreadable, because the symbol is announced by its Unicode name to a screen reader, vanishes in monochrome, and carries no qualification for the middle state where most real answers live.

## Model

```text
                 holds                           changes              read
  matrix         one verdict per pair            when a run changes   always, first
  method         the scenario, once per row      when the test does   when a reader doubts
  evidence       what one subject actually did   per subject, dated   on demand

  a cell holds a symbol, a word, and at most one link
```

## Chapters

| #        | Chapter                                           | One-line hook                                                       |
| -------- | ------------------------------------------------- | ------------------------------------------------------------------- |
| 0        | [Model](./00-model.md)                            | A verdict is an observation, and row selection is the honest part.  |
| 1        | [Table shape](./01-table-shape.md)                | Five columns, split by theme, rows phrased as observable behavior.  |
| 2        | [Verdicts](./02-verdicts.md)                      | Six states, symbol plus word, and why a blank cell is not a value.  |
| 3        | [References](./03-references.md)                  | Label links to the method, verdict links to the evidence.           |
| 4        | [Scenarios](./04-scenarios.md)                    | The method names no expected outcome; the evidence names a version. |
| 5        | [Freshness](./05-freshness.md)                    | Every table is dated, and untested beats a stale yes.               |
| 6        | [Gates](./06-gates.md)                            | Five working commands, and the rules no command can check.          |
| 99       | [Checklist](./99-checklist.md)                    | What must pass before the document merges.                          |
|          | [Sources](./SOURCES.md)                           | The research behind each rule, with confirmation dates.             |
| Template | [Comparison](../templates/TEMPLATE-comparison.md) | Start one.                                                          |

## Apply the genre

1. Instantiate the knowledge-base profile or copy [the stable template](../templates/TEMPLATE-comparison.md) into the project's explanation zone.
2. Name one theme and at most three alternatives that compete in it.
3. Write the method for each row before filling any cell.
4. Run every method against every subject. Anything not run stays `❓ untested`.
5. Fill the `Verified:` line with the run date and every subject version.
6. Wire the five commands from [06 — Gates](./06-gates.md), with the escaped-pipe check ahead of the markdown formatter.
7. Record the re-verification cadence where the project tracks perishable facts.

Start with one theme. A matrix that covers everything on the day it ships covers nothing six months later, because a refresh nobody can finish is a refresh nobody starts.

## House format

This shelf obeys [06 — Format](../method/06-format.md). Chapters stay at or below 200 lines, carry no bold or italics, and spend prose only on a decision, a hazard, or a non-obvious constraint.
