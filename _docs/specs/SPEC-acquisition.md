# Acquisition Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`acquisition:every-pair-carries-a-verdict` — Every pair carries a verdict](#acquisitionevery-pair-carries-a-verdict--every-pair-carries-a-verdict)
  - [`acquisition:a-fragment-names-the-venue-form` — A fragment names the venue form](#acquisitiona-fragment-names-the-venue-form--a-fragment-names-the-venue-form)
  - [`acquisition:status-reports-and-never-judges` — Status reports and never judges](#acquisitionstatus-reports-and-never-judges--status-reports-and-never-judges)
  - [`acquisition:the-tool-serves-and-does-not-edit` — The tool serves and does not edit](#acquisitionthe-tool-serves-and-does-not-edit--the-tool-serves-and-does-not-edit)
  - [`acquisition:one-target-runs-one-mechanism` — One target runs one mechanism](#acquisitionone-target-runs-one-mechanism--one-target-runs-one-mechanism)
  - [`acquisition:the-pin-moves-in-one-transaction` — The pin moves in one transaction](#acquisitionthe-pin-moves-in-one-transaction--the-pin-moves-in-one-transaction)
  - [`acquisition:the-shell-entry-caller-is-rate-limited-and-silent` — The shell-entry caller is rate-limited and silent](#acquisitionthe-shell-entry-caller-is-rate-limited-and-silent--the-shell-entry-caller-is-rate-limited-and-silent)
  - [`acquisition:the-operator-caller-reports-every-outcome` — The operator caller reports every outcome](#acquisitionthe-operator-caller-reports-every-outcome--the-operator-caller-reports-every-outcome)
  - [`acquisition:the-sync-leaves-a-diff-nobody-committed` — The sync leaves a diff nobody committed](#acquisitionthe-sync-leaves-a-diff-nobody-committed--the-sync-leaves-a-diff-nobody-committed)
  - [`acquisition:clean-removes-only-a-named-leftover` — Clean removes only a named leftover](#acquisitionclean-removes-only-a-named-leftover--clean-removes-only-a-named-leftover)

<!--TOC-->

## Purpose

Rules governing how a consumer obtains this tool and keeps its pin fresh. A project pins `sdd` through the manager it already runs, at a version its manager file records, and `sdd self-depend` is the verb set that reads the pin, serves the fragment a manager needs to take one, and moves the pin to a newer release. The rules bind whoever authors that verb set. No instance adopts this spec: its subject is the binary, so an instance holding these rules holds obligations it cannot violate and verifications it cannot run.

Two independent facts decide every verdict here. A manager is what the project declares its development tools in: a Nix flake input, mise, asdf, or devbox. A shell loader such as direnv is not a manager, because it records no version. A venue is where a release of this tool is published: the crate on its registry, the flake this repository serves at every tag, or the archives attached to each forge release. A venue joins the list only where this repository's own release path publishes to it.

## Requirements

### `acquisition:every-pair-carries-a-verdict` — Every pair carries a verdict

The binary MUST hold one verdict for every manager and venue pair: the pair renders, or the pair is manual with a reason from a closed set. The set is no package-set attribute known, a source hash needed that the tool cannot compute offline, no backend for the venue, and no published plugin. The matrix MUST be stated once, in the binary, and restated in no chapter and no skill.

#### Scenario: A manager gains a venue nobody classified

- GIVEN a manager enum with a new variant
- WHEN the matrix test runs
- THEN it fails until every pair the variant forms carries a verdict

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:a-fragment-names-the-venue-form` — A fragment names the venue form

A fragment for a registry-backed pair MUST name the crate as the registry knows it, and a fragment for an archive-backed pair MUST name the binary inside the archive. A flake pair MUST name the repository at a release tag.

#### Scenario: The crate name and the binary name differ

- GIVEN a mise entry for the registry
- WHEN the fragment is rendered
- THEN it names the crate, and the archive form beside it names the binary as its executable

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:status-reports-and-never-judges` — Status reports and never judges

`sdd self-depend status` MUST report offline, one entry per manager in the enum's order with the absent ones included, the shell-entry line and its presence, the stamp, the host probes, and any leftover. It MUST exit 0 for every state it reports, because a report is not a verdict, and it MUST offer `--json`.

#### Scenario: A target pins nothing

- GIVEN a target with no manager file at all
- WHEN status runs
- THEN it reports every manager absent, states the target unwired, and exits 0

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:the-tool-serves-and-does-not-edit` — The tool serves and does not edit

`sdd self-depend add` MUST print each fragment with its anchor and its placement and MUST edit no file the project owns. It MUST seed a file only where the target has none, only with `--apply`, and the shell loader only for the flake pair, because that is the pair where the loader is what puts the binary on the path.

#### Scenario: The target already carries the manager file

- GIVEN a `flake.nix` with no input for this tool
- WHEN add runs with `--apply`
- THEN the fragments are printed, and the file is byte-identical afterwards

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:one-target-runs-one-mechanism` — One target runs one mechanism

A target MUST run one bump mechanism. `add` MUST refuse a manager other than the one already naming this tool, `sync` MUST refuse to choose where several managers name it, and `status` MUST report `ready` only where the line is landed and no leftover remains.

#### Scenario: A second manager is asked for

- GIVEN a target whose mise file already pins this tool
- WHEN add runs with `--manager flake`
- THEN it refuses, names mise as the manager that pins the tool, and writes nothing

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:the-pin-moves-in-one-transaction` — The pin moves in one transaction

Where the manager records two facts, `sync` MUST move the tag and the locked node together or move neither, restoring both files byte-identical on any failure. Where the manager records one fact, it MUST move that one fact in place. The flake input MUST be found by the URL it names, never by the input's name.

#### Scenario: The lock refresh fails

- GIVEN a flake pinned at a tag and a lock beside it
- WHEN the tag is rewritten and the lock refresh exits non-zero
- THEN both files read exactly as they did before the run, and the failure names both

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:the-shell-entry-caller-is-rate-limited-and-silent` — The shell-entry caller is rate-limited and silent

With `--caller envrc`, the default, `sync` MUST run at most one attempt a day per checkout, held by a stamp under the state root, MUST print nothing unless `--json` asks, and MUST exit 0 on every outcome, a failed sync included. It MUST run nothing where `CI` or `SDD_SELF_DEPEND_OFF` is set.

#### Scenario: A second directory entry within the day

- GIVEN a checkout whose stamp records today
- WHEN the shell loader runs the line again
- THEN nothing is read, nothing moves, and the exit code is 0

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:the-operator-caller-reports-every-outcome` — The operator caller reports every outcome

With `--caller operator`, `sync` MUST report every outcome and MUST fail with a distinct exit code for a target that pins nothing, a release that cannot be read, and a transaction that was put back.

#### Scenario: The target pins nothing

- GIVEN a target with no manager naming this tool
- WHEN sync runs with `--caller operator`
- THEN it exits with the usage code and names the verb that serves the fragments

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:the-sync-leaves-a-diff-nobody-committed` — The sync leaves a diff nobody committed

`sync` MUST write only the manager file and its lock, MUST name the same `from` and `to` in the form the wired manager records, and MUST neither commit nor push.

#### Scenario: A one-fact manager moves

- GIVEN a `.tool-versions` pinning this tool at an older version
- WHEN sync runs with `--apply` and `--tag`
- THEN that line names the new version, no other line changed, and no other file changed

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:clean-removes-only-a-named-leftover` — Clean removes only a named leftover

`sdd self-depend clean` MUST remove only a file the predecessor catalog or `--also` names, only with `--apply`, and MUST name every file it leaves byte-identical with the reason, the sync line among them.

#### Scenario: The operator names a predecessor's script

- GIVEN a target carrying a script no catalog knows
- WHEN clean runs with `--also` naming it and without `--apply`
- THEN the script is listed as a leftover, the sync line is listed as kept, and nothing is removed

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`
