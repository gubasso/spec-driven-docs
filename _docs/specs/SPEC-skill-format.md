# Skill Format Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`skill-format:a-package-holds-the-manual-and-its-references` — A package holds the manual and its references](#skill-formata-package-holds-the-manual-and-its-references--a-package-holds-the-manual-and-its-references)
  - [`skill-format:the-manual-is-the-control-plane` — The manual is the control plane](#skill-formatthe-manual-is-the-control-plane--the-manual-is-the-control-plane)
  - [`skill-format:the-manual-links-every-reference-directly` — The manual links every reference directly](#skill-formatthe-manual-links-every-reference-directly--the-manual-links-every-reference-directly)
  - [`skill-format:a-reference-link-carries-its-condition` — A reference link carries its condition](#skill-formata-reference-link-carries-its-condition--a-reference-link-carries-its-condition)
  - [`skill-format:a-fact-lives-in-one-file` — A fact lives in one file](#skill-formata-fact-lives-in-one-file--a-fact-lives-in-one-file)
  - [`skill-format:shared-content-is-authored-once` — Shared content is authored once](#skill-formatshared-content-is-authored-once--shared-content-is-authored-once)
  - [`skill-format:the-manual-follows-the-stage-order` — The manual follows the stage order](#skill-formatthe-manual-follows-the-stage-order--the-manual-follows-the-stage-order)
  - [`skill-format:a-manual-stays-within-12000-characters` — A manual stays within 12,000 characters](#skill-formata-manual-stays-within-12000-characters--a-manual-stays-within-12000-characters)
  - [`skill-format:a-reference-stays-within-32000-characters` — A reference stays within 32,000 characters](#skill-formata-reference-stays-within-32000-characters--a-reference-stays-within-32000-characters)
  - [`skill-format:an-instruction-is-one-checkable-action` — An instruction is one checkable action](#skill-formatan-instruction-is-one-checkable-action--an-instruction-is-one-checkable-action)
  - [`skill-format:the-structure-follows-the-content` — The structure follows the content](#skill-formatthe-structure-follows-the-content--the-structure-follows-the-content)
  - [`skill-format:prose-is-only-a-reason-that-changes-behavior` — Prose is only a reason that changes behavior](#skill-formatprose-is-only-a-reason-that-changes-behavior--prose-is-only-a-reason-that-changes-behavior)
  - [`skill-format:the-rule-that-matters-most-comes-first` — The rule that matters most comes first](#skill-formatthe-rule-that-matters-most-comes-first--the-rule-that-matters-most-comes-first)
  - [`skill-format:a-choice-offers-one-default` — A choice offers one default](#skill-formata-choice-offers-one-default--a-choice-offers-one-default)
  - [`skill-format:a-hard-rule-is-a-check-not-capitals` — A hard rule is a check, not capitals](#skill-formata-hard-rule-is-a-check-not-capitals--a-hard-rule-is-a-check-not-capitals)
  - [`skill-format:one-term-names-one-concept` — One term names one concept](#skill-formatone-term-names-one-concept--one-term-names-one-concept)
  - [`skill-format:every-line-passes-the-deletion-test` — Every line passes the deletion test](#skill-formatevery-line-passes-the-deletion-test--every-line-passes-the-deletion-test)
  - [`skill-format:the-description-says-what-then-when` — The description says what, then when](#skill-formatthe-description-says-what-then-when--the-description-says-what-then-when)
- [Unenforced](#unenforced)

<!--TOC-->

## Purpose

Rules governing how a skill this repository ships is packaged, sized, loaded, and written. A skill is the instruction package an agent loads by name: a `SKILL.md` and the reference files it reads on demand. What the installer lands, and where, belongs to `SPEC-distribution.md`. No instance adopts this spec, because its subject is the skills the binary carries. Each rule states the finding it rests on and names its sources, and [`REFERENCE-skill-format-sources.md`](../reference/REFERENCE-skill-format-sources.md) holds each source with its URL, the date it was verified, and its finding.

## Requirements

### `skill-format:a-package-holds-the-manual-and-its-references` — A package holds the manual and its references

The author MUST keep each skill directory under `skills/` to one `SKILL.md` and the Markdown files directly under its `references/` directory.

The format also allows `scripts/` and `assets/`, but every deterministic step here is an `sdd` verb, and the vendor guides keep a README, a changelog, and an install guide out of a skill. Sources: Agent Skills specification, Anthropic guide, OpenAI skill-creator.

#### Scenario: A skill gains a helper script

- GIVEN a skill directory that gains `scripts/check.sh`
- WHEN the canon test suite runs
- THEN the check fails naming the file, because the step belongs in an `sdd` verb that a test can hold

Verify: `cargo nextest run -E 'test(every_skill_directory_holds_the_manual_and_its_references)'`

### `skill-format:the-manual-is-the-control-plane` — The manual is the control plane

The author MUST keep in `SKILL.md` the workflow, the decisions, and the invariants that bind every run, and MUST move knowledge that only some runs need into a reference file.

A loaded body stays in context on every later turn, so each line is a recurring cost, and a host loads a reference file only when the agent reads it. Sources: Claude Code skills, Agent Skills specification.

#### Scenario: A task's procedure sits in the manual

- GIVEN a `SKILL.md` that holds the whole procedure of a task one run in five needs
- WHEN an agent runs any other task
- THEN it carries that procedure as a distractor, so the procedure moves to a reference file

Verify: reviewer confirms that `SKILL.md` holds only what binds every run

### `skill-format:the-manual-links-every-reference-directly` — The manual links every reference directly

The author MUST link every reference file of a package from its `SKILL.md` by a path relative to the skill root, and MUST keep each reference file free of the path of any other reference file in its package.

An agent tends to preview a file it reached through another file, so the format and the vendor guide keep every reference one level deep from `SKILL.md`. Sources: Agent Skills specification, Claude skill authoring best practices.

#### Scenario: A task file sends the agent to a second task file

- GIVEN a reference file that names the path of another reference file
- WHEN the canon test suite runs
- THEN the check fails naming both files, because the composition of two tasks belongs in `SKILL.md`, the one file that links task files

Verify: `cargo nextest run -E 'test(every_reference_file_is_linked_from_its_manual)' && cargo nextest run -E 'test(no_reference_file_links_another)'`

### `skill-format:a-reference-link-carries-its-condition` — A reference link carries its condition

The author MUST write every link from `SKILL.md` to a skill's own reference file as one instruction that states when the agent reads it, in the form `If <condition>, read references/<file>.md`, and MUST name the pre-flight gate in the pre-flight section, read before any other action, and the plan gate in the gate section, read after the task pre-flights.

An instruction that names when to load a file works better than a bare pointer such as "see references/". Sources: Agent Skills best practices.

#### Scenario: A manual lists its references at the end

- GIVEN a `SKILL.md` that closes with a bare list of its reference files
- WHEN an agent reaches a step that needs one of them
- THEN nothing tells it which file applies, so it reads all of them or none

Verify: `cargo nextest run -E 'test(every_reference_link_carries_its_condition)'`

### `skill-format:a-fact-lives-in-one-file` — A fact lives in one file

The author MUST state each fact of a skill in exactly one file of its package, either `SKILL.md` or one reference file.

Information lives in `SKILL.md` or in a reference file, never in both, and two instructions that contradict each other leave the agent to pick one arbitrarily. Sources: OpenAI skill-creator, Claude Code memory.

#### Scenario: A task file restates a general rule

- GIVEN a general rule in `SKILL.md` that a task file restates in other words
- WHEN one copy changes and the other does not
- THEN the agent holds two rules that disagree, and it can follow either

Verify: reviewer confirms that no fact appears in two files of one package

### `skill-format:shared-content-is-authored-once` — Shared content is authored once

Where two skills need the same content, the author MUST write it once under `skill-shared/` and MUST NOT copy it into a skill directory.

Every tool that installs skills installs one whole directory per skill, and a tool that writes the copies from one source keeps one authored file, so a fix still lands once. Sources: Agent Skills specification, the skill installers.

#### Scenario: A skill carries its own copy of a shared gate

- GIVEN a skill reference file named like a file under `skill-shared/`
- WHEN the package is composed
- THEN the composition fails naming the file, because a package that silently kept one copy would ship a gate that no longer matches its source

Verify: `cargo nextest run -E 'test(a_name_both_sources_claim_fails_the_package) | test(every_skill_package_composes)'`

### `skill-format:the-manual-follows-the-stage-order` — The manual follows the stage order

The author MUST order the sections of `SKILL.md` as the opening paragraph, the general pre-flight, the general rules, the gate, the task links with their composition, and the general post-flight.

Models favor the instructions they read first, and a host that compacts a session keeps only the head of a skill, so what every run needs comes first, in the order the run takes. Sources: IFScale, Claude Code skills.

#### Scenario: A skill places its routing above its pre-flight

- GIVEN a `SKILL.md` whose task routing precedes the checks that classify the target
- WHEN an agent reads it top to bottom
- THEN it picks a task before it knows the state of the target

Verify: `cargo nextest run -E 'test(every_manual_follows_the_stage_order)'`

### `skill-format:a-manual-stays-within-12000-characters` — A manual stays within 12,000 characters

The author MUST keep the body of each `SKILL.md` at or below 12,000 characters, counted in characters and never in lines, and SHOULD keep it at or below 8,000.

12,000 characters is about 4,800 tokens at the 2.5 characters per token Anthropic states for its current tokenizer, under the 5,000 tokens a host keeps of a skill after compaction, and characters track tokens with a median error of 4% here, against 12% for lines. Sources: Claude models overview, Claude Code skills, the measurement.

#### Scenario: A manual gains a task procedure

- GIVEN a `SKILL.md` body that grows past 12,000 characters
- WHEN the canon test suite runs
- THEN the check fails naming the skill and its count, and the procedure moves to a reference file

Verify: `cargo nextest run -E 'test(every_manual_stays_within_12000_characters)'`

### `skill-format:a-reference-stays-within-32000-characters` — A reference stays within 32,000 characters

The author MUST keep each reference file at or below 32,000 characters, and MUST open one over 8,000 characters with a list of its sections.

The cap follows the 32 KiB limit Codex sets for instruction files, accuracy falls as input grows even with perfect retrieval, and an agent tends to preview only the head of a long file. Sources: OpenAI AGENTS.md guide, Du et al., Claude skill authoring best practices.

#### Scenario: A long reference file opens with prose

- GIVEN a reference file of 9,000 characters with no list of its sections
- WHEN the canon test suite runs
- THEN the check fails, because an agent that previews its head cannot see what the rest holds

Verify: `cargo nextest run -E 'test(every_reference_file_stays_within_32000_characters)'`

### `skill-format:an-instruction-is-one-checkable-action` — An instruction is one checkable action

The author MUST write each instruction in the imperative, as one action per step, and concrete enough to check, naming the exact command rather than an outcome such as "make sure it works".

The vendor guides ask for the imperative form and for steps with explicit inputs and outputs, and an instruction concrete enough to check is one an agent follows. Sources: Anthropic skill-creator, OpenAI skills guide, Claude Code memory.

#### Scenario: A step names an outcome

- GIVEN a step that reads "make sure the gates pass"
- WHEN an agent reaches it
- THEN it picks a check of its own, so the step names the exact command that runs the gates instead

Verify: reviewer confirms that each step is one imperative action that names its exact command

### `skill-format:the-structure-follows-the-content` — The structure follows the content

The author MUST use a numbered list where order matters, a bulleted list for unordered rules, and a table for a closed choice.

Numbered steps help where the order or the completeness of steps matters, and headers with bullets read better than dense paragraphs, though no format wins on every model. Sources: Claude prompting best practices, Claude Code memory, He et al..

#### Scenario: A sequence is written as bullets

- GIVEN a landing procedure written as an unordered list
- WHEN an agent follows it
- THEN nothing marks the order, and it can compare against a stage that does not exist yet

Verify: reviewer confirms that each list and table matches the shape of its content

### `skill-format:prose-is-only-a-reason-that-changes-behavior` — Prose is only a reason that changes behavior

The author MUST limit prose to a reason of one clause, written only where the reason changes what the agent does at an edge case.

A reason helps a model generalize a rule, and narration costs context on every turn, so the sources that disagree meet at a reason of one clause. Sources: Agent Skills evaluation guide, Claude prompting best practices, Claude Code skills.

#### Scenario: A rule carries its history

- GIVEN a rule followed by a paragraph on how an older release behaved
- WHEN an agent loads the skill
- THEN it pays for the paragraph on every turn and acts no differently

Verify: reviewer confirms that each prose clause is a reason that changes what the agent does

### `skill-format:the-rule-that-matters-most-comes-first` — The rule that matters most comes first

The author MUST place the rules that matter most first, and MUST write a rule that holds for the whole task as a standing instruction rather than as a step.

A host does not read a skill again on later turns and keeps only its first 5,000 tokens after compaction, and models favor earlier instructions. Sources: Claude Code skills, IFScale.

#### Scenario: A standing rule is written as a step

- GIVEN "read every path from the status report" written as step 2 of one procedure
- WHEN the agent reaches step 6 of another
- THEN it treats the rule as done and spells a path by hand

Verify: reviewer confirms that the rules that bind every run come first and read as standing instructions

### `skill-format:a-choice-offers-one-default` — A choice offers one default

Where the agent faces a choice, the author MUST give one default and the exception that overrides it, never a menu of equal options.

"Provide defaults, not menus". Sources: Agent Skills best practices.

#### Scenario: A step offers three equal profiles

- GIVEN a step that lists three profiles and names no default
- WHEN the request names none
- THEN the agent picks one arbitrarily, or asks a question the skill could have answered

Verify: reviewer confirms that each choice names one default and its exception

### `skill-format:a-hard-rule-is-a-check-not-capitals` — A hard rule is a check, not capitals

The author MUST write no all-caps emphasis in a skill, and MUST turn a rule that holds on every run into an `sdd` check or a test.

A token is a maximal run of ASCII letters and digits, and a token is a word in capitals when it holds two letters or more and every letter is uppercase. The check reads a skill file with its frontmatter, its fenced blocks, indented ones included, its code spans, and its link targets removed, and it accepts these acronyms alone: `API`, `CI`, `CLI`, `EARS`, `HTTP`, `HTTPS`, `ID`, `JSON`, `OIDC`, `PATH`, `SSH`, `TOML`, `URL`, `UTF`, and `YAML`.

Claude Opus 4.5 and 4.6 over-trigger on aggressive wording written for earlier models, and a rule that must hold every time belongs in a mechanism rather than in louder prose. Sources: Claude prompting best practices, Anthropic skill-creator, Claude Code skills.

#### Scenario: A step is shouted

- GIVEN a step that reads "NEVER edit a managed file"
- WHEN the canon test suite runs
- THEN the check fails naming the word, and the rule rests on `sdd verify`, which already reports a managed edit

Verify: `cargo nextest run -E 'test(no_skill_file_uses_capitals_for_emphasis) or test(the_capitals_contract_holds_its_cases)'`

### `skill-format:one-term-names-one-concept` — One term names one concept

The author MUST use one term for one concept across a package, and MUST write no wording that is true only at the time of writing.

"Choose one term and use it throughout the Skill", and time-sensitive wording goes stale. Sources: Claude skill authoring best practices.

#### Scenario: A task file renames the stage

- GIVEN "stage" in `SKILL.md` and "candidate directory" in a task file
- WHEN an agent reads both
- THEN it can take them for two things

Verify: reviewer confirms that each concept carries one term and that no line depends on the date it was written

### `skill-format:every-line-passes-the-deletion-test` — Every line passes the deletion test

The author MUST remove every line whose removal would not change what the agent does.

Compliance falls as the number of instructions rises, focused skills outperform exhaustive ones, and the vendors state the deletion test themselves. Sources: IFScale, ManyIFEval, SkillsBench, Agent Skills best practices, Claude Code memory.

#### Scenario: A skill explains version control

- GIVEN a paragraph that explains what a branch is
- WHEN the deletion test is applied
- THEN the agent acts the same without it, so the paragraph goes

Verify: reviewer confirms that removing any line would change what the agent does

### `skill-format:the-description-says-what-then-when` — The description says what, then when

The author MUST write each `description` in the third person as what the skill does, then when to use it, with the key terms first, within 1,024 characters and with no angle bracket.

The format caps the field at 1,024 characters, a host matches a skill by its description and can shorten it, and a vendor rejects XML tags in it. Sources: Agent Skills specification, Claude skill authoring best practices, OpenAI skills guide.

#### Scenario: A description opens with its trigger

- GIVEN a description that begins "Use this when you need to land docs"
- WHEN the canon test suite runs
- THEN the check fails, because a listing shows what a skill does before when to use it

Verify: `cargo nextest run -E 'test(every_skill_description_says_what_then_when)'`

## Unenforced

These rules require review, because no command can judge what an instruction means.

| Rule                                                        | Reviewer confirms                                                  |
| ----------------------------------------------------------- | ------------------------------------------------------------------ |
| `skill-format:the-manual-is-the-control-plane`              | `SKILL.md` holds only what binds every run                         |
| `skill-format:a-fact-lives-in-one-file`                     | No fact appears in two files of one package                        |
| `skill-format:an-instruction-is-one-checkable-action`       | Each step is one imperative action that names its exact command    |
| `skill-format:the-structure-follows-the-content`            | Each list and table matches the shape of its content               |
| `skill-format:prose-is-only-a-reason-that-changes-behavior` | Each prose clause is a reason that changes what the agent does     |
| `skill-format:the-rule-that-matters-most-comes-first`       | The rules that bind every run come first, as standing instructions |
| `skill-format:a-choice-offers-one-default`                  | Each choice names one default and its exception                    |
| `skill-format:one-term-names-one-concept`                   | Each concept carries one term, and no line depends on its date     |
| `skill-format:every-line-passes-the-deletion-test`          | Removing any line would change what the agent does                 |
