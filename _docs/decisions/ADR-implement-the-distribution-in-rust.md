# Implement the distribution in Rust

## Context and Problem Statement

The distribution grew to twenty-two gates, six orchestration scripts, and three test suites of POSIX shell. Every instance carried a sixteen-tool ambient dependency list, upgrades required a canon checkout, and correctness depended on comments explaining shell traps — no pipefail, byte locales, glob edge cases — rather than on types a compiler holds.

## Considered Options

- `One Rust CLI carrying the whole program` — chosen.
- `Keep POSIX shell` — rejected: the invariants lived in comments, and every consumer machine was part of the test matrix.
- `Node or Python CLI` — rejected: a runtime prerequisite on every instance contradicts the self-contained verification this distribution promises.
- `A thin dispatcher over the existing scripts` — rejected: one command name without one implementation keeps every shell failure mode.

## Decision Outcome

Chosen option: `One Rust CLI carrying the whole program` — a single static binary, `sdd`, built from crate `spec-driven-docs` at the repository root, following the exobrain CLI conventions: clap derive parse shapes, one handler per subcommand, typed errors with a tested exit-code matrix, and exit `1` reserved for a check that found violations.

Rule IDs become an enum held equal to the specs by a test, so a gate cannot cite an address no spec defines. The eighty-five shell controls survive as the unit-test corpus. Exit codes follow BSD sysexits for operational failures.

Enforced by `spec-to-code:a-gate-message-cites-the-rule` and the canon test suite.

## Consequences

- Good: instances need `git`, `pre-commit`, and one binary; the tool list is gone.
- Good: invariants moved from comments into types and tests.
- Bad: contributors need a Rust toolchain the shell never asked for.
- Bad: a compiled distribution needs release binaries per platform, which the release workflow must now produce.

## Status

Accepted
