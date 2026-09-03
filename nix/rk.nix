{ lib, rustPlatform, fetchCrate }:

# release-kit is not in nixpkgs. It drives this project's release convention,
# so the devshell carries it and every machine gets the same rk. The source is
# the published crate rather than the git tag: the repository's integration
# tests need git and a reachable forge, which the Nix sandbox has neither of,
# while the crate excludes /tests and builds clean. Both sources produce the
# same cargoHash. `just rk-bump` moves this pin; cargoHash is discoverable
# only by building, so no field here is edited by hand.
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "release-kit";
  version = "0.2.9";

  src = fetchCrate {
    inherit (finalAttrs) pname version;
    hash = "sha256-x24OQqiZnGYSDeiUleZowShQEZPcxuF0sLpYc1Si4CY=";
  };

  cargoHash = "sha256-RoYFg/tLzAsAKn7pTZBpOmGMYt8ozHvCprvqSsb2HAU=";

  meta = {
    description = "A canonical release workflow and the rk CLI that serves it";
    homepage = "https://github.com/gubasso/release-kit";
    license = [ lib.licenses.mit lib.licenses.cc-by-40 ];
    mainProgram = "rk";
  };
})
