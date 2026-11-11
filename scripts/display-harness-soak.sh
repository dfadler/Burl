#!/usr/bin/env bash
# Soak `cargo xtask display-harness` to catch the local flake (#169). Each run gets a window-list
# sampler beside it (0.25 s: on-screen/all burl windows, frontmost app, display count). Logs of
# passing runs are deleted; failing runs keep harness output, sampler log and a setup snapshot.
#   scripts/display-harness-soak.sh [-n RUNS] [-d DEADLINE_MIN] [-o OUT_DIR] [-- harness args...]
# Defaults: 50 runs, 180 min deadline, OUT_DIR=target/soak. macOS only; needs swiftc.
set -uo pipefail

RUNS=50 DEADLINE_MIN=180
ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=$ROOT/target/soak
while [ $# -gt 0 ]; do
	case "$1" in
	-h | --help)
		sed -n '2,6p' "$0" | sed 's/^# \{0,1\}//'
		exit 0
		;;
	-n)
		RUNS=$2
		shift 2
		;;
	-d)
		DEADLINE_MIN=$2
		shift 2
		;;
	-o)
		OUT=$2
		shift 2
		;;
	--)
		shift
		break
		;;
	*)
		echo "unknown arg: $1 (see --help)" >&2
		exit 2
		;;
	esac
done

mkdir -p "$OUT"
SAMPLER=$OUT/sampler
cat >"$OUT/sampler.swift" <<'EOF'
import AppKit
import CoreGraphics
setbuf(stdout, nil)
func count(_ opt: CGWindowListOption) -> Int {
	let l = CGWindowListCopyWindowInfo([opt, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
	return l.filter { ($0[kCGWindowOwnerName as String] as? String) == "burl" && ($0[kCGWindowLayer as String] as? Int) == 0 }.count
}
while true {
	let front = NSWorkspace.shared.frontmostApplication?.localizedName ?? "?"
	var n: UInt32 = 0
	CGGetActiveDisplayList(0, nil, &n)
	print("\(Date().timeIntervalSince1970) on=\(count(.optionOnScreenOnly)) all=\(count(.optionAll)) front=\(front) displays=\(n) locked=\(CGSessionCopyCurrentDictionary().flatMap { ($0 as? [String: Any])?["CGSSessionScreenIsLocked"] } ?? 0)")
	usleep(250_000)
}
EOF
if ! swiftc -O -o "$SAMPLER" "$OUT/sampler.swift" 2>"$OUT/sampler-build.log"; then
	echo "sampler build failed, see $OUT/sampler-build.log" >&2
	exit 1
fi

# Hard deadline (macOS has no `timeout`): stop starting new runs once it passes.
END=$(($(date +%s) + DEADLINE_MIN * 60))
pass=0 fail=0 ran=0
for i in $(seq 1 "$RUNS"); do
	[ "$(date +%s)" -ge "$END" ] && {
		echo "deadline reached after $ran runs"
		break
	}
	ts=$(date +%Y%m%d-%H%M%S)
	base=$OUT/run-$ts-$i
	{
		echo "load: $(uptime)"
		echo "front: $(osascript -e 'tell application "System Events" to name of first process whose frontmost is true' 2>&1)"
		echo "stray vd: $(pgrep -x vd | tr '\n' ' ')"
		echo "stray burl: $(pgrep -x burl | tr '\n' ' ')"
	} >"$base.setup"
	"$SAMPLER" >"$base.sampler" 2>&1 &
	spid=$!
	(cd "$ROOT" && cargo xtask display-harness "$@") >"$base.out" 2>&1
	rc=$?
	kill "$spid" 2>/dev/null
	wait "$spid" 2>/dev/null
	ran=$((ran + 1))
	if [ "$rc" -eq 0 ]; then
		pass=$((pass + 1))
		rm -f "$base".{out,sampler,setup}
	else
		fail=$((fail + 1))
		echo "run $i FAILED (rc=$rc): $base.{out,sampler,setup}"
	fi
	echo "run $i/$RUNS done: $pass pass, $fail fail"
done
echo "soak finished: $ran runs, $pass pass, $fail fail (logs of failures in $OUT)"
[ "$fail" -eq 0 ]
