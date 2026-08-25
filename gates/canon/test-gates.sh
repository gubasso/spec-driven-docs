#!/usr/bin/env sh
# Every gate this payload ships has a passing control and a failing one.
#
# A rejection is asserted on two things: a non-zero status and the text the gate
# is supposed to print. Status alone counts a syntax error, a missing
# interpreter, or an unrelated `set -e` abort as a rejection, so a gate can be
# neutered without any control noticing -- which is the defect that then ships
# to every instance.
set -eu
# The two gate domains are addressed separately. `hooks` is the delivered set an
# instance receives; `canon_gates` is the set that stays here, checking invariants
# only this repository has. A control that reached for the wrong one would assert
# against a gate the other side never runs.
repo=$(CDPATH='' cd -- "$(dirname "$0")/../.." && pwd -P)
hooks="$repo/gates/instance"
canon_gates="$repo/gates/canon"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/sdd-gates.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
passed=0

accept() {
  name=$1
  shift
  "$@" >/dev/null 2>&1 || {
    echo "FAIL accept: $name"
    exit 1
  }
  passed=$((passed + 1))
}
# For a third-party tool whose failure text this repository does not own.
reject() {
  name=$1
  shift
  if "$@" >/dev/null 2>&1; then
    echo "FAIL reject: $name"
    exit 1
  fi
  passed=$((passed + 1))
}
# For a gate shipped here: the status and the message both have to be right.
reject_msg() {
  name=$1 expected=$2
  shift 2
  if out=$("$@" 2>/dev/null); then
    echo "FAIL reject: $name exited zero"
    exit 1
  fi
  case "$out" in
    *"$expected"*) ;;
    *)
      echo "FAIL reject: $name did not report '$expected'"
      echo "$out"
      exit 1
      ;;
  esac
  passed=$((passed + 1))
}
run_at() {
  dir=$1
  shift
  (cd "$dir" && "$@")
}

accept 'adr filename' "$hooks/adr-filename-shape.sh" _docs/decisions/ADR-use-slugs.md
reject_msg 'adr filename digit' 'decision-records:filename-carries-no-digit' \
  "$hooks/adr-filename-shape.sh" _docs/decisions/ADR-use-v2.md
accept 'ki filename' "$hooks/ki-filename-shape.sh" _docs/reference/known-issues/KI-vendor-500.md
reject_msg 'ki filename counter' 'known-issues:case-id-is-a-slug' \
  "$hooks/ki-filename-shape.sh" _docs/reference/known-issues/KI-001-vendor.md

base="$tmp/base"
mkdir -p "$base/_docs/decisions" "$base/_docs/specs" "$base/_docs/reference/known-issues" "$base/.spec-driven-docs"
printf '# Choice\n\n## Context and Problem Statement\n\nContext.\n\n## Considered Options\n\n- x — chosen.\n\n## Decision Outcome\n\nChosen.\n\n## Consequences\n\n- Good: small\n\n## Status\n\nAccepted\n' >"$base/_docs/decisions/ADR-choice.md"
accept 'adr word cap' run_at "$base" "$hooks/adr-word-cap.sh"
yes word | head -351 | tr '\n' ' ' >>"$base/_docs/decisions/ADR-choice.md"
reject_msg 'adr word cap over' 'decision-records:body-stays-within-350-words' \
  run_at "$base" "$hooks/adr-word-cap.sh"
sed -i '/^word word/d' "$base/_docs/decisions/ADR-choice.md"

printf '# AGENTS\n' >"$base/AGENTS.md"
accept 'agent digest size' run_at "$base" "$hooks/agents-digest-size.sh"
yes line | head -101 >"$base/AGENTS.md"
reject_msg 'agent digest over' 'docs-format:author-instructions-stay-within-budget' \
  run_at "$base" "$hooks/agents-digest-size.sh"
mkdir -p "$base/node_modules/pkg"
yes line | head -400 >"$base/node_modules/pkg/AGENTS.md"
printf '# AGENTS\n' >"$base/AGENTS.md"
accept 'agent digest ignores vendored' run_at "$base" "$hooks/agents-digest-size.sh"

