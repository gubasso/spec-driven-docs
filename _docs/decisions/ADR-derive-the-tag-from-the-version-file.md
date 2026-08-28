# Derive the tag from the version file

## Context and Problem Statement

Two artifacts name a release and are read by different consumers. `upgrade.sh` reads `VERSION` from a canon checkout, and a pre-commit consumer pins a git tag in `rev:`. Either can be authored by hand, and a project where both are authored has no answer to which one is right when they disagree.

## Considered Options

- `VERSION is the source of truth and the tag derives from it` — chosen.
- `The tag is the source of truth` — rejected: a tag is repository metadata, and an offline checkout with no git history has to know its own version.
- `Both authored, reconciled at review` — rejected: the disagreement is silent until a consumer resolves a pin, which is the worst place to discover it.

## Decision Outcome

Chosen option: `VERSION is the source of truth and the tag derives from it` — a file travels with the tree, and self-contained offline verification is what this distribution promises.

`scripts/release.sh` computes `v<VERSION>`, refuses a dirty tree, and refuses a release with no migration guide into it. The commit gate holds the file agreement and refuses to author further under a version already tagged. CI re-checks the name at the tagged commit.

Enforced by `release:versions-are-semantic-and-aligned`, `release:a-tag-derives-from-the-version-file`, and `release:a-released-version-is-not-re-authored`.

## Consequences

- Good: one value to change, and the derived name cannot disagree with it.
- Good: an offline instance reports its version without git.
- Bad: tagging requires a committer identity and a clean tree, so it cannot be scripted into an arbitrary CI job.
- Bad: tag immutability is a forge setting, not a repository check, so the guide asks for a ruleset that this repository cannot verify.

## Status

Accepted
