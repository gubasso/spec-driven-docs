# Staging Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`staging:the-operator-owns-acquisition` — The operator owns acquisition](#stagingthe-operator-owns-acquisition--the-operator-owns-acquisition)
  - [`staging:a-stage-writes-only-the-stage` — A stage writes only the stage](#staginga-stage-writes-only-the-stage--a-stage-writes-only-the-stage)
  - [`staging:a-stage-carries-the-whole-candidate` — A stage carries the whole candidate](#staginga-stage-carries-the-whole-candidate--a-stage-carries-the-whole-candidate)
  - [`staging:production-reads-no-staged-byte` — Production reads no staged byte](#stagingproduction-reads-no-staged-byte--production-reads-no-staged-byte)
  - [`staging:a-stage-persists-until-it-is-cleaned` — A stage persists until it is cleaned](#staginga-stage-persists-until-it-is-cleaned--a-stage-persists-until-it-is-cleaned)
  - [`staging:the-manifest-attributes-every-refresh` — The manifest attributes every refresh](#stagingthe-manifest-attributes-every-refresh--the-manifest-attributes-every-refresh)
  - [`staging:an-unattributed-collision-refuses-first` — An unattributed collision refuses first](#stagingan-unattributed-collision-refuses-first--an-unattributed-collision-refuses-first)
  - [`staging:the-manifest-is-written-last` — The manifest is written last](#stagingthe-manifest-is-written-last--the-manifest-is-written-last)
  - [`staging:one-writer-holds-the-target` — One writer holds the target](#stagingone-writer-holds-the-target--one-writer-holds-the-target)
  - [`staging:every-path-stays-under-the-target` — Every path stays under the target](#stagingevery-path-stays-under-the-target--every-path-stays-under-the-target)
  - [`staging:a-partial-failure-reports-what-it-completed` — A partial failure reports what it completed](#staginga-partial-failure-reports-what-it-completed--a-partial-failure-reports-what-it-completed)
  - [`staging:one-documentation-root-serves-the-run` — One documentation root serves the run](#stagingone-documentation-root-serves-the-run--one-documentation-root-serves-the-run)
  - [`staging:a-user-scope-receipt-shares-no-engine` — A user-scope receipt shares no engine](#staginga-user-scope-receipt-shares-no-engine--a-user-scope-receipt-shares-no-engine)
  - [`staging:owner-only-state-is-checked-on-linux` — Owner-only state is checked on Linux](#stagingowner-only-state-is-checked-on-linux--owner-only-state-is-checked-on-linux)

<!--TOC-->

## Purpose

Rules governing how the installed binary renders a candidate, how an agent inspects that candidate in a stage, and how production landing writes into a target. The candidate is the one version the running binary carries. The stage is a persistent directory of evidence the agent reads and production never does. No instance adopts this spec: its subject is the binary, so an instance holding these rules holds obligations it cannot violate and verifications it cannot run. What a project owes its own installation is stated in `SPEC-instance.md`. The ownership classes and the skill payload are stated in `SPEC-distribution.md`. How a consumer pins this tool is stated in `SPEC-acquisition.md`.

## Requirements

### `staging:the-operator-owns-acquisition` — The operator owns acquisition

Every landing verb MUST render the canon compiled into the running binary, and the operator MUST select and install the version they want before they run it.

Acquisition is a project-manager act. A landing verb that resolves, fetches, or installs another release would own two jobs, and the second one is the one the operator's own tooling already does.

#### Scenario: An operator wants a target landed at an older release

- GIVEN an operator who wants the shape release 0.8.0 lands
- WHEN they look for a selector on `sdd init` or `sdd upgrade`
- THEN there is none, and they install that release and run its own binary, because the release is the only honest witness of what it lands

Verify: `cargo nextest run -E 'binary(cmd_init) + binary(cmd_upgrade) + binary(canon)'`

### `staging:a-stage-writes-only-the-stage` — A stage writes only the stage

`sdd stage` MUST read the target and write nothing outside the stage directory it reports.

The stage path comes from the request, then the automation base the environment declares, then a default beneath this tool's own state root. The first of those that resolves wins, and the report names the one in force.

#### Scenario: A stage runs against a target holding an instance

- GIVEN a target carrying an installed instance and uncommitted edits
- WHEN `sdd stage --target .` runs
- THEN every target byte is unchanged and the stage holds the candidate, because staging exists to be compared against a target rather than to alter one

Verify: `cargo nextest run -E 'binary(cmd_stage)'`

### `staging:a-stage-carries-the-whole-candidate` — A stage carries the whole candidate

A stage MUST hold every instance destination the candidate would land, the canon references of the exact installed version, and a receipt naming the candidate version, the resolved configuration, and the destinations it rendered.

The candidate a stage renders takes the same choices the landing will: the profile, the documentation root, the documentation scratch, the reserved paths, and the writing-style selection. A stage rendered under other choices is evidence about a candidate nobody is going to write. The canon references are every payload root the binary carries that an adopting project reads while it migrates.

#### Scenario: An agent prepares a migration

- GIVEN a stage written against a target the agent has not yet landed
- WHEN the agent compares a destination in the stage against the file in the target
- THEN the comparison is complete for every destination, because a partial stage sends the agent back to the payload it was meant to replace

Verify: `cargo nextest run -E 'binary(cmd_stage)'`

### `staging:production-reads-no-staged-byte` — Production reads no staged byte

A landing MUST render the candidate again from the binary's own sources, and no landing verb MUST offer an argument that names a stage.

A stage is evidence for a reader. Bytes that reach production through a stage make the stage a cache, and a stale cache lands a version nobody chose. Refusing a stage argument is not enough, because an argument that exists is one somebody will pass: there is none.

#### Scenario: An operator looks for the way to apply what the agent inspected

- GIVEN a completed stage and a landing to run
- WHEN the operator reads `sdd init --help` and `sdd upgrade --help`
- THEN no flag takes a stage, the landing renders the candidate again, and the stage is still there afterwards to compare against

Verify: `cargo nextest run -E 'binary(cmd_stage) + binary(cmd_upgrade)'`

### `staging:a-stage-persists-until-it-is-cleaned` — A stage persists until it is cleaned

A stage MUST survive every check, landing, and verification that follows it, and only `sdd stage clean` MUST remove it, refusing where the path it was given is not a stage this tool wrote.

#### Scenario: A landing follows the stage that informed it

- GIVEN a stage and a successful `sdd upgrade --apply`
- WHEN the operator verifies the landed target
- THEN the stage is still readable, because the comparison an operator wants most is the one after the landing

Verify: `cargo nextest run -E 'binary(cmd_stage)'`

### `staging:the-manifest-attributes-every-refresh` — The manifest attributes every refresh

A landing MUST refresh a whole file or a marked region only where the instance manifest attributes it to this tool, and MUST leave adopted specs, decision records, policy, debt, project prose, and seeded state to the project.

#### Scenario: An upgrade meets an adopted spec the project has rewritten

- GIVEN an instance whose adopted spec carries the project's own requirements
- WHEN `sdd upgrade --apply` runs
- THEN the adopted spec is untouched and the managed files are refreshed, because the manifest records which of the two this tool wrote

Verify: `cargo nextest run -E 'binary(cmd_upgrade)'`

### `staging:an-unattributed-collision-refuses-first` — An unattributed collision refuses first

Where a destination holds a whole file the manifest does not attribute, the landing MUST refuse before its first write, naming every such destination in one run.

Missing provenance routes to agent-guided best effort. It never becomes automatic permission, because the file this tool cannot account for is exactly the file somebody else wrote.

#### Scenario: A target carries a managed destination from an unrecorded landing

- GIVEN a destination whose bytes no manifest entry vouches for
- WHEN `sdd init --apply` runs
- THEN it refuses naming that destination, writes no file, and routes the operator to the setup skill

Verify: `cargo nextest run -E 'binary(cmd_init)'`

### `staging:the-manifest-is-written-last` — The manifest is written last

A landing MUST write the instance manifest after every destination it landed, and that record MUST carry the producer version, the resolved configuration, each destination with its class or placement, and each current digest.

A record that describes a landing which did not finish is worse than no record, so the record is the last thing a successful run writes. Fields that exist only to compare one release against another leave the record with the engine that compared them.

#### Scenario: A landing fails after some destinations are written

- GIVEN a run that cannot write its last destination
- WHEN the failure is reported
- THEN the previous manifest is still in place, so the next run reads the state that was actually recorded rather than a half-written claim

Verify: `cargo nextest run -E 'binary(cmd_init) + binary(cmd_upgrade)'`

### `staging:one-writer-holds-the-target` — One writer holds the target

A landing MUST take one exclusive lock on the target before it observes anything, MUST hold it for the whole run, and MUST refuse at once naming the holder where another process holds it.

The lock comes first so that every observation the run acts on describes a target no other run of this tool is changing underneath it. It bounds cooperating processes and nothing else.

#### Scenario: Two landings start against one target

- GIVEN a landing already running against a target
- WHEN a second `sdd upgrade --apply` starts against the same target
- THEN the second refuses naming the holder, because two writers interleaving on one tree produce a state neither of them recorded

Verify: `cargo nextest run -E 'binary(cmd_upgrade)'`

### `staging:every-path-stays-under-the-target` — Every path stays under the target

Every path a landing writes MUST resolve beneath the target directory, checked component by component without following a link, and MUST be re-checked immediately before the write it guards. A replacement MUST happen in the directory of the file it replaces.

A link, a parent traversal, and an absolute destination each refuse the run. The re-check is what bounds the window between the decision and the write. It is a narrow window and not a closed one: this tool names no guarantee against an actor that can swap a component inside it, because a pathname check on a tree somebody else can rewrite is not a proof.

#### Scenario: A destination resolves outside the target

- GIVEN a destination that reaches outside the opened target through a link
- WHEN the landing reaches that destination
- THEN it refuses naming the path, because a tool that writes outside the tree it was pointed at has no boundary at all

Verify: `cargo nextest run -E 'binary(cmd_init) + binary(canon)'`

### `staging:a-partial-failure-reports-what-it-completed` — A partial failure reports what it completed

Where a landing fails partway, it MUST report every path it observed itself complete, MUST leave the previous manifest in place, and MUST name the rerun that finishes the work.

A reported rename failure is re-observed before it is believed, because a remote filesystem may have completed the rename it reported as failed. This tool states no universal filesystem atomicity and no all-or-nothing guarantee. What a partial run leaves is whole files, the previous record, a visible Git diff, and a command that runs again.

#### Scenario: A landing loses permission halfway through

- GIVEN a target whose later destination cannot be written
- WHEN the landing reaches it
- THEN the report names the completed paths and the failing one, and the operator reads the rest from the Git diff

Verify: `cargo nextest run -E 'binary(cmd_upgrade)'`

### `staging:one-documentation-root-serves-the-run` — One documentation root serves the run

The resolved documentation root the candidate will record MUST be the root that budget discovery measures, that the projection lands into, and that the delivered gate configuration declares.

#### Scenario: A project moves its documentation root during an upgrade

- GIVEN an instance recording an older root and a request that resolves a new one
- WHEN `sdd upgrade --apply` runs
- THEN findings, projection, and the landed declaration all name the new root, because a run that measures one root and configures another reports about a tree it did not write

Verify: `cargo nextest run -E 'binary(cmd_upgrade) + binary(cmd_status)'`

### `staging:a-user-scope-receipt-shares-no-engine` — A user-scope receipt shares no engine

Skill installation at user scope MUST keep its own receipt and its own writer, and no instance manifest may record what it wrote.

The two share a pattern, which is ownership and a receipt written last. They share no transaction record, because a home directory and a repository fail in different ways and recover under different hands.

#### Scenario: An instance is verified after a user-scope skill install

- GIVEN an installed instance and a completed `sdd skill install --apply`
- WHEN `sdd verify` runs against the instance
- THEN the report is unchanged by anything under the home directory

Verify: `cargo nextest run -E 'binary(cmd_skill) + binary(cmd_verify)'`

### `staging:owner-only-state-is-checked-on-linux` — Owner-only state is checked on Linux

Where this tool claims that host state is readable by its owner alone, it MUST prove that claim by reading the mode of the created path on the supported Linux target.

The claim is bounded by the target the project supports. It is stated for no other filesystem and no other operating system, because a mode this tool never reads is a promise it never kept.

#### Scenario: A confidentiality claim is added with a source-text check

- GIVEN a rule claiming owner-only state, checked by grepping the source for a mode constant
- WHEN the canon test suite runs
- THEN the check fails, because the constant in the source proves what the author typed rather than what the filesystem holds

Verify: `cargo nextest run -E 'binary(canon) + binary(cmd_stage)'`