printf '# Chapter\n' >"$base/00-chapter.md"
yes line | head -400 >"$base/node_modules/pkg/README.md"
accept 'chapter cap ignores vendored' run_at "$base" "$hooks/chapter-size-cap.sh"
yes line | head -201 >"$base/00-chapter.md"
reject_msg 'chapter cap over' 'docs-format:chapter-stays-within-200-lines' \
  run_at "$base" "$hooks/chapter-size-cap.sh"
printf '00-chapter.md\n' >"$base/.spec-driven-docs/chapter-size-debt.txt"
accept 'chapter debt active' run_at "$base" "$hooks/chapter-size-cap.sh"
printf '# fits\n' >"$base/00-chapter.md"
reject_msg 'chapter debt now fits' 'now fits' run_at "$base" "$hooks/chapter-size-cap.sh"
printf '00-chapter.md' >"$base/.spec-driven-docs/chapter-size-debt.txt"
reject_msg 'chapter debt unterminated now fits' 'now fits' run_at "$base" "$hooks/chapter-size-cap.sh"
printf 'missing.md\n' >"$base/.spec-driven-docs/chapter-size-debt.txt"
reject_msg 'chapter debt deleted' 'deleted' run_at "$base" "$hooks/chapter-size-cap.sh"
printf 'missing.md' >"$base/.spec-driven-docs/chapter-size-debt.txt"
reject_msg 'chapter debt unterminated deleted' 'deleted' run_at "$base" "$hooks/chapter-size-cap.sh"
rm -rf "$base/node_modules" "$base/.spec-driven-docs/chapter-size-debt.txt" "$base/00-chapter.md"

cmp="$tmp/comparison.md"
printf 'Legend: ✅ yes.\n\nVerified: 2026-08-24 — Subject 1.0.\n\n| [Case](#case) | Subject |\n| --- | --- |\n| Runs | ✅ yes |\n' >"$cmp"
accept 'comparison dated' "$hooks/comparison-dated-tables.sh" "$cmp"
sed -i '/^Verified:/d' "$cmp"
reject_msg 'comparison undated' 'comparison-docs:every-table-is-dated' \
  "$hooks/comparison-dated-tables.sh" "$cmp"
sed -i '1a Verified: 2026-08-24 — Subject 1.0.' "$cmp"
accept 'comparison legend' "$hooks/comparison-legend.sh" "$cmp"
sed -i '/^Legend:/d' "$cmp"
reject_msg 'comparison missing legend' 'comparison-docs:a-comparison-carries-a-legend' \
  "$hooks/comparison-legend.sh" "$cmp"
sed -i '1i Legend: ✅ yes.' "$cmp"
accept 'comparison verdict word' "$hooks/comparison-verdict-word.sh" "$cmp"
sed -i 's/✅ yes/✅/' "$cmp"
reject_msg 'comparison bare verdict' 'comparison-docs:a-verdict-carries-its-word' \
  "$hooks/comparison-verdict-word.sh" "$cmp"
sed -i 's/| Runs | ✅ |/| Runs | ✅ yes | ❌ |/' "$cmp"
reject_msg 'comparison bare verdict beside an annotated one' 'comparison-docs:a-verdict-carries-its-word' \
  "$hooks/comparison-verdict-word.sh" "$cmp"
sed -i 's/| Runs | ✅ yes | ❌ |/| Runs | ✅ yes |/' "$cmp"
accept 'comparison one reference' "$hooks/comparison-one-reference-per-cell.sh" "$cmp"
sed -i 's@Runs@Runs [a](#a) [b](#b)@' "$cmp"
reject_msg 'comparison two references' 'comparison-docs:a-cell-carries-one-reference' \
  "$hooks/comparison-one-reference-per-cell.sh" "$cmp"
# shellcheck disable=SC2016
printf '| Pipe | `a\\|b` |\n' >>"$cmp"
accept 'comparison escaped pipe' "$hooks/comparison-escaped-pipes.sh" "$cmp"
sed -i 's/a\\|b/a|b/' "$cmp"
reject_msg 'comparison unescaped pipe' 'comparison-docs:table-pipes-are-escaped' \
  "$hooks/comparison-escaped-pipes.sh" "$cmp"

