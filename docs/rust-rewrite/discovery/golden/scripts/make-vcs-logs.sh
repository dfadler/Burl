#!/bin/bash
# Builds tiny scripted repositories (fixed authors and dates, no network) and writes the logs
# Gource's own log commands produce, as committed inputs for tests/golden/real/ (#17).
# Runs inside the golden image; the repositories live in a temp dir and are not committed.
set -euo pipefail
OUT=${1:-tests/golden/real}
STYLE=$(cd "$(dirname "$0")/../../../../.." && pwd)/data/gource.style
W=$(mktemp -d)
trap 'rm -rf "$W"' EXIT
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
export TZ=UTC HOME=$W/home LC_ALL=C.UTF-8
mkdir -p "$HOME"
D1=2020-01-01T10:00:00Z D2=2020-01-02T11:00:00Z D3=2020-01-03T12:00:00Z

# the same three steps in every VCS: alice adds a.txt and sub/b.txt, bob modifies a.txt,
# alice deletes sub/b.txt
# --- git (also the raw format)
G=$W/git; mkdir -p "$G"; cd "$G"; git init -q -b main
git config user.name x; git config user.email x@example.com
c() { GIT_AUTHOR_NAME=$1 GIT_COMMITTER_NAME=$1 GIT_AUTHOR_EMAIL=$1@e.com GIT_COMMITTER_EMAIL=$1@e.com GIT_AUTHOR_DATE=$2 GIT_COMMITTER_DATE=$2 git commit -q -m "$3"; }
mkdir sub; echo a > a.txt; echo b > sub/b.txt; git add -A; c alice $D1 one
echo a2 >> a.txt; git add -A; c bob $D2 two
git rm -q sub/b.txt; c alice $D3 three
git log --reverse --raw --encoding=UTF-8 --no-renames --no-show-signature --pretty=format:user:%aN%n%ct > "$OUT/git-tiny.log"
git log --reverse --raw --pretty=raw > "$OUT/gitraw-tiny.log"
# --- svn
R=$W/svnrepo; svnadmin create "$R"; printf '#!/bin/sh\nexit 0\n' > "$R/hooks/pre-revprop-change"; chmod +x "$R/hooks/pre-revprop-change"
WC=$W/svnwc; svn -q checkout "file://$R" "$WC"; cd "$WC"
sc() { svn -q commit --username "$1" -m "$3" >/dev/null; svn -q propset --revprop -r HEAD svn:date "$2.000000Z" --username "$1" "file://$R" >/dev/null; }
mkdir sub; echo a > a.txt; echo b > sub/b.txt; svn -q add a.txt sub; sc alice 2020-01-01T10:00:00 one
echo a2 >> a.txt; sc bob 2020-01-02T11:00:00 two
svn -q rm sub/b.txt; sc alice 2020-01-03T12:00:00 three
svn -q update; # svn emits <path> attributes in a random order; pin it so the log is reproducible
svn log -r 1:HEAD --xml --verbose --quiet "file://$R" | python3 -c '
import re, sys
def fix(m):
    a = dict(re.findall(r"(\S+)=\"([^\"]*)\"", m.group(1)))
    return "<path\n   " + "\n   ".join(f"{k}=\"{a[k]}\"" for k in ("action", "prop-mods", "text-mods", "kind") if k in a) + ">"
sys.stdout.write(re.sub(r"<path\s+([^>]*?)>", fix, sys.stdin.read()))' > "$OUT/svn-tiny.log"
# --- hg
H=$W/hg; mkdir "$H"; cd "$H"; hg init; export HGUSER=x
mkdir sub; echo a > a.txt; echo b > sub/b.txt; hg add -q; hg commit -q -u alice -d "2020-01-01 10:00:00 +0000" -m one
echo a2 >> a.txt; hg commit -q -u bob -d "2020-01-02 11:00:00 +0000" -m two
hg rm -q sub/b.txt; hg commit -q -u alice -d "2020-01-03 12:00:00 +0000" -m three
hg log -r 0:tip --style "$STYLE" > "$OUT/hg-tiny.log"
# --- bzr (breezy)
B=$W/bzr; mkdir "$B"; cd "$B"; brz init -q . >/dev/null 2>&1; export BRZ_EMAIL="x <x@example.com>"
mkdir sub; echo a > a.txt; echo b > sub/b.txt; brz add -q >/dev/null 2>&1
brz commit -q --author alice --commit-time "2020-01-01 10:00:00 +0000" -m one >/dev/null 2>&1
echo a2 >> a.txt; brz commit -q --author bob --commit-time "2020-01-02 11:00:00 +0000" -m two >/dev/null 2>&1
brz rm -q sub/b.txt >/dev/null 2>&1; brz commit -q --author alice --commit-time "2020-01-03 12:00:00 +0000" -m three >/dev/null 2>&1
brz log --verbose -r 1..-1 --short -n0 --forward > "$OUT/bzr-tiny.log"
for f in git:git gitraw:git svn:svn hg:hg bzr:bzr; do
  echo "TZ=UTC --log-format ${f#*:}" > "$OUT/${f%%:*}-tiny.case"
done
