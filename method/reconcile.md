# Reconcile

How a target moves from what it is to what a release says it should be. The subject is one repository and one destination release, and the unit of work is a plan. Migrating a corpus onto the method is the question of [Migration](./migration.md). Installing and owning the files is the question of the [instance guide](../instance/README.md). This chapter owns what every landing shares: one plan, one decision, one apply.

## Every write comes from a plan

A plan is one typed document that states what a target is, what the release wants, and every write that closes the difference. It declares the schema `sdd.plan/1`, it is computed and never edited, and it carries its own identity.

The rule is that no write happens outside one. A tool with three verbs that each walk the same projection along their own path has three places for a rule to drift. A tool where every verb computes a plan and one executor runs it has one. The plan is therefore the interface between deciding and doing, and the front verbs are shorthands that reach it.

A plan is also the answer to what a release does here. A running engine reads any release it can describe through the release seam, so an operator asks by planning rather than by obtaining that release's binary first. The single case that still needs a newer binary is a bundle whose payload protocol is above the engine's, and the plan names the version to install.

## The five kinds the model keeps apart

| Kind           | Holds                                                                        | Owner        |
| -------------- | ---------------------------------------------------------------------------- | ------------ |
| Evidence       | the repository, the record, the declaration, the paths, the host, the corpus | the target   |
| Analysis       | findings, operations, compatibility, guidance                                | the engine   |
| Policy         | the requirement each precondition carries                                    | the release  |
| Decisions      | the questions only a person answers                                          | the operator |
| Postconditions | what proves the apply finished                                               | the engine   |

Keeping them apart is what makes an addition safe. A new finding kind changes analysis alone. A new question changes decisions alone. A plan that folded the five into one list would make every addition a change to everything.

## The seven states and the six classifications

A classification is read from the target and never from the verb the operator typed. Six values cover it.

- `setup`: no instance and nothing durable written.
- `migration`: no instance and a settled corpus.
- `upgrade`: an instance older than the destination.
- `drift`: an instance at the destination whose files have moved.
- `current`: an instance at the destination with nothing to do.
- `invalid`: metadata that exists and cannot be trusted.

Seven target states reach those six. An empty target is `setup`. A brownfield target with structural findings and a brownfield target with none are both `migration`, because findings are orthogonal to classification and the count changes the plan rather than the verdict. A landed instance behind the destination is `upgrade`, one at it with moved files is `drift`, and one at it with none is `current`. Unreadable metadata is `invalid`, with cited evidence and no operations, never collapsed into absence.

An unreadable target is a command error rather than a guessed plan. Absence and unreadability are different facts, and a tool that reports the second as the first lands seeds over something it never read.

## Readiness, and why a gap is not permission

Every plan carries preconditions. Each one states a requirement and an evaluation, and the plan's readiness is the worst of them.

- `required` plus an unsatisfied evaluation makes the plan `blocked`. Nothing applies.
- `decision-required` plus an unanswered question makes it `needs-decision`.
- `advisory` never blocks, and an evaluation of `not-observed` carries why it could not be read.

A difference between the target and the release is a finding. Permission to close it is a precondition. The two are deliberately separate, because a tool that treats every gap it can describe as work it may do is a tool that writes on its own authority. The engine describes; the operator authorizes.

Two postconditions close an apply: the record matches the tree, and `sdd verify` reports `OK`.

## The fingerprint and what it binds

A plan's identity is a digest over its semantic inputs, canonically encoded. It answers one question: would this plan still do the same thing to the same target? The observation, the declaration, the resolved release, the operations, the preconditions, and the selected answers are in. Timestamps, presentation text, and advisory evidence are out, because a plan recomputed a second later must keep its identity or no approval could outlive the moment it was given.

An apply recomputes the plan from the world as it is now, compares fingerprints, and refuses on any difference, naming every input that moved. It never re-plans silently. An approval binds to the answers it was given with, so answering a decision computes a new plan with a new identity.

## The destination and the small step

