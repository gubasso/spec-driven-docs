# Declare one documentation catalog

## Context and Problem Statement

The binary carries the method, the spec seeds, and the templates, and all of it renders offline. None of it is discoverable: `sdd method --list` prints nineteen bare names, and the always-loaded block an instance receives names no reader verb. An agent in a repository that adopted this tool can read every word the binary carries and has nothing that tells it to.

A corpus reachable by a command nobody is told to run is not reachable. The cheapest fix is one always-loaded sentence pointing at one described index, not a second copy of the corpus where agents already read.

## Considered Options

- `one catalog, one index verb, one routing line` — chosen.
- `copy the routing table into the managed block` — rejected: a copy of the corpus in the file every session loads, drifting per release.
- `generate each summary` — rejected: a first sentence is written for a reader who already arrived.
- `leave the listings bare` — rejected: the state this record ends.

## Decision Outcome

Chosen option: `one catalog, one index verb, one routing line`. `instance/docs-catalog.toml` carries one entry per served document, plus the operator tasks a help page answers. `sdd docs` is the index, `sdd docs <topic>` resolves one, and the three shelf listings describe each name from it.

Three fields are authored: the title, the summary, and the aliases. Size is read from the embedded bytes when asked, so it cannot go stale. A document declaring a token estimate passes that number through, because this repository runs no tokenizer and prints no number it did not measure.

A target is a document or an argv, and there is no third kind.

Enforced by `docs-discovery:one-catalog-describes-every-served-document`.

## Consequences

- Good: an agent reaches any chapter in two commands, having loaded a line.
- Good: one name cannot be described two ways.
- Bad: a summary can drift from the prose it summarizes. Review holds that.
- Bad: a new document lands with an entry or the suite fails.

## Status

Accepted
