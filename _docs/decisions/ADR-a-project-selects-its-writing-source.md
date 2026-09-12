# A project selects its writing source

## Context and Problem Statement

`ADR-author-the-writing-style-and-drop-the-gate` removed the delivered prose gate, and that was correct. What remained bound an adopter to the chapter anyway: the writing-style spec scoped the style to `method/writing-style.md`, required the block to route to `sdd method writing-style`, and named that file as the one document. A project with a house style had no way to say so, and one wanting the structure without the register still owed conversion to a style it never chose.

## Considered Options

- `let the declaration select builtin, a project document, or none` — chosen.
- `change only the one-document rule` — rejected: the route rule and the spec's purpose would still name the chapter.
- `offer builtin and project without none` — rejected: a project declining a style would still owe conversion to one.
- `let a project name several sources` — rejected: two sources is two registers, and an author reading both has no rule to follow.
- `a delivered gate that judges prose against the selected source` — rejected: the earlier record refused a delivered prose judge, and a project source changes nothing there.

## Decision Outcome

Chosen option: `let the declaration select builtin, a project document, or none` — the project's `.spec-driven-docs/config.yaml` carries `writing_style.source`, the managed documentation block routes authors to the selection, and `none` installs no route and imposes no obligation.

The three writing-style rules keep their IDs and lose their fixed filename: the block routes to the selected source, the rules live in one document whatever it is, and a document converts only where a source is selected. `sdd init --writing-style` records the choice, and `sdd hooks --apply` rewrites the block after an edit.

Enforced by `writing-policy:the-project-selects-one-source` and `writing-policy:none-imposes-no-obligation`.

## Consequences

- Good: a project keeps its own style guide and the method routes to it.
- Good: a project can decline the register and keep the structure.
- Bad: an instance carries one more key, and its older spec sentences disagree with a non-default selection until reconciled.

## Status

Implemented

Enacted by `src/domain/instance_config.rs`, `src/services/agents_render.rs`, and `src/commands/hooks.rs`.