The destination defaults to the release the engine carries, which keeps the default offline. `--to latest` resolves at the registry, and `--to <version>` names one release exactly.

Three compatibility preconditions guard the interval, and each blocks rather than guesses.

- `engine-is-new-enough`: the release declares the lowest engine that can land it, and the plan names the version to install.
- `the-target-is-not-downgraded`: a destination older than what the target records is refused.
- `no-version-is-skipped`: a release may declare versions an interval must pass through, and the plan names the first one to plan toward.

The small-step path is that last case made routine. Where an interval is long, plan to an intermediate version, apply it, and plan again. The engine names the step and never takes it on the operator's behalf.

## The fronts

| Verb                  | Serves                         | Writes         |
| --------------------- | ------------------------------ | -------------- |
| `sdd reconcile plan`  | every classification           | nothing        |
| `sdd reconcile apply` | one stored plan, by its id     | the operations |
| `sdd init`            | setup, upgrade, drift, current | the operations |
| `sdd upgrade`         | upgrade, drift, current        | the operations |
| `sdd assess`          | every classification           | nothing        |

Every one of them writes through the same engine. `sdd init` and `sdd upgrade` are short forms: each turns its flags into the plan's own answers, computes one plan, stores it, and executes exactly that. Neither has a write path of its own, so a landing they perform leaves the same plan id, the same journal, and the same recorded result as `sdd reconcile apply`.

A front constrains which classifications it serves and refuses the rest, naming what the target turned out to be and the verb that serves it. `sdd init` refuses a settled corpus with no instance, because landing seeds there starts a second convention beside the first. `sdd upgrade` refuses an absent instance. Neither widens its own scope, because a front that did would be the classification lying about the target.

A short form can still meet a question. A release in the interval that asks something of a person raises a decision, and `sdd upgrade` stops and names it. Answer it with `--set` there, or ask `sdd reconcile plan` for the step's body first and decide with it in front of you.

`sdd status` reports. A version gap it prints is a fact about the target, not work anybody approved, and the next step is a plan.

## The three reconciliations

None of the three is a single verb, and each has its own shape.

Managed drift. A managed file is a byte-for-byte projection, so an edit is a conflict rather than a preference. The plan is blocked and carries the three-way comparison: the installed bytes, the target's bytes, and the release's. Present that comparison and resolve it at the file. The plan is never widened to keep the target's edit, because a managed file that keeps one is no longer managed.

Adopted drift. An adopted file is seeded once and owned locally, and the manifest records both the installed bytes and the upstream baseline. An edit is reported, the edit is kept, and the baseline moves to the new upstream. This is the ownership working, not a defect to fix.

A retired rule ID. A release that retires a rule leaves every citation of it dangling. The guidance step names the rule and its replacement, the operator re-points the citations, and the decision-record citation gate fails until they are re-pointed. The engine never rewrites a citation, because it cannot know which replacement the author meant.

## Decisions

The plan states each decision, its answer schema, and what each answer costs. Present them with their consequences before asking. Answer with `--set <decision-id>=<answer>`, which recomputes the plan.

An operator who already authorized the upgrade has authorized it. Ask a second question only for a decision the plan names, or for an action outside what the request authorized. A breaking guidance step is always such a decision, and it is accepted with its body in front of the reader.

## What reconcile is not

A profile change or a documentation-root move is a named migration, not a consequence of a new release arriving. It is a plan under new parameters, gated by its own decision, and an operator asks for it deliberately.

A migration checklist is authored from the plan's findings into the plan zone, by whoever runs the migration. The engine never writes there. [Migration](./migration.md) owns the checklist's shape.

## Handling a plan

A plan holds bytes read from the target. It is ephemeral, it is never committed, and it is never pasted into a forge, an issue, or a chat log. No gate sees a plan, so this chapter is where the rule lives.

## One name, two surfaces

`sdd policy reconcile` appends a rule to the project's own specs. `sdd reconcile` is this engine. They sit in different namespaces and both names are right for what they do, and a reader who meets both in one session deserves the sentence.
