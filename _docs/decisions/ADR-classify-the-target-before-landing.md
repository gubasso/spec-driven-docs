# Classify the target before landing

## Context and Problem Statement

The setup and migration skills routed on one signal: whether the target carried an instance. A repository with a settled documentation corpus and no instance read as a fresh start, so a setup could land seeds beside the old convention and leave two sources of truth. The routing needed a verdict about the corpus itself, read from evidence.

## Considered Options

- `a read-only classification verb, with the rule in the binary` — chosen.
- `agent judgment in skill prose` — rejected: "little docs" is a feel, not a rule; two agents classify the same tree differently and neither answer is testable.
- `a binary-owned migration state machine driving the whole sweep` — deferred: revisit if resumed migrations lose state in practice, or a second consumer needs the checklist machine-readable. Until then the checklist convention lives in the migration chapter and the skills.

## Decision Outcome

Chosen option: `a read-only classification verb, with the rule in the binary`. `sdd assess` gathers the evidence — documentation roots, the document inventory, methodology markers, per-profile collisions, the workshop — and computes one verdict by an explicit, unit-tested rule: `brownfield` on a corpus under a documentation root or any methodology marker, `greenfield` on nothing beyond root metadata, `needs-decision` otherwise, with the ambiguous case going to the operator rather than to a guess. The skills route by the verdict; the pre-flight gate names the step for setup and migration tasks at targets with no instance.

Enforced by `distribution:a-landing-classifies-its-target-first` and `distribution:initialization-preserves-project-content`.

## Consequences

- Good: brownfield targets stop reading as greenfield, so a setup cannot silently start a second convention.
- Good: the classification is evidence a plan can cite, and a test can hold.
- Bad: the recognized roots and markers are a fixed list; an unrecognized convention classifies `needs-decision` and costs the operator a question.

## Status

Implemented; `src/services/assess.rs` and `method/12-migration.md` enact it.
