# Re-point the retired rule

`spec-to-code:a-permanent-exception-states-its-reason` no longer resolves. A `SATISFIES` or `VERIFIES` comment that cites it fails the citation check on the first commit after the upgrade.

1. Search the repository for the retired identifier.
2. Re-point each citation, or delete it where the claim it made is gone.
3. Turn on the reason rule your own toolchain ships. Rust has `clippy::allow_attributes_without_reason` and JavaScript has `eslint-comments/require-description`. Ruff ships no equivalent, so a Python suppression leaves its reason to review.

An `sdd: permanent` marker already written stays as inert prose. No gate reads it, and nothing fails on it. `spec-to-code:a-suppression-names-its-case` keeps its identifier, so every citation of that rule still resolves.
