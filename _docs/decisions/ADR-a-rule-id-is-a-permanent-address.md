# A rule ID is a permanent address

## Context and Problem Statement

`docs-format:chapter-stays-within-200-lines` names a number that stopped being universal the day a project could record a ceiling above it. Renaming it would read better and break every adopter: `gate-message-cites-a-rule` resolves the binary's citations against each instance's own specifications, an upgrade never rewrites those, so a renamed ID fails every commit on files nobody touched. An instance's older specification can also disagree with its configuration, and something must detect that without reading prose.

## Considered Options

- `keep every ID and detect an unreconciled instance by a sentinel rule's absence` — chosen.
- `rename with a transitional alias` — rejected: it defers the break, and findings print an ID the adopter cannot find between upgrading the binary and running `sdd upgrade`.
- `rename with an automatic migration of the adopted specifications` — rejected: `distribution:the-declaration-is-seeded-once-and-then-owned` forbids an upgrade from rewriting a file the project owns.
- `match the old sentence to detect an older specification` — rejected: `docs-specs:rule-id-outlives-its-sentence` permits the sentence to change.

## Decision Outcome

Chosen option: `keep every ID and detect an unreconciled instance by a sentinel rule's absence` — an ID is an address, not prose, so a number inside one outlives its universality.

Each declarable feature has one sentinel rule in one new adopted specification, which seeds on upgrade because an addition reaches an existing instance and an edit does not. No delivered gate cites a sentinel. `sdd verify` notes an active declaration whose sentinel the local specifications lack, and `sdd policy reconcile` appends the rule on request, previewing first and writing nothing where the file is not in a shape it recognizes. The boundary is the actor: an upgrade never writes an adopted file, and an operator asking for a previewed change may.

Enforced by `docs-specs:rule-id-outlives-its-sentence` and `distribution:the-declaration-is-seeded-once-and-then-owned`.

## Consequences

- Good: no adopter fails on files nobody touched.
- Good: reconciliation is structural, so a reworded sentence never confuses it.
- Bad: a rule ID can carry a number its statement no longer holds universal.

## Status

Implemented

Enacted by `src/domain/policy.rs`, `src/services/policy.rs`, and `src/commands/policy.rs`.
