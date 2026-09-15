# Carry no plan zone

## Context and Problem Statement

The instance manifest recorded a plan zone: the declared place a project's planning tool writes its work records. It had four forms, a variable override, a reconcile decision at every landing, and one gate. The declaration was meant to keep this framework agnostic of the planning tool. A landing into another project showed the opposite: the operator was asked where plans live, a question with no answer inside this tool's domain.

## Considered Options

- `carry no plan zone` — chosen.
- `keep the declared zone with a none form` — rejected: `none` is still an answer the operator owes, so the dependency stays.
- `keep a planning-tool denylist as the guarantee` — rejected: a list of planning products is knowledge of the category, held dormant.

## Decision Outcome

Chosen option: `carry no plan zone` — a declared value is still a dependency, because the project has to answer for it. The only agnostic form is the absence of the value. The field, the variable, the flag, the decision, the gate, and every work-record requirement are removed, and nothing replaces them.

What survives is the retrieval invariant under the old entry document, renamed the context entrypoint: a session selects one per domain its working subject touches, and each links every required source directly by path.

A documentation root may carry directories this tool does not own, and every delivered gate leaves them alone.

Enforced by `distribution:the-payload-names-no-other-project` and `release:a-delivered-gate-reads-what-the-convention-owns`.

## Consequences

- Good: a landing asks nothing about planning, and an adopter declares nothing it does not own.
- Good: the docs scratch keeps its declaration: a scratch is a stage inside the documentation domain this tool owns.
- Bad: the deleted denylist held planning vocabulary such as sprint, kanban, and standup; a reviewer now catches those words, and no test does.
- Bad: an instance carrying the old field upgrades once to drop it.

## Status

Accepted — supersedes [ADR-stay-agnostic-to-the-planning-tool](./ADR-stay-agnostic-to-the-planning-tool.md) and [ADR-record-the-plan-zone-at-install](./ADR-record-the-plan-zone-at-install.md) whole, and [ADR-name-a-declared-location-by-one-variable](./ADR-name-a-declared-location-by-one-variable.md) where it governs the plan zone; it still binds the docs scratch.
