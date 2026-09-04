# Consume release-kit through the flake input

## Context and Problem Statement

The devshell carried `rk` through a hand-owned `fetchCrate` derivation in `nix/rk.nix`: a `version`, a source hash, and a `cargoHash`, moved by `nix-update`, which writes the version before it resolves either hash — an interruption bricked the pin. release-kit now packages itself; its tagged flake exposes `packages.<system>.default`. The question is where the pin lives and what a bump touches.

## Considered Options

- Pin release-kit as a flake input at its release tag — chosen.
- Keep the `fetchCrate` derivation and its `nix-update` machinery — rejected: two hashes discoverable only by building, a bricked-pin window, and a re-exported package attribute existing only for the updater.
- The input without `inputs.nixpkgs.follows` — rejected, deliberately: a second nixpkgs node in the lock is cheaper than nothing, but with `follows` this consumer owns the compatibility of rk against its own nixpkgs, and the bump script's devshell build plus CI's `nix develop` prove it on every move.

## Decision Outcome

The fact-owner table shrinks to two rows and nothing else names an rk version: the version is the tag in the input URL in `flake.nix`, and the content pin is the `release-kit` node in `flake.lock`. `scripts/rk-bump.sh` keeps its lock and its snapshot envelope, generalized from one file to two — flake.nix and flake.lock snapshot together, restore together — and swaps its middle for a pin-line rewrite plus `nix flake update release-kit` plus a devshell build. `nix/rk.nix`, the hashes, the `watch_file` for the pin, and the `nix-update` invocation are deleted.

## Consequences

- Good: a bump is a two-file diff needing no build to discover anything; the tree carries no `cargoHash`; the source is the tag the provider gates rather than a crate that may lag it.
- Bad: with `follows`, a nixpkgs bump here can break an rk build that is green upstream; the bump-time build is the fence, and the failure lands inside the bump's envelope.

## Status

Implemented: `flake.nix`, `flake.lock`, `.envrc`, `scripts/rk-bump.sh`, `tests/rk-bump.bats`.
