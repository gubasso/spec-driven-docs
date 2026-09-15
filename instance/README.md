# Instance distribution

An instance keeps its current specs, configurations, templates, and manifest in its own checkout. It runs the gates and the verifier from the installed `sdd` binary, which carries the whole payload. Canon provenance is consulted for installation and upgrades, never for routine work.

Ownership has three classes:

- Managed files are byte-for-byte projections. Verification fails on drift. Upgrades replace only an unchanged installed projection.
- Adopted files are seeded once and owned locally. The manifest records both the installed bytes and the upstream baseline they came from. As a result, an edit reports drift until it is reconciled, and upgrades preserve it.
- Integration blocks are delimited regions inside local files. Tools can replace only the marked block.

The only requirements are `git`, `pre-commit`, and `sdd` on the PATH of everyone who runs the hooks, including CI. Install `sdd` with `cargo install spec-driven-docs`, with `cargo binstall spec-driven-docs`, or from a GitHub release's installers. The binary embeds the payload for its own version, so verification and upgrades run offline. `sdd verify` compares its version to the instance's and says which side to move.

## Stage the candidate

What lands is the version of `sdd` that is installed, so the operator chooses it with their own package manager. Before writing anything, render that candidate where you can read it.

```bash
sdd stage --target /path/to/your-project
sdd stage --target /path/to/your-project --json
```

The stage writes nothing into the target. It holds every destination the candidate would land under `artifacts/`, this version's own method, specs, templates and skills under `reference/`, and a receipt naming what it rendered. It survives the landing it informed, and only `sdd stage clean <path>` removes it.

`sdd init` lands a first instance and refuses a target that already documents itself under another convention. `sdd upgrade` moves a landed instance and refuses a target with no instance. Both name what serves the target instead, and both render the candidate again from the binary's own sources rather than reading a staged byte. [method/landing.md](../method/landing.md) owns the model behind all of them.

```bash
sdd init --target /path/to/your-project --profile codebase
sdd init --target /path/to/your-project --profile codebase --apply
```

Use `knowledge-base` to select `_docs/`. `codebase` selects `docs/`. A non-empty target with no instance defaults to a dry run, and `--apply` is what writes into it. Review the managed pre-commit block before applying it to a heavily commented configuration. The block is spliced into the top-level `repos:` sequence, so a configuration without that key is refused rather than rewritten.

## What refuses before the first write

A landing settles what it can refuse on before a byte moves. Every destination must resolve beneath the opened target, checked without following a link. Every whole file this tool would own must be one the record accounts for: a destination holding bytes nothing vouches for refuses the run and names it, because that file is the one somebody else wrote. One writer holds the target for the whole run.

A locally edited managed file is a conflict rather than a question. The stage carries the candidate's bytes to compare against, and the file is resolved before the landing runs. Adopted specs and content outside the markers are never overwritten.

The gates reach a repository by installation, not by remote reference. This repository publishes no `.pre-commit-hooks.yaml`, so it cannot be named as a `repo:` in someone else's configuration: most gates read the layout an instance has. If a repository that never adopted the framework wired those gates, the checks fail on its first commit. Run `sdd init` to become an instance, and the block arrives wired to the layout the profile selected.

## Skills

The binary embeds the agent skills, and `sdd skill install` is the only thing that lands them. They install at user scope alone, and an instance carries none, because an agent resolves a skill by its name. As a result, an instance copy beside the user-scope one offers the same skill twice under one name.

```bash
sdd skill list
sdd skill show sdd-setup
sdd skill install
sdd skill install --apply
```

A skill lands as a package: one directory holding `SKILL.md` and every shared artifact under `references/`. Each skill names its gates by a path relative to its own directory, which is what the Agent Skills format resolves a supporting file against.

`sdd skill install` writes into `~/.claude/skills`, which Claude Code reads, and `~/.agents/skills`, which Codex, OpenCode, and Pi each document that they read. `CLAUDE_CONFIG_DIR` relocates the first root and leaves the second where it is. It previews by default and refuses a destination whose bytes it cannot account for unless `--force` is given. It accounts for two things: the payload it carries, and the receipt under `$XDG_STATE_HOME/spec-driven-docs`, which records the digest of every file each successful apply wrote. A copy an older release left is replaced without asking. A file you edited refuses.

