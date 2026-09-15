# Declare the projection in the bundle

## Context and Problem Statement

What a release lands lived in Rust constants. When 0.7.2 added a specification to the adopted set, that fact existed only inside the 0.7.2 binary, so only that binary could land 0.7.2. An engine planning toward an intermediate release had to guess, and a guess that lands the running engine's set is not that release. The sentinels had the same problem.

## Considered Options

- `declare the projection in the release` — chosen.
- `keep the constants` — rejected: each release stays the only witness of what it lands, so every plan toward another release is an approximation.
- `carry every historical projection in the engine` — rejected: the engine would grow one entry per release forever, written by whoever remembered.
- `derive the projection from the archive's shape` — rejected: which file is managed and which is adopted is a decision, not something a directory listing reveals.

## Decision Outcome

Chosen option: `declare the projection in the release`. `instance/projection.toml` carries the profiles, their documentation roots, the managed and adopted pairs, the canon templates, and the sentinels, and the engine reads it from whichever bundle it was asked about. `payload_schema` is the protocol version between the two, and the engine declares the inclusive range it decodes.

Releases published before the declaration cannot acquire one, because a crate is immutable. A finite catalog, audited against the registry index and the published archives, supplies their facts as virtual metadata. Its floor is where the archives stop carrying the seed root, and a release below it is unavailable rather than guessed at. The catalog closes the day schema 1 ships.

Enforced by `bundle:a-release-declares-what-it-lands` and `bundle:a-pre-schema-release-is-cataloged-or-unavailable`.

## Consequences

- Good: every schema-one release is a witness the engine can read.
- Good: the declaration is one file a reviewer can read, rather than a table in code.
- Bad: the engine carries a compatibility catalog for the pre-schema interval.
- Bad: a projection that no longer parses is a build defect rather than a compile error.

## Status

Superseded by [ADR-render-the-candidate-this-binary-carries](./ADR-render-the-candidate-this-binary-carries.md)
