# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2](https://github.com/gubasso/spec-driven-docs/compare/v0.3.1...v0.3.2) - 2026-08-26

### Fixed

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
