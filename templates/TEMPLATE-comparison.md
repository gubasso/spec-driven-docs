# `<subject>` and the alternatives

Every verdict below is a scenario that was run, not a feature claim. A capability name links to the
scenario; a verdict links to what that tool did when the scenario was run against it.

Legend: ✅ yes, works on the default path · ⚠️ partial, needs configuration or has a stated limit ·
❌ no, was run and failed · ➖ n/a, meaningless for this subject · 🧪 unstable, experimental path ·
❓ untested, not yet run.

## `<theme the reader arrives with>`

`<One sentence introducing what this table settles.>`

Verified: `<YYYY-MM-DD>` — `<subject>` `<version>`, `<alt-1>` `<version>`, `<alt-2>` `<version>`,
`<alt-3>` `<version>`.

| capability                             | `<subject>`                      | `<alt-1>` | `<alt-2>` | `<alt-3>`   |
| -------------------------------------- | -------------------------------- | --------- | --------- | ----------- |
| [`<observable behavior>`](#scenario-a) | [⚠️ partial](#scenario-a-subject) | ✅ yes    | ❌ no     | ❓ untested |
| [`<observable behavior>`](#scenario-b) | ✅ yes                           | ❌ no     | ❌ no     | ➖ n/a      |

## `<theme>`

`<Repeat the block above. A subject appears only in the themes it competes in.>`

## Scenario A

`<What to record, in words, before any subject is named.>`

1. `<Reach the starting state.>`
2. `<Run the command.>`

```bash
<the command>
```

1. `<Observe and record.>`

### Scenario A, `<subject>`

`<subject>` `<version>`, run on `<YYYY-MM-DD>`. `<What was observed.>` `<The one qualification that
makes the verdict partial rather than positive or negative.>`

### Scenario A, `<alt-1>`

`<alt-1>` `<version>`, run on `<YYYY-MM-DD>`. `<What was observed.>`

## Scenario B

`<As above. One method section per row whose label is a link.>`

## Re-verification

`<Cadence, and where it is tracked. Re-running is what a refresh is; editing version numbers without
re-running produces a false date.>`
