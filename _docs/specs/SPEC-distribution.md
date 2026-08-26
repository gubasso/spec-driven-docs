# Distribution Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`distribution:manifest-identifies-every-owned-file` — The manifest identifies every owned file](#distributionmanifest-identifies-every-owned-file--the-manifest-identifies-every-owned-file)
  - [`distribution:initialization-preserves-project-content` — Initialization preserves project content](#distributioninitialization-preserves-project-content--initialization-preserves-project-content)
  - [`distribution:instances-operate-offline` — Instances operate offline](#distributioninstances-operate-offline--instances-operate-offline)
  - [`distribution:upgrade-conflicts-are-atomic` — Upgrade conflicts are atomic](#distributionupgrade-conflicts-are-atomic--upgrade-conflicts-are-atomic)
  - [`distribution:skills-are-part-of-the-payload` — Skills are part of the payload](#distributionskills-are-part-of-the-payload--skills-are-part-of-the-payload)
  - [`distribution:a-skill-obeys-the-portable-format` — A skill obeys the portable format](#distributiona-skill-obeys-the-portable-format--a-skill-obeys-the-portable-format)
  - [`distribution:skill-install-previews-before-writing` — Skill install previews before writing](#distributionskill-install-previews-before-writing--skill-install-previews-before-writing)
  - [`distribution:user-scope-files-stay-unrecorded` — User-scope files stay unrecorded](#distributionuser-scope-files-stay-unrecorded--user-scope-files-stay-unrecorded)

<!--TOC-->

## Purpose

Rules governing installation, ownership classes, offline verification, and upgrades. The distribution is one installed binary, `sdd`, that carries the payload; an instance adopts this spec, and every rule here is one an instance can verify with the binary it runs. The release rules the canon alone runs are stated in `SPEC-release.md`.

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

### `distribution:skills-are-part-of-the-payload` — Skills are part of the payload

The distribution MUST embed every skill authored under `skills/` and install each one as a managed file in every profile.

#### Scenario: A project is initialized for any coding agent

- GIVEN a target repository with no instance
- WHEN `sdd init --apply` runs with either profile
- THEN `.claude/skills/` and `.agents/skills/` carry every skill byte-identical to the payload, recorded managed in the manifest

Verify: `cargo nextest run -E 'binary(cmd_init) + binary(cmd_skill)'`

### `distribution:a-skill-obeys-the-portable-format` — A skill obeys the portable format

Every skill MUST carry only the portable Agent Skills frontmatter fields, a `name` equal to its directory name, and a body at or below 150 lines.

#### Scenario: A skill gains an agent-specific field

- GIVEN a skill edited to add a vendor-only frontmatter key
- WHEN the canon test suite runs
- THEN the conformance test fails and names the offending field

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:skill-install-previews-before-writing` — Skill install previews before writing

When run without `--apply`, `sdd skill install` MUST list every destination and write nothing, and when a destination holds bytes that differ from the payload, an apply MUST refuse atomically, listing every conflict.

#### Scenario: A home directory already carries an edited skill

- GIVEN `~/.claude/skills/sdd-docs/SKILL.md` with locally changed bytes
- WHEN `sdd skill install --apply` runs
- THEN it exits 73 listing every conflicting destination, writes no file, and states `--force` as the override

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:user-scope-files-stay-unrecorded` — User-scope files stay unrecorded

Files `sdd skill install` writes outside an instance MUST NOT appear in any instance manifest; the embedded payload is the reference the installer compares them against.

#### Scenario: An instance is verified after a user-scope install

- GIVEN an installed instance and a completed `sdd skill install --apply`
- WHEN `sdd verify` runs against the instance
- THEN the report is unchanged by anything under the home directory

Verify: `cargo nextest run -E 'binary(cmd_skill)'`
