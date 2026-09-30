# The Markdown linter the devshell and the check phase run: nixpkgs'
# markdownlint-cli2 with the relative-links custom rule bundled beside it.
#
# The rule and its runtime dependencies are unpacked into the linter's own
# node_modules, where Node resolves the bare `markdownlint-rule-relative-links`
# a configuration names in `customRules`. So the executable keeps its name,
# nothing sets NODE_PATH, and no caller installs anything or passes a flag.
# The linter revision stays nixpkgs'. Each tarball is fetched by its npm
# integrity hash, so the build needs no network beyond the fixed outputs.
#
# `ruleVersion` must equal RELATIVE_LINKS_VERSION in
# src/domain/markdownlint.rs, which pins the same rule in the rendered
# pre-commit hook. A canon test holds the two equal. The dependency pins are
# the resolved closure of that rule version.
{ markdownlint-cli2, fetchurl }:

let
  ruleVersion = "5.1.3";
  tarball =
    name: version: hash:
    fetchurl {
      url = "https://registry.npmjs.org/${name}/-/${baseNameOf name}-${version}.tgz";
      inherit hash;
    };
  rule =
    tarball "markdownlint-rule-relative-links" ruleVersion
      "sha512-1ZRV3OyApUoYm4YVdLNZIN6vUVLzacD74n+yKx9ZL4H5yTmiLP2nKTwQMfOla158gblVL0jbB7QLLh3nmKUqvA==";
  dependencies = {
    "markdown-it" =
      tarball "markdown-it" "15.0.1"
        "sha512-9/7gE95FNPkfUWrjJIoHZza2iLmuJlPD0UNMxPi7bxUrbCR525YZY0r+zyfes0dZI5ZZ/uNIXUJca0pJvtw41g==";
    "mime" =
      tarball "mime" "4.1.0"
        "sha512-X5ju04+cAzsojXKes0B/S4tcYtFAJ6tTMuSPBEn9CPGlrWr8Fiw7qYeLT0XyH80HSoAoqWCaz+MWKh22P7G1cw==";
    "argparse" =
      tarball "argparse" "3.0.2"
        "sha512-mFdDM6WqWKraGLsVb+C9CahPnzTXOefAOLq3jYcca2YZ8bEWpr++Tzj+zSaKW9+X9L5uSxcm1AZ3Y6aZJ09OhQ==";
    "entities" =
      tarball "entities" "8.1.0"
        "sha512-kxL7msIffSuh9aaFAMD7rxAIuTRMAHMeBtgHW2yUdWw732ZNh4MehkF2gdjvtdmikkaIP9bFDDJOPlsvm7avrA==";
    "linkify-it" =
      tarball "linkify-it" "6.1.0"
        "sha512-wJ/TwpSDTLepCrQoYWYIExIKg5Zchex2Nn5yk2mFnB+6PtdkHtyLx742md9csRjjOnGkKIS/RrbY7l8D6gT9Vw==";
    "mdurl" =
      tarball "mdurl" "2.1.0"
        "sha512-1+HBaOx0zi/dQWht8rNv9MYf9qqpqL/kxI0hXImU6Y547zM6Sni8BQibt7ifgMcYtQg41ao3Ivd6cnSM86inpg==";
    "punycode.js" =
      tarball "punycode.js" "2.3.1"
        "sha512-uxFIHU0YlHYhDQtV4R9J6a52SLx28BCjT+4ieh7IGbgwVJWO+km431c4yRlREUAsAmt/uMjQUyQHNEPf0M39CA==";
    "uc.micro" =
      tarball "uc.micro" "3.0.0"
        "sha512-U3PppEkleoTnIfi8BozMx3yju3qc/L6SwqWo2Sw+54PX+PX0q9I+r1Um5HCmqD7n9VDX5/v3vQH/AjA6deDdtw==";
  };
  unpack = destination: source: ''
    mkdir -p "${destination}"
    tar -xzf ${source} -C "${destination}" --strip-components=1
  '';
in
markdownlint-cli2.overrideAttrs (old: {
  postInstall = (old.postInstall or "") + ''
    rule=$out/lib/node_modules/markdownlint-cli2/node_modules/markdownlint-rule-relative-links
    ${unpack "$rule" rule}
    ${builtins.concatStringsSep "" (
      builtins.attrValues (
        builtins.mapAttrs (name: source: unpack "$rule/node_modules/${name}" source) dependencies
      )
    )}
  '';
})