ki="$base/_docs/reference/known-issues/KI-vendor.md"
printf '%s\n' '---' 'upstream: https://example.invalid/issues' 'retire_when: release >= 2.0' '---' '# Vendor issue' '## How it works' 'Run.' >"$ki"
accept 'ki retire' run_at "$base" "$hooks/ki-retire-when.sh"
sed -i 's/retire_when:.*/retire_when:/' "$ki"
reject_msg 'ki retire missing' 'known-issues:a-record-carries-its-retirement-condition' \
  run_at "$base" "$hooks/ki-retire-when.sh"
sed -i 's/retire_when:/retire_when: release >= 2.0/' "$ki"
accept 'ki mechanism' run_at "$base" "$hooks/ki-mechanism-walkthrough.sh"
sed -i 's/## How it works/## Mechanism/' "$ki"
reject_msg 'ki mechanism missing' 'known-issues:a-record-walks-the-mechanism' \
  run_at "$base" "$hooks/ki-mechanism-walkthrough.sh"
sed -i 's/## Mechanism/## How it works/' "$ki"
accept 'ki report optional' run_at "$base" "$hooks/ki-report-body.sh"
sed -i 's#issues$#issues/123#' "$ki"
reject_msg 'ki report missing' 'known-issues:a-filed-record-carries-its-report' \
  run_at "$base" "$hooks/ki-report-body.sh"
sed -i 's#issues/123#issues/123 (open)#' "$ki"
reject_msg 'ki report missing with trailing text' 'known-issues:a-filed-record-carries-its-report' \
  run_at "$base" "$hooks/ki-report-body.sh"
sed -i 's#issues/123 (open)#issues/123\#c4#' "$ki"
reject_msg 'ki report missing with a comment anchor' 'known-issues:a-filed-record-carries-its-report' \
  run_at "$base" "$hooks/ki-report-body.sh"
sed -i 's#issues/123\#c4#issues/123#' "$ki"
printf '%s\n' '## Report' '```text' 'body' '```' >>"$ki"
accept 'ki report present' run_at "$base" "$hooks/ki-report-body.sh"
sed -i 's#example.invalid/issues/123#bugzilla.example/show_bug.cgi?id=123#' "$ki"
accept 'bugzilla width' run_at "$base" "$hooks/ki-bugzilla-report-width.sh"
yes x | head -100 | tr -d '\n' >>"$ki"
printf '\n' >>"$ki"
reject_msg 'bugzilla width over' 'known-issues:a-bugzilla-report-body-fits-in-79-columns' \
  run_at "$base" "$hooks/ki-bugzilla-report-width.sh"
sed -i 's#bugzilla.example/show_bug.cgi?id=123#bugs.kde.org/show_bug.cgi?id=123#' "$ki"
reject_msg 'bugzilla width over on a rebranded tracker' 'known-issues:a-bugzilla-report-body-fits-in-79-columns' \
  run_at "$base" "$hooks/ki-bugzilla-report-width.sh"
sed -i 's/^upstream:/  Upstream:/' "$ki"
reject_msg 'bugzilla width over with an indented capitalised key' 'known-issues:a-bugzilla-report-body-fits-in-79-columns' \
  run_at "$base" "$hooks/ki-bugzilla-report-width.sh"
sed -i 's/^  Upstream:/upstream:/' "$ki"

# The same record under a codebase instance: the gates follow the manifest's
# documentation root rather than assuming one, or they pass over nothing.
code="$tmp/codebase"
mkdir -p "$code/docs/reference/known-issues" "$code/.spec-driven-docs"
printf '{\n  "docs_root": "docs"\n}\n' >"$code/.spec-driven-docs/manifest.json"
cp "$ki" "$code/docs/reference/known-issues/KI-vendor.md"
reject_msg 'codebase profile bugzilla width' 'known-issues:a-bugzilla-report-body-fits-in-79-columns' \
  run_at "$code" "$hooks/ki-bugzilla-report-width.sh"
sed -i 's/retire_when:.*/retire_when:/' "$code/docs/reference/known-issues/KI-vendor.md"
reject_msg 'codebase profile ki retire' 'known-issues:a-record-carries-its-retirement-condition' \
  run_at "$code" "$hooks/ki-retire-when.sh"
