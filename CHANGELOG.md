# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/gubasso/spec-driven-docs/compare/v0.5.1...v0.6.0) - 2026-09-09

This version removes the delivered prose gate and the spec that defined its rules. An instance that upgrades has work to do in the same change:

- Run the upgrade so the managed pre-commit block loses the `simple-english` hook. An instance whose configuration still names that hook fails on a gate the binary no longer serves.
- Read the writing style with `sdd method writing-style`. The documentation block in the project's `AGENTS.md` now routes to that command in one line, and no delivered gate judges prose against it.
- Keep `specs/SPEC-simple-english.md` where it is, or delete it. `sdd` no longer seeds that file, so an adopted copy is the project's own from this version on. Its eleven `simple-english:` rule IDs no longer resolve, so re-point any local citation of them.

No document converts on upgrade. A document converts the next time an author edits it, and nothing reports the ones that have not.

```bash
sdd upgrade --target . --dry-run
sdd upgrade --target .
```

### Added

- *(method)* Replace the prose gate with a writing style chapter ([#58](https://github.com/gubasso/spec-driven-docs/pull/58))

## [0.5.1](https://github.com/gubasso/spec-driven-docs/compare/v0.5.0...v0.5.1) - 2026-09-09

This version adds two rules and one gate, and it widens what an existing gate sees. An instance that upgrades has work to do in the same change:

- Give every `masked` or `monitoring` known-issue record a `checked:` ISO date, holding the date its upstream state was last confirmed. Every other state carries none. `ki-checked-date` holds it.
- Give every suppression that `suppression-names-its-case` now sees either a `KI-<slug>` case or an `sdd: permanent <reason>` marker, never both. The gate saw two markdown comment forms before. It now also sees Rust attributes, Python decorators and `noqa` comments, `shellcheck disable=`, `type: ignore`, `zizmor: ignore[`, and `eslint-disable`, each in the files the tool that honors it reads.

```bash
sdd upgrade --target . --dry-run
sdd upgrade --target .
```

### Added

- *(gates)* Date a record's last check and let a suppression state a permanent reason ([#55](https://github.com/gubasso/spec-driven-docs/pull/55))

## [0.5.0](https://github.com/gubasso/spec-driven-docs/compare/v0.4.14...v0.5.0) - 2026-09-07

This version carries the compatibility break that 0.4.14 shipped without marking. Upgrade an existing instance once:

```bash
sdd upgrade --target . --dry-run
sdd upgrade --target .
```

An instance installed before 0.4.14 records manifest schema 2, and 0.4.14 and later read schema 3. Every verb but `sdd upgrade` refuses the older record and names the upgrade. The migration preserves everything the record carried.

### Other

- *(release)* [**breaking**] Mark the manifest schema break in the version ([#50](https://github.com/gubasso/spec-driven-docs/pull/50))

## [0.4.14](https://github.com/gubasso/spec-driven-docs/compare/v0.4.13...v0.4.14) - 2026-09-07

### Added

- *(distribution)* Declare the plan zone and the docs scratch by variable ([#48](https://github.com/gubasso/spec-driven-docs/pull/48))

  The plan zone and the docs scratch became declared values. `sdd init --plan-zone` and `--docs-scratch` record them in the instance manifest, and `SDD_PLAN_ZONE` and `SDD_DOCS_SCRATCH` override the record. An omitted flag never clears a recorded value, and `none` clears one.

  The manifest schema moved from 2 to 3, so an older `sdd` asks for an upgrade rather than failing to parse. The new `spec-change-is-typed` gate replaces the shell command the `spec-to-code` seed carried, and no adopter edits a seeded spec to declare the zone any more.

## [0.4.13](https://github.com/gubasso/spec-driven-docs/compare/v0.4.12...v0.4.13) - 2026-09-06

### Other

- *(guides)* Rewrite the entry points and guides in Plain ([#46](https://github.com/gubasso/spec-driven-docs/pull/46))

## [0.4.12](https://github.com/gubasso/spec-driven-docs/compare/v0.4.11...v0.4.12) - 2026-09-06

### Other

- *(skills)* Rewrite the skills and shared gates in Plain ([#43](https://github.com/gubasso/spec-driven-docs/pull/43))

## [0.4.11](https://github.com/gubasso/spec-driven-docs/compare/v0.4.10...v0.4.11) - 2026-09-06

### Other

- *(method)* Rewrite the last five method chapters in Plain ([#41](https://github.com/gubasso/spec-driven-docs/pull/41))

## [0.4.10](https://github.com/gubasso/spec-driven-docs/compare/v0.4.9...v0.4.10) - 2026-09-06

### Other

- *(method)* Rewrite the second five method chapters in Plain ([#39](https://github.com/gubasso/spec-driven-docs/pull/39))

## [0.4.9](https://github.com/gubasso/spec-driven-docs/compare/v0.4.8...v0.4.9) - 2026-09-06

### Other

- *(method)* Rewrite the first five method chapters in Plain ([#37](https://github.com/gubasso/spec-driven-docs/pull/37))

## [0.4.8](https://github.com/gubasso/spec-driven-docs/compare/v0.4.7...v0.4.8) - 2026-09-06

### Other

- *(specs)* Rewrite the specifications in Plain ([#35](https://github.com/gubasso/spec-driven-docs/pull/35))

## [0.4.7](https://github.com/gubasso/spec-driven-docs/compare/v0.4.6...v0.4.7) - 2026-09-06

### Other

- *(templates)* Rewrite the templates in Plain ([#32](https://github.com/gubasso/spec-driven-docs/pull/32))

## [0.4.6](https://github.com/gubasso/spec-driven-docs/compare/v0.4.5...v0.4.6) - 2026-09-05

### Added

- *(distribution)* Default to SimpleEnglish and track its upstream ([#30](https://github.com/gubasso/spec-driven-docs/pull/30))

## [0.4.5](https://github.com/gubasso/spec-driven-docs/compare/v0.4.4...v0.4.5) - 2026-09-05

### Added

- *(skills)* Route the migration skill and gate what the operator owns ([#28](https://github.com/gubasso/spec-driven-docs/pull/28))

## [0.4.4](https://github.com/gubasso/spec-driven-docs/compare/v0.4.3...v0.4.4) - 2026-09-05

### Added

- *(distribution)* Package for Nix and adopt the devshell pin ([#26](https://github.com/gubasso/spec-driven-docs/pull/26))

### Fixed

- *(release)* Anchor the pin matcher to the assignment and fail closed ([#21](https://github.com/gubasso/spec-driven-docs/pull/21))
- *(release)* Close the review findings on the rk pin bump ([#19](https://github.com/gubasso/spec-driven-docs/pull/19))

## [0.4.3](https://github.com/gubasso/spec-driven-docs/compare/v0.4.2...v0.4.3) - 2026-09-04

### Added

- *(specs)* Split a known-issue record into two governed axes ([#15](https://github.com/gubasso/spec-driven-docs/pull/15))

### Fixed

- *(distribution)* Close two blind spots in the canon layout checks ([#17](https://github.com/gubasso/spec-driven-docs/pull/17))

## [0.4.2](https://github.com/gubasso/spec-driven-docs/compare/v0.4.1...v0.4.2) - 2026-09-03

### Other

- *(deps)* Pin rk in the devshell and keep it current

## [0.4.1](https://github.com/gubasso/spec-driven-docs/compare/v0.4.0...v0.4.1) - 2026-09-03

### Added

- *(distribution)* Classify the target and land the migration workflow ([#12](https://github.com/gubasso/spec-driven-docs/pull/12))
- *(distribution)* Land the pre-flight gate and sdd doctor ([#10](https://github.com/gubasso/spec-driven-docs/pull/10))

## [0.4.0](https://github.com/gubasso/spec-driven-docs/compare/v0.3.2...v0.4.0) - 2026-09-02

### Added

- *(specs)* Bind the guides document class and land it in instances
- *(skills)* Gate every skill on plan, validate, then execute
- *(docs-foundations)* [**breaking**] Make self-containment a rule instances run
- *(distribution/skills)* [**breaking**] Give every skill one owner
- *(distribution/skills)* Record what a skill install wrote
- *(distribution)* [**breaking**] Seed only rules the adopting project can run
- *(distribution)* Enforce agnosticism to the planning tool

### Fixed

- *(skill-shared/plan-gate)* Make the --no-plan path executable

### Other

- *(release)* Land the release-kit trunk convention ([#7](https://github.com/gubasso/spec-driven-docs/pull/7))
- *(release)* Check the canon record and block against their sources
- *(release)* [**breaking**] Serve instances, not remote consumers

## [0.3.2](https://github.com/gubasso/spec-driven-docs/compare/v0.3.1...v0.3.2) - 2026-08-26

### Fixed

- *(release)* Check the changelog while it can still be corrected
- *(lint)* Let dprint pass when every changed file is exempt
- *(release)* Let cargo-dist own the GitHub release
- *(lint)* Stop link-checking the generated changelog
- *(release)* Make the verify step able to pass

## [0.3.1](https://github.com/gubasso/spec-driven-docs/compare/v0.3.0...v0.3.1) - 2026-08-26

### Added

- *(release)* Cut the release from master

### Fixed

- *(release)* Pin the release gate to the release commit
- *(release)* Keep an unattributed commit from wedging the gate

## [0.3.0](https://github.com/gubasso/spec-driven-docs/compare/v0.2.0...v0.3.0) - 2026-08-26

### Added

- *(skills)* Add sdd skill uninstall and thin the just recipe
- *(skills)* Ship cross-agent skills in the binary

### Fixed

- *(gates)* Exempt the generated changelog from the prose gate
- *(links)* Repair the dead StrictDoc link and quiet lychee

### Other

- *(skills)* [**breaking**] Rename the shipped skills for an obvious split
