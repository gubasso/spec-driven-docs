# Run the canon on its previous release

## Context and Problem Statement

This repository described its payload in its instance record, refused `sdd init` on its own checkout, and ran the gates of its tree as hooks. `sdd verify` held it to a canon layout, and several paths were both a payload source and an instance destination. Once every command checks the instance version, a record that the tree rewrites at each release no longer fits an ordinary instance.

## Considered Options

- `the canon as an ordinary instance of a pinned earlier release` — chosen.
- `the canon as a self-describing record that no installer produces` — rejected: it keeps a second set of obligations that no instance shares.
- `an exemption of the canon from the gate` — rejected: the canon then never meets the gate it ships.

## Decision Outcome

Chosen option: `the canon as an ordinary instance of a pinned earlier release` — a compiler builds itself with an earlier release of itself. rustc compiles its source with a `stage0` compiler, pinned in the file `src/stage0`, and Go builds with an earlier Go named by `GOROOT_BOOTSTRAP` (both read on 2026-11-13).

The payload sources that doubled as destinations move under `instance/payload/`. The description of the payload moves to `instance/payload-manifest.json`, which the tree writes. The devshell pins a released sdd, and the hooks and the instance recipes run it. One tree test runs every gate of the tree over the canon's documents, with a negative control. The first conversion runs once with the sdd of the tree, because no release can land the canon yet.

Enforced by `release:the-payload-record-describes-its-tree`, `release:the-canon-is-an-ordinary-instance`, and `release:the-canon-runs-its-previous-release`.

## Consequences

- Good: the canon meets the obligations of every instance.
- Good: the payload record cannot drift from the binary that embeds it.
- Bad: a change to a gate reaches the canon's hooks one release later, and the tree test covers the gap.
- Bad: each release needs one more change that moves the pin and upgrades the canon.

## Status

Accepted
