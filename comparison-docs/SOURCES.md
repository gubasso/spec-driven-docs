# Sources

The research this shelf rests on, with what each source establishes and the date its URL was confirmed to resolve. A rule in a chapter that looks arbitrary is usually one of these. Re-verify here before relaxing it.

## Verification policy

"Confirmed" means the URL was fetched and returned the stated page on that date. A source is cited for the one claim named beside it, not as general endorsement. Re-confirm by fetching the URL. Only treat a citation as broken when it fails to resolve or the resolved page contradicts the claim.

A URL that redirects is stale even though it resolves. Record the address the page lives at now, and prefer the address the publisher calls current over one that declares itself superseded.

## Renderer constraints

| Source                                 | Establishes                                                                                                                                                                                                                                | Confirmed  |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------- |
| GitHub Flavored Markdown specification | A table cell holds inline content only: "Cells containing arbitrary text, in which inlines are parsed". "Block-level elements cannot be inserted in a table." A pipe inside a cell must be escaped, "including inside other inline spans". | 2026-09-12 |
| GitHub footnotes changelog             | GitHub added footnote syntax to every markdown field on 2021-09-30, rendering each one as a superscript link to a section at the foot of the document                                                                                      | 2026-09-12 |
| mdBook markdown reference              | Tables, footnotes, relative `.md` links, and `#heading` fragments all render. The page lists tables and footnotes as "extensions beyond the standard CommonMark specification", and documents the two link behaviors under Links.          | 2026-09-12 |
| Material for MkDocs, footnotes         | `[^id]` footnotes come from the Python Markdown `footnotes` extension, which `mkdocs.yml` must enable. The hoverable tooltip is a separate theme feature, `content.footnote.tooltips`.                                                     | 2026-09-12 |
| Material for MkDocs, data tables       | Cells "can contain arbitrary Markdown, including inline code blocks, as well as icons and emojis"                                                                                                                                          | 2026-09-12 |

The first row is why every qualification leaves the table by a link. The next four are why that link is an ordinary heading anchor rather than a footnote: anchors render identically everywhere, footnotes are per-renderer.

- <https://github.github.com/gfm/>
- <https://github.blog/changelog/2021-09-30-footnotes-now-supported-in-markdown-fields/>
- <https://rust-lang.github.io/mdBook/format/markdown.html>
- <https://squidfunk.github.io/mkdocs-material/reference/footnotes/>
- <https://squidfunk.github.io/mkdocs-material/reference/data-tables/>

## Accessibility

| Source                                             | Establishes                                                                                                                                                                                                                  | Confirmed  |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Google developer documentation style guide, Tables | "Don't present new information in tables through images or symbols alone." A table column heading uses sentence case. An author introduces a table with a complete sentence, because not all screen readers preannounce one. | 2026-09-12 |
| W3C technique H86, WCAG 2.2                        | "Emojis come with their own pre-defined names that might not always match what the author is intending to communicate." An emoji, an emoticon, ASCII art, and leetspeak each need a text alternative.                        | 2026-09-12 |
| Unicode emoji chart, CLDR short names              | The CLDR short name for `✅`, U+2705, is "check mark button"                                                                                                                                                                 | 2026-09-12 |
| W3C WAI tables tutorial, tips                      | Break a complex table into simple tables, one per subtopic. Start a new table when the topic changes. A responsive transformation must preserve every header-to-data relationship.                                           | 2026-09-12 |

An emoji carries a predefined name, and plain markdown offers no way to override it. That name comes from the character data rather than from the author, so it does not state the verdict. The CLDR short name for `✅` is "check mark button". What a given screen reader speaks depends on the assistive technology and the locale, and the shelf cites no test of that. That is the specific reason the shelf requires the word, not a general preference.

H86 is cited at its WCAG 2.2 address. The WCAG 2.0 technique of that number carries a different title, covers no emoji, and states that it is no longer maintained.

- <https://developers.google.com/style/tables>
- <https://www.w3.org/WAI/WCAG22/Techniques/html/H86>
- <https://unicode.org/emoji/charts/full-emoji-list.html>
- <https://www.w3.org/WAI/tutorials/tables/tips/>

