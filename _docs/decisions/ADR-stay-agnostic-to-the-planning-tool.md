# Stay agnostic to the planning tool

## Context and Problem Statement

The spec-to-code seam needs a work record: something that names the sources a session loads and declares, by rule ID, which agreements a change enacted. Every such record is produced by a planning method — stories and epics, tickets, a plain directory of task files. Committing to one would bind every adopter of this framework to that method, and would exclude a project whose planning is already settled and different.

## Considered Options

- `state the contract and name no tool` — chosen.
- `name a reference planning tool` — rejected: an adopter with a different tool would read the framework's own rules as an argument to switch, and the seam would acquire a dependency it does not need.
- `ship a plan zone in the payload` — rejected: the plan zone's shape is the planning method's business, and a seeded one would compete with whatever the project already runs.

## Decision Outcome

Chosen option: `state the contract and name no tool` — the seam asks for four properties of a record and nothing about its producer: one entry document per unit of work, sources named by path, spec changes cited by typed rule ID, and a record the coverage commands can read.

The dependency is bounded in both directions. The specs never name the tool, so replacing it edits the plan zone and nothing under `specs/` or `decisions/`. The one value a project must declare for itself is the plan zone's path, which appears in a single verification command.

Enforced by `distribution:the-payload-names-no-planning-tool`.

## Consequences

- Good: any planning method that can hold a markdown file satisfies the seam, or none.
- Good: the guarantee is a test over the embedded payload rather than an intention stated in prose.
- Bad: the denylist behind that test is a list of names and cannot see a tool it has not heard of.
- Bad: an adopter must retarget one verification command at their own plan zone, and no command can tell them they forgot.

## Status

Accepted
