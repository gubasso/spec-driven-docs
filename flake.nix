{
  description = "Spec-driven documentation development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    release-kit = {
      # Moved by `rk devshell sync`, invoked from .envrc: the tag in this URL
      # is the version, flake.lock is the content pin, and nothing else in
      # this repository names an rk version.
      url = "github:gubasso/release-kit/v0.2.18";
      # A deliberate deal, not a tidy-up: with follows, rk rebuilds against
      # this repository's nixpkgs rather than the revision it tested
      # upstream, so this consumer owns that compatibility — proven by the
      # devshell build `rk devshell sync` runs and CI's nix develop. The
      # alternative is a second nixpkgs node in the lock.
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      release-kit,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rk = release-kit.packages.${system}.default;
      in
      {
        # What belongs here: a tool this project pins, a runtime pre-commit
        # needs to build a hook environment, and a command this project's own
        # documents tell a reader to run. What does not: the host baseline.
        # git is assumed present, because a pre-commit hook has no meaning
        # without it; the Rust toolchain comes from rust-toolchain.toml
        # through the overlay so CI and local development share one compiler.
        # curl is named because the source check in comparison-docs/SOURCES.md
        # calls it. rk comes from the release-kit flake input, pinned by its
        # tag, and `rk devshell sync` moves that pin from .envrc.
        devShells.default = pkgs.mkShell {
          packages = [
            toolchain
            rk
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
          ];
        };
        # The binary this repository publishes, built under Nix from the
        # committed lock. nix/package.nix is release-kit's seed, tuned here;
        # the rustPlatform argument is the override that matters, so the
        # package and the devshell compile with the one pinned toolchain
        # rather than whatever nixpkgs carries.
        packages.default = pkgs.callPackage ./nix/package.nix {
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
        };
        checks.package = self.packages.${system}.default;
        # The built binary answers at all. nix flake check builds only the
        # checks output, so the package is named there as well.
        checks.smoke = pkgs.runCommand "spec-driven-docs-smoke" { } ''
          ${pkgs.lib.getExe self.packages.${system}.default} --version
          touch $out
        '';
        checks.shell = pkgs.runCommand "spec-driven-docs-shell" { } ''
          test -x ${pkgs.bash}/bin/bash
          touch $out
        '';
        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
