# Instance Specification

## Purpose

Rules a project owes the instance installed in it. Covers the installation record the installer writes into the project, the managed regions the installer keeps inside project-owned files, the tracking registry the project owns, and every ownership check reads back. Every rule here is one the project itself can violate, and every verification runs with what the install wires. How the installer produces that record, and what the binary owes at install and upgrade time, belong to `SPEC-distribution.md`, which no instance adopts. The writing convention the project follows belongs to `SPEC-simple-english.md`; the registry's own shape belongs to `SPEC-tracking.md`.

## Requirements

### `instance:the-manifest-stays-readable` — The manifest stays readable

The project MUST keep `.spec-driven-docs/manifest.json` present and valid against its schema, because every ownership check reads that record before it can judge anything.

#### Scenario: A manifest is hand-edited until it no longer parses

- GIVEN an installed instance whose manifest is edited by hand
- WHEN the edit leaves the record missing, truncated, or at an older schema version
- THEN the hook fails naming the manifest, because a record no check can read disables every ownership check at once

Verify: `pre-commit run instance-manifest --all-files`

### `instance:the-agents-block-stays-managed` — The managed AGENTS block stays intact

The project MUST keep the marked documentation block inside the root `AGENTS.md` intact, because the install writes the SimpleEnglish default and the docs routing into that region and an edit inside the markers is a conflict.

#### Scenario: An editor rewrites the marked block

- GIVEN a root `AGENTS.md` carrying the managed documentation block
- WHEN an author edits a line between the markers
- THEN `sdd verify` reports the tampered block, because the region belongs to the install and content outside the markers is the project's own

Verify: `sdd verify --target .`

### `instance:the-tracking-registry-stays-valid` — The tracking registry stays valid

The project MUST keep `<root>/reference/tracking.yaml` valid against its schema and free of overdue or dangling entries, because the freshness gate reads it on every commit.

#### Scenario: A tracked entry falls past its cadence

- GIVEN an adopted tracking registry with one entry
- WHEN the entry's cadence elapses without a revalidation
- THEN the gate fails naming the due date and the recovery steps, and the project revalidates the source before advancing the date

Verify: `pre-commit run tracking-registry --all-files`
