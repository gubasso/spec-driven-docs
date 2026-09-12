# Writing style

This style governs documents. It makes instructions direct and explanations easy to scan.

## Precedence

Apply these authorities from highest to lowest:

1. Accuracy and safety.
2. An explicit instruction from the operator.
3. The project's own documentation convention.
4. This style.

A rule that would delete the answer loses. Keep the document's required shape.

Bad: Shorten a security warning until the required consequence disappears.

Good: Keep the full consequence, even when it exceeds a sentence target.

## Classify the passage first

Decide whether the passage instructs or describes before writing it.

Procedural text tells the reader what to do.

- Use the imperative mood. Bad: "The operator must open the log." Good: "Open the log."
- Put one instruction in each sentence. Bad: "Open the file and replace the value." Good: "Open the file. Replace the value."
- Keep each sentence at 20 words or fewer. Bad: "Open the report and inspect every warning before you decide whether the deployment can safely continue to the next environment." Good: "Open the report. Inspect every warning. Decide whether the deployment can continue."

Descriptive text explains a subject.

- Use simple tenses. Bad: "The worker has been reading the queue." Good: "The worker reads the queue."
- Keep each sentence at 25 words or fewer. Bad: "The manifest records every installed file together with its ownership class, destination, digest, source version, and all details needed during later verification." Good: "The manifest records each installed file. It includes ownership, destination, digest, and source version."
- Keep one topic in each paragraph. Bad: "The parser reads YAML. Releases use tags." Good: Put parser facts in one paragraph and release facts in another.

## Grammar

- Use simple tenses. Bad: "The command has created the file." Good: "The command created the file."
- Use active voice. Bad: "The file is written by the installer." Good: "The installer writes the file."
- Name the actor. Bad: "The cache is cleared." Good: "The command clears the cache."
- Do not put an `-ing` verb after a comma. Bad: "The hook failed, blocking the commit." Good: "The hook failed and blocked the commit."
- Put a condition before its command. Bad: "Read the log if the build fails." Good: "If the build fails, read the log."
- Use `can` for ability, `will` for a certain future result, and `must` for an obligation. Bad: "The user should restart the service." Good: "The user must restart the service."
  - This rule governs lowercase prose. [`rules.md`](./rules.md#keywords) owns the uppercase keywords.
- Do not use contractions. Bad: "The command doesn't change the file." Good: "The command does not change the file."
- Do not use a semicolon. Bad: "The check failed; read the report." Good: "The check failed. Read the report."
- Do not use an em dash. Bad: "The record is immutable—the spec owns the present." Good: "The record is immutable. The spec owns the present."

## Vocabulary

- Use one word for one meaning. Bad: "The command checks, verifies, and validates the record." Good: "The command verifies the record."
- Keep a noun chain at three words or fewer. Bad: "repository release workflow configuration file." Good: "configuration file for the repository release workflow."
- Define a concept term at first use. Bad: "The projection enters the payload." Good: "A projection maps a source file to its installed path."
- State the fact instead of its importance. Bad: "It is crucial to preserve the hash." Good: "Preserve the hash."
- Delete filler such as `simply`, `seamlessly`, `robust`, `powerful`, `comprehensive`, `leverage`, `crucial`, `in order to`, and `it is worth noting`. Bad: "Simply leverage the robust cache." Good: "Read the cache."

## Layout

- Do not use a bold lead-in. Bad: `**Check:** Run the test.` Good: "Run the test."
- Do not use emoji. Bad: "✅ The test passed." Good: "The test passed."
- Use a vertical list for three or more parallel items. Bad: "The record needs a title, status, owner, and date." Good: Put the four fields in a vertical list.
- State the command before the risk in a warning. Bad: "You can lose data, so take care with `rm -rf`." Good: "`rm -rf` deletes the directory without using the trash."

[`format.md`](./format.md) owns Markdown mechanics and size budgets. This chapter owns the prose register inside that structure.

## What the style is not

No command judges a document against this style.

Bad: Add a prose gate that rejects an adopted corpus.

Good: Read the style before authoring or editing prose, then use review for judgment.

A document already in the tree converts the next time an author edits it.

Bad: Rewrite the whole corpus to adopt the style.

Good: Apply the style only to the document under edit.

Nothing counts or reports documents that have not converted.

Bad: Publish a remaining-document count that turns conversion into a sweep.

Good: Leave untouched documents alone until an author edits them.

## Sources

The style merges two sources by layer. It copies neither source as a runtime dependency.

| Source                                                            | Revision read                              | Terms                                | Contribution                                      |
| ----------------------------------------------------------------- | ------------------------------------------ | ------------------------------------ | ------------------------------------------------- |
| [AminBlg/SimpleEnglish](https://github.com/AminBlg/SimpleEnglish) | `d9e523409686e88df175623f7a692d025aff95b1` | MIT, Copyright (c) 2026 AminBlg      | Word and sentence register                        |
| [ayghri/i-have-adhd](https://github.com/ayghri/i-have-adhd)       | `24d22f783e57cb73c957848b588c6f651b6f9cd8` | MIT, Copyright (c) 2026 Ayoub Ghriss | Information architecture and contrasting examples |
