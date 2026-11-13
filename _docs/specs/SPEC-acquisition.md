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
  - [`acquisition:the-check-refreshes-weekly-and-retries-hourly` — The check refreshes weekly and retries hourly](#acquisitionthe-check-refreshes-weekly-and-retries-hourly--the-check-refreshes-weekly-and-retries-hourly)
  - [`acquisition:the-check-warns-and-writes-only-its-cache` — The check warns and writes only its cache](#acquisitionthe-check-warns-and-writes-only-its-cache--the-check-warns-and-writes-only-its-cache)
  - [`acquisition:the-off-switch-stops-the-check-alone` — The off switch stops the check alone](#acquisitionthe-off-switch-stops-the-check-alone--the-off-switch-stops-the-check-alone)
  - [`acquisition:sync-is-the-operators-act` — Sync is the operator's act](#acquisitionsync-is-the-operators-act--sync-is-the-operators-act)
  - [`acquisition:the-operator-caller-reports-every-outcome` — The operator caller reports every outcome](#acquisitionthe-operator-caller-reports-every-outcome--the-operator-caller-reports-every-outcome)
  - [`acquisition:the-sync-leaves-a-diff-nobody-committed` — The sync leaves a diff nobody committed](#acquisitionthe-sync-leaves-a-diff-nobody-committed--the-sync-leaves-a-diff-nobody-committed)
  - [`acquisition:clean-removes-only-a-named-leftover` — Clean removes only a named leftover](#acquisitionclean-removes-only-a-named-leftover--clean-removes-only-a-named-leftover)
  - [`acquisition:clean-swaps-the-retired-line` — Clean swaps the retired line](#acquisitionclean-swaps-the-retired-line--clean-swaps-the-retired-line)
  - [`acquisition:a-host-install-is-a-recorded-decision` — A host install is a recorded decision](#acquisitiona-host-install-is-a-recorded-decision--a-host-install-is-a-recorded-decision)
  - [`acquisition:the-setup-path-offers-the-wire` — The setup path offers the wire](#acquisitionthe-setup-path-offers-the-wire--the-setup-path-offers-the-wire)

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

`sdd self-depend status` MUST report offline, one entry per manager in the enum's order with the absent ones included, the shell-entry line and its presence, the retired sync line where `.envrc` still carries it, the version-check cache, the host probes, and any leftover. It MUST exit 0 for every state it reports, because a report is not a verdict, and it MUST offer `--json`.

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

### `acquisition:the-check-refreshes-weekly-and-retries-hourly` — The check refreshes weekly and retries hourly

`sdd self-depend check` MUST read only its cache, one file per checkout under `$XDG_CACHE_HOME/spec-driven-docs/self-depend/`, unless a refresh is due. The file records the time of the last attempt, the time of the last success, and the latest release found or the last error. After a failed attempt, a refresh MUST be due only when that attempt is one hour old. Otherwise a refresh MUST be due where the cache is missing, where it records no success, or where the last success is seven days old. Times MUST compare as times. A due refresh MUST take a lock file beside the cache first, and a check that cannot take the lock MUST read the cache alone. Under the lock, it MUST record the attempt time, read the latest release, and record the result. A warning MUST NOT change the attempt time.

#### Scenario: A failure after an overdue success

- GIVEN a cache whose last success is eight days old and whose last attempt failed one minute ago
- WHEN the shell loader runs the check line
- THEN nothing reads the network, the check warns with the recorded reason, and the next read happens one hour after that attempt

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:the-check-warns-and-writes-only-its-cache` — The check warns and writes only its cache

The shell-entry line MUST be `sdd self-depend check || true`, and the check MUST write no file except its cache and its lock. On every directory entry it MUST warn on stderr: `sdd <latest> is out, this project pins <pin>. To move: sdd self-depend sync --apply` where the cache holds a newer release, and `sdd could not check for a newer release: <reason>. Next try after <time>.` where it holds an error. Where the cache cannot be written, it MUST warn `sdd cannot record its version check at <path>: <reason>.` and read no network. It MUST exit 0 on every outcome, so the shell loads.

#### Scenario: A newer release is cached

- GIVEN a fresh cache that records a newer release than the pin
- WHEN the shell loader runs the check line
- THEN the check names both versions and the sync command on stderr, reads no network, leaves every project file byte-identical, and exits 0

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:the-off-switch-stops-the-check-alone` — The off switch stops the check alone

The check MUST read nothing and write nothing where `CI` or `SDD_SELF_DEPEND_OFF` is set. Neither variable MUST stop a sync, because only a person runs it.

#### Scenario: An operator moves the pin with the switch set

- GIVEN a shell where `SDD_SELF_DEPEND_OFF` is set
- WHEN the check runs and then the operator runs `sdd self-depend sync --apply`
- THEN the check is silent and writes no cache, and the pin moves

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:sync-is-the-operators-act` — Sync is the operator's act

`sdd self-depend sync --apply` MUST be the one act that moves the pin, and its caller MUST default to the operator. `--caller envrc` MUST fail with the usage code and name `sdd self-depend check`. Where `DIRENV_IN_ENVRC` is set, which direnv exports while it evaluates an `.envrc` and under `direnv exec`, `sync` MUST move nothing and exit 1, so a retired line cannot resume automatic moves. Its warning on stderr MUST name that variable as the cause, `nix develop -c sdd self-depend sync --apply` or the loaded shell as the fix, and the check line. The retired line `sdd self-depend sync --apply || true` still loads the shell, because `|| true` absorbs the exit 1.

#### Scenario: A landed `.envrc` still runs the retired line

- GIVEN an `.envrc` that runs `sdd self-depend sync --apply || true`
- WHEN direnv evaluates it
- THEN the pin stays, the sync exits 1, the warning names `sdd self-depend check || true`, and the shell loads because the line ends in `|| true`

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:the-operator-caller-reports-every-outcome` — The operator caller reports every outcome

`sync` MUST report every outcome and MUST fail with a distinct exit code for a target that pins nothing, a release that cannot be read, and a transaction that was put back.

#### Scenario: The target pins nothing

- GIVEN a target with no manager naming this tool
- WHEN the operator runs sync
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

`sdd self-depend clean` MUST remove from the target only a file the predecessor catalog or `--also` names, only with `--apply`, and MUST name every file it leaves byte-identical with the reason, the check line among them.

#### Scenario: The operator names a predecessor's script

- GIVEN a target carrying a script no catalog knows
- WHEN clean runs with `--also` naming it and without `--apply`
- THEN the script is listed as a leftover, the check line is listed as kept, and nothing is removed

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:clean-swaps-the-retired-line` — Clean swaps the retired line

`sdd self-depend clean` MUST preview the swap of the retired line `sdd self-depend sync --apply || true` in `.envrc`, and with `--apply` it MUST replace that line with `sdd self-depend check || true`, keep every other line byte for byte, and report the swap. Where `.envrc` already carries the check line, it MUST remove the retired line and report a removal. The clean, the kept list, and the status report MUST match a line by one rule. It MUST likewise remove the retired daily stamps, the `*.stamp` files under `$XDG_STATE_HOME/spec-driven-docs/self-depend/`, and keep every other file of the state root. A second run MUST change nothing. `sdd self-depend status` MUST report the retired line as the state `old-line`, whose next step moves the pin first and then runs the clean.

#### Scenario: A landed `.envrc` carries the retired line

- GIVEN an `.envrc` with the retired line among other lines
- WHEN clean runs with `--apply`, and then again
- THEN the first run swaps that one line and reports it, and the second run changes nothing

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:a-host-install-is-a-recorded-decision` — A host install is a recorded decision

Where a project obtains `sdd` from each host's own install rather than a manager file, its declaration MUST record `acquisition` with `source: host` and the lowest release a host may run, and `sdd self-depend status` MUST report that decision as the state `host` rather than `unwired`.

A manager file records its own pin, so only the one method that leaves no file behind needs the key. `sdd self-depend add --host --apply` writes it into a landed declaration, keeping every comment. Before a first landing no declaration exists, so the decision rides the landing: `sdd stage --acquisition host` records it in the candidate, and `sdd init --stage <root>` lands it.

#### Scenario: A project declines a pin

- GIVEN an instance whose operator chose a host install, recorded with `sdd self-depend add --host --apply`
- WHEN `sdd self-depend status --json` runs on another clone
- THEN it reports the state `host` with the recorded minimum, and no later run asks again

Verify: `cargo nextest run -E 'binary(cmd_self_depend)'`

### `acquisition:the-setup-path-offers-the-wire` — The setup path offers the wire

Where a target is wired to one manager and carries no check line, the `provision` task of the setup skill MUST raise the wire as a decision the operator answers before the task closes, stating what the line does, that it reads the network at most once every seven days, and that it never moves the pin, which stays the operator's act. Where no manager is wired, the skill MUST say nothing. An answer of no MUST leave the landing complete and be recorded in the report of the run.

On an answer of yes the setup path MUST place the check line in the target's shell loader itself, under the operator's approval, and MUST NOT report a command that cannot place it. The verb seeds that file only where the target has none, so a target that already carries one is the case the skill writes itself.

#### Scenario: A landing ends with the wire absent

- GIVEN a target whose manager file pins this tool and whose shell loader carries no check line
- WHEN the `provision` task reaches its post-flight
- THEN the operator has been asked about the wire, and the status verb reports `line-absent` until the line lands or the answer is recorded

#### Scenario: The target already carries a shell loader

- GIVEN an operator who answered yes in a target whose `.envrc` the verb refuses to edit
- WHEN the setup skill lands the wire
- THEN the skill appends the line itself under approval, because a path that reported the verb alone would leave the operator re-observing a state nothing writes

Verify: `cargo nextest run -E 'test(the_setup_skill_offers_the_freshness_wire) | test(the_setup_offer_follows_the_wiring_state) | test(the_provisioning_skill_serves_the_check_line)'`
