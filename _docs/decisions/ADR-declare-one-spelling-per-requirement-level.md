# Declare one spelling per requirement level

## Context and Problem Statement

The Keywords section declared eight RFC 2119 keywords and the tree used four. The three synonyms of `MUST` and `MUST NOT`, `required`, `shall`, and `shall not` in capitals, appeared only inside the two shell snippets that checked for them and inside quoted license text. A declared synonym costs a reader a decision: two spellings for one meaning ask whether the author meant a difference.

## Considered Options

- `declare one spelling per level` — chosen.
- `keep the full RFC set` — rejected: the RFC permits synonyms and this project's one-word-one-meaning rule refuses them in prose, so the keyword set cannot declare them.
- `drop SHOULD and SHOULD NOT as well` — rejected: deleting a level promotes every existing `SHOULD` to an obligation, which changes what those rules require and is a different request.

## Decision Outcome

Chosen option: `declare one spelling per level` — the set is `MUST`, `MUST NOT`, `SHOULD`, `SHOULD NOT`, and `MAY`: three requirement levels, one spelling for each, and a negation where the level has one.

Capitalization stays the boundary between a keyword and ordinary prose, which is the RFC 8174 rule. This decision narrows what the project declares and changes nothing about how a declared word is read.

A reader of an outside specification still reads a capitalized `shall` correctly. The decision governs what this project writes, not what it can read, and a quotation keeps its own words. The license files are that quotation.

Enforced by `docs-specs:statement-uses-an-ears-pattern` and `docs-specs:prohibitions-are-capped`, whose `Verify:` commands run the narrowed set, and by a canon test over the authored tree.

## Consequences

- Good: a requirement statement has one spelling at each level, and a check has one pattern.
- Good: the chapter, the gate snippets, and the spec are held to one set by a test rather than by hand.
- Bad: an author who writes a capitalized `shall` from habit is corrected by a test, not by the chapter.

## Status

Implemented

Enacted by `method/rules.md`, `method/gates.md`, `_docs/specs/SPEC-docs-specs.md`, and `tests/canon.rs`.
