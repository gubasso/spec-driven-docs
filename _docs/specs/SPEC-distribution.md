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
  - [`distribution:a-stale-skill-is-not-a-conflict` — A stale skill is not a conflict](#distributiona-stale-skill-is-not-a-conflict--a-stale-skill-is-not-a-conflict)
  - [`distribution:a-skill-install-restores-on-failure` — A skill install restores on failure](#distributiona-skill-install-restores-on-failure--a-skill-install-restores-on-failure)
  - [`distribution:skill-uninstall-removes-only-payload-files` — Skill uninstall removes only payload files](#distributionskill-uninstall-removes-only-payload-files--skill-uninstall-removes-only-payload-files)
  - [`distribution:user-scope-files-stay-unrecorded` — User-scope files stay unrecorded](#distributionuser-scope-files-stay-unrecorded--user-scope-files-stay-unrecorded)
  - [`distribution:the-payload-names-no-planning-tool` — The payload names no planning tool](#distributionthe-payload-names-no-planning-tool--the-payload-names-no-planning-tool)
  - [`distribution:a-seeded-rule-runs-no-canon-command` — A seeded rule runs no canon command](#distributiona-seeded-rule-runs-no-canon-command--a-seeded-rule-runs-no-canon-command)

<!--TOC-->

## Purpose

Rules governing installation, ownership classes, offline verification, and upgrades. The distribution is one installed binary, `sdd`, that carries the payload, and every rule here binds whoever authors that binary. No instance adopts this spec: its subject is the installer, so an instance holding these rules would hold obligations it cannot violate and verifications it cannot run. What a project owes its own installation is stated in `SPEC-instance.md`; the rules the canon alone runs at release time are stated in `SPEC-release.md`.

## Requirements

### `distribution:manifest-identifies-every-owned-file` — The manifest identifies every owned file

The installer MUST record each installed file with its ownership class, destination, and SHA-256, and the record MUST state which canon version produced it.

#### Scenario: An agent encounters a local edit

- GIVEN an installed file differs from its baseline
- WHEN the verifier reads the manifest
- THEN it distinguishes managed drift from adopted reconciliation

Verify: `cargo nextest run -E 'binary(cmd_verify) + binary(cmd_status)'`

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

When run without `--apply`, `sdd skill install` MUST list every destination and write nothing, and when a destination holds bytes neither the payload nor the user-scope record accounts for, an apply MUST refuse atomically, listing every conflict.

#### Scenario: A home directory already carries an edited skill

- GIVEN `~/.claude/skills/sdd-setup/SKILL.md` with bytes the user wrote
- WHEN `sdd skill install --apply` runs
- THEN it exits 73 listing every conflicting destination, writes no file, and states `--force` as the override

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:a-stale-skill-is-not-a-conflict` — A stale skill is not a conflict

Where a user-scope destination holds the bytes a previous apply recorded writing there, `sdd skill install --apply` MUST replace it without `--force`.

#### Scenario: A release edits a skill the user never touched

- GIVEN a home directory whose installed skills came from an older release
- WHEN a newer `sdd skill install --apply` runs
- THEN every destination is rewritten and none is reported as a conflict, because bytes this tool wrote are not the user's work

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:a-skill-install-restores-on-failure` — A skill install restores on failure

Where an apply fails partway, `sdd skill install` MUST restore every destination it backed up and name the path that failed.

#### Scenario: The second skill root cannot be written

- GIVEN two skill roots, the second holding a destination the process cannot write
- WHEN `sdd skill install --apply` has already rewritten the first root
- THEN it exits 73 naming the unwritable path and leaves both roots as found, because one agent reading a newer skill than another is worse than neither being upgraded

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:skill-uninstall-removes-only-payload-files` — Skill uninstall removes only payload files

When run without `--apply`, `sdd skill uninstall` MUST list every removal and delete nothing, and when applied it MUST remove only each embedded skill's `SKILL.md` and its directory when that directory holds nothing else.

#### Scenario: A skill directory carries a user's own note

- GIVEN an installed `~/.claude/skills/sdd-setup/` holding `SKILL.md` and a hand-written `notes.md`
- WHEN `sdd skill uninstall --apply` runs
- THEN `SKILL.md` is removed, `notes.md` and its directory remain, and the kept directory is named in the output

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:user-scope-files-stay-unrecorded` — User-scope files stay unrecorded

Files `sdd skill install` writes outside an instance MUST NOT appear in any instance manifest; the payload and the user-scope record are the references the installer compares them against, and no verification reads either.

#### Scenario: An instance is verified after a user-scope install

- GIVEN an installed instance and a completed `sdd skill install --apply`
- WHEN `sdd verify` runs against the instance
- THEN the report is unchanged by anything under the home directory

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:the-payload-names-no-planning-tool` — The payload names no planning tool

The author MUST keep every embedded payload root free of a planning tool's name, so an instance may pair this framework with any work-record convention or none.

#### Scenario: A method chapter names the tool it was tested against

- GIVEN a chapter edited to illustrate the seam with one planning tool by name
- WHEN the canon test suite runs
- THEN the check fails naming the file and the term, because a framework that names one tool stops being pairable with another

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:a-seeded-rule-runs-no-canon-command` — A seeded rule runs no canon command

Where a spec is seeded into an instance, the author MUST keep the words `cargo` and `just` out of every shell command its verification lines carry.

#### Scenario: A canon-only rule is left in a seeded spec

- GIVEN a seeded spec carrying a rule verified by a cargo test
- WHEN the canon test suite runs
- THEN the check fails naming the spec and the command, because the adopter would read an unrunnable verification as work it owes

Verify: `cargo nextest run -E 'binary(canon)'`
