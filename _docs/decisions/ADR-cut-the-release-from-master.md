# Cut the release from master

## Context and Problem Statement

Tagging and publishing ran on every push to `develop`, and a promote job fast-forwarded `master` afterwards. The irreversible step therefore fired on a merge GitHub never gated: `develop` takes direct pushes, so it can carry no required status check, and ADR-gate-the-release-merge-in-the-recipe put the gate in a shell recipe instead. A convention in a runbook is not enforcement, because a web-interface merge bypasses it, and `master` was a trailing mirror nobody reviewed.

## Considered Options

- `Gate the release at a develop-to-master pull request and release from master` — chosen.
- `Require the test check on develop` — rejected: a required check gates pushes as well as merges, so the direct push that opens a release stops working.
- `Order the release job behind the test job with needs:` — rejected: it sequences two jobs inside one run and leaves the merge that starts the run ungated.
- `Rebase or squash the develop-to-master pull request` — rejected: `master` would diverge from `develop` permanently, so every later pull request would re-show every commit.

## Decision Outcome

Chosen option: `Cut the release from master` — a branch rule GitHub applies is enforcement, and a recipe convention is not. release-plz still opens its version-bump pull request against `develop`, because `release-pr` always targets the branch it ran on, and merging it now publishes nothing. A second pull request, `develop` into `master`, carries a required `test` check; merging it as a merge commit runs `release-plz release` on `master`, which tags that commit and publishes over OIDC. `develop` then fast-forwards onto `master`, so the two never drift.

Enforced by `release:a-tag-derives-from-the-version-file`.

## Consequences

- Good: the release gate is a branch rule GitHub applies, not a recipe convention.
- Good: `master` is a reviewable release branch carrying its own diff, not a mirror.
- Bad: `master-protection` drops `required_linear_history`, because GitHub offers no fast-forward merge method.
- Bad: a release costs two pull requests and a back-merge, and skipping the back-merge leaves the tag unreachable from `develop`.

## Status

Accepted — supersedes [ADR-gate-the-release-merge-in-the-recipe](./ADR-gate-the-release-merge-in-the-recipe.md).
