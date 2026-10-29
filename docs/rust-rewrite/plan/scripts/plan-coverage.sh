#!/bin/bash
# Mechanical coverage check for the planning loop (LEDGER.md, done criterion layer A).
# Reports gaps; never edits. Exit 0 when no gaps, 1 otherwise.
# Usage: plan-coverage.sh            full report
#        plan-coverage.sh --methods src/file.cpp   print the method list a functions/ doc must cover
#        plan-coverage.sh -h
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../../../.." && pwd)
PLAN="$REPO/docs/rust-rewrite/plan"
DISC="$REPO/docs/rust-rewrite/discovery"
cd "$REPO" || exit 2

methods() {
  # Out-of-line member definitions `Type Class::method(` at column 0; dedupe overloads.
  grep -hE '^[A-Za-z_][A-Za-z0-9_:<>*&, ]*[ *&]([A-Za-z_][A-Za-z0-9_]*)::(~?[A-Za-z_][A-Za-z0-9_]*|operator[^ (]*)\s*\(' "$1" \
    | sed -E 's/^.*[ *&]([A-Za-z_][A-Za-z0-9_]*::(~?[A-Za-z_][A-Za-z0-9_]*|operator[^ (]*))\s*\(.*$/\1/' \
    | sort -u
}

case "${1:-}" in
  -h|--help) sed -n '2,6p' "$0"; exit 0 ;;
  --methods) methods "$2"; exit 0 ;;
esac

gaps=0
section() { printf '\n== %s\n' "$1"; }
gap() { gaps=$((gaps+1)); printf '  gap: %s\n' "$1"; }

# A.1 every built translation unit has a row in map/modules.md
section "A.1 translation units -> map/modules.md"
# shellcheck disable=SC1003  # the set is space, tab and a literal backslash (line continuations), not an escaped quote
units=$(tr -s ' \t\\' '\n' < Makefile.am | grep -E '^src/.*\.cpp$' | sort -u)
for u in $units; do
  grep -qF "| $u |" "$PLAN/map/modules.md" 2>/dev/null || gap "no module row for $u"
done

# A.2 function-level docs for the state-producing files
section "A.2 methods -> functions/<file>.md"
STATE_FILES="src/gource.cpp src/gource_shell.cpp src/dirnode.cpp src/file.cpp src/user.cpp src/pawn.cpp src/action.cpp src/zoomcamera.cpp src/spline.cpp src/caption.cpp src/key.cpp src/slider.cpp src/textbox.cpp src/logmill.cpp src/gource_settings.cpp src/main.cpp src/core/quadtree.cpp src/core/seeklog.cpp src/core/regex.cpp src/core/stringhash.cpp src/core/settings.cpp src/core/conffile.cpp src/core/timezone.cpp src/core/vectors.cpp src/formats/commitlog.cpp src/formats/custom.cpp src/formats/git.cpp src/formats/gitraw.cpp src/formats/svn.cpp src/formats/hg.cpp src/formats/bzr.cpp src/formats/cvs-exp.cpp src/formats/cvs2cl.cpp src/formats/apache.cpp"
for f in $STATE_FILES; do
  [ -f "$f" ] || { gap "state file missing in checkout: $f (core submodule not checked out?)"; continue; }
  doc="$PLAN/functions/$(basename "$f" .cpp).md"
  if [ ! -f "$doc" ]; then
    n=$(methods "$f" | wc -l | tr -d ' ')
    gap "no functions doc for $f ($n methods)"
    continue
  fi
  missing=0
  while IFS= read -r m; do
    [ -n "$m" ] || continue
    grep -qF "$m" "$doc" || { missing=$((missing+1)); printf '  gap: %s lacks %s\n' "$(basename "$doc")" "$m"; gaps=$((gaps+1)); }
  done < <(methods "$f")
  for col in 'Rust signature' 'Pseudocode' 'Pins' 'Cite' '## State and order hazards' '## Corrections'; do
    grep -qF "$col" "$doc" || gap "$(basename "$doc") lacks '$col'"
  done
done

# A.3 options -> settings-spec.md
section "A.3 options -> settings-spec.md"
opts=$( { grep -oE -- '--[a-z][a-z0-9-]+' "$DISC/golden/help-long.txt" | sed 's/^--//'; grep -oE '\("[a-z?][a-z0-9-]*", ArgType' crates/gource-conf/src/args.rs | sed -E 's/\("([^"]+)".*/\1/'; } | sort -u)
for o in $opts; do
  grep -qF "\`$o\`" "$PLAN/settings-spec.md" 2>/dev/null || gap "no settings-spec row for $o"
done
for sec in '## ConfFile grammar' '## Load order' '## Per-section reset set'; do
  grep -qF "$sec" "$PLAN/settings-spec.md" 2>/dev/null || gap "settings-spec.md lacks '$sec'"
