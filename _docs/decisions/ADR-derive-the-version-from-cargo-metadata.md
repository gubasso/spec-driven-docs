# Derive the version from cargo metadata

## Context and Problem Statement

The distribution became a crate, and a crate already names its version in `Cargo.toml` — the value cargo builds into the binary and crates.io publishes under. A separate `VERSION` file would be a second authored artifact with no answer to which one is right when they disagree, which is the defect the file originally existed to prevent.

## Considered Options

- `Cargo.toml is the source of truth; release-plz derives everything` — chosen.
- `Keep VERSION beside Cargo.toml` — rejected: two authored values reconciled by a gate is strictly worse than one authored value.
- `Cargo.toml authored, tags cut by hand` — rejected: a hand-authored tag can name a version the tree does not hold, and moving one serves two payloads under one name.

## Decision Outcome

Chosen option: `Cargo.toml is the source of truth; release-plz derives everything`. Conventional Commits drive the bump, release-plz maintains the changelog and opens the release pull request, and merging that request is the one human release decision: the `v<version>` tag, the crates.io publish over trusted publishing, and the installer builds all follow from it. The binary reports the built-in version, so an offline instance still knows its canon without git.

The canon's own manifest must carry the crate version, held by a canon test, and exactly one migration guide must lead into it — the checks that replaced the version gate.

Enforced by `release:versions-are-semantic-and-aligned`, `release:a-tag-derives-from-the-version-file`, and `release:a-release-carries-its-migration-guide`.

## Consequences

- Good: one authored value; the tag, changelog, and publish cannot disagree with it.
- Good: version discipline moves from a commit gate into automation that cannot forget.
- Bad: releasing depends on forge automation and its credentials, not on a local script.
- Bad: commit messages become load-bearing; a mistyped type proposes the wrong bump.

## Status

Accepted
