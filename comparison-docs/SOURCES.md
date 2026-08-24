# Sources

The research this shelf rests on, with what each source establishes and the date its URL was
confirmed to resolve. A rule in a chapter that looks arbitrary is usually one of these; re-verify
here before relaxing it.

## Verification policy

"Confirmed" means the URL was fetched and returned the stated page on that date. A source is cited for
the one claim named beside it, not as general endorsement. Re-confirm by fetching the URL; only treat
a citation as broken when it fails to resolve or the resolved page contradicts the claim.

## Renderer constraints

| Source                                 | Establishes                                                                                                                                                                                                                          | Confirmed  |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------- |
| GitHub Flavored Markdown specification | A table cell holds inline content only: "Cells containing arbitrary text, in which inlines are parsed". "Block-level elements cannot be inserted in a table." A pipe inside a cell must be escaped, including inside an inline span. | 2026-08-18 |
| GitHub footnotes changelog             | Footnotes are a 2021 extension available in markdown fields, outside the core tables specification                                                                                                                                   | 2026-08-18 |
| mdBook markdown reference              | Tables, footnotes, relative `.md` links, and `#heading` fragments all render                                                                                                                                                         | 2026-08-18 |
| Material for MkDocs, footnotes         | `[^id]` footnotes with optional hoverable tooltips, as a theme extension                                                                                                                                                             | 2026-08-18 |
| Material for MkDocs, data tables       | Cells accept arbitrary inline markdown including links, icons, and emoji                                                                                                                                                             | 2026-08-18 |

The first row is why every qualification leaves the table by a link. The next four are why that link
is an ordinary heading anchor rather than a footnote: anchors render identically everywhere, footnotes
are per-renderer.

- <https://github.github.com/gfm/>
- <https://github.blog/changelog/2021-09-30-footnotes-now-supported-in-markdown-fields/>
- <https://rust-lang.github.io/mdBook/format/markdown.html>
- <https://squidfunk.github.io/mkdocs-material/reference/footnotes/>
- <https://squidfunk.github.io/mkdocs-material/reference/data-tables/>

## Accessibility

| Source                                             | Establishes                                                                                                                                                                                             | Confirmed  |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Google developer documentation style guide, Tables | "Don't present new information in tables through images or symbols alone." Also: sentence-case headings, and introduce a table with a complete sentence because not all screen readers preannounce one. | 2026-08-18 |
| W3C technique H86                                  | Emoji, emoticons, and symbols require a text alternative serving an equivalent purpose                                                                                                                  | 2026-08-18 |
| W3C WAI tables tutorial, tips                      | Break a complex table into simple tables, one per subtopic; start a new table when the topic changes; responsive transformations must preserve header-to-data relationships                             | 2026-08-18 |

An emoji's accessible name comes from the Unicode consortium and cannot be overridden in plain
markdown, so `✅` is announced as "white heavy check mark" rather than as the verdict the author meant.
That is the specific reason the shelf requires the word, not a general preference.

- <https://developers.google.com/style/tables>
- <https://www.w3.org/WAI/WCAG20/Techniques/html/H86>
- <https://www.w3.org/WAI/tutorials/tables/tips/>

## Width on a small screen

| Source                              | Establishes                                                                                                                                                                  | Confirmed  |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Nielsen Norman Group, mobile tables | For complex or wordy entries such as comparison tables, roughly two columns fit legibly on a narrow phone; the options are vertical stacking, horizontal scroll, or a toggle | 2026-08-18 |
| UXmatters, designing mobile tables  | A comparison table may need horizontal scrolling to preserve column relationships, where a content table can become a vertical list without losing meaning                   | 2026-08-18 |

No standards body publishes a column limit, and the shelf's five-column rule is an operational
default rather than a cited constant. What is cited is that the failure arrives early and that
splitting is the accessible remedy.

- <https://www.nngroup.com/articles/mobile-tables/>
- <https://www.uxmatters.com/mt/archives/2020/07/designing-mobile-tables.php>

## Design precedents

| Source                               | Establishes                                                                                                                                                                                           | Confirmed  |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| MDN browser compatibility tables     | One footnote per compatibility cell, because the compatibility API supports one note per support assertion; notes carry what a symbol cannot, such as a preference to enable or a platform limitation | 2026-08-18 |
| caniuse contributing guide           | A cell is a support state plus a numbered note marker: `"42": "y #1"` means supported by default, see note 1, with `notes_by_num` holding the catch. The note channel exists for the partial state.   | 2026-08-18 |
| Astral `ty` language-server features | `✅ Supported`, `❌ Not supported`, and `—`, with a trailing notes column carrying issue links; some cells link out                                                                                   | 2026-08-18 |
| Playwright documentation             | A compact table pairing the symbol with a word rather than using the symbol alone                                                                                                                     | 2026-08-18 |
| EMQX feature comparison              | One large comparison split into eleven-plus thematic tables, three subject columns each, plus a "Notes and Links" column                                                                              | 2026-08-18 |
| Docusaurus markdown features         | GFM tables using `✅ Yes` rather than an unexplained emoji                                                                                                                                            | 2026-08-18 |
| microvm.nix hypervisor table         | An in-domain counter-example: no symbols at all, three columns, with the limitation stated as short prose in a Restrictions column                                                                    | 2026-08-18 |

MDN supplies the one-reference-per-cell rule. caniuse supplies the insight that the middle state is
what the note channel is for, and that positive and negative states rarely need one. `ty` and EMQX are
the notes-column pattern the shelf keeps as an escape hatch for narrow comparisons.

- <https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Page_structures/Compatibility_tables>
- <https://github.com/Fyrd/caniuse/blob/main/CONTRIBUTING.md>
- <https://docs.astral.sh/ty/features/language-server/>
- <https://playwright.dev/docs/test-global-setup-teardown>
- <https://docs.emqx.com/en/emqx/latest/getting-started/feature-comparison.html>
- <https://docusaurus.io/docs/next/markdown-features>
- <https://github.com/astro/microvm.nix>

## Re-verify

```bash
# Confirm every cited URL still resolves. Anything outside 200..=206, 401, 403, 429 needs a look.
grep -ohP 'https?://[^>) ]+' comparison-docs/SOURCES.md | sort -u \
  | while read -r u; do printf '%s %s\n' "$(curl -s -o /dev/null -w '%{http_code}' -L --max-time 20 "$u")" "$u"; done
```
