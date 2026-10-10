# Let a skill ship its own references

## Context and Problem Statement

A package held `SKILL.md` and the shared gates alone, so a skill that serves several tasks carried every task in one body, and every run loaded all of them. The skill format lets a skill keep what only some runs need in its own `references/` directory. The installer had no way to land such a file.

## Considered Options

- `compose each package from the skill's own references and the shared root, and refuse a name both claim` — chosen.
- `author every task file under the shared root` — rejected: each package would carry every other skill's tasks, and the shared root would stop meaning shared.
- `keep every task in SKILL.md` — rejected: every run pays for every task.
- `let one source win where both claim a name` — rejected: a package would silently ship a file that differs from its source.

## Decision Outcome

Chosen option: `compose each package from the skill's own references and the shared root, and refuse a name both claim`. One function composes the package the installer, the stage, and the doctor probe read. It takes `SKILL.md` and the files directly under the skill's `references/`, then every shared artifact under `references/`. A file outside that allowlist, or a name both sources claim, fails the composition.

This extends [ADR-install-a-skill-as-a-self-contained-package](./ADR-install-a-skill-as-a-self-contained-package.md). That record composed a package from `SKILL.md` and the shared root alone. The shared root stays one authored source, and the composition still writes a whole package into every agent root.

Enforced by `distribution:a-skill-package-is-self-contained` and `skill-format:shared-content-is-authored-once`.

## Consequences

- Good: a skill can split its tasks into files that load only when a run needs them.
- Good: a collision fails a test before a release, not on a host.
- Bad: an author can no longer place a file of any other kind in a skill directory.

## Status

Accepted
