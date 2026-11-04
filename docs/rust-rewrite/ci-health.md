# CI health (ticket #13)

## Why `macos` showed CANCELLED on every PR (2026-11-04)

Not ours. The `macos` job (`runs-on: macos-15`) never got a runner:

- Affected jobs have an empty `runner_name`, no steps, and `started_at == created_at`; they
  end 29 to 55 minutes later as `cancelled` (runs 41856587272, 41858450893, 41859079543,
  41859429087).
- Check-run annotation on each: "The job was not acquired by Runner of type hosted even after
  multiple attempts" plus "Due to capacity constraints, jobs targeting macOS arm64 runners may
  experience longer queue times."
- `linux` for the same runs succeeded in about 90 s, so the workflow and `concurrency` are fine.
  Every macos job created before 15:19Z that day got a runner (the few `cancelled` ones with a
  runner were superseded by a newer push, which is the intended `cancel-in-progress` on PRs).
  From 15:19Z, ten jobs in a row were starved: a burst of concurrent PRs against GitHub's
  macOS arm64 capacity.

Mitigation: none in the workflow (`timeout-minutes` does not bound queue time). A starved
`macos` is infrastructure, not a code result: re-run it later (`gh run rerun <id> --failed`).
Do not make `macos` a required check unless you accept merges blocked on GitHub capacity.

## What tests the merged result

| Tier | PR | push to master | merge queue (`merge_group`) |
|------|----|----------------|-----------------------------|
| fast (`ci.yml`) | yes | yes (already) | yes |
| heavy (`heavy.yml`) | path-filtered | yes, same path filter | yes, unfiltered |

Master push runs do not cancel each other (`cancel-in-progress` is PR-only), so each merge
gets a verdict, except that a still-pending run is replaced by a newer one.

## Repo settings (the user does these)

1. Settings > Rules > Rulesets > New branch ruleset, target `master`.
2. Require status checks: `linux`, `linux-pcre2`, `deny`, `golden-label` (fast tier; #95 lists `golden-label`,
   which is skipped on `merge_group`, and a skipped required check passes). Leave `macos` out (see above).
3. Enable "Require merge queue". Merge method: merge commit (the repo uses plain merges).
4. Leave heavy checks optional unless you want every queued merge to wait on Docker builds;
   if required, they must be listed in the ruleset so the queue waits for them.

Only after step 3 does `merge_group` ever fire; the triggers are already in the workflows.
Without a merge queue, the master `push` runs are the safety net: they report a broken merge
after the fact.
