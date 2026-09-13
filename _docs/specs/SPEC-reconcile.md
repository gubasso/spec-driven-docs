# Reconcile Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`reconcile:one-plan-is-the-input-to-every-write` — One plan is the input to every write](#reconcileone-plan-is-the-input-to-every-write--one-plan-is-the-input-to-every-write)
  - [`reconcile:the-target-decides-its-classification` — The target decides its classification](#reconcilethe-target-decides-its-classification--the-target-decides-its-classification)
  - [`reconcile:the-planner-is-deterministic` — The planner is deterministic](#reconcilethe-planner-is-deterministic--the-planner-is-deterministic)
  - [`reconcile:a-finding-is-something-the-program-proved` — A finding is something the program proved](#reconcilea-finding-is-something-the-program-proved--a-finding-is-something-the-program-proved)
  - [`reconcile:an-incremental-scope-leaves-no-structural-finding` — An incremental scope leaves no structural finding](#reconcilean-incremental-scope-leaves-no-structural-finding--an-incremental-scope-leaves-no-structural-finding)
  - [`reconcile:readiness-is-the-worst-precondition` — Readiness is the worst precondition](#reconcilereadiness-is-the-worst-precondition--readiness-is-the-worst-precondition)
  - [`reconcile:a-decision-precedes-what-depends-on-it` — A decision precedes what depends on it](#reconcilea-decision-precedes-what-depends-on-it--a-decision-precedes-what-depends-on-it)
  - [`reconcile:a-write-into-adopted-state-is-an-operator-act` — A write into adopted state is an operator act](#reconcilea-write-into-adopted-state-is-an-operator-act--a-write-into-adopted-state-is-an-operator-act)
  - [`reconcile:the-fingerprint-covers-what-the-apply-would-do` — The fingerprint covers what the apply would do](#reconcilethe-fingerprint-covers-what-the-apply-would-do--the-fingerprint-covers-what-the-apply-would-do)
  - [`reconcile:a-plan-writes-nothing` — A plan writes nothing](#reconcilea-plan-writes-nothing--a-plan-writes-nothing)
  - [`reconcile:a-plan-is-stored-and-applied-by-its-id` — A plan is stored and applied by its id](#reconcilea-plan-is-stored-and-applied-by-its-id--a-plan-is-stored-and-applied-by-its-id)
  - [`reconcile:an-apply-refuses-a-plan-whose-inputs-moved` — An apply refuses a plan whose inputs moved](#reconcilean-apply-refuses-a-plan-whose-inputs-moved--an-apply-refuses-a-plan-whose-inputs-moved)
  - [`reconcile:one-writer-holds-a-target` — One writer holds a target](#reconcileone-writer-holds-a-target--one-writer-holds-a-target)
  - [`reconcile:a-run-that-did-not-finish-is-rolled-back` — A run that did not finish is rolled back](#reconcilea-run-that-did-not-finish-is-rolled-back--a-run-that-did-not-finish-is-rolled-back)
  - [`reconcile:an-apply-proves-every-postcondition-it-reports` — An apply proves every postcondition it reports](#reconcilean-apply-proves-every-postcondition-it-reports--an-apply-proves-every-postcondition-it-reports)
  - [`reconcile:a-destination-is-contained-before-it-is-written` — A destination is contained before it is written](#reconcilea-destination-is-contained-before-it-is-written--a-destination-is-contained-before-it-is-written)
  - [`reconcile:a-plan-id-is-a-fingerprint-and-never-a-path` — A plan id is a fingerprint and never a path](#reconcilea-plan-id-is-a-fingerprint-and-never-a-path--a-plan-id-is-a-fingerprint-and-never-a-path)
  - [`reconcile:an-apply-resolves-nothing` — An apply resolves nothing](#reconcilean-apply-resolves-nothing--an-apply-resolves-nothing)
  - [`reconcile:a-target-records-the-release-it-holds` — A target records the release it holds](#reconcilea-target-records-the-release-it-holds--a-target-records-the-release-it-holds)
  - [`reconcile:a-release-declares-what-it-asks-of-its-operator` — A release declares what it asks of its operator](#reconcilea-release-declares-what-it-asks-of-its-operator--a-release-declares-what-it-asks-of-its-operator)

<!--TOC-->

## Purpose

Rules governing the plan: one typed, immutable document that is the input to every write a landing verb makes into a target repository. They cover what the plan is, where its classification comes from, which findings it may report, how readiness is derived, what an operator decides, and what its identity covers. No instance adopts this spec: its subject is the engine, so an instance holding these rules holds obligations it cannot violate. What a release lands is stated in `SPEC-distribution.md`, and how a release is read is stated in `SPEC-bundle.md`.

## Requirements

### `reconcile:one-plan-is-the-input-to-every-write` — One plan is the input to every write

Every write into a target repository MUST come from an operation in one plan, and the operation set MUST be closed. Every front verb MUST reach that engine and MUST NOT carry a write path of its own. An operation MUST name digests and never bytes, MUST name one validated target-relative path, and MUST NOT run a command.

#### Scenario: A landing verb wants to write something the plan did not describe

- GIVEN a plan whose operations do not name a destination
- WHEN a landing verb runs, whichever one the operator typed
- THEN the destination is not written and the run leaves one recorded result under the plan's own id, because three verbs walking one projection on three paths is three places for a rule to drift and one plan is one

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:the-target-decides-its-classification` — The target decides its classification

The classification MUST be read from what was observed and never from the verb the operator ran. A front verb MAY refuse a classification it does not serve, and MUST NOT change one. Metadata that exists and cannot be trusted MUST classify as invalid, never as absent.

#### Scenario: A first landing is asked for over a settled corpus

- GIVEN a repository that already documents itself
- WHEN the operator runs the landing verb
- THEN the plan classifies a migration and the verb refuses it, because landing seeds over a settled corpus starts a second convention beside the first

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:the-planner-is-deterministic` — The planner is deterministic

The planner MUST be a pure function of its inputs. The observation, the release bundles, the selected decisions, and the clock MUST all be inputs, and the same inputs MUST produce the same plan and the same fingerprint.

#### Scenario: One plan is computed twice

- GIVEN an unchanged target and unchanged selections
- WHEN the plan is computed a second time
- THEN both plans carry the same fingerprint, because an approval that could not survive a recomputation would be an approval of nothing

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:a-finding-is-something-the-program-proved` — A finding is something the program proved

A finding MUST be structural or budget, and MUST name its path, its detector, and its measurement where it has one. A structural finding MUST be one the program can prove from the corpus. A document that predates the instance MUST be reported as a style candidate and MUST NOT be reported as a finding, because no delivered gate judges prose.

#### Scenario: A corpus written before the convention is planned

- GIVEN documents whose prose predates the instance
- WHEN the plan is computed
- THEN each is named as a style candidate and none is a finding, because an engine that called them findings would judge prose the convention refuses to judge

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:an-incremental-scope-leaves-no-structural-finding` — An incremental scope leaves no structural finding

The incremental migration scope MUST be offered only where the structural finding count is zero, and an incremental scope selected over a structural finding MUST block.

#### Scenario: A corpus carries a document named by its position

- GIVEN a migration whose corpus holds an ordinal-named document
- WHEN the plan is computed
- THEN only the sweep is offered, because a structural finding is exactly what makes two conventions coexist

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:readiness-is-the-worst-precondition` — Readiness is the worst precondition

Every precondition MUST carry a requirement and an evaluation, and the plan's readiness MUST be the worst verdict among them. A precondition nobody could evaluate MUST weigh as its requirement says, and MUST NOT pass as satisfied.

#### Scenario: A required precondition could not be evaluated

- GIVEN an observation that could not be made
- WHEN readiness is derived
- THEN the plan is blocked, because a gap is honest and is not permission

Verify: `cargo nextest run -E 'binary(canon)'`

### `reconcile:a-decision-precedes-what-depends-on-it` — A decision precedes what depends on it

Decisions MUST form an acyclic graph. Until an upstream decision is selected, the planner MUST omit the findings, decisions, and operations that depend on it rather than compute them against a default. A decision MUST declare its answer schema, and an unknown identifier, a malformed value, a repeated assignment, or an answer the plan no longer offers MUST be a usage error.

#### Scenario: A plan is computed before the profile is chosen

- GIVEN a first landing with no profile selected
- WHEN the plan is computed
- THEN it offers the profile decision and no profile-relative operation, because an operation against a profile nobody chose is a guess with a digest on it

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:a-write-into-adopted-state-is-an-operator-act` — A write into adopted state is an operator act

A write into a file the project owns MUST appear only when the decision that enables it is selected. A version moving MUST NOT imply one.

#### Scenario: An upgrade would record inherited violations

- GIVEN an upgrade the operator did not ask to reconcile
- WHEN the plan is computed
- THEN it carries no debt write and no rule append, because the project owns those files and a version bump is not consent

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:the-fingerprint-covers-what-the-apply-would-do` — The fingerprint covers what the apply would do

The fingerprint MUST be computed over a canonical encoding of the resolved release identity, the target identity, the record and declaration digests, every operation's kind, path, and digests, every precondition that can affect readiness or operations, and every selected decision. It MUST exclude timestamps, presentation text, and advisory evidence. Map ordering MUST NOT change it.

#### Scenario: Two plans differ only in when they were computed

- GIVEN one target planned twice a second apart
- WHEN the fingerprints are compared
- THEN they are equal, because an identity that moved with the clock would refuse every approval it was given

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:a-plan-writes-nothing` — A plan writes nothing

Computing a plan MUST write nothing into the target and MUST reach no network by default. A destination release the binary does not carry MAY be fetched, and the caller MUST have named it.

#### Scenario: A plan is computed with no arguments

- GIVEN a target and no destination
- WHEN the plan is computed
- THEN the target is byte-identical afterwards and no network read happened, because reading what would change must never change it

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:a-plan-is-stored-and-applied-by-its-id` — A plan is stored and applied by its id

A computed plan MUST be stored under its fingerprint, owner-only, outside the target, with every byte it will write. Identical inputs MUST reuse an existing executable plan. An apply MUST read the stored plan rather than recompute its intent, and a terminal result MUST free the fingerprint so identical inputs plan again into a fresh directory.

#### Scenario: One plan is computed twice and applied once

- GIVEN a target planned twice with the same inputs
- WHEN the second plan is computed
- THEN it carries the id of the first, because an operator who plans twice must be approving one thing

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:an-apply-refuses-a-plan-whose-inputs-moved` — An apply refuses a plan whose inputs moved

An apply MUST re-observe the target under the exclusive lock, recompute the fingerprint, and refuse on any difference, naming every field and destination that moved in one pass. It MUST proceed on ready alone, MUST name the unresolved decisions where it waits, and MUST name the failed preconditions where it is blocked. Nothing MUST be written before the revalidation passes.

#### Scenario: A managed file changes between the plan and the apply

- GIVEN a stored plan and a destination edited afterwards
- WHEN the apply runs
- THEN it refuses naming the destination and the target is byte-identical, because an apply that re-planned silently would do something nobody approved

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:one-writer-holds-a-target` — One writer holds a target

Planning MUST take a shared lock on the target and an apply MUST take it exclusively, for the whole run. A busy target MUST refuse at once with the holder, and no flag MUST bypass it. The target lock and the user-scope skill lock MUST be separate, because the two scopes have different owners.

#### Scenario: Two applies start together

- GIVEN one apply already running against a target
- WHEN a second starts
- THEN it refuses naming the holder, because two writers over one repository is the interleaving the lock exists to stop

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:a-run-that-did-not-finish-is-rolled-back` — A run that did not finish is rolled back

Every write an apply makes MUST be journaled before the first rename, and the journal MUST name each destination, the digest it held, and the digest the run intends. Any failure after the journal opens MUST attempt rollback and MUST record what the attempt achieved. The next invocation MUST recover an outstanding journal before it plans new work, and recovery MUST restore every destination the run touched, including the instance record.

#### Scenario: The process dies between two renames

- GIVEN an apply whose first operation landed and whose second did not
- WHEN the next invocation runs
- THEN it restores both destinations before planning anything, because a target holding half of one plan is a state no plan describes

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:an-apply-proves-every-postcondition-it-reports` — An apply proves every postcondition it reports

An apply MUST run a check behind every postcondition its plan declares and MUST NOT report one as held without it. A postcondition this engine has no check for MUST be reported as not held. A postcondition that fails MUST roll the run back.

#### Scenario: A plan declares a postcondition the engine cannot check

- GIVEN a stored plan naming a postcondition no check implements
- WHEN the apply finishes its operations
- THEN the result reports it as not held and the run rolls back, because a result that records proof nobody performed is worse than no result

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:a-destination-is-contained-before-it-is-written` — A destination is contained before it is written

Before any operation is staged, the engine MUST confirm that every path component under the target is not a symlink and that the destination is absent or a regular file. A destination that fails MUST refuse the whole apply before the first write, and no flag MUST bypass it.

#### Scenario: A directory under the target is a symlink elsewhere

- GIVEN a target whose documentation root is a symlink to another writable directory
- WHEN an apply that writes under that root runs
- THEN it refuses before staging anything, because a validated target-relative path is not containment and a rename through a symlink writes outside the repository

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:a-plan-id-is-a-fingerprint-and-never-a-path` — A plan id is a fingerprint and never a path

Every store entry MUST take its identifier as a validated lowercase hexadecimal sha256 and MUST refuse anything else before it reaches a path. A loaded plan whose own identifier is not the one it was fetched by MUST be refused.

#### Scenario: An operator is handed a plan id carrying a path component

- GIVEN an apply invoked with an id that is not a fingerprint
- WHEN the store resolves it
- THEN it refuses naming the id, because an identifier that reaches a join names a directory and a terminal result removes the directory its plan names

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:an-apply-resolves-nothing` — An apply resolves nothing

An apply MUST read the exact release its plan froze, offline, by version rather than by the selector that found it. It MUST NOT reach a registry, and MUST NOT fall back to a network read when the cache no longer holds the release.

#### Scenario: The newest release changes between the plan and the apply

- GIVEN a plan computed with `--to latest` and a newer release published afterwards
- WHEN the apply runs
- THEN it reads the release the plan named and not the newer one, because an approval bound to one release is not an approval of whatever is newest at apply time

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:a-target-records-the-release-it-holds` — A target records the release it holds

The instance record MUST name the release whose bytes landed and MUST NOT name the engine that landed them. A plan toward an older release MUST record that release.

#### Scenario: A newer engine lands an older release

- GIVEN a plan computed with an exact older destination
- WHEN it applies
- THEN the record names that older release, because every later classification, downgrade check, and interval reads this field and a false one makes all three wrong

Verify: `cargo nextest run -E 'binary(cmd_reconcile)'`

### `reconcile:a-release-declares-what-it-asks-of-its-operator` — A release declares what it asks of its operator

Every release MUST declare its minimum engine and the versions an interval may not skip, and MUST carry one guidance ledger entry naming a step file or `none`. The plan MUST turn each into a precondition or a decision, MUST keep only the steps whose destinations the target has, and MUST raise every breaking step as a decision the operator answers.

#### Scenario: An interval crosses a release that asks something of a person

- GIVEN a target two releases behind and a breaking step in between
- WHEN the plan is computed
- THEN it needs a decision carrying that step's body, because a write cannot take a step only a person can take

Verify: `cargo nextest run -E 'binary(cmd_reconcile) + binary(canon)'`
