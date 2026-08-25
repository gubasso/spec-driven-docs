---
digest-of: comparison-docs
last-synced: 2026-08-24
token-estimate: 600
---

# AGENTS

## Scope

The format for a document comparing one project against its alternatives. Covers the artifact model, table width and thematic splitting, the verdict vocabulary, the two reference levels, the shape of a scenario and its evidence, dating and refresh, and the gates.

The documentation method itself — zones, specs, decision records, the markdown register — belongs to `../method/`. This shelf owns one genre inside it.

## How to use this shelf

Load this file, find the owning chapter below, then read that chapter. Do not read the shelf linearly. Building a comparison document from scratch means reading `00-model.md` first, then following `README.md` under Apply the genre.

## Where the rules live

| Question the agent arrives with                             | Owning chapter      |
| ----------------------------------------------------------- | ------------------- |
| What is a verdict allowed to be based on?                   | `00-model.md`       |
| How wide may the table be, and when do I split it?          | `01-table-shape.md` |
| Which verdict do I put in this cell, and how is it written? | `02-verdicts.md`    |
| Where may a link start, where does it land, how many?       | `03-references.md`  |
| What shape is the scenario and its result?                  | `04-scenarios.md`   |
| How is the document dated and refreshed?                    | `05-freshness.md`   |
| What command enforces this rule?                            | `06-gates.md`       |
| What must pass before this merges?                          | `99-checklist.md`   |
| Why does this rule exist?                                   | `SOURCES.md`        |

Template: `../templates/TEMPLATE-comparison.md`.

## Non-negotiables

- A verdict describes a run the author performed; a capability not run is `❓ untested`.
- The method is identical for every subject in its row.
- The document includes the rows where its own subject loses.
- The subject occupies the first data column, in every table.
- A cell holds one verdict from the fixed six, written as a symbol followed by its word.
- A cell is never empty and never a bare dash.
- A cell carries at most one reference, and that reference is a heading anchor.
- A row label links to the method; a verdict links to the evidence; the two never coincide.
- Every table carries a `Verified:` line with the run date and every subject version.
- A pipe inside a code span in a table row is escaped as `\|`, and the check for it runs before the markdown formatter.
- Use no bold or italics, and give every fenced block a language.

## Maintenance

- Regenerate when this shelf's chapters change.
- This digest is a router, never a rules home; the owning chapter wins on disagreement.