## Width on a small screen

| Source                              | Establishes                                                                                                                                                                                                                                | Confirmed  |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------- |
| Nielsen Norman Group, mobile tables | For complex or wordy entries, roughly two columns fit legibly on a narrow phone. A comparison table is the article's example. The remedies are sticky headers and columns, a selective view, an accordion, and a visible scroll indicator. | 2026-09-12 |
| UXmatters, designing mobile tables  | Sideways scrolling forces an oversized element into a mobile screen, and dual-axis scrolling is confusing. Converting a table into a vertical list breaks every table interaction.                                                         | 2026-09-12 |

No standards body publishes a column limit, and the shelf's five-column rule is an operational default rather than a cited constant. What is cited is that the failure arrives early and that splitting is the accessible remedy.

- <https://www.nngroup.com/articles/mobile-tables/>
- <https://www.uxmatters.com/mt/archives/2020/07/designing-mobile-tables.php>

## Design precedents

| Source                                      | Establishes                                                                                                                                                                                                                             | Confirmed  |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| MDN browser-compat-data schema              | `notes` on a support statement is "A string or `array` of strings". A cell holds one support statement, an array of two or more, or `"mirror"`. A preference to enable is a separate `flags` entry, typed `preference`, and not a note. | 2026-09-12 |
| MDN browser-compat-data, `api/Element.json` | A note carries what a symbol cannot: "Fully supported on Windows and Linux, no support on ChromeOS."                                                                                                                                    | 2026-09-12 |
| caniuse contributing guide                  | A cell is a support state, and a numbered note marker can follow it: `"42": "y #1"` means supported by default, see note 1, with `notes_by_num` holding the note text. A note is "often to explain what partial support refers to".     | 2026-09-12 |
| Astral `ty` language-server features        | `✅ Supported`, `❌ Not supported`, and `—`, with a trailing notes column that carries issue links. Some cells link out.                                                                                                                | 2026-09-12 |
| Playwright documentation                    | A compact table pairing the symbol with a word rather than using the symbol alone: `✅ Yes`, `✅ Fully supported`, `❌ Not shown`                                                                                                       | 2026-09-12 |
| EMQX feature comparison                     | One large comparison split into fifteen thematic tables, three subject columns each, with a "Notes and Links" column on every table but one                                                                                             | 2026-09-12 |
| Docusaurus markdown features                | A GFM table using `✅ Yes` rather than an unexplained emoji                                                                                                                                                                             | 2026-09-12 |
| microvm.nix hypervisor table                | An in-domain counter-example: no symbols at all, three columns, with the limitation stated as short prose in a Restrictions column                                                                                                      | 2026-09-12 |

No source here supplies the one-reference-per-cell rule. That rule rests on the shelf's own scannability argument, and [References](./references.md) states it. MDN and caniuse both establish the note channel and what it carries. caniuse ties the note to the partial state, and the further claim that positive and negative states rarely need one is the shelf's own. `ty` and EMQX are the notes-column pattern the shelf keeps as an escape hatch for narrow comparisons.

- <https://github.com/mdn/browser-compat-data/blob/main/schemas/compat-data-schema.md>
- <https://github.com/mdn/browser-compat-data/blob/main/api/Element.json>
- <https://github.com/Fyrd/caniuse/blob/main/CONTRIBUTING.md>
- <https://docs.astral.sh/ty/features/language-server/>
- <https://playwright.dev/docs/test-global-setup-teardown>
- <https://docs.emqx.com/en/emqx/latest/getting-started/feature-comparison.html>
- <https://docusaurus.io/docs/next/markdown-features>
- <https://github.com/microvm-nix/microvm.nix>

## Re-verify

```bash
# Confirm every cited URL still resolves, and print where it lands. Anything outside
# 200..=206, 401, 403, 429 needs a look, and so does a landing address that differs from the cited one.
grep -ohP 'https?://[^>) ]+' comparison-docs/SOURCES.md | sort -u \
  | while read -r u; do printf '%s <- %s\n' "$(curl -s -o /dev/null -w '%{http_code} %{url_effective}' -L --max-time 20 "$u")" "$u"; done
```
