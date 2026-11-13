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
  - [`distribution:an-instance-carries-the-skills` — An instance carries the skills](#distributionan-instance-carries-the-skills--an-instance-carries-the-skills)
  - [`distribution:an-unattributed-skill-folder-is-never-replaced` — An unattributed skill folder is never replaced](#distributionan-unattributed-skill-folder-is-never-replaced--an-unattributed-skill-folder-is-never-replaced)
  - [`distribution:a-skill-is-readable-before-an-instance` — A skill is readable before an instance](#distributiona-skill-is-readable-before-an-instance--a-skill-is-readable-before-an-instance)
  - [`distribution:a-skill-obeys-the-portable-format` — A skill obeys the portable format](#distributiona-skill-obeys-the-portable-format--a-skill-obeys-the-portable-format)
  - [`distribution:a-landing-classifies-its-target-first` — A landing classifies its target first](#distributiona-landing-classifies-its-target-first--a-landing-classifies-its-target-first)
  - [`distribution:a-skill-checks-its-host-before-it-plans` — A skill checks its host before it plans](#distributiona-skill-checks-its-host-before-it-plans--a-skill-checks-its-host-before-it-plans)
  - [`distribution:the-doctor-names-a-shadowing-user-copy` — The doctor names a shadowing user copy](#distributionthe-doctor-names-a-shadowing-user-copy--the-doctor-names-a-shadowing-user-copy)
  - [`distribution:a-skill-plans-before-it-acts` — A skill plans before it acts](#distributiona-skill-plans-before-it-acts--a-skill-plans-before-it-acts)
  - [`distribution:skill-uninstall-removes-only-what-it-wrote` — Skill uninstall removes only what it wrote](#distributionskill-uninstall-removes-only-what-it-wrote--skill-uninstall-removes-only-what-it-wrote)
  - [`distribution:a-skill-package-is-self-contained` — A skill package is self-contained](#distributiona-skill-package-is-self-contained--a-skill-package-is-self-contained)
  - [`distribution:the-payload-names-no-other-project` — The payload names no other project](#distributionthe-payload-names-no-other-project--the-payload-names-no-other-project)
  - [`distribution:a-declared-location-is-named-by-its-variable` — A declared location is named by its variable](#distributiona-declared-location-is-named-by-its-variable--a-declared-location-is-named-by-its-variable)
  - [`distribution:the-payload-roots-are-declared-once` — The payload roots are declared once](#distributionthe-payload-roots-are-declared-once--the-payload-roots-are-declared-once)
  - [`distribution:a-seeded-rule-runs-no-canon-command` — A seeded rule runs no canon command](#distributiona-seeded-rule-runs-no-canon-command--a-seeded-rule-runs-no-canon-command)
  - [`distribution:the-declaration-is-seeded-once-and-then-owned` — The declaration is seeded once and then owned](#distributionthe-declaration-is-seeded-once-and-then-owned--the-declaration-is-seeded-once-and-then-owned)
  - [`distribution:a-delivered-configuration-serves-a-delivered-rule` — A delivered configuration serves a delivered rule](#distributiona-delivered-configuration-serves-a-delivered-rule--a-delivered-configuration-serves-a-delivered-rule)

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

When a target is non-empty, the installer MUST preview its changes before writing any file. The preview a landing offers is the plan, and the classification decides which verb serves the target: a settled corpus is a migration, and the landing verb refuses it rather than seeding beside it.

#### Scenario: A repository has a hand-commented hook configuration

- GIVEN comments outside the managed markers
- WHEN initialization inserts its block
- THEN every outside comment remains byte-identical

Verify: `cargo nextest run -E 'binary(cmd_init)'`

### `distribution:instances-operate-offline` — Instances operate offline

The installed binary MUST verify and upgrade an instance without a network or canon checkout, reading the release it carries. Every verb reads that one release, and `SPEC-staging.md` states why the operator selects it rather than a selector. The readers `sdd docs`, `sdd method`, `sdd spec`, and `sdd template` serve the corpus the binary carries and touch no network, and `SPEC-docs-discovery.md` states what describes that corpus.

#### Scenario: The canon repository is unreachable

- GIVEN a fully installed target
- WHEN `sdd verify` and `sdd upgrade` run with no network
- THEN they check hashes, the managed block, rule IDs, and the binary's own version against the manifest, from the payload the binary carries

Verify: `cargo nextest run -E 'binary(cmd_verify) + binary(cmd_upgrade)'`

### `distribution:upgrade-conflicts-are-atomic` — Upgrade conflicts are atomic

If a managed file differs from its installed hash and the operator did not name it with `--replace`, then the upgrader MUST abort without changing the target. It names every conflict in one pass, and it decides before its first write, which `SPEC-staging.md` states as the rule a landing holds for any destination it cannot account for, beside the rule that keeps a named conflict's bytes.

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

The landing MUST own every skill at instance scope, so `sdd init` and `sdd upgrade` write each skill into the instance and no verb of this distribution writes a skill under the home directory.

An agent resolves a skill by name, and a second copy under one name is a second entry offering the same skill. The copy that matches the pin of the project is the one the instance carries, so a home copy is the one that goes. The record of the instance states the version in `canon_version`, and a skill file carries no version stamp.

#### Scenario: A project is initialized on a host that holds no skill

- GIVEN a home directory with no skill and a target repository with no instance
- WHEN `sdd init --apply` runs with either profile
- THEN the target carries both skills under `.claude/skills/` and `.agents/skills/`, the record lists each file, no file under the home changes, and `sdd skill install --apply` refuses and names the landing

Verify: `cargo nextest run -E 'test(init_lands_both_skills_into_both_roots) + test(skill_install_refuses_and_names_the_landing)'`

### `distribution:an-instance-carries-the-skills` — An instance carries the skills

Every landing MUST project each file of each skill package under both `.claude/skills/<name>/` and `.agents/skills/<name>/` as a managed entry byte-equal to the package, and `sdd verify` MUST judge each one against the one skill inventory the candidate reads.

The candidate derives the skill entries from the payload, so the projection declares no skill row. One function yields the inventory, so the candidate and the verifier cannot disagree on which entries an instance owes. An upgrade moves the skills with the instance, and the project commits them.

#### Scenario: A landed skill entry is removed

- GIVEN an instance whose record and tree both lost one landed skill file
- WHEN `sdd verify` runs
- THEN it fails naming the missing destination, because the inventory still expects it whatever the record says

Verify: `cargo nextest run -E 'test(a_landed_skill_is_byte_equal_to_the_package) + test(an_upgrade_brings_the_skills_to_an_older_instance) + test(an_edited_landed_skill_fails_verify) + test(a_removed_skill_entry_fails_verify)'`

### `distribution:an-unattributed-skill-folder-is-never-replaced` — An unattributed skill folder is never replaced

Where a skill package folder exists under either skill root and the record attributes none of its files, the landing, the preview of `sdd upgrade`, and `sdd stage compare` MUST refuse the whole folder, name it, and state that sdd lands its own copy there.

One function decides the refusal, and all three callers read it, so a preview and an apply agree. A folder that holds only an extra file is refused, and so is a folder whose files equal the candidate bytes but carry no record.

#### Scenario: A folder holds the same bytes the candidate carries

- GIVEN a target with `.claude/skills/sdd-setup/` holding the package bytes and no instance record
- WHEN `sdd stage compare` runs and then `sdd init --apply` runs
- THEN both name the same refused folder, and the apply writes nothing

Verify: `cargo nextest run -E 'test(a_landing_refuses_an_unattributed_skill_folder) + test(preview_and_apply_agree_on_a_refused_package)'`

### `distribution:a-skill-is-readable-before-an-instance` — A skill is readable before an instance

`sdd skill show` MUST print the `SKILL.md` of a skill, and with `--file <path>` each file of its package byte for byte, from the payload the binary carries, in a folder that holds no instance.

The setup skill lands the instance it lives in, so an agent in a project with no instance reads it from the binary. An unknown path exits 66.

#### Scenario: An agent starts a landing in a bare repository

- GIVEN a repository with no instance and a home with no skill copy
- WHEN the agent runs `sdd skill show sdd-setup --file references/pre-flight-gate.md`
- THEN it reads the authored bytes of the shared gate, so the skill does its own job before any copy exists on disk

Verify: `cargo nextest run -E 'test(skill_show_prints_one_reference_file)'`

### `distribution:a-skill-obeys-the-portable-format` — A skill obeys the portable format

Every skill MUST carry only the portable Agent Skills frontmatter fields, a `name` matching its directory name, and a manual and reference files within the character budget `SPEC-skill-format.md` states.

#### Scenario: A skill gains an agent-specific field

- GIVEN a skill edited to add a vendor-only frontmatter key
- WHEN the canon test suite runs
- THEN the conformance test fails and names the offending field

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:a-landing-classifies-its-target-first` — A landing classifies its target first

The landing MUST read a target's classification from the target alone, never from the verb the caller ran, and each front verb MUST refuse a classification it does not serve, naming what was found and what serves it. `sdd status --json` reports that one classification as `classification`, one of `setup`, `migration`, `needs-decision`, `upgrade`, `drift`, `current`, or `invalid`, or `null` where the instance is newer than the binary, and a skill reads it rather than routing by verb. `sdd assess` MUST keep its three-verdict projection: it reports its evidence and exactly one of `brownfield`, `greenfield`, or `needs-decision`, writes nothing, and exits 0.

#### Scenario: A landing verb meets a target it does not serve

- GIVEN a repository holding a populated documentation root and no instance manifest
- WHEN `sdd init --target . --profile codebase --apply` runs
- THEN the command refuses, names the target as a migration, and points at `sdd stage` and the setup skill, so no seed lands beside the corpus

Verify: `cargo nextest run -E 'binary(cmd_assess) + binary(cmd_status) + binary(cmd_stage) + binary(canon)'`

### `distribution:a-skill-checks-its-host-before-it-plans` — A skill checks its host before it plans

Every skill MUST direct the agent to run the shared pre-flight gate, which observes the host with `sdd doctor`, before planning, whatever the request's flags. The `--no-plan` flag changes only when the plan gate asks for approval.

#### Scenario: A request says to skip the checks

- GIVEN a request carrying `--no-plan` and an instruction to act immediately
- WHEN the agent follows the skill's opening section
- THEN the pre-flight still runs, because the task's steps have the same dependencies whatever the request says. Only the plan gate's approval turn is skipped

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:the-doctor-names-a-shadowing-user-copy` — The doctor names a shadowing user copy

`sdd doctor` MUST run every cataloged probe and exit 0 whatever they find, and its `skill-shadow` probe MUST fail while a `SKILL.md` of a payload skill exists under a home agent root, naming `sdd skill uninstall --apply`.

A personal skill wins over a project skill of the same name in one agent, and another agent shows both, so a home copy shadows the copy the instance pins. A folder that holds only notes passes. A manual the uninstall kept because the operator edited it gets a message to read it and remove it by hand.

#### Scenario: A home still holds a copy an earlier release installed

- GIVEN `~/.claude/skills/sdd-setup/SKILL.md` and an instance that carries its own copy
- WHEN `sdd doctor --json` runs
- THEN `skill-shadow` fails naming `sdd skill uninstall --apply`, and the exit code is 0

Verify: `cargo nextest run -E 'test(the_shadow_probe_names_a_user_scope_manual)'`

### `distribution:a-skill-plans-before-it-acts` — A skill plans before it acts

Every skill MUST open its body, after its opening paragraph, with one section that directs the agent to read the pre-flight gate before any other action, and MUST name the point after the pre-flight and before the first write at which the agent reads the plan gate, each gate named by its path inside the skill's own package. The plan gate holds its three phases, plan, validate, and execute, for the whole task.

#### Scenario: A skill gains a section above the pre-flight

- GIVEN a skill edited so another section precedes the pre-flight section
- WHEN the canon test suite runs
- THEN the conformance test fails naming the skill, because an agent acts on the first instruction it reads

Verify: `cargo nextest run -E 'binary(canon)'`

### `distribution:skill-uninstall-removes-only-what-it-wrote` — Skill uninstall removes only what it wrote

When run without `--apply`, `sdd skill uninstall` MUST list every removal and delete nothing. When applied, it MUST remove a file only where its current digest equals the one the receipt records, and MUST name every file it keeps with the reason. It MUST remove a package directory only once nothing else is left in it.

The verb cleans the home of the copies that earlier releases installed there. It reads a receipt of every schema those releases wrote, honors the variable that relocates the Claude configuration directory, and holds the user-scope lock for its whole run.

#### Scenario: The operator edited an installed skill

- GIVEN an installed skill package whose `SKILL.md` the operator has since rewritten
- WHEN `sdd skill uninstall --apply` runs
- THEN the edited file stays and is named as kept, because the receipt vouches for bytes and these are not those bytes

Verify: `cargo nextest run -E 'binary(cmd_skill)'`

### `distribution:a-skill-package-is-self-contained` — A skill package is self-contained

Every skill package MUST be one directory holding `SKILL.md`, and under `references/` the skill's own reference files and every shared artifact, materialized from the one authored source of each, and the payload MUST refuse a reference name that both sources claim. Every skill MUST name a reference by a path relative to its own root, and no skill may name one outside its own directory.

#### Scenario: A skill is landed under one skill root

- GIVEN an instance and a completed `sdd init --apply`
- WHEN the landed package is read
- THEN it holds `SKILL.md` and a `references/` directory carrying the skill's own reference files and both gates, so the skill resolves each the way the format resolves a supporting file

Verify: `cargo nextest run -E 'binary(cmd_skill) | test(a_skill_reference_lands_beside_the_shared_artifacts)'`

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

### `distribution:a-delivered-configuration-serves-a-delivered-rule` — A delivered configuration serves a delivered rule

Every configuration file the payload lands or seeds MUST set only what a delivered rule requires, so that no setting it carries contradicts a rule this tool delivers. A target that holds no markdownlint configuration of its own MUST pass every hook the landing wired, over the files that same landing wrote.

#### Scenario: A delivered linter configuration leaves the tool's defaults on

- GIVEN a delivered configuration naming one linter rule and a target with no configuration of its own
- WHEN the hook the landing wired runs over the seed the landing wrote
- THEN the seed fails on a default the payload never chose, so the adopter cannot commit the landing and reaches for the repairs the method forbids

Verify: `cargo nextest run -E 'binary(cmd_landed_tree)'`
