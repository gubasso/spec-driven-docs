# Distribution Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`distribution:manifest-identifies-every-owned-file` — The manifest identifies every owned file](#distributionmanifest-identifies-every-owned-file--the-manifest-identifies-every-owned-file)
  - [`distribution:initialization-preserves-project-content` — Initialization preserves project content](#distributioninitialization-preserves-project-content--initialization-preserves-project-content)
  - [`distribution:instances-operate-offline` — Instances operate offline](#distributioninstances-operate-offline--instances-operate-offline)
  - [`distribution:upgrade-conflicts-are-atomic` — Upgrade conflicts are atomic](#distributionupgrade-conflicts-are-atomic--upgrade-conflicts-are-atomic)
  - [`distribution:versions-are-semantic-and-aligned` — Versions are semantic and aligned](#distributionversions-are-semantic-and-aligned--versions-are-semantic-and-aligned)
  - [`distribution:a-tag-derives-from-the-version-file` — A tag derives from the version file](#distributiona-tag-derives-from-the-version-file--a-tag-derives-from-the-version-file)
  - [`distribution:a-released-version-is-not-re-authored` — A released version is not re-authored](#distributiona-released-version-is-not-re-authored--a-released-version-is-not-re-authored)
  - [`distribution:a-release-carries-its-migration-guide` — A release carries its migration guide](#distributiona-release-carries-its-migration-guide--a-release-carries-its-migration-guide)
  - [`distribution:license-declares-both-halves` — The license declares both halves](#distributionlicense-declares-both-halves--the-license-declares-both-halves)
  - [`distribution:the-delivered-gate-set-is-declared-once` — The delivered gate set is declared once](#distributionthe-delivered-gate-set-is-declared-once--the-delivered-gate-set-is-declared-once)
  - [`distribution:a-canon-gate-is-not-delivered` — A canon gate is not delivered](#distributiona-canon-gate-is-not-delivered--a-canon-gate-is-not-delivered)

<!--TOC-->

## Purpose

Rules governing installation, ownership classes, offline verification, upgrades, licensing, and the
release: which artifact states the version, and how a tag derives from it.

## Requirements

### `distribution:manifest-identifies-every-owned-file` — The manifest identifies every owned file

The installer MUST record each installed file with its ownership class, destination, and SHA-256.

#### Scenario: An agent encounters a local edit

- GIVEN an installed file differs from its baseline
- WHEN the verifier reads the manifest
- THEN it distinguishes managed drift from adopted reconciliation

Verify: `pre-commit run instance-manifest --all-files`

### `distribution:initialization-preserves-project-content` — Initialization preserves project content

When a target is non-empty, the installer MUST preview its changes before writing any file.

#### Scenario: A repository has a hand-commented hook configuration

- GIVEN comments outside the managed markers
- WHEN initialization inserts its block
- THEN every outside comment remains byte-identical

Verify: `just test-instantiation`

### `distribution:instances-operate-offline` — Instances operate offline

The installed verifier MUST validate an instance without a network or canon checkout.

#### Scenario: The canon checkout is unavailable

- GIVEN a fully installed target
- WHEN its vendored verifier runs with `--offline`
- THEN it checks tools, hashes, rule IDs, and the marked integration block locally

Verify: `just test-instantiation`

### `distribution:upgrade-conflicts-are-atomic` — Upgrade conflicts are atomic

If a managed file differs from its installed hash, then the upgrader MUST abort without changing the target.

#### Scenario: One managed hook is edited locally

- GIVEN a valid installed instance with one managed edit
- WHEN an upgrade is requested
- THEN it lists the conflict and changes no target byte

Verify: `just test-upgrade`

### `distribution:versions-are-semantic-and-aligned` — Versions are semantic and aligned

`VERSION` states the release identity as one semantic version, and every other artifact naming a version MUST carry that value.

#### Scenario: A second artifact names a different version

- GIVEN `VERSION` reads 0.2.0 and the instance manifest reads 0.1.0
- WHEN the version gate runs
- THEN it fails and names `VERSION` as the value to correct toward, not the disagreement

Verify: `pre-commit run version-source-of-truth --all-files`

### `distribution:a-tag-derives-from-the-version-file` — A tag derives from the version file

A release tag MUST be `v<VERSION>` at the commit it names, and MUST be produced by `scripts/release.sh`.

#### Scenario: A maintainer tags by hand

- GIVEN a working tree whose `VERSION` reads 0.2.0
- WHEN a tag named `v0.3.0` is pushed
- THEN the release job rejects it, because the tag resolves consumers to a version the tree does not hold

Verify: `just test-release`

### `distribution:a-released-version-is-not-re-authored` — A released version is not re-authored

If a tag named `v<VERSION>` already exists, then the author MUST raise `VERSION` before committing further.

#### Scenario: A fix lands under a version already pinned

- GIVEN a consumer pinned to `rev: v0.2.0`
- WHEN a commit changes managed content while `VERSION` still reads 0.2.0
- THEN the gate fails, because shipping it needs the tag moved and a moved tag serves two payloads under one name

Verify: `pre-commit run version-source-of-truth --all-files`

### `distribution:a-release-carries-its-migration-guide` — A release carries its migration guide

Where a previous release exists, the release MUST carry exactly one `migrations/<previous>-to-<version>.md` before it is tagged.

#### Scenario: A version ships with no way into it

- GIVEN an instance installed at the previous version
- WHEN it upgrades to a release whose guide was never written
- THEN the upgrade aborts, so the guide is required at the tag rather than discovered by the consumer

Verify: `just test-release`

### `distribution:license-declares-both-halves` — The license declares both halves

The release MUST carry a named license file for the method and one for the distribution, and the root `LICENSE` MUST name both.

#### Scenario: A project vendors the distribution without the method

- GIVEN a target that installs the hooks and the verifier
- WHEN a reader opens `LICENSE` to learn the terms
- THEN one identifier covers what was installed and the other stays with what was left behind

Verify: `pre-commit run license-split --all-files`

### `distribution:the-delivered-gate-set-is-declared-once` — The delivered gate set is declared once

Every gate the release delivers MUST be declared in one file that both the projection into an instance and the manifest published to a consumer are rendered from.

#### Scenario: A gate reaches the payload but no wiring

- GIVEN a gate copied into an instance and named by no pre-commit entry
- WHEN the instance runs its hooks
- THEN the gate is hashed, executable, and never runs, so the declaration is what both deliveries derive from

Verify: `pre-commit run delivered-domain --all-files`

### `distribution:a-canon-gate-is-not-delivered` — A canon gate is not delivered

A gate checking an invariant that only this repository has MUST stay outside the delivered set, and MUST reach neither the projection nor the published manifest.

#### Scenario: An instance receives the release gates

- GIVEN a gate holding VERSION against the tag that names it
- WHEN it is projected into a knowledge base that cuts no release
- THEN the instance is gated on a process it does not run, so the boundary is asserted rather than assumed

Verify: `pre-commit run delivered-domain --all-files`
