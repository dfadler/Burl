#!/usr/bin/env bash
# Hermetic test for scripts/repo-setup.sh: a stub `gh` on PATH, no network, no real repo.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
SCRIPT=${REPO_SETUP:-$ROOT/scripts/repo-setup.sh}
T=$(mktemp -d "${TMPDIR:-/tmp}/repo-setup-test.XXXXXX")
trap 'rm -rf "$T"' EXIT
mkdir "$T/bin"
export CALLS="$T/calls"
: >"$CALLS"

cat >"$T/bin/gh" <<'STUB'
#!/usr/bin/env bash
echo "$*" >>"$CALLS"
case "$*" in
"repo view"*) echo octo/cat ;;
*"--method"*) echo '{}' ;;
"api repos/octo/cat --jq"*) echo main ;;
"api user --jq .id") echo 4242 ;;
"api repos/octo/cat/rulesets") echo '[{"id":7,"name":"burl-master","enforcement":"active"}]' ;;
*"/branches/main/protection"*) echo "Not Found" >&2; exit 1 ;;
"api repos/octo/cat/environments") echo '{"environments":[{"name":"release"}]}' ;;
*"deployment-branch-policies"*) echo '{"branch_policies":[]}' ;;
*) echo '{}' ;;
esac
STUB
chmod +x "$T/bin/gh"
export PATH="$T/bin:$PATH"

fail=0
ok() { echo "ok: $*"; }
bad() {
	echo "FAIL: $*" >&2
	fail=1
}

out=$("$SCRIPT" check --repo octo/cat 2>&1) || bad "check exited non-zero"
for name in linux linux-pcre2 macos deny golden-label; do
	echo "$out" | grep -q "\"$name\"" || bad "check output lacks required check $name"
done
echo "$out" | grep -q '"visual-ssim"' && bad "heavy checks required by default"
echo "$out" | grep -q 'would PUT repos/octo/cat/rulesets/7' || bad "existing ruleset not updated by id"
echo "$out" | grep -q '"id": 4242' || bad "owner id missing from reviewers"
echo "$out" | grep -q '"name": "v\*"' || bad "tag policy payload missing"
grep -qE -- '--method|--input| -X' "$CALLS" && bad "check mode issued a write"
ok "check mode"

out=$("$SCRIPT" check --repo octo/cat --require-heavy --require-review --admin-bypass 2>&1)
for s in '"visual-ssim"' '"pull_request"' '"RepositoryRole"'; do
	echo "$out" | grep -q "$s" || bad "options: missing $s"
done
ok "options"

: >"$CALLS"
rc=0
"$SCRIPT" apply --repo octo/cat </dev/null >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 2 ] || bad "apply without flag exited $rc, want 2"
[ ! -s "$CALLS" ] || bad "apply without flag called gh"
rc=0
echo "no" | "$SCRIPT" apply --confirm-apply --repo octo/cat >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 1 ] || bad "apply with wrong confirmation exited $rc, want 1"
grep -qE -- '--method' "$CALLS" && bad "apply wrote without confirmation"
ok "apply refusals"

echo "apply octo/cat" | "$SCRIPT" apply --confirm-apply --repo octo/cat >/dev/null 2>&1 || bad "confirmed apply failed"
grep -q -- '--method PUT repos/octo/cat/rulesets/7' "$CALLS" || bad "confirmed apply did not PUT the ruleset"
grep -q -- '--method POST repos/octo/cat/environments/release/deployment-branch-policies' "$CALLS" || bad "no policy POST"
ok "confirmed apply (stub)"

exit "$fail"
