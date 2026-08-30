# Serve instances, not remote consumers

## Context and Problem Statement

The repository carried two audiences. An instance adopts the framework: `sdd init` lands the payload and splices a managed block rendered from the gate registry at install time. A remote consumer adopts nothing and names this repository as a pre-commit `repo:`, selecting gates by id out of a published `.pre-commit-hooks.yaml`.

The second audience was never served. Against a repository with no docs root and no manifest, 7 of the 25 published gates fail rather than skip, because they read an instance layout and report its absence as a violation. The published example named one of the seven. Consuming any gate also costs a Rust toolchain, whatever the consumer's own stack.

## Considered Options

- `serve instances only` — chosen.
- `publish a truthful subset` — rejected: a `scope` field on each registry row would render only layout-independent gates, but it commits their portability as public API, and hook ids become names that cannot be refactored.
- `make a missing layout skip` — rejected: inside an instance a vanished docs root is a real defect, and silencing it there to serve a consumer inverts whose correctness matters.

## Decision Outcome

Chosen option: `serve instances only`. The published manifest is withdrawn, the renderer keeps one output shape, and the gate set reaches a repository by installation alone. What an instance receives is unchanged, byte for byte.

The registry stays the single declaration, and nothing derived from it is committed: the block is rendered when installed, so no checked-in copy can fall behind. A snippet duplicating the block, read by nothing, is withdrawn with it.

Enforced by `release:the-delivered-gate-set-is-declared-once` and `release:a-canon-gate-is-not-delivered`.

## Consequences

- Good: every delivered gate can assume the instance layout, so failing on its absence is unambiguously correct.
- Good: hook ids stay internal names, refactorable without breaking a stranger's configuration.
- Bad: the on-ramp is gone, and someone wanting one check must adopt the framework or copy it.
- Bad: anyone pinning a tag breaks on upgrade, with an error that reads like a broken repository.

## Status

Accepted
