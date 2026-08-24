# 11 — Operational Documents

Operational material is written under pressure: a fix becomes a runbook, a failure becomes a case
study, a command someone ran at 3 a.m. becomes a lookup. It lands outside the model unless something
states what each one contains. [01 — Placement](./01-placement.md) puts each in a zone and
[10 — Procedures](./10-procedures.md) owns the steps inside one; this chapter owns the contents.

## Four shapes

| Document          | Contains                                                                 | Zone      |
| ----------------- | ------------------------------------------------------------------------ | --------- |
| Runbook           | trigger, preconditions, ordered actions, verification, rollback criteria | guides    |
| Diagnostic        | symptom, signal, expected result, interpretation, the guide that uses it | reference |
| Case study        | what happened, the signals present, what fixed it, the durable lesson    | reference |
| Setup walkthrough | start state, ordered actions, end state, verification                    | guides    |

- An operational document MUST carry every part of its shape.

The parts are what make the document usable by someone who did not write it. A runbook missing its
trigger is a page nobody opens at the right moment; one missing its rollback criteria is a page that
gets a reader halfway and leaves them there.

## The runbook

The trigger opens it: the alert, the symptom, or the condition that makes this the right page. A
reader arrives mid-incident and needs to confirm in one line that they are in the right place.

Rollback is two things and the second is the one usually missing. The commands that revert are the
easy half; the criteria that say when to stop trying and revert are the half that decides whether an
incident ends in twenty minutes or two hours.

- A runbook MUST state the condition under which the reader stops and reverts.

Background moves out. A runbook that requires understanding the architecture before acting is not a
runbook: the explanation belongs to whatever owns it, and the runbook links it once.

## A destructive action shows its dry run first

- Where a tool offers a dry-run or inspection form, a destructive step MUST show that form as its own
  step first.
- A step whose effect cannot be undone MUST state what the reader is about to lose, before the
  command.

````markdown
3. Confirm the snapshots the policy would drop.

   ```bash
   restic forget --keep-last 3 --dry-run
   ```

   A correct result lists snapshots and removes nothing.

4. Drop them. This deletes snapshot data and cannot be undone.

   ```bash
   restic forget --keep-last 3 --prune
   ```
````

The confirmation is a step rather than a warning because a warning is prose a reader under pressure
skims. A step is something they perform, and its output is what they check against.

## The diagnostic

A diagnostic decides which bucket a failure is in. It does not solve every bucket inline; it hands
off to the guide that does.

```markdown
| Symptom                | Signal                 | Expected      | Means                      |
| ---------------------- | ---------------------- | ------------- | -------------------------- |
| Backup exits 1, silent | `restic cat config`    | the JSON body | non-zero: repo unreachable |
| Backup hangs at 0 B    | `ss -tnp \| rg restic` | one ESTAB     | none: name resolution      |
```

- A diagnostic MUST state the expected result of every signal it names.

A signal with no expected result is a command, not a diagnostic. The reader can run it either way;
what they cannot do is tell whether what came back was the problem.

## The case study

A case study records what happened, the signals that were present, what fixed it, and the lesson
that outlives the incident. It is reference, because its reader arrives looking something up rather
than executing.

It is not where a new rule lives. When an incident changes policy, the rule goes to the owning spec
and a decision record argues for it; the case study links the rule ID and stops. A rule stated only
in a case study is a rule that binds nobody, in a document nobody loads.

## Placement is zone-first

Operational documents go in the zone their reader-need selects, under the topic they concern. They do
not get a directory named after their type.

```text
docs/guides/backups/restore-from-object-storage.md
docs/reference/backups/diagnosing-a-stalled-backup.md
```

A `runbooks/` or `case-studies/` directory sorts pages by a category the author invented rather than
by what the reader is trying to do, and it is the filesystem-index shape
[00 — Model](./00-model.md) forbids: a container that exists to hold a label. When a topic's guides
become hard to scan, the fix is the zone's index, not a new bucket.

## Anti-patterns

- Runbook as architecture tour: the background a reader must absorb before the first command.
- Rollback commands with no rollback criteria: how to revert, with no statement of when to.
- Bare destructive step: the irreversible command with no inspection step and no statement of loss.
- Signal with no expectation: a command whose output the reader cannot judge.
- Case study as policy: the new rule stated only in the incident write-up.
- Type directory: `runbooks/` beside `guides/`, sorting by label rather than by reader need.

## Sources

- Diátaxis, on each mode having its own purpose and being kept distinct from the others:
  <https://diataxis.fr/>
- Nobl9, on the runbook template carrying triggers, rollback criteria, and verification:
  <https://www.nobl9.com/it-incident-management/runbook-example>
- Google Cloud, on an operational playbook existing before it is complete:
  <https://cloud.google.com/blog/products/devops-sre/how-to-start-and-assess-your-sre-journey>
