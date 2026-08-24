# Split the license by product

## Context and Problem Statement

This repository holds two products: a method written to be read, and a distribution written to be
installed and run. One license covering both asks a reader of the method and an operator of the
hooks to accept the same terms, and neither set of terms fits the other's use.

## Considered Options

- `CC BY 4.0 for the method and MIT for the distribution` — chosen.
- `CC BY 4.0 throughout` — rejected: an attribution obligation on a shell script a project vendors
  into its own hooks is a term no consumer of tooling expects.
- `MIT throughout` — rejected: the method is a knowledge product, and MIT says nothing about the
  attribution that reuse of prose should carry.
- `Dual license every file under both` — deferred: revisit if a consumer needs to relicense the
  method under software terms.

## Decision Outcome

Chosen option: `CC BY 4.0 for the method and MIT for the distribution` — the boundary already
governs ownership, routing, and upgrade, so licensing follows it rather than cutting a second one
across the same tree.

`LICENSE` states which directories fall on each side, and `LICENSE-CC-BY-4.0` and `LICENSE-MIT`
carry the terms.

Enforced by `distribution:license-declares-both-halves`.

## Consequences

- Good: a project installing the distribution takes on no attribution obligation for the files it
  vendors, and a reader quoting the method takes on one.
- Bad: an artifact that quotes the method inside a distribution file carries both terms at once, and
  only the prose says so.
- Bad: a repository with no single `LICENSE` body is reported as `Other` by license detectors.

## Status

Accepted
