# Make self-containment a rule rather than a habit

## Context and Problem Statement

A document reaches readers who are not its author, on machines that are not the author's. Two things break that: an absolute path into the author's home directory, which resolves for one person and names someone who never agreed to be named; and a rule the project requires but leaves another project's documentation to state, which changes when someone else edits it. Both were avoided here by habit, neither was checked, and one narrow payload check — no planning tool may be named — was the whole guarantee.

## Considered Options

- `state both clauses and gate the mechanical half` — chosen.
- `gate both` — rejected: whether a link carries a claim or supports it requires knowing the project's domain, which no denylist holds.
- `leave both to review` — rejected: a rule presented as binding and never checked teaches readers that specs describe intentions.

## Decision Outcome

Chosen option: `state both clauses and gate the mechanical half`. The personal-path clause is a shape a command can see, so it ships as a delivered gate every instance wires. The ownership clause is a judgement, so it joins the declared unenforced list rather than pretending to a verification.

Citing outward stays welcome, and the placement chapter draws the line: a link that supports a claim is evidence a reader may follow or ignore, and a link that carries a claim is a dependency. The exemptions are by purpose — a file whose job is one person's environment is where a real path belongs, and an untracked file was never published.

Enforced by `docs-foundations:a-document-carries-no-personal-path` and `docs-foundations:a-document-owns-what-it-governs`.

## Consequences

- Good: every instance receives the check, so the guarantee travels with the method.
- Good: the payload scan generalizes to any outside project, over roots declared once.
- Bad: the ownership clause rests on a reviewer, and a wrong call looks like a right one.
- Bad: the gate reads shape, so a real home directory spelled like a placeholder passes.

## Status

Accepted
