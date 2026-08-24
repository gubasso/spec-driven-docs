# Distribution Specification

## Purpose

Rules governing installation, ownership classes, offline verification, upgrades, semantic versions,
and parity between script and plugin entry points.

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

The release MUST use one semantic version in `VERSION`, the plugin manifest, and the instance manifest.

#### Scenario: A plugin version changes alone

- GIVEN the root version is unchanged
- WHEN the plugin structural gate runs
- THEN version disagreement fails

Verify: `pre-commit run plugin-layout --all-files`

### `distribution:plugin-and-scripts-have-parity` — Plugin and scripts have parity

The plugin MUST route initialization, verification, and upgrade to the same scripts available without Claude.

#### Scenario: Claude is unavailable

- GIVEN a target with POSIX tools
- WHEN a maintainer uses the scripts directly
- THEN all three workflows remain available

Verify: `just test-plugin`
