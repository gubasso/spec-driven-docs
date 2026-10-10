# Staging Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`staging:the-operator-owns-acquisition` — The operator owns acquisition](#stagingthe-operator-owns-acquisition--the-operator-owns-acquisition)
  - [`staging:a-stage-writes-only-the-stage` — A stage writes only the stage](#staginga-stage-writes-only-the-stage--a-stage-writes-only-the-stage)
  - [`staging:a-stage-carries-the-whole-candidate` — A stage carries the whole candidate](#staginga-stage-carries-the-whole-candidate--a-stage-carries-the-whole-candidate)
  - [`staging:production-reads-no-staged-byte` — Production reads no staged byte](#stagingproduction-reads-no-staged-byte--production-reads-no-staged-byte)
  - [`staging:a-stage-takes-the-recorded-choices` — A stage takes the recorded choices](#staginga-stage-takes-the-recorded-choices--a-stage-takes-the-recorded-choices)
  - [`staging:a-landing-takes-the-choices-its-stage-recorded` — A landing takes the choices its stage recorded](#staginga-landing-takes-the-choices-its-stage-recorded--a-landing-takes-the-choices-its-stage-recorded)
  - [`staging:compare-reports-every-destination-the-stage-carries` — Compare reports every destination the stage carries](#stagingcompare-reports-every-destination-the-stage-carries--compare-reports-every-destination-the-stage-carries)
  - [`staging:a-replacement-keeps-the-bytes-it-replaces` — A replacement keeps the bytes it replaces](#staginga-replacement-keeps-the-bytes-it-replaces--a-replacement-keeps-the-bytes-it-replaces)
  - [`staging:an-upgrade-lists-every-retired-citation-first` — An upgrade lists every retired citation first](#stagingan-upgrade-lists-every-retired-citation-first--an-upgrade-lists-every-retired-citation-first)
  - [`staging:a-stage-persists-until-it-is-cleaned` — A stage persists until it is cleaned](#staginga-stage-persists-until-it-is-cleaned--a-stage-persists-until-it-is-cleaned)
  - [`staging:the-manifest-attributes-every-refresh` — The manifest attributes every refresh](#stagingthe-manifest-attributes-every-refresh--the-manifest-attributes-every-refresh)
  - [`staging:an-unattributed-collision-refuses-first` — An unattributed collision refuses first](#stagingan-unattributed-collision-refuses-first--an-unattributed-collision-refuses-first)
  - [`staging:the-manifest-is-written-last` — The manifest is written last](#stagingthe-manifest-is-written-last--the-manifest-is-written-last)
  - [`staging:one-writer-holds-the-target` — One writer holds the target](#stagingone-writer-holds-the-target--one-writer-holds-the-target)
  - [`staging:every-path-stays-under-the-target` — Every path stays under the target](#stagingevery-path-stays-under-the-target--every-path-stays-under-the-target)
  - [`staging:a-partial-failure-reports-what-it-completed` — A partial failure reports what it completed](#staginga-partial-failure-reports-what-it-completed--a-partial-failure-reports-what-it-completed)
  - [`staging:one-documentation-root-serves-the-run` — One documentation root serves the run](#stagingone-documentation-root-serves-the-run--one-documentation-root-serves-the-run)
  - [`staging:a-seed-lands-rendered-for-its-root` — A seed lands rendered for its root](#staginga-seed-lands-rendered-for-its-root--a-seed-lands-rendered-for-its-root)
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

The candidate a stage renders takes the same choices the landing will: the profile, the documentation root, the documentation scratch, the reserved paths, and the writing-style selection. A stage rendered under other choices is evidence about a candidate nobody is going to write.

The canon references are every payload root the binary carries that an adopting project reads while it migrates, plus this version's release notes. A skill is copied as the package an agent resolves rather than as the authored tree, because a skill names its own supporting files relative to its own root.

#### Scenario: An agent prepares a migration

- GIVEN a stage written against a target the agent has not yet landed
- WHEN the agent compares a destination in the stage against the file in the target
- THEN the comparison is complete for every destination, because a partial stage sends the agent back to the payload it was meant to replace

Verify: `cargo nextest run -E 'binary(cmd_stage)'`

### `staging:production-reads-no-staged-byte` — Production reads no staged byte

A landing MUST render the candidate again from the binary's own sources, and MUST read no byte of a stage but the choices its receipt records.

A stage is evidence for a reader. Bytes that reach production through a stage make the stage a cache, and a stale cache lands a version nobody chose. The choices a receipt records are not bytes a landing writes: they are the answers the operator gave once, at the stage, so the landing and the stage cannot disagree about them.

#### Scenario: An operator applies what the agent inspected

- GIVEN a completed stage whose artifacts nobody can read, and a landing to run
- WHEN the operator runs `sdd upgrade`, with or without `--stage` naming that stage
- THEN the landing renders the candidate again from the binary, and the stage is still there afterwards to compare against

Verify: `cargo nextest run -E 'binary(cmd_stage) + binary(cmd_upgrade)'`

### `staging:a-stage-takes-the-recorded-choices` — A stage takes the recorded choices

Where the target carries an instance, `sdd stage` MUST project the profile, the documentation scratch, the writing style, and the reserved paths the instance records, and MUST refuse a `--profile` that differs from the recorded one.

A stage over an instance is evidence for that instance's next landing. A profile change moves the documentation root, which is a migration somebody asks for by name, never an upgrade's candidate.

#### Scenario: A knowledge-base instance is staged with no flag

- GIVEN an instance that records the `knowledge-base` profile
- WHEN `sdd stage --target .` runs, and then `sdd stage --target . --profile codebase`
- THEN the first stages a `knowledge-base` candidate, and the second refuses naming both profiles

Verify: `cargo nextest run -E 'binary(cmd_stage)'`

### `staging:a-landing-takes-the-choices-its-stage-recorded` — A landing takes the choices its stage recorded

Where `sdd init` or `sdd upgrade` names a stage with `--stage`, the landing MUST take the profile, the documentation scratch, the writing style, the reserved paths, and the acquisition decision from that stage's receipt, MUST refuse a flag that sets one of them, and MUST refuse a receipt another release wrote or a receipt for another target.

The operator states each choice once, at the stage, so the bytes the landing writes are the bytes the stage showed. A receipt is ordinary JSON, so it authorizes choices and never bytes.

#### Scenario: A stage from an older binary meets a newer one

- GIVEN a stage rendered by an older `sdd`, and a newer `sdd` installed since
- WHEN `sdd init --stage <root> --apply` runs
- THEN it refuses naming both versions, writes nothing, and the operator stages again

Verify: `cargo nextest run -E 'binary(cmd_init) + binary(cmd_upgrade)'`

### `staging:compare-reports-every-destination-the-stage-carries` — Compare reports every destination the stage carries

`sdd stage compare <root>` MUST report one row per destination the stage carries and one per recorded managed file the candidate retires, each with its state, its ownership, and, where the bytes differ or retire, its provenance, and MUST judge the record half with the check the landing refuses on.

The state is `equal`, `absent`, `differs`, or `retired`. The ownership is `managed`, `integration`, or `adopted`, and a marked region is compared by its own bytes. The provenance is `attributed` where the record vouches for the bytes and the history touches the path, `reduced` where one of the two holds, and `unknown` where neither does. Where Git is absent or the target is no repository, the report states why the history half is missing. Each row also carries `replaceable`, true only where the verb that lands the stage accepts the path in `--replace`, judged by the check `sdd upgrade` applies to each name: the path is a conflict that verb lists, and the landing can write it. It is false on every other row, and on every row before an instance exists, because `sdd init` takes no name.

#### Scenario: A landing follows a comparison

- GIVEN a stage, a target with one edited managed file, and a comparison
- WHEN the landing runs against the same target
- THEN it refuses exactly the rows the comparison reported as unvouched, and after a landing with the conflict resolved, every managed and integration row reads `equal`

Verify: `cargo nextest run -E 'binary(cmd_stage)'`

### `staging:a-replacement-keeps-the-bytes-it-replaces` — A replacement keeps the bytes it replaces

Where `sdd upgrade --replace <path>` names a conflict, the upgrade MUST save the bytes the target holds there before its first write, MUST report where, MUST refuse while any conflict it was not given remains, and MUST refuse a name that is no conflict.

The copy goes to `replaced/<path>` under the docs scratch the instance declares, or under a snapshot directory beneath the state root where none is declared, and never into a stage. A copy already there with other bytes refuses rather than being overwritten, so no byte is lost.

#### Scenario: An operator keeps one edit and discards another

- GIVEN two edited managed files, one named with `--replace`
- WHEN `sdd upgrade --replace <path>` runs
- THEN it refuses naming the other, writes nothing, and a second run naming both saves each before it replaces it

Verify: `cargo nextest run -E 'binary(cmd_upgrade)'`

### `staging:an-upgrade-lists-every-retired-citation-first` — An upgrade lists every retired citation first

`sdd upgrade --dry-run` MUST list every citation of a rule ID in a domain this binary defines that this binary no longer defines, by file and line, before any write.

A rule heading is a definition and is not listed. A decision record is read on its `Enforced by` line alone, a file this tool manages is about to be replaced, and the changelog is history, so none of those is listed. Re-pointing a citation stays the author's, because the tool cannot know which rule the author meant.

#### Scenario: A guide cites a rule the new release retired

- GIVEN a guide citing a rule ID the installed release no longer defines
- WHEN `sdd upgrade --dry-run --json` runs
- THEN `retired_citations` names the guide, the line, and the ID, and nothing is written

Verify: `cargo nextest run -E 'binary(cmd_upgrade)'`

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

A landing MUST take one exclusive lock on the target before it reads the bytes it will write from, MUST hold it for the whole run, and MUST refuse at once naming the holder where another process holds it.

The lock precedes the observation the run acts on, so the bytes a marked region splices into cannot move between the read and the write. A report computed before the lock, such as the drift a verb prints before it decides to land, is advice rather than authority: the writer re-checks under the lock and refuses there. The lock bounds cooperating processes and nothing else.

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

### `staging:a-seed-lands-rendered-for-its-root` — A seed lands rendered for its root

The candidate MUST substitute the resolved documentation root for every root placeholder in an adopted seed's content and destination, and MUST record the rendered bytes as the seed's baseline.

A seed is copied once, so the landing is the one moment that knows the root it names. The rendering never rewrites an adopted file that already exists, because `distribution:the-declaration-is-seeded-once-and-then-owned` gives those bytes to the project.

#### Scenario: A codebase instance receives the tracking registry

- GIVEN a codebase profile whose documentation root is `docs`
- WHEN `sdd init --apply` lands the tracking template
- THEN its example names `docs/reference/model-pricing.md` and the record's baseline is the digest of those bytes, because an example naming a root the instance does not have fails the gate that judges it

Verify: `cargo nextest run -E 'binary(cmd_landed_tree) + binary(cmd_policy)'`

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
