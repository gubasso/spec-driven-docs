# 03 — Rules

A rule is the smallest unit this framework names, cites, and checks. It is one
``### `<spec-slug>:<rule-slug>` — <title>`` block inside a spec, never a file of its own. This chapter owns the block: its grammar, its
identifier, and the command that proves it.

## The block

Five parts, all required. The identifier and the title share the heading line, so the generated
table of contents becomes an index of rule IDs rather than a list of sentences.

```markdown
### `decision-records:filename-carries-no-digit` — Decision record filenames carry no number

When an author creates a decision record, the author MUST name it `ADR-<slug>.md`.

#### Scenario: Two worktrees add a record for the same seam

- GIVEN two branches each add a decision record
- WHEN both name the same architectural seam
- THEN the filenames collide and git reports a conflict

Verify: `find _docs/decisions -name 'ADR-*' | rg '[0-9]' && exit 1 || exit 0`
```

| Part      | Carries                                            |
| --------- | -------------------------------------------------- |
| Title     | what the rule is about, as a statement             |
| ID        | the citation token                                 |
| Statement | the binding sentence                               |
| Scenario  | one case that distinguishes compliance from breach |
| Verify    | the command that decides it                        |

A rule is not a file. Splitting a domain across dozens of files costs a read apiece and defeats the
one-level-deep rule in [05 — Agent Context](./05-agent-context.md); a fifteen-rule spec is one read.

## Grammar

Every statement is one sentence in one of five patterns, with an RFC 2119 keyword in place of the
traditional `shall`.

| Pattern           | Template                                                      |
| ----------------- | ------------------------------------------------------------- |
| Ubiquitous        | `The <subject> MUST <response>.`                              |
| State-driven      | `While <precondition>, the <subject> MUST <response>.`        |
| Event-driven      | `When <trigger>, the <subject> MUST <response>.`              |
| Optional feature  | `Where <feature is included>, the <subject> MUST <response>.` |
| Unwanted behavior | `If <trigger>, then the <subject> MUST <response>.`           |

Patterns combine when a rule has both a precondition and a trigger:

```text
While <precondition>, when <trigger>, the <subject> MUST <response>.
```

- A requirement statement MUST match one of the five patterns.
- A requirement statement MUST be one sentence.
- A requirement statement MUST name a subject that can act.

The grammar is what makes leanness enforceable rather than aspirational. "Prefer short records" is
not a pattern and fails the gate; "The author MUST keep a record at or below 350 words" is.

Naming a subject that can act is the rule that catches most bad requirements. "The documentation MUST
be consistent" has no actor and no failure condition. "The author MUST use one term for one concept
throughout a spec" has both.

## Keywords

Use MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT, MAY, in capitals, as RFC 2119 and
RFC 8174 define them. Capitalization is what makes them normative and greppable; the same words in
lowercase are ordinary prose.

Prefer MUST and MUST NOT. A rule stated as SHOULD is a rule nobody is accountable for, and a spec
whose every rule is a SHOULD is a style guide.

## Positive statements

State what to do. A prohibition tells a reader the space of wrong answers and leaves them to find the
right one, and compliance falls as prohibitions accumulate.

- A specification SHOULD state each rule as an action to take.
- Where a specification states a prohibition, it MUST pair it with the action that replaces it.
- A specification MUST NOT carry more than five prohibitions.

```text
weaker:   The author MUST NOT use bold.
stronger: The author MUST mark identifiers with inline code, normativity with RFC 2119
          keywords, and structure with headings.
```

The cap is the part that bites. Past roughly half a dozen prohibitions, a model begins dropping them,
and the ones it drops are not the ones the author would have chosen. When a spec wants a seventh,
the prohibitions are standing in for a positive rule that has not been written.

## Identifiers

```text
<spec-slug>:<rule-slug>
```

- A rule ID MUST be `<spec-slug>:<rule-slug>`, lowercase and kebab-case.
- A rule ID MUST be unique across the project.
- A rule ID MUST NOT change when the statement is reworded.
- A rule ID MUST NOT contain a number allocated by a counter.

