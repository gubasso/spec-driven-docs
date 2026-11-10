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
# Call it with the absolute path of the checkout to inspect:
#
#   nix eval --impure --json --expr "import ./nix/systems.nix \"$PWD\""
#
# The path is read as a `git+file` reference. A bare path makes getFlake fetch
# the folder as a `path:` flake, which copies every file into the store, the
# ignored ones included: each evaluation of a checkout with a built `target/`
# added gigabytes. The `git+file` form copies the tracked tree, as
# `nix flake check` does.
#
# `shallow=1` lets the reference read a shallow clone, which is what CI's
# checkout makes. Without it, the Nix that CI installs refuses the clone,
# because a shallow repository has no `revCount`. A full clone reads the same
# tree either way.
reference:
let
  flake = builtins.getFlake ("git+file://" + toString reference + "?shallow=1");
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
