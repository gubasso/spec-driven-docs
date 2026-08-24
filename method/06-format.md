# 06 — Format

Markdown is the house format. This chapter keeps the constructs that carry structure, drops the
inline emphasis that carries decoration, sets the register every document is written in, and owns the
size budget the whole framework is measured against.

## Budgets

| Artifact                         | Budget                                               |
| -------------------------------- | ---------------------------------------------------- |
| Root author-instructions file    | 100 lines                                            |
| Subtree author-instructions file | 150 lines                                            |
| Spec                             | 300 lines excluding the TOC; TOC generated above 100 |
| Requirement statement            | one sentence                                         |
| Decision record                  | 350 words                                            |
| Chapter                          | 200 lines                                            |
| Catalog                          | 300 lines                                            |
| Reference depth                  | one level from the entry document                    |

- A document MUST stay within the budget for its artifact class.
- A project MUST enforce every budget stated as a count with a command that fails the change.
- A project MUST NOT raise a budget to admit a document that exceeds it.
- A gate that exempts a file MUST fail once that file no longer needs the exemption.

Six of the budgets are line or word counts and are gated in [08 — Gates](./08-gates.md). The other
two are not counts: a requirement statement being one sentence, and a reference being one level from
the entry document, are reviewer judgment, and that chapter declares them unenforced rather than
approximating them. A count nobody runs is advice, and a project that publishes these numbers
without hooks has documented an intention.

Adopting a budget over a corpus written before it is the case that tempts the raised number. Gate
the scope already inside the budget and list the files outside it, so what is uncovered is an
enumeration a reader can count rather than a silence. The list must fail the gate once a file it
names fits, or it stops shrinking the day it stops being read.

The numbers are not arbitrary and they are not measured constants either. Retrieval accuracy falls as
input length grows, non-uniformly and on tasks as simple as finding one sentence, so a cap is the
crude instrument that keeps documents in the range where retrieval is reliable. Where a first-party
figure exists it anchors the class: an instruction file read in full is held under a few hundred
lines, and a file over a hundred lines is assumed to be read in part.

When a chapter wants more room, the excess is a requirement in a spec or a decision record, not a
longer chapter.

A catalog is the exception, and gets the spec's number for the same reason a spec does: its length is
a function of how many rules exist, one entry apiece, so the chapter cap would punish the corpus for
growing rather than punish an author for rambling. The gate list, the checklist, the glossary, and a
shelf index are catalogs. What a catalog owes is one entry per rule and no argument; an argument in a
catalog belongs to the chapter that owns the rule.

## Structural markdown is kept

- Headings, which carry the document's shape and are what retrieval keys on.
- Ordered and unordered lists, for genuinely parallel items or sequences.
- Tables, for comparisons and exact mappings.
- Fenced code blocks, always with a language; use `text` when none applies.
- Inline code, for identifiers, paths, flags, commands, keywords, and status values.
- Links.
- Blockquotes, for quoted material only.

## Decorative markdown is dropped

- A document MUST mark identifiers with inline code, normativity with RFC 2119 keywords, and
  structure with headings.
- A document MUST NOT contain bold or italic text.

Emphasis was doing three jobs and each has a better home. An identifier becomes inline code. A binding
statement becomes a MUST. A phrase tempting to bold as a mini-heading becomes a heading, and if that
produces too many headings the section was carrying more than one idea.

The rule is zero, not sparingly. A density budget invites a judgment call on every line and loses it
slowly, so zero is the only version applied consistently.

It binds authoring and is held by review rather than by a gate. A project adopting it inherits a
corpus written before it, and the exemption list that would clear that corpus costs more than the
rule is worth: emphasis markers are one to two percent of a corpus, and removing one changes nothing
a reader can act on. [08 — Gates](./08-gates.md) records it as unenforced, which is the honest form,
and [07 — Lifecycle](./07-lifecycle.md) owns the choice between the two.

Do not justify this with token savings. The dominant costs are restating a fact another document owns
and loading documents the work does not need, and [00 — Model](./00-model.md) and
[05 — Agent Context](./05-agent-context.md) fix those.

## Register

The framework's documents are read by someone about to act, not by someone studying.

- A document MUST put its most consequential content first.
- A document MUST spend prose only on a decision, a hazard, or a non-obvious constraint.
- A command MUST appear in a fence, not inside a sentence.
- A document MUST use numbered steps only where order is load-bearing.

Two habits produce documents that fail this, and both look like helpfulness.

The first is narration: a sentence introducing the step, the command, then a sentence describing what
just happened. The introduction restates the heading and the description restates the command's own
output. Neither survives deletion.

