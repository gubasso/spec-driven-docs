# Author the writing style and drop the gate

## Context and Problem Statement

A delivered prose gate reached every Markdown file in a corpus that nobody wrote for its rules. The binding upstream pattern also prevented this project from authoring the style it needed.

## Considered Options

- `author one style chapter and remove the delivered prose gate` — chosen.
- `keep the gate unchanged` — rejected: it rejects correct prose in projects that adopted only the documentation method.
- `narrow the gate` — rejected: any delivered prose judge still imposes this project's register on another project's text.
- `keep a narrowed scanner as an on-demand command` — rejected: it creates a second definition that can drift from the chapter.
- `vendor the upstream unchanged` — rejected: an upstream pattern cannot express the project's chosen layers and precedence.

## Decision Outcome

Chosen option: `author one style chapter and remove the delivered prose gate` — `method/writing-style.md` owns the rules, and `sdd method writing-style` serves them on demand.

Enforced by `writing-style:an-existing-document-converts-when-edited`, `writing-style:the-documentation-block-routes-to-the-style`, `writing-style:no-delivered-gate-judges-prose`, `writing-style:sources-name-the-revision-read`, and `writing-style:the-style-lives-in-one-document`.

The prior citations map as follows. `simple-english:the-upstream-pattern-is-binding` and `simple-english:plain-is-the-default` map to the one-document rule and the documentation-block route. `simple-english:an-objective-check-matches-its-upstream-rule` maps to the rule that prohibits a delivered prose gate. `simple-english:the-dependency-is-available-offline` has no successor and is dropped.

## Consequences

- Good: a project can adopt the documentation method without rewriting its prose.
- Good: the writing convention has one reviewable source with explicit precedence.
- Bad: no command detects a document that ignores the style.
- Bad: conversion happens only when an author edits a document.

## Status

Implemented

Enacted by `method/writing-style.md`, the method reader, and the managed root `AGENTS.md` block.
