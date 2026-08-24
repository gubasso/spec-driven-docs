# Migration from <from> to <to>

## Applicability

Apply this guide to instances whose manifest reports `<from>`.

## Preconditions

1. Run offline verification.

   ```bash
   .spec-driven-docs/verify.sh --target /path/to/your-project --offline
   ```

## Managed changes

- <Describe projections that change.>

## Living rule-ID changes

- <Name each added, modified, or removed rule ID.>

## Local integration action

1. <State the required reconciliation.>

## Verification

1. Run the project gate and offline verifier.

   ```bash
   .spec-driven-docs/verify.sh --target /path/to/your-project --offline
   ```

## Rollback

1. Restore the project snapshot taken before the upgrade.
