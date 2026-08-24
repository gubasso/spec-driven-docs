# 06 — Gates

Five rules in this shelf are mechanically checkable and the rest are not. This chapter gives the
command for each of the five, states where it runs, and lists the unenforced rules honestly rather
than implying the whole shelf is gated.

Each command exits non-zero on a violation and prints `file:line`. Set `FILES` to the comparison
documents the project publishes.

## Escaped pipes, before the formatter

An unescaped pipe inside a code span splits the table row. A markdown formatter run afterwards
re-pads the broken table down to the header's column count, so the evidence is gone before a
column-count linter sees it. This check MUST run ahead of the formatter in the hook order.

```bash
awk -F'`' '/^\|/{for(i=2;i<=NF;i+=2){s=$i; gsub(/\\\|/,"",s);
  if(index(s,"|")){printf "%s:%d: unescaped | inside a code span\n",FILENAME,NR; rc=1}}}
  END{exit rc+0}' $FILES
```

## Every symbol carries its word

```bash
! grep -noP '(✅|⚠️|❌|➖|🧪|❓)(?! [a-z])' $FILES
```

A match is the violation, so the command negates `grep`. Requires a `grep` built with PCRE.

## One reference per cell

```bash
awk -F'|' '/^\|/{for(i=2;i<NF;i++){n=gsub(/\]\(/,"&",$i);
  if(n>1){printf "%s:%d: cell %d carries %d references\n",FILENAME,NR,i-1,n; rc=1}}}
  END{exit rc+0}' $FILES
```

## Every table is dated

```bash
awk '/^## /{v=0} /^Verified: /{v=1}
  /^\|[ :|-]*-[ :|-]*\|/{if(!v){printf "%s:%d: table with no Verified line\n",FILENAME,NR; rc=1}}
  END{exit rc+0}' $FILES
```

The state resets at each `##`, so a `Verified:` line covers the tables in its own section and not the
next one.

## A legend exists

```bash
for f in $FILES; do
  grep -q '^Legend: ' "$f" || { printf '%s:1: no legend\n' "$f"; rc=1; }
done; exit ${rc:-0}
```

## Reference integrity

Two checks already common in a markdown repository cover the rest, and they are why
[03 — References](./03-references.md) chooses heading anchors over footnotes:

- A relative-link rule that resolves both the file and its `#fragment` against real headings on disk.
  A renamed evidence section then fails the build instead of failing silently.
- A link checker for the external citations in the sources sections.

## Unenforced

No command checks these. They are review obligations, and a reviewer who is told which rules are
unchecked spends their attention where it matters.

- That a verdict describes a run the author performed.
- That the method was identical for every subject in its row.
- That the rows where the subject loses were included.
- That a `Verified:` date is the date of the runs rather than the date of the edit.
- That a repeated-run claim rests on the number of runs it states.

The first is the shelf's central rule and the least checkable one. Nothing distinguishes a verdict
read off a changelog from a verdict observed, which is why
[04 — Scenarios](./04-scenarios.md) requires the evidence section to carry a version and a date: not
because that proves the run happened, but because inventing one is a deliberate act rather than a
lapse.
