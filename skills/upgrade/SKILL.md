---
name: upgrade
description: Upgrade managed documentation payload while preserving adopted and local content.
---

# Upgrade

1. Inspect both manifests, load the target's affected local specs, and read every crossed migration.
2. Run `scripts/upgrade.sh` with `--dry-run` first.
3. Refuse any managed drift conflict and never overwrite adopted or user-owned content.
4. Apply a clean upgrade, then run the target's offline verifier.
5. Report changed upstream rule IDs and every affected file for reconciliation.

This skill maintains documentation canon only and emits snippets for surfaces owned by other tools.
