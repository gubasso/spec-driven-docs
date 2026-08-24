# Spec-Driven Docs

A canonical method for keeping current specifications, immutable decision records, executable gates,
and code traceability coherent for people and coding agents.

## What this is

Two products in one repository. The method is what a reader loads: `method/`, `comparison-docs/`,
`templates/` and `reference/` state the rules and the shapes that carry them. The distribution is
what a project installs: `scripts/`, `.hooks/`, `instance/`, `skills/` and `commands/` put those
rules into a repository and keep them upgradable. This repository is the canon; a project that
installs it is an instance, and [method/glossary.md](./method/glossary.md) fixes both terms.

## Quick paths

- Read the method: [method/README.md](./method/README.md).
- Author a comparison: [comparison-docs/README.md](./comparison-docs/README.md).
- Instantiate into a project: [instance/README.md](./instance/README.md#instantiate).
- Verify an installed instance: [instance/README.md](./instance/README.md#verify).
- Upgrade an installed instance: [instance/README.md](./instance/README.md#upgrade).
- Copy stable templates: [templates/](./templates/).

The scripts are the agent-agnostic interface. Claude skills and commands route to the same scripts
and are optional.
