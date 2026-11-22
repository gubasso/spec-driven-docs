# Install Sources

External sources behind the install commands of the `## Install` section of the root `README.md`. Each manager owns its own command, and this file states what each command was checked against.

Verified against the listed sources on 2026-11-22. Each entry names its URL, the finding, and a short quote where the wording matters.

## Nix flake

### Flake references

- URL: <https://nix.dev/manual/nix/2.28/command-ref/new-cli/nix3-flake>.
- Finding: the reference `github:<owner>/<repo>/<rev-or-ref>` selects a tag by name. Quote: "`<rev-or-ref>` specifies the name of a branch or tag (`ref`), or a commit hash (`rev`)."
- Finding: `inputs.<name>.follows` takes a `/`-separated path of input names. The manual shows the empty value only in its example of a circular input, `inputs.b.inputs.a.follows = "";`, with the comment "Tell the 'b' flake not to fetch 'a' again, to ensure its 'a' is this 'a'." The README uses the same form on the input `spec-driven-docs` inside the sdd input, so `flake.lock` holds no node for the previous sdd release.

### Updating one input

- URL: <https://nix.dev/manual/nix/2.28/command-ref/new-cli/nix3-flake-update>.
- Finding: the synopsis is `nix flake update [option...] inputs...`, and the example `nix flake update nixpkgs` updates only the named input in `flake.lock`.

## mise

- URL: <https://mise.jdx.dev/dev-tools/backends/cargo.html>.
- Finding: the cargo backend installs a crate at a pinned version. Quote: "pin one with `mise use cargo:eza@0.23.5`."
- Finding: the backend "downloads a prebuilt binary when cargo-binstall can find one and builds the crate with Cargo otherwise."

## devbox

- URL: <https://github.com/jetify-com/devbox/blob/58c59a28c257e66c3020b2910e2270af66001d06/examples/flakes/README.md>. The page it links, `https://www.jetify.com/docs/devbox/guides/using-flakes/`, returned HTTP 404 on 2026-11-22.
- Finding: a remote flake is a package name. Quote: "Use `github:<org>/<repo>/<ref>#<output>` as the package name to install from a Github repo."
- Finding: the command is `devbox add <pkg>...`, from `internal/boxcli/add.go` at the same commit.

## asdf

- URL: <https://github.com/asdf-vm/asdf-plugins/tree/1fc7a2c7cbf185f4147e84f1a58e1f5513dc8041/plugins>.
- Finding: the plugin index holds no entry named `sdd` or `spec-driven-docs`. No other asdf plugin for sdd exists.

## The GitHub release

- URL: <https://github.com/gubasso/spec-driven-docs/releases/tag/v0.19.1>.
- Finding: the release carries `spec-driven-docs-x86_64-unknown-linux-gnu.tar.xz` and `spec-driven-docs-installer.sh`. Its notes give the installer command `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/gubasso/spec-driven-docs/releases/download/v0.19.1/spec-driven-docs-installer.sh | sh`.

## Host install

- URL: <https://github.com/cargo-bins/cargo-binstall>.
- Finding: `cargo binstall <crate>` reads the crate from crates.io, then searches the linked repository for release artifacts, and builds with `cargo install` only as a last resort. Quote: "publish pre-built binaries to GitHub Releases on each release."
- URL: <https://crates.io/crates/spec-driven-docs>. Finding: the crate name is `spec-driven-docs`, so `cargo install spec-driven-docs` builds it from source.
