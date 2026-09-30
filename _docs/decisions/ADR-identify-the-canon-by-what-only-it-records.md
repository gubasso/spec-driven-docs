# Identify the canon by what only it records

## Context and Problem Statement

`sdd verify` holds a record to one of two layouts: an installed instance, or this repository, where every owned file sits at its authored path. It told them apart by checking that every managed source appeared among the managed destinations. Once the managed lint configuration landed at `{docs_root}/.markdownlint-cli2.jsonc`, a knowledge-base instance resolved that destination to the source itself. It was then read as the canon, owed specs and templates it never held, and failed a fresh landing.

## Considered Options

- `the presence of a skill-shared/ managed entry` — chosen.
- `managed source equal to managed destination` — rejected: a templated destination makes it true for a knowledge-base instance.
- `a new layout field in the record` — rejected: it duplicates evidence the self-manifest already records, and a field is as editable as the entry it would replace.

## Decision Outcome

Chosen option: `the presence of a skill-shared/ managed entry` — `sdd self-manifest` records the shared skill artifacts at their authored paths, and no consumer landing writes anything there. The canon branch owes the managed sources, the embedded specs, the canon templates, and the pre-commit block. The consumer branch owes every declared destination and both integration blocks.

The marker recognizes the ordinary layouts and authenticates nothing. The record is a file a user can edit, and a record claiming the other layout changes which obligations the projection check enforces. The marker is not a defense against a record that shrinks what it holds.

Enforced by `distribution:manifest-identifies-every-owned-file`.

## Consequences

- Good: a templated managed destination no longer changes how an instance is read.
- Bad: a forged marker still moves a record to the canon's obligations, and a unit test states that boundary.

## Status

Implemented
