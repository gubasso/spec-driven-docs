# Read a migration's scope from its findings

## Context and Problem Statement

Brownfield was one word for three kinds of debt. A project with a numbered decision tree and a wiki export was brownfield. So was one whose directories already fit the convention and whose prose ran long. Both got the sweep. The method already had the marginal path for two of the three: prose converts when a document is edited, and an inherited budget ratchets down. Nothing read a corpus to say which kind it was.

## Considered Options

- `read the kind of debt, and offer the scope it allows` — chosen.
- `keep the sweep as the only scope` — rejected: it charges a full migration to a project whose debt converts on its own.
- `let the operator declare the scope` — rejected: they would declare what only a reading of the corpus establishes.
- `measure prose and report it` — rejected: no delivered gate judges prose, and an engine that judged it would refuse the convention by another name.

## Decision Outcome

Chosen option: `read the kind of debt, and offer the scope it allows`. A structural finding makes two conventions coexist: a foreign documentation root, a spec-shaped document with no rule ID, a document named by its position, a record outside the decisions directory, or a settled corpus with no specs directory. Each forces the sweep. A budget finding is a measurement over a cap, and becomes debt. A document written before the instance is a style candidate: named, never judged, converted when edited.

Incremental is offered only where the structural count is zero, and over one it blocks. That is the harm the sweep exists to stop, stated as a precondition rather than a default.

Enforced by `reconcile:a-finding-is-something-the-program-proved` and `reconcile:an-incremental-scope-leaves-no-structural-finding`.

## Consequences

- Good: a project whose debt is prose is charged for prose.
- Good: each finding names its detector, so a disputed one is arguable.
- Bad: a detector the engine lacks is a finding nobody sees.
- Bad: the detector set is maintained against the convention.

## Status

Accepted
