# Jira wiki markup

> <https://jira.atlassian.com/secure/WikiRendererHelpAction.jspa?section=all>

Jira renders its own wiki markup, so unlike [Bugzilla](./bugzilla-comment-syntax.md) real headings
and emphasis are available and should be used. Markdown is not: `**bold**` and `## Head` come out
literal.

## The markup that matters

| Need           | Markup                                           |
| -------------- | ------------------------------------------------ |
| Heading        | `h1.` … `h6.` at line start, then a space        |
| Bold, italic   | `*bold*`, `_italic_`                             |
| Inline literal | `{{monospace}}`                                  |
| Block literal  | `{noformat}` … `{noformat}`, or `{code}`         |
| Table          | `\|\|header\|\|header\|\|` then `\|cell\|cell\|` |
| Link           | `[text\|url]`                                    |
| Lists          | `*` bulleted, `#` numbered, nest by repeating    |

## Headings

Start at `h2.` inside a description. An `h1.` competes visually with the issue summary, which is
already the page's title. Nesting is cheap here because the renderer supplies real weight and size,
so the plain-text ban on depth does not apply.

It rarely matters: a ticket that exists to point at a source of truth carries a short summary,
expected against actual, and a link, and that needs no headings at all. Reach for `h2.` once a
ticket holds sections a reader would otherwise scroll past.

## Wrapping

There is no hard width here, and the 79-column rule described in
[Bugzilla comment syntax](./bugzilla-comment-syntax.md) deliberately does not reach a Jira body —
which is why a prepared body is named `<stem>.jira.txt` and the gate matches `*.bugzilla.txt`. Jira
reflows prose to the panel, so a fixed width is invisible to every reader and enforceable only
against the author.

What does not reflow is `{noformat}` and `{code}` blocks and `||` tables. Wrap those by hand,
narrow enough to read in a comment panel. That is authoring judgement per block, not a file-wide
column a hook can check.

## Traps

- A `{` starts a macro. Text that must show a brace goes inside `{{...}}` or a `{noformat}` block.
- The Cloud rich-text editor converts wiki markup on paste and cannot always be talked out of it.
  Paste into the plain-text markup mode where the instance offers one, then read the preview. This
  is why a prepared ticket body is stored as `.txt` and verified after pasting rather than trusted.
- A `|` inside a table cell breaks the row; escape it as `\|` or reword.