done

# A.4 matrix rows -> golden or generator exists
section "A.4 parity/matrix.md rows"
if [ -f "$PLAN/parity/matrix.md" ]; then
  rows=$(grep -cE '^\| *[0-9]+ *\|' "$PLAN/parity/matrix.md")
  [ "$rows" -ge 28 ] || gap "matrix has $rows numbered rows, expected at least 28"
  while IFS= read -r line; do
    id=$(echo "$line" | sed -E 's/^\| *([0-9]+).*/\1/')
    path=$(echo "$line" | grep -oE '(golden|script): *[^ |]+' | head -1 | sed -E 's/^[a-z]+: *//')
    if [ -z "$path" ]; then gap "matrix row $id names no golden: or script: path"
    elif [ ! -e "$REPO/$path" ] && [ ! -e "$PLAN/$path" ]; then gap "matrix row $id path missing: $path"; fi
  done < <(grep -E '^\| *[0-9]+ *\|' "$PLAN/parity/matrix.md")
else
  gap "parity/matrix.md missing"
fi

# A.5 crates -> version, docs.rs cite with date, compiled snippet
section "A.5 rust/crates.md rows"
if [ -f "$PLAN/rust/crates.md" ]; then
  # shellcheck disable=SC2016  # the backticks in the sed and grep patterns are literal markdown, not command substitution
  while IFS= read -r line; do
    name=$(echo "$line" | sed -E 's/^\| *`([^`]+)`.*/\1/')
    echo "$line" | grep -qE '[0-9]+\.[0-9]+(\.[0-9]+)?' || gap "crate $name has no version"
    echo "$line" | grep -qE 'docs\.rs/[^ |]+' || gap "crate $name has no docs.rs cite"
    echo "$line" | grep -qE '20[0-9]{2}-[0-9]{2}-[0-9]{2}' || gap "crate $name cite has no date"
    [ -f "$PLAN/rust/snippets/$name/Cargo.toml" ] || gap "crate $name has no snippet under rust/snippets/$name/"
  done < <(grep -E '^\| *`[a-z0-9_-]+` *\|' "$PLAN/rust/crates.md")
  for d in "$PLAN"/rust/snippets/*/; do
    [ -d "$d" ] || continue
    (cd "$d" && cargo check -q >/dev/null 2>&1) || gap "snippet does not cargo check: $d"
  done
else
  gap "rust/crates.md missing"
fi

# A.6 phase briefs
section "A.6 phases/phase-N.md"
for n in 1 3 4 5 6 7; do
  p="$PLAN/phases/phase-$n.md"
  [ -f "$p" ] || { gap "phase brief missing: phase-$n.md"; continue; }
  for h in '## Goal' '## Inputs' '## Outputs' '## Module tree' '## Interfaces' '## Behaviour' '## Gate' '## Acceptance' '## Size' '## Decisions touched' '## Not in this phase'; do
    grep -qF "$h" "$p" || gap "phase-$n.md lacks '$h'"
  done
done

# B dry-runs
section "B dry-runs/phase-N.md"
for n in 1 3 4 5 6 7; do
  d="$PLAN/dry-runs/phase-$n.md"
  [ -f "$d" ] || { gap "no dry-run for phase $n"; continue; }
  grep -qE '^questions: 0$' "$d" || gap "phase $n dry-run has questions"
  grep -qE '^cpp_opens: 0$' "$d" || gap "phase $n dry-run opened disallowed files"
  grep -qE '^gate: (pass|not capturable here)' "$d" || gap "phase $n dry-run gate not passing"
done

# C verification lines newer than last content change
section "C verified: lines"
for f in "$PLAN"/*.md "$PLAN"/*/*.md; do
  [ -f "$f" ] || continue
  case "$(basename "$f")" in LEDGER.md|LOOP.md|BRIEFS.md|decisions.md) continue ;; esac
  rel=${f#"$REPO/"}
  v=$(grep -oE '^verified: [0-9a-f]{7,40}' "$f" | tail -1 | awk '{print $2}')
  if [ -z "$v" ]; then gap "$rel has no verified: line"; continue; fi
  last=$(git log -1 --format=%H -- "$rel" 2>/dev/null)
  # the verified commit must be the last commit that touched the file, or come after it
  if [ -n "$last" ] && ! git merge-base --is-ancestor "$last" "$v" 2>/dev/null; then
    gap "$rel changed after its verified: line ($v)"
  fi
  git diff --quiet -- "$rel" 2>/dev/null || gap "$rel has uncommitted changes after verification"
done

printf '\nSUMMARY: %d gaps\n' "$gaps"
[ "$gaps" -eq 0 ]
