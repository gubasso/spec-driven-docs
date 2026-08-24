# 00 — Model

Three artifacts carry a project's durable knowledge, and each answers exactly one question. This
chapter assigns the questions, settles what wins when two artifacts disagree, and states the
ownership rule every other chapter applies to one artifact class.

## The three artifacts

| Artifact           | Answers          | Changes         | Loaded by an agent |
| ------------------ | ---------------- | --------------- | ------------------ |
| `SPEC-<domain>.md` | what is true now | edited in place | always             |
| `ADR-<slug>.md`    | why, at the time | never           | on demand only     |
| code               | exact behavior   | continuously    | as the work needs  |

A spec is the current contract. It is authored to be read by an agent before it acts, and it is
corrected the moment it stops being true.

A decision record is one event in a log. It states why one option was chosen over others at the
moment of choosing, and it is never revised to match a later design.

Code is the last word on behavior. A name, a type, a schema, or a test participates in change; prose
does not.

## Precedence

```text
observed behavior:  code  >  spec  >  ADR
agreed behavior:    spec  >  code  >  ADR
```

When a spec and a decision record disagree, the spec is right and the record is history. A reader
who finds the disagreement MUST NOT edit the record to agree; the record is correct about the past.

When code and a spec disagree, the phase decides which is the defect. While an open unit of work
cites the rule's ID, the spec states the agreement and the code is catching up; otherwise the code
is the observed truth and the spec is stale. Fix the wrong one in the same change, and say in the
commit which one it was. [09 — Spec to Code](./09-spec-to-code.md) owns the marker.

## Rules this produces

- An agent resolving a conflict MUST follow the precedence above and MUST cite the artifact it
  followed.

Two consequences follow from precedence and are owned where they are applied: updating the spec when
behavior changes belongs to [07 — Lifecycle](./07-lifecycle.md), and leaving a record unrevised
belongs to [04 — Decisions](./04-decisions.md).

## The projection is authored, not derived

A spec looks like a projection of the decision log, and it is not one. Replaying every record would
not rebuild it, for two reasons: only significant choices earn a record, and most of what a spec
asserts was never a deliberated decision at all.

This is why precedence runs toward the spec rather than toward the log. In a system where state is
derived, the log wins. Here the spec is written by hand, so it is the source and the log is the
footnote.

The cost is that consistency is not free. Nothing recomputes the spec when a record is added, which
is why the first rule above is a rule and not an observation.

## One owner per durable fact

A durable fact has exactly one home. Every other mention links to it.

Repetition is not a style problem. A fact stated twice drifts, and once two pages disagree a reader
has to choose between them, which sends the reader back to chat logs and guesswork.

The test is disposability. Delete the passage: if the canonical fact is still stated somewhere and
still discoverable, the passage was a summary and may stay. If deleting it removes the only current
statement of the rule, it had become a second owner.

A carrying sentence is allowed. Restating a source's conclusion so the reader understands a claim
before following the link passes the test; copying the source's argument does not.

- A document whose purpose is to point at a source of truth MUST carry only what orients the reader
  and the link.

An issue opened to track a bug filed elsewhere, an index entry, a stub: each exists to route someone
to the owner. What orients them is a sentence or two of what it is about, and where a contract is at
stake, what was expected against what happened. A summary table, an environment block, a mechanism,
or a paragraph of background is a second owner by the test above, and it rots while the target moves.
The reader is one link away.

- A reference MUST carry the claim it relies on rather than only the coordinates of where to find it.

A line number decays silently. Text moves above it, the citation now points at something else, and
nothing anywhere reports the drift. A heading anchor or a rule ID survives that same edit, and the
carrying sentence means a reader who never follows the link still understands the passage.

## Behavior contracts belong in code

Prose is the weakest place to state a contract. Before writing a requirement, ask whether a function
signature, a schema, an enum, or a test can carry it instead. If one can, that is the home, and the
spec points at it.

A spec describes shape, boundaries, and the constraints that hold. It does not list fields, and it is
never the only place a required argument or a status value exists.

Generated material follows the same rule. Generation does not confer ownership: the generator's
source owns the fact, and a reader must be able to tell whether to edit the output, the source, or
neither.

## The filesystem owns its own state

Directory structure, filenames, and what a directory currently contains are facts the listing already
holds. A hand-maintained copy drifts on the next add, rename, or delete, and then competes with the
real structure.

What is forbidden is a shape, not a syntax: an index of a directory, kept because the directory
exists. A table of contents of the tree, a topic-to-filename table, a `source-files` frontmatter
list. Each mirrors the disk and each acquires a regeneration ritual to service its decay.

Naming files in prose is normal. A list, a table, or a full tree stays wherever its entries carry
their own payload: what a directory reserves, what a file specifies, its domain, its scope.

The test is what a change to the tree would cost. Strip the paths out and read what is left. If the
passage still teaches, it was documentation that happened to name files. If nothing survives, it was
the listing.

The heading is the tell, and is worth choosing deliberately. `Source map`, `Contents`, and `Files`
promise an inventory, so a section under one of them is judged as an inventory and generally fails.
`Directory domains` promises something else — what each part of the tree reserves, and what belongs
there — and a tree or table under it passes on the strength of what it says. Name the section after
the knowledge it carries, and the shape follows.

A layout a reader is told to create in their own project is a specification, not a copy, and stays.
A generated table of contents of a document's own headings is not disk state either: it indexes the
document rather than the tree, and belongs to its generator.

## Sources

- OpenSpec, on specs as current behavior and changes as proposals:
  <https://github.com/Fission-AI/OpenSpec/blob/main/docs/concepts.md>
- Python PEP 1, on a resolved PEP being a historical document rather than a living specification:
  <https://peps.python.org/pep-0001/#pep-maintenance>
- AWS, on an accepted decision record becoming immutable:
  <https://docs.aws.amazon.com/prescriptive-guidance/latest/architectural-decision-records/adr-process.html>