sed -i 's/## How it works/## Mechanism/' "$code/docs/reference/known-issues/KI-vendor.md"
reject_msg 'codebase profile ki mechanism' 'known-issues:a-record-walks-the-mechanism' \
  run_at "$code" "$hooks/ki-mechanism-walkthrough.sh"
sed -i '/^## Report$/d' "$code/docs/reference/known-issues/KI-vendor.md"
reject_msg 'codebase profile ki report' 'known-issues:a-filed-record-carries-its-report' \
  run_at "$code" "$hooks/ki-report-body.sh"

# A consumer who installed the published hooks without the payload has no
# manifest. The root is discovered, or these gates pass over records they never
# looked at.
bare="$tmp/bare-codebase"
mkdir -p "$bare/docs/reference/known-issues" "$bare/docs/specs"
printf '%s\n' '---' 'upstream: https://bugs.kde.org/show_bug.cgi?id=123' 'retire_when:' '---' \
  '# Missing everything' '## Mechanism' 'Run.' >"$bare/docs/reference/known-issues/KI-a.md"
{
  printf '%s\n' '---' 'upstream: https://bugs.kde.org/show_bug.cgi?id=124' 'retire_when: release >= 2.0' '---' \
    '# Over width' '## How it works' 'Run.' '## Report' '```text'
  yes x | head -100 | tr -d '\n'
  printf '\n%s\n' '```'
} >"$bare/docs/reference/known-issues/KI-b.md"
reject_msg 'manifest-less codebase ki retire' 'known-issues:a-record-carries-its-retirement-condition' \
  run_at "$bare" "$hooks/ki-retire-when.sh"
reject_msg 'manifest-less codebase ki mechanism' 'known-issues:a-record-walks-the-mechanism' \
  run_at "$bare" "$hooks/ki-mechanism-walkthrough.sh"
reject_msg 'manifest-less codebase ki report' 'known-issues:a-filed-record-carries-its-report' \
  run_at "$bare" "$hooks/ki-report-body.sh"
reject_msg 'manifest-less codebase bugzilla width' 'known-issues:a-bugzilla-report-body-fits-in-79-columns' \
  run_at "$bare" "$hooks/ki-bugzilla-report-width.sh"

present="$tmp/present.md" history="$tmp/history.md"
printf '# Present\n\nThe rule applies now.\n' >"$present"
printf '# History\n\nThis replaces an older rule.\n' >"$history"
accept 'self narration clean' "$hooks/no-self-narration.sh" "$present"
reject_msg 'self narration violation' 'docs-format:document-states-the-present' \
  "$hooks/no-self-narration.sh" "$history"

spec="$base/_docs/specs/SPEC-sample.md"
# shellcheck disable=SC2016
printf '# Sample Specification\n\n## Purpose\n\nRules.\n\n## Requirements\n\n### `sample:works` — Sample works\n\nThe sample MUST work.\n\n#### Scenario: Run\n\n- GIVEN input\n- WHEN run\n- THEN output\n\nVerify: `pre-commit run sample-hook --all-files`\n' >"$spec"
printf 'repos:\n  - repo: local\n    hooks:\n      - id: sample-hook\n        name: sample\n        entry: true\n        language: system\n' >"$base/.pre-commit-config.yaml"
accept 'spec parts' "$hooks/spec-requirement-parts.sh" "$spec"
accept 'spec parts under the C locale' env LC_ALL=C "$hooks/spec-requirement-parts.sh" "$spec"
sed -i '/^Verify:/d' "$spec"
reject_msg 'spec parts missing verify' 'docs-specs:requirement-carries-five-parts' \
  "$hooks/spec-requirement-parts.sh" "$spec"
# shellcheck disable=SC2016
printf 'Verify: `pre-commit run sample-hook --all-files`\n' >>"$spec"
accept 'spec IDs unique' run_at "$base" "$hooks/spec-rule-id-unique.sh"
cp "$spec" "$base/_docs/specs/SPEC-copy.md"
reject_msg 'spec duplicate ID' 'docs-specs:rule-id-is-unique-and-slugged' \
  run_at "$base" "$hooks/spec-rule-id-unique.sh"
