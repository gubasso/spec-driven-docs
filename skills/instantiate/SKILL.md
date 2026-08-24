---
name: instantiate
description: Install a spec-driven documentation instance without overwriting project-owned content.
---

# Instantiate

1. Inspect the target and load the local documentation specs that govern it.
2. Run `scripts/instantiate.sh` with `--dry-run` and report every proposed path.
3. Refuse to overwrite user-owned content or edit outside the managed markers.
4. Run the requested installation with `--apply` only after the preview is clean.
5. Run the installed verifier with `--offline` and report exact files and rule IDs.

This skill maintains documentation canon only. Repository, Nix, CI, security, task-runner, and planning
surfaces remain owned by their existing tools; emit integration snippets for those owners.
