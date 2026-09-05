# Seeded by release-kit: a starting point this project owns and tunes;
# release-kit reports drift here and never rewrites it.
#
# Supported shape: one crate with a [package] table — an implicit
# src/main.rs binary or an explicit [[bin]] entry — building with the
# committed Cargo.lock. A workspace root fails by name below rather than
# throwing on a missing attribute: point the importTOML call at the member
# crate's Cargo.toml and set mainProgram yourself.
{ lib, rustPlatform }:

let
  cargoToml = lib.importTOML ../Cargo.toml;
  package =
    cargoToml.package
      or (throw "nix/package.nix: Cargo.toml has no [package] table; this seed does not support a workspace root");
in
rustPlatform.buildRustPackage {
  pname = package.name;
  version = package.version;

  # lib.cleanSource drops the version-control and editor noise and nothing
  # else, so the build directories are named here. They must not reach the
  # store: each one changes on every local build and would rebuild the
  # package for no source change. Everything else is source, because build.rs
  # embeds the payload — method/, templates/, skills/, instance/snippets/,
  # and .markdownlint/ — at compile time.
  src = lib.cleanSourceWith {
    src = lib.cleanSource ../.;
    filter =
      path: _type:
      let
        name = baseNameOf path;
      in
      !builtins.elem name [
        "target"
        "result"
        ".direnv"
        ".jj"
      ];
  };
  cargoLock.lockFile = ../Cargo.lock;

  meta = {
    # The first [[bin]] name where one is declared, else the package
    # name — the implicit src/main.rs binary. nix run resolves the
    # binary through this attribute.
    mainProgram = if cargoToml ? bin then (lib.head cargoToml.bin).name else package.name;
    # Cargo.toml declares `MIT AND CC-BY-4.0`, the split LICENSE states:
    # MIT for the payload, CC BY 4.0 for the method. A Nix license list is
    # read as a conjunction, so it carries the same expression.
    license = [
      lib.licenses.mit
      lib.licenses.cc-by-40
    ];
  }
  // lib.optionalAttrs (package ? description) { inherit (package) description; }
  // lib.optionalAttrs (package ? homepage) { inherit (package) homepage; };
}
