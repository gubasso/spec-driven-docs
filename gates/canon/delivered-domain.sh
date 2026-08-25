#!/usr/bin/env sh
# The delivered gate set is one set, declared once, and reaches both deliveries.
#
# A gate leaves this repository by two routes: projected into an instance by
# `scripts/instantiate.sh`, and published to a consumer who installs this
# repository as a pre-commit repo through `.pre-commit-hooks.yaml`. Those routes
# drifted apart silently -- a gate can sit in the payload, hashed and
# executable, wired into neither -- and a gate that never runs is
# indistinguishable from a gate that passes.
#
# So `instance/gates.json` is the declaration and this check holds three things
# against it: the scripts on disk, the published manifest, and the boundary with
# `gates/canon/`. A gate here checks an invariant only this repository has -- its
# VERSION against its own tag, its dual licence, its own negative controls --
# and delivering one would gate an instance on a release process it does not run.
set -eu

decl=instance/gates.json
published=.pre-commit-hooks.yaml
[ -f "$decl" ] || {
  echo "FAIL release:the-delivered-gate-set-is-declared-once $decl: missing"
  exit 1
}
[ -f "$published" ] || {
  echo "FAIL release:the-delivered-gate-set-is-declared-once $published: missing"
  exit 1
}

work=$(mktemp -d "${TMPDIR:-/tmp}/sdd-domain.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
status=0

script_root=$(jq -r '.script_root' "$decl")
jq -r '.gates[].script' "$decl" | sort >"$work/declared-scripts"
jq -r '.gates[].id' "$decl" | sort >"$work/declared-ids"
jq -r '.support[]' "$decl" | sort >"$work/declared-support"
sort "$work/declared-scripts" "$work/declared-support" >"$work/declared-files"

# Every declared gate is a script that is there and can run. A declaration
# naming a file the payload does not hold projects a manifest entry that no
# install can satisfy.
while IFS= read -r script; do
  [ -f "$script_root/$script" ] || {
    echo "FAIL release:the-delivered-gate-set-is-declared-once $script_root/$script: declared but absent"
    status=1
    continue
  }
  [ -x "$script_root/$script" ] || {
    echo "FAIL release:the-delivered-gate-set-is-declared-once $script_root/$script: not executable"
    status=1
  }
done <"$work/declared-scripts"

# And the reverse: a script sitting in the delivered directory that nothing
# declares is the shape of the original defect -- payload with no wiring.
for file in "$script_root"/*; do
  [ -f "$file" ] || continue
  name=${file##*/}
  grep -Fxq "$name" "$work/declared-files" || {
    echo "FAIL release:the-delivered-gate-set-is-declared-once $file: present but undeclared"
    status=1
  }
done

# The declaration and the installer must name the same directory. The installer
# projects `managed_directories` from the profile and never reads this file, so a
# profile pointing somewhere else would ship a set nothing here ever checked --
# the boundary would hold over a directory no instance receives.
for profile in instance/profiles/*.json; do
  [ -f "$profile" ] || continue
  # Count and element are compared separately. Joining the array into one string
  # first makes `["gates", "instance"]` equal a root literally named
  # `gates,instance`, so a profile projecting two directories would satisfy a
  # check that only ever inspects one.
  dir_count=$(jq -r '.managed_directories | length' "$profile")
  [ "$dir_count" -eq 1 ] || {
    echo "FAIL release:a-canon-gate-is-not-delivered $profile: projects $dir_count directories, the declaration names one"
    status=1
    continue
  }
  declared_dir=$(jq -r '.managed_directories[0]' "$profile")
  [ "$declared_dir" = "$script_root" ] || {
    echo "FAIL release:a-canon-gate-is-not-delivered $profile: projects $declared_dir, declaration names $script_root"
    status=1
  }
done

# An id or a script name outside the slug shape would reach YAML as a plain
# scalar, where a colon or a quote means something other than itself.
jq -r '.gates[] | .id + " " + .script' "$decl" | while read -r gid gscript; do
  case "$gid" in *[!a-z0-9-]* | '') echo "FAIL release:the-delivered-gate-set-is-declared-once $decl: unsafe gate id: $gid" ;; esac
  case "$gscript" in *[!a-z0-9.-]* | '') echo "FAIL release:the-delivered-gate-set-is-declared-once $decl: unsafe script name: $gscript" ;; esac
done >"$work/shape"
[ ! -s "$work/shape" ] || {
  cat "$work/shape"
  status=1
}

# The published manifest is the declaration, rendered. Comparing id sets alone
# left every other field free to drift: a `files:` pattern, an `exclude:`, or an
# `always_run:` could differ between what an instance runs and what a consumer
# installs, and both would still be "the same gates". So the render is produced
# here and the file is required to equal it byte for byte, comments aside.
render="$work/rendered"
"$(dirname "$0")/../../scripts/render-gate-block.sh" --gates "$decl" \
  --docs-root '_?docs' --entry-root "$script_root" \
  --language script --style manifest >"$render" || {
  echo "FAIL release:the-delivered-gate-set-is-declared-once $decl: the declaration does not render"
  exit 1
}
sed -n '/^- id: /,$p' "$published" >"$work/published-body"
if ! diff -u "$render" "$work/published-body" >"$work/render-diff"; then
  echo "FAIL release:the-delivered-gate-set-is-declared-once $published: does not match the rendered declaration"
  sed -n '1,40p' "$work/render-diff" | sed 's/^/  /'
  status=1
fi

# The boundary, stated as its own assertion rather than inferred from the two
# above: nothing under `gates/canon/` is declared or published.
for file in gates/canon/*; do
  [ -f "$file" ] || continue
  name=${file##*/}
  if grep -Fxq "$name" "$work/declared-files"; then
    echo "FAIL release:a-canon-gate-is-not-delivered $file: a canon gate is in the delivered declaration"
    status=1
  fi
  # The entries, not the whole file: this header documents the boundary by
  # naming a gate on the canon side of it, and a check reading the comments
  # would fail on the sentence that explains it.
  grep -Fq "gates/canon/$name" "$work/published-body" && {
    echo "FAIL release:a-canon-gate-is-not-delivered $file: a canon gate is published to consumers"
    status=1
  }
done

exit "$status"
