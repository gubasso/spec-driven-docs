# Give every skill one owner

## Context and Problem Statement

`sdd init` installed each skill into the instance at `.claude/skills/` and `.agents/skills/`, and `sdd skill install` wrote the same bytes into those directories under the home. An agent resolves a skill by name across both scopes, so a session opened inside an instance offered two entries per skill, with no way to tell which would run. Two owners of one name is a defect byte-identical copying cannot fix.

## Considered Options

- `user scope owns every skill` — chosen.
- `instance scope owns every skill` — rejected: `sdd-setup` lands an instance in a repository that has none, so a skill reachable only from inside an instance cannot do its own job.
- `keep both and dedupe at read time` — rejected: the reader is the agent, not this distribution, and no agent offers a deduplication hook to write against.
- `install only one directory family` — rejected: it breaks again the moment an agent reads the other family.

## Decision Outcome

Chosen option: `user scope owns every skill` — one installed binary already serves every repository, so the skills routing into it belong at the same scope. No profile projects a skill, and `sdd upgrade` prunes what earlier versions installed.

Enforced by `distribution:a-skill-has-one-owner` and `distribution:an-install-sweeps-what-the-payload-dropped`.

## Consequences

- Good: a skill name resolves to exactly one file, and the operator sees one entry per skill in any repository.
- Good: an install or uninstall now removes the destinations the record vouches for but the payload no longer carries, so a renamed or dropped skill leaves nothing behind.
- Bad: an instance no longer pins the skill text to its own canon version; every repository reads whatever version the home directory last installed.
- Bad: an instance installed by an earlier version keeps its copies until `sdd upgrade` runs there.

## Status

Implemented — `src/domain/profile.rs` carries no skill projection, `src/services/skill_installer.rs` sweeps recorded leftovers, and `src/services/upgrader.rs` removes the directory a pruned skill empties.
