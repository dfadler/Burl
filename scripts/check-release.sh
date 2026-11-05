#!/usr/bin/env bash
# Release sign-off gate (#21, #14): a human step the release workflow cannot skip.
#   scripts/check-release.sh vX.Y.Z
# Checks, on the tagged commit: the tag equals the workspace version; the interactive checklist
# has every box ticked and a sign-off per platform; every automated proof it names still exists;
# the version and the DEVIATIONS.md blob hash recorded in the sign-off match (so any later register
# edit invalidates the review).
set -euo pipefail

case "${1:-}" in -h | --help | "")
	sed -n '2,7p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

TAG=$1
ROOT=$(cd "$(dirname "$0")/.." && pwd)
SIGNOFF=${SIGNOFF_FILE:-$ROOT/docs/rust-rewrite/INTERACTIVE-CHECKLIST.md}
REGISTER=${REGISTER_FILE:-$ROOT/docs/rust-rewrite/DEVIATIONS.md}
fail=0
bad() { echo "FAIL: $*" >&2; fail=1; }

version=${TAG#v}
core=${version%%-*}
ws=$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)
[ "$core" = "$ws" ] || bad "tag $TAG does not match the workspace version $ws"

# The register hash and version live in the sign-off block; `key: value`, TODO or empty = unsigned.
field() { sed -n "s/^$1: *//p" "$SIGNOFF" | head -1; }
signed() { local v; v=$(field "$1"); [ -n "$v" ] && [ "$v" != TODO ]; }

[ -f "$SIGNOFF" ] || { echo "FAIL: $SIGNOFF missing" >&2; exit 1; }
if grep -q '^- \[ \]' "$SIGNOFF"; then
	bad "unticked checklist items: $(grep -c '^- \[ \]' "$SIGNOFF")"
fi
# Every automated item names its proof as `path::name` in a table row. A Rust name must still be a
# `fn`; anything else (a window-check message, a CI job name) must still appear literally.
bt='`' # a variable, so shellcheck does not read the backticks as command substitution
proofs=$(grep '^|' "$SIGNOFF" | grep -o "${bt}[^${bt}]*::[^${bt}]*${bt}" | tr -d "$bt" | sort -u || true)
[ -n "$proofs" ] || bad "no automated-check proofs in $SIGNOFF"
while IFS= read -r p; do
	[ -n "$p" ] || continue
	file=${p%%::*} needle=${p#*::}
	if [ ! -f "$ROOT/$file" ]; then
		bad "proof file missing: $p"
	elif [[ $file == *.rs && $needle =~ ^[a-z0-9_]+$ ]]; then
		grep -qE "fn ${needle}[(<]" "$ROOT/$file" || bad "proof test gone: $p"
	else
		grep -qF -- "$needle" "$ROOT/$file" || bad "proof text gone: $p"
	fi
done <<<"$proofs"
# The Known gap section lists what the old checklist asked for but the code lacks. It must be
# empty before a release: each item implemented, or recorded in DEVIATIONS.md and then removed.
gaps=$(awk '/^## Known gap/ {s=1; next} /^## / {s=0} s && /^- /' "$SIGNOFF" | wc -l | tr -d ' ')
[ "$gaps" = 0 ] || bad "known gaps unresolved: $gaps (implement each or record it in DEVIATIONS.md, then remove it from the Known gap section)"
for k in macos-retina macos-non-retina linux-x11; do
	signed "$k" || bad "no sign-off for $k"
done
[ "$(field version)" = "$core" ] || bad "sign-off version '$(field version)' != $core"
reg=$(git hash-object "$REGISTER")
[ "$(field deviation-register)" = "$reg" ] ||
	bad "deviation register changed since review (blob is $reg, sign-off has '$(field deviation-register)')"

[ "$fail" = 0 ] && echo "release sign-off OK for $TAG"
exit "$fail"
