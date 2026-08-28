# Seed only rules the instance can violate

## Context and Problem Statement

`SPEC-distribution.md` was seeded into every instance while nine of its ten rules stated obligations of the `sdd` binary, verified by cargo tests no adopting project can run. Because a failing verification is this method's only signal for unbuilt behavior, an adopter received nine permanently failing commands and no way to read them as anything but their own backlog. The corpus offers no other way to say "not your obligation".

## Considered Options

- `sort the corpus by whose obligation each rule states` — chosen.
- `keep seeding the spec and mark the canon-only rules` — rejected: a marker is a stored status, which `spec-to-code:a-spec-may-lead-its-code` forbids precisely because it drifts from the command beside it.
- `make the commands portable` — rejected: the rules describe installing, upgrading, and skill management, so no rewording makes them checkable by a project that ships none of those.

## Decision Outcome

Chosen option: `sort the corpus by whose obligation each rule states`. `SPEC-distribution.md` becomes canon-only, joining `SPEC-release.md`, which had already drawn this line for the release rules. `SPEC-instance.md` is seeded in its place and states what a project owes its own installation.

The question is whether the adopter could be at fault. A rule whose subject is the installer cannot be violated by a project that runs it, so it never belongs in that project's specs. The distinction is audience, not subject matter: seeded rules still reach into the adopter's code, comments, and tests, which is what makes the seam worth adopting.

Enforced by `distribution:a-seeded-rule-runs-no-canon-command`, and by review for what no command decides.

## Consequences

- Good: an adopter's backlog holds only work the adopter can do.
- Good: the two toolchains a seed may assume, `sdd` and `pre-commit`, are prerequisites the install declares and wires.
- Bad: an instance installed before this change keeps `SPEC-distribution.md`, because an adopted file is the project's own and no upgrade may delete it.
- Bad: only the command is checked, so a canon-only rule with a portable command passes.

## Status

Accepted