The second is inlined rationale: the concept behind a rule explained at the rule. It reads as thorough
and it is a placement failure. The explanation belongs to whoever owns that fact, and this document
links it.

The test: strip the prose out and read what is left. If the remaining rules are still executable, the
prose was earning its place. If the document no longer makes sense, the rules were incomplete and the
prose was carrying them.

A rule that reads as arbitrary is the case where prose is required. Two sentences saying why a
counterintuitive rule exists prevent the next author from deleting it as pointless.

## A report leads with the run, not the detail

- A report MUST place a walkthrough of one concrete run directly after its summary.
- A report MUST state expected against actual before it presents supporting detail.

A reader who has not seen the defect cannot judge the evidence for it. Summary, walkthrough, expected
against actual, and only then environment, reproducibility, code, and captures gives them the story
first and the proof second, so the excerpt they reach later reads as confirmation rather than as
homework. A body opening with an environment table and a list of locations asks the reader to
assemble the story themselves, and most stop before they do.

This binds the copy filed into a tracker as much as the record it came from: the same order, in the
tracker's markup ([Bugzilla](../reference/tracker-markup/bugzilla-comment-syntax.md),
[Jira](../reference/tracker-markup/jira-wiki-markup.md)).

## A claim about code shows the code

- A document making a claim about what code does MUST quote the span that shows it.
- A quoted span MUST open with a header naming its path, its revision, and its start line.
- A document MUST NOT support such a claim with a location citation alone.

A reader who has to open a checkout and count lines to reach `file.py:127` does not check the claim.
They take it on trust or they stop reading, and a report whose evidence nobody opens is a report
without evidence. Quoting the deciding lines is what turns a citation into one, and the header kept
above the quote is what lets a reader find the span and confirm it is still there.

Quote the gist, not the region: the smallest span where the mechanism is visible, the parts that
carry nothing elided, and the line that arms the defect and the line that fires it annotated in the
margin. Annotate meaning, not position, because one header line locates the span and repeating a line
number against each line adds noise a reader does not need. The header owns the location, so the
prose above the fence does not restate it. A one-line change is shown as a diff hunk rather than
described.

Two habits fail the rule. The first is a list of line ranges under a heading like `Code path`, which
reads as thorough and hands the reading back to the reader. The second is a fence marked with the
source language, which invites a formatter to rewrite quotes, wrapping, and indentation; a quoted
span is fenced as `text`, because a reformatted excerpt is no longer the thing that was quoted.

[08 — Gates](./08-gates.md) records it as unenforced: whether a span shows the mechanism requires
reading the code beside it.

## One default, not a survey

- A document MUST recommend one option and MAY name one escape hatch.
- A document MUST NOT compare alternatives it does not recommend.

A list of four approaches leaves the reader to choose and leaves an agent to pick arbitrarily. Name
the default, name the one case that justifies departing from it, and stop. The comparison that
produced the default belongs in the decision record.

## No document narrates its own history

- A document MUST state what is true now.
- A document MUST NOT record what it used to say, what it replaces, or why something is absent.

Do not contrast current text with an earlier state or explain an absence. A reader arriving today
needs only the state they can act on.

The test: delete the clause. If nothing a reader can act on disappeared, it was archaeology.

Decision records are the exemption, because holding history is their entire job. Git holds the rest,
and holds it better. This binds the change that removes something too: a deletion leaves no trace in
prose, only in the log.

## Consistent terminology

- A project MUST use one term for one concept across every document.

Mixing spec, specification, contract, and requirements doc for one artifact weakens retrieval for all
four. Pick one, put it in the glossary, and use it.

## Mechanics

- One `#` per file. `##` for sections, `###` only for genuinely parallel sub-parts.
- An unheaded opening states what the file owns, in two or three sentences.
- Hand-wrap at 100 columns.
- Relative links carry an explicit prefix: `./name.md`, `../dir/name.md`, or a multi-segment path.
- `<angle>` placeholders stand for project-specific values.
- Cite a source inline at the claim it supports; collect them in a `## Sources` section when a
  document carries three or more.
- No `## See also` section; an index and inline links carry navigation.

## Sources

- Chroma, on degradation with input length and on position sensitivity:
  <https://www.trychroma.com/research/context-rot>
- He et al., on prompt format changing task accuracy substantially for weaker models:
  <https://arxiv.org/abs/2411.10541>
- Anthropic, on conciseness, consistent terminology, and avoiding a survey of options:
  <https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices>
