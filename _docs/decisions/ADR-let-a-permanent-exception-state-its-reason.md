# Let a permanent exception state its reason

## Context and Problem Statement

The suppression rule required a known-issue case at every suppression, without qualification. A lint disabled over a construct this project chose and keeps masks nothing external, so no record could carry a retirement condition anyone can meet. Writing one anyway produces the unremovable mask the rule exists to prevent. The gate recognized two markdown comment forms, so every other suppression a repository carries bound by authorship and no command saw it.

## Considered Options

- `a second rule and a marker, with the form set widened` — chosen.
- `a known-issue record for every suppression` — rejected: a record whose condition nobody meets is the artifact the case rule exists to prevent.
- `narrowing the rule to the two markdown forms` — rejected: it is honest and it leaves the first problem open, because the rule's headline case is a suppressed test the gate still cannot see.

## Decision Outcome

Chosen option: `a second rule and a marker, with the form set widened` — a suppression over an external defect names its case, and a suppression that masks no external defect states its reason in an `sdd: permanent` marker and names no case. The two are exclusive, so a gate can decide which one a site chose.

A form counts only in a file the tool that honors it reads. That scoping is what lets a spec, a chapter, and the gate itself name a form without being judged by it, and it needs no split literals or exempt paths.

Enforced by `spec-to-code:a-suppression-names-its-case` and `spec-to-code:a-permanent-exception-states-its-reason`.

## Consequences

- Good: a deliberate exception is greppable and carries its reason, rather than passing unseen.
- Good: the two rules no longer contradict the rule that a record carries a retirement condition.
- Bad: an extensionless script, a block comment, and a manifest lint table stay outside a line-scoped scan, and the module doc must say so.

## Status

Implemented

Enacted by `src/gates/suppression_names_its_case.rs`.
