# Measure skill size in characters

## Context and Problem Statement

A skill body was capped at 150 lines. Prose here is unwrapped, so one line holds anything from one word to a whole paragraph. A measurement over this repository's Markdown found that lines predict tokens with a median error of 12% and a worst error of 126%, against 4% and 39% for characters. The sources disagree on the unit: 500 lines, 5,000 tokens, and 5,000 words.

## Considered Options

- `characters: 12,000 for a body, 32,000 for a reference file` — chosen.
- `tokens` — rejected: no tokenizer is public for every agent, and token counts differ between tokenizers by 20% to 50%.
- `words` — rejected: a median error of 9%, and a long path or command counts as one word.
- `keep lines, at a higher cap` — rejected: the worst error stays above 100%, so a line cap allows anything from about 3,000 to about 17,000 tokens.

## Decision Outcome

Chosen option: `characters: 12,000 for a body, 32,000 for a reference file`. 12,000 characters is about 4,800 tokens at the conservative rate of 2.5 characters per token Anthropic states for its current tokenizer. That keeps a body under the 5,000 tokens a host keeps of a skill after compaction. 32,000 characters follows the 32 KiB limit Codex sets for instruction files. A reference file over 8,000 characters opens with a list of its sections, because an agent tends to preview only its head.

Enforced by `skill-format:a-manual-stays-within-12000-characters`, `skill-format:a-reference-stays-within-32000-characters`, and `distribution:a-skill-obeys-the-portable-format`.

## Consequences

- Good: a dense line and a short one count by their real size.
- Good: one count serves every agent, with no tokenizer to pin.
- Bad: characters stay a proxy. A code-heavy or non-English file carries more tokens per character, and the caps leave room for that rather than measure it.

## Status

Accepted