One pattern, no special case: a spec with a single rule still gives it an ID. A second pattern would
cost every reader a decision on every rule.

Slugs rather than numbers, for the reason that decides decision-record names too. A counter needs an
allocator, and two branches allocating in parallel collide on a value that means nothing. Two branches
choosing the same slug have collided on a subject, which is a conflict worth having.

The ID is stable and the sentence is not. Reword the statement freely; the ID is what commits,
reviews, comments, and code refer to.

Cite the ID wherever the rule is the reason for something:

```text
commit    fix(docs): honor decision-records:filename-carries-no-digit
review    violates decision-records:filename-carries-no-digit
hook      FAIL decision-records:filename-carries-no-digit
comment   # decision-records:filename-carries-no-digit
```

## Identifiers make citations checkable

This is the property that repays the whole scheme. An agent asserting "the rules require X" cannot be
audited. An agent asserting "per `decision-records:filename-carries-no-digit`" can:

```bash
rg -F 'decision-records:filename-carries-no-digit' _docs/specs
```

A fabricated citation returns nothing. A real one returns the line that binds it, and the reader can
read the sentence rather than trust the summary.

- An agent citing a rule MUST cite it by ID.

## Scenarios

One scenario per requirement, in GIVEN / WHEN / THEN, four bullets or fewer. It shows the case that
separates compliance from breach.

Write the scenario that would be argued about. A scenario restating the rule in other words adds
length and settles nothing; a scenario naming the ambiguous case settles the ambiguity.

## Open questions

A rule sometimes must be written while something about it is undecided. Mark the question at it.

```markdown
### `plan-record:lane-holds-one-story` — A lane holds one story

While a lane is active, the lane MUST hold exactly one story.

[NEEDS CLARIFICATION: a hotfix must preempt an active lane without evicting its story]
```

- A marker MUST be `[NEEDS CLARIFICATION: <question>]` and carry the question, not the topic.
- A spec MUST NOT carry more than three markers.
- A unit of work MUST NOT enact a rule whose requirement carries a marker.

It is the one exception to the no-prose-between-requirements rule, because it is a property of the
requirement above it. A spec that hides the question invites an agent to resolve it silently.

Three is the cap because a marker is for a question that changes scope, reads several ways with
different consequences, or has no sensible default. Blocking enactment rather than the commit keeps
it honest: an open question is legitimate state for an agreed spec, and shipping code against one is
not. The inventory is `rg -F '[NEEDS CLARIFICATION' <root>/specs`, never a page that lists them.

## Verification

- Every requirement MUST carry a `Verify:` line.
- A verification command MUST exit non-zero when the rule is violated.

The command is the rule's teeth. It appears three times over: in the spec so a reader can run it, in
the hook so a commit is gated, and in the failure message so a breach names its own rule.

When a rule genuinely cannot be checked by a command, the `Verify:` line names the human procedure
instead, and [08 — Gates](./08-gates.md) records it as unenforced. An unenforced rule declared as such
is honest; an unenforced rule presented as binding is the reason readers stop believing specs.

```text
checkable  Verify: `find _docs/decisions -name 'ADR-*' | rg '[0-9]' && exit 1 || exit 0`
human      Verify: reviewer confirms the scenario names the contested case, not a restatement
```

The checkable form exits non-zero on breach and zero otherwise. A command that merely reports, such as
a bare count, is not a verification: it passes whatever it finds.

## Sources

- EARS, for the five patterns: <https://alistairmavin.com/ears/>
- GitHub Spec Kit, for the clarification marker and its cap:
  <https://github.com/github/spec-kit/blob/main/spec-driven.md>
- RFC 2119 and RFC 8174, for the keywords and the capitalization rule:
  <https://www.rfc-editor.org/rfc/rfc2119>, <https://www.rfc-editor.org/rfc/rfc8174>
- Anthropic, on stating what to do rather than what not to do:
  <https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices>
