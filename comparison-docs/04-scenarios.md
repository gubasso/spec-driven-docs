# 04 — Scenarios

The scenario is what a comparison document actually contains; the matrix is a view over it. This
chapter fixes the shape of a method section, the shape of an evidence section, and the rule that
keeps a scenario runnable by someone who did not write it.

## The method section

One per row. It states what to do, identically for every subject.

- A method MUST be a numbered list of commands and observations.
- A method MUST state what to record, in words, before any subject is named.
- A method MUST NOT mention a subject's expected outcome.
- A command MUST appear in a fence.

```markdown
## Survives a fork bomb

Record the state of the host shell after step 3, and whether step 4 returns.

1. Enter the environment.
2. Run `ulimit -u` and record the value.
3. Run the fork bomb `:(){ :|:& };:`.
4. From a second host terminal, run `uptime`.
```

Naming an expected outcome in the method is how a scenario stops being a test. The next author reads
the expectation, sees it matched, and never runs step 3.

## The evidence section

One per subject per qualified verdict. It states what happened, and nothing else.

- An evidence section MUST name the subject version and the date the run happened.
- An evidence section MUST state the observation before any interpretation.
- An evidence section MUST NOT restate the method.
- An evidence section SHOULD be at most six lines.

```markdown
### Fork bomb, podman

Podman 5.4.0, run on 2026-08-18. `ulimit -u` reports the host value; step 4 did not return for
90 seconds. Setting `--pids-limit` in the run invocation contains it, which is why the verdict is
partial rather than negative.
```

The length limit is the point. An evidence section that grows into an essay is an argument that has
found its way back into the table's orbit, and the reader who followed a `⚠️ partial` link wanted one
sentence.

## Reproducibility

- A scenario MUST run from a state the reader can reach: a documented install, a named version, no
  prior artifacts.
- A scenario MUST NOT depend on the author's machine, an unpublished tool, or a private path.
- A scenario that cannot be made portable MUST be marked untested rather than reported.

A verdict a reader cannot reproduce is an assertion with a symbol in front of it. If the honest
answer is that the run needs hardware the reader does not have, say so in the evidence section and
keep the verdict; if it needs the author's own setup, the row is not ready.

## One run is not a result

- A verdict that depends on timing, contention, or an intermittent failure MUST rest on repeated runs.
- An evidence section reporting a repeated run MUST say how many.

A single clean run shows the outcome is possible. Performance rows, race conditions, and anything
described as flaky need the count, and the count belongs in the evidence where a reader can weigh it.
