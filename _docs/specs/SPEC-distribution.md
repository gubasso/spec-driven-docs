# Distribution Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`distribution:manifest-identifies-every-owned-file` — The manifest identifies every owned file](#distributionmanifest-identifies-every-owned-file--the-manifest-identifies-every-owned-file)
  - [`distribution:initialization-preserves-project-content` — Initialization preserves project content](#distributioninitialization-preserves-project-content--initialization-preserves-project-content)
  - [`distribution:instances-operate-offline` — Instances operate offline](#distributioninstances-operate-offline--instances-operate-offline)
  - [`distribution:upgrade-conflicts-are-atomic` — Upgrade conflicts are atomic](#distributionupgrade-conflicts-are-atomic--upgrade-conflicts-are-atomic)
  - [`distribution:skills-are-part-of-the-payload` — Skills are part of the payload](#distributionskills-are-part-of-the-payload--skills-are-part-of-the-payload)
  - [`distribution:a-skill-has-one-owner` — A skill has one owner](#distributiona-skill-has-one-owner--a-skill-has-one-owner)
  - [`distribution:a-skill-obeys-the-portable-format` — A skill obeys the portable format](#distributiona-skill-obeys-the-portable-format--a-skill-obeys-the-portable-format)
  - [`distribution:a-landing-classifies-its-target-first` — A landing classifies its target first](#distributiona-landing-classifies-its-target-first--a-landing-classifies-its-target-first)
  - [`distribution:a-skill-checks-its-host-before-it-plans` — A skill checks its host before it plans](#distributiona-skill-checks-its-host-before-it-plans--a-skill-checks-its-host-before-it-plans)
  - [`distribution:the-doctor-answers-for-the-installed-skills` — The doctor answers for the installed skills](#distributionthe-doctor-answers-for-the-installed-skills--the-doctor-answers-for-the-installed-skills)
  - [`distribution:a-skill-plans-before-it-acts` — A skill plans before it acts](#distributiona-skill-plans-before-it-acts--a-skill-plans-before-it-acts)
  - [`distribution:skill-install-previews-before-writing` — Skill install previews before writing](#distributionskill-install-previews-before-writing--skill-install-previews-before-writing)
  - [`distribution:a-skill-install-restores-on-failure` — A skill install restores on failure](#distributiona-skill-install-restores-on-failure--a-skill-install-restores-on-failure)
  - [`distribution:skill-uninstall-removes-only-what-it-wrote` — Skill uninstall removes only what it wrote](#distributionskill-uninstall-removes-only-what-it-wrote--skill-uninstall-removes-only-what-it-wrote)
  - [`distribution:an-install-sweeps-what-the-payload-dropped` — An install sweeps what the payload dropped](#distributionan-install-sweeps-what-the-payload-dropped--an-install-sweeps-what-the-payload-dropped)
  - [`distribution:user-scope-files-stay-unrecorded` — User-scope files stay unrecorded](#distributionuser-scope-files-stay-unrecorded--user-scope-files-stay-unrecorded)
  - [`distribution:a-skill-package-is-self-contained` — A skill package is self-contained](#distributiona-skill-package-is-self-contained--a-skill-package-is-self-contained)
  - [`distribution:a-user-scope-receipt-is-required-state` — A user-scope receipt is required state](#distributiona-user-scope-receipt-is-required-state--a-user-scope-receipt-is-required-state)
  - [`distribution:the-payload-names-no-planning-tool` — The payload names no planning tool](#distributionthe-payload-names-no-planning-tool--the-payload-names-no-planning-tool)
  - [`distribution:the-payload-names-no-other-project` — The payload names no other project](#distributionthe-payload-names-no-other-project--the-payload-names-no-other-project)
  - [`distribution:a-declared-location-is-named-by-its-variable` — A declared location is named by its variable](#distributiona-declared-location-is-named-by-its-variable--a-declared-location-is-named-by-its-variable)
  - [`distribution:the-payload-roots-are-declared-once` — The payload roots are declared once](#distributionthe-payload-roots-are-declared-once--the-payload-roots-are-declared-once)
  - [`distribution:a-seeded-rule-runs-no-canon-command` — A seeded rule runs no canon command](#distributiona-seeded-rule-runs-no-canon-command--a-seeded-rule-runs-no-canon-command)
  - [`distribution:the-declaration-is-seeded-once-and-then-owned` — The declaration is seeded once and then owned](#distributionthe-declaration-is-seeded-once-and-then-owned--the-declaration-is-seeded-once-and-then-owned)

<!--TOC-->

## Purpose

Rules governing installation, ownership classes, offline verification, and upgrades. The distribution is one installed binary, `sdd`, that carries the payload, and every rule here binds whoever authors that binary. No instance adopts this spec: its subject is the installer, so an instance holding these rules holds obligations it cannot violate and verifications it cannot run. What a project owes its own installation is stated in `SPEC-instance.md`. The rules the canon alone runs at release time are stated in `SPEC-release.md`.

## Requirements

### `distribution:manifest-identifies-every-owned-file` — The manifest identifies every owned file

The installer MUST record each installed file with its ownership class, destination, and SHA-256, and the record MUST state which canon version produced it.

#### Scenario: An agent encounters a local edit

- GIVEN an installed file differs from its baseline
- WHEN the verifier reads the manifest
- THEN it distinguishes managed drift from adopted reconciliation

Verify: `cargo nextest run -E 'binary(cmd_verify) + binary(cmd_status)'`

### `distribution:initialization-preserves-project-content` — Initialization preserves project content

When a target is non-empty, the installer MUST preview its changes before writing any file.

#### Scenario: A repository has a hand-commented hook configuration

- GIVEN comments outside the managed markers
- WHEN initialization inserts its block
- THEN every outside comment remains byte-identical

Verify: `cargo nextest run -E 'binary(cmd_init)'`

### `distribution:instances-operate-offline` — Instances operate offline

The installed binary MUST verify and upgrade an instance without a network or canon checkout, reading the release it carries. A verb that reaches the registry MUST be one the operator asked for by naming another release, and `SPEC-bundle.md` states what that read owes.

#### Scenario: The canon repository is unreachable

- GIVEN a fully installed target
- WHEN `sdd verify` and `sdd upgrade` run with no network
- THEN they check hashes, the managed block, rule IDs, and the binary's own version against the manifest, from the payload the binary carries

Verify: `cargo nextest run -E 'binary(cmd_verify) + binary(cmd_upgrade)'`

### `distribution:upgrade-conflicts-are-atomic` — Upgrade conflicts are atomic

If a managed file differs from its installed hash, then the upgrader MUST abort without changing the target.

#### Scenario: One managed configuration is edited locally

- GIVEN a valid installed instance with one managed edit
- WHEN an upgrade is requested
- THEN it lists every conflict in one run and changes no target byte

Verify: `cargo nextest run -E 'binary(cmd_upgrade)'`

### `distribution:skills-are-part-of-the-payload` — Skills are part of the payload

The distribution MUST embed every skill authored under `skills/`, so a binary carries the skills of its own version and no instance fetches them.

#### Scenario: A skill is authored under the canon's skills directory

- GIVEN a skill directory added under `skills/`
- WHEN the binary is built
- THEN `sdd skill list` names it and `sdd skill show` prints it byte-identical to the authored file

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:a-skill-has-one-owner` — A skill has one owner

The distribution MUST install every skill at user scope alone. No profile can project a skill into an instance, because an agent resolves a skill by name. A second copy under one name is a second entry offering the same skill.

#### Scenario: A project is initialized inside a home that already carries the skills

- GIVEN a home directory holding the skills and a target repository with no instance
- WHEN `sdd init --apply` runs with either profile
- THEN the target carries no `.claude/skills/` or `.agents/skills/` file and the manifest records none, so each skill resolves to exactly one file

Verify: `cargo nextest run -E 'binary(cmd_init) + binary(cmd_skill)'`

### `distribution:a-skill-obeys-the-portable-format` — A skill obeys the portable format

Every skill MUST carry only the portable Agent Skills frontmatter fields, a `name` matching its directory name, and a body at or below 150 lines.

#### Scenario: A skill gains an agent-specific field

- GIVEN a skill edited to add a vendor-only frontmatter key
- WHEN the canon test suite runs
- THEN the conformance test fails and names the offending field

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:a-landing-classifies-its-target-first` — A landing classifies its target first

Where a setup or migration task finds no instance at a target, the skills and the shared pre-flight gate MUST route by `sdd assess`. The command MUST report its evidence and exactly one verdict, write nothing, and exit 0 for every produced classification. The three verdicts are: `brownfield` where a documentation root is populated or a methodology marker exists, `greenfield` where no document beyond root metadata exists, and `needs-decision` otherwise.

#### Scenario: A documented target carries no instance

- GIVEN a repository holding a populated documentation root and no instance manifest
- WHEN `sdd assess --target . --json` runs
- THEN the report classifies `brownfield` and exits 0, so the routing skill loads the migration path instead of landing seeds beside the corpus

Verify: `cargo nextest run -E 'binary(cmd_assess) + binary(canon)'`

### `distribution:a-skill-checks-its-host-before-it-plans` — A skill checks its host before it plans

Every skill MUST direct the agent to run the shared pre-flight gate, which observes the host with `sdd doctor`, before planning, whatever the request's flags. The `--no-plan` flag changes only when the plan gate asks for approval.

#### Scenario: A request says to skip the checks

- GIVEN a request carrying `--no-plan` and an instruction to act immediately
- WHEN the agent follows the skill's opening section
- THEN the pre-flight still runs, because the task's steps have the same dependencies whatever the request says. Only the plan gate's approval turn is skipped

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:the-doctor-answers-for-the-installed-skills` — The doctor answers for the installed skills

`sdd doctor` MUST run every cataloged probe and exit 0 whatever they find. Its skill probes MUST compare whole packages, file by file, and MUST pick the remediation by the user-scope receipt: drift the receipt vouches for is a stale install corrected by a plain apply. Drift it cannot account for is the user's own, corrected only with `--force`.

#### Scenario: A home holds a skill an older release installed

- GIVEN an agent root holding a package file whose digest the user-scope receipt vouches for
- WHEN `sdd doctor --json` runs
- THEN the `skill-payload` probe fails naming `sdd skill install --apply` without `--force`, and the exit code is 0

Verify: `cargo nextest run -E 'binary(cmd_doctor)'`

### `distribution:a-skill-plans-before-it-acts` — A skill plans before it acts

Every skill MUST open its body with one section that precedes every other section. That section MUST direct the agent to read the two gates in order before the first action of a task, each named by its path inside the skill's own package: the pre-flight gate first, then the plan gate. It MUST also direct the agent to hold the plan gate's three phases, plan, validate, and execute, for the whole task.

#### Scenario: A skill gains a section above the gate

- GIVEN a skill edited so another section precedes the gate section
- WHEN the canon test suite runs
- THEN the conformance test fails naming the skill, because an agent acts on the first instruction it reads

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:skill-install-previews-before-writing` — Skill install previews before writing

When run without `--apply`, `sdd skill install` MUST list every destination and write nothing. Three references decide each one: bytes matching the payload are current, bytes the receipt vouches for are this tool's and are replaced without `--force`, and every other byte is the user's. An apply MUST refuse atomically on the last kind, listing every conflict.

#### Scenario: A home directory already carries an edited skill

- GIVEN `~/.claude/skills/sdd-setup/SKILL.md` with bytes the user wrote
- WHEN `sdd skill install --apply` runs
- THEN it exits 73 listing every conflicting destination, writes no file, and states `--force` as the override

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:a-skill-install-restores-on-failure` — A skill install restores on failure

An apply of `sdd skill install` or `sdd skill uninstall` MUST hold the user-scope lock for its whole run and MUST refuse at once, naming the holder, where another process holds it. It MUST write a journal before its first replacement, MUST restore every destination it backed up where it fails partway, and MUST roll back a run the process did not finish before it plans new work. It MUST refuse a destination reached through a link, whatever `--force` says, and MUST re-check that immediately before each write. Recovery after the process is killed is guaranteed at every one of those boundaries. Recovery after power loss rests on the persistence order and on the platform's sync semantics, and is claimed no further.

#### Scenario: The second skill root cannot be written

- GIVEN two skill roots, the second holding a destination the process cannot write
- WHEN `sdd skill install --apply` has already rewritten the first root
- THEN it exits 73 naming the unwritable path and leaves both roots as found. It leaves them because one agent reading a newer skill than another is worse than neither being upgraded

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:skill-uninstall-removes-only-what-it-wrote` — Skill uninstall removes only what it wrote

When run without `--apply`, `sdd skill uninstall` MUST list every removal and delete nothing. When applied, it MUST remove a file only where its current digest equals the one the receipt records, and MUST name every file it keeps with the reason. It MUST remove a package directory only once nothing else is left in it.

#### Scenario: The operator edited an installed skill

- GIVEN an installed skill package whose `SKILL.md` the operator has since rewritten
- WHEN `sdd skill uninstall --apply` runs
- THEN the edited file stays and is named as kept, because the receipt vouches for bytes and these are not those bytes

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:an-install-sweeps-what-the-payload-dropped` — An install sweeps what the payload dropped

Where the user-scope receipt vouches for a destination the current payload no longer carries, `sdd skill install --apply` and `sdd skill uninstall --apply` MUST remove it and the directory it empties. Both MUST leave a destination the receipt cannot vouch for alone, and the doctor MUST name what they left.

#### Scenario: A release renames a skill

- GIVEN a home directory holding a skill under its old name, recorded by the apply that wrote it
- WHEN a newer `sdd skill install --apply` runs
- THEN the old name's `SKILL.md` and its directory are gone, the new name is installed, and only one entry is offered under either name

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:user-scope-files-stay-unrecorded` — User-scope files stay unrecorded

Files `sdd skill install` writes outside an instance MUST NOT appear in any instance manifest. The payload and the user-scope record are the references the installer compares them against, and no verification reads either.

#### Scenario: An instance is verified after a user-scope install

- GIVEN an installed instance and a completed `sdd skill install --apply`
- WHEN `sdd verify` runs against the instance
- THEN the report is unchanged by anything under the home directory

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:a-skill-package-is-self-contained` — A skill package is self-contained

Every installed skill MUST be one directory holding `SKILL.md` and every shared artifact under `references/`, materialized by the installer from the one authored source. Every skill MUST name a shared artifact by a path relative to its own root, and no skill may name one outside its own directory.

#### Scenario: A skill is installed under one agent root

- GIVEN an agent root and a completed `sdd skill install --apply`
- WHEN the installed package is read
- THEN it holds `SKILL.md` and a `references/` directory carrying both gates, so the skill resolves them the way the format resolves a supporting file

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:a-user-scope-receipt-is-required-state` — A user-scope receipt is required state

An apply that cannot write the user-scope receipt MUST fail and MUST roll back every file it wrote. A receipt that vouches for nothing MUST be removed rather than left empty.

#### Scenario: The receipt cannot be replaced

- GIVEN an apply that has already replaced every package file
- WHEN the receipt cannot be written
- THEN the apply fails and every destination goes back, because a landing this tool cannot vouch for is a landing it would later refuse to take back

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:the-payload-names-no-planning-tool` — The payload names no planning tool

The author MUST keep planning tool names out of every embedded payload root, so instances can pair this framework with any work-record convention or none.

#### Scenario: A method chapter names the tool it was tested against

- GIVEN a chapter edited to illustrate the seam with one planning tool by name
- WHEN the canon test suite runs
- THEN the check fails naming the file and the term, because a framework that names one tool stops being pairable with another

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:the-payload-names-no-other-project` — The payload names no other project

The author MUST keep every embedded payload root free of the name of any project, repository, or organization outside this one. The forges, agents, and reference works the method documents as integrations are the exception.

#### Scenario: A chapter carries an example from the repository it was drafted in

- GIVEN a chapter or skill that names a sibling project while illustrating a rule
- WHEN the canon test suite runs
- THEN the check fails naming the file and the term, because a reader who lacks that project meets a reference they cannot follow

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:a-declared-location-is-named-by-its-variable` — A declared location is named by its variable

The author MUST keep the retired name of a declared location out of the authored corpus. A path the binary reports MUST be named by the report rather than restated, so no skill and no chapter spells one.

#### Scenario: A chapter reintroduces the docs scratch's old fixed path

- GIVEN a chapter edited to name the retired directory, or a skill edited to spell a path the status report carries
- WHEN the canon test suite runs
- THEN the check fails naming the file and the path, because a corpus that fixes the location has taken the declaration back

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:the-payload-roots-are-declared-once` — The payload roots are declared once

The author MUST declare the embedded payload roots in one place that the binary, the build script, and the canon suite all read.

#### Scenario: An eighth root is embedded

- GIVEN a new root added to the embedding module alone
- WHEN the canon suite scans the payload for what it must not carry
- THEN the scan walks a list that no longer describes the payload, so the root ships unscanned unless one declaration feeds all three

Verify: `cargo nextest run -E 'kind(lib)'`

### `distribution:a-seeded-rule-runs-no-canon-command` — A seeded rule runs no canon command

Where a spec is seeded into an instance, the author MUST keep `cargo` and `just` out of every shell command its verification lines carry.

#### Scenario: A canon-only rule is left in a seeded spec

- GIVEN a seeded spec carrying a rule verified by a cargo test
- WHEN the canon test suite runs
- THEN the check fails naming the spec and the command, because the adopter reads an unrunnable verification as work it owes

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:the-declaration-is-seeded-once-and-then-owned` — The declaration is seeded once and then owned

`sdd init` MUST write `.spec-driven-docs/config.yaml` in every case, record it among the adopted files, and never overwrite it again. An upgrade MUST carry it forward unchanged and re-render the managed blocks from it.

The boundary is the actor, not the file. An automatic write to an adopted file during install or upgrade is forbidden. An operator-invoked command that previews a specific change first and writes it on request is not: `sdd debt tighten --apply` lowers the project's debt file, and `sdd policy reconcile --apply` appends to an adopted specification the rule that authorizes a declaration the project made, and each updates the record for the file it changed.

#### Scenario: An instance that declares an exclusion is upgraded

- GIVEN an instance whose declaration reserves a path
- WHEN `sdd upgrade` runs
- THEN the upgrade neither conflicts on the file nor drops the reservation, because the declaration is adopted rather than managed and the block is rendered rather than hand-edited

Verify: `pre-commit run cargo-test --all-files`
