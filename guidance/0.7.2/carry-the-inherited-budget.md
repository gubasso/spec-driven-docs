# Carry the inherited budget

The four budget gates now judge a recorded ceiling instead of the cap where the project records one. Nothing fails on an instance that records nothing, so this step is optional and worth taking once.

1. Run `sdd debt baseline` and read every current violation.
2. Run `sdd debt baseline --apply` to record them. A recorded ceiling only comes down.
3. An instance still carrying the older flat list takes `sdd debt migrate` instead, which preserves every exemption and broadens nothing.

`SPEC-budget-debt.md` seeds on this upgrade. Where the project already holds a file at that destination, the upgrade keeps it and says so, and `sdd policy reconcile` offers the rule that authorizes the declaration.
