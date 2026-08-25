# Distribute a single binary

## Context and Problem Statement

The vendored payload projected some thirty hashed files into every instance, and the manifest's whole guarantee was that those bytes still matched. A binary cannot be vendored the same way: its hash differs per platform, and committing it would put megabytes in every consumer's history. What does an instance hold, and what guarantees which gates ran?

## Considered Options

- `The consumer installs sdd; the manifest pins a version` — chosen.
- `Vendor the binary into the instance` — rejected: platform-specific hashes make the manifest unshareable, and the blob outweighs the instance.
- `Keep vendored scripts beside the binary` — rejected: two implementations of one gate set drift.

## Decision Outcome

Chosen option: `The consumer installs sdd; the manifest pins a version`. The managed set shrinks to the markdownlint configurations; adopted seeds still land on disk because the instance owns and edits them. The manifest moves to schema version 2, and `canon_version` replaces payload hashing as the answer to which gates ran: `sdd verify` fails when the binary is older than the instance and points at `sdd upgrade` when newer.

Upgrades no longer take a canon checkout: the newer binary carries the newer payload and its migration guides, walks them version by version, reinstalls, and prunes what it stopped managing. Remote pre-commit consumers build the binary from this repository via `language: rust`.

Enforced by `distribution:manifest-identifies-every-owned-file` and `distribution:instances-operate-offline`.

## Consequences

- Good: install, verify, and upgrade are one tool with no checkout and no network.
- Good: version skew between tool and instance is detected instead of silent.
- Bad: everyone who runs the hooks, including CI, must have `sdd` on the PATH.
- Bad: a fix to any gate ships only with a release, never by editing a vendored file.

## Status

Accepted
