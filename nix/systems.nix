# Every system this flake exposes an output for, as a flat list.
#
# The support boundary has an evaluated half
# (release:the-binary-builds-for-every-declared-target), and this is what CI
# evaluates. `nix flake show --json` reports the same facts, but its schema has
# changed between Nix versions — a `version` number beside the output families
# on one build, a nested tree on another — so a check written against that
# report breaks on a Nix upgrade rather than on a real change. Reading the
# flake's own outputs asks the flake directly.
#
# Only the families whose first attribute is a system are read. An output like
# `overlays` or `nixosConfigurations` is keyed by a name rather than a system,
# so including it would report names that are not systems.
#
# Call it with the flake reference to inspect:
#
#   nix eval --impure --json --expr 'import ./nix/systems.nix "."'
reference:
let
  flake = builtins.getFlake (toString reference);
  families = [
    "packages"
    "devShells"
    "checks"
    "formatter"
    "apps"
    "legacyPackages"
    "devShell"
    "defaultPackage"
  ];
  present = builtins.filter (name: builtins.hasAttr name flake) families;
  keysOf = name: builtins.attrNames (builtins.getAttr name flake);
in
builtins.concatLists (map keysOf present)
