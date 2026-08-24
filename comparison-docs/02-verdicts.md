# 02 — Verdicts

A cell holds one verdict from a fixed vocabulary, written as a symbol and a word. This chapter owns
the vocabulary, why the word is not optional, and the two states that must never be spelled as an
absence.

## The vocabulary

| Verdict       | Meaning                                                               |
| ------------- | --------------------------------------------------------------------- |
| `✅ yes`      | The scenario succeeds on the default path, with no extra setup        |
| `⚠️ partial`   | The scenario succeeds only with configuration, or with a stated limit |
| `❌ no`       | The scenario was run and did not succeed                              |
| `➖ n/a`      | The scenario does not mean anything for this subject                  |
| `🧪 unstable` | The scenario succeeds, on a path the subject calls experimental       |
| `❓ untested` | The scenario has not been run against this subject                    |

- A cell MUST hold exactly one verdict from this table.
- A verdict MUST use the word given here, unchanged.

The fixed word is what makes the column greppable. A reviewer counting how many capabilities are
still untested runs one grep; a column of synonyms defeats that and nothing else was gained.

## Symbol and word, never symbol alone

- A verdict MUST pair its symbol with its word.
- A verdict MUST NOT be a bare symbol.

A symbol alone fails four readers. A screen reader announces the Unicode name assigned by the
consortium, so `✅` reads as "white heavy check mark" and never as "yes"; markdown gives no way to
override that name. A monochrome or e-ink reader loses the green-red distinction, and so does anyone
with a red-green color vision deficiency. A terminal or diff without emoji fonts renders a box. The
Google style guide states the general rule directly: do not present new information in a table
through symbols alone.

The word costs four characters and removes all four failures. The symbol earns its place by making
the column scannable at a glance, which is the whole reason the genre uses a table.

## The legend

- A document MUST carry a legend immediately above its first table.
- The legend MUST list every verdict the document uses, and no others.
- The legend MUST NOT use bold or italics.

One legend serves every table in the document. Repeating it above each one is noise; placing it at
the bottom means the first table is read without it.

```markdown
Legend: ✅ yes, works on the default path · ⚠️ partial, needs configuration or has a stated limit ·
❌ no, was run and failed · ➖ n/a, meaningless for this subject · 🧪 unstable, experimental path ·
❓ untested, not yet run.
```

## Absence is not a verdict

- A cell MUST NOT be empty.
- A cell MUST NOT hold a bare dash, an em dash, or `-` as a verdict.

An empty cell means one of four different things and the reader cannot tell which: not applicable,
not tested, tested and negative, or the author stopped filling the table. `➖ n/a` and `❓ untested`
exist to separate the two honest cases, and they read as deliberate where a blank reads as unfinished.

`❓ untested` is the state authors are most tempted to skip, and it is the one that keeps the document
trustworthy. [05 — Freshness](./05-freshness.md) owns what to do with it.

## Sources

- Google developer documentation style guide, on not conveying information by symbol alone:
  <https://developers.google.com/style/tables>
- W3C technique H86, on text alternatives for emoji and symbols:
  <https://www.w3.org/WAI/WCAG20/Techniques/html/H86>
- Playwright and Astral `ty`, two published feature tables that pair the symbol with a word:
  <https://playwright.dev/docs/test-global-setup-teardown> and
  <https://docs.astral.sh/ty/features/language-server/>
