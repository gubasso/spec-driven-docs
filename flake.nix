{
  description = "Spec-driven documentation development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    release-kit = {
      # The tag in this URL is the version, flake.lock is the content pin,
      # and nothing else in this repository names an rk version. The
      # `exobrain outdated` line of .envrc warns about a newer release, and
      # the operator moves the pin with `flake-edit update release-kit`.
      url = "github:gubasso/release-kit/v0.10.3";
      inputs = {
        # A deliberate deal, not a tidy-up: with follows, rk rebuilds against
        # this repository's nixpkgs rather than the revision it tested
        # upstream, so this consumer owns that compatibility — proven by the
        # devshell build that a pin move runs and CI's nix develop. The
        # alternative is a second nixpkgs node in the lock.
        nixpkgs.follows = "nixpkgs";
        # release-kit reads spec-driven-docs for its own development; here
        # that is this flake, so the lock carries no nested chain of either.
        spec-driven-docs.follows = "";
        # rk pins an earlier rk release for its own development. The empty
        # follows points that input at this flake, so flake.lock carries no
        # node for that release, and nothing here builds it.
        release-kit.follows = "";
        # One node of each shared input.
        rust-overlay.follows = "rust-overlay";
      };
    };
    # The previous release of this tool. It lands, verifies, and upgrades
    # this repository's own instance, and the rendered hooks run it, as in
    # every consumer; the sdd built from the tree runs only under test
    # (release:the-canon-runs-its-previous-release). The `exobrain outdated`
    # line of .envrc warns about a newer release. The operator moves the pin
    # with `flake-edit update spec-driven-docs` after each release, then runs
    # `sdd upgrade` here. The follows keep one node of each shared input.
    # The pinned release brings its own previous-release input, and the
    # empty follows points it at this flake, so flake.lock holds one sdd
    # node rather than a chain of older ones.
    spec-driven-docs = {
      url = "github:gubasso/spec-driven-docs/v0.19.1";
      inputs = {
        spec-driven-docs.follows = "";
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
        release-kit.follows = "release-kit";
      };
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      release-kit,
      spec-driven-docs,
    }:
    let
      # One system, declared here rather than mapped over a default set.
      # x86_64 Linux is the only target this project releases and the only
      # one its required check compiles, so a flake that offered more would
      # advertise support nothing proves
      # (ADR-linux-is-the-only-supported-target).
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };
      toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      rk = release-kit.packages.${system}.default;
      # The Markdown linter with the relative-links rule bundled beside it,
      # so the devshell and the package's check phase run one derivation.
      markdownlint-cli2 = pkgs.callPackage ./nix/markdownlint.nix { };
    in
    {
      # What belongs here: a tool this project pins that no more specific
      # owner declares, a runtime the hook runner needs to build a hook
      # environment, and a command this project's own documents tell a reader
      # to run. A hook tool that the hook runner can build (typos, committed,
      # md-toc, shellcheck, shfmt, ...) is pinned in .pre-commit-config.yaml,
      # and a language library never enters this shell. What does not: the
      # host baseline.
      # git is assumed present, because a git hook has no meaning
      # without it; the Rust toolchain comes from rust-toolchain.toml
      # through the overlay so CI and local development share one compiler.
      # curl is named because the source check in comparison-docs/SOURCES.md
      # calls it. rk comes from the release-kit flake input, pinned by its
      # tag. The `exobrain outdated` line of .envrc warns about a newer
      # release, and the operator moves the pin with
      # `flake-edit update release-kit`.
      devShells.${system}.default = pkgs.mkShell {
        packages = [
          toolchain
          rk
          # The pinned previous release; see the input above.
          spec-driven-docs.packages.${system}.default
          pkgs.cargo-nextest
          pkgs.cargo-deny
          # The dist-plan and dist-build jobs of ci.yml run the dist that
          # dist-workspace.toml pins (cargo-dist-version).
          pkgs.cargo-dist
          pkgs.just
          pkgs.dprint
          # The markdownlint-cli2 hook builds on this nodejs
          # (default_language_version in .pre-commit-config.yaml).
          pkgs.nodejs
          pkgs.ripgrep
          # The cargo tests run markdownlint-cli2 and check-jsonschema from PATH.
          markdownlint-cli2
          pkgs.lychee
          pkgs.jq
          pkgs.check-jsonschema
          pkgs.curl
          # The nixfmt and statix hooks run these. nixfmt is the formatter
          # below, so the hook formats as `nix fmt` does.
          pkgs.nixfmt
          pkgs.statix
        ];
      };
      # The binary this repository publishes, built under Nix from the
      # committed lock. nix/package.nix is release-kit's seed, tuned here;
      # the rustPlatform argument is the override that matters, so the
      # package and the devshell compile with the one pinned toolchain
      # rather than whatever nixpkgs carries.
      packages.${system}.default = pkgs.callPackage ./nix/package.nix {
        inherit markdownlint-cli2;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };
      };
      checks.${system} = {
        package = self.packages.${system}.default;
        # The built binary answers at all. nix flake check builds only the
        # checks output, so the package is named there as well.
        smoke = pkgs.runCommand "spec-driven-docs-smoke" { } ''
          ${pkgs.lib.getExe self.packages.${system}.default} --version
          touch $out
        '';
        shell = pkgs.runCommand "spec-driven-docs-shell" { } ''
          test -x ${pkgs.bash}/bin/bash
          touch $out
        '';
      };
      formatter.${system} = pkgs.nixfmt;
    };
}
