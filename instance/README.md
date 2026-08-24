# Instance distribution

An instance keeps its current specs, gates, configurations, templates, verifier, and manifest in its
own checkout. Canon provenance is consulted for installation and upgrades, never for routine work.

Ownership has three classes:

- Managed files are byte-for-byte projections. Verification fails on drift; upgrades replace only an
  unchanged installed projection.
- Adopted files are seeded once and owned locally. The manifest records both the installed bytes and
  the upstream baseline they came from, so an edit reports drift until it is reconciled and upgrades
  preserve it.
- Integration blocks are delimited regions inside local files. Tools may replace only the marked block.

Required tools are POSIX shell, `awk`, `cat`, `cut`, `dirname`, `find`, `grep`, `head`, `jq`, `sed`,
`mktemp`, `rm`, `sha256sum`, `sort`, `tr`, `uniq` and `wc`, with a `grep` that accepts `-I` and `--exclude-dir` (GNU and BSD do; BusyBox
does not). Each gate is
run from the directory it was installed into; invoking one through a symlink placed elsewhere is
not supported, because a gate resolves its shared library beside itself. Offline verification
checks each one and names anything missing. A remote pre-commit consumer needs the same ambient tools.
The strict profile vendors the payload and avoids runtime network access. Toolchain versions belong to
the instance; current MD043 arrays support markdownlint-cli2 0.22 and later.

## Instantiate

Run a dry run first. A non-empty target with no instance defaults to dry-run, and `--apply` is what
writes into it.

```bash
scripts/instantiate.sh --target /path/to/your-project --profile codebase --dry-run
scripts/instantiate.sh --target /path/to/your-project --profile codebase --apply
```

Use `knowledge-base` to select `_docs/`; `codebase` selects `docs/`. Review the managed pre-commit
block before applying it to a heavily commented configuration. The block is spliced into the
top-level `repos:` sequence, so a configuration without that key is refused rather than rewritten.

Lightweight consumers may use the public remote hooks with an immutable release tag:

```yaml
repos:
  - repo: https://github.com/gubasso/spec-driven-docs
    rev: v0.1.0
    hooks:
      - id: spec-rule-id-unique
```

## Verify

Run the instance-owned verifier:

```bash
.spec-driven-docs/verify.sh --target /path/to/your-project --offline
```

Use `--check-upstream /path/to/canon-checkout` only when comparing versions explicitly.

## Upgrade

Review every applicable guide under `migrations/`, then dry-run the upgrade:

```bash
scripts/upgrade.sh --target /path/to/your-project --from /path/to/canon-checkout --dry-run
scripts/upgrade.sh --target /path/to/your-project --from /path/to/canon-checkout
```

A locally edited managed file aborts the entire operation, and every conflict is listed in one run.
Reconcile them explicitly; adopted specs and content outside the markers are never overwritten, and
the upgrade never touches `.git`.
