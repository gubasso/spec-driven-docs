# Package the binary for Nix

## Context and Problem Statement

This repository builds `sdd` and consumes release-kit through a flake input, but published no package of its own: a Nix user could enter the devshell and could not install or run the binary from the flake. release-kit ships the packaging capability as an opt-in seed, `nix/package.nix`, which the landing withholds where the target owns its flake. The question is whether to take it, and where its build proof runs.

## Considered Options

- `Take the seed and wire it into the owned flake` — chosen.
- `Publish no Nix package` — rejected: the flake already exists and already pins the toolchain, so the package is a few lines over facts the tree carries, and its absence is the only reason `nix run` answers nothing.
- `Take the seed and its own nix workflow` — rejected: the landing withholds that workflow with the flake, and a second workflow would run outside the one job branch protection requires, so a broken package could merge green.

## Decision Outcome

Chosen option: `Take the seed and wire it into the owned flake` — `packages.default` calls `nix/package.nix` with a `rustPlatform` built from the toolchain in `rust-toolchain.toml`, so the package and the devshell compile with one compiler. `checks.package` and `checks.smoke` prove the build and that the built binary answers.

The proof runs in `ci.yml`, the job branch protection requires: `nix build .#default` beside `nix flake check`, because a flake check builds only the checks output.

## Consequences

- Good: `nix run`, `nix build`, and a flake reference all answer, from the same lock CI uses.
- Good: the packaging cannot go stale unnoticed, because its build blocks the merge.
- Bad: every pull request now compiles the crate twice, once under cargo and once under Nix.
- Bad: the flake advertises every default system while the runner proves `x86_64-linux` alone. A wider claim than the proof, kept because the devshell already carried it.

## Status

Implemented: `flake.nix`, `nix/package.nix`, `.github/workflows/ci.yml`, `Cargo.toml`.
