# Instance distribution

An instance keeps its current specs, configurations, templates, and manifest in its own checkout. It runs the gates and the verifier from the installed `sdd` binary, which carries the whole payload. Canon provenance is consulted for installation and upgrades, never for routine work.

Ownership has three classes:

- Managed files are byte-for-byte projections. Verification fails on drift. Upgrades replace only an unchanged installed projection.
- Adopted files are seeded once and owned locally. The manifest records both the installed bytes and the upstream baseline they came from. As a result, an edit reports drift until it is reconciled, and upgrades preserve it.
- Integration blocks are delimited regions inside local files. Tools can replace only the marked block.

The only requirements are `git`, `pre-commit`, and `sdd` on the PATH of everyone who runs the hooks, including CI. Install `sdd` with `cargo install spec-driven-docs`, with `cargo binstall spec-driven-docs`, or from a GitHub release's installers. The binary embeds the payload for its own version, so verification and upgrades run offline. `sdd verify` compares its version to the instance's and says which side to move.

## Instantiate

Run without flags first. A non-empty target with no instance defaults to a dry run, and `--apply` is what writes into it.

```bash
sdd init --target /path/to/your-project --profile codebase
sdd init --target /path/to/your-project --profile codebase --apply
```

Use `knowledge-base` to select `_docs/`. `codebase` selects `docs/`. Review the managed pre-commit block before applying it to a heavily commented configuration. The block is spliced into the top-level `repos:` sequence, so a configuration without that key is refused rather than rewritten.

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

Every apply runs as one transaction. It holds a lock for its whole run, so a second install refuses at once naming the holder rather than interleaving. It stages each write beside its destination, records what it is about to replace, and rolls back a run the process did not finish before it plans new work. It refuses a destination reached through a symlink, whatever `--force` says. An apply that cannot write the receipt fails and puts every file back, because a landing the tool cannot vouch for is a landing it would later refuse to take back.

Both verbs sweep: a destination the receipt vouches for that the current payload no longer carries is removed along with the directory it empties. A home installed before the packages therefore loses the two files under the retired shared root on its first apply. `sdd skill uninstall` reverses the install, also previewing by default. It removes a file only where its digest is the one the receipt records, and names every file it keeps with the reason. As a result, any file you added alongside survives, as does any file you edited yourself.

User-scope files are never recorded in an instance manifest, and no verification reads the receipt.

## Read a release

```bash
sdd payload
sdd payload --release 0.8.0
sdd payload --release latest --json
```

`sdd payload` reports what a release carries: its version, the protocol version between it and this binary, where its facts came from, a digest over its content, and how many artifacts it holds. With no flag it reads the release the binary carries and touches no network. With `--release` it resolves at crates.io, verifies the archive against the registry checksum, and caches it under `$XDG_CACHE_HOME/spec-driven-docs`. `--offline` accepts an exact version already cached and refuses `latest`, because only the index says which release is newest. A release older than the tool can describe reports unavailable with its evidence rather than a guess.

## Report

```bash
sdd status --target /path/to/your-project --json
```

The object declares the schema `sdd.status/2`, and its `paths` section is where every path the tool names comes from. `paths.user` carries the roots under the invoking user's home, each with the variable that moved it. `paths.active` is the landed instance, or `null` where there is none. `paths.candidates` carries one destination set per profile, and `paths.proposals` carries what the target offers for the two locations the project owns. Every entry states its source, so a recorded answer and a derived one never read alike. Read a path from here rather than writing it down: a skill, a runbook, or a chapter that spells one is a copy that drifts at the first rename.

## Verify

```bash
sdd verify --target /path/to/your-project
```

Verification is always offline: hashes, the managed block, local rule IDs, and the version alignment between the binary and the instance.

## Upgrade

Install the newer `sdd`, review its changelog, then dry-run the upgrade:

```bash
sdd upgrade --target /path/to/your-project --dry-run
sdd upgrade --target /path/to/your-project
```

A locally edited managed file aborts the entire operation, and every conflict is listed in one run. Reconcile them explicitly. Adopted specs and content outside the markers are never overwritten, and the upgrade never touches `.git`.
