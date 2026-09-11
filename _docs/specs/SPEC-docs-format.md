# Documentation Format Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`docs-format:chapter-stays-within-200-lines` — A chapter stays within 200 lines](#docs-formatchapter-stays-within-200-lines--a-chapter-stays-within-200-lines)
  - [`docs-format:author-instructions-stay-within-budget` — Author instructions stay within budget](#docs-formatauthor-instructions-stay-within-budget--author-instructions-stay-within-budget)
  - [`docs-format:every-budget-carries-a-gate` — Every count-shaped budget carries a gate](#docs-formatevery-budget-carries-a-gate--every-count-shaped-budget-carries-a-gate)
  - [`docs-format:document-uses-structural-markdown-only` — A document uses structural markdown only](#docs-formatdocument-uses-structural-markdown-only--a-document-uses-structural-markdown-only)
  - [`docs-format:document-states-the-present` — A document states the present](#docs-formatdocument-states-the-present--a-document-states-the-present)
  - [`docs-format:fence-declares-a-language` — Every fence declares a language](#docs-formatfence-declares-a-language--every-fence-declares-a-language)
  - [`docs-format:prose-stays-unwrapped` — Prose stays unwrapped](#docs-formatprose-stays-unwrapped--prose-stays-unwrapped)

<!--TOC-->

## Purpose

Rules governing the markdown every document in this project is written in, and the size budgets. Those budgets keep documents inside the range where retrieval stays reliable.

## Requirements

### `docs-format:chapter-stays-within-200-lines` — A chapter stays within 200 lines

The author MUST keep a chapter at or below 200 lines.

A chapter is a markdown document directly inside `method/` or `comparison-docs/`, or a file named `glossary.md` or `README.md` anywhere. `AGENTS.md` is never a chapter, because `docs-format:author-instructions-stay-within-budget` owns it.

A catalog chapter takes a 300-line cap instead. The catalog names are `gates.md`, `checklist.md`, `glossary.md`, `README.md`, and `SOURCES.md`.

#### Scenario: A chapter acquires a second subject

- GIVEN a chapter approaching the cap
- WHEN more rules arrive
- THEN the excess becomes a requirement in a spec or a decision record, not a longer chapter

Verify: `pre-commit run chapter-size-cap --all-files`

### `docs-format:author-instructions-stay-within-budget` — Author instructions stay within budget

The author MUST keep the root author-instructions file at or below 100 lines and a subtree one at or below 150.

#### Scenario: A root file accumulates a subtree's rules

- GIVEN a rule that binds one subtree only
- WHEN it is written into the root file and pushes it past the cap
- THEN it belongs in that subtree's own author-instructions file, which the root points at

Verify: `pre-commit run agents-digest-size --all-files`

### `docs-format:every-budget-carries-a-gate` — Every count-shaped budget carries a gate

The project MUST enforce every count-shaped budget with a command that fails the change, and MUST NOT raise a budget to admit an over-budget document.

#### Scenario: A document arrives over its budget

- GIVEN a chapter that will not fit in 200 lines
- WHEN an author reaches for the cap rather than the content
- THEN the chapter splits, because a gate that admits it admits the next one too

Verify: `for h in adr-word-cap agents-digest-size spec-size-cap chapter-size-cap; do grep -q "id: $h$" .pre-commit-config.yaml || exit 1; done`

### `docs-format:document-uses-structural-markdown-only` — A document uses structural markdown only

The author MUST NOT use bold or italic text.

#### Scenario: An agent drafts a rule list

- GIVEN a generated document with a bold lead-in on every item
- WHEN a reader scans it
- THEN nothing is emphasized because everything is, and the identifiers belong in inline code

Verify: reviewer confirms the change introduces no bold or italic text

### `docs-format:document-states-the-present` — A document states the present

The author MUST NOT record what a document used to say, what it replaces, or why something is absent.

The gate reads the markdown under the documentation root, and it skips the decision records there. A record states the reasoning of one moment and is the only document class that carries history.

#### Scenario: A rule is removed

- GIVEN a rule the project drops
- WHEN the author removes it
- THEN the deletion leaves no trace in prose, only in the log

Verify: `pre-commit run no-self-narration --all-files`

### `docs-format:fence-declares-a-language` — Every fence declares a language

The author MUST give every fenced code block a language, using `text` when none applies.

#### Scenario: A diagram is added in a bare fence

- GIVEN an ASCII diagram in a fenced block
- WHEN no language is declared
- THEN the gate rejects it and `text` is the correct declaration

Verify: `pre-commit run markdownlint-cli2 --all-files`

### `docs-format:prose-stays-unwrapped` — Prose stays unwrapped

The author MUST keep each paragraph, list item, and blockquote paragraph on one source line. Only fenced code, tables, and explicit hard breaks span more than one line.

A generated `CHANGELOG.md` is exempt: the rule binds the author, and a release tool writes that file at its own wrap width.

The gate reads the markdown under the documentation root. Prose the project keeps elsewhere belongs to the project, so the rule binds its author and no delivered gate judges it.

#### Scenario: A paragraph is hard-wrapped at a column width

- GIVEN a paragraph broken across source lines at an arbitrary column
- WHEN an edit touches one sentence
- THEN the diff rewraps neighboring lines it never changed, so the paragraph joins back onto one line and the editor soft-wraps it

Verify: `pre-commit run prose-stays-unwrapped --all-files`
