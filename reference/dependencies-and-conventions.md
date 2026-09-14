# Dependencies and conventions

The external work this project stands on, and how it stands on each one. A source is material this project reads at a resolved revision. A dependency is code or content this project carries or links. A convention is a pattern this project implements. Prior art is work this project learned from but does not depend on. The relationship word in each row is exact: `draws from`, `implements`, `integrates`, `uses`, or `evaluated`.

## Sources read

A source this project reads at a resolved revision and does not carry.

| Source                                                               | Relationship | Contribution                                                                                                                                                                                                           | Revision source                                                                                                        |
| -------------------------------------------------------------------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| SimpleEnglish                                                        | `draws from` | Word and sentence register                                                                                                                                                                                             | [`method/writing-style.md`](../method/writing-style.md#sources)                                                        |
| Cargo Book, "Registry Index" and "Registry Web API", read 2026-09-12 | `draws from` | The sparse index layout, the `dl` download template and its markers, the per-version `cksum`, and the `yanked` flag the resolver reads                                                                                 | [doc.rust-lang.org/cargo/reference/registry-index.html](https://doc.rust-lang.org/cargo/reference/registry-index.html) |
| The crates.io index for this crate, read 2026-09-12                  | `draws from` | The registry checksum of every published release, and the archive audit that sets the legacy capability floor at 0.6.6: every release from 0.2.0 to 0.6.5 is missing the seed root, and 0.6.6 onward carries all eight | [`release-compat/index.toml`](../release-compat/index.toml)                                                            |
| i-have-adhd                                                          | `draws from` | Information architecture and contrasting examples                                                                                                                                                                      | [`method/writing-style.md`](../method/writing-style.md#sources)                                                        |

The exact Rust libraries and their versions live in `Cargo.toml` and `Cargo.lock`. This catalog does not repeat them.

## Implemented conventions

A pattern this project implements. No code is vendored.

| Convention                              | Relationship | Where it applies                                       |
| --------------------------------------- | ------------ | ------------------------------------------------------ |
| ASD-STE100 Simplified Technical English | `implements` | The controlled-language source behind SimpleEnglish.   |
| Cargo registry index protocol           | `implements` | How the release resolver finds and verifies a release. |
| EARS                                    | `implements` | The requirement statement pattern in every spec.       |
| RFC 2119 and RFC 8174                   | `implements` | The normative keywords a requirement uses.             |
| Diátaxis                                | `implements` | Where each document is placed.                         |
| CommonMark and GitHub Flavored Markdown | `implements` | The markdown every document is written in.             |
| Agent Skills format                     | `implements` | The portable shape of every skill.                     |
| pre-commit                              | `integrates` | The runner an instance wires the delivered gates into. |

## Runtime requirements

What a consumer's host must carry.

| Requirement | Relationship | Why                                                                                   |
| ----------- | ------------ | ------------------------------------------------------------------------------------- |
| git         | `uses`       | The host prerequisite for a pre-commit hook, and the transport for `sdd track check`. |
| pre-commit  | `uses`       | Runs the delivered gates, including in CI.                                            |

No instance runtime needs Python, Node.js, or a network.

## Repository delivery tooling

Tooling this repository uses to cut a release. A consumer receives none of it.

| Tool                        | Relationship | Version source                               |
| --------------------------- | ------------ | -------------------------------------------- |
| release-kit                 | `uses`       | `.release-kit/manifest.json` and `flake.nix` |
| Scoped Conventional Commits | `implements` | `.pre-commit-config.yaml`                    |
| release-plz                 | `uses`       | `.release-kit/manifest.json`                 |
| cargo-dist                  | `uses`       | `dist-workspace.toml`                        |
| Trusted publishing          | `uses`       | `.github/workflows/release-plz.yml`          |

Hook integrations and their revisions live in `.pre-commit-config.yaml`.

## Evaluated prior art

Work this project read and learned from. It carries no code from any of these.

| Prior art                       | Relationship | Record                                           |
| ------------------------------- | ------------ | ------------------------------------------------ |
| Spec-driven development writing | `evaluated`  | `reference/prior-art/spec-driven-development.md` |
