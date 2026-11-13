# Land the skills into each instance at its pin

## Context and Problem Statement

sdd installed its skills once under the home, and every project pinned its own sdd. When the projects of one host moved their pins, the one home copy still came from an older release, the doctor failed its skill probes, and the setup skill stopped an upgrade. Nothing owned that copy, and it could match at most one pin.

## Considered Options

- `instance scope in the landing` — chosen.
- `user scope` — rejected: one copy cannot match the pins of many projects.
- `one home-manager copy at a host pin` — rejected: it moves the mismatch to the host pin and still serves every project one version.

## Decision Outcome

Chosen option: `instance scope in the landing` — the skill text then follows the pin of the project that reads it, as Prisma ships its skills in the package a project installs and commits.

`sdd init` and `sdd upgrade` write each package under `.claude/skills/` and `.agents/skills/`, and the project commits them. The record of the instance states the version, and a skill file carries no stamp. No verb writes a skill under the home.

This answers both objections of the record it supersedes. The duplicate name goes away with the home copy: Claude Code prefers a personal skill over a project skill of the same name, and Codex shows both, so the home copy is the one that goes. And `sdd skill show` serves the setup skill from the binary before an instance exists.

Enforced by `distribution:a-skill-has-one-owner`, `distribution:an-instance-carries-the-skills`, `distribution:an-unattributed-skill-folder-is-never-replaced`, `distribution:a-skill-is-readable-before-an-instance`, and `distribution:the-doctor-names-a-shadowing-user-copy`.

## Consequences

- Good: each project reads the skill text of the release it pins.
- Good: an upgrade moves the skills with the rest of the instance.
- Bad: every project commits two copies of each package.
- Bad: a host that still holds a home copy shadows the instance copy until the operator removes it.

## Status

Accepted