Every apply holds the user-scope lock for its whole run, so a second install refuses at once naming the holder rather than interleaving. It stages each write beside its destination, replaces one file at a time, and writes the receipt last. It refuses a destination reached through a symlink, whatever `--force` says. A run that stops partway names the destinations it finished, leaves the previous receipt standing, and finishes the rest when it runs again.

Both verbs sweep: a destination the receipt vouches for that the current payload no longer carries is removed along with the directory it empties. A home installed before the packages therefore loses the two files under the retired shared root on its first apply. `sdd skill uninstall` reverses the install, also previewing by default. It removes a file only where its digest is the one the receipt records, and names every file it keeps with the reason. As a result, any file you added alongside survives, as does any file you edited yourself.

User-scope files are never recorded in an instance manifest, and no verification reads the receipt.

## Keep the pin fresh

```bash
sdd self-depend status --target /path/to/your-project --json
sdd self-depend add --target /path/to/your-project --manager mise --venue crates
sdd self-depend sync --target /path/to/your-project --caller operator --apply
sdd self-depend clean --target /path/to/your-project --also scripts/bump.sh
```

Where the target pins `sdd` through a manager, a landing does not close until the operator has answered about the freshness wire. After the variables land, run `sdd self-depend status --target . --json` and hold `wired` and `envrc_sync`. Where `wired` is `null`, there is no pin to keep fresh. Where `wired` names a manager and `envrc_sync` is `false`, decide whether the project lands the line `sdd self-depend sync --apply || true` in its `.envrc`: on each directory entry it moves the pin to the latest release through that manager, it attempts at most one bump a day, and it leaves a diff for the operator to review and commit. On yes, `sdd self-depend add --target . --manager <wired>` prints the line with its placement and seeds `.envrc` only where the target has none; re-run the status verb until `envrc_sync` reads `true`. On no, the landing is complete without the wire, and the answer goes into the task's close.

A project pins this tool through the manager it already runs, and `sdd self-depend` keeps that pin fresh. `status` reports offline, one entry per manager with the absent ones included, plus the shell-entry line, the stamp, the host probes, and any leftover; every state exits 0. `add` prints the fragments one manager and venue pair needs with their anchors and placements, edits no file the project owns, and seeds a manager file only where the project has none and only with `--apply`. `sync` moves the pin to the latest release through the wired manager in one transaction: a flake moves its tag and its lock together or not at all. The `--caller envrc` default, the line `sdd self-depend sync --apply || true` in `.envrc`, attempts at most once a day per checkout, stays silent, exits 0 on every outcome, and leaves a diff for review; `--caller operator` reports every outcome and fails loudly. `CI` or `SDD_SELF_DEPEND_OFF` switches the loop off. `clean` removes only what a predecessor mechanism left and names what it keeps.

## Read the corpus

```bash
sdd docs
sdd docs context budget
```

`sdd docs` prints one described index of everything the binary carries: the method chapters, the specifications, the templates, and the operator tasks whose answer is a help page. `sdd docs <topic>` resolves one entry by its id, an alias, or a unique prefix, and renders it. An ambiguous query names every candidate rather than guessing one. The managed documentation block a landing writes names this verb, so an agent that has never opened the canon reaches any chapter in two commands.

## Report

```bash
sdd status --target /path/to/your-project --json
```

The object declares the schema `sdd.status/3`, and its `paths` section is where every path the tool names comes from. `paths.user` carries the roots under the invoking user's home, each with the variable that moved it. `paths.active` is the landed instance, or `null` where there is none. `paths.candidates` carries one destination set per profile, and `paths.proposals` carries what the target offers for the docs scratch. Every entry states its source, so a recorded answer and a derived one never read alike. Read a path from here rather than writing it down: a skill, a runbook, or a chapter that spells one is a copy that drifts at the first rename.

## Land

```bash
sdd init --target /path/to/your-project --profile codebase --apply
sdd upgrade --target /path/to/your-project --dry-run
sdd upgrade --target /path/to/your-project
```

The landing replaces each destination in its own directory and writes the record last, so until the record lands the previous one still describes the target. It never touches `.git`. There is no rollback and no stored result: a run that stops names the destinations that hold candidate bytes, and running it again finishes the rest.

Two things prove it finished: the record matches the tree, and verification passes. Compare the result against the stage before removing it.

```bash
sdd verify --target /path/to/your-project
```

Verification is always offline: hashes, the managed block, local rule IDs, and the version alignment between the binary and the instance.
