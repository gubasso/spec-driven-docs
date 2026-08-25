# Embed the payload at compile time

## Context and Problem Statement

The binary must carry everything an instance needs — spec seeds, templates, lint configurations, snippets, migration guides, and the method chapters — with no canon checkout at install or upgrade time. Any copy of those files can drift from the authored originals, and drift between what the canon states and what the binary ships is the failure the whole distribution exists to prevent.

## Considered Options

- `include_dir! from the canonical authored paths` — chosen.
- `rust-embed` — rejected: its debug builds read from the filesystem at runtime, reintroducing the drift a compile-time embedding exists to close.
- `A build step copying assets into src/` — rejected: the copy is a second location, and the build step is the thing that forgets.
- `Download the payload at runtime` — rejected: instances operate offline.

## Decision Outcome

Chosen option: `include_dir! from the canonical authored paths`. One module owns every embedding, each pointing at the real file — `_docs/specs/`, `templates/`, `.markdownlint/`, `instance/snippets/`, `method/`, `migrations/`, and the license texts — so the compiler reads the authored bytes and no third state exists between repository and binary. A build script names the directories so a new file rebuilds the crate.

The published crate must therefore include those directories: the package exclude list is curated rather than the ecosystem's usual everything-but-source shape, and `cargo package` is the gate that proves the tarball still builds.

Enforced by `distribution:instances-operate-offline` and the embedding parity tests.

## Consequences

- Good: canon and binary cannot drift; the build fails or the bytes are current.
- Good: `sdd method`, `sdd spec`, and `sdd migration` work anywhere, offline.
- Bad: every payload edit requires a recompile before the binary reflects it.
- Bad: the crate tarball carries documentation the ecosystem convention would exclude.

## Status

Accepted
