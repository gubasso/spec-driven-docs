# Spec-Driven Docs

A canonical method for keeping current specifications, immutable decision records, executable gates,
and code traceability coherent for people and coding agents.

## What this is

Two products in one repository. The method is what a reader loads: `method/`, `comparison-docs/`,
`templates/` and `reference/` state the rules and the shapes that carry them. The distribution is
what a project installs: the `sdd` binary, built from `src/`, carries the whole payload — gates,
verifier, spec seeds, templates, and the method itself — and keeps instances
upgradable. This repository is the canon; a project that installs it is an instance, and
[method/glossary.md](./method/glossary.md) fixes both terms.

## Install

```bash
cargo install spec-driven-docs
```

Prebuilt installers ship with each GitHub release, and `cargo binstall spec-driven-docs` resolves
them.

## Quick paths

- Read the method: [method/README.md](./method/README.md), or `sdd method --list` anywhere.
- Author a comparison: [comparison-docs/README.md](./comparison-docs/README.md).
- Instantiate into a project: `sdd init` — [instance/README.md](./instance/README.md#instantiate).
- Verify an installed instance: `sdd verify` — [instance/README.md](./instance/README.md#verify).
- Upgrade an installed instance: `sdd upgrade` — [instance/README.md](./instance/README.md#upgrade).
- Copy stable templates: [templates/](./templates/), or `sdd template --list`.
- Cut a release: [_docs/guides/release.md](./_docs/guides/release.md).

The `sdd` CLI is the whole interface. Nothing here is tied to a particular editor or agent.

## License

The method is under [CC BY 4.0](./LICENSE-CC-BY-4.0) and the distribution that installs it is under
the [MIT License](./LICENSE-MIT). [LICENSE](./LICENSE) states which side each file falls on, and
`sdd license` prints the terms the binary carries.
