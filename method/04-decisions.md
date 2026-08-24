# 04 — Decisions

A decision record states why one option was chosen over others, at the moment of choosing. It is one
entry in an append-only log. It is not loaded by default, it is never rewritten, and it never claims
to describe the present. This chapter owns the record; [02 — Specs](./02-specs.md) owns what binds
today.

## Filenames

```text
ADR-<slug>.md
```

- A decision record MUST be named `ADR-<slug>.md`, whose slug names the choice.
- A decision record filename MUST NOT contain a digit.
- Once merged, a decision record filename MUST NOT change.

Examples: `ADR-use-postgresql-for-primary-storage.md`, `ADR-a-comment-cites-the-rule-not-the-record.md`.

The slug names the choice in whatever voice states it plainly. An imperative fits a record that picks
a tool; a record that fixes a rule reads better as the rule itself, and forcing either into the
other's voice costs the reader the sentence the filename was carrying.

The `ADR-` prefix makes a record identifiable from a directory listing, so a glob, a grep, or an agent
separates decisions from templates and indexes without opening a file.

The slug is the identifier. Cite it bare in prose: `ADR-use-postgresql-for-primary-storage`.

A slug survives parallel work and a sequential counter does not. Two worktrees, two branches, or two
agents each allocate the next number, each is correct, and the merge produces two records claiming one
identity. Stability is the only property the number carried, and a filename declared immutable carries
it without needing an allocator.

The immutability is the part to enforce. A title may be improved at any time; the file it lives in
may not be renamed, because every commit, review, and comment that cites the slug depends on it.

## Body

Five sections, fixed. [08 — Gates](./08-gates.md) wires the check.

```markdown
# <Short title naming the choice, not the task>

## Context and Problem Statement

<Two or three sentences. What problem, and why it matters.>

## Considered Options

- `<option 1>` — chosen.
- `<option 2>` — rejected: <one sentence>.
- `<option 3>` — deferred: revisit if <condition>.

## Decision Outcome

Chosen option: `<option 1>` — <one sentence: why>.

## Consequences

- Good: <trade-off gained>
- Bad: <trade-off accepted>

## Status

<Proposed | Accepted | Implemented | Deprecated | Superseded | Rejected>
```

- A filled record MUST be at or below 350 words.
- A filled record MUST carry exactly one `Status`.
- A record title MUST name the choice, not the implementation task.

"Use PostgreSQL for primary storage" records a decision. "Implement storage support" is a ticket.

The word cap is a forcing function. A record that will not fit is usually two decisions, and the
correct response is two records. Diagrams, option matrices, benchmark data, and migration steps belong
in reference, linked from the record.

The cap is a local optimizer, so it needs a counterweight. It reads one file and asks whether that
file is small, which any partition of a large argument satisfies — a subsystem can be designed in
350-word installments, each record clean and no artifact stating what the subsystem is. A run of
related records about one domain is evidence that the domain lacks a spec, not that it needs another
record. Write or extend the spec before adding the next one. The signal is a shared slug prefix or a
repeated subject across consecutive records; it is a review question, never a gate, because related
records are often genuinely independent decisions and a hook that failed on them would be wrong in
the cases the corpus most needs.

## Dispositions

`Considered Options` carries one line per option: what it was, and what happened to it.

- Every considered option MUST carry a disposition of chosen, rejected, or deferred.
- A rejected option MUST state why in one sentence.
- A deferred option MUST name the condition that would reopen it.

The reopening condition is what keeps deferrals from accumulating. "We will look at it later" is not
a decision and belongs in the plan zone. "Revisit if append-only ingestion becomes the dominant
workload" is a boundary on the current choice and earns its line.

## Status

| Value         | Means                           |
| ------------- | ------------------------------- |
| `Proposed`    | open for review                 |
| `Accepted`    | chosen, not necessarily built   |
| `Implemented` | enacted by the project          |
| `Deprecated`  | no longer applies, no successor |
| `Superseded`  | replaced by a later record      |
| `Rejected`    | explicitly not chosen           |

- A record MUST use a status from this table.
- An implemented record MUST link what enacts it.
- A superseded record MUST link its successor.
- A deprecated record MUST state why it stopped applying.

Do not invent synonyms. `Done`, `Canceled`, and `Obsolete` make the field unfilterable, and status is
data: a reviewer or a script classifies a record by grepping for it.

## Records are never deleted and never revised

- A merged decision record MUST NOT be deleted.
- A decision record MUST NOT be edited to describe a later design.

Every durable proposal process preserves its log for the same reason: a reader in two years needs the
trail, not only the latest state. Deleting the losing arguments leaves a record that looks
authoritative and is not.

Never-revise does not mean never-correct. Fix typos, broken links, and metadata that was wrong when
written. Change a status when the status changes. Do not rewrite the body so a later design appears to
have been the original choice.

There is no amendment annotation in this framework, and the absence is deliberate. An annotation
saying "part of this record no longer holds" exists only where records are asked to describe the
present. They are not asked to here, so a reader who wants the present reads the spec, and a record
left alone for two years is not stale.

## A record is not loaded

An agent starting work loads the specs that bind it, not the decision log. The log is read when
someone asks why: during a review that reopens a settled question, or when a rule looks arbitrary.

This is what makes an unbounded log affordable. A corpus of four hundred records costs nothing on a
session that does not consult it, while four hundred records treated as current rules would be
unreadable and mutually contradictory.

- A rule an agent must apply MUST be stated in a spec.
- A decision record MUST NOT be the only statement of a binding rule.

A record whose consequence is enforceable produces two artifacts in the same change: the record
explaining the choice, and the requirement in a spec binding it. The record links the rule ID.

## When a choice earns a record

Write one when at least one holds:

- The choice is cross-cutting: more than one area has to know.
- It is expensive to reverse once work depends on it.
- It constrains future work, so later choices are made inside it.
- It rejects a plausible alternative someone will otherwise propose again.

Everything else belongs in the plan, in the code, or in a comment. A choice that is local, obvious,
and fully expressed by a name or a signature is not a decision record.

A rejection with no positive counterpart still earns a record. If a proposal was rejected and the
project simply carried on, the record is the only thing standing between the project and the same
debate next quarter.

## Sources

- Log4brains, on adopting the slug as the record's unique identifier:
  <https://thomvaill.github.io/log4brains/adr/adr/20201016-use-the-adr-slug-as-its-unique-id/>
- MADR, for the five-section minimal shape: <https://adr.github.io/madr/>
- AWS, on an accepted record being immutable and the collection forming a decision log:
  <https://docs.aws.amazon.com/prescriptive-guidance/latest/architectural-decision-records/adr-process.html>
- Kubernetes KEP process, on retaining rejected and replaced records:
  <https://github.com/kubernetes/enhancements/blob/master/keps/sig-architecture/0000-kep-process/README.md>
