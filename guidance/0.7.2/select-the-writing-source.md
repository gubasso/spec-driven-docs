# Select the writing source

`writing_style` in the project's declaration selects `builtin`, a project document, or none. An absent key reads as `builtin`, so an upgraded instance keeps the route it already had and nothing fails.

1. Decide which source the project's authors follow.
2. Set `writing_style` in the declaration.
3. Run `sdd hooks --apply`, which rewrites the documentation block in the root agent digest as well as the pre-commit block.

`SPEC-writing-policy.md` seeds on this upgrade, and `sdd policy reconcile` offers the rule that authorizes a selection other than `builtin`.
