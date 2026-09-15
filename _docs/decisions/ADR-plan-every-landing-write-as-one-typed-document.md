# Plan every landing write as one typed document

## Context and Problem Statement

Three verbs walked one projection on three paths. The installer computed a target's whole state, the upgrader reinstalled through that function and re-derived its conflict rules, and the verifier held the record to the same constants again. Three paths are three places for one rule to drift, and an operator saw only what each verb printed as it went. Nothing could be read, approved, and then proved to be what ran.

## Considered Options

- `one typed plan every write comes from` — chosen.
- `keep the three verbs and share helpers` — rejected: a helper is not a document an operator can approve, and each verb still decides.
- `a plan of shell commands` — rejected: what a plan can do would be bounded by what a string says, and nothing could say what a command would touch.
- `carry the bytes inline` — rejected: a plan would hold a repository's content, and printing or storing one would carry it too.

## Decision Outcome

Chosen option: `one typed plan every write comes from`. It keeps five kinds apart: evidence is what was observed, analysis what the engine derived, policy the requirement on each precondition, decisions the operator's, and postconditions what proves completion. The operation set is closed, each operation names one validated target-relative path and digests rather than bytes, and none runs a command.

The planner is pure. The observation, the release bundles, the selected decisions, and the clock are inputs, so the same inputs produce the same plan and the same fingerprint. That is what lets an approval bind: an apply regenerates the fingerprint and refuses on any difference.

Enforced by `reconcile:one-plan-is-the-input-to-every-write` and `reconcile:the-planner-is-deterministic`.

## Consequences

- Good: what a verb will do is readable and approvable before it runs.
- Good: one place derives operations, so a rule holds in every verb.
- Bad: a plan is a large document, and a reader needs its sections.
- Bad: every landing verb becomes a front, which is one more layer.

## Status

Superseded by [ADR-render-the-candidate-this-binary-carries](./ADR-render-the-candidate-this-binary-carries.md)