rm "$base/_docs/specs/SPEC-copy.md"
accept 'spec size' run_at "$base" "$hooks/spec-size-cap.sh"
yes line | head -301 >>"$spec"
reject_msg 'spec size over' 'docs-specs:spec-stays-within-300-lines' \
  run_at "$base" "$hooks/spec-size-cap.sh"
sed -i '22,$d' "$spec"
accept 'spec hook exists' run_at "$base" "$hooks/spec-verify-hooks-exist.sh"
sed -i 's/id: sample-hook/id: renamed-hook/' "$base/.pre-commit-config.yaml"
reject_msg 'spec hook renamed' 'docs-specs:verification-names-a-live-hook' \
  run_at "$base" "$hooks/spec-verify-hooks-exist.sh"
sed -i 's/id: renamed-hook/id: sample-hook/' "$base/.pre-commit-config.yaml"

gates="$tmp/gates"
mkdir -p "$gates/hooks" "$gates/_docs/specs"
cp "$hooks/lib-instance-paths.sh" "$hooks/gate-message-cites-a-rule.sh" "$gates/hooks/"
# The fixture spec defines the gate's own rule too, because the gate reads
# every hook in the directory it is run from, including itself.
# shellcheck disable=SC2016
printf '# Sample Specification\n\n## Requirements\n\n### `sample:works` — Sample works\n\nThe sample MUST work.\n\nVerify: `true`\n\n### `spec-to-code:a-gate-message-cites-the-rule` — A gate message cites the rule\n\nThe author MUST make every rule ID a gate prints resolve to a requirement.\n\nVerify: `true`\n' >"$gates/_docs/specs/SPEC-sample.md"
printf '#!/usr/bin/env sh\necho "FAIL %s nothing"\n' 'sample:works' >"$gates/hooks/sample.sh"
accept 'gate message cites a rule' run_at "$gates" "$gates/hooks/gate-message-cites-a-rule.sh"
printf '#!/usr/bin/env sh\necho "FAIL %s nothing"\n' 'sample:renamed' >"$gates/hooks/sample.sh"
reject_msg 'gate message cites a missing rule' 'spec-to-code:a-gate-message-cites-the-rule' \
  run_at "$gates" "$gates/hooks/gate-message-cites-a-rule.sh"

accept 'suppression absent' run_at "$base" "$hooks/suppression-names-its-case.sh"
printf '<!-- markdownlint-%s -->\n' disable >"$base/local.md"
reject_msg 'suppression without case' 'spec-to-code:a-suppression-names-its-case' \
  run_at "$base" "$hooks/suppression-names-its-case.sh"
printf '<!-- markdownlint-%s KI-absent-record -->\n' disable >"$base/local.md"
reject_msg 'suppression naming no record' 'resolves to no record' \
  run_at "$base" "$hooks/suppression-names-its-case.sh"
rm "$base/local.md"

accept 'md spec shape' markdownlint-cli2 --config "$repo/.markdownlint/spec.markdownlint-cli2.jsonc" "$spec"
bad_spec="$tmp/bad-spec.md"
sed 's/## Purpose/## purpose/' "$spec" >"$bad_spec"
reject 'md spec wrong case' markdownlint-cli2 --config "$repo/.markdownlint/spec.markdownlint-cli2.jsonc" "$bad_spec"
adr="$tmp/ADR-sample.md"
printf '# Sample\n\n## Context and Problem Statement\n\nContext.\n\n## Considered Options\n\n- x — chosen.\n\n## Decision Outcome\n\nChosen.\n\n## Consequences\n\n- Good: small\n\n## Status\n\nAccepted\n' >"$adr"
accept 'md adr shape' markdownlint-cli2 --config "$repo/.markdownlint/adr.markdownlint-cli2.jsonc" "$adr"
bad_adr="$tmp/bad-adr.md"
sed 's/## Status/## status/' "$adr" >"$bad_adr"
reject 'md adr wrong case' markdownlint-cli2 --config "$repo/.markdownlint/adr.markdownlint-cli2.jsonc" "$bad_adr"

