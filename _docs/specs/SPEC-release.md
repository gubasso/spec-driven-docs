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
  - [`release:the-payload-record-describes-its-tree` — The payload record describes its tree](#releasethe-payload-record-describes-its-tree--the-payload-record-describes-its-tree)
  - [`release:the-canon-is-an-ordinary-instance` — The canon is an ordinary instance](#releasethe-canon-is-an-ordinary-instance--the-canon-is-an-ordinary-instance)
  - [`release:the-canon-runs-its-previous-release` — The canon runs its previous release](#releasethe-canon-runs-its-previous-release--the-canon-runs-its-previous-release)
  - [`release:the-rk-pin-has-two-facts-and-one-mover` — The rk pin has two facts and one mover](#releasethe-rk-pin-has-two-facts-and-one-mover--the-rk-pin-has-two-facts-and-one-mover)
  - [`release:third-party-notices-travel-with-the-payload` — Third-party notices travel with the payload](#releasethird-party-notices-travel-with-the-payload--third-party-notices-travel-with-the-payload)
  - [`release:every-release-declares-what-it-asks` — Every release declares what it asks](#releaseevery-release-declares-what-it-asks--every-release-declares-what-it-asks)
  - [`release:the-binary-builds-for-every-declared-target` — The binary builds for every declared target](#releasethe-binary-builds-for-every-declared-target--the-binary-builds-for-every-declared-target)
  - [`release:a-machine-scope-recipe-drops-a-session-variable` — A machine-scope recipe drops a session variable](#releasea-machine-scope-recipe-drops-a-session-variable--a-machine-scope-recipe-drops-a-session-variable)
  - [`release:the-devshell-supplies-every-project-tool` — The devshell supplies every project tool](#releasethe-devshell-supplies-every-project-tool--the-devshell-supplies-every-project-tool)
  - [`release:the-local-hooks-run-every-check-of-ci` — The local hooks run every check of CI](#releasethe-local-hooks-run-every-check-of-ci--the-local-hooks-run-every-check-of-ci)
- [Unenforced](#unenforced)

<!--TOC-->

## Purpose

Rules governing how this repository cuts a release. They cover which artifact states the version, how a tag derives from it, what a release carries, what a delivered gate reaches, and how the license splits. No instance adopts this spec. A cargo test that never ships verifies every rule here, so an instance holding these rules holds rules it cannot run.

## Requirements

### `release:versions-are-semantic-and-aligned` — Versions are semantic and aligned

`Cargo.toml` states the release identity as one semantic version, and every other artifact naming the version of this tree MUST carry that value.

The instance record names the sdd that landed the canon's own instance, which is the pinned previous release, so it names no version of this tree.

#### Scenario: A second artifact names a different version

- GIVEN `Cargo.toml` reads 0.3.0 and the payload record reads 0.2.0
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

### `release:the-payload-record-describes-its-tree` — The payload record describes its tree

The tree MUST describe its payload in `instance/payload-manifest.json`, which carries the crate version of `Cargo.toml` and the hash of each payload source under the declared roots, and which `cargo run -q -- self-manifest` writes.

The payload record has its own schema id and holds no instance field, and nothing compares it with an instance record. The tree builds it as it builds its binary, so a checkout whose crate version differs from the pinned sdd still gets a correct record, because the pinned sdd never writes it.

#### Scenario: A payload source is edited and the record is not regenerated

- GIVEN a requirement reworded in an adoptable spec under `instance/payload/` without running `just manifest`
- WHEN the release checks run
- THEN they fail naming the file, because the record no longer describes the bytes the binary embeds

Verify: `cargo nextest run -E 'test(the_payload_record_describes_its_tree)'`

### `release:the-canon-is-an-ordinary-instance` — The canon is an ordinary instance

`sdd verify` MUST hold this repository's instance to the obligations of every instance, with no selection of a canon layout, and `sdd init` MUST land in this checkout as it lands in any repository.

The payload sources sit apart from the canon's own instance, under `instance/payload/` where a source would otherwise double as a destination. The canon's own documentation root holds landed copies and the documents only the canon carries. The checks of the payload are tree tests.

#### Scenario: The canon's instance loses a skill entry

- GIVEN this repository's instance with one landed skill file and its record entry removed
- WHEN `sdd verify --target .` runs
- THEN it fails naming the destination, whatever the payload record says

Verify: `cargo nextest run -E 'test(verify_holds_the_canon_to_the_instance_obligations) + test(init_lands_in_the_canon_checkout)'`

### `release:the-canon-runs-its-previous-release` — The canon runs its previous release

The devshell MUST carry a released sdd, pinned by the `spec-driven-docs` input of `flake.nix`, and the rendered hooks and the recipes that operate an instance MUST run that sdd, while the sdd built from the tree runs only where the tree is under test or builds its own artifact.

A compiler builds itself with an earlier release of itself, and this repository lands and upgrades its own instance the same way. One tree test runs every gate of the tree over the canon's documents, with a negative control, so the gates under development still judge the canon. After a release publishes, one change moves the pin and runs `sdd upgrade` on this repository, as in every consumer. The first conversion of the canon into an ordinary instance runs once with the sdd of the tree, because no released sdd can land it yet. Where the pinned release cannot verify that instance, the hooks keep the sdd of the tree until the first pin move after the release that ships the conversion, which ends the exception.

#### Scenario: A hook runs the sdd of the tree

- GIVEN a pin that names a release able to verify the canon, and a managed hook entry that runs `cargo run -q --` instead of `sdd`
- WHEN the test suite runs
- THEN it fails naming the entry, because the instance would be judged by a binary no release produced

Verify: `cargo nextest run -E 'test(the_devshell_sdd_is_the_pinned_release) + test(the_instance_recipes_use_the_pinned_sdd) + test(the_tree_gates_pass_on_the_canon)'`

### `release:the-rk-pin-has-two-facts-and-one-mover` — The rk pin has two facts and one mover

The devshell's pinned release-workflow CLI MUST have two facts and one mover: its version is the tag in its flake input URL in `flake.nix`, and its content is that input's node in `flake.lock`. `rk self-depend sync` is the only thing that moves either. This project MUST carry no second mechanism over those files, because their transaction belongs to the CLI's own verb and two movers undo each other.

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

A release that changes a managed projection's bytes, drops an adopted seed, retires a rule ID, adds a declaration key, or widens a gate's judged set MUST say so in its changelog entry, and the setup skill MUST carry the migration procedure that lands it.

The instruction an instance takes travels in two places a reader already opens: the changelog says what changed, and the skill says what to do about it. Neither is a protocol one release writes for another to parse.

#### Scenario: A release retires a rule ID

- GIVEN a release whose diff retires a rule ID
- WHEN an operator upgrades an instance onto it
- THEN the changelog entry names the retirement and the setup skill carries the step that lands it, because an instance that takes a release has work somebody must describe

Verify: `cargo nextest run -E 'binary(canon)'`

### `release:the-binary-builds-for-every-declared-target` — The binary builds for every declared target

`dist-workspace.toml` MUST declare exactly one target, `x86_64-unknown-linux-gnu`, and exactly one installer, `shell`. `flake.nix` MUST bind exactly one Nix system, `x86_64-linux`, MUST map no output over a system set, and MUST key every system-keyed output by that one binding. The crate root MUST carry a `compile_error!` whose `#[cfg]` attribute refuses every operating system but Linux. No live document outside the decision records and the changelog MUST name a retired target triple or a retired installer artifact.

The ordinary required pull-request job compiles the declared target, so that compilation is the evidence this rule rests on, and no lexical scan of source text substitutes for it.

Each check below states what it holds, because a check that overstates its reach is the defect this rule was rewritten to remove.

- The release declaration is read as parsed data. A comment can carry a string a substring search accepts, so text matching cannot hold a declared list.
- The flake's system set is held by evaluation, in the required job, by `scripts/check-one-system.sh`. The canon test reads `flake.nix` as text and holds one canonical authored form only: Nix spells the same tree several ways, so a nested attribute set declares an output no text scan sees. The cargo test keeps the authored file readable and fails fast without Nix; the evaluated assertion is what holds the boundary. That assertion reads the flake's own outputs rather than the JSON `nix flake show` prints, whose schema differs between Nix versions.
- The crate refusal is matched as adjacent live lines, with no further `cfg` above them. A commented-out block keeps its text, and a form carrying several `cfg` attributes is removed when any predicate is false.
- The retired-artifact list is finite and closed. It holds every artifact this project shipped and no longer ships. A document inventing support for a target the project never built is a different defect, and review catches that one.

#### Scenario: The declaration grows a second target

- GIVEN `dist-workspace.toml` declaring a target beside `x86_64-unknown-linux-gnu`
- WHEN the canon test suite runs
- THEN it fails naming the declaration, whatever the added triple is, because it compares the parsed target list rather than searching the file for a string

#### Scenario: The flake exposes a second system

- GIVEN `flake.nix` exposing an output for a system beside `x86_64-linux`, however it is spelled
- WHEN the required job runs
- THEN `scripts/check-one-system.sh` names the systems it found and the job fails, because `nix flake check` without `--all-systems` checks the system it runs on and proves nothing about the others

#### Scenario: The crate refusal is disabled

- GIVEN `src/lib.rs` whose `compile_error!` block is commented out, or carries a second `cfg` above the negated-Linux one
- WHEN the canon test suite runs
- THEN it fails, because the attribute no longer governs a live macro invocation, even though every string remains in the file

#### Scenario: A document keeps advertising a retired artifact

- GIVEN a README line naming a retired target triple or a retired installer artifact
- WHEN the canon test suite runs
- THEN it fails naming the file and the claim, because a support claim outlives the code that made it

Verify: `cargo nextest run -E 'binary(canon)'`

### `release:a-machine-scope-recipe-drops-a-session-variable` — A machine-scope recipe drops a session variable

The `uninstall` recipe cleans the home of the skill copies earlier releases installed, so its skill step MUST run without the variable that relocates the Claude configuration directory. A session wrapper sets that variable per terminal, to an isolated directory it owns and seats its own links in, so an inherited value aims a machine-scope cleanup at one terminal and misses the copies under the user's own directory. The verb itself MUST keep honouring the variable, which is what a cleanup of a relocated directory needs.

#### Scenario: The recipe stops dropping the variable

- GIVEN an `uninstall` recipe whose skill step no longer drops the variable
- WHEN the canon test suite runs
- THEN it fails naming the recipe, because every other test proves the verb's relocation default on purpose and none of them would notice

Verify: `cargo nextest run -E 'binary(canon)'`

### `release:the-devshell-supplies-every-project-tool` — The devshell supplies every project tool

The devshell of `flake.nix` MUST supply the language toolchain, `just`, and every tool that a recipe, a `language: system` hook, or a `language: script` hook calls from `PATH`. The hook runner supplies the tools of a hook whose language gets an environment that the runner builds. The host supplies only `nix` and `direnv`, which enter the devshell, the hook runner `prek`, and the base tools: `git`, `bash`, `awk`, `sed`, `grep`, and the coreutils. A CI job MUST install no project tool outside the devshell, and a job that compiles this project MUST compile it in the devshell. The hook runner is a host tool, so a CI job installs it with an action pinned by commit SHA, and runs it in the devshell.

A workflow that a release tool generates follows its generator: `release-plz.yml` and `release.yml` only release, and their generators own how they install a toolchain.

#### Scenario: A job installs its own toolchain

- GIVEN a workflow job that installs Rust with a toolchain action and then runs `cargo run`
- WHEN the change is reviewed
- THEN it is refused, because the job compiles with a compiler that neither `rust-toolchain.toml` nor the devshell pins, so a green local run says nothing about it

Verify: reviewer confirms that no job of a workflow this repository authors installs a project tool outside `nix develop`

### `release:the-local-hooks-run-every-check-of-ci` — The local hooks run every check of CI

If a push passed the local git hooks, CI MUST pass. The pre-commit stage and the pre-push stage together MUST run every check of `ci.yml`, the costly ones at pre-push, and the pre-push stage MUST NOT run a hook of the pre-commit stage again. `ci.yml` runs the two stages in its `test` job, and beside `test` only the CI-only checks this rule lists. The first step of `test` fails when a job that `test` needs did not pass, and checks nothing else.

A check that only CI can run is listed here with its reason. The list has two entries, the `dist-plan` and `dist-build` jobs, which `test` needs and which run every `dist` command in the devshell. `dist-plan` checks that dist runs at the version `dist-workspace.toml` pins, that `release.yml` is what that dist generates, and that the configuration plans a release, and it computes the build matrix. `dist-build` builds the release archive of every target of `dist-workspace.toml` before a merge, so a target that fails to build fails the pull request, not the release after its tag exists. rk's `dist-target-unbuilt` invariant judges from the workflow alone that a job the gate needs builds every target, so no hook can satisfy it, and the operator keeps the dist checks out of the local hooks. The hooks can run more than CI: CI skips `lychee-system`, which checks external links, because an external site answers a CI runner with a block or a timeout that it does not give a developer host. These workflows check nothing that a push starts, so they are outside the rule: `release-plz.yml`, `release.yml`, and `align-manifest.yml` release, and `pr-title.yml` judges a pull request title, which the commit-msg hooks judge on every commit.

#### Scenario: A check runs only in CI

- GIVEN a step of `ci.yml` that no hook runs
- WHEN a push passes its hooks
- THEN CI can still fail on that step, so the step moves into a hook, at pre-push when it is costly, or this rule lists it with its reason

Verify: reviewer confirms that every step of the `test` job of `ci.yml` after its first runs a hook stage of `.pre-commit-config.yaml` or sets up the runner, and that `ci.yml` holds no job beside `test` and the CI-only checks this rule lists

## Unenforced

No command decides these rules, or a command decides only part of each. A reviewer confirms the rest.

| Rule                                               | Reviewer confirms                                                                          |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `release:a-tag-derives-from-the-version-file`      | Every `v*` tag came from the release automation on `master`                                |
| `release:a-released-version-is-not-re-authored`    | Each correction ships as a new version, never as a moved tag                               |
| `release:the-devshell-supplies-every-project-tool` | No job of a workflow this repository authors installs a project tool outside `nix develop` |
| `release:the-local-hooks-run-every-check-of-ci`    | Every job of `ci.yml` runs a hook stage or is a check the rule lists                       |
