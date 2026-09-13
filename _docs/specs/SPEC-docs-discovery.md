# Docs Discovery Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`docs-discovery:one-catalog-describes-every-served-document` — One catalog describes every served document](#docs-discoveryone-catalog-describes-every-served-document--one-catalog-describes-every-served-document)
  - [`docs-discovery:an-instance-is-routed-to-the-index` — An instance is routed to the index](#docs-discoveryan-instance-is-routed-to-the-index--an-instance-is-routed-to-the-index)

<!--TOC-->

## Purpose

Rules governing how a reader reaches the corpus this binary carries. They cover the one catalog that describes every served document, and the route an installed instance gets to the index. No instance adopts this spec: its subject is the binary's own corpus, so an instance holding these rules holds obligations it cannot violate. What a release lands into a target is stated in `SPEC-distribution.md`. What a document's own shape is belongs to `SPEC-docs-format.md`.

## Requirements

### `docs-discovery:one-catalog-describes-every-served-document` — One catalog describes every served document

Every document a shelf serves MUST have exactly one entry in `instance/docs-catalog.toml`, and every entry MUST resolve to a document a shelf serves or to an argv whose first value is a subcommand this binary offers. An entry MUST carry a title, a one-line summary, and a kind, and MUST NOT carry a size. Identifiers and aliases MUST be unique when case-folded. Every listing that names a served document MUST take its description from that catalog.

#### Scenario: A chapter lands with no catalog entry

- GIVEN a new method chapter added to the shelf and no entry written for it
- WHEN the canon test suite runs
- THEN it fails naming the chapter, because a document nobody described is a document the index cannot route to

Verify: `cargo nextest run -E 'binary(canon) + binary(cmd_docs)'`

### `docs-discovery:an-instance-is-routed-to-the-index` — An instance is routed to the index

The managed documentation block an instance receives MUST name the index verb in at most one line, and MUST NOT restate what a topic says. A digest MUST route to a topic rather than copy it. A skill MUST route to a topic through `sdd docs` or the shelf verb that serves it, and MUST NOT restate the topic's own description.

#### Scenario: An agent opens a repository that adopted this tool

- GIVEN an instance whose always-loaded block is the only thing the agent has read
- WHEN the agent has a question about operating `sdd`
- THEN the block names `sdd docs`, so the agent reaches any chapter in two commands and loads no corpus it does not need

Verify: `cargo nextest run -E 'binary(canon) + binary(cmd_init)'`
