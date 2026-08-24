# 09 — Spec to Code

A spec may exist before the code it binds. This chapter owns the seam between the two: how a
requirement written first becomes work, how the work declares what it changed, and how coverage is
derived rather than stored. It states the contract any planning tool can satisfy; it names none.

## A failing verification is an unimplemented rule

Every requirement carries a `Verify:` command that exits non-zero on violation. Before the behavior
exists, the command fails — and that failure is not a defect in the spec. It is the definition of
"not yet built."

- An author MAY write a requirement whose verification command does not yet pass.
- A unit of work enacting a requirement MUST leave its verification command passing.

This is what makes the spec a legitimate greenfield artifact. The requirement states the agreement,
the failing command states the distance, and the work closes it. Writing the check before the
behavior is the same discipline as writing the failing test first, applied to documentation.

## Requirement state is derived, never stored

Run the verification commands of a domain and the failures are its unimplemented requirements. That
is the whole status system.

- A specification MUST NOT carry a status marker on a requirement.

A stored status (`status: implemented`, a checkbox, a phase column) is a second copy of a fact the
command already decides, and the copy drifts the first time behavior changes without the marker.
Derived state cannot disagree with the code, because it is recomputed from the code on every ask.

## Precedence is phase-dependent

[00 — Model](./00-model.md) owns precedence and states both directions. The marker that selects the
direction lives here: a unit of work is in flight for a rule while an open entry document cites that
rule's ID. While it is, the spec states the agreement and divergent code is the defect. When no work
cites the rule, the code is the observed truth and a divergent spec is the defect.

## The entry document enacts rules by ID

[05 — Agent Context](./05-agent-context.md) gives each unit of work one entry document that names
its sources by path. When the work changes agreed behavior, path-level naming is not enough: the
entry document also names the rules, so enactment is greppable.

- An entry document that changes agreed behavior MUST cite the affected rule IDs.
- An entry document citing a spec change MUST type it as `ADDED`, `MODIFIED`, or `REMOVED`.

The three types are the three operations of [07 — Lifecycle](./07-lifecycle.md), stated from the
work's side. One clause per affected rule, on the line that names the owning spec:

```markdown
- `_docs/specs/SPEC-auth.md` — ADDED `auth:token-expiry-is-bounded`
- `_docs/specs/SPEC-auth.md` — MODIFIED `auth:refresh-requires-reauth`
```

The clause grammar is fixed so a command can check the shape: the type in capitals, then the rule ID
in inline code, matching `[a-z0-9-]+:[a-z0-9-]+`. A typed clause whose ID token is malformed is a
gate failure; whether a story that changed a spec declared the clause at all is a review question,
because no command can see the omission.

## A comment cites the rule, never the record

Code is the last word on behavior, so a comment restating behavior is a second copy of it. What a
comment holds is the rule this code exists to satisfy, and an invariant no spec of this project owns.

- A comment citing an agreement MUST cite it by rule ID.
- A comment MUST NOT name a decision record.

The clause grammar is the entry document's, extended to code: the type in capitals, then the rule ID.
`SATISFIES` marks the code that implements a rule, and `VERIFIES` marks the test that proves it.

```python
# SATISFIES retry-artifacts:cleanup-follows-upload
cleanup_after_upload()
```

Naming a record instead breaks the walk in both directions. A record is frozen, can be superseded,
and holds the argument rather than the obligation, so a reader who follows it arrives at what was
decided once rather than at what binds now. The rule ID resolves to the binding sentence, and
resolves under a grep.

An invariant imposed by another system carries no rule ID, because no spec here owns it. It stays in
the comment, stated so a reader can falsify it.

```python
# The vendor returns 200 with an empty body on a replayed idempotency key.
if not response.body:
    return cached
```

Everything else is a deletion or a rename. A comment restating the next line goes, and a comment
compensating for a vague name becomes the name.

## A suppression names its case and its exit

The invariant a comment may hold has a second form: a live defect in a system this project does not
own, worked around here. It carries no rule ID, because nothing about it was agreed. What makes it
honest is the case it names and the condition that ends it.

- A suppression MUST name its case id at the suppression.
- A suppression MUST carry the condition under which it is removed.

