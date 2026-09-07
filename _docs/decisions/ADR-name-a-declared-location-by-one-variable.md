# Name a declared location by one variable

## Context and Problem Statement

The method fixed one path for exploratory material and staged rewrites, and wrote it as a literal in fourteen places: the chapters, the seeded rules template, the two skills, and the classifier. A project that keeps its checkout pristine could honor none of them. The framework was tool-agnostic and location-fixed at once.

## Considered Options

- `name each declared location by one variable, and offer paths only in the question` — chosen.
- `keep the path fixed and document an override` — rejected: the fixed path stays the rule, so every reader learns the literal and the override becomes an exception nobody reads.
- `let each document offer its own candidates` — rejected: fourteen places offering paths is fourteen places to edit when the offer changes, and they drift apart.

## Decision Outcome

Chosen option: `name each declared location by one variable, and offer paths only in the question` — the corpus writes `SDD_PLAN_ZONE` and `SDD_DOCS_SCRATCH`, and a concrete candidate path appears only in the `AskUserQuestion` a skill asks the operator.

The rule extends rather than accumulates: a different staging purpose gets its own variable by the same pattern, offered in one question and nowhere else. A migration needs none of its own, because it stages under `migration/` inside the docs scratch.

The concept is renamed to the docs scratch, because a slug a reader meets cold must answer "scratch of what".

Enforced by `distribution:a-declared-location-is-named-by-its-variable`.

## Consequences

- Good: changing what the skills offer edits one question, never a chapter, a spec, or a template.
- Good: a canon test holds the retired literal and the retired word out of the payload, so the location cannot be re-fixed by an edit nobody notices.
- Bad: a reader of a chapter learns a variable name rather than a path, and runs a skill to see where it points.
- Bad: the classifier keeps one discovery candidate for a target that has declared nothing yet, so one literal survives in code.

## Status

Accepted
