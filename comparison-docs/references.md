# References

A table cell cannot hold prose, so every qualification leaves the table by a link. This chapter fixes the two places a link can start, where each one lands, and how many are allowed.

## Two levels, two destinations

- A row label MUST link to the method for that row.
- A verdict MAY link to the evidence for that one subject.
- The two MUST NOT point at the same heading.

The row label answers "how was this decided", once, for every subject in the row. The verdict answers "what did this one do", and only where the symbol is not the whole answer. Collapsing them loses the distinction that makes the matrix checkable.

```markdown
| [Survives a fork bomb](#survives-a-fork-bomb) | [⚠️ partial](#fork-bomb-podman) | ✅ yes |
```

The row label is always a link. The first verdict is a link because `⚠️ partial` is meaningless without the limit it refers to. The second is plain text because `✅ yes` is the whole answer.

## Link to a heading, not to a footnote

- A reference MUST be an ordinary markdown link to a heading anchor.
- A reference MAY be a footnote only for short provenance: a version number, a date, an issue URL.

A heading anchor is checkable. A markdown linter with relative-link verification resolves the file and the `#fragment` against real headings on disk. As a result, a renamed section breaks the build instead of breaking silently. Footnotes are a renderer extension outside the core table specification, they render differently across sites, and no linter validates where one lands.

The destination also matters to the reader. A footnote drops them at the bottom of the page with no heading to orient by. A heading anchor drops them at a titled section they can link to, quote, and navigate back from.

## One reference per cell

- A cell MUST carry at most one reference.

Two references in a cell mean the row is really two rows. A reader who meets two links in one cell stops scanning and starts reading, which is the cost [Table shape](./table-shape.md) and [Verdicts](./verdicts.md) already price. The rule is this shelf's own. The MDN schema permits an array of notes on one support statement, so it imposes no such limit.

## Where the destination lives

- The method MUST be a heading in a document the row label can reach.
- Evidence MAY live in the same document, below the matrix, or in a separate document.
- A heading used as an anchor MUST be unique within its section.

Same document is the default: the round trip is one jump each way and the reader never loses the table. Split to a separate document when the evidence for one theme outgrows the matrix that indexes it. Keep one file per theme rather than one file per subject, so the shared method stays beside the results it produced.

Uniqueness is not cosmetic. Repeating `### Result` under every capability produces a set of anchors that a linter accepts and a reader cannot distinguish. Name the heading for the pair it describes: `### Fork bomb, podman`.

## The escape hatch: a notes column

- A table MAY replace verdict-level links with one trailing notes column when it compares at most two subjects.

The notes column is what several published matrices use, and it reads well when the width is available. It costs a column against the budget in [Table shape](./table-shape.md). It puts prose back into a cell that cannot hold a paragraph, a list, or a line break. Above two subjects the linked verdict wins on both counts. Do not use both mechanisms in one document.

## Sources

- The GitHub Flavored Markdown specification, on cells holding inline content only and block-level elements being excluded: <https://github.github.com/gfm/>
- MDN browser-compat-data schema, on `notes` being a string or an array of strings: <https://github.com/mdn/browser-compat-data/blob/main/schemas/compat-data-schema.md>
- caniuse, whose cell carries a support state plus a numbered note pointing at the catch: <https://github.com/Fyrd/caniuse/blob/main/CONTRIBUTING.md>
- EMQX, a published matrix using a trailing notes column across thematic tables: <https://docs.emqx.com/en/emqx/latest/getting-started/feature-comparison.html>
