# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
