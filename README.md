# Spec-Driven Docs

A canonical method for keeping current specifications, immutable decision records, executable gates,
and code traceability coherent for people and coding agents.

## What this is

Two products in one repository. The method is what a reader loads: `method/`, `comparison-docs/`,
`templates/` and `reference/` state the rules and the shapes that carry them. The distribution is
what a project installs: `scripts/`, `.hooks/` and `instance/` put those rules into a repository
and keep them upgradable. This repository is the canon; a project that
installs it is an instance, and [method/glossary.md](./method/glossary.md) fixes both terms.

## Quick paths

- Read the method: [method/README.md](./method/README.md).
- Author a comparison: [comparison-docs/README.md](./comparison-docs/README.md).
- Instantiate into a project: [instance/README.md](./instance/README.md#instantiate).
- Verify an installed instance: [instance/README.md](./instance/README.md#verify).
- Upgrade an installed instance: [instance/README.md](./instance/README.md#upgrade).
- Copy stable templates: [templates/](./templates/).
- Cut a release: [_docs/guides/release.md](./_docs/guides/release.md).

The scripts are the whole interface. Nothing here is tied to a particular editor or agent.

## License

The method is under [CC BY 4.0](./LICENSE-CC-BY-4.0) and the distribution that installs it is under
the [MIT License](./LICENSE-MIT). [LICENSE](./LICENSE) states which side each directory falls on.
