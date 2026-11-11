#!/usr/bin/env bash
# One-time repository protection for Burl (#180 stories 1-6, #95). Owner-run.
#
#   scripts/repo-setup.sh [check] [options]           read-only: GETs only, prints the payloads
#   scripts/repo-setup.sh apply --confirm-apply [...]  writes; then asks you to type a confirmation
#
# Creates (or updates, by name; idempotent):
#   - ruleset "burl-master" on the default branch requiring status checks;
#   - environment "release" with the authenticated user as required reviewer and a deployment
#     policy that accepts only tags matching v*.
#
# Options (decisions that belong to the owner; all off by default):
#   --require-heavy    also require the heavy-tier checks. CAUTION: heavy.yml is path-filtered, so a
#                      PR touching none of its paths never reports them and would wait forever.
#   --require-review   require 1 approving review (a solo owner cannot approve their own PR).
#   --admin-bypass     let repository admins bypass the ruleset.
#   --repo OWNER/REPO  default: the repo `gh` resolves for this checkout.
#
# Required check names (evidence: check-runs of merge commit 795e249, PR #182, and ci.yml):
#   fast tier  linux, linux-pcre2, macos (job `fast`, name: ${{ matrix.name }}, so the matrix
#              name is the check name, NOT "fast"), deny, golden-label (pull_request only; a
#              skipped run counts as passing, which also means a merge queue would need care).
#   heavy tier goldens-reproduce, sim-goldens-reproduce, slow-tests, render-smoke, visual-ssim,
#              window-xvfb.
#
# NOT VERIFIED: GitHub has no dry-run for rulesets or environments, so the payloads built here
# are validated only for JSON well-formedness and shape (jq) and against the REST docs:
#   https://docs.github.com/en/rest/repos/rules
#   https://docs.github.com/en/rest/deployments/environments
#   https://docs.github.com/en/rest/deployments/branch-policies
# Whether GitHub accepts them is only known by running `apply`. Apply re-reads the settings after.
# The admin role id (5) used by --admin-bypass is from memory, not from those pages.
set -euo pipefail

readonly EXIT_OK=0
readonly EXIT_FAILURE=1
readonly EXIT_USAGE=2
readonly EXIT_DEPENDENCY=4

RULESET_NAME=burl-master
FAST_CHECKS=(linux linux-pcre2 macos deny golden-label)
HEAVY_CHECKS=(goldens-reproduce sim-goldens-reproduce slow-tests render-smoke visual-ssim window-xvfb)

usage() {
	sed -n '2,/^set -euo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'
}

die() {
	local code=$1
	shift
	echo "repo-setup: $*" >&2
	exit "$code"
}

mode=check
confirm_flag=0
heavy=0
review=0
bypass=0
repo=""
while [ $# -gt 0 ]; do
	case $1 in
	-h | --help)
		usage
		exit "$EXIT_OK"
		;;
	check | apply) mode=$1 ;;
	--confirm-apply) confirm_flag=1 ;;
	--require-heavy) heavy=1 ;;
	--require-review) review=1 ;;
	--admin-bypass) bypass=1 ;;
	--repo)
		[ $# -ge 2 ] || die "$EXIT_USAGE" "--repo needs OWNER/REPO"
		repo=$2
		shift
		;;
	*) die "$EXIT_USAGE" "unknown argument: $1 (see --help)" ;;
	esac
	shift
done

# Refuse before any network call.
if [ "$mode" = apply ] && [ "$confirm_flag" -ne 1 ]; then
	die "$EXIT_USAGE" "apply writes repository settings; pass --confirm-apply (run 'check' first)"
fi

for c in gh jq; do
	command -v "$c" >/dev/null 2>&1 || die "$EXIT_DEPENDENCY" "$c not found on PATH"
done

[ -n "$repo" ] || repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner)
branch=$(gh api "repos/$repo" --jq .default_branch)
owner_id=$(gh api user --jq .id)
case $owner_id in "" | *[!0-9]*) die "$EXIT_FAILURE" "could not read a numeric user id from 'gh api user'" ;; esac

checks=("${FAST_CHECKS[@]}")
[ "$heavy" -eq 1 ] && checks+=("${HEAVY_CHECKS[@]}")

