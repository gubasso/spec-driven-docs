# 02 — Specs

A spec states what is true now for one domain, in rules an agent can apply and a command can check.
It is the artifact loaded before work starts and the artifact that wins when two documents disagree.
This chapter owns the container; [03 — Rules](./03-rules.md) owns the requirement blocks inside it.

## Shape

Every spec has the same four-level shape. The shape is a contract: tools and sessions assume the
headings are where this says they are.

```markdown
# <Domain> Specification

## Purpose

<One paragraph: what this domain is, and what the spec binds.>

## Requirements

### `<spec-slug>:<rule-slug>` — <Behavior title>

<One sentence in an EARS pattern with an RFC 2119 keyword.>

#### Scenario: <Name>

- GIVEN <initial state>
- WHEN <action>
- THEN <expected outcome>

Verify: `<command that exits non-zero on violation>`
```

- A spec MUST use the headings `## Purpose` and `## Requirements`, in that order.
- Every requirement MUST be an `### \`<id>\` — <title>`heading under`## Requirements`.
- A spec MUST NOT introduce a section outside this shape.

[08 — Gates](./08-gates.md) wires the check that holds it.

## Purpose

One paragraph. What the domain covers, and where its boundary runs against neighbouring domains.

The paragraph exists so a reader loading the wrong spec finds out in one sentence. It is not an
introduction to the subject and it carries no rules.

## Requirements

The body is an ordered list of requirement blocks and nothing else. No narrative between them, no
rationale, no history of how a rule came to be. A rule that needs an argument has a decision record,
and the record names the rule; the spec states the rule and stops. The single exception is the
clarification marker, owned by [03 — Rules](./03-rules.md).

Order matters. Put the requirement most likely to be violated first: retrieval accuracy is highest
near the start of a document, and a rule buried at line 240 is a rule an agent may not act on.

- Requirements MUST be ordered with the most consequential first.
- A requirement MUST NOT be separated from the next by anything but a clarification marker.

## One spec per domain

- A domain MUST have at most one spec.
- A spec MUST cover exactly one domain.

When a spec grows past its budget, the usual cause is two domains sharing a file. Split by capability,
not by size: a split that leaves a reader loading both halves has not helped.

## Size

| Limit                 | Value                        |
| --------------------- | ---------------------------- |
| Spec length           | 300 lines, excluding the TOC |
| Table of contents     | generated above 100 lines    |
| Requirement statement | one sentence                 |
| Scenario              | four bullets or fewer        |

A file longer than 100 lines may be read in part rather than whole, and a partial read starting at the
top must still show everything the spec covers. That is what the table of contents buys.

The TOC is excluded from the length because the cap bounds authored content and a generated index is
not authored. Counting it would let one added requirement push a spec over the cap through index
growth rather than content growth.
[06 — Format](./06-format.md) owns the full budget table and the reasoning behind the numbers.

## Companion artifacts

A spec is one file. Where it needs supporting artifacts, they go in a directory beside it carrying the
same name.

```text
_docs/specs/
├── SPEC-auth.md
├── SPEC-plan-record.md
└── SPEC-plan-record/
    ├── lane.schema.json
    └── config.schema.json
```

- The author MUST place a spec's supporting artifacts in `<root>/specs/SPEC-<domain>/`.
- The author MUST create that directory only when a requirement names an artifact inside it.

The directory is not a second home for documentation. The test is whether the artifact has a reader
who arrives without the spec: a pricing table, a diagnostics matrix, or a field list has one, and
belongs in reference. A JSON Schema a verification command runs against, a fixture that command
consumes, or a golden file it compares to has no independent reader and no reader-need zone at all.
Those are what the directory is for.

An artifact a reader is expected to copy and run is neither. It ships beside whatever explains it, as
product content rather than metadata about the project.

The companion directory is not loaded with the spec. A requirement names the artifact it depends on,
usually in its `Verify:` line, and the file is read when that command runs.

## A spec is corrected in place

The spec describes the present, so it is edited whenever the present changes.

- A retired requirement MUST be deleted, not struck through or marked obsolete.

Deleting is safe here because the record of the change is elsewhere. The commit holds the diff and a
decision record holds the reasoning, so a spec that keeps its own history is keeping a third copy.

State the change, not the unchanged. A change description names what was added, modified, or removed,
and does not restate the requirements it left alone.

## The reference runs one way

- A spec MUST NOT link or name a decision record.

A record names the rule ID it enforces and the spec never names the record, so the log sits off the
traversal path: an agent walking from the author-instructions file to a spec and on to what the spec
names never arrives at a record. That is what makes an unbounded log free.

Asking the reader not to load the log is the weaker half. A reader must resist the link on every
session; an author removes it once.

Rationale is found by searching, `rg -F '<domain-slug>:<rule-slug>' <root>/decisions`, which returns
every record that argued for the rule. A forward link could name only one and would go stale as soon
as a second landed.

A rule that would otherwise read as arbitrary gets two sentences of rationale here, where they are
already loaded. One needing more than that is underspecified, not under-linked.

## When a spec exists

Write a spec for a domain when both hold:

- The domain has at least one rule that an agent or reviewer must apply.
- At least one of those rules names the command or the named file that will check it.

Do not create a spec in advance of its rules. An empty spec is loaded on every session, costs context
on every session, and teaches nothing on any of them.

A domain with exactly one rule still gets a spec. The overhead is four headings.

## What a spec is not

- Not a tutorial. A reader learning the subject reads explanation; a spec is read by someone about
  to act.
- Not a field list. Exact values belong in reference and exact behavior belongs in code; see
  [00 — Model](./00-model.md).
- Not a decision record. A spec states the rule; the record that argued for it names the rule ID.
- Not a plan. What the project builds next is perishable and lives in the plan zone.

## Worked example

A project whose commit subjects must carry a type prefix writes one requirement:

```markdown
# Commit Messages Specification

## Purpose

Rules governing commit messages in this repository. Covers the subject line. Branch naming and merge
policy are covered by the contribution guide.

## Requirements

### `commit-messages:subject-carries-a-type-prefix` — A commit subject carries a type prefix

When an author writes a commit, the author MUST begin the subject with `<type>(<scope>):`.

#### Scenario: A change touches two areas

- GIVEN a commit spanning two scopes
- WHEN the author cannot name a single scope
- THEN the commit is split, because one commit carries one scope

Verify: `git log -1 --format=%s | rg -q '^[a-z]+(\(.+\))?: ' || exit 1`
```

That is a complete spec: one domain, one rule, and a command that decides it.

## Sources

- OpenSpec, for the `## Purpose` / `### Requirement:` / `#### Scenario:` shape:
  <https://github.com/Fission-AI/OpenSpec/blob/main/docs/concepts.md>
- Chroma, on retrieval accuracy falling with document length and being highest near the start:
  <https://www.trychroma.com/research/context-rot>
