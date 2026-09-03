{
  description = "Spec-driven documentation development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        release-kit = pkgs.callPackage ./nix/rk.nix { };
      in {
        # What belongs here: a tool this project pins, a runtime pre-commit
        # needs to build a hook environment, and a command this project's own
        # scripts call. What does not: the host baseline. git is assumed
        # present, because a pre-commit hook has no meaning without it; the
        # Rust toolchain comes from rust-toolchain.toml through the overlay so
        # CI and local development share one compiler. curl and flock are
        # named because scripts/rk-autobump.sh calls them and macOS ships
        # neither an flock(1) nor a guarantee about the rest.
        devShells.default = pkgs.mkShell {
          packages = [
            toolchain
            release-kit
            pkgs.cargo-nextest
            pkgs.cargo-deny
            pkgs.just
            pkgs.pre-commit
            pkgs.dprint
            pkgs.editorconfig-checker
            pkgs.nodejs
            pkgs.ripgrep
            pkgs.python3Packages.md-toc
            pkgs.typos
            pkgs.committed
            pkgs.markdownlint-cli2
            pkgs.lychee
            pkgs.ripsecrets
            pkgs.shellcheck
            pkgs.shfmt
            pkgs.jq
            pkgs.check-jsonschema
            pkgs.curl
            pkgs.flock
            pkgs.bats
          ];
        };
        # Named so `nix-update --flake` has an attribute to resolve when
        # `just rk-bump` rewrites the pin.
        packages.release-kit = release-kit;
        checks.shell = pkgs.runCommand "spec-driven-docs-shell" { } ''
          test -x ${pkgs.bash}/bin/bash
          touch $out
        '';
        formatter = pkgs.nixfmt-rfc-style;
      });
}
