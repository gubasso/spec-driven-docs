# Keep prose lines unwrapped

## Context and Problem Statement

The corpus was hand-wrapped at 100 columns. A hard-wrapped paragraph makes every small edit reflow its neighbors, so diffs show lines nobody changed, `grep` misses phrases split across a break, and agents quoting a rule get a fragment. The wrap column is also a per-editor preference frozen into every file, enforced by nothing.

## Considered Options

- `Unwrap prose: one source line per paragraph or list item, soft-wrapped by the reader's editor` — chosen.
- `Keep hand-wrapping at 100 columns` — rejected: reflow noise in diffs, broken phrase search, and no gate ever enforced the column.
- `Reflow automatically at a fixed width on commit` — rejected: dprint `textWrap: always` normalizes the noise but still splits phrases across lines and rewrites untouched neighbors.

## Decision Outcome

Chosen option: `Unwrap prose` — the source line becomes the semantic unit, so a diff touches only the paragraph that changed and a phrase is always greppable on one line.

Enforced by `docs-format:prose-stays-unwrapped`.

## Consequences

- Good: diffs, blame, and search operate on whole paragraphs; the delivered gate carries the rule to every instance.
- Good: dprint (`textWrap: never`) joins wrapped prose mechanically, so adoption over an existing corpus is one format run.
- Bad: long lines depend on editor soft-wrap, and side-by-side diff views get harder to read without word-level highlighting.
- Bad: fenced code, tables, and hard breaks stay multi-line, so the gate must classify blocks and can misread exotic markdown.

## Status

Implemented — `sdd gate prose-stays-unwrapped`, wired in `.pre-commit-config.yaml` and delivered via the gate registry.
