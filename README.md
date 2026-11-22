# Spec-Driven Docs

A canonical method for keeping current specifications, immutable decision records, executable gates, and code traceability coherent for people and coding agents.

## What this is

Two products in one repository. The method is what a reader loads: `method/`, `comparison-docs/`, `templates/` and `reference/` state the rules and the shapes that carry them. The distribution is what a project installs: the `sdd` binary, built from `src/`, carries the whole payload and keeps instances upgradable. The payload is gates, verifier, spec seeds, templates, and the method itself. This repository is the canon. A project that installs it is an instance, and [method/glossary.md](./method/glossary.md) fixes both terms.

## Install

Pick the manager that the project already runs. `X.Y.Z` is the release version, and the binary is `sdd`. [REFERENCE-install-sources.md](./_docs/reference/REFERENCE-install-sources.md) cites the upstream documentation behind each command.

### Nix flake

Add the input to `flake.nix`:

```nix
spec-driven-docs = {
  url = "github:gubasso/spec-driven-docs/vX.Y.Z";
  inputs.nixpkgs.follows = "nixpkgs";
  inputs.spec-driven-docs.follows = "";
};
```

Then add `spec-driven-docs.packages.${system}.default` to the packages of the devshell. The empty follows cuts the previous sdd release that the flake pins for its own development, so `flake.lock` holds one sdd node.

To move the pin, edit the tag in the URL, then run:

```bash
nix flake update spec-driven-docs
```

`exobrain outdated` warns when a newer release is out.

### mise

```bash
mise use cargo:spec-driven-docs@X.Y.Z
```

### devbox

```bash
devbox add github:gubasso/spec-driven-docs/vX.Y.Z#default
```

### asdf

No asdf plugin for sdd exists. An asdf user installs the binary from the GitHub release.

### The GitHub release

Each release tag carries a prebuilt x86_64 Linux archive and a shell installer. This route serves asdf users and any host with no manager:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/gubasso/spec-driven-docs/releases/download/vX.Y.Z/spec-driven-docs-installer.sh | sh
```

### Host install

```bash
cargo install spec-driven-docs
```

`cargo binstall spec-driven-docs` fetches the prebuilt archive of the GitHub release instead of a build from source.

Linux is the only supported target. A build for another operating system is refused at compile time, and [ADR-linux-is-the-only-supported-target](./_docs/decisions/ADR-linux-is-the-only-supported-target.md) says why.

## Quick paths

- Find anything the binary carries: `sdd docs`, then `sdd docs <topic>`.
- Read the method: [method/README.md](./method/README.md), or `sdd method --list` anywhere.
- Author a comparison: [comparison-docs/README.md](./comparison-docs/README.md).
- See what a landing would do: `sdd stage`, documented in [instance/README.md](./instance/README.md#stage-the-candidate).
- Instantiate into a project: `sdd init`, the short form for a first landing.
- Upgrade an installed instance: `sdd upgrade`, the short form for a move to a newer release.
- Land and verify the result: `sdd init --apply` or `sdd upgrade`, then `sdd verify`, documented in [instance/README.md](./instance/README.md#land).
- Copy stable templates: [templates/](./templates/), or `sdd template --list`.
- Get the agent skills: `sdd init` and `sdd upgrade` land them into the instance, and `sdd skill show` reads them before one exists, documented in [instance/README.md](./instance/README.md#skills).
- Cut a release: [_docs/guides/release.md](./_docs/guides/release.md). First-time bootstrap: [_docs/guides/release-setup.md](./_docs/guides/release-setup.md).

The `sdd` CLI is the whole interface. Nothing here is tied to a particular editor or agent.

## Conventions and dependencies

The writing convention is the project's to select: this convention's chapter, available offline through `sdd method writing-style`, a document of the project's own, or none. A fresh `sdd init` routes document authors to the selection from the project's `AGENTS.md`. No delivered gate judges prose against the style. The chapter draws its word and sentence register from [SimpleEnglish](https://github.com/AminBlg/SimpleEnglish) and its information architecture from [i-have-adhd](https://github.com/ayghri/i-have-adhd), at the revisions it names.

Every adopted convention and material dependency has one owner and an outward link:

- The writing contract, which states how the style binds and how a project reaches it: [`_docs/specs/SPEC-writing-style.md`](./_docs/specs/SPEC-writing-style.md). The selection itself is stated in [`_docs/specs/SPEC-writing-policy.md`](./_docs/specs/SPEC-writing-policy.md), and the chapter it routes to by default is served by `sdd method writing-style`.
- The full catalog of dependencies and conventions: [reference/dependencies-and-conventions.md](./reference/dependencies-and-conventions.md).
- The pinned upstream, its license, and the vendored paths: [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md), also printed by `sdd license --third-party`.
- The freshness registry that detects a moved upstream: `sdd track status` reads it offline, and `sdd track check` compares the pinned revision online.

## Develop

The host supplies these tools, and this repository installs none of them:

- Each tool that [`host-requirements.toml`](./host-requirements.toml) lists, which a recipe or a hook calls. `exobrain host-check` reports each one that is missing or too old.
- `direnv`, which enters the devshell of `flake.nix`. The devshell holds every other tool.
- `bash`, `awk`, `sed`, `grep`, and the coreutils.
- `gh`, authenticated, for the release guides only.

If the host lacks one of them, the command that needs it fails. Install the tool on the host, then run the command again. [`release:the-devshell-supplies-every-project-tool`](./_docs/specs/SPEC-release.md#releasethe-devshell-supplies-every-project-tool--the-devshell-supplies-every-project-tool) states which tool the devshell holds and which the host supplies.

## License

The method is under [CC BY 4.0](./LICENSE-CC-BY-4.0) and the distribution that installs it is under the [MIT License](./LICENSE-MIT). [LICENSE](./LICENSE) states which side each file falls on, and `sdd license` prints the terms the binary carries.
