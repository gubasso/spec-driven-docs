# Linux is the only supported target

## Context and Problem Statement

The release declared five Rust targets and two installers, and the Nix flake mapped over the default system set. Nothing in the ordinary required path compiled four of them: the only job that did ran on a tag, after the release decision. Two promises rested on that gap. The plan store called its permission enforcement `owner_only` and returned `Ok` off Unix, so the mode said nothing on three declared targets. A canon test scanned nearby source text for a `#[cfg]` and stood in for the compiler, proving no compilation either way. Release v0.9.0 split across platforms, and v0.9.1 existed to repair it.

## Considered Options

- `one target` — chosen.
- `five targets, each compiled in the required path` — rejected: four build matrices and three permission implementations for platforms with no user here and no way to test behavior against.
- `five targets, keeping the lexical guard` — rejected: the guard proves no compilation, so the promise stays unheld and the v0.9.0 repair recurs.
- `more architectures` — deferred: revisit when a request names a platform and someone can run the suite there.

## Decision Outcome

Chosen option: `one target` — `x86_64-unknown-linux-gnu` and the `x86_64-linux` Nix output, because the required pull-request job compiles it on every run, which makes the compiler the evidence. Source portability outside it is not a product promise: the crate root refuses another operating system at compile time rather than building and then keeping promises nothing proves.

Enforced by `release:the-binary-builds-for-every-declared-target`.

## Consequences

- Good: one permission implementation, one build, one Nix output, and a support claim the required check holds.
- Good: the owner-only promise and the code keeping it agree on every supported target.
- Bad: a build from source elsewhere is refused outright rather than succeeding quietly and unsupported.
- Bad: adding a target back costs a decision record, not a line in a list.

## Status

Implemented.

Enacted by `dist-workspace.toml`, `flake.nix`, the refusal in `src/lib.rs`, and the canon tests `the_declaration_names_one_target_and_the_crate_refuses_the_rest`, `no_live_document_advertises_an_unsupported_platform`, and `the_generated_workflow_names_no_unsupported_artifact`.
