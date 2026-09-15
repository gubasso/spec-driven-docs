# Spec to Code

A spec can exist before the code it binds. This chapter owns the seam between the two. It covers how a requirement written first becomes work, how the work declares what it changed, and how coverage is derived rather than stored.

## A failing verification is an unimplemented rule

Every requirement carries a `Verify:` command that exits non-zero on violation. Before the behavior exists, the command fails. That failure is not a defect in the spec. It is the definition of "not yet built."

- An author MAY write a requirement whose verification command does not yet pass.
- Work enacting a requirement MUST leave its verification command passing.

This is what makes the spec a legitimate greenfield artifact. The requirement states the agreement, the failing command states the distance, and the work closes it. Writing the check before the behavior is the same discipline as writing the failing test first, applied to documentation.

## Requirement state is derived, never stored

Run the verification commands of a domain and the failures are its unimplemented requirements. That is the whole status system.

- A specification MUST NOT carry a status marker on a requirement.

A stored status (`status: implemented`, a checkbox, a phase column) is a second copy of a fact the command already decides. The copy drifts the first time behavior changes without the marker. Derived state cannot disagree with the code, because it is recomputed from the code on every ask.

## Precedence is phase-dependent

[Model](./model.md) owns precedence and states both directions. The marker that selects the direction lives here. Work is in flight for a rule while an open change cites that rule's ID. While it is, the spec states the agreement and divergent code is the defect. When no work cites the rule, the code is the observed truth and a divergent spec is the defect.

## A comment cites the rule, never the record

Code is the last word on behavior, so a comment restating behavior is a second copy of it. What a comment holds is the rule this code exists to satisfy, and an invariant no spec of this project owns.

- A comment citing an agreement MUST cite it by rule ID.
- A comment MUST NOT name a decision record.

The clause grammar is fixed so a command can check the shape: the marker in capitals, then the rule ID in `[a-z0-9-]+:[a-z0-9-]+` form. `SATISFIES` marks the code that implements a rule, and `VERIFIES` marks the test that proves it.

```python
# SATISFIES retry-artifacts:cleanup-follows-upload
cleanup_after_upload()
```

Naming a record instead breaks the walk in both directions. A record is frozen, can be superseded, and holds the argument rather than the obligation. As a result, a reader who follows it arrives at what was decided once rather than at what binds now. The rule ID resolves to the binding sentence, and resolves under a grep.

An invariant imposed by another system carries no rule ID, because no spec here owns it. It stays in the comment, stated so a reader can falsify it.

```python
# The vendor returns 200 with an empty body on a replayed idempotency key.
if not response.body:
    return cached
```

Everything else is a deletion or a rename. A comment restating the next line goes, and a comment compensating for a vague name becomes the name.

## A suppression names its case and its exit

The invariant a comment can hold has a second form: a live defect in a system this project does not own, worked around here. It carries no rule ID, because nothing about it was agreed. What makes it honest is the case it names and the condition that ends it.

- A suppression over a defect this project does not own MUST name its case id at the suppression.
- A suppression over a defect this project does not own MUST carry the condition under which it is removed.
- A suppression that masks no external defect MUST state its reason where its own tool reads one.

The rule reaches every tool, not only the test runner. A formatter range, a linter disable comment, and a dependency pinned back one version are the same act with the same failure mode. A suppression with no exit becomes permanent by default.

```python
@pytest.mark.xfail(reason="KI-upstream-500-on-replayed-webhook", strict=True)
def test_webhook_replay_is_idempotent():
    ...
```

Prefer the strict form. A non-strict expected failure keeps passing after the upstream fix lands. The suppression then outlives the bug it was written for, and nobody learns the case can close. A strict one turns the suite red the moment the fix arrives, which is the signal that closes it.

Some suppressions have no exit. A lint disabled over a construct this project chose and keeps masks nothing external. No record can carry a condition anyone meets. Writing one anyway produces the unremovable mask the case rule exists to prevent. That suppression states its reason instead, in the position its own tool defines.

No gate of this framework reads that reason. Where the language's own linter has a rule for it, turn that rule on.

```text
Rust        clippy::allow_attributes_without_reason
JavaScript  eslint-comments/require-description
```

Not every toolchain ships one. Ruff has no rule that requires a description on a `noqa` directive, and `RUF100` reports an obsolete directive rather than an undescribed one. Where the language offers nothing, the reason stays a review obligation.

A gate this framework delivers parses no grammar somebody else defines. What it reads is the `KI-` token this convention owns, and it fails where that token resolves to no record. [Gates](./gates.md) holds the check and carries this expectation in its unenforced table.

A test that must not hide the bug at all keeps failing, with the case id in a comment beside it. The case id is the record's filename, so it resolves the same way a rule ID does. The reason string needs no restated summary. The record it names holds the symptom, the workaround, and the retire condition. The case, its states, and its retirement belong to [Lifecycle](./lifecycle.md).

## Coverage is a grep

The rule ID is one string in three record sets. The spec defines it, a decision record argues for it, and a comment marks the code that satisfies it. Traceability is therefore derived on demand, in both directions, from the records that already exist.

```bash
rg -o '^### `([a-z0-9-]+:[a-z0-9-]+)`' -r '$1' _docs/specs | sort -u > /tmp/agreed
rg -o '(SATISFIES|VERIFIES) ([a-z0-9-]+:[a-z0-9-]+)' -r '$2' --glob '!_docs/**' \
  | sort -u > /tmp/cited
comm -13 /tmp/agreed /tmp/cited
```

What that prints is a rule ID cited in code that no spec defines: a fabricated citation, and the check that makes citing worth anything. The inverse, `comm -23`, prints the agreed rules no code cites, computed from two record sets and stored in neither.

- A project MUST NOT maintain a stored coverage artifact.

A traceability matrix or a coverage index restates what the greps derive. Each is the filesystem-index shape [Model](./model.md) forbids. It is a copy kept because the records exist, and it drifts on the next change to either side.

## Sources

- GitHub Spec Kit, on tests written first and confirmed to fail before implementation: <https://github.com/github/spec-kit/blob/main/spec-driven.md>
- AWS Kiro, on tasks tracing to requirement identifiers: <https://kiro.dev/docs/specs/>
- StrictDoc, for the relation marker in a source comment and its implementation and verification roles: <https://strictdoc.readthedocs.io/en/stable/stable/docs/strictdoc_01_user_guide.html>
- OpenFastTrace, for coverage tags written as source comments against specification item ids: <https://github.com/itsallcode/openfasttrace>
- DO-178C, on every source code element tracing back to a requirement: <https://www.parasoft.com/learning-center/do-178c/requirements-traceability/>
