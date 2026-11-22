# Leave acquisition to the project manager

## Context and Problem Statement

sdd owned how a consumer obtains sdd and keeps its pin fresh. A family of verbs served each manager's fragment, warned from a weekly version-check cache, and moved the pin. It spanned four managers and three venues, read the network, kept a cache, and took a lock. Each manager already installs a tool and moves its pin with its own commands, so sdd duplicated them and owned failures that a documentation tool has no reason to own. The question was who owns acquisition.

## Considered Options

- `leave acquisition to the project manager` — chosen.
- `keep the warning and drop the pin mover` — rejected: the network read, the cache, and the lock stay, for a signal that a portfolio tool gives once for every project.
- `keep the whole family behind an opt-in` — rejected: the code, its rules, and its tests stay.

## Decision Outcome

Chosen option: `leave acquisition to the project manager` — the manager that the project runs owns how the project obtains sdd and keeps its pin fresh. The README shows the install for each manager. sdd carries no installer, no version check, and no pin mover, and no declaration key records a host install.

The one gap that no manager fills, a warning when a tag-pinned flake input has a newer release, belongs to the operator's portfolio tooling. After the operator moves a pin, the binary still names the route to the upgrade.

Enforced by `staging:the-operator-owns-acquisition` and `setup:a-pin-move-names-the-skill-route`.

## Consequences

- Good: sdd keeps no version-check cache, reads no release registry, and takes no lock for a pin.
- Good: how to install sdd has one owner, the README.
- Bad: a project with no portfolio tooling learns of a new release from the release page or from its manager.
- Bad: the acquisition rule IDs retire, and a citation of one no longer resolves.

## Status

Accepted

Supersedes [ADR-check-for-a-newer-release-and-let-the-operator-move-the-pin](./ADR-check-for-a-newer-release-and-let-the-operator-move-the-pin.md).
