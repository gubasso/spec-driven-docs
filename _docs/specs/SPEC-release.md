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

A delivered gate MUST select its input under the instance's documentation root, by a filename shape this convention defines, or by resolving its own scope, and MUST NOT select by file type alone.

#### Scenario: Another tool renders a file at the project root

- GIVEN a project that installs this convention beside a tool that renders and hashes a file at the project root
- WHEN a gate selecting by file type alone reads that rendered file and reports a finding
- THEN neither tool is wrong about the file it owns, and the project owner carries a hand-patched exclusion that the next upgrade re-renders over

Verify: `pre-commit run cargo-test --all-files`

### `release:the-canon-record-describes-its-tree` — The canon record describes its tree

This repository is an instance of itself, so its committed instance record MUST name and hash the files the same commit carries.

#### Scenario: An adopted spec is edited and the record is not regenerated

- GIVEN a requirement reworded in `_docs/specs/` without running `sdd self-manifest`
- WHEN the release checks run
- THEN they fail naming the file, because `sdd verify` reports an adopted edit as a note every other instance is entitled to carry

Verify: `pre-commit run cargo-test --all-files`

### `release:the-rk-pin-has-two-facts-and-one-mover` — The rk pin has two facts and one mover

The devshell's pinned release-workflow CLI MUST have two facts and one mover: its version is the tag in its flake input URL in `flake.nix`, and its content is that input's node in `flake.lock`. `rk devshell sync`, invoked from `.envrc`, is the only thing that moves either. This project MUST carry no second mechanism over those files, because their transaction belongs to the CLI's own verb and two movers undo each other.

#### Scenario: A second mechanism rewrites the pin

- GIVEN a script or a recipe in this project that rewrites that input URL or its lock node
- WHEN the devshell wiring is judged
- THEN it is reported as a leftover of a predecessor mechanism, because one project runs one mover

Verify: `rk devshell status --target . --json` reports `ready` and an empty `leftovers` list

### `release:third-party-notices-travel-with-the-payload` — Third-party notices travel with the payload

The release MUST carry a third-party notice naming every third-party source the payload derives from, its upstream license, and its resolved revision, and `sdd license --third-party` MUST print it.

#### Scenario: A consumer installs the binary and asks for the terms

- GIVEN a writing-style source read at a resolved revision
- WHEN a consumer runs `sdd license --third-party`
- THEN the notice prints byte-identical to `THIRD_PARTY_NOTICES.md` and names the source's upstream MIT terms and resolved object ID. The canon test proves the notice appears in the packaged crate

Verify: `pre-commit run cargo-test --all-files`
