# Render the documentation root into seed content

## Context and Problem Statement

A seed landed byte for byte, so no seed could name the root of the instance it landed in. The tracking template's example gave a path relative to the documentation root, while the gate resolves every path from the repository root, so an adopter who uncommented it met a failing gate. Two seeded specs named `_docs/`, which is wrong in every codebase-profile instance.

## Considered Options

- `render {docs_root} in adopted seed content at landing, through one helper` — chosen.
- `per-profile wording in prose` — rejected: every seed would explain both roots, and a reader would translate the path the landing already knows.
- `per-profile copies of each seed` — rejected: two copies of one fact drift, and the payload doubles for every seed that names a path.

## Decision Outcome

Chosen option: `render {docs_root} in adopted seed content at landing, through one helper` — the candidate knows the resolved root, so it writes the path an instance actually has. The same helper resolves destinations, wiring patterns, and the `AGENTS.md` block, so no second substitution can render a seed one way and a pattern another. The rendered bytes are the recorded baseline, so a fresh landing reports no drift. A seed the project already holds is never rewritten.

Enforced by `staging:a-seed-lands-rendered-for-its-root`.

## Consequences

- Good: a landed example passes the gate that judges it, in both profiles.
- Good: seeded specs state root-neutral prose, and a cargo test refuses a profile root named as the reader's.
- Bad: `sdd spec` and `sdd template` serve the raw bytes, so a template read there shows the placeholder rather than a path.
- Bad: method chapters other than the two this change edits still name `_docs/`, because `sdd docs` serves them unrendered.

## Status

Implemented
