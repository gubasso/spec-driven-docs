# 99 — Checklist

The pre-merge gate for a documentation change. Every box is answerable yes or no from the diff or from
a command. This file is derived: it restates rules in test form and owns none of them. If a box and
its owning chapter disagree, the chapter wins.

## Model

Owner: [00 — Model](./00-model.md).

- [ ] Every durable fact touched by the change has exactly one owner, and the reviewer can name it.
- [ ] No document restates a fact another document owns; non-owners link instead.
- [ ] Every pointer document carries what orients the reader and the link, and no further facts.
- [ ] A behavior contract is carried by a name, a type, or a test where one can carry it.
- [ ] No passage indexes the filesystem for its own sake; a listing that stays earns it by what its
      entries teach.
- [ ] Where the change touched both a spec and a decision record, the spec states the present and the
      record was not edited to match it.

## Placement

Owner: [01 — Placement](./01-placement.md).

- [ ] The docs root matches the product: `docs/` for a codebase, `_docs/` for a content tree, and no
      product content sits under it.
- [ ] Each document has one primary reader need and lives in the matching zone.
- [ ] Every spec is at `<root>/specs/SPEC-<domain>.md` and none is co-located with what it governs.
- [ ] Every file whose kind this framework fixes carries its uppercase prefix: `SPEC-`, `ADR-`,
      `KI-`, or `TEMPLATE-`. Guides, reference, and explanation pages carry none.
- [ ] No prefixed filename carries a counter; the slug is the identifier.
- [ ] No exploratory material entered the docs root.

## Specs

Owner: [02 — Specs](./02-specs.md).

- [ ] The spec uses `## Purpose` then `## Requirements`, and introduces no section outside the shape.
- [ ] Requirements are ordered with the most consequential first.
- [ ] No narrative sits between two requirements; a clarification marker is the one exception.
- [ ] Every marker is `[NEEDS CLARIFICATION: <question>]`, the spec carries at most three, and no
      unit of work enacts a marked rule.
- [ ] No spec links or names a decision record, and no entry document does either; the reference
      runs from the record to the rule ID.
- [ ] A behavior change in this diff is reflected in the spec.
- [ ] A retired requirement was deleted, not marked obsolete.
- [ ] A supporting artifact sits in `<root>/specs/SPEC-<domain>/` only when it has no reader who
      arrives without the spec; anything with an independent reader is in reference.
- [ ] The spec is within 300 authored lines, and a spec over 100 lines carries a generated TOC.

## Rules

Owner: [03 — Rules](./03-rules.md).

- [ ] Every requirement is one `###`<id>`— <title>` heading with a statement, a scenario, and a
      `Verify:` line.
- [ ] Every statement is one sentence in one of the five patterns, with an RFC 2119 keyword.
- [ ] Every statement names a subject that can act.
- [ ] Every rule ID is `<spec-slug>:<rule-slug>` and unique across the project.
- [ ] No rule ID changed because its sentence was reworded.
- [ ] Prohibitions are at or below five per spec, and each is paired with the action replacing it.
- [ ] Every scenario names the contested case rather than restating the rule.

## Decisions

Owner: [04 — Decisions](./04-decisions.md).

- [ ] The choice cleared the threshold: cross-cutting, expensive to reverse, constraining, or
      rejecting a plausible alternative.
- [ ] The filename is `ADR-<slug>.md` and carries no digit.
- [ ] No merged record was renamed or deleted.
- [ ] No record was edited to describe a later design.
- [ ] The body is at or below 350 words, uses the five sections, and carries exactly one `Status`.
- [ ] Every considered option carries a disposition; rejections state why, deferrals name a reopening
      condition.
- [ ] An enforceable consequence of this record is stated as a requirement in a spec, and the record
      cites the rule ID.

## Agent context

Owner: [05 — Agent Context](./05-agent-context.md).

- [ ] The always-loaded files are within budget: 100 lines at the root, 150 in a subtree.
- [ ] Subtree-local rules live in the subtree, and the root file points rather than imports.
- [ ] Every source an entry document needs is linked directly from it, not through another document.
- [ ] Filenames indicate their contents.

