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
      # Bumped by scripts/rk-bump.sh: the tag in this URL is the version,
      # flake.lock is the content pin, and nothing else in this repository
      # names an rk version.
      url = "github:gubasso/release-kit/v0.2.13";
      # A deliberate deal, not a tidy-up: with follows, rk rebuilds against
      # this repository's nixpkgs rather than the revision it tested
      # upstream, so this consumer owns that compatibility — proven by the
      # devshell build scripts/rk-bump.sh runs and CI's nix develop. The
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
        # scripts call. What does not: the host baseline. git is assumed
        # present, because a pre-commit hook has no meaning without it; the
        # Rust toolchain comes from rust-toolchain.toml through the overlay so
        # CI and local development share one compiler. curl and flock are
        # named because scripts/rk-autobump.sh and scripts/rk-bump.sh call
        # them and macOS ships neither an flock(1) nor a guarantee about the
        # rest. rk comes from the release-kit flake input, pinned by its tag.
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
            pkgs.flock
            pkgs.bats
          ];
        };
        checks.shell = pkgs.runCommand "spec-driven-docs-shell" { } ''
          test -x ${pkgs.bash}/bin/bash
          touch $out
        '';
        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