ruleset_payload=$(
	printf '%s\n' "${checks[@]}" | jq -R . | jq -s \
		--arg name "$RULESET_NAME" --argjson review "$review" --argjson bypass "$bypass" '
    { name: $name, target: "branch", enforcement: "active",
      bypass_actors: (if $bypass == 1 then [{actor_id: 5, actor_type: "RepositoryRole", bypass_mode: "always"}] else [] end),
      conditions: {ref_name: {include: ["~DEFAULT_BRANCH"], exclude: []}},
      rules: ([{type: "required_status_checks", parameters: {
                 strict_required_status_checks_policy: false,
                 required_status_checks: map({context: .})}}]
              + (if $review == 1 then [{type: "pull_request", parameters: {
                 required_approving_review_count: 1, dismiss_stale_reviews_on_push: false,
                 require_code_owner_review: false, require_last_push_approval: false,
                 required_review_thread_resolution: false}}] else [] end)) }'
)
env_payload=$(jq -n --argjson id "$owner_id" '
  {wait_timer: 0, prevent_self_review: false,
   reviewers: [{type: "User", id: $id}],
   deployment_branch_policy: {protected_branches: false, custom_branch_policies: true}}')
policy_payload='{"name":"v*","type":"tag"}'

# Shape check: a malformed payload must never be shown as one we "would send".
echo "$ruleset_payload" | jq -e '
  (.rules[0].parameters.required_status_checks | length > 0 and all(.context | type == "string"))
  and .enforcement == "active" and (.conditions.ref_name.include == ["~DEFAULT_BRANCH"])' >/dev/null ||
	die "$EXIT_FAILURE" "internal: ruleset payload failed its shape check"
echo "$env_payload" | jq -e '(.reviewers | length == 1) and .deployment_branch_policy.custom_branch_policies' >/dev/null ||
	die "$EXIT_FAILURE" "internal: environment payload failed its shape check"

echo "== repo: $repo  default branch: $branch  owner id: $owner_id"
echo "== current state (GET only)"
rulesets=$(gh api "repos/$repo/rulesets")
echo "rulesets: $(echo "$rulesets" | jq -c '[.[] | {id, name, enforcement}]')"
# No classic protection is a 404; that is information, not an error.
if prot=$(gh api "repos/$repo/branches/$branch/protection" 2>/dev/null); then
	echo "classic branch protection: $(echo "$prot" | jq -c '{checks: .required_status_checks.contexts}')"
else
	echo "classic branch protection: none"
fi
envs=$(gh api "repos/$repo/environments")
echo "environments: $(echo "$envs" | jq -c '[.environments[] | {name}]')"
policies='{"branch_policies":[]}'
if echo "$envs" | jq -e '.environments[] | select(.name == "release")' >/dev/null 2>&1; then
	policies=$(gh api "repos/$repo/environments/release/deployment-branch-policies")
	echo "release policies: $(echo "$policies" | jq -c '[.branch_policies[] | {name, type}]')"
fi

ruleset_id=$(echo "$rulesets" | jq -r --arg n "$RULESET_NAME" '[.[] | select(.name == $n) | .id][0] // empty')
have_policy=$(echo "$policies" | jq -r '[.branch_policies[] | select(.name == "v*" and .type == "tag")] | length')
if [ -n "$ruleset_id" ]; then
	echo "== would PUT repos/$repo/rulesets/$ruleset_id"
else
	echo "== would POST repos/$repo/rulesets"
fi
echo "$ruleset_payload" | jq .
echo "== would PUT repos/$repo/environments/release"
echo "$env_payload" | jq .
if [ "$have_policy" -eq 0 ]; then
	echo "== would POST repos/$repo/environments/release/deployment-branch-policies"
	echo "$policy_payload" | jq .
else
	echo "== tag policy v* already present; nothing to POST"
fi

[ "$mode" = apply ] || {
	echo "== check only: nothing was changed. These payloads are NOT proven accepted (no dry-run exists)."
	exit "$EXIT_OK"
}

echo
printf 'Type "apply %s" to write these settings: ' "$repo"
read -r answer || answer=""
[ "$answer" = "apply $repo" ] || die "$EXIT_FAILURE" "confirmation did not match; nothing was changed"

if [ -n "$ruleset_id" ]; then
	echo "$ruleset_payload" | gh api --method PUT "repos/$repo/rulesets/$ruleset_id" --input - >/dev/null
else
	echo "$ruleset_payload" | gh api --method POST "repos/$repo/rulesets" --input - >/dev/null
fi
echo "$env_payload" | gh api --method PUT "repos/$repo/environments/release" --input - >/dev/null
if [ "$have_policy" -eq 0 ]; then
	echo "$policy_payload" | gh api --method POST "repos/$repo/environments/release/deployment-branch-policies" --input - >/dev/null
fi

echo "== re-reading settings (story 5)"
gh api "repos/$repo/rulesets" --jq '[.[] | {id, name, enforcement}]'
gh api "repos/$repo/environments/release" --jq '{name, reviewers: [.protection_rules[]? | select(.type == "required_reviewers") | .reviewers[].reviewer.login], policy: .deployment_branch_policy}'
gh api "repos/$repo/environments/release/deployment-branch-policies" --jq '[.branch_policies[] | {name, type}]'
echo "== done. Next: open a PR with a failing fast check to confirm it cannot merge."
