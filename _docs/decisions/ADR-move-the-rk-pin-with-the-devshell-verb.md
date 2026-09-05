# Move the rk pin with the devshell verb

## Context and Problem Statement

[ADR-consume-release-kit-through-the-flake-input](./ADR-consume-release-kit-through-the-flake-input.md) put the rk pin in a flake input and gave it one mover this repository wrote by hand: `scripts/rk-bump.sh`, `scripts/rk-autobump.sh`, and two bats suites, 679 lines in all. release-kit now ships that mover as `rk devshell`, and a consumer that keeps its own runs two mechanisms over the same two files. Two movers over one pin fight or silently undo each other.

## Considered Options

- `Move the pin with rk devshell sync` — chosen.
- `Keep the hand-rolled scripts` — rejected: this repository would maintain 679 lines of a transaction the CLI now owns, and the two would drift apart on every release.
- `Keep both, with the script disabled by a switch` — rejected: a dormant second mover is still a second mover, and the switch is one more fact to hold true.

## Decision Outcome

Chosen option: `Move the pin with rk devshell sync` — the verb owns the snapshot, the lock update, and the devshell build as one transaction, and `.envrc` invokes it in one line. `RK_DEVSHELL_SYNC=0` in `.envrc.local` holds the pin where it is.

The deletion ran through `rk devshell clean`, not by hand, so the catalog behind every later consumer was tested against the real predecessor.

Enforced by `release:the-rk-pin-has-two-facts-and-one-mover`.

## Consequences

- Good: 679 lines leave the tree, and the transaction is maintained upstream with its own tests.
- Good: `rk devshell status` judges the wiring, so a second mover cannot return unnoticed.
- Bad: the mover now moves on release-kit's schedule, not this repository's, and a defect in it is fixed upstream and waited for.

## Status

Implemented: `.envrc`, `.envrc.local.example`, `flake.nix`, `justfile`, `.editorconfig`, `_docs/specs/SPEC-release.md`.

Supersedes [ADR-consume-release-kit-through-the-flake-input](./ADR-consume-release-kit-through-the-flake-input.md) on the mover alone: the two facts and their files are unchanged.
