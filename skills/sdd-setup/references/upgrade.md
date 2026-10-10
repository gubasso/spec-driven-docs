# Task: upgrade

Move an instance older than the `sdd` on PATH to that version. The gate picks it for the `upgrade` state. Read `sdd docs reconcile` for the reconciliations an upgrade can raise.

## Pre-flight

1. Run `sdd upgrade --target . --dry-run --json`, and hold `retired_citations`: every citation of a rule ID this binary no longer defines, with its path and line.
2. Hold `legacy_state` from `sdd status --target . --json`: what an older release left under the state root.

## Execution

Fill the landing pipeline with:

- Stage: `sdd stage --target . --json`. It reads the recorded profile, docs scratch, writing style, and reserved paths, and it refuses a different `--profile`, because a profile change is a migration.
- Compare: route every row by the pipeline's own table. Each keep-or-discard answer names the path in `--replace`.
- Prepare: re-point each retired rule-ID citation to the replacement the changelog names. The operator approves each one, because the tool cannot know which rule the author meant.
- Land: `sdd upgrade --stage <root> --dry-run` to preview, then `sdd upgrade --stage <root>` as the gated step, with one `--replace <path>` per answered conflict. An unnamed conflict still refuses.

## Post-flight

1. The Prove step passed, and `sdd status --target . --json` reads `classification` `current`.
2. Route each saved copy by the rule at the end of the landing pipeline.
3. Offer the removal of each `legacy_state` entry whose `active` is false, by its exact path, as a gated step.
