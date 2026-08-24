# 01 — Table shape

The matrix is scanned, not read. This chapter fixes its width, how a wide comparison is split, and
how a row is phrased so the verdict in it is a fact rather than a label.

## Width

- A table MUST hold at most five columns: the capability label, the subject, and three alternatives.
- A table MAY hold six when every column heading and every verdict is at most twelve characters.
- A comparison needing more subjects MUST be split by theme, never widened.

No standards body publishes a column limit; the constraint is what a reader can hold. For wordy
comparison entries roughly two columns fit legibly on a narrow phone, so horizontal scrolling is
already the normal case at five and the question is only how far it goes. Splitting is also the
accessibility answer: the W3C tables tutorial says to break a complex table into simple ones, one per
subtopic, and to start a new table when the topic changes.

Check the result at 320 CSS pixels and at 200 percent zoom. That observation settles it; a column
count only predicts it.

## Splitting by theme

- A section heading MUST name the question its table answers.
- A subject MUST NOT appear in a table whose theme it does not compete in.

Group by the question the reader arrived with, not by the taxonomy of the subjects. A reader deciding
about isolation does not want provisioning rows interleaved, and the subject list for the two
questions is rarely the same set.

```text
## Isolation            subject, three isolation tools
## Provisioning         subject, three environment managers
## Day-to-day speed     subject, the two it is actually slower than
```

## Rows

- A row label MUST be phrased so its verdict is an observable behavior.
- A row label SHOULD be at most six words.
- A row MUST hold one capability.

Prefer a label that names what happens over one that names a feature area. "Survives a fork bomb"
has an answer that can be run; "Resource limits" has an answer that can only be asserted. When a
label needs a conjunction it is two rows.

## Columns

- A column heading MUST name one subject.
- Two subjects MUST NOT share a column unless every verdict in that table is identical for both.
- A column heading that is an identifier MUST be inline code.

A merged heading is a convenience that turns into a lie in the first row where the two diverge, and
the row that exposes it is usually the interesting one.

## Mechanics

- A pipe inside a cell MUST be escaped as `\|`, including inside a code span.
- Column headings MUST use sentence case and carry no terminal punctuation.
- A table MUST be introduced by a complete sentence before it.

The pipe rule is the one with teeth. An unescaped pipe inside a code span splits the row, and a
formatter run afterwards re-pads the broken table down to the header's column count, destroying the
evidence before a column-count linter can see it. [06 — Gates](./06-gates.md) wires the check ahead
of the formatter.

The introductory sentence is an accessibility requirement, not a stylistic one: not all screen
readers preannounce a table, so a reader arriving by audio needs the sentence to know one is coming.

## Sources

- W3C WAI, on splitting complex tables by subtopic and preserving header relationships:
  <https://www.w3.org/WAI/tutorials/tables/tips/>
- Nielsen Norman Group, on how few wordy comparison columns survive a narrow screen:
  <https://www.nngroup.com/articles/mobile-tables/>
- Google developer documentation style guide, on headings, sentence case, and introducing a table:
  <https://developers.google.com/style/tables>
