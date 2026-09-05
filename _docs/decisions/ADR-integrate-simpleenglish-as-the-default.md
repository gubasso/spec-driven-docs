# Integrate SimpleEnglish as the default writing convention

## Context and Problem Statement

This project needed one writing convention for technical text that both people and coding agents follow. A convention that a user must install, select, or invoke would reach few documents. A convention this project wrote from scratch would drift from the source it borrowed and would carry no shared vocabulary.

## Considered Options

- `vendor SimpleEnglish and activate it by default` — chosen.
- `write a local plain-English style` — rejected: a private copy drifts from the upstream pattern and names nothing a reader can look up.
- `ship an optional skill the user selects` — rejected: a convention no one activates governs nothing.

## Decision Outcome

Chosen option: `vendor SimpleEnglish and activate it by default` — the binary carries the pinned SimpleEnglish surface, the install projects it into every instance, and the root `AGENTS.md` block names its `Plain` mode as the default for technical text. This project owns only the compatibility adapters a single offline binary needs, never a forked writing pattern.

Enforced by `simple-english:the-upstream-pattern-is-binding`, `simple-english:plain-is-the-default`, `simple-english:the-dependency-is-available-offline`, and `simple-english:an-objective-check-matches-its-upstream-rule`.

## Consequences

- Good: a user never installs or selects a separate skill, and every instance reads the same pattern offline.
- Good: the deterministic subset is a rule-citing gate, and the judgment rules stay review-held.
- Bad: the vendored surface must be refreshed by a tracked, reviewed vendor run rather than edited in place.

## Status

Implemented

Enacted by `src/gates/simple_english.rs`, the profile projection under `.spec-driven-docs/upstreams/simpleenglish/`, and the managed root `AGENTS.md` block.
