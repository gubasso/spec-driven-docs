# Apply the rendered block to this repository

## Context and Problem Statement

This repository is an instance of itself, and the one whose managed pre-commit region no installer wrote. The region was kept by hand. It differed from the render in six ways: canon-only hooks inside the markers, hook order, configuration paths, the entry prefix, name quoting, and fixture arguments on eight gates. Only hook-id presence and per-gate selectors held it to the registry, so the repository ran a layout it did not ship.

## Considered Options

- `apply the render with this checkout's entry prefix` — chosen.
- `a hand-kept region checked by id presence` — rejected: it proves a gate is named and nothing about how it is wired, so the canon could drift from every instance while its checks stayed green.
- `an entry-prefix key in the declaration` — rejected: the prefix is a fact about this checkout, not about what the gates judge, and `sdd hooks` already takes it as `--entry`.

## Decision Outcome

Chosen option: `apply the render with this checkout's entry prefix` — `just hooks` runs `sdd hooks --apply --entry 'cargo run -q --'`, and a cargo test runs the same command with `--check`. Hooks only this repository runs sit outside the markers. The fixture arguments became a declaration filter and repository-only cargo tests, which run the record gates over the fixture records.

Enforced by `release:the-delivered-gate-set-is-declared-once`.

## Consequences

- Good: this repository runs exactly the region an instance receives, byte for byte.
- Good: a new gate reaches this repository through `just manifest`, not a hand edit.
- Bad: coverage the fixture arguments gave now lives in cargo tests, which a reader of the hook configuration does not see.

## Status

Implemented
