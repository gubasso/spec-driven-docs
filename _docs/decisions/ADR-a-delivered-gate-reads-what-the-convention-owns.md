# A delivered gate reads what the convention owns

## Context and Problem Statement

Three delivered gates selected by file type with no path pattern, so each read every matching file in an adopting project. One project installs this convention beside a release convention, and paid for that reach twice.

That convention renders the root `AGENTS.md` and records its hash, and this one splices a documentation block into the same file. The three gates read the whole file, rendered region included. Editing that region drifts a file the other tool owns, and leaving it fails the commit. A hand-patched exclusion does not survive the next render.

The same project then wrote a local rule holding an uppercase keyword to its specification zone. Written against the whole tree, it failed on its first run, on the rendered line carrying `MUST`.

## Considered Options

- `anchor every delivered gate to what this convention owns` — chosen.
- `a per-gate exclusion list in the installation record` — rejected: it breaks `instance/manifest.schema.json` and asks each adopter to enumerate what other tools render.
- `an advisory severity outside the owned zone` — rejected: a scope problem does not belong in the finding layer.
- `document the exclusions an adopter adds by hand` — rejected: an upgrade re-renders the managed region over them.

## Decision Outcome

Chosen option: `anchor every delivered gate to what this convention owns` — a gate selects under the instance's documentation root, by a filename shape this convention defines, or by resolving its own scope. Everything else belongs to the project, which wires its own wider check.

Enforced by `release:a-delivered-gate-reads-what-the-convention-owns`.

## Consequences

- Good: a gate reads this convention's own documents alone, so an adopter carries no repair across upgrades.
- Good: the registry states each gate's zone, and a canon test refuses a row that states none.
- Bad: `no-personal-path` loses its breadth. A personal path in a project's root `README.md` reaches no delivered gate, and its rule binds the author with no check behind it.

## Status

Implemented

Enacted by three `files` patterns in `src/gates.rs` and by `every_file_passed_gate_anchors_its_scope` in `tests/canon.rs`.
