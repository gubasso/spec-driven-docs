#!/usr/bin/env sh
# Render the delivered gate set as pre-commit hook entries.
#
# One renderer serves both deliveries. `instantiate.sh` calls it to write the
# managed block into an instance, and `gates/canon/delivered-domain.sh` calls it
# to reproduce what `.pre-commit-hooks.yaml` publishes to a consumer who
# installs this repository as a pre-commit repo. Rendering both from
# `instance/gates.json` is what keeps a gate from reaching one delivery and
# missing the other.
#
# `{docs_root}` is substituted rather than left to the consumer: an instance
# knows its own root from the manifest, so its block names that root exactly
# instead of a pattern that would also match a root it does not use.
set -eu

usage() {
  echo 'usage: render-gate-block.sh --gates <gates.json> --docs-root <root> --entry-root <dir> [--language system|script] [--indent <spaces>]' >&2
  exit 2
}
gates=
docs_root=
entry_root=
language=system
indent='  '
while [ "$#" -gt 0 ]; do
  case "$1" in
    --gates)
      [ "$#" -ge 2 ] || usage
      gates=$2
      shift 2
      ;;
    --docs-root)
      [ "$#" -ge 2 ] || usage
      docs_root=$2
      shift 2
      ;;
    --entry-root)
      [ "$#" -ge 2 ] || usage
      entry_root=$2
      shift 2
      ;;
    --language)
      [ "$#" -ge 2 ] || usage
      language=$2
      shift 2
      ;;
    --indent)
      [ "$#" -ge 2 ] || usage
      indent=$2
      shift 2
      ;;
    *) usage ;;
  esac
done
[ -n "$gates" ] && [ -n "$docs_root" ] && [ -n "$entry_root" ] || usage
[ -f "$gates" ] || {
  echo "FAIL unreadable gate declaration: $gates" >&2
  exit 1
}

# The set is asserted non-empty before a line is written. A gate declaration that
# parsed but yielded nothing would render an empty `hooks:` sequence, which
# pre-commit accepts and which runs no gate at all -- the exact failure this
# whole boundary exists to make impossible.
count=$(jq -r '.gates | length' "$gates")
[ "$count" -gt 0 ] 2>/dev/null || {
  echo "FAIL gate declaration names no gate: $gates" >&2
  exit 1
}

jq -r --arg root "$docs_root" --arg entry "$entry_root" --arg lang "$language" --arg i "$indent" '
  def sub_root: gsub("\\{docs_root\\}"; $root);
  .gates[]
  | ($i + "    - id: " + .id),
    ($i + "      name: " + .name),
    ($i + "      entry: " + $entry + "/" + .script),
    ($i + "      language: " + $lang),
    (if .files then ($i + "      files: " + "'"'"'" + (.files | sub_root) + "'"'"'") else empty end),
    (if .types then ($i + "      types: [" + (.types | join(", ")) + "]") else empty end),
    (if .exclude then ($i + "      exclude: " + "'"'"'" + (.exclude | sub_root) + "'"'"'") else empty end),
    (if .always_run then ($i + "      always_run: true") else empty end),
    (if .always_run then ($i + "      pass_filenames: false") else empty end)
' "$gates"