The rule reaches every tool, not only the test runner. A formatter range, a linter disable comment,
and a dependency pinned back one version are the same act with the same failure mode: the hazard is
that a suppression with no exit becomes permanent by default.

```python
@pytest.mark.xfail(reason="KI-upstream-500-on-replayed-webhook", strict=True)
def test_webhook_replay_is_idempotent():
    ...
```

Prefer the strict form. A non-strict expected failure keeps passing after the upstream fix lands, so
the suppression outlives the bug it was written for and nobody learns the case can close. A strict one
turns the suite red the moment the fix arrives, which is the signal that closes it.

A test that must not hide the bug at all keeps failing, with the case id in a comment beside it. The
case id is the record's filename, so it resolves the same way a rule ID does — the reason string
needs no restated summary, because the record it names holds the symptom, the workaround, and the
retire condition. The case, its states, and its retirement belong to
[07 — Lifecycle](./07-lifecycle.md).

## Coverage is a grep

The rule ID is one string in four record sets: the spec defines it, a decision record argues for it,
an entry document enacts it, and a comment marks the code that satisfies it. Traceability is
therefore derived on demand, in both directions, from the records that already exist.

```bash
rg -o '^### `([a-z0-9-]+:[a-z0-9-]+)`' -r '$1' _docs/specs | sort -u > /tmp/agreed
rg -oe '(ADDED|MODIFIED|REMOVED) `[a-z0-9-]+:[a-z0-9-]+`' -r '$0' <plan-zone> \
  | rg -o '[a-z0-9-]+:[a-z0-9-]+' | sort -u > /tmp/enacted
comm -23 /tmp/agreed /tmp/enacted
```

The third command prints the agreed rules no work has enacted: the spec-first backlog, computed from
two record sets and stored in neither. The same shape run against code prints the opposite defect.

```bash
rg -o '(SATISFIES|VERIFIES) ([a-z0-9-]+:[a-z0-9-]+)' -r '$2' --glob '!_docs/**' \
  | sort -u > /tmp/cited
comm -13 /tmp/agreed /tmp/cited
```

What that prints is a rule ID cited in code that no spec defines: a fabricated citation, and the
check that makes citing worth anything.

- A project MUST NOT maintain a stored coverage artifact.

A traceability matrix, a rules-to-stories index, or a backlog file restates what the greps derive,
and each is the filesystem-index shape [00 — Model](./00-model.md) forbids: a copy kept because the
records exist, drifting on the next change to either side.

## What the planning tool owes

This framework does not name a planning tool. Any tool serves whose work record satisfies the
contract the rules above already state: one entry document per unit of work, sources named by path,
spec changes cited by typed rule ID, and the record readable by the greps in this chapter. The
inverse dependency is also bounded: the specs never name the tool, so replacing it edits the plan
zone and nothing under `specs/` or `decisions/`.

## Unenforced

Two rules in this chapter no command can decide: that a unit of work which changed a spec declared
the typed clause at all, and that the cited type matches the diff. A gate checks every declared
clause and cannot see an omitted or mistyped one; the reviewer compares the spec diff against the
entry document. [08 — Gates](./08-gates.md) carries both in the unenforced list.

## Sources

- GitHub Spec Kit, on tests written first and confirmed to fail before implementation:
  <https://github.com/github/spec-kit/blob/main/spec-driven.md>
- OpenSpec, for the `ADDED` / `MODIFIED` / `REMOVED` delta typing:
  <https://github.com/Fission-AI/OpenSpec/blob/main/docs/concepts.md>
- AWS Kiro, on tasks tracing to requirement identifiers:
  <https://kiro.dev/docs/specs/>
- StrictDoc, for the relation marker in a source comment and its implementation and verification
  roles: <https://strictdoc.readthedocs.io/en/stable/sphinx/strictdoc_01_user_guide.html>
- OpenFastTrace, for coverage tags written as source comments against specification item ids:
  <https://github.com/itsallcode/openfasttrace>
- DO-178C, on every source code element tracing back to a requirement:
  <https://www.parasoft.com/learning-center/do-178c/requirements-traceability/>
