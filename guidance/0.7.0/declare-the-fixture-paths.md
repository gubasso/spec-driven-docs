# Declare the fixture paths

`sdd gate suppression-names-its-case` now reads every `KI-<slug>` token outside the documentation root. A test that builds a known-issue record in a temporary directory carries such a token and is not citing a case.

1. Run `sdd gate suppression-names-its-case` and read what it names.
2. For each path whose tokens are fixtures rather than citations, add an entry under `gates:` in the project's declaration.
3. Run the gate again. It reports nothing.
