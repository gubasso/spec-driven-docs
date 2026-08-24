---
name: verify
description: Verify a local spec-driven documentation instance offline.
---

# Verify

1. Inspect the target manifest and load the affected local specs.
2. Run `.spec-driven-docs/verify.sh --target <absolute-path> --offline` first.
3. Do not modify managed, adopted, or user-owned content.
4. Report managed drift, adopted reconciliation notices, missing tools, exact files, and rule IDs.

This skill maintains documentation canon only and does not own repository bootstrap surfaces.
