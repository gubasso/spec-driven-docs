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

`sdd init` lands the embedded agent skills managed at `.claude/skills/`, which Claude Code reads, and `.agents/skills/`, which Codex, Gemini CLI, and Copilot read, so every agent working in the instance carries the same operating knowledge.

```bash
sdd skill list
sdd skill show sdd-docs
sdd skill install
sdd skill install --apply
```

`sdd skill install` writes the same files at user scope — `~/.claude/skills` and `~/.agents/skills` — previewing by default and refusing a destination whose bytes differ from the payload unless `--force` is given. User-scope files are never recorded in an instance manifest; the embedded payload is their reference.

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
