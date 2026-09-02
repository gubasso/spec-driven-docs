# Adopt the release-kit trunk convention

## Context and Problem Statement

The two-branch flow of ADR-cut-the-release-from-master gated the publish behind a develop-to-master pull request, at the cost of two pull requests and a back-merge per release, a merge-commit history, and a bespoke gate job only this repository maintained. release-kit packages the same guarantees — a version file that leads, a bot-maintained release request, a protected trunk, attested artifacts — as a landed, upgradable convention with its own CLI and checks.

## Considered Options

- `Adopt release-kit's trunk convention` — chosen.
- `Keep the two-branch gate` — rejected: bespoke automation duplicates what the landed payload maintains, and drifts from it.
- `Adopt the payload but keep develop` — rejected: the single-trunk invariant is what the landed protections and checks assert; a second long-lived branch fails them.

## Decision Outcome

Chosen option: `Adopt release-kit's trunk convention` — the landed payload maintains what this repository previously hand-built, and `rk upgrade` keeps it current.

`master` is the only long-lived branch and the default; work reaches it through squash-merged pull requests whose titles a required check holds to scoped Conventional Commits. release-plz maintains one release pull request against the trunk, and merging that request is the release decision: it tags and publishes over OIDC, and no other merge does, because the release half keys on the bot's own branch. cargo-dist builds every artifact and attests it in its host phase. The canon-manifest alignment survives as this repository's own workflow beside the landed payload. The gate pull request, the pinned `release/v<version>` cut, and the back-merge are retired.

Enforced by `release:a-tag-derives-from-the-version-file`.

## Consequences

- Good: one pull request per release, a linear trunk, and protections and hooks a payload upgrade maintains rather than hand-kept scripts.
- Good: every downloadable artifact carries a GitHub artifact attestation.
- Bad: the main checkout commits nothing under worktree mode, so deliberate main-checkout surgery needs the documented SKIP escape.
- Bad: files release-kit owns can no longer be hand-tuned; a local edit surfaces as drift in `rk status --check`.

## Status

Accepted — supersedes [ADR-cut-the-release-from-master](./ADR-cut-the-release-from-master.md).
