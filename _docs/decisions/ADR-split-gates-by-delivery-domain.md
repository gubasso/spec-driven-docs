# Split gates by delivery domain

## Context and Problem Statement

Gates lived in one directory and left this repository by two routes: projected into an instance by
the installer, and published to consumers who install this repository as a pre-commit repo. Neither
route derived from the other, and the installer wrote a managed block naming only the verifier, so
an instance received every gate as a hashed, executable file that nothing ever ran. The verifier
compares hashes and reads no document, so it reported a healthy instance over gates that were
silently inert.

## Considered Options

- `two gate directories, one declaration` — chosen.
- `one directory with a filter list` — rejected: the boundary lives in a list a reader has to hold
  against the directory, and a new gate defaults to the wrong side.
- `let the verifier run the gates` — rejected: it conflates payload integrity with document
  conformance, and a consumer installing by rev never calls the verifier at all.

## Decision Outcome

Chosen option: `two gate directories, one declaration` — `gates/instance/` is what an instance
receives, `gates/canon/` is what checks invariants only this repository has, and
`instance/gates.json` declares the delivered set that both the projected block and the published
manifest are rendered from.

Enforced by `distribution:the-delivered-gate-set-is-declared-once` and
`distribution:a-canon-gate-is-not-delivered`.

## Consequences

- Good: a gate cannot reach one delivery and miss the other, and a canon-only gate cannot reach an
  instance at all.
- Good: the directory a gate is written into is the decision about who runs it.
- Bad: adding a gate now edits three places — the script, the declaration, and the published
  manifest — and two of them are held in agreement by a gate rather than by hand.

## Status

Implemented

Enacted by `scripts/render-gate-block.sh`, `gates/canon/delivered-domain.sh`, and the wiring
assertions in `tests/test-instantiation.sh`.
