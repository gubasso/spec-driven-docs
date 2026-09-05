# SimpleEnglish Integration Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`simple-english:the-upstream-pattern-is-binding` — The upstream pattern is binding](#simple-englishthe-upstream-pattern-is-binding--the-upstream-pattern-is-binding)
  - [`simple-english:plain-is-the-default` — Plain is the default mode](#simple-englishplain-is-the-default--plain-is-the-default-mode)
  - [`simple-english:strict-is-explicit` — Strict is an explicit choice](#simple-englishstrict-is-explicit--strict-is-an-explicit-choice)
  - [`simple-english:the-dependency-is-available-offline` — The dependency is available offline](#simple-englishthe-dependency-is-available-offline--the-dependency-is-available-offline)
  - [`simple-english:upstream-terms-keep-their-names` — Upstream terms keep their names](#simple-englishupstream-terms-keep-their-names--upstream-terms-keep-their-names)
  - [`simple-english:structure-supplies-the-passage-mode` — Structure supplies the passage mode](#simple-englishstructure-supplies-the-passage-mode--structure-supplies-the-passage-mode)
  - [`simple-english:an-objective-check-matches-its-upstream-rule` — An objective check matches its upstream rule](#simple-englishan-objective-check-matches-its-upstream-rule--an-objective-check-matches-its-upstream-rule)
  - [`simple-english:an-exception-names-its-reason` — An exception names its reason](#simple-englishan-exception-names-its-reason--an-exception-names-its-reason)
  - [`simple-english:protected-content-stays-exact` — Protected content stays exact](#simple-englishprotected-content-stays-exact--protected-content-stays-exact)
  - [`simple-english:marketing-copy-stays-out-of-scope` — Marketing copy stays out of scope](#simple-englishmarketing-copy-stays-out-of-scope--marketing-copy-stays-out-of-scope)
  - [`simple-english:compatibility-adapters-match-upstream` — Compatibility adapters match upstream](#simple-englishcompatibility-adapters-match-upstream--compatibility-adapters-match-upstream)
- [The enforcement matrix](#the-enforcement-matrix)
- [The compatibility table](#the-compatibility-table)
- [Unenforced](#unenforced)

<!--TOC-->

## Purpose

Rules governing how this project adopts SimpleEnglish as its writing convention. SimpleEnglish is the upstream project this project depends on. It draws on ASD-STE100 Simplified Technical English, the controlled language that aerospace uses so a reader cannot misread an instruction. This project vendors the consumed SimpleEnglish surface byte-for-byte. It activates `Plain` mode by default for technical writing. It owns only the compatibility layer a single offline binary needs. This spec owns that integration boundary. It does not restate the SimpleEnglish rules. The rules live in the vendored skill at `.spec-driven-docs/upstreams/simpleenglish/skills/simple-english/SKILL.md`. The markdown a document is written in belongs to `SPEC-docs-format.md`. Where the vendored surface comes from and how it is attributed belongs to `SPEC-release.md`.

## Requirements

### `simple-english:the-upstream-pattern-is-binding` — The upstream pattern is binding

The author MUST write technical text by the complete SimpleEnglish pattern at the adopted release.

#### Scenario: An author reaches for a private style rule

- GIVEN a technical document under this convention
- WHEN the author applies a rule the vendored skill does not state
- THEN the review rejects it, because the vendored pattern is the one source and this project defines no second writing catalog

Verify: reviewer confirms the document follows the vendored SimpleEnglish pattern and adds no competing rule

### `simple-english:plain-is-the-default` — Plain is the default mode

The author MUST write in `Plain` mode for the technical texts SimpleEnglish names: documentation, guides, agent instructions, reference, CLI help, error messages, and release notes.

#### Scenario: A guide is written without a mode named

- GIVEN a new guide and no mode named in the request
- WHEN the author drafts it
- THEN `Plain` applies, because it is the default for every technical text this project writes

Verify: `pre-commit run simple-english --all-files`

### `simple-english:strict-is-explicit` — Strict is an explicit choice

Where a user or project names STE, ASD-STE100, or compliance, the author MUST apply `Strict` mode and MUST NOT apply it otherwise.

#### Scenario: A request asks for STE compliance

- GIVEN a request that names ASD-STE100 compliance
- WHEN the author writes the document
- THEN `Strict` adds the dictionary discipline in the vendored `references/strict-vocabulary.md`, and the reply to the user stays `Plain`

Verify: reviewer confirms `Strict` mode appears only where the request names STE, ASD-STE100, or compliance

### `simple-english:the-dependency-is-available-offline` — The dependency is available offline

The distribution MUST project the consumed SimpleEnglish surface into every instance, so an author reads the pattern without a network or a plugin install.

#### Scenario: An author opens the convention with no network

- GIVEN an installed instance and no network
- WHEN the author reads `.spec-driven-docs/upstreams/simpleenglish/skills/simple-english/SKILL.md`
- THEN the full pattern is present, because the binary carries it and the install projects it

Verify: `sdd verify --target .`

### `simple-english:upstream-terms-keep-their-names` — Upstream terms keep their names

The author MUST use the upstream names `SimpleEnglish`, `Plain`, and `Strict`, and MUST NOT coin a competing product or policy label.

#### Scenario: A document renames the mode

- GIVEN a document describing the default mode
- WHEN it calls the mode `Plain English` or `docs-language`
- THEN the review rejects the coined name, because the upstream names are the shared vocabulary

Verify: reviewer confirms the text uses `SimpleEnglish`, `Plain`, and `Strict`, and coins no competing label

### `simple-english:structure-supplies-the-passage-mode` — Structure supplies the passage mode

The gate MUST resolve the passage mode from document structure and MUST NOT infer it from a verb, a heading name, or an author declaration.

#### Scenario: A step and a paragraph carry different limits

- GIVEN a numbered step command in a guide and an ordinary paragraph
- WHEN the gate measures each sentence
- THEN the step command carries the 20-word limit and every other sentence carries 25, resolved from structure alone

Verify: `pre-commit run simple-english --all-files`

### `simple-english:an-objective-check-matches-its-upstream-rule` — An objective check matches its upstream rule

The gate MUST report every deterministic violation, and every finding MUST cite this rule with its upstream rule number and finding category.

#### Scenario: A descriptive sentence runs to 26 words

- GIVEN a descriptive sentence of 26 words outside every protected span
- WHEN the gate measures it
- THEN it prints a finding citing `simple-english:an-objective-check-matches-its-upstream-rule`, upstream rule 6.3, and the category `sentence-over-limit`

Verify: `pre-commit run simple-english --all-files`

### `simple-english:an-exception-names-its-reason` — An exception names its reason

Where a region needs an exception, the author MUST wrap it in the reasoned exception directive, and a bad directive MUST fail the gate.

#### Scenario: A directive opens and never closes

- GIVEN an opening exception directive with no closing directive
- WHEN the gate runs
- THEN it fails naming the unmatched directive, because a region left open silences the rest of the file

Verify: `pre-commit run simple-english --all-files`

### `simple-english:protected-content-stays-exact` — Protected content stays exact

The gate MUST leave every protected span exact and count it as one word: code, identifiers, commands, paths, URLs, product names, placeholders, quoted output, and uppercase RFC 2119 keywords.

#### Scenario: A backticked command runs long

- GIVEN a step whose command in backticks holds many words
- WHEN the gate counts the sentence
- THEN the command counts as one word and the keyword `MUST` is left unflagged, because both are protected

Verify: `pre-commit run simple-english --all-files`

### `simple-english:marketing-copy-stays-out-of-scope` — Marketing copy stays out of scope

The author MUST keep marketing and brand writing outside the convention, marking each such region with the exception directive and its reason.

#### Scenario: The README opens with a sales passage

- GIVEN a promotional passage in the public README
- WHEN the author writes it in a persuasive voice
- THEN it sits inside a reasoned exception directive, and the technical prose around it still follows `Plain`

Verify: `pre-commit run simple-english --all-files`

### `simple-english:compatibility-adapters-match-upstream` — Compatibility adapters match upstream

Where this project ports an upstream check, the author MUST keep it faithful to the vendored source and record every difference in the compatibility table.

#### Scenario: A ported check drops an upstream exception

- GIVEN the compiled gate porting an upstream boundary check
- WHEN it omits an exception the vendored source keeps
- THEN the conformance comparison fails, unless the difference is a documented row in the compatibility table

Verify: reviewer confirms each adapter difference from the vendored source is a row in the compatibility table

## The enforcement matrix

One row per deterministic check the gate runs. Each finding cites `simple-english:an-objective-check-matches-its-upstream-rule` with the upstream rule number and the category. The vendored skill and its catalog are normative. The vendored `evals/ste_lint.py` is advisory evidence.

| Category              | Upstream rule                   | Scope          | Parser behavior                                                                                                   |
| --------------------- | ------------------------------- | -------------- | ----------------------------------------------------------------------------------------------------------------- |
| `sentence-over-limit` | 5.1 procedural, 6.3 descriptive | every sentence | 20 words for a numbered step command in a guide, 25 for every other sentence                                      |
| `contraction`         | 4.2                             | every sentence | flags `n't`, `'ll`, `'re`, `'ve`, `'d`, `it's`, `you're`, not a possessive `'s`                                   |
| `perfect-tense`       | 3.4                             | every sentence | flags `has`, `have`, `had` before a past participle                                                               |
| `ing-verb`            | 3.5                             | every sentence | flags a comma before an `-ing` clause used as a verb                                                              |
| `banned-modal`        | 3.2                             | every sentence | flags `should`, `would`, `may`, `might`, `could`, `shall`                                                         |
| `semicolon`           | 8.1                             | every sentence | flags a semicolon outside a protected span                                                                        |
| `logic-dash`          | Section 8 dash rule             | every sentence | flags an em dash, a spaced double hyphen, or a spaced single hyphen between two statements, not a range or a flag |

## The compatibility table

The differences this project's adapters keep from the vendored source, each required by a single-binary, offline, citable-failure invariant.

| Upstream behavior                             | Local behavior                                                                     | Governing rule                                                | Test                        |
| --------------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------- | --------------------------- |
| A Python regex counter with look-around       | A compiled Rust scanner with an explicit tokenizer                                 | `simple-english:compatibility-adapters-match-upstream`        | `simple_english` unit tests |
| No rule citation on a finding                 | Each finding cites `simple-english:<rule>`                                         | `spec-to-code:a-gate-message-cites-the-rule`                  | `gate-message-cites-a-rule` |
| Mode named by the caller                      | Mode resolved from document structure                                              | `simple-english:structure-supplies-the-passage-mode`          | passage-mode suite          |
| Command split from its following sentence     | Whole list item carries 20 words where the parser cannot split them                | `simple-english:an-objective-check-matches-its-upstream-rule` | list-item fallback test     |
| Uppercase words are ordinary text             | Uppercase RFC 2119 keywords stay exact                                             | `simple-english:protected-content-stays-exact`                | protected-span suite        |
| A Node.js and Python hook activates the skill | The compiled Rust adapter serves instances, and no instance runs Node.js or Python | `simple-english:the-dependency-is-available-offline`          | scratch-instance checks     |

## Unenforced

These adopted rules are real and no command decides them. Cite `docs-specs:unenforced-rules-are-declared`. A reviewer asks them.

| Rule                                             | Why no command                            |
| ------------------------------------------------ | ----------------------------------------- |
| Active voice, agent named                        | requires reading the sentence             |
| One item, one name                               | requires knowing which words are synonyms |
| A technical term is defined at first use         | requires knowing the reader               |
| A word is common rather than jargon              | requires judging the alternative          |
| Marketing classification                         | requires reading the passage              |
| A sign of AI writing is absent                   | requires judging the prose                |
| A warning states the command before the risk     | requires reading the warning              |
| A vertical list mixes no instruction with a fact | requires reading the list                 |