root="$repo"
accept 'license split' run_at "$root" "$canon_gates/license-split.sh"
lic="$tmp/license"
mkdir -p "$lic"
cp "$root/LICENSE" "$root/LICENSE-MIT" "$root/LICENSE-CC-BY-4.0" "$lic/"
sed -i '/LICENSE-MIT/d' "$lic/LICENSE"
reject_msg 'license names one half' 'distribution:license-declares-both-halves' \
  run_at "$lic" "$canon_gates/license-split.sh"
cp "$root/LICENSE" "$lic/LICENSE"
rm "$lic/LICENSE-MIT"
reject_msg 'license half missing' 'distribution:license-declares-both-halves' \
  run_at "$lic" "$canon_gates/license-split.sh"

# The version gate is run against copies rather than the checkout, because the
# tag half of it reads the repository it stands in: a control that edited
# `VERSION` here would be asserting against this repository's own tags.
accept 'version source of truth' run_at "$root" "$canon_gates/version-source-of-truth.sh"
ver="$tmp/version"
mkdir -p "$ver/.spec-driven-docs"
cp "$root/VERSION" "$ver/VERSION"
cp "$root/.spec-driven-docs/manifest.json" "$ver/.spec-driven-docs/manifest.json"
accept 'version files only' run_at "$ver" "$canon_gates/version-source-of-truth.sh" --files-only
printf 'v1.2\n' >"$ver/VERSION"
reject_msg 'version not semantic' 'distribution:versions-are-semantic-and-aligned' \
  run_at "$ver" "$canon_gates/version-source-of-truth.sh"
printf '9.9.9\n' >"$ver/VERSION"
reject_msg 'manifest version drift' 'distribution:versions-are-semantic-and-aligned' \
  run_at "$ver" "$canon_gates/version-source-of-truth.sh"

accept 'instance manifest' run_at "$root" "$hooks/instance-manifest.sh"
instance="$tmp/instance"
mkdir -p "$instance/.spec-driven-docs"
cp "$root/.spec-driven-docs/manifest.json" "$instance/.spec-driven-docs/manifest.json"
sed -i 's/"schema_version": 1/"schema_version": 2/' "$instance/.spec-driven-docs/manifest.json"
reject_msg 'instance manifest schema' 'distribution:manifest-identifies-every-owned-file' \
  run_at "$instance" "$hooks/instance-manifest.sh"

# The boundary gate runs against a copy of the repository, because it reads the
# declaration, the published manifest and both gate directories by relative
# path. Each control breaks exactly one of the three agreements it holds.
accept 'delivered domain' run_at "$root" "$canon_gates/delivered-domain.sh"
dom="$tmp/domain"
mkdir -p "$dom/instance" "$dom/gates"
cp "$root/instance/gates.json" "$dom/instance/gates.json"
cp "$root/.pre-commit-hooks.yaml" "$dom/.pre-commit-hooks.yaml"
cp -R "$root/gates/instance" "$root/gates/canon" "$dom/gates/"
accept 'delivered domain copy' run_at "$dom" "$canon_gates/delivered-domain.sh"

printf '#!/usr/bin/env sh\nexit 0\n' >"$dom/gates/instance/stray.sh"
chmod 755 "$dom/gates/instance/stray.sh"
reject_msg 'delivered gate undeclared' 'distribution:the-delivered-gate-set-is-declared-once' \
  run_at "$dom" "$canon_gates/delivered-domain.sh"
rm "$dom/gates/instance/stray.sh"

sed -i '/^- id: adr-word-cap$/d' "$dom/.pre-commit-hooks.yaml"
reject_msg 'delivered gate unpublished' 'distribution:the-delivered-gate-set-is-declared-once' \
  run_at "$dom" "$canon_gates/delivered-domain.sh"
cp "$root/.pre-commit-hooks.yaml" "$dom/.pre-commit-hooks.yaml"

printf -- '- id: license-split\n  name: license split\n  entry: gates/canon/license-split.sh\n  language: script\n  always_run: true\n  pass_filenames: false\n' >>"$dom/.pre-commit-hooks.yaml"
reject_msg 'canon gate published' 'distribution:a-canon-gate-is-not-delivered' \
  run_at "$dom" "$canon_gates/delivered-domain.sh"

echo "OK $passed gate controls"
