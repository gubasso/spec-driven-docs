# Comparison Docs

A format for the document that answers whether to use this instead of that. The matrix indexes verdicts. Each verdict points at a scenario that was run and dated. The separation is what lets a reader check a claim instead of believing it. The same separation is what lets someone who did not write the document refresh it a year later.

## Problem

Two failures produce most comparison pages. The first is a matrix built by reading everyone's documentation. It is fast to write, impossible to re-verify, and wrong within a release. It also looks exactly like an honest one. The second is a matrix of bare emoji, which is scannable and unreadable at once. The symbol carries a name from the character data rather than the author's verdict, and it vanishes in monochrome. The symbol also carries no qualification for the middle state where most real answers live.

## Model

```text
                 holds                           changes              read
  matrix         one verdict per pair            when a run changes   always, first
  method         the scenario, once per row      when the test does   when a reader doubts
  evidence       what one subject actually did   per subject, dated   on demand

  a cell holds a symbol, a word, and at most one link
```

## Chapters

| Chapter                                                    | One-line hook                                                       |
| ---------------------------------------------------------- | ------------------------------------------------------------------- |
| [Model](./model.md)                                        | A verdict is an observation, and row selection is the honest part.  |
| [Table shape](./table-shape.md)                            | Five columns, split by theme, rows phrased as observable behavior.  |
| [Verdicts](./verdicts.md)                                  | Six states, symbol plus word, and why a blank cell is not a value.  |
| [References](./references.md)                              | Label links to the method, verdict links to the evidence.           |
| [Scenarios](./scenarios.md)                                | The method names no expected outcome. The evidence names a version. |
| [Freshness](./freshness.md)                                | Every table is dated, and untested beats a stale yes.               |
| [Gates](./gates.md)                                        | Five working commands, and the rules no command can check.          |
| [Checklist](./checklist.md)                                | What must pass before the document merges.                          |
| [Sources](./SOURCES.md)                                    | The research behind each rule, with confirmation dates.             |
| [Comparison template](../templates/TEMPLATE-comparison.md) | Start one.                                                          |

## Apply the genre

1. Instantiate the knowledge-base profile or copy [the stable template](../templates/TEMPLATE-comparison.md) into the project's explanation zone.
2. Name one theme and at most three alternatives that compete in it.
3. Write the method for each row before filling any cell.
4. Run every method against every subject. Anything not run stays `❓ untested`.
5. Fill the `Verified:` line with the run date and every subject version.
6. Wire the five commands from [Gates](./gates.md), with the escaped-pipe check ahead of the markdown formatter.
7. Record the re-verification cadence where the project tracks perishable facts.

Start with one theme. A matrix that covers everything on the day it ships covers nothing six months later, because nobody starts a refresh nobody can finish.

## House format

This shelf obeys [Format](../method/format.md). Chapters stay at or below 200 lines, carry no bold or italics, and spend prose only on a decision, a hazard, or a non-obvious constraint.
