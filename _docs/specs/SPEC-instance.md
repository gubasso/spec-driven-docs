# Instance Specification

## Purpose

Rules a project owes the instance installed in it. Covers the installation record the installer writes into the project, and the managed regions the installer keeps inside project-owned files. It also covers the tracking registry the project owns, and every ownership check that reads these back. Every rule here is one the project itself can violate, and every verification runs with what the install wires. How the installer produces that record, and what the binary owes at install and upgrade time, belong to `SPEC-distribution.md`, which no instance adopts. The writing convention the project follows belongs to `SPEC-writing-style.md` and `sdd method writing-style`. The registry's own shape belongs to `SPEC-tracking.md`.

## Requirements

### `instance:the-manifest-stays-readable` — The manifest stays readable

The project MUST keep `.spec-driven-docs/manifest.json` present and valid against its schema, because every ownership check reads that record before it can judge anything.

#### Scenario: A manifest is hand-edited until it no longer parses

- GIVEN an installed instance whose manifest is edited by hand
- WHEN the edit leaves the record missing, truncated, or at an older schema version
- THEN the hook fails naming the manifest, because a record no check can read disables every ownership check at once

Verify: `pre-commit run instance-manifest --all-files`

### `instance:the-agents-block-stays-managed` — The managed AGENTS block stays intact

The project MUST keep the marked documentation block inside the root `AGENTS.md` intact. The install writes the documentation and writing-style routing into that region, so an edit inside the markers is a conflict.

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

### `instance:the-project-declares-what-its-gates-judge` — The project declares what its gates judge

The project MAY state, in `.spec-driven-docs/config.yaml`, which paths no delivered gate judges and which filters a named gate takes, and the tool MUST apply that statement to every route a subject path reaches a gate by. A declaration that does not parse, or that names a gate this version does not deliver, MUST fail once and name the key, never fall back to the default.

#### Scenario: Another tool owns a region of a file at the project root

- GIVEN a project that records that path under `reserved:`
- WHEN any delivered gate runs, whether pre-commit passes the path or the gate walks to it
- THEN no gate judges the path, and `sdd gate --explain <path>` names `reserved` as the layer that decided

Verify: `pre-commit run cargo-test --all-files`

### `instance:the-managed-block-agrees-with-the-declaration` — The managed block agrees with the declaration

The managed pre-commit block is rendered from the declaration, so the project MUST NOT edit the block, and the per-gate wiring in the block MUST match what the declaration renders. `sdd hooks --apply` is what brings a stale block back into agreement, and it works at the same version.

The documentation block in the root author-instructions file carries the writing-style route the declaration selects, so the same verb rewrites it and the same check holds it.

#### Scenario: The declaration is edited and nothing else is run

- GIVEN a project that adds an exclusion to its declaration
- WHEN `sdd verify` runs
- THEN it fails naming the gate whose wiring disagrees and the command that fixes it, because an upgrade returns early at the same version and would never reach the block

Verify: `sdd verify --target .`
