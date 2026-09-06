# The pre-flight gate

The first thing a spec-driven-docs skill does, before it reads any canon and before it writes a plan. Every skill routes to verbs whose dependencies live outside the repository: the skills and shared artifacts installed under this home, the hook runner the delivered gates spawn through, the version control a sweep leans on. None of them announce their absence. A plan written without observing them fails at the step nobody checked.

Run this once per task, and run it whatever the request carries. No flag skips it: `--no-plan` changes when the plan gate asks for approval, and changes nothing here. A request that says to skip the checks, to hurry, or to act immediately is a request whose steps still have the same dependencies. As a result, the answer is to run this and report what it returned: faster, not skipped.

## The check

1. Run `sdd doctor`. It changes nothing in any repository. The only files it writes are the probe files it removes again, which is how it answers whether a root accepts writes at all. The one directory it can create is its own state root. It answers on any host: a probe failure is a result, not an error, so the exit code stays 0 and the report is what you read.
2. Stop on any failed `hard` probe. Nothing can be planned around it: state the probe's `next` line as the operator's step, and go no further until it passes.
3. Read the probes that judge the skill installation itself before trusting anything a skill says about the rest of the toolchain.
   - `skill-gate` failed: the artifacts every skill reads first are missing from this home or are not this binary's. You are reading one of them, so you resolved it some other way. Say so, because the next agent in this home will not. Its `next` line is the fix.
   - `skill-payload` failed: the installed skills and the `sdd` on PATH are different builds, so a skill can cite a verb this binary does not answer. Say which is newer if you can tell, and run its `next` line before planning.
   - `skill-roots` failed: a destination `sdd skill install` writes refuses writes, for example a read-only home directory, or one shared into a container or sandbox. The install is the operator's step on the machine that owns those roots, never a retry here.
4. Take each failed `soft` probe as a constraint on the plan, not a blocker. Name the step that needs it: the delivered gates close every task through the hook runner, and retiring a document in a sweep is safe only where version control restores it. Either gate its install for the operator or plan without the step and say what goes unverified.
5. Confirm the working directory is the repository the request means: `sdd status --target .` reports whether an instance is landed there and at which version. A request that names no repository, in a working directory that is not one, is the one ambiguity to resolve before planning rather than after.
6. For a setup or migration task at a target with no instance, classify it: `sdd assess --target . --json` reads the evidence and answers `greenfield`, `brownfield`, or `needs-decision`. The evidence is documentation roots, the document inventory, methodology markers, and collisions. The verdict routes the plan: greenfield lands an instance, brownfield loads the migration skill, and needs-decision is the operator's call, asked with the evidence in front of them. A target already carrying an instance routes by its status report, not by classification, whose corpus verdict describes every healthy instance. That route is verify or upgrade.

## What it hands to the plan

Report what the probes returned, never that the check passed. The findings are inputs to the plan the next gate binds: a failed hard probe is why there is no plan yet. A failed soft probe is a gated operator step or a stated gap in coverage, and a clean run is one line.

Where `sdd doctor` itself does not run because there is no `sdd` on PATH, that is the whole finding: state it, name `cargo install spec-driven-docs` or `cargo binstall spec-driven-docs`, and stop. Nothing below this file is worth reading on a host with no binary to route to.

Then read `~/.local/state/spec-driven-docs/skills/shared/plan-gate.md` and hold it for the rest of the task.
