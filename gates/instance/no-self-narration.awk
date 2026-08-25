# A document states what is true now. This reports the markers that narrate how
# it got that way, and exits non-zero when it finds one.
#
# Code is stripped before matching, because a document stating the rule quotes
# the words it forbids and an unstripped match reports the definition as a
# breach. Two constructs carry that code, and each is stripped on CommonMark's
# terms rather than an approximation, because a shortcut here fails in one of
# two ways and both are bad: prose the gate never reads, or a conforming
# document the gate refuses.
#
# A fence opens on three or more backticks or tildes at the content indent of
# whatever holds it, so the container is measured first — blockquote depth and
# the indent inside it. The fence continues only while a line stays inside that
# container, on both counts. A fence left open therefore ends where its
# container ends, whether the next line steps out of the quote, out of the list
# item, or into a different container at the same time.
#
# A code span is a backtick run closed by a run of the same length, and
# CommonMark lets one cross a line ending but never a blank line — block
# structure is settled before inline structure is read. So prose is stripped one
# paragraph at a time: a whole-file buffer pairs the stray backtick in one
# paragraph with the stray backtick in another and blanks everything between.
# The span's content is blanked rather than deleted so newlines survive and every
# report still names the line it came from; a run that never closes is literal
# text, and stays.
#
# awk is the hook entry itself rather than the body of a shell loop. It takes
# every file pre-commit hands it, and its exit code is the verdict — which
# removes the inversion a wrapper invites, where `&&` fires on the clean files
# and lets every real breach through.

function strip_spans(s,   out, i, n, j, k, m, run, endpos, seg) {
  out = ""
  i = 1
  n = length(s)
  while (i <= n) {
    if (substr(s, i, 1) != "`") {
      out = out substr(s, i, 1)
      i++
      continue
    }
    j = i
    while (j <= n && substr(s, j, 1) == "`") j++
    run = j - i
    endpos = 0
    k = j
    while (k <= n) {
      if (substr(s, k, 1) == "`") {
        m = k
        while (m <= n && substr(s, m, 1) == "`") m++
        if (m - k == run) { endpos = m; break }
        k = m
      } else {
        k++
      }
    }
    if (endpos) {
      seg = substr(s, i, endpos - i)
      gsub(/[^\n]/, " ", seg)
      out = out seg
      i = endpos
    } else {
      out = out substr(s, i, run)
      i = j
    }
  }
  return out
}

function report(   text, lines, i) {
  if (held == 0) return
  text = ""
  for (i = 1; i <= held; i++) text = text prose[i] "\n"
  split(strip_spans(text), lines, "\n")
  for (i = 1; i <= held; i++) {
    if (tolower(lines[i]) ~ /formerly|used to be|this replaces|inherited from/) {
      printf "FAIL docs-format:document-states-the-present %s:%d: %s\n", owner, proseline[i], raw[i]
      bad = 1
    }
  }
  held = 0
}

FNR == 1 { report(); fence = 0; owner = FILENAME }

{
  line = $0

  quotes = 0
  while (match(line, /^[ \t]*>/)) {
    quotes++
    sub(/^[ \t]*>[ \t]?/, "", line)
  }
  indent = 0
  if (match(line, /^[ \t]+/)) {
    indent = RLENGTH
    line = substr(line, RSTART + RLENGTH)
  }

  # A fence left open ends with the container that held it.
  if (fence && line != "" && (quotes < fquotes || indent < findent)) fence = 0

  if (match(line, /^(`{3,}|~{3,})/)) {
    delim = substr(line, RSTART, RLENGTH)
    if (fence == 0) {
      fence = 1
      fchar = substr(delim, 1, 1)
      flen = length(delim)
      fquotes = quotes
      findent = indent
    } else if (substr(delim, 1, 1) == fchar && length(delim) >= flen &&
               line ~ /^(`+|~+)[ \t]*$/) {
      fence = 0
    }
    next
  }

  if (fence) next

  # A blank line closes the paragraph, and a code span cannot cross one.
  if (line == "") { report(); next }

  held++
  prose[held] = line
  proseline[held] = FNR
  raw[held] = $0
}

END { report(); exit bad }
