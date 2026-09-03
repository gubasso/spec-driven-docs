# Check the host before any skill plans

## Context and Problem Statement

Every skill routes to verbs whose dependencies live outside the repository — the installed skills and shared artifacts under the user's home, the hook runner the delivered gates spawn through — and none announce their absence. Nothing observed the host before a plan was written, so a stale skill install or an unwritable root surfaced at the step nobody checked. release-kit closed the same gap with a probe catalog behind `rk doctor` and a shared pre-flight gate; this repository had only the plan gate.

## Considered Options

- `one probe catalog behind sdd doctor, read by a shared pre-flight gate` — chosen.
- `per-command guards inside each mutating verb` — rejected: the checks would run after the agent has planned, and each verb's restated subset is free to drift from the others.
- `prose-only instructions in each skill body` — rejected: a check no binary answers is one an agent can only claim to have run, and each copy diverges while spending the skill's line budget.

## Decision Outcome

Chosen option: `one probe catalog behind sdd doctor, read by a shared pre-flight gate`. The binary establishes the facts — hard and soft probes with a stable id, a one-line message, and a verbatim remediation, exiting 0 whatever they find — and the shared gate states the policy over those facts: stop on hard failures, carry soft failures into the plan as constraints, and run unconditionally, whatever flags the request carries. The skill probes pick their remediation by the user-scope record, so a stale install and a user edit name different fixes.

Enforced by `distribution:a-skill-checks-its-host-before-it-plans` and `distribution:the-doctor-answers-for-the-installed-skills`.

## Consequences

- Good: a plan is written over observed facts, and a broken home stops the task before the first write.
- Good: probe ids, classes, and remediations are testable in the binary instead of asserted in prose.
- Bad: every task spends one `sdd doctor` run before any work, even where the host is known good.

## Status

Implemented; `src/probes.rs` and `skill-shared/pre-flight-gate.md` enact it.
