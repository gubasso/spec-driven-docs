# Lint Markdown through native configuration discovery

## Context and Problem Statement

The heading shapes lived in private files that three hooks loaded with `--config`. A direct run and an editor never read them, so the routes judged different settings. The relative-links file set `"default": false` and never enabled its own rule, so that check never ran. The rendered `AGENTS.md` block also failed the linter's defaults: it opened at a second-level heading and carried unwrapped items past 80 columns.

## Considered Options

- `one managed configuration at the documentation root, found by discovery` — chosen.
- `a private directory loaded by --config` — rejected: only the hook reads it.
- `one configuration per directory` — rejected: two managed files would hold what one file's `overrides` hold.
- `a Rust heading-shape gate` — rejected: it duplicates MD043, and editors never run it.
- `reserved: copied into the linter hook's exclude:` — rejected: a commit would skip a file a direct run still judges, so the scope would live in two files.
- `refusing any markdownlint configuration at the repository root` — rejected: a root file cannot turn the shapes off, so the refusal would make a project migrate a configuration that already composes.
- `a disable and enable pair around the AGENTS.md block` — rejected: it leaves the project's MD013 state altered after the markers.
- `MD013: false in the root seed` — rejected: the seed yields to a project's own configuration, so the setting reaches nothing there.

## Decision Outcome

Chosen option: `one managed configuration at the documentation root, found by discovery` — every route reads the same settings, and the project's own configuration merges beneath it. The root seed loads the relative-links rule, and the block suppresses MD013 on its own lines alone.

Enforced by `instance:the-lint-configuration-composes`, `docs-specs:a-spec-follows-the-heading-shape`, and `decision-records:a-record-follows-the-heading-shape`.

## Consequences

- Good: the heading shapes judge the same files in a hook, a direct run, and an editor.
- Bad: two locations still break composition, and `sdd verify` refuses them by filename.

## Status

Implemented
