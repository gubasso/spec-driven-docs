# Instance distribution

An instance keeps its current specs, configurations, templates, and manifest in its own checkout, and runs the gates and the verifier from the installed `sdd` binary, which carries the whole payload. Canon provenance is consulted for installation and upgrades, never for routine work.

Ownership has three classes:

- Managed files are byte-for-byte projections. Verification fails on drift; upgrades replace only an unchanged installed projection.
- Adopted files are seeded once and owned locally. The manifest records both the installed bytes and the upstream baseline they came from, so an edit reports drift until it is reconciled and upgrades preserve it.
- Integration blocks are delimited regions inside local files. Tools may replace only the marked block.

The only requirements are `git`, `pre-commit`, and `sdd` on the PATH of everyone who runs the hooks, including CI. Install `sdd` with `cargo install spec-driven-docs`, with `cargo binstall spec-driven-docs`, or from a GitHub release's installers. The binary embeds the payload for its own version, so verification and upgrades run offline; `sdd verify` compares its version to the instance's and says which side to move.

## Instantiate

Run without flags first. A non-empty target with no instance defaults to a dry run, and `--apply` is what writes into it.

```bash
sdd init --target /path/to/your-project --profile codebase
sdd init --target /path/to/your-project --profile codebase --apply
```

Use `knowledge-base` to select `_docs/`; `codebase` selects `docs/`. Review the managed pre-commit block before applying it to a heavily commented configuration. The block is spliced into the top-level `repos:` sequence, so a configuration without that key is refused rather than rewritten.

Lightweight consumers may use the public remote hooks with an immutable release tag; pre-commit builds the binary from this repository with cargo:

```yaml
repos:
  - repo: https://github.com/gubasso/spec-driven-docs
    rev: v0.2.0
    hooks:
      - id: spec-rule-id-unique
```

## Skills

The binary embeds the agent skills, and `sdd skill install` is the only thing that lands them. They install at user scope alone — an instance carries none — because an agent resolves a skill by its name, so an instance copy beside the user-scope one would offer the same skill twice under one name.

```bash
sdd skill list
sdd skill show sdd-setup
sdd skill install
sdd skill install --apply
```

`sdd skill install` writes into `~/.claude/skills`, which Claude Code reads, and `~/.agents/skills`, which Codex, Gemini CLI, and Copilot read, previewing by default and refusing a destination whose bytes it cannot account for unless `--force` is given. It accounts for two things: the payload it carries, and `~/.local/state/spec-driven-docs/skills.json`, which records the digest each successful apply wrote. A copy an older release left is replaced without asking; a file you edited refuses. An apply that fails partway restores every destination, so the two roots never end up on different versions of one skill. User-scope files are never recorded in an instance manifest, and no verification reads that state file.

Both verbs also sweep: a destination the record vouches for that the current payload no longer carries is removed along with the directory it empties, so a skill the canon renamed leaves nothing for an agent to keep offering. `sdd skill uninstall` reverses the install, also previewing by default; it removes each skill's `SKILL.md` and its directory when empty, so any file you added alongside survives, as does any leftover you edited yourself.

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

A locally edited managed file aborts the entire operation, and every conflict is listed in one run. Reconcile them explicitly; adopted specs and content outside the markers are never overwritten, and the upgrade never touches `.git`.
