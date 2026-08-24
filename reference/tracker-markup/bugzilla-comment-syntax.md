# Bugzilla comment syntax

> <https://bugzilla.readthedocs.io/>

A Bugzilla comment is plain text. Nothing is rendered: `**bold**` shows as asterisks, `## Head` shows
as hashes, and a `|` table shows as pipes with its columns wandering. Every convention below has to
work as literal monospace text, and the good ones also survive if the instance happens to have
Bugzilla's optional Markdown mode enabled — an author cannot tell from the outside which one they
are writing into, so nothing may depend on rendering.

## What Bugzilla interprets

- URLs autolink.
- Bug references autolink: `bug NNNNNN`, and on SUSE instances `bsc#NNNNNN`, `boo#NNNNNN`,
  `bnc#NNNNNN`.
- In-bug references autolink: `comment #N`, `attachment #N`.
- A leading `>` marks quoted text and is styled as a quote.

That is the whole list. Everything else is decoration the author supplies and the reader interprets.

## Headings

One level: all-caps over a dash underline of the same length.

```text
CODE PATH
---------
```

The underline is the only decoration that wins both ways. In monospace it is an unmistakable rule
under the title, and under a Markdown renderer `TEXT` plus `---` is a setext H2, so it upgrades
instead of breaking. A `#`-prefixed heading and a `== Wiki ==` heading both degrade into noise.

Reserve `=====` banner rules for copy-paste framing — a summary line to lift into the Summary field,
a do-not-paste field block — so a reader tells operator instructions from report content at a glance.
That distinction is worth more than a second heading level, which is why `===` is not the level-one
rule even though the setext ladder would put it there.

## Sub-headings

Stop at two levels, and make the second a bare title-case line with no underline. The contrast
between `ALL-CAPS` over a rule and title case over nothing is the hierarchy; the plain line reads as
subordinate precisely because it carries no decoration.

Each alternative fails for a concrete reason:

- Another underline character: `~~~` opens a code fence in CommonMark, `***` is a horizontal rule,
  and `^^^` or `...` are conventions nobody shares. Every invented character costs a lookup.
- Indentation: it signals subordination well in monospace, but four spaces is a literal block — the
  device the body already uses for source and log excerpts — so a sub-heading becomes
  indistinguishable from a snippet.
- RFC numbering (`1.`, `1.1`): the only convention that scales past two levels, which is the tell.
  Needing `1.1` means writing a document, not a bug comment.

Depth is what makes plain text unreadable, because no font weight or size carries the levels — only
decoration invented on the spot. Where a section wants substructure, numbered steps or a run-in
lead-in beat a heading; where it genuinely needs three levels, attach a file and point at it.

## Blocks, tables, and lists

- Literal blocks: indent four spaces. Bugzilla preserves the whitespace, and the same indent is a
  code block under a Markdown renderer. This is how source excerpts and log lines are shown.
- Tables: aligned columns under an ASCII rule of dashes and spaces. A `|`-delimited Markdown table
  renders as pipes and misaligns as soon as a cell grows.
- Lists: `*` or `-` with a hanging indent lining the continuation up under the text.
- Emphasis: none. Word order carries it, and an occasional all-caps word is the only emphasis that
  reads as deliberate rather than as leftover markup.
- Wrapping: hard-wrap every line at 79 columns — prose, tables, code and log excerpts, ASCII
  layout, and the summary line itself. A comment renders in a fixed-width box and Bugzilla never
  reflows, so a longer line either wraps where the tracker chooses, taking the aligned table or the
  annotated excerpt with it, or forces horizontal scrolling. Those are exactly the parts that
  cannot survive an arbitrary break. 79 is the width terminals, diff views, and quoted replies
  already agree on, and it leaves room for a `>` quote prefix.

## Enforcing the width

The width is a rule with an id, not a habit:
`known-issues:a-bugzilla-report-body-fits-in-79-columns`, verified by a hook so a project that
adopts the convention adopts its gate with it. Register it where the project keeps its rules,
carrying the verification line that names the hook:

```markdown
### `known-issues:a-bugzilla-report-body-fits-in-79-columns` — A Bugzilla report fits in 79 columns

A record whose `upstream:` names Bugzilla MUST hold its `## Report` body in a fenced block whose
lines are at most 79 columns. A record filed into a tracker that reflows MUST NOT be held to a
width.

Verify: `pre-commit run ki-bugzilla-report-width --all-files`
```

The rule is Bugzilla's, not every tracker's. GitHub and Jira reflow, so a hard width there is
invisible to every reader and enforceable only against the author — a habit wearing a rule id. So
the gate needs a per-tracker discriminator it can read: for a fenced section that is the record's
`upstream:`, and for a standalone body it is the filename, below.

Where the body is a fenced section of a record, the gate reads the fence and nothing else. Body
text outside it fails too: a markdown formatter owns the wrapping of everything unfenced, at a
width it chooses rather than the tracker's.

````sh
awk '
  FNR == 1         { report = 0; fence = 0; bugzilla = 0 }
  /^upstream:.*([Bb]ugzilla|bsc#|boo#|bnc#)/ { bugzilla = 1 }
  /^## Report$/    { report = 1; next }
  !fence && /^## / { report = 0 }
  report && /^```/ { fence = !fence; next }
  !bugzilla        { next }
  report && fence && length($0) > 79 {
    printf "FAIL known-issues:a-bugzilla-report-body-fits-in-79-columns %s:%d: %d columns\n",
      FILENAME, FNR, length($0); bad = 1 }
  report && !fence && $0 != "" {
    printf "FAIL known-issues:a-bugzilla-report-body-fits-in-79-columns %s:%d: body outside a fence\n",
      FILENAME, FNR; bad = 1 }
  END { exit bad }
' "$@"
````

The heading match is guarded by the fence state. A body whose own markup spells a heading with
`##` would otherwise close the section from inside itself, and every line after it would leave the
check silently — the failure a width gate is most likely to have and least likely to show.

Where the body is a standalone file instead, it is the same rule through editorconfig, which needs
no code: no formatter claims `.txt`, which is what leaves the width to a checker that reads it.

Name that file `<stem>.bugzilla.txt`. The compound suffix is the `*.test.js` / `*.spec.ts`
convention: the real extension stays last, so editors, formatters, and `file(1)` still see plain
text, while the segment before it names the role — here the tracker — and gives a glob something
precise to match. That is what makes the per-tracker rule expressible in a config with no
conditionals: a sibling `<stem>.jira.txt` in the same directory is simply not matched. Scoping the
stanza by directory instead would sweep the Jira body in with it.

```ini
# .editorconfig
[*.bugzilla.txt]
max_line_length = 79
```

```yaml
# .pre-commit-config.yaml
- repo: https://github.com/editorconfig-checker/editorconfig-checker
  rev: v3.7.0
  hooks:
    - id: editorconfig-checker
```

Scope the stanza to the Bugzilla bodies rather than setting the width globally: 79 columns is a
property of Bugzilla's rendering box, not of every file in a repository, and a global cap collides
with whatever formatter owns the other file types. Either gate is worth only what a failing run
proves, so exercise a new one against a line of 85 columns before trusting it — and against the
same line in a body of a reflowing tracker, which must pass, or the narrowing is not real.
