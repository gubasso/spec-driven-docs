# Distribution Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`distribution:manifest-identifies-every-owned-file` — The manifest identifies every owned file](#distributionmanifest-identifies-every-owned-file--the-manifest-identifies-every-owned-file)
  - [`distribution:initialization-preserves-project-content` — Initialization preserves project content](#distributioninitialization-preserves-project-content--initialization-preserves-project-content)
  - [`distribution:instances-operate-offline` — Instances operate offline](#distributioninstances-operate-offline--instances-operate-offline)
  - [`distribution:upgrade-conflicts-are-atomic` — Upgrade conflicts are atomic](#distributionupgrade-conflicts-are-atomic--upgrade-conflicts-are-atomic)

<!--TOC-->

## Purpose

Rules governing installation, ownership classes, offline verification, and upgrades. The
distribution is one installed binary, `sdd`, that carries the payload; an instance adopts this
spec, and every rule here is one an instance can verify with the binary it runs. The release rules
the canon alone runs are stated in `SPEC-release.md`.

## Requirements

### `distribution:manifest-identifies-every-owned-file` — The manifest identifies every owned file

The installer MUST record each installed file with its ownership class, destination, and SHA-256, and the record MUST state which canon version produced it.

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

Verify: `cargo nextest run -E 'binary(cmd_init)'`

### `distribution:instances-operate-offline` — Instances operate offline

The installed binary MUST verify and upgrade an instance without a network or canon checkout.

#### Scenario: The canon repository is unreachable

- GIVEN a fully installed target
- WHEN `sdd verify` and `sdd upgrade` run with no network
- THEN they check hashes, the managed block, rule IDs, and the binary's own version against the manifest, from the payload the binary carries

Verify: `cargo nextest run -E 'binary(cmd_verify) + binary(cmd_upgrade)'`

### `distribution:upgrade-conflicts-are-atomic` — Upgrade conflicts are atomic

If a managed file differs from its installed hash, then the upgrader MUST abort without changing the target.

#### Scenario: One managed configuration is edited locally

- GIVEN a valid installed instance with one managed edit
- WHEN an upgrade is requested
- THEN it lists every conflict in one run and changes no target byte

Verify: `cargo nextest run -E 'binary(cmd_upgrade)'`
