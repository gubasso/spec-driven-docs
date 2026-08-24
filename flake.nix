{
  description = "Spec-driven documentation development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = import nixpkgs { inherit system; };
      in {
        # What belongs here: a tool this project pins, and a runtime pre-commit
        # needs to build a hook environment. What does not: the host baseline.
        # A POSIX userland and git are assumed present -- git because a
        # pre-commit hook has no meaning without it, and the POSIX tools because
        # `scripts/verify.sh` already preflights them by name on every instance,
        # which is where a missing one would actually be discovered.
        devShells.default = pkgs.mkShell {
          packages = [
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
          ];
        };
        checks.shell = pkgs.runCommand "spec-driven-docs-shell" { } ''
          test -x ${pkgs.bash}/bin/bash
          touch $out
        '';
        formatter = pkgs.nixfmt-rfc-style;
      });
}
