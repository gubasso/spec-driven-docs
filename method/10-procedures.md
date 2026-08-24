# 10 — Procedures

A guide is the one zone whose reader is executing rather than reading. This chapter owns that
document's body: how large a step is, what closes the page, and what a multi-phase procedure names at
each boundary so every value it carries has a producer.

## The step

A step is one action the reader performs and can confirm.

- A step MUST be one action, written in the imperative.
- A step MUST NOT carry more than one sentence after its command.
- A result the command does not show MUST be a verification step, not a sentence.
- A single-step procedure MUST be a bullet rather than a numbered list.
- An optional step MUST open with `Optional:`.

The permitted sentence follows the command instead of preceding it, so a reader who does not need it
has already moved on. [06 — Format](./06-format.md) fixes what that sentence may carry and keeps the
command out of it.

````markdown
2. Initialize the repository.

   ```bash
   restic init --repo "s3:<BUCKET_ENDPOINT>/backups"
   ```

   Choose the passphrase before running this: it cannot be changed without re-encrypting every
   snapshot.

3. Verify the repository answers.

   ```bash
   restic snapshots --repo "s3:<BUCKET_ENDPOINT>/backups"
   ```

   A correct result is an empty snapshot table, not an error.
````

Step 2 keeps a hazard, which is one of the three things prose buys. Step 3 is what a sentence
describing an outcome becomes once the outcome has a command.

The test for a sentence in a guide: delete it and ask whether the reader can still perform the step.
If they can, it was narration. A step needing a paragraph is two steps, or one step plus a link to
the page that owns the concept.

One command per fence where a reader runs them one at a time and checks between. Group commands only
where they are pasted together.

## Preconditions and verification

A guide states what must be true before step one and how the reader knows they are done. Both are
part of the procedure.

- A guide MUST open with its preconditions and close with a verification step.
- A precondition that can be checked by a command MUST carry that command.
- A verification step MUST state what a correct result looks like.

Preconditions are tools installed, access held, and state assumed, each one checkable. The closing
step is where an outcome no command printed becomes visible: a background job, a remote state change,
a silent success. A guide whose verification is that it should work now has not been finished.

## The artifact token

An artifact is anything a phase produces that something later needs: a string, a passphrase, a file, a
name the project has to agree on, a record written to paper. Name each one once, as an upper-snake
token in angle brackets.

```text
<BACKUP_PASSPHRASE>
```

- An artifact token MUST match `[A-Z][A-Z0-9_]*` inside angle brackets.
- An artifact token MUST name the artifact rather than the step that produced it.
- One artifact token MUST mean one artifact across the whole docs tree.
- An artifact token MUST NOT carry a real or realistic value.

`<BUCKET_KEY_ID>`, not `<STEP_3_OUTPUT>`: a step number is wrong the moment a step is inserted. A slot
shown as `key_id: 002abc4e91` reads as a value to copy rather than one to replace, and a reader
working through a recovery at speed will copy it.

Case is the discriminator against the placeholder [06 — Format](./06-format.md) already owns. A
placeholder is lowercase and stands in for anything project-specific, as `<project>` or `<host>` do.
An artifact token is upper-snake and asserts more: that some phase produces this, and that the guide
says which. A project already spending upper-snake angle names on something else picks a different
delimiter for one of the two, because the two meanings cannot share a spelling.

In prose the token is inline code. Inside a fenced block it is written bare.

## The inputs line

One plain line directly under the phase heading, before the first step.

```text
Inputs: `<BACKUP_PASSPHRASE>` (§4), `<BUCKET_KEY_ID>` (§3).
```

- A phase MUST open with an inputs line naming each token's producer.
- A phase that consumes nothing MUST write `Inputs: none.`

A producer is a section reference on the same page, or an explicit relative link where it is another
guide. The line is the reader's re-entry point: someone resuming a day later reads one line to learn
what must already be in hand.

Naming producers exposes the guide's real dependency graph, which is half the value. An inputs line
that has to cite a later section is an ordering defect the prose was hiding; reorder the guide, or
state the exception and cross-link both directions so neither phase is a surprise.

## The outputs block

An introductory line, then a fenced `text` block carrying one line per artifact as
`<TOKEN> — what it is; where it is kept`.

````text
Outputs of this phase:

```text
<BACKUP_PASSPHRASE> — the repository passphrase; password manager and paper record
<BUCKET_KEY_ID> — the key id the vendor issues; password manager
```
````

- A phase that produces an artifact MUST end with an outputs block.
- An outputs block MUST NOT be a heading.
- An outputs block MUST name the artifact and where it is kept, and MUST NOT define it.
- A phase that carries nothing forward MUST write `Outputs: none` and the reason.

The block is the last element of its section, so a reader skimming to the next heading still passes
it. Headings are the link namespace, and a derived list does not earn a target on every phase. The
definition stays with whatever owns that fact, per [00 — Model](./00-model.md); a block that explains
its artifact leaves two places to change. Silence about a phase that produces nothing reads as an
omission rather than as an answer.

## Where the phase devices apply

- Where a guide has more than one phase and an artifact crosses a phase boundary, the author MUST use
  an inputs line and an outputs block.

The three devices are for that case and no other.

- Not a single-phase guide. One outputs block with nothing downstream of it is ceremony.
- Not reference, explanation, or decision records. They have no phases, and a reference page that
  lists artifacts is the inventory, which owns them by name already.
- Not a guide written to be read with no access to the rest of the tree, such as a printed recovery
  procedure. It cannot carry a line that points off-page, so it carries one requirements list at the
  top, and the exemption is recorded with the project's other local exceptions.

## Length

The heading list is the length signal, not the line count. A reader scanning the headings sees the
whole procedure without scrolling. Where it no longer fits, the page is carrying more than one task,
or it is carrying reference material a lookup page should own.

Split it by the test in [01 — Placement](./01-placement.md): what the reader is doing stays in the
guide, what they are looking up moves to reference, and why it is so moves to explanation or a
decision record.

## Anti-patterns

- Narrated step: a sentence announcing the command and another describing its output.
- Inlined rationale: the concept explained at the step rather than linked to its owner.
- Buried command: the thing to type sitting in prose instead of a fence.
- Guide as encyclopedia: a complete option table pasted into a task page instead of linked.
- Verification by assertion: a closing claim that the procedure worked, with no command that shows it.
- Realistic placeholder: a slot filled with something shaped like a real value.
- Step-named token: `<STEP_3_OUTPUT>`, wrong the moment a step is inserted.
- Outputs heading: promoting the block to `## Outputs` puts a derived list in the link namespace and
  invites inbound links that a later phase cannot move.
- Second definition: an outputs block explaining what its artifact is rather than naming it.
- Notation everywhere: tokens scattered through reference and explanation pages, where nothing
  produces or consumes them.

## Sources

- Diátaxis, on a how-to guide as a sequence carrying action and only action:
  <https://diataxis.fr/how-to-guides/>
- Google developer documentation style guide, on numbered steps, one action per step, and the
  `Optional:` prefix: <https://developers.google.com/style/procedures>
