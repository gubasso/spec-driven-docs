# Split the known-issue record into two axes

## Context and Problem Statement

A known-issue record carried one word, `state:`, saying how this project handles an external defect. It never answered where the case stands upstream, so "which cases can we file today" was a question only a full read of the zone could settle. The word was also ungoverned: no gate read it, and a record could state a handling the method never defined. The gate that demanded a retire condition demanded one from every record, which contradicted the rule that a mitigated record carries none.

## Considered Options

- `two orthogonal one-word axes` — chosen.
- `one longer state vocabulary` — rejected: the two questions are independent, so one word covering both is their cross product, and every value then answers half a question.
- `a committed registry file listing every case` — rejected: one file every new case edits is the branch collision the slug case id already avoids, and it holds a second copy of what the record states.

## Decision Outcome

Chosen option: `two orthogonal one-word axes` — `state:` says how this project handles the defect, `filing:` says where the case stands upstream, and each carries a closed vocabulary a gate holds. A grep for `filing: ready` is what the split buys.

Enforced by `known-issues:a-record-carries-one-state` and `known-issues:a-record-carries-one-filing-state`.

## Consequences

- Good: `known-issues:a-filed-record-carries-its-report` triggers on a stated field rather than on a regex reading an issue number out of a URL.
- Good: `known-issues:a-record-carries-its-retirement-condition` can be conditional, because the gate now reads the state the rule turns on.
- Good: the index is derived by `sdd ki list`, so no committed listing can drift from the records.
- Bad: every record carries two fields rather than one, and a record written before this decision fails until both are added.

## Status

Implemented

Enacted by `src/gates/ki_state.rs`, `src/gates/ki_filing.rs`, and the shared axis reader in `src/gates/ki_record.rs`.
