# Spec-Driven Docs

A canonical method for keeping current specifications, immutable decision records, executable gates, and code traceability coherent for people and coding agents.

## What this is

Two products in one repository. The method is what a reader loads: `method/`, `comparison-docs/`, `templates/` and `reference/` state the rules and the shapes that carry them. The distribution is what a project installs: the `sdd` binary, built from `src/`, carries the whole payload and keeps instances upgradable. The payload is gates, verifier, spec seeds, templates, and the method itself. This repository is the canon. A project that installs it is an instance, and [method/glossary.md](./method/glossary.md) fixes both terms.

## Install

```bash
cargo install spec-driven-docs
```

Prebuilt installers ship with each GitHub release, and `cargo binstall spec-driven-docs` resolves them.

## Quick paths

- Read the method: [method/README.md](./method/README.md), or `sdd method --list` anywhere.
- Author a comparison: [comparison-docs/README.md](./comparison-docs/README.md).
- Instantiate into a project: `sdd init`, documented in [instance/README.md](./instance/README.md#instantiate).
- Verify an installed instance: `sdd verify`, documented in [instance/README.md](./instance/README.md#verify).
- Upgrade an installed instance: `sdd upgrade`, documented in [instance/README.md](./instance/README.md#upgrade).
- Copy stable templates: [templates/](./templates/), or `sdd template --list`.
- Install the agent skills at user scope: `sdd skill install`, documented in [instance/README.md](./instance/README.md#skills).
- Cut a release: [_docs/guides/release.md](./_docs/guides/release.md). First-time bootstrap: [_docs/guides/release-setup.md](./_docs/guides/release-setup.md).

The `sdd` CLI is the whole interface. Nothing here is tied to a particular editor or agent.

## Conventions and dependencies

The writing convention lives in one project-owned chapter, available offline through `sdd method writing-style`. A fresh `sdd init` routes document authors to that command from the project's `AGENTS.md`. No delivered gate judges prose against the style. The chapter draws its word and sentence register from [SimpleEnglish](https://github.com/AminBlg/SimpleEnglish) and its information architecture from [i-have-adhd](https://github.com/ayghri/i-have-adhd), at the revisions it names.

Every adopted convention and material dependency has one owner and an outward link:

- The writing contract, which states how the style binds and how a project reaches it: [`_docs/specs/SPEC-writing-style.md`](./_docs/specs/SPEC-writing-style.md). The style it routes to is the chapter, served by `sdd method writing-style`.
- The full catalog of dependencies and conventions: [reference/dependencies-and-conventions.md](./reference/dependencies-and-conventions.md).
- The pinned upstream, its license, and the vendored paths: [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md), also printed by `sdd license --third-party`.
- The freshness registry that detects a moved upstream: `sdd track status` reads it offline, and `sdd track check` compares the pinned revision online.

## License

The method is under [CC BY 4.0](./LICENSE-CC-BY-4.0) and the distribution that installs it is under the [MIT License](./LICENSE-MIT). [LICENSE](./LICENSE) states which side each file falls on, and `sdd license` prints the terms the binary carries.
