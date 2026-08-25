# 05 — Freshness

A comparison document is the most perishable page a project publishes: every cell is a claim about software that ships without asking. This chapter fixes what each table must date, how a stale verdict is retired, and what the untested state is for.

## Every table is dated

- A table MUST be preceded by a `Verified:` line naming the date and the version of every subject in it.
- The date MUST be the date the runs happened, not the date the file was edited.

```markdown
Verified: 2026-08-18 — subject 0.4.1, podman 5.4.0, Docker 27.5.1, `nix develop` on nix 2.26.
```

Dating the table rather than the document is what makes a partial refresh honest. Re-running one theme updates one line, and the tables nobody re-ran keep saying so.

## Untested is the safe state

- A verdict whose subject version has changed MUST become untested until it is re-run.
- A verdict MUST NOT be carried forward across a subject's major version.

A stale positive is worse than a gap. A reader who sees `❓ untested` knows exactly what they have and can go run it; a reader who sees `✅ yes` from two major versions ago has been given a wrong answer with no way to notice. Demoting is cheap and reversible, and it moves the cost onto the author who has the context rather than the reader who has none.

## Perishability is tracked, not remembered

- A document MUST carry a re-verification cadence.
- The cadence MUST live wherever the project tracks perishable facts, not only in prose.

Pick the cadence from what moves. A row about a subject on a six-week release train needs a look each release; a row about a filesystem boundary that has not changed in a decade does not. One cadence for the whole document sets it wrong for most rows.

## Retiring a subject

- A subject that is no longer maintained MUST be deleted from the tables, not marked dead.
- Removing a subject MUST NOT leave a note explaining the absence.

A dead column costs width in every table and teaches nothing a reader can act on. The removal is in the log. If the subject mattered enough that its disappearance is itself the lesson, that is a decision record, not a table row.

## What a refresh actually is

1. Reinstall each subject at its current version.
2. Re-run every method in the theme, unchanged.
3. Update the evidence sections with the new observations and the new date.
4. Update the `Verified:` line.
5. Demote to untested anything step 1 could not install.

Step 2 is the one that gets skipped. A refresh that edits version numbers without re-running is a false date, and it is worse than no refresh because it resets the reader's suspicion.