## Format

Owner: [06 — Format](./06-format.md).

- [ ] No bold or italic text anywhere in the change.
- [ ] Every fenced block declares a language.
- [ ] RFC 2119 keywords appear only in normative statements.
- [ ] Every report leads with its summary, then one concrete run, then expected against actual.
- [ ] Every claim about what code does quotes the span that shows it, headed with its path,
      revision, and start line.
- [ ] Prose is spent only on a decision, a hazard, or a non-obvious constraint.
- [ ] No document narrates its own history or explains an absence.
- [ ] The change recommends one default rather than surveying alternatives it does not recommend.
- [ ] One term is used for one concept.
- [ ] Every chapter is within 200 lines and every catalog within 300.
- [ ] Every budget stated as a count has a hook that fails the change, and no budget was raised to
      admit a document.
- [ ] A gate that skips a named file fails once that file fits or disappears.

## Lifecycle

Owner: [07 — Lifecycle](./07-lifecycle.md).

- [ ] A behavior change in this diff updated the owning spec in the same change.
- [ ] The change states what was added, modified, or removed, and does not restate what it left alone.
- [ ] No exploratory material entered the docs root, and any promoted draft was deleted.
- [ ] A fact depending on an external source has an entry in the tracking registry.
- [ ] A workaround added here names the condition that retires it, and a workaround whose condition is
      met was removed along with its record.
- [ ] Every known-issue record is named `KI-<slug>.md`, carries exactly one state, and a masked one
      carries a retire condition where a mitigated one carries none.
- [ ] Every known-issue record walks its mechanism step by step, showing the state each step leaves
      behind, rather than naming the defect once.
- [ ] Every record filed upstream carries a `## Report` section holding the filed body in the
      tracker's own markup.
- [ ] A resolved symptom that could recur and be misread left a diagnostic entry, and no archive of
      resolved cases was created.

## Spec to code

Owner: [09 — Spec to Code](./09-spec-to-code.md).

- [ ] A spec change in this diff is cited in the enacting entry document as a typed clause:
      `ADDED`, `MODIFIED`, or `REMOVED`, then the rule ID in inline code.
- [ ] No requirement carries a stored status marker, and no stored coverage artifact was added.
- [ ] The cited type matches the diff: a new requirement is `ADDED`, a reworded one `MODIFIED`, a
      deleted one `REMOVED`.
- [ ] Every comment citing an agreement cites it by rule ID as `SATISFIES` or `VERIFIES`, and no
      comment names a decision record.
- [ ] Every rule ID cited in code resolves to a requirement in a spec.
- [ ] A comment that survived holds what the code cannot express; one restating the next line was
      deleted and one covering for a vague name became the name.
- [ ] Every suppressed or failing test names its case id, every case id resolves to a `KI-<slug>.md`
      record, and every expected failure uses the strict form.

## Procedures

Owner: [10 — Procedures](./10-procedures.md).

- [ ] Every step is one action in the imperative, with at most one sentence after its command.
- [ ] An outcome no command prints is a verification step rather than a sentence describing it.
- [ ] The guide opens with checkable preconditions and closes with a verification step that states
      what a correct result looks like.
- [ ] Every artifact token is upper-snake, names the artifact rather than the step, and carries no
      realistic value.
- [ ] Every phase of a multi-phase guide carries an inputs line naming each producer, and every phase
      that produces an artifact ends with an outputs block.
- [ ] No outputs block was promoted to a heading, and none defines the artifact it names.

## Operational documents

Owner: [11 — Operational](./11-operational.md).

- [ ] Each operational document carries every part of its shape, and the runbook states the condition
      under which the reader stops and reverts.
- [ ] Every destructive step shows its dry-run or inspection form as its own prior step, and states
      what cannot be recovered before the command.
- [ ] Every signal a diagnostic names carries its expected result.
- [ ] A rule an incident produced was written to the owning spec rather than left in the case study.
- [ ] No directory was named after a document type.

## Gates

Owner: [08 — Gates](./08-gates.md).

- [ ] A rule added by this change is checked by a hook, or listed as unenforced.
- [ ] The hooks pass on the changed files.
