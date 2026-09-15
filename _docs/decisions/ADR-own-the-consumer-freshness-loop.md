# Own the consumer freshness loop

## Context and Problem Statement

Every project that pins this tool moved the pin by hand, and one was found four releases behind. A tool distributed as a development dependency owes its consumers a loop that moves the pin without a person: one transaction, one rate limit, two reporting registers, and no write the consumer did not review. The release-workflow tool this repository pins already ships that loop for its own consumers, so the question was whose code moves the `sdd` pin.

## Considered Options

- `build the loop in this binary, against the portable chapter` — chosen.
- `depend on the release-workflow tool's crate or call it at runtime` — rejected: every consumer of this tool would then need the other one.
- `vendor its modules` — rejected: a copy drifts at the first upstream fix.
- `document the wiring and let each consumer script it` — rejected: a capability nobody offers reaches only the operator who already knew it existed.

## Decision Outcome

Chosen option: `build the loop in this binary, against the portable chapter` — `sdd self-depend` carries its own manager detection, venue matrix, pin transaction, stamp, and fragments, written against the chapter that states the obligation. The two tools read each other only as worked examples.

The venue list is this tool's own, proven by its release path: the crate on its registry, the flake served at every tag, and the archives attached to each forge release.

Enforced by `acquisition:every-pair-carries-a-verdict`, `acquisition:the-pin-moves-in-one-transaction`, and `acquisition:the-shell-entry-caller-is-rate-limited-and-silent`.

## Consequences

- Good: a consumer adopts this tool alone, and a landing can offer the wire, because the binary that lands is the binary that syncs.
- Good: a new manager or venue is one variant plus its rows, and the matrix test fails until every pair is classified.
- Bad: two tools carry two implementations of one pattern, and a defect in one is fixed in the other by hand.
- Bad: this repository does not pin its own binary, so the loop runs only in consumers and in tests.

## Status

Accepted
