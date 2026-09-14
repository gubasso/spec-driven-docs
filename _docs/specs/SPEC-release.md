# Release Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`release:versions-are-semantic-and-aligned` — Versions are semantic and aligned](#releaseversions-are-semantic-and-aligned--versions-are-semantic-and-aligned)
  - [`release:a-tag-derives-from-the-version-file` — A tag derives from the version file](#releasea-tag-derives-from-the-version-file--a-tag-derives-from-the-version-file)
  - [`release:a-released-version-is-not-re-authored` — A released version is not re-authored](#releasea-released-version-is-not-re-authored--a-released-version-is-not-re-authored)
  - [`release:license-declares-both-halves` — The license declares both halves](#releaselicense-declares-both-halves--the-license-declares-both-halves)
  - [`release:the-delivered-gate-set-is-declared-once` — The delivered gate set is declared once](#releasethe-delivered-gate-set-is-declared-once--the-delivered-gate-set-is-declared-once)
  - [`release:a-canon-gate-is-not-delivered` — A canon gate is not delivered](#releasea-canon-gate-is-not-delivered--a-canon-gate-is-not-delivered)
  - [`release:a-delivered-gate-reads-what-the-convention-owns` — A delivered gate reads what the convention owns](#releasea-delivered-gate-reads-what-the-convention-owns--a-delivered-gate-reads-what-the-convention-owns)
  - [`release:the-canon-record-describes-its-tree` — The canon record describes its tree](#releasethe-canon-record-describes-its-tree--the-canon-record-describes-its-tree)
  - [`release:the-rk-pin-has-two-facts-and-one-mover` — The rk pin has two facts and one mover](#releasethe-rk-pin-has-two-facts-and-one-mover--the-rk-pin-has-two-facts-and-one-mover)
  - [`release:third-party-notices-travel-with-the-payload` — Third-party notices travel with the payload](#releasethird-party-notices-travel-with-the-payload--third-party-notices-travel-with-the-payload)
  - [`release:every-release-declares-what-it-asks` — Every release declares what it asks](#releaseevery-release-declares-what-it-asks--every-release-declares-what-it-asks)
  - [`release:the-binary-builds-for-every-declared-target` — The binary builds for every declared target](#releasethe-binary-builds-for-every-declared-target--the-binary-builds-for-every-declared-target)
  - [`release:a-machine-scope-recipe-drops-a-session-variable` — A machine-scope recipe drops a session variable](#releasea-machine-scope-recipe-drops-a-session-variable--a-machine-scope-recipe-drops-a-session-variable)

<!--TOC-->

## Purpose

Rules governing how this repository cuts a release. They cover which artifact states the version, how a tag derives from it, what a release carries, what a delivered gate reaches, and how the license splits. No instance adopts this spec. A cargo test that never ships verifies every rule here, so an instance holding these rules holds rules it cannot run.

## Requirements

### `release:versions-are-semantic-and-aligned` — Versions are semantic and aligned

`Cargo.toml` states the release identity as one semantic version, and every other artifact naming a version MUST carry that value.

#### Scenario: A second artifact names a different version

- GIVEN `Cargo.toml` reads 0.3.0 and the instance manifest reads 0.2.0
- WHEN the canon test suite runs
- THEN it fails and names `Cargo.toml` as the value to correct toward, not the disagreement

Verify: `pre-commit run cargo-test --all-files`

### `release:a-tag-derives-from-the-version-file` — A tag derives from the version file

A release tag MUST be `v<version>` for the `Cargo.toml` version at the commit it names, and MUST come from the release automation, never by hand.

#### Scenario: A maintainer tags by hand

- GIVEN a working tree whose `Cargo.toml` reads 0.3.0
- WHEN a tag named `v0.4.0` is pushed
- THEN the tag ruleset rejects it, because the tag resolves consumers to a version the tree does not hold

Verify: reviewer confirms every `v*` tag was created by the release automation on `master`, never authored by hand

### `release:a-released-version-is-not-re-authored` — A released version is not re-authored

If a version is published, then further changes MUST ship as a new version. A published version is never re-authored.

#### Scenario: A fix lands after a version is published

- GIVEN a consumer pinned to `rev: v0.3.0` or the published crate
- WHEN a defect in 0.3.0 needs correcting
- THEN the fix merges forward and the automation proposes 0.3.1. This is because the registry refuses a second 0.3.0 and a moved tag serves two payloads under one name

Verify: reviewer confirms corrections are cut forward as a new version, never by retagging

### `release:license-declares-both-halves` — The license declares both halves

The release MUST carry a named license file for the method and one for the distribution. The root `LICENSE` MUST name both. The crate metadata MUST carry the combined SPDX expression.

#### Scenario: A project installs the binary without the method

- GIVEN a target that installs the hooks and the verifier
- WHEN a reader opens `LICENSE` or the crate metadata to learn the terms
- THEN one identifier covers what was installed and the other stays with what was left behind

Verify: `pre-commit run cargo-test --all-files`

### `release:the-delivered-gate-set-is-declared-once` — The delivered gate set is declared once

Every gate the release delivers MUST be declared in the one registry that the projection into an instance is rendered from.

#### Scenario: A gate reaches the payload but no wiring

- GIVEN a gate compiled into the binary and named by no pre-commit entry
- WHEN the instance runs its hooks
- THEN the gate exists and never runs. That is why the projection is rendered from the registry at install time rather than copied from a committed file that can fall behind it

Verify: `pre-commit run cargo-test --all-files`

### `release:a-canon-gate-is-not-delivered` — A canon gate is not delivered

A check of an invariant only this repository has MUST stay a canon-side test rather than reaching the projection an instance receives.

#### Scenario: An instance receives the release checks

- GIVEN a check holding the crate version against the instance manifest of the canon
- WHEN it is delivered to a knowledge base that cuts no release
- THEN the instance is gated on a process it does not run, so the boundary is asserted rather than assumed

Verify: `pre-commit run cargo-test --all-files`

### `release:a-delivered-gate-reads-what-the-convention-owns` — A delivered gate reads what the convention owns

A delivered gate MUST declare the subject paths it judges, and MUST NOT judge a path its resolved filter excludes. A subject path is one whose content the gate judges and which can appear in a finding; a support path, which the gate reads to know what to judge, is not filtered.

#### Scenario: A gate walks the whole repository

- GIVEN a gate that resolves its own scope by walking the repository from its root
- WHEN it reaches a path the project's declaration excludes
- THEN it judges nothing there, because every route a subject path takes passes through one filter, and a gate has no way to ask for a path the filter dropped

#### Scenario: Another tool renders a file at the project root

- GIVEN a project that installs this convention beside a tool that renders and hashes a file at the project root
- WHEN the project reserves that path in its declaration
- THEN no delivered gate judges it, and the managed block is rendered from the declaration rather than hand-patched, so the next upgrade carries the choice rather than re-rendering over it

Verify: `pre-commit run cargo-test --all-files`

### `release:the-canon-record-describes-its-tree` — The canon record describes its tree

This repository is an instance of itself, so its committed instance record MUST name and hash the files the same commit carries.

#### Scenario: An adopted spec is edited and the record is not regenerated

- GIVEN a requirement reworded in `_docs/specs/` without running `sdd self-manifest`
- WHEN the release checks run
- THEN they fail naming the file, because `sdd verify` reports an adopted edit as a note every other instance is entitled to carry

Verify: `pre-commit run cargo-test --all-files`

### `release:the-rk-pin-has-two-facts-and-one-mover` — The rk pin has two facts and one mover

The devshell's pinned release-workflow CLI MUST have two facts and one mover: its version is the tag in its flake input URL in `flake.nix`, and its content is that input's node in `flake.lock`. `rk self-depend sync`, invoked from `.envrc`, is the only thing that moves either. This project MUST carry no second mechanism over those files, because their transaction belongs to the CLI's own verb and two movers undo each other.

#### Scenario: A second mechanism rewrites the pin

- GIVEN a script or a recipe in this project that rewrites that input URL or its lock node
- WHEN the devshell wiring is judged
- THEN it is reported as a leftover of a predecessor mechanism, because one project runs one mover

Verify: `rk self-depend status --target . --json` reports `ready` and an empty `leftovers` list

### `release:third-party-notices-travel-with-the-payload` — Third-party notices travel with the payload

The release MUST carry a third-party notice naming every third-party source the payload derives from, its upstream license, and its resolved revision, and `sdd license --third-party` MUST print it.

#### Scenario: A consumer installs the binary and asks for the terms

- GIVEN a writing-style source read at a resolved revision
- WHEN a consumer runs `sdd license --third-party`
- THEN the notice prints byte-identical to `THIRD_PARTY_NOTICES.md` and names the source's upstream MIT terms and resolved object ID. The canon test proves the notice appears in the packaged crate

Verify: `pre-commit run cargo-test --all-files`

### `release:every-release-declares-what-it-asks` — Every release declares what it asks

Every release MUST add exactly one entry to the guidance ledger, selecting a guidance file or `none`. A release that changes a managed projection's bytes, drops an adopted seed, retires a rule ID, adds a declaration key, or widens a gate's judged set MUST NOT select `none`. Every step MUST name its kind, its destinations, its actor, and a prose body the payload carries, and every release MUST carry its compatibility declaration.

#### Scenario: A release retires a rule and declares nothing

- GIVEN a release whose diff retires a rule ID and whose ledger entry reads `none`
- WHEN the canon test suite runs
- THEN it fails naming the change, because an instance that takes the release has work the release did not describe

Verify: `cargo nextest run -E 'binary(canon)'`

### `release:the-binary-builds-for-every-declared-target` — The binary builds for every declared target

`dist-workspace.toml` MUST declare exactly one target, `x86_64-unknown-linux-gnu`, and exactly one installer, `shell`. `flake.nix` MUST bind exactly one Nix system, `x86_64-linux`, MUST map no output over a system set, and MUST key every system-keyed output by that one binding. The crate root MUST carry a `compile_error!` whose `#[cfg]` attribute refuses every operating system but Linux. No live document outside the decision records and the changelog MUST name a retired target triple or a retired installer artifact.

The ordinary required pull-request job compiles the declared target, so that compilation is the evidence this rule rests on, and no lexical scan of source text substitutes for it.

Each check below states what it holds, because a check that overstates its reach is the defect this rule was rewritten to remove.

- The release declaration is read as parsed data. A comment can carry a string a substring search accepts, so text matching cannot hold a declared list.
- The flake is read as authored text, and holds its shape rather than its evaluation. Evaluating it needs Nix on the test host, which the suite does not assume, so CI's `nix flake check` owns the evaluation and this check owns the shape.
- The crate refusal is matched as adjacent live lines, because `#[cfg]` governs the item that follows it and a commented-out block keeps its text.
- The retired-artifact list is finite and closed. It holds every artifact this project shipped and no longer ships. A document inventing support for a target the project never built is a different defect, and review catches that one.

#### Scenario: The declaration grows a second target

- GIVEN `dist-workspace.toml` declaring a target beside `x86_64-unknown-linux-gnu`
- WHEN the canon test suite runs
- THEN it fails naming the declaration, whatever the added triple is, because it compares the parsed target list rather than searching the file for a string

#### Scenario: The flake exposes a second system

- GIVEN `flake.nix` mapping its outputs over a system set, or keying an output by anything but the one system binding
- WHEN the canon test suite runs
- THEN it fails naming the key it found, because `nix flake check` without `--all-systems` checks the system it runs on and proves nothing about the others

#### Scenario: The crate refusal is commented out

- GIVEN `src/lib.rs` whose `compile_error!` block is prefixed with `//`
- WHEN the canon test suite runs
- THEN it fails, because the attribute no longer sits on a live macro invocation, even though both strings remain in the file

#### Scenario: A document keeps advertising a retired artifact

- GIVEN a README line naming a retired target triple or a retired installer artifact
- WHEN the canon test suite runs
- THEN it fails naming the file and the claim, because a support claim outlives the code that made it

Verify: `cargo nextest run -E 'binary(canon)'`

### `release:a-machine-scope-recipe-drops-a-session-variable` — A machine-scope recipe drops a session variable

The `install` and `uninstall` recipes install this checkout for the machine, so their skill step MUST run without the variable that relocates the Claude configuration directory. A session wrapper sets that variable per terminal, to an isolated directory it owns and seats its own links in, so an inherited value aims a machine-scope install at one terminal and refuses at the wrapper's link on the way. The verb itself MUST keep honouring the variable, which is what installing into a relocated directory needs.

#### Scenario: The recipe stops dropping the variable

- GIVEN an `install` or `uninstall` recipe whose skill step no longer drops the variable
- WHEN the canon test suite runs
- THEN it fails naming the recipe, because every other test proves the verb's relocation default on purpose and none of them would notice

Verify: `cargo nextest run -E 'binary(canon)'`
