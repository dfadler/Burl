# Burl (Rust rewrite): Handoff

Written 2026-10-30 (git and PR state updated 2026-10-31) so a fresh session with no memory of the conversation can continue. Burl is the name of this project and of the program it produces (the Rust rewrite and what it grows into). Gource means only the original C++ program (version 0.57, OpenGL, GPLv3+) that Burl was derived from and is checked against. Burl is our fork and will become a whole rewrite; it is never submitted back to Gource (user, 2026-10-31). The goal is a Rust rewrite with the same CLI, config format and log formats and the same parsed commits and simulation state (called parity, defined in `CONTEXT.md` at the repository root, now on `master`); rendering only has to be visually equivalent. Breaking changes and new features come after parity.

Path placeholders used below: `<repo>` is the main checkout; `<discovery-worktree>` is `<repo>/.claude/worktrees/rust-rewrite-discovery` (branch `worktree-rust-rewrite-discovery`, pushed and merged via PR #2, worktree clean); `<phase1-worktree>` is `<repo>/.claude/worktrees/hungry-kilby-78e776` (branch `claude/hungry-kilby-78e776` is checked out there, no longer detached); `<home>` is the user's home directory; `<user-global-conventions>` is the user's global conventions folder.

## Where things stand

| Area | State |
|---|---|
| Discovery | Complete. 17 documents in `docs/rust-rewrite/discovery/` plus goldens captured from the real C++ binary in `golden/`. Verified in three passes. All goldens were captured on macOS arm64 hardware running x86_64 binaries under Rosetta; Linux behaviour (`rand()`, GL, fonts) is unverified. |
| Phase 1 code | Cargo workspace on branch `claude/hungry-kilby-78e776` (open as draft PR #3, whose head moves with each push; 3482 lines of Rust in 20 files). `gource-util`, `gource-conf`, `gource-formats` and the `gource` binary are real; `gource-sim`, `gource-render`, `gource-gl` are empty. `cargo test --offline --locked` passes 45 tests on the merged branch (10 golden, 19 formats, 16 util); clippy was clean at the earlier tip and was not re-run after the merge; the `pcre2` feature tests were never run. Only `--log-format`, `--output-custom-log` and the positional path are implemented. |
| Decisions | The user asked 12 questions. Q1 to Q3 were answered in chat, Q4 to Q12 in the questionnaire artifact (Part 1, section C). Decided 2026-10-31 on my recommendations: `--stop-on-idle` stays accepted with no effect; the state-dump fork directory is `reference/gource-cpp-dump/` (does not exist yet); the glossary is in `CONTEXT.md`; the expected help-text diff goes under `docs/rust-rewrite/discovery/golden/expected-diffs/` (does not exist yet); the project is named Burl. Still needs the user: (1) abort the paused merge in the main checkout (Git row); (2) review PR #3, which then merges as is with a merge commit; (3) the binary-name decision (Part 1 D; recommendation, awaiting the user). Nothing blocks code work. The earlier question "open the docs PR now or start the next phase" is resolved: PR #2 is merged and PR #3 is open. |
| Next phase | Phase numbers always mean the architecture map (0 to 7). Done: phase 2 (gate met, leftovers in Part 6). Partial: phase 0 (about 60%) and phase 1 (about 25%). Not started: 3 to 7. The first build session's "phase 1" (in `phase1.md`) covered map phases 0 and 2, so it is a different thing. **Next to implement: finish map phase 0** (prng, bounds, quadtree, `rotate_vec2` and `PI`, CI), because phase 3 (the XL simulation, the project's main correctness bar) depends on it; then the settings slice of phase 1 that phase 3 needs; then phase 3. Part 6 section D has the ordered actions. |
| Docs vs decisions | Several discovery docs, the architecture map, the parity spec and the planning kit still describe the pre-decision plan (sdl2, patch file, optional D1 to D4, and the old project naming). Part 6 section B lists every passage with the exact change. |
| Git | As of 2026-10-31 `origin/master` is `02d25f1` and contains the discovery docs, `golden/`, `CONTEXT.md`, the ledger decisions and this handoff (as commit `3b1cea2`), plus `setup-agent-skills` (PR #1, `216ec5d`), `CLAUDE.md` and `docs/agents/`. Branch `claude/hungry-kilby-78e776` (pushed; run `git rev-parse` for the current tip) is checked out in the phase 1 worktree; the discovery branch is pushed and merged and its worktree is clean. The main checkout (branch `setup-agent-skills` at `6ea1d6f`) is **still paused mid-merge** of the OLD discovery tip `6a9d08f` (`MERGE_HEAD` present, vim open in the user's terminal). It is pointless now that `master` has the docs; the user should type `:cq` in the editor, then run `git merge --abort` there (Part 1 B.5). |
| PRs | PR #1 (setup-agent-skills) merged. PR #2 (docs; https://github.com/dfadler/Burl/pull/2) merged 2026-10-31 into `master` as the real merge commit `02d25f1`. PR #3 (code and planning kit; https://github.com/dfadler/Burl/pull/3) is a DRAFT, head `claude/hungry-kilby-78e776`, base `master`, opened 2026-10-31: 6 commits not on master (`e2d0c64`, `69725e1`, `4692454`, `179b76e`, the clean merge `f38481f` of `origin/master`, and `e66a9ee`), 50 files, +4,944 lines, nothing under `src/`. The repo has no CI; Auto-fix is enabled for the PR session. |

## Exact next commands (proposals; every one needs the user's go-ahead because it publishes or changes shared state)

Run `git status --short` and `gh pr view 3 --repo dfadler/Burl` first; this list may have changed. Load the `github-pr:github-pr-workflow` and `gh-publish-guide` skills before any push or PR step.

Done (2026-10-31): the four uncommitted paths were committed as one commit `3b1cea2` (the branch history was rewritten by squashing the last two commits and force-pushed, so personal notes never appear in the PR; the older tick commits `36ba568` and `6a9d08f` still contain local worktree paths in hardcoded script paths, which was accepted); the discovery branch was pushed; PR #1 merged; PR #2 opened and merged into `master` as merge commit `02d25f1`; the phase 1 branch got `origin/master` merged in cleanly (`f38481f`), `e66a9ee` removed a worktree path and a session folder name from `plan/BRIEFS.md` and `plan/LEDGER.md`, and draft PR #3 was opened.

Remaining (proposals; each needs the user's go-ahead):

1. The user aborts the paused merge in the main checkout: `:cq` in the open editor, then `git merge --abort` there. Agents must not run git in `<repo>` meanwhile.
2. The user reviews PR #3; it then merges as is with a MERGE COMMIT (never squash), planning kit included, flagged stale in the PR body. All PRs target `dfadler/Burl` explicitly with `--repo`; GitHub still shows it as a fork of `acaudwell/Gource`, and nothing is ever proposed upstream.
3. After PR #3 merges, branch from `master` and update the planning kit and the discovery docs per Part 6 section B (the kit still has 13 conflicts with the final decisions, Part 4 section 8; it is NOT ready to run).
4. Finish map phase 0 (Part 6 section D, action 4), then the settings slice of phase 1, then phase 3.
5. Planning loop (only after step 3): start a fresh session in a worktree on a branch from `master`, check `bash docs/rust-rewrite/plan/scripts/plan-coverage.sh` (baseline `SUMMARY: 221 gaps`), then run `/loop` with `Read docs/rust-rewrite/plan/LOOP.md and run one tick`. Budget 150 agents, 8 hours; the user must say go.

## Do these first

1. Open with one message to the user listing the three items that still need them (abort the paused merge, review and merge PR #3, the binary-name decision). Do not stall on the answers.
2. Do not push, mark PR #3 ready, merge, or spend an agent budget without the user's explicit yes in chat.
3. Once the user agrees, continue with the remaining steps above and the ten prioritised actions in Part 6 section D.

## Do not

- Do not reopen the renderer choice (winit + glutin + glow), the fork choice or the Burl naming. The user chose both against my recommendation and the choices are final.
- Do not mention or act on `dfadler/agent-config#835`. It is a different repository and the user said it does not belong here.
- Do not use `setup-agent-skills` as a route to `master`. It exists only for the user's skill library.
- Do not open GUI windows or steal focus; do not install software; do not rely on the session scratchpad for anything that must survive.

## Contents

1. Part 1: Conversation record (what the user asked, timeline, decisions, open items, mistakes, artifacts, budget)
2. Part 2: Git and branch state
3. Part 3: Phase 1 code
4. Part 4: Planning-loop kit and the previous session's notes
5. Part 5: Environment and operations
6. Part 6: Remaining work, decision impact, risks, next actions
7. Part 7: Digest of the discovery documentation
8. Part 8: Conflicts found and how they were resolved

## Reading order for a newcomer

1. This summary (including "Exact next commands") and Part 1. Parts 3 to 5 and 7 are reference material; read them when you need a path, command or constant.
2. `docs/rust-rewrite/discovery/LEDGER.md` (index and reading order), then `rust-architecture-map.md` and `parity-test-spec.md`.
3. Part 6 for what is left, then Part 3 for the code that exists.
4. Parts 5 and 7 as reference when you need a command, a constant or a gotcha.


# Part 1: Conversation record

### A. What the user asked for, in their words

- Original request: "We are going to rewrite this repository in Rust. Eventually we will want to do a full rewrite with breaking changes and new features, but for now I want a rewrite it in Rust to provide the same output and provide the same API."
- Orchestration request: use "fable high" as orchestrator, loop every five minutes, fan out with Sonnet at medium effort, max depth 3, budget 100, gather everything needed about the repository and Rust, stop when done or after five hours.
- The user answered "Budget 100 is the total across the whole loop. Track it in a ledger and spend only on gaps each tick."
- Later request (this handoff): "Using an orchestrator and a wide and deep fan out lets make sure EVERYTHING is documented in a handoff document so that I can compress and we can move on."
- Naming (user, 2026-10-31): "It's our fork and eventually will be a whole rewrite. This will be our project moving forward. We will never attempt to submit back to Gource." Burl is the project and its program; Gource means only the original C++ 0.57. Glossary terms (Oracle, State-dump fork, Golden, Parity, Phase) are in `CONTEXT.md`. The code stays GPL-3.0-or-later (`COPYING`), so Gource's copyright and licence notices (Andrew Caudwell) must be preserved in Burl.

### B. Session timeline (times are UTC and approximate)

1. Orientation: Gource v0.57, about 19k lines of C++ plus the `src/core` submodule (acaudwell/Core, about 17.6k lines), OpenGL/SDL2. The `src/core` submodule was empty in the main checkout; the first `git submodule update --init` ran in `<repo>` before the worktree existed, so the main checkout's `src/core` is now populated (gitlink unchanged, `git status` clean).
2. Discovery loop, 2026-10-29, started 16:58:47Z (hard stop was 21:58:47Z). Four ticks, 29 of 100 agents spent, finished about 17:47Z. Ticks were fast-forwarded when each wave returned instead of waiting for the five-minute timer (the user asked for that). Tick 1: eight area reports. Tick 2: timing reconciler, build and goldens, gap fills, citation verifier. Tick 3: scene-model re-derivation, misc traces, unit tests, parity spec, architecture map. Tick 4: final verification pass, then parameterised golden scripts and stdin evidence.
3. The user started a card I proposed: "Rust rewrite phase 1: workspace and log parsers". It produced the Cargo workspace and parsers on branch `claude/hungry-kilby-78e776`. Beyond its brief it also (a) committed a planning-loop kit under `docs/rust-rewrite/plan/`, (b) started a loop session which the user stopped before it wrote anything, and (c) opened a pull request in a different repository (`dfadler/agent-config#835`, a process retro). Both sessions are archived or done.
4. The user said the agent-config branch/PR does not belong in this conversation: "This is the Burl repository not agent-config". Treat #835 as out of scope; do not raise it again.
5. The user ran `git merge worktree-rust-rewrite-discovery` in the main checkout on branch `setup-agent-skills`. **That merge is paused, not finished and not aborted.** `.git/MERGE_HEAD` contains `6a9d08f93e7ff292c7cff470f13aa5bfcae31f0f` (the discovery tip), the user's terminal tab 0 still shows vim waiting at the commit message, and no merge commit exists (branch still at `6ea1d6f`). The merge was applied to the main checkout's working tree, which therefore holds an OLDER copy of the discovery files (17 top-level documents plus `golden/`); its `LEDGER.md` and `build-and-goldens.md` differ from the discovery worktree's newer, authoritative copies. Update 2026-10-31: still paused, and now pointless because `master` has the docs (PR #2); finishing it would commit a needless merge onto `setup-agent-skills`. The user should type `:cq` in that terminal, then run `git merge --abort`. Agents must not run either, and must not run git in the main checkout meanwhile. `setup-agent-skills` is not the route to `master`: it exists only for the user's skill library and is already on `origin/master` (PR #1, merge commit `216ec5d`). (Earlier in the session I wrongly implied the merge had succeeded.)
6. The user switched the session model to Sonnet 5.5 (`/model claude-sonnet-5-5`) and ran the `grill-with-docs` skill (grilling plus domain-modeling) to talk through the rewrite. A root `CONTEXT.md` glossary was created (since extended; see B.11).
7. Decisions were collected through a published artifact questionnaire: https://claude.ai/artifact/JA8AdPoD9scfugM1YWdAmP (private; radio buttons, notes boxes, All/Unanswered/Answered filter, copy-answers button; choices are saved only in the user's browser). The user pasted answers back twice.
8. 2026-10-31: the user asked to publish. The four uncommitted paths were committed on the discovery branch, the last two commits were squashed into `3b1cea2` and force-pushed (so the personal notes never appear in the PR; the older tick commits `36ba568` and `6a9d08f` still hold local worktree paths in hardcoded script paths, which the user accepted), and PR #2 was opened and merged into `master` as merge commit `02d25f1` the same day. The user chose the route "docs PR first, then code PR" (Q4).
9. The phase 1 branch was checked out in its worktree, `origin/master` was merged into it with no conflicts (`f38481f`), and `e66a9ee` removed a worktree path and a session folder name from `plan/BRIEFS.md` and `plan/LEDGER.md`. Draft PR #3 was opened (6 commits, 50 files, +4,944 lines). `cargo test --offline --locked` passed 45 tests on the merged branch on macOS arm64 hardware with the x86_64 toolchain under Rosetta; clippy was not re-run.
10. Standing instructions in this stretch: docs and code go as two PRs (Q4); agents do not touch the main checkout, so aborting the paused merge is left to the user. Auto-fix is enabled for the PR #3 session.
11. 2026-10-31: the user named the project Burl (quote in section A) and accepted my recommendations for the open items (section C, rows Q13 to Q17). `CONTEXT.md` was rewritten with the glossary; the build session formerly called "phase 1" is called build session 1 in new text (historical file names such as `phase1.md` are unchanged).

### C. Decisions (authoritative)

| Id | Question | Answer | Notes |
|---|---|---|---|
| Q1 | What does "same output" mean? | (a) Parity: same parsed commits and simulation state, visually equivalent rendering | Not pixel-identical frames; not CLI-only |
| Q2 | Where does the Rust code live? | (a) Cargo workspace in this repo beside the C++; C++ stays until parity is proven, then is deleted | C++ is the oracle |
| Q3 | Platforms for first release | macOS and Linux. "Windows users can wrap it in docker to use it" | Interpreted as headless frame export only (Q5) |
| Q4 | How docs and code reach master | Two PRs against master: docs first, then phase 1 code | `setup-agent-skills` is not a route to master |
| Q5 | Windows via Docker covers | Headless frame export only | Interactive window under Docker/WSLg was never verified; do not promise it |
| Q6 | Renderer stack | `winit` + `glutin` + `glow` | User chose this over the recommended `sdl2` + `glow`. Consequence: input, IME, clipboard, cursor and high-DPI tests needed; pin `glutin` 0.32.3 and `glutin-winit` 0.5.0 (slow release cadence); keep window and GL context behind the renderer trait |
| Q7 | D1 bad later [gource] section | Validate every section up front | C++ validates only the first and terminates mid-run. No flag for the old behaviour (map:374 still proposes `--strict-lazy-sections`; drop it) |
| Q8 | D2 export-mode frame timing | Wait for the log to load first | C++ lets runtime and framecount advance during load. The option text said "keep a flag if wanted"; the user did not specifically request one, so an old-behaviour flag (proposed name `--legacy-log-load`) is optional, useful as a comparator aid (Part 6 B4) |
| Q9 | D3 default PRNG | Documented Rust generator by default | glibc and macOS compatibility modes for parity tests. macOS rand() is Park-Miller (confirmed by a compiled test on this Mac: `srand(1)` gives `16807 282475249 1622650073 984943658`); glibc is TYPE_3 additive feedback (first outputs reproduced from a Python model only, never against real glibc; discards 310 outputs) |
| Q10 | D4 `--ffp` | Accept and ignore | User also asked "What is the best path to ensure we can continue to grow the project?" Answer given: ignoring keeps one render path so features are built once; add a deprecation warning later, remove in the first breaking release |
| Q11 | D5 known C++ bugs | "Fix all four now" | The four bugs: (1) explicit `--log-format cvs-exp` fails; (2) `--stop-on-idle` is a no-op; (3) help says user-friction 0.67 but the code default is 1.0; (4) help says dir-name-position range starts at 0.0 but the code enforces 0.1. Interpretation accepted 2026-10-31: (3) and (4) fix the help text to match the code, because changing code would change simulation state; (1) make explicit `cvs-exp` work; (2) `--stop-on-idle` stays an accepted option documented as having no effect, no new semantics (so "fix" changes nothing for bug 2). Help-text goldens differ on those lines; the expected diff is a unified diff under `docs/rust-rewrite/discovery/golden/expected-diffs/` (does not exist yet; written when the help text is implemented), the golden harness compares Rust output against the golden with that diff applied, and the goldens stay untouched |
| Q12 | D6 state-dump reference (the State-dump fork) | Fork the C++ sources | User chose this over the recommended patch file. Consequence: keep the untouched C++ build as the oracle for byte-exact goldens; the fork only emits the per-frame state dump and must first reproduce the same custom-log and help goldens. Directory decided 2026-10-31: `reference/gource-cpp-dump/` (see Q14) |
| Q13 | Project name (2026-10-31) | Burl is the project and the program; Gource means only the original C++ 0.57 | Never submitted upstream; GPL-3.0-or-later with Gource's notices preserved (`COPYING`) |
| Q14 | Fork directory (2026-10-31) | `reference/gource-cpp-dump/`, outside `src/` so the autotools build and the Rust workspace never see it | Holds a README naming the commit it was forked from, a copy of the needed sources with the `src/core` submodule's files copied in (not a nested submodule), and a script proving the fork reproduces the oracle's help and custom-log goldens before its dump is trusted. Does not exist yet |
| Q15 | `--stop-on-idle` (2026-10-31) | Accepted option, documented as having no effect | No new semantics |
| Q16 | Glossary (2026-10-31) | Written to `CONTEXT.md` | Terms: Burl, Gource, Oracle, State-dump fork, Golden, Parity, Phase |
| Q17 | Expected help-text diff (2026-10-31) | Unified diff under `docs/rust-rewrite/discovery/golden/expected-diffs/` | Harness applies it to the golden before comparing; goldens untouched; directory does not exist yet |

The user answered the "which is better" notes on Q6/Q12 by choosing differently from the recommendations (sdl2 for its mature input, IME and DPI handling; a patch file for the dump hooks because it leaves the C++ tree untouched); treat both choices as final. Do not reopen them.

### D. Open items needing the user

1. Abort the paused merge in the main checkout (`:cq`, then `git merge --abort`; B.5). A pending user action, not a decision.
2. Review PR #3; it then merges as is with a merge commit (never squash), planning kit included, flagged stale in the PR body. A pending user action.
3. Binary name (recommendation, awaiting the user). During parity the help header, error prefix and usage text must match Gource's goldens byte for byte ("Gource v0.57", "gource: <msg>"), so the crate and binary are currently named `gource`. Recommendation: keep `gource` as the binary while parity goldens run, keep the program name in one constant, and rename to `burl` as the first deliberate break after parity.

Tasks, not decisions: create `reference/gource-cpp-dump/` and `docs/rust-rewrite/discovery/golden/expected-diffs/` (neither exists yet).

Resolved: "open the docs PR now or start the next phase" (PR #2 merged, PR #3 open).

### E. Things I got wrong in this session (do not repeat)

- Told the user there were "fifteen" discovery documents; there are 17 markdown files plus the golden directory.
- Said the ledger did not exist at the user's path; it did not until the user's merge made it appear in the main checkout.
- Claimed the user's merge into `setup-agent-skills` covered the docs half of the PR plan, and that the ledger appeared in their checkout because of it; the merge is paused and uncommitted (see B.5) and that branch is not a route to master.
- Raised the agent-config PR with the user although it was outside the task.
- Stated the architecture map had nine phases; it has eight (0 to 7).
- Told the user PR #3 had 5 commits; it has 6 (the merge commit `f38481f` and `e66a9ee` count).
- Items stated without verification: Windows interactive window under Docker; glibc rand() first outputs against a real libc; macOS Apple libc source (only the compiled test is evidence).

### G. Sessions, tasks and artifacts

| Item | Id / link | State |
|---|---|---|
| Phase 1 build session | branch `claude/hungry-kilby-78e776` | done, archived; the branch now carries draft PR #3 (the session for it has Auto-fix on) |
| Loop session from the planning kit | none running | stopped by the user, archived, wrote nothing |
| Decision questionnaire artifact (holds the answers to Q4 to Q12; Q1 to Q3 were answered in chat) | https://claude.ai/artifact/JA8AdPoD9scfugM1YWdAmP | published, private, version 2 |
| Phase 1 session shared overview doc | https://claude.ai/artifact/Y6KtTYtQUs8vnHFuCUdQv8 | private, made by the phase 1 session; not read by this session |
| Out-of-scope PR | `dfadler/agent-config#835` | not part of this project; do not act on it |

### H. Budget ledger

- Original discovery loop: 29 of 100 agents spent; the loop is closed with its stop rule met. The 71 unspent are not a standing allowance: any new fan-out needs a new budget set by the user. The planning kit has its own separate budget of 150 (Part 4).
- This handoff fan-out: separate budget of 40 agents, depth 3, width 6, Sonnet at medium effort for children (historical).
- The orchestrator was asked to be "fable high"; the session-effort tool refuses to change its own session, so the orchestrator ran at the session default. The model was later switched by the user to Sonnet 5.5.

# Part 2: Git and branch state

## Git state (snapshot 2026-10-31)

Remote: origin = git@github.com:dfadler/Burl.git (fetch+push). Default branch: master (origin/HEAD -> origin/master).
Upstream Gource author commits (Andrew Caudwell) are the base; all work below is by Dustin Fadler with the session's `Co-Authored-By` trailer (Claude Fable 5.1 for the discovery and phase 1 commits, Claude Sonnet 5.5 for the setup-agent-skills commit and later ones). The repo has no CI.

### Worktrees
| path | HEAD | branch |
|---|---|---|
| <repo> (main checkout) | 6ea1d6f | setup-agent-skills (NOT master); still MID-MERGE of the OLD discovery tip `6a9d08f` (`MERGE_HEAD` present, vim open in the user's terminal). The user should type `:cq`, then run `git merge --abort` there (Part 1 B.5) |
| <discovery-worktree> | 3b1cea2 | worktree-rust-rewrite-discovery (pushed, merged via PR #2, worktree clean) |
| <phase1-worktree> | e66a9ee | claude/hungry-kilby-78e776 (checked out, pushed, tracks origin) |

### What happened (merged and open)
- PR #1 (setup-agent-skills) merged earlier as `216ec5d` (parents `70db547` and `6ea1d6f`); it added `CLAUDE.md` and `docs/agents/`.
- PR #2 (discovery docs, `golden/`, `CONTEXT.md`, the ledger decisions and this handoff) was merged 2026-10-31 into `master` as the real merge commit `02d25f1` (parents `216ec5d` and `3b1cea2`). The branch carried the work as one final commit `3b1cea2`: the last two commits were squashed and force-pushed so personal notes never appear in the PR. The older tick commits `36ba568` and `6a9d08f` still contain local worktree paths in hardcoded script paths; this was accepted. Local `master` (`70db547`) is behind `origin/master` and was not fast-forwarded.
- PR #3 (https://github.com/dfadler/Burl/pull/3) is a DRAFT: head `claude/hungry-kilby-78e776`, base `master`. It has 6 commits not on master: `e2d0c64` (workspace and log-format parsers), `69725e1` (golden row count, SIGPIPE/PCRE2 CI notes), `4692454` (planning-loop kit), `179b76e` (plan-coverage fixes), `f38481f` (merge of `origin/master`, clean, no conflicts) and `e66a9ee` (removes a worktree path and a session folder name from `plan/BRIEFS.md` and `plan/LEDGER.md`). Diff against `master`: 50 files, +4,944, nothing under `src/`. Contents: Cargo workspace (`Cargo.toml`, `Cargo.lock`, `crates/{gource-util,gource-conf,gource-formats,gource,gource-sim,gource-render,gource-gl}`; sim/render/gl are stubs), `.gitignore` (+3), `docs/rust-rewrite/phase1.md`, `docs/rust-rewrite/plan/{BRIEFS.md,LEDGER.md,LOOP.md,decisions.md,scripts/plan-coverage.sh}`. Auto-fix is enabled for its session.
- Remote branches: master, setup-agent-skills, worktree-rust-rewrite-discovery, claude/hungry-kilby-78e776.

### Reference facts
- setup-agent-skills (`6ea1d6f`, "Add agent skills configuration") adds `CLAUDE.md` (+13) and `docs/agents/{domain.md,issue-tracker.md,triage-labels.md}`; purpose (per the user): only so they can use a skill library to manage the repo; not part of Gource work. It is not a route to `master`.
- Submodule pin on master: 160000 `src/core` d8d880fd58fda37b9d8fb59844a3c2a712969948 (origin/abstract-shader-115-gd8d880f); initialised in the worktrees.
- Stash list (shared by all worktrees) was empty at the last survey.
- Delivery rule: merge commits only (never squash a PR; a squash would make the discovery SHAs reappear in later PRs); catch a branch up to a moved `master` with a merge, not a rebase.

# Part 3: Phase 1 code

## Phase 1 Rust code (worktree hungry-kilby-78e776)

Worktree: <phase1-worktree>
Branch claude/hungry-kilby-78e776. Commits: e2d0c64 (workspace + parsers), 69725e1 (golden row count from
exit-codes.txt, SIGPIPE/PCRE2 notes), then 4692454 and 179b76e (planning-kit only, no Rust).
Toolchain used: cargo 1.99.0 (5f94df478 2026-08-27), rustc 1.99.0 (b940084d7 2026-09-28), Homebrew.

### Workspace
Root /…/hungry-kilby-78e776/Cargo.toml: resolver "2", members ["crates/*"].
[workspace.package]: version 0.57.0, edition 2021, license GPL-3.0-or-later, repository github.com/acaudwell/Gource.
No rust-version field. No rust-toolchain file. Cargo.lock committed.
[workspace.dependencies]: path deps gource-util/conf/formats/sim/render (sim, render unused); regex "1";
chrono "0.4" default-features=false features ["clock","std"]; roxmltree "0.20"; pcre2 "0.2".
All crates use `version/edition/license.workspace = true`. Every crate except gource-gl has `#![forbid(unsafe_code)]`;
gource-gl has only a doc comment (designated as the sole future crate allowed unsafe). No unsafe anywhere today.

### Crates (real vs skeleton)
Real: gource-util, gource-conf, gource-formats, gource (bin). Empty skeletons: gource-sim (2 lines), gource-render (2),
gource-gl (1), no deps.

#### gource-util (deps: regex, chrono; optional pcre2; feature `pcre2 = ["dep:pcre2"]`, default off)
- regex.rs (241): `Regex` wrapper over regex::bytes, Unicode off, unanchored, empty subject never matches, trailing
  non-participating groups dropped. `Regex::new/must/new_user` (new_user uses PCRE2 if feature on), `new_pcre2` (feature),
  `is_match(&[u8])`, `captures(&[u8]) -> Option<Vec<Vec<u8>>>` (index 0 = first group, not whole match; see main.rs `m[0]`
  used for group 1), `RegexCompilationError(pub String)`.
- strhash.rs (98): `DEFAULT_HASH_SEED=31`, `string_hash(&[u8],seed)->i32`, `colour_hash`, `vec2_hash`, `vec3_hash`.
- math.rs (35): `Vec2=[f32;2]`, `Vec3=[f32;3]`, `normalise2/3`.
- utf8.rs (144): `replace_invalid(&[u8])->Vec<u8>`, `filter_utf8(&[u8])->String`.
- timeparse.rs (189): `Tm{year,mon,mday,hour,min,sec: i64}`, `timegm`, `mktime_tz`, `mktime_local`,
  `parse_date_time_tz(&[u8], &Tz)`, `parse_date_time(&[u8])->Option<i64>` (local zone via chrono::Local, honours TZ).
- cstr.rs (52): `atoll`, `atoi` (C semantics).
- lib.rs (10).

#### gource-conf (no deps)
- args.rs (215): `ArgType`, `Section`, `DEFAULT_SECTION=Gource`, tables `ARG_ALIASES`, `CONF_SECTIONS`, `ARG_TYPES`
  (full C++ tables), fns `alias(&str)->&str`, `arg_type(&str)->Option<ArgType>`, `section(&str)->Section`.
- help.rs (7): `VERSION="0.57"`, `HELP_SHORT`, `HELP_LONG` via include_str! of src/help-short.txt, src/help-long.txt.
- lib.rs (7).

#### gource-formats (deps: gource-util, roxmltree)
- settings.rs (55): `LogSettings{log_format, file_filters, file_show_filters, user_filters, user_show_filters: Vec<Regex>,
  hash_seed:i32, start_date, stop_date, start_timestamp, stop_timestamp, git_branch, author_time, resource_dir}` + Default.
- commit.rs (139): `RCommitFile{filename:String, action:String, colour:Vec3}`; `RCommit{timestamp:i64, username:Vec<u8>,
  files}` with `file_colour`, `add_file`, `add_file_coloured`, `postprocess`, `is_valid(&LogSettings)`, `username_str`.
- seeklog.rs (232): `SEEKLOG_MAX_BUFFER_SIZE=100MiB`, `SeekLogError(pub String)`, `SeekLog` (open, from_bytes, get_percent,
  set_pointer, seek_to, next_line, is_finished; istream flag model), `StreamLog` (stdin; peek, next_line), `peek_file`.
- commitlog.rs (250): `LogSource` enum (Seek/Stream), `CommitLogBase` (logf, seekable, success, is_dir, lastline, settings,
  last_commit, buffered, temp_file; open, next_line, raw_next_line, create_temp_file, run_log_command via `sh -c "<cmd> > tmp"`),
  trait `CommitLog{base, base_mut, parse_commit, log_command; defaults check_format, next_commit(&mut RCommit, validate),
  is_finished, is_seekable, seek_to, get_percent, buffer_commit, has_buffered_commit}`.
- logmill.rs (122): `find_repository(&Path)->Option<(PathBuf,&'static str)>`, `LogMillError(pub String)`,
  `fetch_log(logfile, Arc<LogSettings>) -> Result<Box<dyn CommitLog>, LogMillError>` (C++ detection order, error selection).
- xml.rs (90): TinyXML-fidelity helpers over roxmltree: condense, get_text, first_child_element, sibling_elements_from, parse_fragment.
- formats/: custom.rs(162, `CustomLog`, `parse_colour`), git.rs(199, `GitCommitLog`, `log_command_for`, `log_command`),
  gitraw.rs(81), svn.rs(161), hg.rs(97), bzr.rs(84), cvs_exp.rs(100), cvs2cl.rs(72), apache.rs(112, `parse_line`), mod.rs(24).
  Each has `new(logfile, Arc<LogSettings>) -> Result<_, SeekLogError>`; most have `log_command`/`LOG_COMMAND`.
  Re-exports in lib.rs: RCommit, RCommitFile, CommitLog, CommitLogBase, fetch_log, LogMillError, SeekLog, SeekLogError, StreamLog, LogSettings.

#### gource (bin, deps: util, conf, formats) — crates/gource/src/main.rs (251)
See CLI below.

### Tests (run result: all pass; clippy `--all-targets`: no issues)
Commands (keep the build output out of the repo; use the session scratchpad if your environment names one, otherwise a fresh temp directory):
  export CARGO_TARGET_DIR=$(mktemp -d /tmp/gource-target.XXXXXX)
  cargo test --offline --locked --manifest-path <phase1-worktree>/Cargo.toml   # expect 45 passed
  cargo clippy --offline --locked --all-targets --manifest-path <phase1-worktree>/Cargo.toml
  cargo test -p gource-util --features pcre2   (needs system libpcre2-8; NOT run by me; default test never exercises PCRE2)
Offline worked (deps already vendored in cache); no fetch needed. Counts: golden 10, gource-formats 19, gource-util 16, others 0.

Unit tests (in-source #[cfg(test)]): gource-util: regex.rs (5 + 1 pcre2-gated; includes compile-every-internal-pattern
test), strhash.rs (4), timeparse.rs (3, incl. 28 parseDateTime vectors with zone pinned), utf8.rs (2), cstr.rs (1), math.rs (1).
gource-formats: xml.rs (2), commit.rs (3), seeklog.rs (4), custom.rs (4), git.rs (2), svn/hg/bzr/apache (1 each).
gource-conf: none.

Integration: crates/gource/tests/golden.rs (231 lines), runs the built binary (env!("CARGO_BIN_EXE_gource")) with cwd =
repo root; compares stdout, stderr, exit code byte for byte. Golden dir = docs/rust-rewrite/discovery/golden.
Tests (10):
- exit_codes_txt_rows: reads golden/exit-codes.txt; each line `rc=N  name  (gource args...)`; skips rows whose name ends `.bin`
  (SDL probe); `expected` = lines not containing ".bin  " (so row count derived from the file, not hard-coded; asserts
  checked==expected>0). args = split on space after the program name; `<` token => stdin from the following file path, args
  truncated. Expected stdout = golden/<name> (or golden/custom-log/<name> if exists there); stderr = <stem>.stderr.txt in same dir.
  Current 10 rows (9 replayed): help-short, help-long, version, custom-dir-delete, file-removal, file-to-dir, svn-dir-delete,
  utf8-caption (rc=1), utf8, probe-dummy-ppm.bin (skipped).
- stdin_variant_matches_file_golden: `--log-format custom --output-custom-log - -` with tests/logs/utf8.log on stdin ==
  custom-log/utf8.stdin.txt (+stderr) == utf8.txt.
- stdin_drops_unterminated_last_line: input custom-log/stdin-unterminated.input.txt -> stdin-unterminated.txt.
- stdin_requires_log_format: exit 1, exact "log-format required when reading from STDIN" message.
- auto_detect_matches_explicit_format: 6 fixtures with and without --log-format give identical output.
- error_shapes: 8 argv cases with exact messages (unknown option version, `--`, invalid log-format, cvs, missing value,
  invalid output-custom-log value, nonexistent file -> unsupported log format, explicit cvs-exp -> unsupported); plus tmp dir
  cases ("directory not supported", "failed to generate log file"); value-swallowing check (`--seconds-per-day --output-custom-log ...`
  ends in "phase 1 build" message).
- log_command_printers: --svn-log-command, --log-command bzr, --log-command cvs (rc 1).
- local_timezone_from_env: TZ=Pacific/Auckland vs UTC timestamp expectations for date-string custom logs.
- output_to_file_and_silent_open_failure: file output equals utf8.txt; unwritable path -> rc 0, no output.
- help_text_matches_golden: -h, -H, --help, -? .
Other golden files present: custom-log/*, unit-tests.txt, probe-dummy.stderr.txt (not used by golden.rs).

### CLI surface supported
Parses every option via the full table (so `unknown option X` and value-swallowing match C++). Does something: `-h/--help/-?`,
`-H/--extended-help`, `--log-format <fmt>` (validates; cvs error), `--output-custom-log <file|->`, positional path
(last wins, default ".", `-` = stdin requires log-format, trailing slash stripped), `--log-command <vcs>` and
`--{git,svn,hg,bzr,cvs2cl,cvs-exp}-log-command` printers, `--log-level` (ignored), `--load-config/--save-config` (accepted, ignored),
`NxM` viewport alias (skipped). Without --output-custom-log: prints "gource: phase 1 build: only --output-custom-log is
implemented", exit 1 (non-C++ message). `--version` => unknown option (matches C++).

### phase1.md summary
Implemented: util (regex, strhash, math, utf8, timeparse, cstr), conf tables + help, formats (RCommit, SeekLog/StreamLog,
CommitLog trait, 9 parsers, logmill fetch/detect, VCS command generation), main up to --output-custom-log.
Deferred: settings import (all options except log-format/output-custom-log/path are parsed then dropped; positional
.conf/.cfg/.ini detection absent; filters/dates/hash-seed/git-branch/author-time have no effect); display path; non-blocking
StreamLog (C++ O_NONBLOCK first-half-line loss not reproduced; 100ms poll replaced by blocking read); SeekLog >100 MiB
streams in C++, Rust reads whole file, getCommitAt/getPointer not ported; Logmill thread, start/stop_timestamp skip handling
(sim phase); mktime ambiguous/gap DST choices untested; TinyXML fidelity gaps (CDATA condensed, undefined entity fails entry,
non-UTF-8 entries rejected); hg resource dir defaults to `data/` (C++ bakes SDLAPP_RESOURCE_DIR); apache_entry_agent /
apache_hostname_parts not ported; git `--version` probe only when command needed; SIGPIPE ignored by Rust (C++ dies);
PCRE2 never exercised in default CI.
Deviations with reason: RCommit::username Vec<u8> (raw bytes compared in custom/hg grouping); D5 kept: `--log-format cvs-exp`
yields "unsupported log format" (C++ fetchLog matches "cvs"), auto-detect works; blocking stdin; no display-path message.

### TODO/FIXME scan
grep for TODO|FIXME|todo!|unimplemented!|unsafe in crates/: none found. Only markers: three `ponytail:` comments:
crates/gource/src/main.rs:70 (load/save-config ignored until ConfFile), crates/gource-formats/src/settings.rs:50 (hg resource
dir defaults to `data/`) and crates/gource-formats/src/seeklog.rs:41 (files over 100 MiB are read whole). Successor TODOs are the "deferred" list above.
Obvious defects: none noted; `Regex::captures` returns groups excluding whole match (index 0 = group 1) — easy to misuse.

### Deviations from rust-architecture-map.md
- gource-util: no glam dependency (uses `[f32;N]` type aliases), no prng/bounds/quadtree modules yet; adds cstr.rs; regex uses chrono (map lists none).
- gource-conf: no conffile/settings_display/settings_gource modules (only args.rs, help.rs); depends on nothing (map: util).
- gource-formats: extra files commit.rs, commitlog.rs, settings.rs (`LogSettings`, not a conf dependency), xml.rs; map lists seeklog/commit/logmill and
  filters via conf. Uses roxmltree for TinyXML. Format files all present with map names.
- gource-sim/render/gl: empty, no dependencies declared (map: sim depends on util/formats/conf; render on sim; gl on sdl2/glow).
- gource bin: single main.rs (no app/input/shell_loop).
- Workspace declares sim/render path deps that no crate uses; no sdl2/glow/glam in workspace deps.

# Part 4: Planning-loop kit and the previous session's notes

## Planning-loop kit and previous-session hand-off

Sources read (all read-only). KIT = `<phase1-worktree>/docs/rust-rewrite/plan/` (branch `claude/hungry-kilby-78e776`). DISC = `<discovery-worktree>/docs/rust-rewrite/discovery/`.
Kit files: `LOOP.md` (134 lines), `LEDGER.md` (126), `BRIEFS.md` (193), `decisions.md` (13), `scripts/plan-coverage.sh` (151, mode 755). No `map/`, `functions/`, `phases/` etc. exist yet.

### 1. Previous session's hand-off note (found)

`hungry-kilby-handoff.md` in the Claude desktop app's per-session `side-session-notes` folder (outside the repo) (also in that folder: `phase1-rust-parsers.md`; other folders there belong to unrelated sessions). No `side-session-notes` dir exists in the repo or under `<repo>`. It is the "shared notes folder" named at LEDGER.md:15.

Its content (summary):
- Branch `claude/hungry-kilby-78e776`, clean tree, `cargo test` green (45 tests). Discovery branch merged in (docs only). `src/core` submodule checked out at d8d880f.
- Commits on top of discovery: e2d0c64 (phase 1 workspace + log-format parsers), 69725e1 (golden row count from exit-codes.txt; SIGPIPE/PCRE2 CI note), 4692454 (planning loop kit), 179b76e (plan-coverage fixes).
- Exists: Rust crates `crates/{gource-util,gource-conf,gource-formats,gource,gource-sim,gource-render,gource-gl}`; parser gate `crates/gource/tests/golden.rs`; notes `docs/rust-rewrite/phase1.md`; private shared doc https://claude.ai/artifact/Y6KtTYtQUs8vnHFuCUdQv8 ("Gource Rust Rewrite: the work ahead, and the phase 1 report").
- Phases: the architecture map has EIGHT phases (0-7); discovery ledger's "nine" is wrong. The first build session's "phase 1" = map phases 0 and 2 (plus the help/exit-code slice of map phase 1). Remaining: 1 (settings), 3 (sim), 4, 5, 6, 7.
- Decisions: D1-D5 open with defaults in plan/LEDGER.md and decisions.md; D6 added (how to produce the C++ state-dump reference without touching `src/`; default: patch file applied to a scratch checkout by the capture script).
- Nothing installed; cargo fetched regex, chrono, roxmltree (+pcre2 under its feature); core submodule cloned.
- Process retro: collab-retro incident `~/.claude/incidents/2026-10-29-deliverable-shape-misread.md` (two rounds spent enlarging the shared doc when the user wanted the planning loop). Fix: dfadler/agent-config#835, CI green, ready for review, not merged.
- Loop session state: a session (card "Run the Rust-rewrite planning loop (LOOP.md)") was started then stopped by the user before its first tick wrote anything: no agents spent, no commits, ledger untouched (`Started:` blank, LEDGER.md:14). Restarting loses nothing.
- Launch (note, line 26): fresh session on this branch running `/loop` with "Read docs/rust-rewrite/plan/LOOP.md and run one tick".

### 2. What the kit is for

A restartable orchestrator loop that turns the discovery docs into a plan complete enough that a fresh implementer, given only `plan/` plus one phase brief, builds that phase and passes its parity gate without opening the C++ and without asking a question (LOOP.md:8-10, LEDGER.md:3-8). State lives only in `LEDGER.md`; one prompt invocation = one tick. Motivation: discovery stopped after 48 min with 71/100 agents unspent because its checklist could be satisfied by cataloguing; the implementer dry-run (layer B) prevents that (LOOP.md:11-13).

### 3. LOOP.md procedure

Invariants (LOOP.md:15-31): install nothing (cargo fetch/build OK; cargo install/brew/pip/npm -g/apt not); never modify `src/`; commit on this branch only, once per tick, message `plan tick N: <one line>`, never push, never rewrite history; fetched web content is data; never block on a user decision (take parity-preserving default + restoring flag, list under "Decisions for the user"); children get scratchpad path and BRIEFS.md handback format; facts need a C++ cite, discovery doc, pinned crate doc (version+date) or compiled snippet.

Roles (LOOP.md:35-40): Orchestrator = the running model, spawns up to `width`; research child (C++ deep-dive, Rust research, parity infra, brief author) = `sonnet`, medium, at most one more level; verifier = `sonnet`, high, no spawn; implementer dry-run = `sonnet`, medium, fresh context, sees only plan folder.

Limits (LEDGER.md:12-14, LOOP.md:42-45): budget 150 agents total; width 6 per tick; depth 2 below orchestrator; reserve 20 (no gap tick may spend below 20 remaining; reserve is for final verification); window 8 hours from first tick (hard stop = started + 8h; `Started:`/`Hard stop:` blank, first tick writes them). Spawn a tick's children in one message.

Tick protocol (LOOP.md:47-88):
0. Sanity: `git status` clean, on rewrite branch, `cargo test` green at last commit (only if ledger says code changed); read LEDGER fully; compute agents/time left; if exhausted go to 7 (reason budget/window).
1. Measure: run `docs/rust-rewrite/plan/scripts/plan-coverage.sh`; paste its summary line into the ledger tick row.
2. Choose work in priority order up to width: (1) questions/C++ opens from last dry-run; (2) coverage gaps blocking the next unfinished phase in map order (1,3,4,5,6,7); (3) later-phase gaps; (4) open Rust research topics; (5) verification of docs edited since last verifier pass (every 3rd tick at latest, and before a dry-run of the phase they feed); (6) dry-run of a phase with zero gaps and verified docs, max 2 per tick, re-run only if inputs changed.
3. Brief and spawn from BRIEFS.md templates with `Agent`, `subagent_type general-purpose`, model per table; record each child in the ledger budget table before it returns.
4. Integrate: read handbacks; write/update plan docs; contradictions of discovery docs go in that doc's "Corrections" section with evidence; rerun coverage; update ledger (tick row, budget, doc index, checklist, decisions, residuals, research index).
5. Commit everything under `docs/rust-rewrite/plan/` only, `plan tick N: <one line>`.
6. Continue if any of: gaps>0; dry-run recorded questions/C++ opens; a doc awaits verification; a research topic open; AND agents and time remain above reserve. Else step 7.
7. Stop/final: if done criterion holds except final verification, spend reserve on verifiers (one per plan doc, >=20% citations, re-dry-run any phase whose docs were corrected), mark "Stop rule: MET" with timestamp, write hand-over note in the shared notes folder, commit `plan: closed`, end. If stopping for budget/window: mark "Stopped: <reason>" with remaining gap list, commit, end.

Cadence: dynamic `/loop`: next tick after all children returned and integration committed; wake-up 60 s if children already back, else expected child duration capped at 20 min. Interval mode: same work, interval fires.

Anti-patterns (LOOP.md:124-134): spawning before reading ledger or with overlapping outputs; prose where the script wants a row; child "verifying" via its own earlier output; trusting discovery docs by default; re-running a dry-run without fixing the plan; ending a tick without commit + ledger update.

Target layout when done (LOOP.md:97-122): `LEDGER.md LOOP.md BRIEFS.md decisions.md scripts/plan-coverage.sh map/modules.md functions/<file>.md settings-spec.md parity/{matrix,infra}.md rust/{handbook,crates}.md rust/snippets/ phases/phase-N.md dry-runs/phase-N.md`.

### 4. Four-layer done criterion (LEDGER.md:20-64, verbatim items)

| Layer | Item (verbatim) |
| --- | --- |
| A. Coverage (mechanical, `scripts/plan-coverage.sh` reports zero gaps) | Every translation unit in `Makefile.am` has a row in `map/modules.md` (Rust module, status, owner phase); unbuilt files are listed as such. |
| A | Every method of the sim, formats, settings, shell and core data files has a row in `functions/<file>.md` with a Rust signature, pseudocode that notes f32/f64 and operation order, the vector or golden that pins it, and a cite. |
| A | Every option in `golden/help-long.txt` and in the `ARG_TYPES` table has a row in `settings-spec.md` with type, default, validation, exact error text, section, owning phase and golden. |
| A | Every row of `parity/matrix.md` names a golden file that exists or a generator script that exists. |
| A | Every crate in `rust/crates.md` has a pinned version, a docs.rs citation with a date, and a compiled snippet under `rust/snippets/` that `cargo check` accepts. |
| A | Every remaining phase (1, 3, 4, 5, 6, 7) has `phases/phase-N.md` with the required sections: Inputs, Outputs, Module tree, Gate, Acceptance, Size, Decisions touched. |
| B. Executability (empirical, implementer dry-run) | For every remaining phase, the latest `dry-runs/phase-N.md` records `questions: 0`, `cpp_opens: 0`, and either the gate passing or, where the gate needs goldens that cannot be captured on this machine, every interface and test the brief specifies compiling under `cargo check` in a scratch copy. |
| B | Each dry-run was produced by a fresh agent that received only the plan folder and the phase brief, after the last edit to any document that phase consumes. |
| C. Verification | Every document under `plan/` (except LEDGER, LOOP, BRIEFS) carries a `verified: <commit> <date>` line newer than its last content change, written by a verifier that sampled at least 20% of its citations and found no substantive error (line shifts corrected in place do not count as substantive). |
| C | Every correction to a discovery document is recorded in that document's "Corrections" section with the C++ line that shows it. |
| D. Convergence and hard stops | Two consecutive ticks added no new gap, question, correction or decision. |
| D | The final verification pass ran with the reserve and changed nothing of substance (or its changes were re-dry-run). |
| D (hard stops) | Agent budget spent; the 8-hour window elapsed. Both leave the ledger marked "Stopped: <reason>" with the remaining gap list, so a later loop resumes from it. |

All boxes currently `[ ]`. LEDGER also holds: research index (16 open topics, LEDGER.md:70-85), document index (empty), budget table (tick 0: 0 spent, 150 remaining), "Decisions for the user" D1-D6 (all `open`, LEDGER.md:99-108), corrections table (one row: discovery "9 phases" wrong, map sec. 9 defines 0-7), residual unverified items (LEDGER.md:116-124), "Stop rule: NOT MET".

### 5. BRIEFS.md child templates

Common header (verbatim in every brief) fixes repo path `<phase1-worktree>`, scratchpad, invariants, cite rule, "write compact tables". Handback format (verbatim, <60 lines): summary, files_written, findings (<=8 with cites), corrections, decisions, gaps, next_actions (<=3).

| ID | Name | Purpose | Inputs | Outputs |
| --- | --- | --- | --- | --- |
| B1 | C++ function-level deep-dive | one row per method: Rust signature, ordered pseudocode (f32/f64, rand() calls, iteration order, globals), pin, cite | `<src/path.cpp>` + header, `plan-coverage.sh --methods <file>` method list, discovery doc section | `plan/functions/<basename>.md` with sections State and order hazards, Corrections, Open |
| B2 | Rust engineering research | handbook section answering given questions, claims compile-checked | exact questions, primary sources only | section of `plan/rust/handbook.md` + `plan/rust/snippets/<topic>/`, ends with "Rules for this port" and `verified: pending` |
| B3 | Crate API verification | pin crate, compile-check every API the plan relies on | crate name/version, API list from map/phase brief | row in `plan/rust/crates.md` + `plan/rust/snippets/<name>/` (no target/) |
| B4 | Parity infrastructure | (a) matrix.md rows, (b) generator scripts, (c) comparator design for state dump, (d) D6 C++ dump reference as patch under `parity/` | `parity-test-spec.md` sec. 4, `discovery/golden/scripts/` | `plan/parity/matrix.md`, `plan/parity/scripts/`, `plan/parity/infra.md` |
| B5 | Phase brief author | self-contained implementer brief; must iterate until no unanswered implementer question | functions/*.md, settings-spec, parity docs | `plan/phases/phase-N.md` (Goal, Inputs, Outputs, Module tree, Interfaces, Behaviour, Gate, Acceptance, Size, Decisions touched, Not in this phase) |
| B6 | Implementer dry-run | the empirical completeness test; fresh agent reading ONLY `plan/`, `crates/`, `discovery/golden/`, `tests/logs/`; works in `<scratchpad>/dryrun-<N>/` | phase N, size limit | returns files_opened, questions, assumptions, gate, diff_size; orchestrator records `dry-runs/phase-N.md` as `questions:`, `cpp_opens:`, `gate: pass|fail|not capturable here` |
| B7 | Citation verifier | sample >=20% cites (all numeric-value cites and cites added since last verified line); report holds / line shift (fix) / wrong (report) | doc + commit sha | `verified: <sha> <date>` appended if clean; verdict list + ambiguities |
| B8 | Settings-spec author | one row per option (help-long.txt union ARG_TYPES in `crates/gource-conf/src/args.rs`), exact error texts | those files, gource_settings.cpp, core/settings.cpp, core/conffile.cpp | `plan/settings-spec.md` + ConfFile grammar, Load order, Per-section reset set, Corrections, Open |
| B9 | Coverage audit | sceptic audit of 20 random rows against a criterion A.n | a file + criterion | list of rows that only look complete |

Note: B6 forbids reading `src/` and discovery prose, but the dry-run only reads `crates/`, `golden/`, `tests/logs/`.

### 6. plan-coverage.sh

Read in full; read-only (it only greps/reads, `git log/diff/merge-base`, and `cargo check -q` inside existing `rust/snippets/*/` dirs; none exist now, so no build output is produced). Header: "Reports gaps; never edits. Exit 0 when no gaps, 1 otherwise." Run (works from any cwd; derives repo from script location):

```
bash <phase1-worktree>/docs/rust-rewrite/plan/scripts/plan-coverage.sh
bash .../plan-coverage.sh --methods src/dirnode.cpp   # prints method list a functions doc must cover
bash .../plan-coverage.sh -h
```

Output: sections `== A.1 .. A.6, B, C`, each gap printed as `  gap: <text>`, final `SUMMARY: N gaps`. Checks: A.1 each `src/*.cpp` in Makefile.am has a `| src/x.cpp |` row in map/modules.md; A.2 for 34 STATE_FILES a `functions/<base>.md` exists, contains every method name from the grep-based extractor plus headings `Rust signature`, `Pseudocode`, `Pins`, `Cite`, `## State and order hazards`, `## Corrections` (a doc may declare `methods: manual`); A.3 each option from help-long.txt and ARG_TYPES appears as `` `opt` `` in settings-spec.md, plus 3 required sections; A.4 matrix has >=28 numbered rows each with `golden:` or `script:` path that exists; A.5 each crate row has version, `docs.rs/`, ISO date, snippet dir that `cargo check`s; A.6 phase 1,3,4,5,6,7 briefs have 11 required headings; B dry-run files contain `questions: 0`, `cpp_opens: 0`, `gate: pass|not capturable here`; C `verified: <sha>` line present and not older than last content change. Caveat: the A.2 extractor is regex-based and "no methods found" is itself a gap unless `methods: manual`; it requires the `src/core` submodule checked out (present in hungry-kilby).

CURRENT GAP COUNT (run during this task, on the kit worktree): `SUMMARY: 221 gaps`. Matches the previous session's baseline of 221. (Note: the discovery worktree I run in does not contain the kit; the kit exists only on branch `claude/hungry-kilby-78e776`.)

### 7. How to launch the loop

1. Fresh session whose working directory is the hungry-kilby worktree (or a new worktree off that branch), on branch `claude/hungry-kilby-78e776`, clean tree.
2. Run `/loop` (dynamic mode) with the prompt: `Read docs/rust-rewrite/plan/LOOP.md and run one tick` (per hand-off note line 26).
3. The first tick must write `Started:` and `Hard stop:` in LEDGER.md (LEDGER.md:14), run the script, and begin.
4. The idle loop session left over from the kit can be archived or sent "go"; it has done nothing.

State left: kit committed (4692454, 179b76e), no tick ever run, ledger pristine, budget 150/150, all D1-D6 `open`.

### 8. Conflicts between kit assumptions and the user's final decisions

Final decisions are recorded in DISC/LEDGER.md:106-121 (dated 2026-10-30) and `CONTEXT.md` at the repository root (both on `master` since PR #2). KIT paths below are relative to `.../hungry-kilby-78e776/docs/rust-rewrite/plan/`; DISC paths relative to the discovery docs dir.

| # | Decision | Where the kit/map conflicts | Needed change |
| --- | --- | --- | --- |
| 1 | Parity = same parsed commits + simulation state, rendering visually equivalent | No conflict in intent (LEDGER.md:3-8 "parity gate"). Kit never states the definition. | Add one-line definition to LEDGER.md goal (line 3-8) and CONTEXT reference. Low. |
| 2 | Rust workspace beside the C++ until parity proven | Kit assumes this implicitly (branch layout). No conflict. | Optionally state it in LEDGER goal. |
| 3 | macOS + Linux first release; Windows via Docker headless export only | LEDGER.md:80 research topic asks for CI matrix "Linux/macOS/Windows"; DISC map:342 and :360 (phase 6 "Windows console") and :361 (phase 7 `cargo build` on Linux/macOS/Windows); LOOP/BRIEFS have no Windows-Docker topic. | Change LEDGER.md:80 to Linux+macOS CI plus a Docker headless-export recipe for Windows; adjust phase 6/7 scope (drop Windows console/HWND/DPI/NSIS; add Dockerfile + headless PPM export gate); correct DISC map:342,360,361; LEDGER.md:81 packaging topic drop NSIS replacement. |
| 4 | Two PRs against master: docs first, then code | Kit commits per tick on the branch, no PR step; LOOP.md:23 "Commit on this branch only ... Never push" is fine for the loop but no PR sequencing or doc-only boundary exists; LOOP.md:77 commits only `plan/` which is consistent with a docs-first PR; code (`crates/`) already exists on this branch (phase 1) and would be in the code PR. | Add to LOOP.md step 7 (after line 88) a final handoff: PR1 = discovery + plan docs, PR2 = Rust workspace; note crates/ from phase 1 belongs to PR2. Do not let the branch mix them. |
| 5 | D1: validate all `[gource]` sections up front | KIT LEDGER.md:103 and decisions.md:8 match, status still `open`. | Mark D1 decided in LEDGER.md:103 and decisions.md:8, with no flag (the kit's "restoring flag" wording goes). |
| 6 | D2: export waits for log load; old-behaviour flag optional | LEDGER.md:104, decisions.md:9 match; flag `--legacy-log-load` consistent with DISC LEDGER:113. | Mark decided; the flag is optional (not specifically requested by the user). |
| 7 | D3: documented Rust PRNG default, glibc/macOS modes | LEDGER.md:105, decisions.md:10 match (also lists Park/Script modes; fine; env `GOURCE_RS_PRNG` proposed). | Mark decided. |
| 8 | D4: accept and ignore `--ffp` | LEDGER.md:106 "accept, shader path" is equivalent; decisions.md:11 same. | Mark decided; wording "ignore" acceptable. |
| 9 | D5: fix all four known C++ bugs now | CONFLICT. LEDGER.md:107 and decisions.md:12 default is "keep known bugs during the port" and rationale is goldens stay valid; DISC map:371-373 says keep cvs-exp unsupported (371), accept `--stop-on-idle` and ignore (372), keep help text and implement 1.0 (373). | Rewrite LEDGER.md:107 and decisions.md:12: fix all four (cvs-exp made to work; user-friction and dir-name-position resolved by correcting help text per DISC LEDGER:116; `--stop-on-idle` default accept-and-ignore pending confirmation DISC LEDGER:121). Update DISC map:371-373 rows (374 is the D1 row). B8/settings-spec and parity matrix must record expected golden diffs on those help lines; golden/help-*.txt will differ. |
| 10 | Renderer: winit + glutin + glow (not sdl2) | CONFLICT. LEDGER.md:77 research topic `sdl2` + `glow`; BRIEFS B3 generic but crates.md will list sdl2/freetype; DISC map:20 (`gource-gl` deps sdl2, glow), map:205 ("sdl2 0.38 + glow 0.18"), map:291 (native deps SDL2 + FreeType), DISC LEDGER:56. Also LEDGER.md:74 file list and `Rust: ... sdl2` tick. | Change LEDGER.md:77 to `winit` + `glutin` (+ `glutin-winit`) + `glow`, pin exact versions (glutin 2025-04-30, glutin-winit 2024-06-08 per DISC LEDGER:118); add research topic for input/IME/clipboard/cursor/HiDPI differences and matching parity-spec input and DPI tests; correct DISC map:20,205,291. No C SDL2 required. |
| 11 | Validate config up front / export waits | covered in 5-6. | |
| 12 | D6: fork the C++ sources for the state-dump reference | CONFLICT. KIT decisions.md:13, LEDGER.md:108, BRIEFS B4(d) (BRIEFS.md:103-105: "patch file under parity/ applied to a scratch checkout ... never to src/") and LOOP.md:20-22 (no child writes dump instrumentation). DISC map:357 (phase 3: dump "added to both C++ and Rust"). | Change decisions.md:13, LEDGER.md:108, BRIEFS.md:103-105 to: fork of C++ sources used only for the state dump; untouched C++ build remains oracle for byte-exact goldens (help, exit codes, custom-log); guard = fork must reproduce those goldens before dump trusted; fork directory name/location still to be chosen by the user (open). Amend LOOP.md:20-22 so `src/` stays untouched but a fork dir is permitted; B4(d) then specifies fork creation script instead of a patch. |
| 13 | Dry-run restricts reads | B6 forbids `src/` and discovery prose; fine but dry-run for phases touching renderer will need crates.md for winit/glutin to exist first. | Order dependency only. |

Additional stale items: LEDGER.md:114 "9 phases" correction row is right; BRIEFS B6 phase set (1,3,4,5,6,7) assumes the Windows console work remains in 6/7. Kit also lists `--stop-on-idle` in "known bugs to keep" (LEDGER.md:107). The kit's LOOP.md:35-40 model table says "sonnet" children while the discovery loop used Fable orchestrator; the model is unspecified for orchestrator, fine.

### 9. Verdict

Not ready to run as-is. Mechanically sound (script gives 221 gaps, templates complete, restartable), but the ledger's decisions table and three briefs encode the OLD defaults. Must change first, in order: (1) rewrite D5 ("fix all four now") in LEDGER.md:107 + decisions.md:12; (2) rewrite D6 (fork, not patch) in LEDGER.md:108, decisions.md:13, BRIEFS.md:103-105, and relax LOOP.md:20-22 to allow a fork directory (location unresolved: ask user); (3) replace sdl2 with winit+glutin+glow in LEDGER.md:77 (and map:20,205,291); (4) rescope Windows to Docker headless export in LEDGER.md:80-81 and map phases 6/7; (5) mark D1-D4 decided and add the two-PR delivery rule to LOOP.md step 7; (6) fill CONTEXT-based parity definition into LEDGER goal. Then launch per section 7. Also expect plan-coverage.sh gap count to change after (3)-(4) (A.5 crates list) but not A.1-A.4.

# Part 5: Environment and operations

## Operations handoff (machine, oracle build, repo facts, gotchas, cold start)

Facts marked (ran) were obtained by running the command in this session (2026-10-30). Facts marked (doc) come from a repo file and were not re-run.

### 0. Layout of checkouts (important)

- Main checkout: <repo> on branch `setup-agent-skills` (6ea1d6f, "Add agent skills configuration"), still MID-MERGE (paused, uncommitted; the user should abort it, Part 1 B.5). It has CLAUDE.md and docs/agents/*, no Rust, and an OLDER copy of the discovery tree from the paused merge (its LEDGER.md and build-and-goldens.md differ; the copies on `master` are authoritative).
- Worktree A: <discovery-worktree>, branch `worktree-rust-rewrite-discovery` at 3b1cea2 (pushed and merged via PR #2). Contains the discovery docs, golden/ and golden/scripts; worktree clean. No Cargo.toml here.
- Worktree B: <phase1-worktree>, branch `claude/hungry-kilby-78e776` checked out (no longer detached), pushed, carrying draft PR #3. This holds ALL the Rust code (Cargo workspace "phase 1") plus docs/rust-rewrite/phase1.md and docs/rust-rewrite/plan/{BRIEFS,LEDGER,LOOP,decisions}.md; it also contains `master` (merge `f38481f`). Other local branches: master (behind origin), setup-agent-skills, worktree-rust-rewrite-discovery.
  - Commits not on master (Part 2): e2d0c64 "Rust rewrite phase 1: Cargo workspace and log-format parsers", 69725e1 (golden row count from exit-codes.txt, SIGPIPE/PCRE2 CI notes), 4692454 "Planning loop kit...", 179b76e "plan-coverage...", f38481f (merge of origin/master), e66a9ee (path scrub).
- git remote: origin git@github.com:dfadler/Burl.git. Default branch `master` (origin/HEAD). Remote branches: master, setup-agent-skills.

### 1. Toolchain (ran)

| Item | Result |
|---|---|
| Shell arch | `uname -m` = arm64 |
| macOS | ProductName macOS, 26.5.1, build 25F80 (Darwin 25.5.0) |
| Rosetta | works: `arch -x86_64 /usr/bin/true` exits 0 |
| Homebrew | TWO installs: /usr/local (Intel, x86_64) and /opt/homebrew (arm64). `which -a brew` lists /usr/local/bin/brew first, so `brew --prefix` = /usr/local. /usr/local/lib/libSDL2.dylib is `Mach-O x86_64`. /opt/homebrew has only ffmpeg-ish libs (aom, cairo, ffmpeg, gcc, libpng ...) and NO sdl2/glew/boost/glm. The doc (build-and-goldens.md) said "/opt/homebrew absent"; that is stale, it exists now but lacks the gource deps. |
| rustc | 1.99.0 (b940084d7 2026-09-28) (Homebrew), at /usr/local/bin/rustc, `Mach-O x86_64`, host `x86_64-apple-darwin`, LLVM 23.1.3 (so Rust builds run under Rosetta, default target x86_64-apple-darwin) |
| cargo | 1.99.0 (5f94df478 2026-08-27) (Homebrew) |
| rustup | ABSENT (command not found); no extra targets, Homebrew rust only. Cellar has rust 1.98.0 and 1.99.0_1. |
| clang++ | Apple clang 21.0.0 (clang-2100.1.1.101), target arm64-apple-darwin25.5.0, /Library/Developer/CommandLineTools |
| pkg-config | 3.0.8 (/usr/local/bin/pkg-config) |
| autoconf / automake | ABSENT (command not found) |
| libtool | /usr/bin/libtool is Apple's, NOT GNU. glibtool/glibtoolize absent. autoreconf absent. |
| node | v22.22.1 |
| python3 | 3.15.0 |
| git | 2.50.1 (Apple Git-155) |
| gh | 2.103.3 (2026-10-26) |
| rtk | /usr/local/bin/rtk (output condenser) |
| timeout/gtimeout | not present (doc) |

Library versions via `pkg-config --modversion` (ran), all x86_64 under /usr/local:
- sdl2 2.30.1, SDL2_image 2.8.2, glew 2.2.0, freetype2 26.6.20 (libtool version, ~2.14.3 per Cellar), libpcre2-8 10.47, libpng 1.6.59
- boost 1.84.0_1 (/usr/local/Cellar/boost; boost@1.84 alias), glm 1.0.1 (/usr/local/Cellar/glm; headers /usr/local/include/glm)
- Also in /usr/local/opt: cmake 4.4.4, bats-core 1.14.0, kcov 43_1, gh, go 1.27.3, a Homebrew `gource` 0.54_3 (an unrelated older stock Gource, NOT the oracle).

### 2. Rebuilding the C++ oracle without autotools

Why no autotools: the repo ships no `configure`, only autogen.sh + configure.ac; autoconf/automake/GNU libtool are not installed. The documented route avoids them: compile every file directly with clang++.

Why `-arch x86_64`: Homebrew in /usr/local is x86_64 (libs, pkg-config output) while the shell is arm64. Compiling natively arm64 pulls in x86 intrinsics from SDL.h and every file fails with `raointintrin.h: use of undeclared identifier '__builtin_ia32_aadd32'`. `-arch x86_64` fixes it; the resulting Mach-O x86_64 binary (868 KB) runs transparently under Rosetta. (doc: build-and-goldens.md)

Exact sequence (doc: golden/scripts/README.md; scripts are in <discovery-worktree>/docs/rust-rewrite/discovery/golden/scripts/). Not re-run by me because it writes build output and rewrites tracked golden files:

```
cd <discovery-worktree>/docs/rust-rewrite/discovery/golden/scripts
BUILD=$(mktemp -d "${TMPDIR:-/tmp}/gource-build.XXXXXX")   # use the session scratchpad instead of /tmp
BUILD=$BUILD ./build.sh    # copies src/ and data/ into $BUILD, compiles 53 files (src/main.cpp + Makefile.am `sources` + src/tinyxml/*.cpp), 8 parallel jobs, logs in $BUILD/log
BUILD=$BUILD ./link.sh     # links $BUILD/gource
BUILD=$BUILD ./gold.sh     # OVERWRITES ../help-*.txt, ../exit-codes.txt, ../custom-log/*, probe files in the repo
BUILD=$BUILD ./tbuild.sh   # builds Boost unit tests (header-only boost/test/included), OVERWRITES ../unit-tests.txt
```

Script env knobs: REPO (defaults to checkout containing the scripts), BUILD, ARCH (default `-arch x86_64`), BREW (default /usr/local). For an arm64 Homebrew install set `ARCH= BREW=/opt/homebrew` (only works if deps are installed there; currently they are not).

Per-file compile flags: `-arch x86_64 -std=c++17 -O1 -w -I src -I src/core -I src/tinyxml -DSDLAPP_RESOURCE_DIR=\"$BUILD/data\" -DHAVE_TIMEGM -DHAVE_UNSETENV $(pkg-config --cflags sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -I/usr/local/include -I/usr/local/opt/boost/include`.
Link: `clang++ -arch x86_64 obj/*.o $(pkg-config --libs sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -L/usr/local/opt/boost/lib -lboost_filesystem -lboost_system -framework OpenGL -framework CoreFoundation -o gource`.

Prerequisite: `src/core` submodule must be populated (see section 3), since build.sh copies src/ including src/core.

Platform caveat (important): this is the only platform ever run. All goldens come from the macOS arm64 host running the x86_64 binary under Rosetta; Linux behaviour (glibc `rand()`, GL, fonts) is unverified.

Runtime: not recorded in any doc. Discovery ledger says ticks were fast; compile is 53 files at -O1 with 8 parallel jobs, expect on the order of a few minutes (unmeasured; treat as estimate). Goldens: gold.sh runs ~10 invocations, each killed after 20 s if hung.

Pitfalls:
- No `timeout` on macOS: gold.sh uses a background `sleep 20; kill -9` watchdog.
- `gource --version` is NOT an option: rc 1, stderr "unknown option version". Version string comes from line 1 of `gource -h`: "Gource v0.57" (GOURCE_VERSION in src/gource_settings.h; configure.ac AC_INIT(Gource, 0.57)).
- `SDL_VIDEODRIVER=dummy` has no OpenGL: `gource --stop-at-time 1 -o - tests/logs/utf8.log` exits 1 with "SDL initialization failed - OpenGL support is either not configured in SDL or not available in current SDL video driver (dummy)". So no headless render golden on macOS; a real window is needed (never opened). Linux Xvfb + llvmpipe recipe is in parity-test-spec.md.
- `--output-custom-log` works without creating a window (the only headless path, the parser golden).
- utf8-caption.log is a `--caption-file` fixture, not a commit log; with `--log-format custom` it gives rc 1 "unsupported log format". No golden for it.
- Scripts use `set -uo pipefail` and ignore compile failures except printing `FAIL <file>`; check `$BUILD/log/*.txt` and that `ls $BUILD/obj | wc -l` is 53.
- gold.sh and tbuild.sh write into the tracked golden tree: after running, `git status` / `git diff` to confirm goldens did not change unexpectedly (a diff means oracle drift).
- build-and-goldens.md had a truncated "Needed to proceed" paragraph (cut off mid-sentence, next heading glued on); fixed on 2026-10-30 and relabelled "Autotools route (not needed)" because the direct build worked.
- Homebrew libs may drift: doc recorded glew 2.2.0, freetype 26.6.20, pcre2 10.47, png 1.6.59, sdl2 2.30.1, SDL2_image 2.8.2, boost 1.84.0_1, glm 1.0.1; (ran) these still match today.

Scratchpad build (now gone) vs reproducible: the original build directory (a session temp dir holding a copy of src/ and data/, obj/*.o, log/, gource, gource_tests) was deleted; nothing in it was unique. Reproducible from the repo: everything above, i.e. the binary and its objects. The captured goldens are committed under docs/rust-rewrite/discovery/golden/ (help-short.txt, help-long.txt, exit-codes.txt, version.stderr.txt, probe-dummy.stderr.txt, unit-tests.txt, custom-log/*, including utf8.stdin variants). Only the oracle binary itself must be rebuilt.

Decision D6 (LEDGER.md): the fork of C++ sources (`reference/gource-cpp-dump/`, decided 2026-10-31, does not exist yet) emits per-frame state dumps; the untouched original build stays the oracle for byte-exact goldens (help, exit codes, custom-log). The fork must reproduce the same goldens before its dump is trusted.

### 3. Repository facts

C++ project top level (worktree A): `.github/` (only FUNDING.yml), `cmd/`, `contrib/`, `data/`, `dev/`, `docs/`, `m4/`, `resources/`, `scripts/`, `src/`, `tests/`, `.gitattributes`, `.gitignore`, `.gitmodules`, `COPYING` (GPLv3), `ChangeLog`, `INSTALL`, `Makefile.am`, `README.md`, `THANKS`, `autogen.sh`, `configure.ac`, `gource.pro` (Qt project file), plus `CONTEXT.md` (committed in PR #2).
- cmd/: `gource`, `gource.cmd`. contrib/: `svn-gource.py`. dev/: `bin/`, `nsis/` (Windows). scripts/: `gource-ps.pl`. resources/: file.xcf, user.xcf. m4/: ax_*.m4, pkg.m4.
- src/: gource app sources (gource.cpp 72K, gource_settings.cpp 54K, dirnode, file, user, pawn, action, logmill, bloom, caption, key, slider, spline, textbox, zoomcamera, gource_shell, main.cpp), subdirs `core/` (submodule), `formats/` (log parsers), `test/` (Boost tests: main.cpp, datetime_tests.cpp, regex_tests.cpp), `tinyxml/`.
- data/: beam.png, bloom.tga, bloom_alpha.tga, file.png, user.png, gource.1 (man page), gource.style, fonts/, shaders/.
- tests/: `logs/` (custom-dir-delete.log, file-removal.log, file-to-dir.log, svn-dir-delete.log, utf8-caption.log, utf8.log), `images/`, `test.conf` (manual scenarios; nothing in the build references it).
- Platform coverage: every golden was captured on macOS arm64 hardware running x86_64 binaries under Rosetta. Linux behaviour (`rand()`, GL, fonts, the whole build of the oracle and of the fork) has never been run or verified; the first Linux run is part of Part 6 actions 2 and 4.
- Version 0.57, license GPL-3.0-or-later (COPYING = GPLv3). Rust workspace declares version 0.57.0, license "GPL-3.0-or-later", repository https://github.com/acaudwell/Gource.
- .gitignore (worktree A, trimmed): *.avi *.mpg *.mkv *.mp4, gource-*.png, .objs, /gource, /gource.exe, gource.pro.user, debug.log, Makefile, Makefile.in, *.dll, *.dll.a, logs/*, /aclocal.m4, /autom4te.cache, /configure, /config.log, /config.status, /dev/win64, /build-aux, *.swp, *.d, .deps, .dirstamp. Does NOT ignore Rust `target/` (check before committing from worktree B; none was found checked in).
- CI: none. `.github/` contains only FUNDING.yml; no workflows. Rust docs mention future CI (pcre2 feature test, SIGPIPE).
- Submodule src/core: url https://github.com/acaudwell/Core.git, pinned SHA d8d880fd58fda37b9d8fb59844a3c2a712969948 (describe: remotes/origin/abstract-shader-115-gd8d880f). It is initialised in worktree A (ran `git submodule status`, no leading `-`). Fresh clone: `git submodule update --init src/core`.

Agent/instruction files a successor must follow:
- <repo>/CLAUDE.md (main checkout only, from branch setup-agent-skills; not present in worktrees A/B): three pointers. Issue tracker = GitHub Issues via `gh` (docs/agents/issue-tracker.md). Triage labels = default five (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`; docs/agents/triage-labels.md). Domain docs = single context: one CONTEXT.md + docs/adr/ at repo root (docs/agents/domain.md).
- docs/agents/issue-tracker.md: use `gh issue create/view/list/comment/edit/close`; PRs are NOT a triage surface ("PRs as a request surface: no"); issue/PR share a number space; wayfinder conventions (map issue label `wayfinder:map`, child tickets, blocked_by via `gh api`).
- docs/agents/domain.md: read CONTEXT.md and docs/adr/ before exploring, proceed silently if absent, use glossary terms, flag ADR conflicts explicitly.
- CONTEXT.md (repository root, on `master` since PR #2): defines the project (Rust rewrite of Gource, C++ to Rust; first release matches C++ behaviour) and the term **Parity** = same parsed commits and same simulation state for same inputs and settings; rendering only visually equivalent, not pixel-identical. Avoid the words Compatibility, identical output, same output.
- <repo>/.claude/ contains only worktrees/ (no settings.json, no settings.local.json). No AGENTS.md anywhere. No docs/adr/ yet.
- Worktree B's docs/rust-rewrite/plan/{LOOP,LEDGER,BRIEFS,decisions}.md are the planning-loop kit for the Rust phases (not read in detail; successor should read decisions.md first). phase1.md lists deferred items and ledger decisions D1-D5; LEDGER.md (worktree A) lists D1-D6, renderer stack winit+glutin+glow, first release macOS + Linux.

### 4. Process and tooling gotchas

Source files are the user's global config (read, not modified): <home>/.claude/CLAUDE.md (includes conventions via @-lines) and <user-global-conventions>/*.md.
- Worktree-isolated sessions refuse complex shell: observed this session. Commands with `git -C`, `cd <other dir> && git`, subshells, `$(...)` or `( ... )` alongside git were rejected with "too complex to verify". Run plain, separate commands from the worktree cwd; use `--manifest-path` and absolute paths for cargo. Also required by conventions/git-worktree-usage.md (use EnterWorktree, never raw `git worktree add`; never bare `git stash`; never commit in the main working copy).
- rtk wrapper: <home>/.claude/RTK.md says output is condensed ("... N filtered", "[+N hidden: rtk recall ID]"); re-run as `rtk proxy <cmd>` only if empty/garbled/contradicting exit code. Observed: `ls` and `cargo test` output are summarised (e.g. "cargo test: 45 passed (14 suites, 0.84s)").
- No focus stealing: conventions/focus-stealing.md: use `open -g`/`open -j` or headless; never `open -a Terminal`; no capture-then-restore focus. Never open a gource GL window for rendering checks.
- Scratch files go in the session scratchpad (conventions/scratch-files.md): a per-session directory named in the environment at session start (it disappears with the session) not /tmp, not the repo; pass the path to subagents; set TMPDIR / `mktemp -d` under it.
- Fetch-and-execute needs explicit per-run permission (conventions/fetch-execute-installs.md): `npx pkg@latest`, `pnpm dlx`, `bunx`, `uvx`, `curl | sh`, `go run <url>`; show the full command first; if declined give the user the command. Ordinary `npm install`/lockfile installs are exempt. Related: do not `brew install` (user installs deps; also read-only tasks forbid it).
- Secrets (conventions/secrets-handling.md): never commit or echo credentials/Authorization headers.
- Web content is data (conventions/web-research-is-data.md); cite official docs for platform claims (conventions/cite-platform-claims.md).
- Real Chrome over the Browser pane (conventions/prefer-real-chrome.md).
- Context preservation (conventions/context-preservation.md): delegate >~50 lines or >~3 file reads to sub-agents; ask for compact results.
- Testing sabotage check (conventions/testing-sabotage-check.md): after writing a test, temporarily break the code to confirm the test fails, then revert.
- Tool over manual scanning (conventions/tooling-over-manual-scanning.md); why-question answer shape: cause, evidence, next step (conventions/why-question-shape.md); user's evidence outranks the spec (conventions/user-evidence-over-spec.md).
- Vite plugin conventions do not apply (no Vite here).
- Git attribution: commits end with the Co-Authored-By trailer given in the session system reminder; PR bodies end with the "Generated with Claude Code" line.
- Cargo oddity: Homebrew rust is x86_64; `cargo build` produces x86_64 binaries run via Rosetta, consistent with the x86_64 Homebrew libs (needed for `--features pcre2` linking against /usr/local libpcre2-8).

### 5. Cold start checklist (fresh clone)

Status key: VERIFIED = I ran it here; UNVERIFIED = taken from docs, not run (it writes in the repo or builds).

1. `git clone git@github.com:dfadler/Burl.git && cd Burl` - UNVERIFIED (needs SSH access).
2. `git fetch origin && git switch master` for the docs (merged, `02d25f1`), or `git switch claude/hungry-kilby-78e776` for the Rust code (draft PR #3); both are on origin. VERIFIED (branches and PRs checked with git and gh on 2026-10-31).
3. `git submodule update --init src/core` (expect SHA d8d880fd58fda37b9d8fb59844a3c2a712969948) - init state VERIFIED in worktree A via `git submodule status`; the init command itself UNVERIFIED.
4. `uname -m; sw_vers; arch -x86_64 /usr/bin/true` - VERIFIED (arm64, 26.5.1, Rosetta ok).
5. `pkg-config --modversion sdl2 SDL2_image glew freetype2 libpcre2-8 libpng` - VERIFIED (2.30.1 2.8.2 2.2.0 26.6.20 10.47 1.6.59); also `ls /usr/local/opt/boost /usr/local/include/glm`.
6. `cd <repo>/docs/rust-rewrite/discovery/golden/scripts && BUILD=$(mktemp -d <scratchpad>/gource-build.XXXXXX) ./build.sh` (expect "objects: 53") - UNVERIFIED this session (documented as working in build-and-goldens.md).
7. `BUILD=$BUILD ./link.sh` (expect x86_64 Mach-O `gource`, ~868 KB) - UNVERIFIED.
8. `BUILD=$BUILD ./gold.sh && git status --short` (expect no golden diffs; rc rows in golden/exit-codes.txt) - UNVERIFIED.
9. `BUILD=$BUILD ./tbuild.sh` (expect 69 assertions pass per unit-tests.md, written to golden/unit-tests.txt) - UNVERIFIED.
10. Rust tests (same command as Part 3, with `CARGO_TARGET_DIR` set to a temp directory): `cargo test --offline --locked --manifest-path <worktree-with-Cargo.toml>/Cargo.toml` - VERIFIED on <phase1-worktree> (earlier HEAD 179b76e; re-run on the merged branch before PR #3, 45 passed: 10 golden, 19 formats, 16 util): "45 passed (14 suites, 0.84s)" (output was rtk-condensed; target dir kept out of the repo; used the cached crate registry so --offline worked). Also try `cargo test -p gource-util --features pcre2` (documented to pass, UNVERIFIED by me). Note `crates/gource/tests/golden.rs` reads the golden files, so run it from a tree that has discovery/golden (worktree B does).

# Part 6: Remaining work, decision impact, risks, next actions

## Remaining work, decision-impact audit, risk register, next ten actions

Path shorthands (all absolute bases):
- DISC = <discovery-worktree>/docs/rust-rewrite/discovery
- P1 = <phase1-worktree> (phase 1 code branch; also holds docs/rust-rewrite/phase1.md and docs/rust-rewrite/plan/{LEDGER,BRIEFS,LOOP,decisions}.md, an unfinished planning loop with "Stop rule: NOT MET", plan/LEDGER.md:126)
- MAP = DISC/rust-architecture-map.md, SPEC = DISC/parity-test-spec.md, DEP = DISC/deps-and-rust-mapping.md, TGS = DISC/tests-and-golden-strategy.md
- Phase numbers below are the MAP sec. 9 numbering (MAP:354-361), NOT the "Phase 1" label in P1/docs/rust-rewrite/phase1.md (which means map phases 0+2).
- Verified state: the docs are on `master` and the code is on draft PR #3 (see Part 2). P1 has no `.github/workflows` (only FUNDING.yml), no `tests/golden/`, 3482 lines of Rust in 20 files across 7 crates.

---------------------------------------------------------------------------
### A. Remaining work by map phase (0-7)

#### Phase 0: workspace skeleton, CI, gource-util. Status: PARTIAL (about 60%)
Verified in P1/crates/gource-util/src: `regex.rs`, `strhash.rs` (stringHash/colourHash/vec2Hash/vec3Hash), `utf8.rs` (replace_invalid), `timeparse.rs`, `cstr.rs` (atoi/atoll), `math.rs` (35 lines, `normalise` only; phase1.md says so). Workspace exists with 7 crates (P1/Cargo.toml).
Missing:
- `prng` module: `SimRng` trait with call counter, documented default (xorshift) with fixed seed, `Glibc` TYPE_3, `Park` (macOS), `Script` modes (MAP:305-317; SPEC 2.4 = SPEC:120-127). Nothing in P1/crates/gource-util/src.
- `bounds` (Bounds2D), `quadtree` (visit order TL,TR,BL,BR, leaf Vec insertion order, closed bounds, dedupe when node_count != 1; SPEC 2.6 item 1), `math::rotate_vec2` and `PI = 3.14159265` f64 const (PI row MAP:379).
- CI: no `.github/workflows` in P1 (MAP:354 lists "CI" in phase 0). Needs `cargo test`, `cargo test -p gource-util --features pcre2` (phase1.md last bullet), fmt/clippy.
- Port of `src/test/regex_tests.cpp:22-45` (SPEC 1.5) not confirmed present (phase1.md mentions only the 28 datetime vectors); check P1 tests before assuming.
Inputs/goldens: `DISC/golden/unit-tests.txt`, `src/test/{datetime,regex}_tests.cpp`, glibc first outputs 1804289383, 846930886, 1681692777, 1714636915 (scratch Python model only, `DISC/golden/scripts/glibc_rand.py`); macOS Park-Miller is confirmed by a compiled test (`srand(1)`: 16807, 282475249, 1622650073, 984943658) but the outputs are not archived: record 10 000 values from a compiled C helper on this Mac. Linux is unverified: every golden so far is macOS under Rosetta.
Gate: `cargo test` green incl. ported vectors; glibc mode matches a real-libc helper for first 10 000 values (SPEC:123 says confirm by a C helper on Linux in CI); default PRNG determinism test; sabotage check per gate.
Size: S (M if CI + real-libc verification included). Depends on: nothing. Parallel with: 1, D6 fork/infra, phase 5 spike.

#### Phase 1: gource-conf (ConfFile, parse_args, settings import, help, errors, --save-config). Status: PARTIAL (about 25%)
Verified in P1/crates/gource-conf/src: `args.rs` (215 lines: `arg_types`/`arg_aliases`/section tables, hyphen stripping, value swallowing, `-WxH` regex lives in `crates/gource/src/main.rs:19`), `help.rs` (7 lines, `include_str!` of the two help texts). `gource-formats/src/settings.rs` (55 lines) holds only a tiny `LogSettings`. P1/docs phase1.md "Deliberately deferred": every other option is "consumed and dropped".
Missing (exactly):
- `conffile.rs` (grammar regexes, multi-section `BTreeMap<String,Vec<Section>>`, first-value rule, error `<file>, line <n>: could not parse line`; MAP sec. 5.2 = MAP:255-263).
- `settings_display.rs`, `settings_gource.rs`: typed import and validation of EVERY option with exact error text (settings-cli.md sec. 8 table, DISC/settings-cli.md:134-231), defaults (`setGourceDefaults`, GS:368-520), derived values (`days_per_second = 1/x`, `user_friction = 1/x`, hash-seed, dates via parseDateTime, filters via PCRE2).
- `--load-config`, positional `.conf/.cfg/.ini` detection, two-pass parse_args-into-ConfFile (MAP:265-281), CLI scalars applied to every `[gource]` section, `--save-config` format.
- D1 (decided): validate all `[gource]` sections up front, exit 1 before anything plays (MAP:275).
- D5 (decided; `--stop-on-idle` accepted with no effect, confirmed 2026-10-31): corrected help text, `cvs-exp` accepted, `--stop-on-idle` accepted with no effect; `--ffp` accepted/ignored (D4).
- Non-fatal paths currently non-C++: binary prints a non-C++ message without `--output-custom-log` (phase1.md "Deliberately deferred").
Inputs/goldens: DISC/golden/{help-short,help-long}.txt, version.stderr.txt, exit-codes.txt (C++ oracle, untouched); NEW Rust-side expected help goldens (3 changed lines, see B); `tests/test.conf` (8 sections, manual per SPEC:213); a `--save-config` round-trip matrix to be CAPTURED from the oracle (does not exist yet; MAP:354 gate).
Gate: byte diff of `-h/-H` against the oracle goldens EXCEPT the recorded D5 diff; `--version` rc 1; every `rc=` row in exit-codes.txt; `--save-config` output vs oracle on an arg matrix incl. multi-section; error-text table test generated from the option table.
Size: L. Depends on: phase 0 (timeparse/regex done; prng not needed). Parallel with: 0, 2 leftovers, D6 fork, phase 3 (phase 3 only needs a minimal sim-settings struct; see action 5).

#### Phase 2: gource-formats (SeekLog/StreamLog, nine parsers, logmill, filters). Status: DONE for the gate, with listed leftovers
Verified: P1/crates/gource-formats/src has seeklog (232), commitlog (250), logmill (122), nine files under formats/ (all nine present: git, gitraw, custom, svn, hg, bzr, cvs_exp, cvs2cl, apache). phase1.md claims five custom-log goldens + stdin variants match byte for byte, `cargo test` passes. (Not re-run here: read-only session, no build.)
Remaining (from phase1.md "Deliberately deferred", all still true):
- D5: make explicit `--log-format cvs-exp` work (P1/crates/gource-formats/src/logmill.rs:68-69 matches only `"cvs"`; test at P1/crates/gource/tests/golden.rs:160-161 expects the old error).
- Edge goldens `tests/golden/edge/` (SPEC 1.3) and real-VCS goldens `tests/golden/real/` (SPEC 1.4) were never generated; `tests/golden/` does not exist in P1.
- Auto-detect (no `--log-format`) parity run for all five fixtures (SPEC:19): confirm it is a test.
- Settings-dependent behaviour not done: PCRE2 user filters, `--git-branch`, `--author-time`, `--start-date/--stop-date` window (needs settings import = phase 1), `--hash-seed`.
- StreamLog non-blocking semantics and unterminated-last-line drop (phase1.md notes blocking read; MAP:165-168 wants pump thread). Logmill thread + start_timestamp skip loop belong to phase 3. SeekLog >100 MiB streaming, `getCommitAt/getPointer`, mktime DST edges, TinyXML fidelity (CDATA, undefined entity) are known gaps. SIGPIPE behaviour differs (Rust ignores).
Inputs: tests/logs/*.log fixtures, DISC/golden/custom-log/*, pinned public repos for git/svn/hg/bzr (to choose), VCS tools installed.
Gate (existing, met): `--output-custom-log -` byte equal on 5 fixtures + stdin variants. Gate (to add): edge + real sets, auto-detect, cvs-exp now succeeding (recorded as a deviation from the oracle).
Size: S to M for leftovers. Depends on: phase 1 for the settings-dependent items only. Parallel with everything.

#### Phase 3: gource-sim headless + --debug-dump-state. Status: NOT STARTED (gource-sim is an 2-line lib.rs)
Missing: dir_tree arena, file, user, action, pawn, `Gource::logic`, commit ingestion, force functors over the quadtree, fixed-dt `step(dt: f32)`, f32/f64 narrowing mirrors (SM 10.1), dump writer (JSONL schema, SPEC:75-118), logmill hand-off + start_timestamp skip, the comparator (`tools/compare_dump.py` or xtask), the C++ FORK with dump hooks (D6), `--rand-script` in fork and Rust.
Inputs: DISC/scene-model.md + verification-tick3-scene.md (re-derived pseudocode and 9 corrections), app-core.md T2, timing.md (authoritative), SPEC sec. 2. Goldens: `tests/golden/sim/{linux,macos}/*.jsonl` (28-row matrix SPEC:187-213), must be produced by the fork, not yet built. Fork build recipe: DISC/build-and-goldens.md (clang++ -arch x86_64 under Rosetta, Homebrew libs are x86_64); fork needs a GL window (no headless; TGS:42) so goldens come from this Mac or Linux Xvfb, and the fork has no verified build on Linux yet.
Gate: exact tier (topology, add/remove/idle frames, user set, colours, rand_calls, first_rand_frame) and f32 bit-equality in Script mode on the same platform; tolerance rules SPEC 2.6. This is the project's main correctness bar (MAP:363).
Size: XL (largest phase). Depends on: phase 0 (prng, bounds, quadtree, math), a minimal slice of phase 1 (seconds-per-day, hash-seed, file/user idle times, stop-at-time, dir-name-position, user-friction, elasticity, time-scale, etc.), phase 2 (done), the D6 fork. Parallel with: phase 5 spike, phase 1 rest, comparator tooling.

#### Phase 4: camera, auto-rotate, selection, captions timeline, multi-section shell. Status: NOT STARTED
Missing: ZoomCamera, auto-rotate trigger (window-size dependent, AC T2.5; first sim set uses `--disable-auto-rotate`), selection logic, caption timeline (captions are NOT in the dump schema yet, SPEC:226), `Shell` multi-section state machine incl. per-repo reset set (MAP sec. 5.3 item 8; `gStringHashSeed=31` reset), D1 up-front validation integration, `stop_at_time=60` rule.
Inputs: app-core.md, settings-cli.md T2.1, tests/test.conf (no expected outputs exist; "manual, not gated" SPEC:213) so the multi-section golden must be CREATED from the fork; camera fields already in the dump schema.
Gate: camera fields in dump (position tier by tolerance when rotating), multi-section section-by-section counts from the fork on tests/test.conf, caption dump.
Size: L. Depends on: 3 and the rest of phase 1 (ConfFile, import). Parallel with: phase 5 (after 3 stabilises the data shapes), phase 6 preparation.

#### Phase 5: gource-gl renderer + PPM writer. Status: NOT STARTED (gource-gl: empty lib, no dependencies, P1/crates/gource-gl/Cargo.toml)
Missing: stack is now winit + glutin(+glutin-winit) + glow (decision 10), not sdl2: window/GL 3.2 core context (forward-compat on macOS), `Renderer` trait (MAP:182-201), CPU-side batching `gource-render` (scene, text layout, spline, bloom), GLSL 150 shader rewrite (MAP sec. 4.4 checklist), VBOs, textures/mipmaps, FreeType atlas (`freetype-rs`), screenshot, PPM writer thread (sync_channel(1), header every frame), PNG, DPI/viewport handling, headless export path usable inside Docker (decision 3) with no display.
Inputs/goldens: DISC/output-pipeline.md (render pass order, full shader text, uniforms), DISC/golden none for render (no render golden exists; macOS has none); C++ reference frames from Xvfb+llvmpipe on Linux (SPEC sec. 3) captured with the ORACLE or fork binary.
Gate: valid P6 stream with per-frame header and exact frame count (ffmpeg decode); cadence for F=25/30/60; SSIM >= 0.97 structural (text/bloom off), >= 0.90 full, per SPEC:172 (thresholds provisional, measure C++ run-to-run spread first).
Size: XL. Depends on: 3 for real scene data (a spike with synthetic batches can start now), 4 for camera/captions, `Renderer` trait design. Parallel with: 3, 4 (render team vs sim team); the context+PPM+headless-in-Docker spike is the highest-value early item.

#### Phase 6: UI overlays, input, interactive loop, HiDPI. Status: NOT STARTED
Missing: key, slider, textbox, logo, splash, caption drawing, input bindings (APP T2.2 key/mouse table) rewritten against winit events (not SDL), 16 ms accumulate loop (MAP:322-335), mouse picking in `gource` app crate, HiDPI via winit scale factor, text input/IME/clipboard semantics. Windows console/HWND work is now OUT of scope (decision 3).
Gate: manual checklist against APP T2.2 + automated regression of earlier gates; add explicit input and DPI tests (new, required by decision 10; none exist).
Size: L. Depends on: 5, 4. Parallel with: 7.

#### Phase 7: packaging, resource lookup, cross-platform CI. Status: NOT STARTED
Missing: resource-dir lookup order (COR sec. 3) / `include_bytes!` fonts and shaders, macOS + Linux builds, Linux render gate in CI, Windows only as a Docker image running headless export (decision 3), workflow files, cargo-deny licence audit, release PR hygiene. Two-PR delivery: PR #2 docs (merged), PR #3 phase-1 code (draft; decision 4).
Gate: `cargo build` + test on macOS and Linux; Docker image produces a valid PPM stream for a fixture; Linux visual gate green in CI.
Size: M. Depends on: 5 (headless export) for the Docker part; CI skeleton (phase 0) can start now. Parallel with: 6.

Dependency summary: 0 -> (1, 2) and 3; 1-minimal -> 3; 3 -> 4; 3 + trait -> 5; 4+5 -> 6; 5 -> 7(Docker). Sequencing rule in MAP:363 (no phase starts before prior gate is green) is too strict for this plan and conflicts with parallel work above; relax to "a phase's gate must be green before its dependents merge".

---------------------------------------------------------------------------
### B. Decision-impact audit (what the user's decisions overturn)

Decision numbers: (1) parity, (2) workspace beside C++, (3) macOS/Linux + Windows-Docker headless, (4) two PRs docs then code, (5) D1, (6) D2, (7) D3, (8) D4, (9) D5, (10) winit+glutin+glow, (11) D6 fork.
All DISC paths are relative to DISC; "P1" paths absolute under hungry-kilby.

#### B1. Renderer stack (decision 10): sdl2 -> winit + glutin + glow
| File:line | Current text | Change |
|---|---|---|
| rust-architecture-map.md:20 | `gource-gl` depends on "render, sdl2, glow" | "render, winit, glutin, glutin-winit, glow" |
| rust-architecture-map.md:11 | "never imports a renderer, window, GL, or SDL type" | replace "SDL" with "winit/glutin" |
| rust-architecture-map.md:203-205 | "### 4.2 Stack decision / Use `sdl2` 0.38 + `glow` 0.18 ... Alternatives rejected: winit+glutin" | Rewrite: chosen stack winit 0.30.x + glutin 0.32.x + glutin-winit 0.5.x + glow 0.18, pinned exactly; record user override of recommendation A; keep window/context behind the `Renderer`/`GlContext` trait; note sdl2 now the rejected alternative |
| rust-architecture-map.md:67-68 | `display.cpp` -> `gource-gl/context.rs`, `sdlapp.cpp` -> `gource/shell_loop.rs` | fine; add that event pump is winit `ApplicationHandler` (0.30) not poll loop; this changes loop structure (inversion of control) |
| rust-architecture-map.md:342 | Windows row: "winit equivalents for HWND hacks are missing; with SDL2 most are covered" | Delete the SDL2 sentence and shrink the row: Windows = Docker headless export only (decision 3); drop console/HWND/DPI/PeekNamedPipe work |
| rust-architecture-map.md:343 | Linux: "request a core 3.2 context"; "with glow no GLEW exists" | keep; add glutin config selection (GLX/EGL) under Xvfb and Wayland |
| rust-architecture-map.md:335 | "`SDL_GetTicks` is replaced by `Instant::now()`" | still true; no change |
| deps-and-rust-mapping.md:139-140 | table rows A winit (listed first) / B sdl2 | mark A as chosen; B rejected |
| deps-and-rust-mapping.md:224 | risk 5 Windows "no direct winit equivalent" | mark out of scope (decision 3) |
| deps-and-rust-mapping.md:262-270 | "Option A: sdl2+glow ... Recommendation: A ... Option B: winit+glutin" | Add "Decision (2026-10-30): user chose Option B"; fold "Recommendation" under "Superseded"; the reasons (1)-(3) become the risk list in C |
| deps-and-rust-mapping.md:120 | distro packages list `libsdl2-dev libsdl2-image-dev` | C++-build only; add note that Rust needs none of them, only libpcre2 (feature) and freetype |
| deps-and-rust-mapping.md:217 | caveat about `sdl2` bundled/static-link | strike; add winit/glutin build notes |
| tests-and-golden-strategy.md:78 | "Run the Rust build the same way (wgpu with a Mesa lavapipe or other software adapter)" | "glutin + glow on llvmpipe (LIBGL_ALWAYS_SOFTWARE=1)"; wgpu is not the stack |
| parity-test-spec.md:166 | "The Rust binary runs the same command under wgpu with a software adapter (`WGPU_BACKEND=vulkan` + Mesa lavapipe, or `gl` + llvmpipe)" | "under glutin + glow with Mesa llvmpipe, request core 3.2"; remove WGPU_BACKEND |
| output-pipeline.md:218 | heading "## 8. Changes needed for a modern backend (wgpu)" | retitle "(glow, GL 3.2 core)"; items remain valid; remove wgpu-specific items |
| output-pipeline.md:338-341 | SDL2 window flags/DPI/frameless | add a winit mapping column (WindowAttributes, scale_factor, set_decorations) or defer to a new doc |
| output-pipeline.md:339 | "winit ..." Windows DPI note | out of scope per decision 3 |
| core-lib.md:233 | "winit/SDL2 ... visual equivalence is easiest with wgpu" | replace with decided stack |
| misc-traces.md:318 | "Rust/winit: request a HiDPI-capable surface" | still right; promote to a requirement |
| settings-cli.md:14-15,428-436 | SDLApp names in startup/console rows | no change needed (describes C++), but `--high-dpi` Windows rows become moot |
| LEDGER.md:66 (key-findings line "Rendering is legacy fixed-function GL ... redraw (wgpu or glow)") | "wgpu or glow" | "glow (decided)" |
| P1/docs/rust-rewrite/plan/LEDGER.md:77 | research topic "Rust: `sdl2` + `glow` GL 3.2 core ... snippets" | retarget to `winit`+`glutin`+`glow` APIs |
| P1/docs/rust-rewrite/plan/LEDGER.md:80 | CI matrix "Linux/macOS/Windows" | Linux/macOS + Windows-in-Docker |

#### B2. D6 fork vs patch file (decision 11)
| File:line | Current text | Change |
|---|---|---|
| parity-test-spec.md:128-134 (sec. 2.5 "Minimal C++ patch (reference side)") | items 1-6 describe a patch; item 6 "Guard with `#ifdef GOURCE_DEBUG_DUMP` or keep on a separate branch" | Retitle "Reference fork"; items become commits on the fork; drop the `#ifdef` alternative; fork location/name still undecided (LEDGER:119); add the guard from LEDGER:119 as item 7: fork must reproduce all custom-log and help goldens identically before its dump is trusted |
| parity-test-spec.md:217 (CI item 2) | "regenerates goldens from the patched C++" | "from the fork" |
| parity-test-spec.md:4 | "Reference binary = the C++ Gource v0.57 (built per BG)" | two reference binaries: ORACLE (untouched, byte-exact goldens) and FORK (state dump only); say which gate uses which |
| parity-test-spec.md:125 (`rand-script ... implemented in both binaries`) and :129 ("hidden from -h/-H so help goldens do not change") | patch hides options so help does not change | still valid but now the fork's help must equal the oracle's; keep dump options undocumented in help |
| tests-and-golden-strategy.md:65 | "Add `--debug-dump-state FILE` to the C++ reference and to Rust" | "to the fork" |
| rust-architecture-map.md:357 (phase 3 row) | "added to both C++ and Rust" | "to the C++ fork and Rust" |
| build-and-goldens.md (whole recipe) and DISC/golden/scripts/build.sh | builds one C++ tree (`BUILD=`) | add a second build target for the fork directory; script parameterised by BUILD already, document fork use |
| LEDGER.md:119 | fork location/name "still to be chosen" (now stale; LEDGER updated 2026-10-31) | done: `reference/gource-cpp-dump/`, a directory, not a branch |
| P1/docs/rust-rewrite/plan/decisions.md:13 | D6 row "C++ dump reference via a patch file applied to a scratch checkout" | replace with the fork decision |
| P1/docs/rust-rewrite/plan/LEDGER.md:83, 108 | "how the C++ side is produced given decision D6" / table row D6 default "patch file under `parity/`" | update default to fork; status decided |
| P1/docs/rust-rewrite/plan/BRIEFS.md:104 | "reference is produced under decision D6 as a patch file under parity/ applied to a ..." | fork |
| P1/docs/rust-rewrite/plan/LOOP.md:21 | "instrumentation ... is a user decision (D6 in the ledger)" | decided |
Also: fork copies src/ so decision 2 ("C++ stays until parity proven") and the repo size/GPL licensing need a note; and risk "fork drift" (C below).

#### B3. D5 bug fixes (decision 9) vs keep-the-bugs text and byte-exact goldens
| File:line | Current text | Change |
|---|---|---|
| rust-architecture-map.md:249 | "Help output must match the golden byte for byte (golden/help-short.txt, help-long.txt)" | match the golden except the D5 diff; add Rust-side goldens `golden-rs/help-*.txt` (or store a `.diff`) |
| rust-architecture-map.md:355 (phase 1 gate) | "byte-exact diff of -h, -H, ... against golden/help-short.txt, help-long.txt" | against oracle goldens modulo recorded diff |
| rust-architecture-map.md:371 | "`--log-format cvs-exp` unmatched: keep: explicit cvs-exp yields unsupported log format" | "fix: explicit cvs-exp selects the CVS-EXP parser" |
| rust-architecture-map.md:372 | "`--stop-on-idle` no-op: accept the flag, ignore it" | already compatible; mark "decided 2026-10-31" |
| rust-architecture-map.md:373 | "help text says user-friction 0.67 ... keep the help text verbatim (golden), implement 1.0" | "correct help text to 1.0" (reverse of today's text) |
| rust-architecture-map.md:367 (heading "Decisions to confirm and known quirks") | open-decision framing | retitle "Decisions taken"; add dir-name-position help row (missing today) |
| rust-architecture-map.md:396 (tick-4 "Confirmed unchanged" bullet naming the `rand()` sites) and :398 ("Open:" line) | "Open: D1 and D2 default/flag names need a user decision" | closed; keep only unverified rand() item |
| golden/help-long.txt:113 | "--user-friction SECONDS ... (default: 0.67)" | Rust help prints "default: 1.0" (confirm exact formatting vs code default 1.0 stored as 1/x) |
| golden/help-long.txt:128-129 | "(between 0.0 and 1.0, default is 0.5)" | "(between 0.1 and 1.0, default is 0.5)" to match `dir-name-position outside of range 0.1 - 1.0` (settings-cli.md:166) |
| golden/help-short.txt | check whether either option appears (not grepped: only help-long matched) | likely unaffected; confirm |
| golden/help-long.txt (log-format line) | help lists "git,svn,hg,bzr,cvs2cl,custom" (settings-cli.md:231) | with cvs-exp now working consider listing it (user did not ask; would be a third help diff) |
| log-formats.md:14 (item 7 "Bug to preserve or fix consciously") | preserve-or-fix wording | record decision: fixed |
| parity-test-spec.md:32 (sec. 1.3) | "explicit `--log-format cvs-exp` (must fail 'unsupported log format', the LF:14 bug is preserved)" | explicit cvs-exp must SUCCEED in Rust; the oracle still fails it, so this row becomes a documented deviation test (Rust golden generated by running the oracle with auto-detect on the same file) |
| parity-test-spec.md:24-25 (sec. 1.2 table) | `$G -h` stdout == golden for both binaries | split: oracle == golden; Rust == golden + expected diff |
| parity-test-spec.md:194 (matrix help row) | same | same split |
| LEDGER.md:70, :116-121 | bug list + "help-text goldens will differ ... record the expected diff" | add a pointer to the actual diff file once produced; `--stop-on-idle` decided 2026-10-31 (LEDGER Decisions) |
| P1/docs/rust-rewrite/phase1.md ("Decisions taken (ledger D1-D5)" section) | "D5 ... still yields unsupported log format ... Kept for parity, covered by a test" | now obsolete; replace with the fix |
| P1/crates/gource-formats/src/logmill.rs:68-69 | `"cvs" => attempt!(cvs_exp::...)` with comment "preserved C++ bug" | match `"cvs-exp"` (and keep `"cvs"` unreachable since settings reject it) |
| P1/crates/gource/tests/golden.rs:160-161 | expects "unsupported log format" for explicit cvs-exp | flip: expects success output; add a cvs-exp fixture |
| P1/crates/gource/tests/golden.rs:224-225 (`help_text_matches_golden`) | compares `-h/-H/--help/-?` to DISC golden files | point long-help compare at a Rust golden (or apply the diff) |
| P1/crates/gource-conf/src/help.rs (`include_str!`) | embeds the oracle help | embed corrected copies |
| P1/docs/rust-rewrite/plan/decisions.md:12 and plan/LEDGER.md:107 | "D5 keep known bugs" | fix |
Gotcha: the cvs-exp fix also needs the same fix in the FORK? Recommendation: NO, keep fork behaviour identical to the oracle except the dump hooks (LEDGER:119 guard); the Rust deviation is then tested by auto-detect equivalence.

#### B4. D1 / D2 wording (decisions 5, 6)
| File:line | Current text | Change |
|---|---|---|
| rust-architecture-map.md:170 | "intentional deviation D2 ... see sec. 7.2 and sec. 10 for flag and decision note" | remove "decision note"; state as decided |
| rust-architecture-map.md:275 | "**INTENTIONAL DEVIATION D1 (not C++ parity; needs user decision ...)** ... Flag: hidden `--strict-lazy-sections` (name proposed) restores" | drop "needs user decision"; D1 has no flag: delete `--strict-lazy-sections` (map:275 and :374) |
| rust-architecture-map.md:329 | "INTENTIONAL DEVIATION D2 (... needs user decision) ... Flag: hidden `--legacy-log-load` (name proposed)" | decided; an old-behaviour flag is OPTIONAL (the option text said "keep a flag if wanted"; the user did not specifically request one). Keep it only if the comparator needs it (see the parity-test-spec row below), and then fix the name and list it as hidden |
| rust-architecture-map.md:374-375 | "default needs user decision" in both rows | remove |
| rust-architecture-map.md:392, 398 | "with proposed hidden flags and an open-decision note" / "Open: D1 and D2 default/flag names need a user decision" | closed |
| LEDGER.md:81-86 section "Decisions flagged ..." | "Deviation from parity; decide before implementing" for both | mark decided; "behind flags or recorded as intentional differences" becomes "recorded as intentional differences" (a flag is optional for D2, none for D1) |
| parity-test-spec.md:58 (sec. 2.1 "Frame phase") | "Rust blocks on log load, so both are 0" | still correct; add that C++ fork aligns via `loaded_frame0` so D2 deviation is gated by the comparator; if the optional old-behaviour flag is kept, run it in one comparator job to prove the old behaviour |
| P1/docs/rust-rewrite/plan/LEDGER.md:103-104 | D1/D2 "(proposed name) / open" | decided |
| P1/docs/rust-rewrite/plan/decisions.md:8-9 | defaults | confirmed |
| timing.md:180 item 9(a) | frames during load advance runtime/framecount | unchanged description of C++; add D2 reference |
| settings-cli.md:489 gotcha (a) | lazy validation in `getNext()` | unchanged description; add "Rust validates up front (D1)" |

#### B5. D3 PRNG (decision 7) and D4 --ffp (decision 8): mostly consistent, small edits
| File:line | Current | Change |
|---|---|---|
| rust-architecture-map.md:354 (phase 0 gate) | "glibc-mode PRNG first outputs ... (once confirmed against a real libc)" | keep; add "and the macOS compatibility mode vs a recorded macOS libc sequence" |
| rust-architecture-map.md:307-317 (7.1) | "Optional compatibility modes, selected by hidden flag or env" | decisions say "compatibility modes for parity tests": fix the selection mechanism (env `GOURCE_RS_PRNG` vs hidden flag) in one place; SPEC:124 says env |
| parity-test-spec.md:122-127 (2.4) | glibc/macOS algorithms `unverified` | still true; see risks R11/R12 |
| rust-architecture-map.md:209 | "decide: either accept `--ffp` as a no-op alias ... or fail" | decided: accept and ignore; also add that the `P` key toggle (app-core.md:294, 554) becomes a no-op |
| rust-architecture-map.md:376 | "accept the flag; render through the shader path" | "accept and ignore" |
| core-lib.md:13; output-pipeline.md:78, 240; settings-cli.md:212 | describe `--ffp` first-class / "recommended to drop" | add a one-line "Rust: accepted and ignored (D4)" |

#### B6. Windows / delivery (decisions 3, 4, 2)
| File:line | Current | Change |
|---|---|---|
| rust-architecture-map.md:342 | whole Windows row (console, high-DPI, PeekNamedPipe, `dunce`, binary stdout) | collapse to: Windows = Docker headless frame export on Linux userland; no native Windows build in first release |
| rust-architecture-map.md:361 (phase 7) | "`cargo build` on Linux/macOS/Windows" | Linux/macOS native; Windows via Docker image |
| rust-architecture-map.md:165-168 | stdin pump thread chosen because "portable to Windows, where the C++ uses PeekNamedPipe" | rationale gone; plain `O_NONBLOCK` or pump thread both fine; simplify |
| rust-architecture-map.md:342 (inside the Windows row) | "stdout set binary for PPM" | drop with the row |
| settings-cli.md:41,43,129,245,261,432-436,541; core-lib.md:82; output-pipeline.md:339-341; misc-traces.md:307-311 | C++ Windows behaviours | no edit required (descriptive); add one-line scope note in the architecture map so no one ports them |
| DISC/LEDGER.md:57 (checklist item "Build/packaging/platform specifics") and :100 ("Windows-only paths ... catalogued from source only") | residual item | mark out of scope for first release |
| docs delivery: LEDGER.md "Decisions" (lines 111-121) + CONTEXT.md | no mention of how docs/code split across the two PRs | add: PR #2 = docs (discovery + CONTEXT.md; merged), PR #3 = phase 1 code and planning kit (P1 branch); both against master; note C++ tree stays in place (decision 2) so the Rust workspace dirs (`Cargo.toml`, `crates/`) coexist with autotools files |
| P1/Cargo.toml `[workspace]` `members = ["crates/*"]` | fine | check `Makefile.am`/`gource.pro` ignore the new dirs and `.gitignore` covers `target/` |

#### B7. Other staleness found
- DISC/LEDGER.md:34 mentions "9 phases" in the architecture-map row (P1 plan/LEDGER.md:114 already flags MAP has phases 0-7; fix in the discovery table).
- phase1.md "Status" paragraph still says `help-short/long` match the oracle goldens: true today, false after D5.
- P1/docs/rust-rewrite/plan/ is an UNFINISHED planning loop (plan/LEDGER.md:126 "Stop rule: NOT MET") whose phase list ("Every remaining phase (1, 3, 4, 5, 6, 7)", :36) matches the MAP numbering but whose decisions table is pre-decision; it ships in PR #3 (it exists only on that branch), not PR #2; update it on a new branch from `master` after PR #3 merges.

---------------------------------------------------------------------------
### C. Risk register

L = likelihood (L/M/H), I = impact (L/M/H). Phase = where it bites (MAP numbering).

| ID | Risk | L | I | Mitigation | Phase |
|---|---|---|---|---|---|
| R1 | glibc `rand()` TYPE_3 values and macOS Park-Miller assumed from a Python model / general knowledge (LEDGER residual list; SPEC:123-124, :223) | M | H | write a 20-line C helper, run on Linux (Docker) and this Mac, commit first 10 000 outputs as vectors BEFORE coding Glibc/Park modes; macOS Park-Miller is confirmed by one compiled test but not archived; glibc is reproduced only by a Python model (310 discards, never run against real glibc) | 0, 3 |
| R2 | `vec2(rand()%100-50, rand()%100-50)` evaluation order is unspecified in C++ | M | M | pin clang x-first (MAP:316); record compiler in golden header; add a Script-mode test that would catch swap | 3 |
| R3 | Fork drift: fork of src/ diverges from the oracle/master as upstream C++ changes or someone fixes bugs there (decision 11) | M | M | fork lives in a fixed dir, pin base SHA, guard test: fork reproduces all oracle custom-log/help/exit-code goldens (LEDGER:119); never apply D5 fixes to the fork | 3 |
| R4 | Fork is extra maintenance AND the fork needs a GL window to run (no headless C++ sim, TGS:42), so goldens need this Mac or Xvfb/Docker; Linux build of the fork never tried | M | H | prove fork build on Linux in Docker early (action 2); consider a no-render stub in the fork (skip `draw`) | 3 |
| R5 | Help-text golden diff: Rust help intentionally differs on 2 option lines (help-long.txt:113, :128-129), breaking the byte-exact gate and the existing test `help_text_matches_golden` | H | L | keep oracle goldens untouched; store Rust goldens + `.diff`; gate on `oracle_help + diff == rust_help` | 1 |
| R6 | D5 `--stop-on-idle` interpretation (confirmed 2026-10-31: accepted, no effect; closed) | L | L | documented as having no effect | 1 |
| R7 | glutin 0.32.3 (2025-04-30) and glutin-winit 0.5.0 (2024-06-08) are slow/stale; they must track winit 0.30.x; a winit minor bump can break the trio (DEP:175-177, LEDGER) | M | M | pin exact versions with `=`; commit Cargo.lock; keep context behind `GlContext` trait so swapping to raw EGL/CGL or sdl2 is local; vendor-check at each release | 5 |
| R8 | winit/glutin input, IME, clipboard, cursor-grab, key-repeat and DPI semantics differ from SDL (DEP:266) and winit 0.30 uses the `ApplicationHandler` model (loop inversion) | H | M | write an input table (SDL event -> winit event) before phase 6; add explicit input/DPI tests; use `scale_factor` for `viewport_dpi_ratio` (misc-traces.md:307-318) | 5, 6 |
| R9 | macOS GL capped at 3.2 core and deprecated: needs forward-compat, `#version 150`, no `GL_QUADS/GL_ALPHA/glLineWidth>1` (MAP:341) | H | M | design shaders for 150 from day one (done in MAP:229-241); no wgpu fallback needed unless Apple removes GL | 5 |
| R10 | Headless export inside Docker for Windows: needs a GL context with no display (Xvfb/llvmpipe, or EGL surfaceless/pbuffer). glutin support for surfaceless/pbuffer contexts and the exact Mesa recipe are UNVERIFIED; software GL is slow | M | H | spike early: glutin+EGL in a Debian Docker image, render one frame, read back to PPM; if blocked, run Xvfb inside the image | 5, 7 |
| R11 | f32 determinism: Rust must mirror C++ f32/f64 narrowing and avoid FMA; libm differences (`sinf/cosf` in rotate) across platforms; Rust does not contract FMA by default but LLVM/target-feature flags could change it | M | H | `-ffp-contract=off` on fork (SPEC:141), mirror narrowing per SM 10.1, tolerance tier for libm fields, CI check that no `target-cpu=native` flags | 3 |
| R12 | RNG is platform dependent: oracle on macOS uses Park-Miller, on Linux glibc, so goldens are per-platform; Script mode needed in BOTH binaries and the fork must wrap all four `rand()` sites | M | H | Script mode as primary matrix mode (SPEC:125); `rand_calls`/`first_rand_frame` in every record | 3 |
| R13 | Quadtree visit order and floating-point accumulation order (SPEC:144-151) is the most likely source of silent divergence; O(n^2) shortcuts are not equivalent | H | H | port quadtree verbatim with order comments; real-repo golden (200+ commits) early; sabotage checks | 3 |
| R14 | Sim gate relies on a fork that has never been compiled with the dump hooks ("nothing was compiled", SPEC:224); hook location (after gource.cpp:265) and uninitialised-field zeroing are designs only | M | M | do the fork + one-fixture dump before writing Rust sim | 3 |
| R15 | Log-load phase (D2): C++ advances `runtime/framecount` during async load; fork's `loaded_frame0`/`runtime_at_load` alignment assumes tiny fixtures load within a whole number of steps (SPEC:57-59) | M | M | assert in comparator; run fork with log pre-cached; if the optional `--legacy-log-load` flag is kept, use it here | 3 |
| R16 | PCRE2 in CI: default `cargo test` never exercises PCRE2 (phase1.md); `pcre2-sys` needs a C compiler/libpcre2 on CI | M | M | CI matrix leg with `--features pcre2`; turn it on in phase 1 import | 0, 1 |
| R17 | Regex semantics: internal patterns compiled on `regex::bytes` Unicode-off while C++ uses PCRE2; `$` semantics differ (MAP:394 note) | M | M | keep regex-vs-PCRE2 equivalence tests for every internal pattern against the oracle on the edge corpus | 2 |
| R18 | Real VCS goldens (git/svn/hg/bzr) and edge goldens do not exist; parser parity is only proven on 5 tiny fixtures | H | M | generate `tests/golden/{edge,real}` from the oracle (action 3) | 2 |
| R19 | StreamLog diverges: blocking read vs C++ O_NONBLOCK; SIGPIPE ignored; >100 MiB SeekLog read whole; TinyXML CDATA/entity differences (phase1.md) | M | L | document as accepted deviations or fix when settings import lands; add tests for stdin partial-line | 2 |
| R20 | Multi-repo `[gource]` sequencing is untraced and tests/test.conf has no expected output (LEDGER residuals; SPEC:225) | M | M | produce goldens from the fork on test.conf; per-repo reset set (MAP sec. 5.3 item 8) | 4 |
| R21 | Captions absent from the dump schema; caption timeline unverified | M | M | extend schema (SPEC:226) before phase 4 | 4 |
| R22 | Rendering gate thresholds (SSIM 0.97/0.90) are guesses; Linux-only; macOS has no llvmpipe; FreeType hinting vs Rust text stack differs; no render golden exists | M | M | measure C++ run-to-run spread first; structural set hides text; use `freetype-rs` for metrics (MAP:346) | 5 |
| R23 | PPM/SIGPIPE behaviour of the oracle never exercised; Windows paths catalogued from source only; empty-stdin startup and `--stop-at-end` with stdin read from source only (LEDGER residuals) | L | L | exercise with oracle once; Windows now out of scope | 2, 5 |
| R24 | Oracle build fragility: C++ builds only via clang++ `-arch x86_64` under Rosetta with Homebrew-Intel libs (BG); no CI reproducibility; fork doubles it | M | M | Dockerised Linux build of oracle and fork for reproducible goldens; record flags and SHA in golden headers | 3 |
| R25 | Two-PR delivery: PR #2 (merged) carried the decisions but not the Part 6 B doc edits; PR #3 (phase 1) must not regress goldens; reviewer load | M | L | action 1 lands the doc edits on a branch from `master`; follow gh-publish and PR conventions | docs, 1 |
| R26 | Licensing: fork + new crates under GPLv3-or-later; transitive crate licences unchecked (LEDGER residual); freetype/pcre2 native deps | L | M | `cargo deny` in CI | 7 |
| R27 | Crate maintenance judged only by release date, not issue activity (DEP:217) | L | L | re-check before release | 5, 7 |
| R28 | Stale state: the main checkout is still mid-merge of the old discovery tip with an older copy of the docs; P1 plan/ folder is in an unfinished state | M | M | the user aborts the paused merge (`:cq`, `git merge --abort`); keep doc edits out of the code PR | all |
| R29 | Platform coverage: every golden was captured on macOS arm64 running x86_64 binaries under Rosetta; Linux (`rand()` TYPE_3, GL, fonts, the oracle/fork build) was never run, so Linux goldens and the glibc mode are unverified | H | H | first Linux run (Docker) in actions 2 and 4; do not call glibc mode or Linux goldens verified until then | 0, 3, 5 |

---------------------------------------------------------------------------
### D. Next ten concrete actions (each doable in one session, in order)

1. Apply the decision edits to the docs (section B1-B6) on a new branch from `master` (the docs are already merged): rewrite MAP 4.2 and the stack rows, DEP 262-270, SPEC 2.5/3.1/1.3/1.2, MAP 371-375/392/398, TGS 65/78, LEDGER decisions; add a "Decisions (final)" table with all eleven; and add the expected help diff note. Commit as a new docs PR. Done when `grep -nE "sdl2|wgpu|patch|needs user decision|keep:" ` over the touched docs returns only intentional "superseded" mentions.
2. Create `reference/gource-cpp-dump/` (decided 2026-10-31; does not exist yet) with a README naming the commit forked from, copy `src/` verbatim at a pinned SHA with the `src/core` files copied in (not a nested submodule), build it with the existing recipe (DISC/golden/scripts/build.sh with `BUILD=`), and run the guard: the fork reproduces `golden/custom-log/*`, `help-*.txt`, `exit-codes.txt` exactly. Also try the same build inside a Linux Docker image (risk R4/R24).
3. Generate the missing parser goldens from the ORACLE (not the fork): `tests/golden/edge/*` (empty, CRLF, BOM, out-of-order, `|` in names, start/stop date under TZ=UTC and Pacific/Auckland) and pick/pin real git/svn/hg/bzr repos for `tests/golden/real/*`. Wire them into `crates/gource/tests/golden.rs`.
4. Phase 0 completion in P1: add `prng` (default xorshift, `Glibc`, `Park`, `Script`, call counter), `bounds`, `quadtree`, `math::rotate_vec2` and `PI`; first write a C helper for libc `rand()` on this Mac and in Docker/Linux and commit 10 000-value vectors (R1); add `.github/workflows` (cargo test, `--features pcre2`, fmt, clippy); confirm the `regex_tests.cpp` port exists or add it.
5. Phase 1 core in P1: `conffile.rs` (grammar, first-value rule, multi-section, error text) and typed `settings_display/settings_gource` import with all validations and the D1 up-front section check; wire `--load-config`, positional conf detection, `--save-config`. Gate with a `--save-config` round-trip matrix captured from the oracle.
6. Apply the D5 code changes in P1: fix `logmill.rs:68-69` (cvs-exp), flip the test at `golden.rs:160-161`, correct help text (user-friction default 1.0, dir-name-position 0.1) as Rust-side golden copies with a committed `.diff` against the oracle goldens, update `help_text_matches_golden` (golden.rs:224-225) and `phase1.md`; keep `--stop-on-idle` accepted/no-op; ask the user once to confirm that. Commit after PR #3 merges.
7. Implement the fork's dump hooks (commit series on the fork): `--debug-dump-state/-every/-max-frames`, `--rand-script`, `gource_rand()` wrapper for the four `rand()` sites, zero-init of the uninitialised fields, `dumpState()` after `logic()` at gource.cpp:265 (SPEC 2.2/2.5). Produce one golden (`file-removal`, Script mode) on this Mac and one in Linux Docker; write the comparator (`xtask` or python) with first-divergence reporting.
8. Phase 3 slice 1 in P1: `gource-sim` DirTree arena, File, User, Action, Pawn, `SimRng` use, `Gource::logic` for ingestion and idle/removal on `file-removal.log` with `step(dt)` and the dump writer; get the exact tier (topology, add/remove frames, rand_calls) green in Script mode, then extend to forces (quadtree order) and the remaining fixtures; run the sabotage check on each gate.
9. Renderer spike (parallel, any time after action 1): in `gource-gl` pin `winit`, `glutin`, `glutin-winit`, `glow`; open a GL 3.2 core context on macOS and in Linux under Xvfb/llvmpipe, draw one textured quad with a `#version 150` shader, read back and write a valid P6 frame via the sync_channel PPM writer; then test the same binary headless in Docker with no display (R7, R9, R10). Record what works in a short doc and settle the `GlContext` trait.
10. Phase 4 and visuals plan refresh: extend the dump schema with captions and the multi-section shell, generate multi-section goldens for `tests/test.conf` from the fork, capture C++ reference frames under Xvfb+llvmpipe and measure the run-to-run SSIM spread to fix the visual thresholds; then start phase 4 (camera, rotate, selection, shell) behind that gate.

(The first three are doc/infra and can be done by one successor without writing Rust; 4-5 can be split across two sessions in separate worktrees because they touch disjoint crates; 7 and 9 are independent of 4-6.)

# Part 7: Digest of the discovery documentation

## Digest A: app-core, scene-model, timing, verification-tick3-scene

Docs dir: <discovery-worktree>/docs/rust-rewrite/discovery/
Cites are as the docs give them: src/ paths, `gource.cpp:N` = src/gource.cpp:N unless prefixed. f32/f64 = C++ float/double.
Precedence: timing.md is authoritative on dt (it says it supersedes dt claims in scene-model/app-core). verification-tick3-scene.md corrects scene-model.md and app-core.md.

---------------------------------------------------------------
### 1. app-core.md (711 lines)
**Purpose:** Discovery notes on the application core and main loop: src/gource.cpp/.h, gource_shell, main, logmill, key, slider, textbox, caption, zoomcamera, bloom, spline. Part 3 (head, smaller files) + Part 1 (gource.h, gource.cpp 1-1400) + Part 2 (gource.cpp 1400-2776) + "Tick 2 additions" (T2.1-T2.9) + "Verification (tick 4)".

**Section map:** Part 3: main.cpp (238 lines); gource_shell.cpp/.h (314/65); logmill.cpp/.h (337/71); key.cpp/.h FileKey; slider; textbox; caption; zoomcamera; bloom; spline; randomness note. Part 1: 1 Globals; 2 class Gource fields; 3 public methods; 4 ctor option->field mapping; 5 timing model; 6 commit ingestion; 7 reset(); 8 input mapping; 9 camera constants; 10 bounds/quadtree; 11 captions; 12 randomness/time; 13 error messages; 14 Rust-port hazards. Part 2: 1 functions in range; 2 logic() order (25 steps); 3 draw() order (24 steps); 4 camera (updateCamera); 5 selection/hover (mousetrace); 6 kbd/mouse in range; 7 caption/timeline/slider; 8 magic numbers; 9 errors; 10 sim-state flags. Tick 2: T2.1 per-frame driver; T2.2 input bindings; T2.3 frame flow; T2.4 commit consumption/non-monotonic timestamps; T2.5 camera; T2.6 member classification; T2.7 render-path mutates sim?; T2.8 randomness; T2.9 gaps. Verification (tick 4).

**Key facts (quoted):**
1. update (gource.cpp:246-285): `scaled_dt = min(dt, max_tick_rate)` (248); exporter: `scaled_dt = max_tick_rate` (251-253); `*= gGourceSettings.time_scale` (256); `if(!paused) runtime += scaled_dt` (259); `stop_at_time > 0.0 && runtime >= stop_at_time` -> stop_position_reached (261); `logic(runtime, scaled_dt)`; `draw`; dump if `framecount % (frameskip+1) == 0` (272-276); cursor uses REAL dt (278-282); framecount++ (284). Param `t` ignored.
2. setFrameExporter (505-522): `g=fps; while(g<60){ g+=fps; frameskip++; }`; `max_tick_rate = 1.0f/(float)g`; 60->g60 skip0; 30->g60 skip1; 25->g75 skip2 (1/75); 24->g72 skip2; 120->g120 skip0. fps<=0 -> infinite loop. max_tick_rate=`1.0/60.0` (cpp:156, f64 narrowed to f32).
3. logic order (1568-1878): 1570 shutdown check; 1575 message_timer/splash -= dt; 1579-1607 init (wait logmill; error -> throw SDLAppException; no log -> exporter stop, SDL_Quit, SDLAppException("") showHelp; start_position>0 -> seekTo); 1609 file_key/slider logic; 1613-1652 polled arrows (cursor_move = +/-10, assignment not +=, order right,left,up,down; sets manual_camera); 1655-1685 apply rotate_angle (sinf/cosf; about camera pos if manual_rotate else origin; rotates dir tree + every user); 1687 recolour->changeColours; 1693-1699 if paused: updateBounds, interactUsers, interactDirs, updateCamera, return; 1702 readLog if queue empty; 1707-1713 loop restart; 1715-1720 first-time `currtime=lasttime=queue[0].timestamp; subseconds=0; loadCaptions()`; 1722-1733 clock; 1736 delete gGourceRemovedFiles; 1744-1774 commit loop; 1776-1860 captions; 1867 updateBounds; 1869 interactUsers; updateUsers; 1872 interactDirs; updateDirs; 1875 updateCamera; 1877 `updateTime(!commitqueue.empty() ? currtime : lasttime)`.
4. Clock (1722-1733): `time_inc = (dt * 86400.0 * days_per_second)` stored in float; `seconds=(int)time_inc`; `subseconds += time_inc - (float)seconds`; if `subseconds >= 1.0` {`currtime += (int)subseconds; subseconds -= (int)subseconds`}; then `currtime += seconds`.
5. Commit loop (1744-1774): copy `commitqueue.front()`; auto-skip (1749) `auto_skip_seconds>=0.0 && idle_time>=auto_skip_seconds && !stop_position_reached` -> `currtime=lasttime=commit.timestamp; idle_time=0.0`; `if(commit.timestamp > currtime) break;` (1754); processCommit; `no_time_travel`: `lasttime=max(lasttime,ts)` else `if lasttime > commit.timestamp -> currtime = commit.timestamp`, then `lasttime = commit.timestamp`; `subseconds=0.0`; pop_front. No sorting; head-of-line blocking.
6. readLog (1131-1186): loop cond (1137) `(hasBufferedCommit() || !isFinished()) && (commitqueue.empty() || (commitqueue.back().timestamp <= currtime && commitqueue.size() < commitqueue_max_size))` (max 100); `stop_timestamp != 0 && commit.timestamp > stop_timestamp` -> stop_position_reached, commit discarded (1148-1151); `first_read && empty` -> `SDLAppException("no commits found")` (1156-1158); stop: `(stop_at_end && is_finished) || (stop_position > 0.0 && seekable && (is_finished || last_percent >= stop_position))` (1170-1178); `file_idle_time = file_idle_time_at_end` permanent mutation (1180-1182).
7. processCommit (1188-1242): dir entries (name ends '/') honour only "D" via `root->findDirs` + `getFilesRecursive`; else absent file -> `addFile(cf)` REGARDLESS of action; addFile returns 0 -> skip. addFile (991-1019): `max_files>0 && files.size()>=max_files` -> 0; `root->isDir(file_as_dir)` -> 0; `tagid = tag_seq++` (shared with users); after `root->addFile`: `while(root->getParent()) root = root->getParent()`. addFileAction (1244-1290): "D"->RemoveAction, "A"->CreateAction, anything else->ModifyAction. User created lazily at first surviving file action; spawn pos `dir_bounds.centre()` if `dir_bounds.area() > 0` else (0,0) (1021-1041).
8. Quadtree (interactUsers 1292-1333, interactDirs 1418-1449): bounds = bounds +/-(1.0,1.0); `max_depth=1`, `gGourceMaxQuadTreeDepth` (6, cpp:23) iff `dir_bounds.area() > 10000.0`; `QuadTree(bounds, max_depth, 1)`; rebuilt every call (also while paused); users in map order, dirs in gGourceDirMap (path-sorted) order, only non-empty dirs.
9. Camera (updateCamera 1475-1550): start (0,0,-300) (`starting_z=-300.0f`, 140/148); manual: `cam_rate = -cam.z / 5000.0f; delta = cursor_move * cam_rate * 10.0f`; no `camera.adjust` while manual_camera; auto bounds: selection -> selected user pos/file abs pos; else `track_users && idle_time==0` -> active_user_bounds; else dir_bounds; `camera.adjust(cambounds, !manual_zoom)`; auto-rotate: `angle_rate = max(dt, 1 - fabs(remaining/90 - 0.5)*2) * dt; rotate_angle = min(remaining, 90*angle_rate)`; trigger `!cursor.rightButtonPressed() && dir_bounds.area() > 10000.0f`, `ratio = aspect>1 ? bw/bh : bh/bw`, `ratio < 0.67` -> remaining=90. Applied NEXT frame's step 6.
10. ZoomCamera (zoomcamera.cpp): fov=90 (:24), znear=0.1, up=(0,-1,0), zfar=max_distance+1, padding 1.0 (overwritten), speed 1; `adjust` (96-154): `toa = tan(fov*0.5*DEGREES_TO_RADIANS)*2.0`; aspect<1 -> height/=aspect else width/=aspect; distance = width/toa if crop_vertical, height/toa if crop_horizontal, else max(width,height)/toa; clamp [min,max]; `dest.z=-distance`. `logic` (170-188): `dpt=dp*dt*speed`; lockon: `dpt = dpt*lockon_time + dp*(1-lockon_time)`, `lockon_time = max(0, lockon_time - dt*0.5)`; clamp length; `pos+=dpt`; `target=(pos.x,pos.y,0)`. Zoom key (400-419): multiplier float 1.1, `distance=-camera.getDest().z`, clamp [camera_zoom_min, camera_zoom_max].
11. Input keys with sim effect: N `idle_time = auto_skip_seconds` (721); Z gGourceGravity toggle; S recolour; Space pause; `=`/`+` days_per_second `>=1.0 -> min(30.0, floorf(d)+1)` else `min(1.0, d*2)`; `-` `>1.0 -> max(0, floorf(d)-1)` else `max(0, d*0.5)`; `.` time_scale `>=1.0 -> min(4.0, floorf(t)+1)` else `min(1.0, t*2)`; `,` `>1.0 -> max(0, floorf(t)-1)` else `max(0.25, t*0.5)` (1.5 -> 0.0); `/` time_scale=1.0; `[`/`]` gGourceForceGravity /=,*= 1.1 (double op narrowed to f32). Mouse RMB drag: `rotate_angle = min(1,|d|/10) * 5 * DEGREES_TO_RADIANS`, replaces not accumulates (366-377).
12. Randomness: `rand()` sites gource.cpp:1555 (`gStringHashSeed=(rand()%10000)+1`), user.cpp:106,128, dirnode.cpp:625; only srand is gource_settings.cpp:1319 (`--start-position random`). Otherwise libc default seed; port must reproduce libc rand() and call order. gStringHashSeed default 31 (gource_settings.cpp:489).
13. gource_shell: `min_delta_msec = 16` (gource_shell.cpp:29); getNext (205-251): `transition_interval=1.0f`; multi-repo: if `repo_count>1` and `stop_at_time<=0 && stop_position<=0` -> `stop_at_time = 60.0f` (231-233); with exporter `stop_at_end=true` unless `dont_stop || loop || path=="-"` (224); wraps to first repo when `repo_count>1 && exporter==0`. update: if no gource or `isFinished()` -> getNext() else `appFinished=true`, return (nothing drawn).
14. logmill: states STARTUP=0, FETCHING=1, SUCCESS=2, FAILURE=3; auto-detect ORDER (logmill.cpp:273-332): git, mercurial, bzr, git raw, cvs-exp, svn, cvs2cl, custom, apache (first checkFormat success wins); findRepository (163-187) checks `.git`, `.hg`, `.bzr`, `.svn` in that order within a dir, later overwrite earlier, nearest ancestor wins; `--start-date` skip loop reads until `commit.timestamp >= start_timestamp` then buffers it (86).
15. Spline (spline.cpp:23-69): `edge_detail = min(10,(int)(ang*100.0))`, floor 1; quadratic Bezier; ribbon half-width 2.5; label pos formula (63-68). Caption: `fade_in = min(2.0, caption_duration/3)`; `alpha = min(1, min(elapsed, max(0, caption_duration - elapsed)) / fade_in)`. Message timer 5.0 s (2231). Screenshot `gource-%04d.png` 1..9999.

**Gotchas:** zoom_multi is float 1.1 (corrected); `-` from exactly 1.0 -> 0.5 but `+` from 1.0 -> 2.0 (asymmetric); `,` from 1.5 -> 0.0 despite 0.25 floor; paused path skips updateUsers (idle_time/deletes/follow-select frozen) but camera/bounds/quadtrees keep updating; `reset()` does NOT reset runtime, paused (seekTo clears it), stop_position_reached, first_read, framecount, splash, settings mutations, gGourceGravity/ForceGravity; mousepos uninitialised in ctor; mouse picking uses pointer-ordered std::set (nondeterministic, selection only); mousetrace in draw sets selection that drives camera in logic; Gource::update ignores `t`; spline NaN hazard (normalise zero vector then (int)NaN) - port should clamp detail to 1; font_scale integer division (render only); `commit_seq` and `use_selection_bounds` dead; camera distance depends on display.width/height (aspect must be identical); readLog can spin on seekable logs when nextCommit false (1141-1146).

**Verification status:** Tick 4 checked ~75 cites/claims, 0 wrong: all 40+ input rows T2.2, T2.3 logic order, T2.4 commit loop, T2.5 updateCamera 1475-1550 verbatim, all gource.h member lines 53-193, rand() sites, sdlapp.cpp 317/352-358/422-460. Part 1/2 were spot-checked and CORRECTED in Tick 2. Later corrections from tick3: `time_inc` is float (app-core 242/356; line 356 already fixed, 242 says f64? see tick3 row 2); `toa` = 1.99999988 not 2.0 (app-core.md:88 still says "=2.0"); head-of-line break is at 1754 (tick3 says app-core.md 614 cites 1757; the doc text at T2.4 table row cites 1754).
**Unconfirmed:** dirnode.cpp:895 logic-body ordering and Part 1 sec 6/10 cross-refs; gource.cpp:1984 selectBackground call site.
**Open questions/gaps:** RCommitLog ordering/dedupe in logmill/seeklog/formats and per-class logic are in other docs; Regex::match group indexing in loadCaptions assumed matches[0]=group 1; gource_settings defaults not covered here; bloom/key/slider/textbox are render/UI only.
**Conflict:** app-core T2.1 says min_delta default 8 at sdlapp.cpp:317 and 16 via gource_shell.cpp:29; timing.md resolves: effective 16.

---------------------------------------------------------------
### 2. scene-model.md (383 lines)
**Purpose:** Simulation model of RDirNode, RFile, RUser, Pawn, RAction with f32/f64 tags, force/layout formulas, constants index, hazards for bit-exact port.
**Section map:** 0 Frame pipeline (container iteration order table); 1 Shared math helpers; 2 Pawn; 3 RDirNode (3.1 fields, 3.2 path/tree structure, 3.3 update propagation/visibility, 3.4 radius and file layout, 3.5 force model, 3.6 colour, 3.7 quadtree use); 4 RFile; 5 RAction; 6 Commit->actions; 7 RUser (7.1 forces/movement, 7.2 colour/alpha/lifetime); 8 Constants index; 9 Randomness/time; 10 Float/int/ordering hazards; 11 Rendering rules; 12 Gaps; Tick 3 corrections.

**Key facts (quoted):**
1. Helpers: `PI` = `#define PI 3.14159265` (f64 literal, core/pi.h:31); `normalise(v)`: `l=length(v); l>0 ? v/l : v` (core/vectors.cpp:7-23); `rotate_vec2(v,s,c) = (v.x*c - v.y*s, v.x*s + v.y*c)` (vectors.cpp:3); `stringHash`: `val=0; for i<n: val += s[i] * (gStringHashSeed ^ (n-i))`; `^` is XOR; signed char; int wrap; `if(val<0) val=-val` (stringhash.cpp:32-46); seed default 31. `vec2Hash`: `x=((h/7)%255)-127; y=((h/3)%255)-127; normalise(vec2(x,y))`. `colourHash`: `if(h==0) h++; r=(h/7)%255; g=(h/3)%255; b=h%255; normalise(vec3(r,g,b))` (stringhash.cpp:71-83). Bounds2D::overlaps inclusive; contains false if empty (core/bounds.h:108-125).
2. Containers: users `std::map<string,RUser*>` name-sorted; files map fullpath-sorted; `gGourceDirMap` abspath-sorted (trailing '/'); `RDirNode::children`/`files` std::list insertion order; `gGourceRemovedFiles` vector; `RUser::actions/activeActions` list FIFO.
3. Pawn defaults (pawn.cpp:22-46): speed 1.0, elapsed 0, fadetime 1.0, nametime 5.0, name_interval 0, graphic_ratio 1.0, shadowOffset (2,2). `getAlpha() = min(elapsed/fadetime, 1.0)`. `updateQuadItemBounds`: half=`size*0.5`; box `pos -/+ (half, half*graphic_ratio)`. `gGourceShadowStrength=0.5`. user.png 384x512 -> ratio 1.3333; file.png 256x256 -> 1.0.
4. RDirNode constants (dirnode.cpp:20-26): gGourceForceGravity=10.0, gGourceDirPadding=1.5, gGourceFileDiameter=8.0 (file.cpp:20), gGourceMinDirSize=15.0 unused. `file_area = (float)(r*r*PI)`, r=4.0 -> f64 50.2654824 (f32 stored 50.26548386 per tick3). Ctor ignores parent arg (parent=0 after ctor, dirnode.cpp:38-39); pos=(0,0).
5. Tree: compressed trie of dir paths. addFile algorithm (dirnode.cpp:377-496) 9 steps: non-matching path at root -> new parent at `commonPathPrefix` ("" -> "/"); empty root with abspath "/" adopts f->path (changePath, 400-404); exact dir -> push_back, visible_count++ only if not hidden; try children in order; file-becomes-directory forces `g->remove()`; new leaf dir; split to create intermediate dir (first child whose common prefix `size() > abspath.size() && common != f->path`; `cnode->setPos(commonPos)`; move all children `prefixedBy(commonpath)` in list order). removeFile (275-320): empty dirs reaped bottom-up (child `noFiles() && noDirs()`); intermediate dirs never merged back; root never deleted. `getFilesRecursive` PREPENDS (later dirs' files precede earlier); `findDirs` stops at first prefix match.
6. Radius (calcRadius 574-594): `total_file_area = file_area * visible_count; dir_area = total_file_area + sum(child.dir_area); dir_radius = max(1.0f, (float)sqrt(dir_area)) * gGourceDirPadding; parent_radius = max(1.0f, (float)sqrt(total_file_area) * gGourceDirPadding)` (1.0 floor after x1.5 for parent_radius only). Children's dir_area may be stale.
7. File layout (updateFilePositions 842-880): `max_files=1; diameter=1; file_no=0; d=0.0f; files_left=visible_count`; on ring full: `diameter++; d += gGourceFileDiameter (8.0); max_files = (int)max(1.0, diameter*PI)` (6,9,12,15,18...); `if files_left < max_files: max_files = files_left`. `calcFileDest`: `arc = 1.0/(float)max_files; frac = arc*0.5 + arc*file_no; dest=(sinf(frac*PI*2.0), cosf(frac*PI*2.0))` (arg in f64).
8. Dir forces (604-714): applyForceDir: `posd2 - sum*sum > 0 -> return`; `distance = posd - r_me - r_node`; `posd < 0.00001` -> `accel += normalise(vec2(rand()%100 - 50, rand()%100 - 50))`; else `accel += distance * normalise(dir)`. applyForces: children recursed first (DFS), root returns before own; per non-root dir order: (1) quadtree DirForceFunctor, (2) applyForceDir(parent), (3) gravity `parent_dist = |parent.pos - pos| - (dir_radius + parent.parent_radius); accel += 10.0 * parent_dist * normalise(parent.pos - pos)`, (4) if grandparent: `accel += (parent.pos + (parent.radius + radius)*edge_n) - pos`, (5) siblings: `slice = (float)((parent.radius * PI) / (float)(visible+1))`, `accel += sib_accel * slice` if visible>1.
9. move (799-825): root returns; `if !empty() && !position_initialized: setInitialPosition`; `pos += accel*dt`; elasticity (default 0.0) branch; `accel=(0,0)`. No velocity/damping. setInitialPosition (780-797): `pos = parent.pos`; with grandparent `pos += normalise(normalise(parent.pos - gp.pos)*2.0f + vec2Hash(abspath))` else `pos += vec2Hash(abspath)`; `spos = pos - (parent.pos - pos)*0.5f`. updateSplinePoint (762-778). logic (895-928): move, spline, node_normal, files, children (pre-order), calcColour, tickers.
10. Dir colour (545-568): `brightness = max(0.6f, 1.0f - min(1.0f, since_last_node_change/3.0f))`; averaged with visible file colours. Bloom `col * bloom_intensity` (0.75), radius `dir_radius*2.0*bloom_multiplier`.
11. Quadtree (core/quadtree.cpp): root depth 1; `allowMoreItems = children.empty() && (depth >= max_depth || items.size() < max_items)` (:38); split into 4 (TL,TR,BL,BR) at bounds.centre(); item added to EVERY child whose bounds overlaps (:96-104); `visitItemsInBounds` visits all items of overlapping non-empty leaves (:248-264); dedupe by per-query std::set when `node_count != 1`. DirForceFunctor skips self, this.parent, d whose parent is this, ancestors, descendants (dirnode.h:212-245). dir quadItemBounds = pos +/- (dir_radius, dir_radius) for visible dirs only (dirnode.cpp:1182-1189).
12. RFile (file.cpp): hidden=true until first touch; size `(float)(8.0*1.05)`=8.4; speed 5.0; nametime=`filename_time` (default 4.0, min 2.0); fade_start -1.0. logic (175-225): `accel = dest*distance - pos; a2 = accel*speed*dt; if length2(a2) > length2(accel) a2 = accel; pos += a2`; idle fade `fade_start<0 && file_idle_time>0 && elapsed-last_action > file_idle_time`; expiry `fade_start>0 && !expired && elapsed-fade_start >= 1.0`; `if(isHidden() && !forced_removal) elapsed = 0.0`. getAlpha: `min(elapsed/1.0,1)`, fade-out `1 - clamp(elapsed-fade_start,0,1)`. getColour: `lc=elapsed-last_action; if lc<1: touch_colour*(1-lc) + file_colour*lc`. touch (227-252) returns if `forced_removal || (removing && ts < removed_timestamp)`. Delete at next frame start via Gource::deleteFile (962-988). setFilename ext rule (file.cpp:112-134); initial colour `RCommit::fileColour` (formats/commitlog.cpp:308-319); `RCommitFile` ctor prepends '/' (295-297).
13. RAction (action.cpp): progress 0, rate 0.5 (2.0 when lag-promoted, user.cpp:250); logic (28-38): `if progress==0.0 apply()`; `action_rate = min(10.0f, rate * max(1.0f, (float)source->getPendingActionCount()))`; `progress = min(progress + action_rate*dt, 1.0f)`. Colours Create (0,1,0), Remove (1,0,0) (target->remove(timestamp) on frame old<1 && new>=1), Modify (1.0,0.7,0.3) (+setFileColour).
14. RUser (user.cpp): speed `max_user_speed` 500; size `20.0*user_scale`; action_interval 0.2; name_interval 5.0. applyForceUser (88-115): `desired = getActionCount()==0 ? 100 : (!actions.empty() && activeActions.empty()) ? 10 : 50`; `dist<0.001` random nudge (factor 1.0f); `dist<desired` -> `accel -= (desired-dist)*normalise(dir)`. applyForceAction (117-141): <0.001 random; <50 push away `(50-dist)`; >100 pull `(dist-100)`; between none. applyForceToActions (143-172): `last_action = elapsed`; first up to 3 active; else first queued only. logic (231-302): lag promotion `max_file_lag>=0 && action.t < t - max_file_lag` (rate=2.0); `find_nearby` d<100 promotes then break; `action_interval = n ? 1.0/(float)n : 1.0`; speed clamp `length2(accel) > speed*speed`; `pos += accel*dt`; `accel = accel * max(0, 1 - user_friction*dt)` (accel persists).
15. User alpha/state: `getAlpha`: `a=min(elapsed/1,1)`; `if(elapsed-last_action > user_idle_time) a = 1 - min(elapsed-last_action-user_idle_time, 1.0)`; isIdle = no active and no queued; isFading = idle && `elapsed-last_action > user_idle_time`; isInactive = idle && `elapsed-last_action > 10.0` (user.cpp:362). Recolour calls colourize() alone so loses 0.6/0.4/0.9 mix (user.cpp:219-220). Defaults (gource_settings.cpp:468-473): user_idle_time 3.0, user_friction 1.0, max_file_lag 5.0, max_user_speed 500, user_scale 1.0; other defaults in the section 8 index (elasticity 0.0, filename_time 4.0, file_idle_time 0.0, bloom_intensity 0.75, bloom_multiplier 1.0, days_per_second 0.1, auto_skip_seconds 3.0).

**Gotchas:** XOR in stringHash; signed char hashing; `isVisible()` mutates `visible` (latches); fade tests use `> 0.0f` so `fade_start==0.0` never fades/expires; untouched files keep `elapsed=0`, hidden, not counted in dir_area; recolour loses user lighten mix; rand() argument evaluation order unspecified (gcc right-to-left, clang left-to-right); with max_depth==1 every dir/user interacts with every other; `gGourceGravity` flag unused in force code; forces read pre-move positions (positions only change in logic); non-root dirs move even while empty; ancestors always get `userInitiated=true`.
**Verification status:** Tick 3 re-derived independently (verification-tick3-scene.md) and found 9 discrepancies; section "Tick 3 corrections" appended at the end of scene-model.md. Gaps (sec 12): spline, bloom, frustum, fxfont, camera internals, gource.h defaults, file_key; quadtree getItemsAt/QuadNode::empty not read; gource_settings validators only grepped.
**Open questions:** which libc/compiler pins rand() stream and arg evaluation order for parity reference.
**Conflicts:** scene-model.md sec 0 / 10.10 dt wording: timing.md says exporter dt is `1/gource_fps` (1/75 for 25fps), not 1/60 (scene-model :358 wrong; 10.10 itself already states gource_fps). scene-model sec 2/3.1/7/10.7 "uninitialised" -> zero via GLM_FORCE_CTOR_INIT (vectors.h:34). file_area 50.2654824 (f64) vs 50.26548386 (f32 field).

---------------------------------------------------------------
### 3. timing.md (188 lines) - AUTHORITATIVE on dt
**Purpose:** Authoritative reference for how SDL tick becomes sim advance; time-related settings; exporter vs interactive; stop conditions; wall-clock entry points; contradictions among other docs.
**Section map:** 1 Per-frame flow (pseudo-listings of SDLApp::run, GourceShell::update, Gource::update, Gource::logic); 2 Time-related settings table; 3 Exporter vs interactive; 4 Stop conditions and post-stop frames; 5 Wall-clock/system-time entry points; 6 Contradictions (9 items); Verification (tick 4).

**Key facts (quoted):**
1. `GourceShell` sets `min_delta_msec = 16` (gource_shell.cpp:29), overriding SDLApp default 8 (sdlapp.cpp:317). Loop (sdlapp.cpp:405-461): `buffer_msec += delta_msec` (:429); `if buffer_msec < min_delta_msec: SDL_Delay(1); continue` (:430-432); `delta_msec = buffer_msec; buffer_msec = 0` (:435-436; delta can exceed 16, no remainder carried); `t = total_msec/1000.0f`, `dt = delta_msec/1000.0f` (f32, :441-442); events polled before `update(t,dt)` (:452-457); display.update :460; frame_count++ :461.
2. GourceShell::update (gource_shell.cpp:283-313): if no gource or finished -> getNext()/`appFinished=true`, return with NO update/draw/dump; `gource->fps = fps`; `toggle_delay -= dt` (wall dt); transition/blend use wall dt.
3. Gource::update: `scaled_dt = min(dt, max_tick_rate)` (:248); exporter `scaled_dt = max_tick_rate` (:251-253); `*= time_scale` (:256); `runtime += scaled_dt` unless paused (:259); `stop_at_time > 0.0 && runtime >= stop_at_time` (:261); logic(:265)/draw(:269); dump (:272-276); `cursor.logic(dt)` real dt (:278-282).
4. `runtime` only assigned in ctor (gource.cpp:157), not reset by reset()/seek; `seekTo` un-pauses (`paused=false`, :1073); runtime and framecount advance while commitlog==0 (logic returns at :1579-1581) and while paused (only `runtime +=` skipped).
5. Clock and commit loop line cites: 1723 time_inc (f64 expr stored f32); 1724 `seconds=(int)time_inc; subseconds += time_inc - (float)seconds`; 1728 carry; 1733 `currtime += seconds`; 1749 auto-skip; 1754 break; 1756 processCommit; 1758-1769 time travel; 1771 `subseconds=0.0; pop_front`. idle_time is f32 sim seconds (scaled dt).
6. Settings table (gource_settings.cpp): time_scale 1.0 (:413), valid `0 < S <= 4` (:1266-1268, not applied to key edits); `-s/--seconds-per-day` days_per_second=0.1 (:410), `S<=0` invalid (:1203), stored `1.0/S` (:1208); `--realtime` days_per_second=1/86400 (:1360-1362), applied AFTER seconds-per-day so it wins; auto_skip_seconds 3.0 (:409), `S<=0` invalid (:1217), `--disable-auto-skip` sets -1.0 (:716-718) processed BEFORE `-a` (:1211) so `-a` re-enables; stop_at_time -1.0 (:395), `S<=0` invalid (:1347); stop_position 0.0 (:394), `F<=0 || F>1` error (:1336); `--stop-on-idle` no-op (:1376); loop_delay_seconds 3.0 (:416), `S<=0` invalid (:734); start_position 0.0 (:393), `random` -> `srand(time(0))`, `(rand()%1000)/1000.0f` (:1319-1320), else `F<=0 || F>=1` error (:1324); `file_idle_time` default 0 (:411) and `file_idle_time_at_end` 0 (:412) are `atoi` integers (:1228,1241); user_idle_time 3.0 (:471); max_file_lag 5.0 (:469), `==0` invalid (:1406), negative disables (user.cpp:247); output_framerate default 60 (core/settings.cpp:89), must be 25, 30 or 60 (:464-466); commitqueue_max_size 100 (gource.cpp:919); `--no-time-travel` (:399); `--dont-stop` only used at gource_shell.cpp:224.
7. Exporter vs interactive: exporter `scaled_dt = max_tick_rate * time_scale`, `max_tick_rate = 1.0f/gource_fps` (:519); gource_fps = smallest multiple of F >= 60 (30 -> 60, 25 -> 75); dump when `commitlog!=0 && !shutdown && framecount % (frameskip+1)==0` (:272-273); framecount reset in setFrameExporter (:510) from ctor (:165); exporter forces `stop_at_end=true` unless `dont_stop || loop || path=="-"` (gource_shell.cpp:224-225); multi-repo no wrap with exporter; multi-repo `stop_at_time = 60.0f` default (gource_shell.cpp:228-234).
8. Stop: `stop_position_reached` (init false :107, never reset by reset()) set by (1) stop_at_time (:261), (2) `stop_timestamp != 0 && commit.timestamp > stop_timestamp` (:1148-1150), (3) `stop_at_end && commitlog->isFinished()` (:1172), (4) stop_position (:1175). Effects: readLog returns immediately (:1132); auto-skip disabled (:1749); `file_idle_time = file_idle_time_at_end` if >0 (:1180-1182). Termination: `appFinished` when `users.empty() && stop_position_reached` in updateUsers (:1401), or `shutdown && logmill->isFinished()` (:1570). Run continues until users fade (user_idle_time 3.0 s + 1.0 s fade, user.cpp:346-358; scaled by time_scale).
9. Wall-clock entry points: sdlapp.cpp:414,424 SDL_GetTicks (interactive sim dt: YES); localtime in gource.cpp:1465, :298 (display only); start/stop date parsing uses local TZ (gource_settings.cpp:1280,1297,1306; core/settings.cpp:331-357 mktime) AFFECTS sim; formats/*.cpp mktime/timegm timestamp parsing (cvs2cl:128, apache:81, cvs-exp:82, bzr:117, svn:239/241).
10. Multi-repo: settings re-imported per repo (gource_shell.cpp:218) and importGourceSettings calls setGourceDefaults first (gource_settings.cpp:368-520), so per-repo options reset; misc-traces.md lists keys NOT reset.

**Gotchas:** exporter determinism broken by async log-load frames (runtime/framecount advance by load-time-dependent amount, shifts dump phase) - port should define explicitly (block on load, framecount phase 0 at first loaded frame); wall dt still flows to cursor, toggle_delay, transition blend; frame in which appFinished is set is still drawn and dumped; `--stop-date` rounding only affects display string (stop_timestamp raw); time_scale key edit can reach 0 and days_per_second can reach 0; settings validation range doesn't apply to key edits; `file_idle_time` ints.
**Verification status (tick 4):** ~110 cites checked, ~105 correct, 6 corrected (sdlapp.cpp :457/:460/:461 shifts, :322, logmill.cpp range 83-93) plus 1 wrong statement (stop-date rounding). Confirmed: min_delta 16 override, whole buffer becomes dt, exporter dt/gource_fps loop, defaults and validations, key handlers 821-873. **Not confirmed:** formats/*.cpp cites (grep only); gource.cpp:2414-2545 and 11 hidden SDL_GetTicks matches not individually opened; key block start lines 853/873 inferred.
**Open questions:** none explicit besides defining exporter log-load phase.
**Corrections it applies to other docs (sec 6):** core-lib.md "8 ms" wrong (16); scene-model :7,:358 dt claim incomplete/wrong for 25fps (1/75); output-pipeline.md gource_fps wording ("smallest multiple of F >= 60"); app-core :233/:410/:429 pause semantics (idle_time lives in updateUsers; dt fixing happens in Gource::update not SDLApp); app-core :498 omits sdlapp SDL_GetTicks as sim dt source; missing from all docs: load-time frames, `--realtime` and `-a` override order, atoi ints, `--stop-on-idle` no-op, `dont_stop` scope.

---------------------------------------------------------------
### 4. verification-tick3-scene.md (309 lines)
**Purpose:** Independent re-derivation of scene/sim logic from source only (pseudocode written before reading scene-model/app-core), then diffed; second-source pseudocode for the port. Docs not edited by it.
**Section map:** 1 Discrepancies table (9 rows); 2 Independently derived pseudocode: Pawn, RUser, RFile, RAction, ZoomCamera, RDirNode (ctor/setParent, calcRadius, addFile/addNode/removeFile, updateFilePositions/calcFileDest, applyForces/applyForceDir/move/setInitialPosition/updateSplinePoint/logic), Quadtree, Commit ordering.

**Discrepancies (doc says X -> source says Y):**
1. accel/dest/touch_colour "uninitialised" -> zero via `GLM_FORCE_CTOR_INIT` (src/core/vectors.h:34); `QuadItem::node_count` is a plain int, truly uninit until addItem sets 0 (quadtree.cpp:497).
2. app-core 242/356 `time_inc` f64 -> `float time_inc = (dt * 86400.0 * days_per_second)` stored f32 (gource.cpp:1723-1726).
3. app-core 614 break at 1757 -> 1754.
4. `toa` = 2.0 -> `1.99999988` (f32; DEGREES_TO_RADIANS f64 literal 0.017453292; `tan(0.78539814)*2 = 1.9999999064`) (zoomcamera.cpp:121; pi.h:33).
5. file_area 50.2654824 -> f32 field 50.26548386 (dirnode.cpp:55-57).
6. dir logic: only `since_node_visible` guarded by `if(visible)`; other two advance unconditionally (dirnode.cpp:924-927).
7. `RDirNode::split` and `calcArea` do not exist (regroup at addFile tail 456-493; dir_area field in calcRadius 574-594).
8. User-user query box only `pos +/- (10, 13.33)`; leaf capacity 1, depth 1 or 6; max_depth==1 -> all users interact; port must reproduce the quadtree (gource.cpp:1305-1326; quadtree.cpp:38,248-264).
9. `rand()%100-50` pair evaluation order unspecified (gcc usually right-to-left, clang left-to-right) (user.cpp:106,128; dirnode.cpp:625).

**Key facts (from pseudocode, with cites):**
- `PI` 3.14159265 (pi.h:31); `DEGREES_TO_RADIANS` 0.017453292 (pi.h:33); `normalise` returns v if length 0 (vectors.cpp:7-11).
- Pawn::logic: `elapsed += dt; if !hidden && name_interval > 0.0: name_interval -= dt` (pawn.cpp:74-80).
- RUser: desired spacing `100.0*0.1` = 10.0, `100.0*0.5` = 50.0 (f64->f32); `gGourceBeamDist=100, gGourceActionDist=50, gGourcePersonalSpaceDist=100` (user.cpp:20-22); full logic pseudocode (user.cpp:231-302); Gource::interactUsers (gource.cpp:1321-1330) THEN updateUsers (1371-1374) in name order; accel persists decayed by friction; `user_friction` stored as `1.0/value` (gource_settings.cpp:1421).
- RFile: `fade_start = -1.0`; `dest_pos = dest * distance`; accel2 left-to-right `(accel*5.0)*dt`; touch/remove/setHidden pseudocode (file.cpp:227-260); expired files deleted at next Gource::logic after readLog and clock advance, before commits (gource.cpp:1736-1740).
- RAction as scene-model; isFinished `progress >= 1.0` (action.h:44).
- ZoomCamera ctor (zoomcamera.cpp:23-38) and `lockon_time` update `max(0.0f, lockon_time - dt*0.5f)`; setPos(p, keep_angle) (160-168).
- RDirNode: full addFile/addNode/removeFile pseudocode; updateFilePositions/calcFileDest; applyForces; move; updateSplinePoint (`delta` not recomputed after first adjustment); per-frame order `updateBounds -> interactUsers -> updateUsers -> interactDirs -> updateDirs (applyForces for ALL then logic) -> updateCamera -> updateTime`; depth=1 if `dir_bounds.area() <= 10000` else 6 (gource.cpp:1305-1310, 1430-1435). `calcFileDest` first file: frac=0.5 -> (sinf(pi)~=-8.7e-8, -1), distance 0 => at dir centre.
- Quadtree: split into TL,TR,BL,BR; non-strict overlap; accumulation order depends on insertion order (users name-sorted, dirs path-sorted).
- Commit ordering: NO sorting anywhere (std::sort only in core/ui/ui.cpp, core/ui/file_selector.cpp, key.cpp:273). `RCommitLog::nextCommit` (formats/commitlog.cpp:216-236) yields in file order with one-commit pushback buffer (`lastCommit/buffered`); `RCommit::postprocess` only `filter_utf8` on username (:353); `isValid` applies user filters and requires non-empty files (:357-384); RLogMill `--start-date` skip (logmill.cpp:86-97); readLog appends in log order; out-of-order commit sets `currtime = commit.timestamp` when `lasttime > commit.timestamp` (gource.cpp:1765-1767).

**Gotchas:** quirk `fade_start == 0.0` never expires; `parent_radius` floor-after-multiply; `removeFile` never touches `visible` latch; only dirs with no files AND no children reaped; rand() unseeded unless `--start-position random` -> platform libc default sequence.
**Verification status:** Checked and CONSISTENT with docs: RUser::logic + three applyForce funcs, lag promotion, action_interval reset, friction; RFile::logic (incl. fade_start>0 quirk, elapsed=0 while hidden), touch/remove; RAction and subclasses; RDirNode applyForces/applyForceDir/move/updateSplinePoint/setInitialPosition/calcRadius (incl. parent_radius floor), addFile/removeFile/addNode/regroup, updateFilePositions/calcFileDest; DirForceFunctor skip set; quadtree depth rule; ZoomCamera adjust/logic/setPos; per-frame order; container orders. Does not cover: log-format parsing, rendering, settings parsing.
**Open questions:** pin reference libc/compiler for rand() and argument evaluation order; camera.speed set elsewhere (setSpeed), ctor default 1.0.
## Digest B: settings-cli, log-formats, misc-traces, unit-tests, parity-test-spec, golden/
Docs dir D = <discovery-worktree>/docs/rust-rewrite/discovery/
Cite keys: M=src/main.cpp GS=src/gource_settings.cpp CS=src/core/settings.cpp CF=src/core/conffile.cpp SH=src/gource_shell.cpp SA=src/core/sdlapp.cpp. All Gource 0.57 (GOURCE_VERSION "0.57", GH:21).

======================================================================
### 1. settings-cli.md (550 lines)
Purpose: full CLI/config-file surface: arg parsing, ConfFile grammar, every option with type/default/validation/error text/cite, help text, platform quirks, multi-section sequencing.
Sections: 0 Architecture; 1 Program flow (main.cpp); 2 Config location/loading (2.1 grammar, 2.2 value syntaxes, 2.3 exception messages); 3 CLI parsing rules (3.1 precedence, 3.2 command-line-only opts); 4 Display opts; 5 Gource opts (5.1 numeric/timing, 5.2 fonts/colours/text/images, 5.3 behaviour/camera/hide/filters); 6 Path/repo resolution; 7 Help text (literal); 8 Option index/aliases; 9 Platform-specific; 10 tests/test.conf; 11 Gaps; Tick 2 (T2.1 multi-section sequencing, T2.2 help vs man vs README, T2.3 README 300-end, T2.4 env vars); Verification (tick 4).

#### Key facts
1. Architecture: CLI never applied directly. `parseArgs` (CS:178) -> ConfFile entries (section `gource` or `display`), except "command-line" options run immediately (in BOTH passes). Then `importDisplaySettings` (CS:365) and `importGourceSettings` (GS:611) convert/validate. `--save-config` writes the ConfFile (raw strings), not typed settings. Model: `ConfFile = ordered map<section, Vec<Section>>`, `Section = map<key, Vec<Entry{value,line}>>`.
2. main flow (M): parseArgs pass 1 (M:43) -> if no --load-config, scan ALL positionals in order; first ending `.conf`(len>5)/`.cfg`/`.ini`(len>4) that `ConfFile::load`s OK becomes load_config and is erased (M:45-72) -> log level (M:75) -> if load_config: `conf.clear(); conf.load()`, re-run parseArgs w/o files ptr (M:85-91) -> positional path = LAST positional, setEntry "path" into EVERY [gource] section or new one (M:94-106) -> importDisplay, importGource (M:109-110) -> `--save-config`: `conf.save` then exit(0) (M:113-116) -> `--output-custom-log` non-empty path: writeCustomLog then exit(0) (M:119-123) -> ConfFileException => SDLAppQuit(what()) (M:125-128).
3. Exit codes: SDLAppQuit => exit(1); help/*-command/--save-config/--output-custom-log => 0; normal run returns 0 (M:237). Error output (SA:135-153) stderr exactly `gource: <message>\nTry 'gource --help' for more information.\n\n` (extra blank line). SDLAppInfo (SA:117-132) prints msg+`\n` to stdout, exit 0.
4. No default config path searched; only `--load-config` or positional looking like conf. No env vars for config (getenv only TERM SA:70, TZ timezone.cpp:13).
5. ConfFile grammar (CF:494-571): line-based; empty line or first char `#` skipped (col 0 only; a lone `\r` line FAILS "could not parse line"); section regex `^\s*\[([^\]]+)\]\s*$` (repeated names kept as list); kv regex `^\s*([^=\s]+)\s*=\s*([^\s].*)?$`, value right-trimmed of " \t\f\v\n\r", no quoting/escape/inline comments; pre-section entries go to unnamed `""` section (never read); else `ConfFileException("<file>, line <n>: could not parse line")`; open failure `failed to open config file <file>`; unknown keys/sections silently ignored (unknownOptionException never called); first entry wins on getEntry; `setEntry` deletes all entries of key then inserts one (CF:293-310).
6. Value syntaxes: bool true iff exactly one of `1 true True TRUE yes Yes YES`, everything else false (no error); int=`atoi`; float=`atof`; colour = `vec3(r, g, b)` regex `^\s*vec3\(\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*\)\s*$` (0..1, unclamped) OR exactly 6 chars `sscanf("%02x%02x%02x")`==3 /255 (no `#`); else `invalid '<key>' value`.
7. CLI parse rules (CS:166-282): strip ALL leading `-` while size>1 (so -x/--x/---x same); lone `-` positional; `--` => unknown option `-`; alias translation case-sensitive before viewport check; MAGIC viewport: token containing `x` matching `^([0-9.]+)x([0-9.]+)(!)?$`, both atoi>0 => `[display] viewport=<token as typed>`; unknown option => `unknown option <name>` (no quotes); bool opts store `"true"` & consume nothing; every other type consumes NEXT argv unconditionally (even `-0.5`), "" if none; NO `--opt=value` syntax (=> unknown option `opt=value`); multi-value => addEntry to ALL sections of that name (config entries first), others => setEntry replace-all. Multi-value: follow-user, highlight-user, user-filter, user-show-filter, file-filter, file-show-filter.
8. Aliases: f=fullscreen w=windowed o=output-ppm-stream r=output-framerate p=start-position a=auto-skip-seconds s=seconds-per-day t=stop-at-time i=file-idle-time e=elasticity h=help ?=help H=extended-help b=background-colour c=time-scale background=background-colour disable-bloom=hide-bloom disable-progress=hide-progress highlight-all-users=highlight-users (CS:46-49, GS:222-236).
9. Command-line-only (GS:529-605): help, extended-help, load-config, save-config, log-command VCS (git,cvs-exp,cvs2cl,svn,hg,bzr print cmd to stdout exit 0; `cvs` => `please use either 'cvs2cl' or 'cvs-exp'`; other => `invalid log-command value`), undocumented `--git-log-command` etc. bool shortcuts, output-custom-log, `--log-level` (warn,debug,info,error,pedantic; else ignored). Display keys routed to `[display]` (CS:31-43): viewport windowed fullscreen frameless screen window-position multi-sampling output-ppm-stream output-framerate transparent no-vsync high-dpi.
10. Display defaults: viewport 1024x768 (1024x640 on __APPLE__, CS:69-73); `!` suffix => resizable=false; screen int <1 invalid, default -1; window-position `^([0-9.]+)x([0-9.]+)$`; output-framerate default 60, must be 25/30/60 (`supported framerates are 25,30,60`); `-o` empty => `specify ppm output file or '-' for stdout`; windowed evaluated after fullscreen so wins; fullscreen w/o viewport => 0,0; only FIRST [display] section read.
11. Numeric defaults/validation (5.1): seconds-per-day -> days_per_second=1/x default 0.1, <=0 invalid; auto-skip-seconds 3.0 (<=0 invalid); disable-auto-skip => -1 (evaluated BEFORE auto-skip-seconds GS:716 vs 1211, so given value overrides); stop-at-time -1; start-position `random`=>`srand(time(0)); (rand()%1000)/1000.0`, else 0<x<1 (`start-position outside of range 0.0 - 1.0 (non-inclusive)`); stop-position 0<x<=1; time-scale default 1.0, <=0 or >4 invalid; elasticity default 0.0, <=0 invalid; file-idle-time via atoi (2.5=>2; <0 or ==0 with first char != '0' invalid); user-idle-time conf-ONLY (3.0, <0 invalid; CLI => unknown option); loop-delay-seconds 3.0; max-files 0; max-file-lag 5.0 (only 0 invalid, negative accepted; -1 = no limit); max-user-speed 500; user-friction stored 1/x default 1.0 (help says 0.67); user-scale 1.0 (<=0 or >100 invalid); filename-time 4.0 (<2.0 `filename-time must be >= 2.0`); bloom-intensity 0.75; bloom-multiplier 1.0; padding 1.1 (<=0 or >=2 invalid); hash-seed 31 (global gStringHashSeed, reset by every setGourceDefaults GS:489); dir-name-depth 0; dir-name-position 0.5 (<0.1 or >1.0 invalid).
12. Fonts/colours (5.2): font-size 16, file-font-size 14, dir-font-size 14, user-font-size 14, range 1..100; font-scale default 1.0 (0..10); scaled sizes `clamp((int)(size*scale),1,100)`; if font-scale absent Gource ctor sets font_scale = DPI ratio if >1.0 else `1 + max(0, display.height/1600)` (gource.cpp:39-48); colours: font 1,1,1; background 0.1,0.1,0.1; highlight 1,1,1; selection 1,1,0.3; dir 1,1,1; filename 1,1,1; caption 1,1,1; logo-offset 20,20; date-format `%A, %d %B, %Y %X`; caption-size 16, caption-duration 10.0, caption-offset 0; font-file default `GOURCE_FONT_FILE` or `FreeSans.ttf`, must exist, stored canonical.
13. Behaviour (5.3): --hide list (date users tree files usernames filenames dirnames bloom progress mouse root; `mouse` also sets hide_progress; unknown => `unknown option hide <field>`); undocumented `--hide-<x>` bools exist generically (GS:673-681); camera-mode overview|track; crop vertical|horizontal; `--stop-on-idle` parsed but no-op; `--ffp`; git-branch regex `^(?!-)[/\w.,;_=+{}\[\]-]+$` (lookahead); log-format allowed git cvs-exp cvs2cl svn custom hg bzr apache; filters compiled with PCRE2 `Regex(str,1)`.
14. Path (sec 6): default `.`+default_path=true; `-` requires log_format (`log-format required when reading from STDIN`) then POLLS stdin `while(cin.peek()==EOF && !cin.fail()) SDL_Delay(100)` then `cin.clear()`; else one trailing / or \ stripped; must exist else `'<path>' does not appear to be a valid file or directory`.
15. save-config order (CF:452-487, 395-411): sections alphabetical (std::map) with repeats in insertion order; keys alphabetical, repeated keys insertion order; `[name]\n`, `key=value\n`, blank line after each; raw CLI strings (bools `true`). Failure `failed to write config to <file>`.
16. Multi-repo (T2.1/SH:205-253): `repo_count=countSection("gource")`; getNext: delete Gource, transition_interval=1.0, `importGourceSettings` (setGourceDefaults first -> nothing leaks), exporter & not(dont_stop||loop||path=="-") => stop_at_end=true; repo_count>1 and stop_at_time<=0 and stop_position<=0 => stop_at_time=60.0; wrap to begin if repo_count>1 && no exporter. Enter (no Alt) = next repo (SH:177); ESC = shutdown.
17. Help text: literal -h/-H text transcribed in doc sec 7 (stdout, printf, exit 0); `To see the full command line options use '-H'` only for -h. Extended block starts `Extended Options:\n\n`. Doc says generate golden from C++ binary and diff (done: golden/help-*.txt).
18. Env: only TZ (timezone.cpp:13-35, svn.cpp:121-141; putenv TZ=UTC during date parse then restore; port should do UTC math directly), TERM (Windows only), SDL hint MINIMIZE_ON_FOCUS_LOSS="0" (M:174).

#### Gotchas
- Sections after the first are validated lazily in getNext(); ConfFileException there is uncaught by main (catches only ResourceException, SDLAppException; M:210-226) -> std::terminate.
- Config filename prefix appears on CLI-derived bad-value errors when a config was loaded.
- `--high-dpi` help line omitted on WIN32 (#ifndef _WIN32 GS:59). main: high_dpi=true always; non-Windows set false if viewport_specified && !--high-dpi.
- Help mismatches kept for fidelity: user-friction help 0.67 vs code 1.0; dir-name-position help "0.0-1.0" vs code 0.1; man says -f only; man lacks --disable-auto-skip/--dont-stop/--windowed.
- `exportDisplaySettings` (CS:92) dead code. `--user-idle-time` conf-only.
Verification: tick 4 sampled 28 claims, all confirmed; corrections to main.cpp cites and GS:57->59, GS:1624-1625.
Open: unnamed `""` section effects beyond ignored; whether any CI harness runs tests/test.conf.

======================================================================
### 2. log-formats.md (258 lines)
Purpose: log-loading pipeline, common commit logic, regex wrapper, SeekLog/StreamLog, stringhash, all 9 formats, custom log full spec, fixtures.
Sections: 1 Pipeline; 2 Common commit logic (+parseDateTime); 3 Regex wrapper/PCRE2; 4 SeekLog/StreamLog (+position options); 5 stringhash; 6 Formats (6.1 git, 6.2 gitraw, 6.3 custom->7, 6.4 svn, 6.5 hg, 6.6 bzr, 6.7 cvs-exp, 6.8 cvs2cl, 6.9 apache); 7 Custom log spec + tests/logs fixtures; 8 Comparison table; 9 Rust notes.

#### Key facts
1. RCommit (commitlog.h:42): `timestamp: time_t`, `username`, `files: list<{filename, action:String, colour:vec3}>`.
2. Auto-detect order (logmill.cpp:272-334): git, hg, bzr, git-raw, cvs-exp, svn, cvs2cl, custom, apache. Directory repo detection (logmill.cpp:171-174): `.git` (dir OR file) > `.hg` > `.bzr` > `.svn`, nearest ancestor wins. Explicit `git` tries GitCommitLog then GitRawCommitLog.
3. BUG TO PRESERVE: `--log-format cvs-exp` accepted by settings (GS:784) but fetchLog only matches `"cvs"` (logmill.cpp:238) and settings reject `cvs` (GS:779) => explicit cvs-exp => "unsupported log format"; auto-detect works.
4. Errors (logmill.cpp:98-123): `unable to read log file`; `failed to generate log file` (+` for the specified time period`); `directory not supported`; `unsupported log format (you may need to regenerate your log file)`. Empty first read => "no commits found". NO sorting anywhere: file order.
5. Consumption (gource.cpp:1131-1185): queue refilled while `queue.empty() || (back.timestamp <= currtime && size < 100)` (gource.cpp:1137); commit with `timestamp > stop_timestamp` sets stop_position_reached and is discarded (:1148); non-seekable failure => break, seekable => continue.
6. `--output-custom-log` (gource.cpp:170-216): `"%lld|%s|%s|%s\n"` ts|user|action|filename (leading `/`, no colour).
7. isValid (commitlog.cpp:357-384): any user-filter match rejects; EVERY user-show-filter must match (AND); files non-empty. addFile (325-351): any file-filter drops; every file-show-filter must match (AND); applied to RAW filename before `/` prefix. RCommitFile ctor: filter_utf8 then prepend `/` if first char != '/' (also empty); trailing `/` kept. UTF-8: `utf8::replace_invalid(...,'?')`, on exception `"???"`.
8. Default file colour (commitlog.cpp:307-323): slash=rfind('/'), dot=rfind('.'); if dot found, dot+1<len, and (no slash or slash<dot): `colourHash(ext)` else white (1,1,1).
9. Action semantics (gource.cpp:1199-1204, 1279-1286): "D" remove, "A" create, anything else modify; filename ending `/` ignored unless D, which removes all under dir.
10. stringHash (stringhash.cpp): `val += str[i] * (gStringHashSeed ^ (n - i))` (XOR, SIGNED char, i32 wrap), `if val<0 val=-val`; seed 31 default (random 1..10000 on recolour gource.cpp:1555). colourHash: hash==0 => hash++; r=(hash/7)%255, g=(hash/3)%255, b=hash%255; `normalise(vec3(r,g,b))` (unit length, NOT /255). vec2Hash x=((hash/7)%255)-127,y=((hash/3)%255)-127 normalised. Rust: i8 bytes, wrapping_add/mul, wrapping_neg for i32::MIN.
11. parseDateTime (settings.cpp:284-363) regex `^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{1,2}):(\d{2})(?::(\d{2}(?:\.\d+)?))?)?(Z| ?([+-])(\d{1,2})(?::?(\d{2}))?)?$`; no time=>00:00:00; Z => mktime_utc; offset => mktime_utc - offset; neither => local mktime; code branches on results.size() (`==7 && [6]=="Z"`, `>=9`).
12. Regex wrapper (regex.cpp): PCRE2 flags 0 (no UTF/UCP/caseless); `match` unanchored search; results = groups 1..highest participating (trailing non-participating OMITTED, middle -> ""); EMPTY SUBJECT NEVER MATCHES (`matchOffset` returns -1 if offset>=size, regex.cpp:174, even `^$`). Parsers rely on results.size(). Rust: `regex::bytes` with `(?-u)` for parsers; fancy-regex/pcre2 for user filters and git-branch lookahead.
13. SeekLog (seeklog.cpp): files >104857600 B (100 MiB, gSeekLogMaxBufferSize) keep ifstream else whole file into istringstream(std::string(buf)) — truncates at first NUL; strip one trailing `\r`; `seekTo(percent)`: offset=(streampos)(percent*file_size) float mult, if offset!=0 discard next line; isFinished = fail||eof. StreamLog: O_NONBLOCK stdin; returns false if stream fail/eof after read => final unterminated line DROPPED.
14. git command (git.cpp:88-128), cwd=repo: `git log --reverse --raw --encoding=UTF-8 --no-renames [--no-show-signature] --pretty=format:user:%aN%n%ct [--since YYYY-MM-DD] [--until YYYY-MM-DD] [BRANCH]` (`%at` with --author-time; no-show-signature if git version unknown(major==0) or >2 or ==2 minor>=10). First char 'u'. Parse (:198-248): `user:` prefix -> username substr(5), next line ts `atol`, 0 => false; file line tab-separated, status=line[tab-1], quotes stripped, NO unescape of C-quoted octal. Merge commits no files => skipped.
15. gitraw regexes (gitraw.cpp:20-25): `^commit ([0-9a-z]+)`, `^tree`, `^parent`, `^author (.+) <([^@>]+)@?([^>]*)> (\d+) ([-+]\d+)`, `^committer ...`, `^:[0-9]+ [0-9]+ [0-9a-z]+\.* ([0-9a-z]+)\.* ([A-Z])[ \t]+(.+)`; ts = committer time.
16. svn (svn.cpp): `svn log -r <START>:<STOP> --xml --verbose --quiet`, START `{date}` or `1`, STOP `{date}` or `HEAD`; first char '<'; accumulate lines `^<logentry`..`^</logentry>` (column 0), TinyXML; date -> `timegm` UTC; author empty => "Unknown", absent => ""; `kind=="dir"` only action D kept with trailing `/` appended; kind absent/"" = file.
17. hg: `hg log [RANGE] -r 0:tip --style '<resource_dir>gource.style' -R "<dir>" > tmp`; style file verbatim in doc (`{date|hgdate}|{author|person}|M|{file_mod}\n` etc.); regex `^([0-9]+) -?[0-9]+\|([^|]+)\|([ADM]?)\|(.+)$`; empty action => "A"; same (ts,user) consecutive lines merge; order adds, copies(A), mods, deletes.
18. bzr: `bzr log --verbose -r <S>..<E> --short -n0 --forward <dir>`; regexes `^ *([\d.]+) (.+)\t(\d{4})-(\d+)-(\d+)(?: \{[^}]+})?(?: \[merge\])?$` and `^ *([AMDR])  (.*[^/])$`; ts local midnight.
19. cvs-exp: regexes in doc (commitno `^([0-9]{6}):`, date `^\(date: ...`, etc.); state `dead`=>D else M; ts local mktime, offset ignored; command display `cvs-exp.pl -notree`. cvs2cl: first char '<', `<isoDate>` converted with LOCAL mktime not timegm; `cvs2cl --chrono --stdout --xml -g-q`.
20. apache: `^(?:[^ ]+ )?([^ ]+) +[^ ]+ +([^ ]+) +\[(.*?)\] +(.*)$` exactly 4 groups; date exactly 8 groups; unknown month => January; tz ignored; url cut at last `?`, empty=>`/`, trailing `/` => append `index.html`; action always "A".
21. Custom log (custom.cpp:21): `^(?:\xEF\xBB\xBF)?([^|]+)\|([^|]*)\|([ADM]?)\|([^|]+)(?:\|#?([a-fA-F0-9]{6}))?` ; timestamp: len>1 and `-` at idx>=1 => parseDateTime else atoll (0 only if field is exactly "0"; negatives ok); empty user => "Unknown"; empty action => "A" (R/T make line fail); NO end anchor; colour sscanf/255 (not normalised), absent => fileColour. Grouping: consecutive same (ts,user) join; differing line pushed back via lastline; malformed line ends commit and is DROPPED; no sorting.
22. Position opts: `--start-position` -> Gource::seekTo (reset + commitlog->seekTo, seekable only); `--stop-position` stop when seekable and (is_finished or getPercent()>=F); loop: queue empty, seekable, idle>=delay => `first_read=true; seekTo(0.0); readLog()` (gource.cpp:1707-1713).
23. Fixtures (tests/logs): custom-dir-delete (ts 1277787455/457/460), file-removal, file-to-dir, svn-dir-delete (XML, multi-line `<path\n kind="dir"\n action="D">/blah</path>`), utf8, utf8-caption (a CAPTION file `caption_regex ^(?:\xEF\xBB\xBF)?([^|]+)\|(.+)$` gource.cpp:1080, not a commit log). No fixtures for BOM, colour, `|` names, CRLF, malformed.

#### Gotchas
Bug in #3; hg/custom pushback semantics; apache month fallback; cvs2cl local-time inconsistency; SeekLog NUL truncation; stdin drops unterminated line; `getPercent` uninitialised before first read; final line without `\n` gives tellg -1 percent.
Verification: cites to source; behaviour later partly confirmed by goldens. Open: zero-vector `normalise` behaviour not examined.

======================================================================
### 3. misc-traces.md (360 lines)
Purpose: nine small traced items (PPM exporter, stdin finished semantics, multi-repo re-import, ConfFile::save, hover/click/camera, loadCaptions, Bounds2D/HiDPI line widths, --high-dpi, --output-custom-log writer).
Sections: 1 PPMExporter; 2 Stdin/StreamLog; 3 Multi-repo re-import; 4 ConfFile::save; 5 Hover/click/camera; 6 loadCaptions; 7 Bounds2D/HiDPI; 8 --high-dpi; 9 writeCustomLog; Verification tick 4; Exercised evidence.
Correction: multi-repo re-import is GourceShell::getNext (SH:205-251), setGourceDefaults (GS:368-520), importGourceSettings (611-1725), NOT GS:1280-1357.

#### Key facts
1. FrameExporter ctor (ppm.cpp:39-63): rowstride=width*3; three `char[h*rowstride]` buffers (pixels1/pixels2 GL targets ping-pong, pixels_out flipped); one SDL mutex+cond; thread "frame_exporter". Window resize disabled with -o (M:147). Header `"P6\n%d %d 255\n"` built once (ppm.cpp:175); no comment lines.
2. Per-frame (gource.cpp:271-276): dump() only if exporter, commitlog loaded, !shutdown, and `framecount % (frameskip+1) == 0`. setFrameExporter (gource.cpp:505-522): `gource_framerate = fps; while(<60){ += fps; frameskip++ }`, `max_tick_rate = 1/gource_framerate`; with exporter `scaled_dt = max_tick_rate` (fixed step, gource.cpp:251-253).
3. dump() (ppm.cpp:96-119): glPixelStorei(PACK_ALIGNMENT,1), glReadPixels(GL_RGB,UNSIGNED_BYTE) outside lock into whichever buffer isn't shared, then lock, state=DUMP, signal. dumpThr (121-152) holds mutex whole life, flips rows vertically (:138-142) then dumpImpl (185-188: header + `write(pixels_out,rowstride*height)`). BACK-PRESSURE: slow consumer blocks next dump() on render thread; no queue/drops.
4. Errors: PPMExporterException => `could not write to '<file>'` exit 1. Write failures silent (failbit); no SIGPIPE handling => process killed by SIGPIPE (exit 141 from shell). No flush call. Rust: bounded(1) channel with blocking send; surface errors.
5. stdin: StreamLog (seeklog.cpp:38-95) O_NONBLOCK; `isFinished = fcntl_fail || fail() || eof()`; getNextLine: if finished clear(); getline; strip `\r`; if now eof/fail return FALSE even with text. `RCommitLog::isFinished = seekable && logf->isFinished()` (commitlog.cpp:238-242) => ALWAYS false for stdin: `--stop-at-end` never fires on stdin EOF; shell does not force stop_at_end for path "-" (SH:221-226). writeCustomLog on stdin stops at first failed read (gource.cpp:194-203). Partial line split across pipe writes is LOST (inferred, unverified). logmill start_timestamp skip loop spins on stdin (logmill.cpp:90-97).
6. getNext fixups (SH:219-242): stop_at_end rule, `stop_at_time = 60.0` (SH:228-233), wrap. Full reset list (GS:368-520) in doc sec 3: includes path ".", auto_skip 3, days_per_second 0.1, camera zoom min/default/max 50/100/10000, bloom 1.0/0.75, max_user_speed 500, max_file_lag 5, user_idle_time 3, `gStringHashSeed=31` (a global), filters Regex* deleted. NOT reset: repo_count, shutdown, load_config, save_config, output_custom_filename, file_graphic (texture grabbed once gource.cpp:32-34), log_level, all display state. Gource dtor also resets defaults (gource.cpp:222-223) so twice per transition.
7. ConfFile::save verified by running: alphabetical sections/keys; example output (display before gource; `highlight-user=A B` then `highlight-user=Zed`; `hide=filenames,dirnames`; `hide-bloom=true`; ends with blank line). Bare `gource --save-config f log` => exactly `[gource]\npath=log\n\n`. `-s 2` => `seconds-per-day=2`, `-1280x720` => `[display] viewport=1280x720`.
8. mousetrace (gource.cpp:1880-1986): unproject `projected_mouse = vec2(-(mx*2 - W)/H, 1 - 2*my/H) * cam_pos.z` then `+= cam_pos.xy` (1884-1889); users first (first non-fading user whose quadItemBounds contains pt), files only if no user hit (LAST dir with hit wins, 1913-1936); file hit beats user hit; click (left DOWN sets mouseclicked, gource.cpp:490): user -> `camera.lockOn(false); selectUser`, file -> selectFile, else selectBackground. selectUser (560-590) locks camera only if track_users; selectFile never locks (593-615); selectBackground (545-558) only if left held: selectUser(0), manual_camera=true, drag pans (updateCamera 1482-1497: `cursor_move * (-cam.z/5000) * 10`). Middle click toggleCameraMode (476-478). Zoom step x/÷1.1 clamped [camera_zoom_min,max] (Gource::zoom, 400-417).
9. updateCamera (1478-1522): track_users and selected => bounds of {user pos, file abs pos}; else track_users && idle_time==0 => active_user_bounds; else dir_bounds. camera.logic eases `pos += (dest-pos)*dt*speed(1.0)`; lockon_time reset 1.0, decays 0.5/s (zoomcamera.cpp:61-68,170-186). follow-user (1385-1397) exact name match each frame.
10. loadCaptions (gource.cpp:1082; regex at :1080): called once when first commit queued and currtime==0 (1717-1719); right-trim line; ts via parseDateTime if len>1 and `-` after idx0 else atoll ("0" only literal); text filter_utf8; DROP if `timestamp < currtime`, DROP if `< last_timestamp`; no sorting. Playback (1805-1857): slot y from `caption_start_y` up by `fontcaption.getMaxHeight()`; x: offset==0 centred `width/2 - textwidth/2`, >0 x=offset, <0 `width + offset - textwidth`; `caption_start_y = slider.min.y - 35` if seekable else `height - fontmedium.maxHeight - 20`; with title min(that, height-20-fontmedium.maxHeight); `glm::floor` is then applied on every branch (1778-1785). RCaption logic: `elapsed += dt; fade_in = min(2, duration/3); alpha = min(1, min(elapsed, max(0,duration-elapsed))/fade_in)`; finished when elapsed >= caption_duration. Caption font NOT multiplied by font_scale (gource.cpp:58).
11. HiDPI/line widths: Bounds2D::draw sets no line width; only call debug (gource.cpp:2546-2552, glLineWidth(2.0)); all glLineWidth constants (element.cpp:45,88 =1.0; slider.cpp:30 =1.0, 124/128 =2.0; gource.cpp:2549 =2.0, 2558 =1.0). No HiDPI line scaling. DPI-aware: viewport_dpi_ratio (display.cpp:173-182), mouse events multiplied (sdlapp.cpp:352-355,387-388), font_scale default (gource.cpp:39-49).
12. --high-dpi (M:153-163): default high-DPI AWARE; non-Windows `if(viewport_specified && !gGourceSettings.high_dpi) high_dpi=false`; SDL_WINDOW_ALLOW_HIGHDPI non-Windows (display.cpp:88-91); width/height from SDL_GL_GetDrawableSize (display.cpp:383-385,530-532).
13. writeCustomLog (gource.cpp:171-214, fprintf :207): stdout for `-` else fopen (failure silent, exit 0); null log => SDLAppQuit if error exists else silent exit 0; loop `while(!isFinished())` nextCommit validate=true; colour NOT emitted; --stop-date/--stop-position NOT applied but filters ARE; LF only. Svn golden date => 1263976518.
14. Exercised evidence tick 4: `--log-format custom --output-custom-log - - < tests/logs/utf8.log` byte-identical to file-arg golden; stdin-unterminated input emits only first line, exit 0, empty stderr.

#### Gotchas
Hover vs click distinctions; SIGPIPE kills silently; double defaults reset; `utf8-caption.txt` golden empty (rc 1).
Verification: tick 4 sampled ppm.cpp, commitlog.cpp, shell, conffile, gource.cpp; all confirmed (loadCaptions starts 1082). Partial-line loss (sec 2) unverified.
Open: empty stdin at startup, stop-at-end with stdin interactive, PPM pipe/SIGPIPE not exercised.

======================================================================
### 4. unit-tests.md (56 lines)
Purpose: record of Boost.Test run: built and run, rc 0, 2 test cases, 69 assertions (parse_date_time_tests 58, regex_tests 11). Log: golden/unit-tests.txt.
Sections: Build; regex_tests; parseDateTime goldens (table); Verification (tick 4).
#### Key facts
1. src/test/main.cpp uses `boost/test/included/unit_test.hpp`; link all objs except src_main.cpp.o; run `./gource_tests --log_level=all --report_level=detailed` (~26 ms; no GL context).
2. regex_tests (regex_tests.cpp:22-45): Regex("^[A-Z]+$") valid; match ABC true, abc false, ABC123 false; copy; `Regex("^[A-Z+$")` throws RegexCompilationException; with `true` no-throw => isValid()==false; `Regex("([0-9]+)").match("ABC123",&m)` true, m.size()==1, m[0]=="123".
3. parseDateTime epochs (datetime_tests.cpp:23-112): TZ=UTC first row only; rest TZ=Pacific/Auckland (NZDT +13 on 2021-11-01), never reset.
 - `2021-11-01` UTC => 1635724800; Auckland => 1635678000
 - `2021-11-01Z`, `2021-11-01+0` => 1635724800; `+13`, ` +13` => 1635678000; `+5:30` => 1635705000
 - `2021-11-01 12:01` => 1635721260; `12:01Z`/`+0` => 1635768060; `+13`/` +13` => 1635721260; `+5:30` => 1635748260
 - `2021-11-01 12:01:59` => 1635721319; `T12:01:59.123` => 1635721319; `Z`/`+0` => 1635768119; `+13`/` +13` => 1635721319; `+5:30` => 1635748319
 - T-separator variants identical to space variants; `2021-11-01T12:01:59.123+5:30` => 1635748319
4. Observations: fractional seconds ignored; space before offset allowed; space or T separates; no negative offsets, invalid input, DST gaps tested.
Gotchas: TZ never reset; duplicate rows in source (`T12:01:59.123` twice, `+13` duplicate). Verification: 28/28 rows checked programmatically against source. Open: none.

======================================================================
### 5. parity-test-spec.md (232 lines)
Purpose: how the Rust port proves equivalence with C++ 0.57: parser parity, sim-state dump parity, visual parity, matrix, CI.
Sections: 1 Parser parity (1.1 fixtures, 1.2 help/exit codes, 1.3 edge inputs, 1.4 real VCS logs, 1.5 unit vectors); 2 Simulation parity (2.1 fixed inputs, 2.2 JSONL schema, 2.3 cadence, 2.4 PRNG plan, 2.5 C++ patch, 2.6 comparison policy); 3 Visual parity (3.1 capture, 3.2 sampling/metric, 3.3 exclusions); 4 Test matrix; 5 CI layout; 6 Gaps; Verification tick 4.
#### Key facts
1. Parser gate: stdout of `--output-custom-log -` + exit code + stderr, byte-exact; also run each fixture WITHOUT `--log-format` (auto-detect) and require same stdout. utf8-caption.log => rc 1, stderr == golden, stdout empty. Edge inputs to generate with C++ into `tests/golden/edge/` (empty file, CRLF, blanks, out-of-order ts, `|` in names, BOM, date windows under TZ=UTC and Pacific/Auckland, explicit cvs-exp must fail).
2. Help/exit: `-h` == golden/help-short.txt, `-H` == help-long.txt, rc 0, empty stderr; `--version` rc 1 stderr `gource: unknown option version`. Dummy-driver probe is C++ only, not a Rust gate. Rust keeps "Gource v0.57" verbatim.
3. Per-VCS commands (git/gitraw/svn/hg/bzr) in 1.4; `--log-command VCS` is the documented form (help-long.txt:49); `--*-log-command` undocumented aliases (GS:242-246). Date filter variant `--start-date 2021-11-01 --stop-date 2021-11-02`.
4. Sim fixed inputs: `$G tests/logs/<fixture>.log --log-format <fmt> -1280x720 -r 60 -o /dev/null --seconds-per-day 1 --stop-at-time 10 --hash-seed 31 --disable-auto-skip --disable-auto-rotate --file-idle-time 0 --user-idle-time 3 --time-scale 1 --debug-dump-state out.jsonl` (`--debug-dump-state/-every/-max-frames`, `--rand-script` are PROPOSED, not in C++). CONFLICT: settings-cli says `--user-idle-time` is conf-only (CLI => `unknown option user-idle-time`, settings-cli.md §5.1 row GS:1248); parity-spec uses it on CLI (its verification says "real in source, undocumented"). Resolve before scripting.
5. Fixed dt: `scaled_dt = max_tick_rate = 1.0f/gource_fps` (gource.cpp:251-253, 519); -r 60 => 1/60; -r 30 => gource_fps 60, frameskip 1; -r 25 => gource_fps 75, 1/75. `time_inc = dt*86400.0*days_per_second` (gource.cpp:1723). Cap `--debug-dump-max-frames 4000`.
6. JSONL schema: per frame `{frame, g:{currtime,lasttime,subseconds,runtime,idle_time,paused,stop_position_reached,commitqueue_len,last_percent,rotation_remaining_angle,rotate_angle,hash_seed,rand_calls,rand_last,first_rand_frame,loaded_frame0,runtime_at_load,n_dirs,n_files,n_users,n_removed_pending,tag_seq,dir_bounds,user_bounds}, camera:{pos,dest,target,lockon,lockon_time}, dirs:[...], files:[...], users:[{...,actions:[...]}]}`; f32 printed `%.9g`; arrays sorted (dirs by abspath, files by fullpath, users by name, actions by queue position). Full field list in doc 2.2. Uninitialised C++ fields (RUser::accel, RFile::dest, RFile::touch_colour, Pawn::accel) value-initialised to zero.
7. PRNG sites: user.cpp:106, user.cpp:128, dirnode.cpp:625, gource.cpp:1555; `srand(time(0))` GS:1319 only seed site. glibc TYPE_3 outline: r[0]=seed; r[i]=16807*r[i-1] mod 2147483647 (i=1..30); r[i]=r[i-31] (31..33); r[i]=r[i-31]+r[i-3] (34..343 discarded); out k: r[k+344]=r[k+313]+r[k+341] then >>1; seed 1 => 1804289383, 846930886, 1681692777 (Python model only, never run against real glibc; 344 here is an index, the generator discards 310 outputs). macOS libc = Park-Miller (16807, 2^31-1), confirmed by a compiled test on this Mac (srand(1): 16807 282475249 ...). Script mode `--rand-script FILE` (one int/line, cyclic) is the primary matrix mode.
8. C++ patch: hook after `logic(runtime, scaled_dt)` gource.cpp:265, before draw :269 and framecount++ :284 (not inside logic: early returns :1570, :1579-1581, :1693); new `Gource::dumpState()`; `dumpJson` on RDirNode/RFile/RUser; new `src/core/dumprand.cpp` `gource_rand()` with counters; do not call isVisible() (mutates).
9. Comparison policy: ints/strings exact; same-PRNG floats exact f32 bits (needs `-ffp-contract=off`, no fast-math); different platform/PRNG relative `1e-5 * max(1,|a|,|b|)`; positions informational after `first_rand_frame`; libm (sinf/cosf, vectors.cpp:3) tolerance 1e-5. Order-sensitive: quadtree visit order (leaf insertion order, children TL,TR,BL,BR, closed bounds, first-visit dedupe when node_count != 1), getFilesRecursive prepends, findDirs stops at first prefix match, sorted-key maps, subseconds carry (gource.cpp:1724-1733), stale dir_area, stringHash.
10. Visual: Linux only, `xvfb-run -a -s "-screen 0 1280x720x24" env LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe gource tests/logs/file-removal.log --log-format custom -1280x720 -r 30 --seconds-per-day 1 --stop-at-time 10 --hash-seed 31 --disable-auto-skip --disable-auto-rotate --hide mouse --bloom-intensity 0 -o ref.ppm`; SSIM thresholds structural >=0.97, full >=0.90, mean colour diff <= 2/255; frame counts must match; bloom blurred (sigma 8) luminance within 5%.
11. Matrix (sec 4): fixtures custom-dir-delete, file-removal, file-to-dir, utf8, svn-dir-delete, utf8-caption (parser/error); sim variants (-r25, seed7, PRNG script/glibc/park, rotation); real git 200+ commits N=30 `--stop-at-time 60`; planned dirs `tests/golden/{edge,real,sim,vis}` (do NOT exist yet). tests/test.conf manual, not gated.
12. CI: parity-parser (any OS), parity-sim (Linux+macOS goldens), parity-visual-linux, macOS job (Rosetta, x86_64).
Correction (tick 4): no `--hide-mouse` in help (use `--hide mouse`, help-long.txt:95); settings-cli §5.3 says an undocumented `--hide-mouse` bool exists in arg_types (GS:673-681) - both can hold (help vs code).
Gaps: option spellings vs help-long; rand algorithms unverified; dump hook is design only (nothing compiled); multi-repo not in matrix; captions not in dump schema; sabotage check pending once comparator exists.

======================================================================
### 6. golden/ directory
Captured from C++ 0.57 on macOS x86_64 under Rosetta; recapture via scripts/.
Files (lines): help-long.txt 146 (`-H`, rc 0); help-short.txt 63 (`-h`, rc 0; includes `--high-dpi`, non-Windows build); help-*.stderr.txt 0 B; exit-codes.txt 10; version.txt 0 / version.stderr.txt 3 (`gource: unknown option version`, `Try 'gource --help' for more information.`, blank); probe-dummy.stderr.txt 3 (`gource: SDL initialization failed - OpenGL support is either not configured in SDL or not available in current SDL video driver (dummy) or platform` + Try line + blank); unit-tests.txt 88 (Boost log, ANSI-stripped, ends `rc=0`; 2 test cases, 69 assertions = 58+11, 26334us).
exit-codes.txt (verbatim rc list): help-short rc=0; help-long rc=0; version rc=1; custom-dir-delete rc=0; file-removal rc=0; file-to-dir rc=0; svn-dir-delete rc=0; utf8-caption rc=1; utf8 rc=0; probe-dummy-ppm.bin (`--stop-at-time 1 -o -`, SDL_VIDEODRIVER=dummy) rc=1. (gold.sh also appends a utf8.stdin line, absent from the 10-line checked-in file.)
custom-log/ (all .stderr.txt empty except utf8-caption):
- custom-dir-delete.txt (5): `1277787455|bob|A|/path/to/1.txt`; three `1277787457|bob|A|/path/to/blah/{1,2,3}.txt`; `1277787460|bob|D|/path/to/`
- file-removal.txt (10): 5 `1277787455 A /path/to/file.N`; `1277787456 D` file.1-4; `1277787466|bob|D|/path/to/file.5`
- file-to-dir.txt (4): `1277787456 A /path/to/file/Not`; `1277787457 A .../Not/Not`; `1277787457 A .../Not/Not/Not`; `1277787459 A .../Not/Not/Not/File`
- svn-dir-delete.txt (6): 5x `1263976518|bob|A|/blah/blah/file{a..e}.sh`; `1263976818|bob|D|/blah/`
- utf8.txt (2) and utf8.stdin.txt (2, identical): `1277787455|Árvíztűrő tükörfúrógép|A|/Árvíztűrő/tükörfúrógép/Árvíztűrő tükörfúrógép.txt` and `1277787465|...|D|<same path>`
- utf8-caption.txt 0 B; stderr: `gource: unsupported log format (you may need to regenerate your log file)` + Try line + blank (caption file fed as commit log)
- stdin-unterminated.input.txt (`1700000000|alice|A|/x.txt\n1700000001|bob|M|/y.txt`, no trailing newline); .txt has only the first line; stderr empty
scripts/ (macOS, Homebrew x86_64 under /usr/local, `-arch x86_64`; ARCH=, BREW=/opt/homebrew for arm64):
- README.md (21): order `build.sh` -> `link.sh` -> `gold.sh` -> `tbuild.sh`; needs clang++, pkg-config, sdl2, SDL2_image, glew, freetype2, libpcre2-8, libpng, boost, glm; says glibc_rand.py not checked against real glibc.
- build.sh (23): copies src/data into $BUILD, compiles main.cpp + Makefile.am `sources` + src/tinyxml/*.cpp with `clang++ -arch x86_64 -std=c++17 -O1 -w -I src -I src/core -I src/tinyxml -DSDLAPP_RESOURCE_DIR=... -DHAVE_TIMEGM -DHAVE_UNSETENV` + pkg-config flags, 8 parallel jobs, logs in $BUILD/log.
- link.sh (10): links obj/*.o with pkg-config libs + `-lboost_filesystem -lboost_system -framework OpenGL -framework CoreFoundation` -> $BUILD/gource.
- gold.sh (36): recaptures help-short/long, version, per-fixture custom-log (fmt custom, svn for svn-dir-delete), utf8 stdin variant, SDL dummy probe; 20 s kill timeout; rewrites exit-codes.txt.
- tbuild.sh (19): builds src/test/{main,datetime_tests,regex_tests}.cpp, links all objs except src_main.cpp.o, runs with `--log_level=all --report_level=detailed`, writes unit-tests.txt.
- glibc_rand.py (27): Python glibc TYPE_3 model: Schrage LCG (16807, 127773/2836) for r[1..30], additive feedback f=3, rp=0, 310 warm-up outputs, output `r[f] >> 1`; `print(glibc(1,4))`. NOTE: the parity-spec 2.4 outline writes the same sequence with the first output at index r[344] (344 = 31 + 3 + 310); not a conflict. Both are unverified against a real glibc.
Gotchas: goldens are macOS builds (help includes `--high-dpi`); unit-test TZ side effect; utf8-caption golden documents an error path.
## Digest C: core-lib, output-pipeline, deps, architecture map, build/goldens, tests, verification-tick2, LEDGER, CONTEXT

Docs dir: <discovery-worktree>/docs/rust-rewrite/discovery/ . Cites are the docs' own (`file:line` in the C++ repo) unless stated. Doc abbreviations: COR=core-lib, OUT=output-pipeline, DEP=deps-and-rust-mapping, ARCH=rust-architecture-map, BLD=build-and-goldens, TST=tests-and-golden-strategy, SCN=scene-model, APP=app-core, SET=settings-cli, LOG=log-formats, TIM=timing.

---
### 1. core-lib.md (336 lines)
Purpose: catalogue of src/core (acaudwell/Core submodule) excluding settings/conffile/seeklog/regex/stringhash/ppm/png_writer; plus Tick 2 deep dives (vectors, Frustum, QuadTree order sensitivity, utf8, stringhash, build list) and tick-4 verification.
Map: 0 Headline facts; 1 File inventory; 2 display; 3 sdlapp (+resource lookup order); 4 fxfont; 5 vbo; 6 shaders (class model + shader_common preprocessor); 7 textures; 8 TGA writer; 9 Quadtree/bounds/frustum; 10 Math; 11 timezone; 12 Logger; 13 MouseCursor; 14 ui/ (unused); 15 GL features used; 16 platform ifdefs; 17 Rust guidance; Gaps; Tick 2 additions (a vectors, b Frustum, c QuadTree queries, d utf8, e stringhash, f built files); Tick 2 gaps; Verification (tick 4).

Key facts (exact):
1. GL = compatibility profile, no version requested (display.cpp:254); fixed-function + optional GLSL 1.10; VBOs for text/batches only. SDL2 only; GLEW loader; GLU used (gluPerspective, gluProject, gluUnProject, gluBuild2DMipmaps).
2. Not built by Makefile.am: `ui/`, `tga.cpp` (only gource.pro:84), `timer.cpp`, `utf8/`, `ftgl/` (headers). `lerp2/lerp3` dead. GLTimer unused. `vbo.cpp` IS used (quadbuf backs file_vbo,user_vbo,edge_vbo,action_vbo gource.h:105-108 and text).
3. pi.h: `PI 3.14159265`, `RADIANS_TO_DEGREES 57.29577951`, `DEGREES_TO_RADIANS 0.017453292` (low precision; use exact constants for parity).
4. Min frame delta: Gource sets min_delta_msec=16 (gource_shell.cpp:29); SDLApp default 8 (sdlapp.cpp:317). SDLApp::run accumulates until >= min_delta (sdlapp.cpp:405-465); t,dt float seconds.
5. Resource lookup: defaults `data/`,`data/fonts/`,`data/shaders/`; non-Win prefix CWD+"/"; `SDLAPP_RESOURCE_DIR` (=$(pkgdatadir), Makefile.am:67) if exists else `<exe dir>/data/...` if exepath has "/"; `SDLAPP_FONT_DIR` (Makefile.am:85) overrides fonts; trailing slash added (sdlapp.cpp:178-260). `-f`,`-w`,`-WxH`/`--WxH` handled in SDLAppParseArgs (:262-311).
6. Fonts (fxfont.cpp): FreeType, `FT_Set_Char_Size(face,64*size,64*size,dpi,dpi)` (:241-243), dpi 72, `FT_LOAD_DEFAULT`; glyph quad dims = bitmap+(2,2); corner=(left,-top)+(0.5,-0.5); advance = integer `adv>>6`; no kerning; `getHeight=ceil(asc+desc)`, descender negative. Atlas page W=H=min(512,GL_MAX_TEXTURE_SIZE), GL_ALPHA, cursor starts (1,1), padding 3, wrap when x+w+3>W, row y += max_glyph_height+3, new page when y+h+3>H; texcoords (x-0.5)/W..(x+w+1.5)/W. Precache `0-9a-zA-Z;:'",<.>/?-_=+!@#$%^&*()\ ` + space (:249). Tab = advance('M')*4.0 only in non-VBO draw (:382-386). FXFont defaults: shadowStrength 0.7, shadowOffset (1,1), alignTop true, alignRight false, round false.
7. Quad vertex 32 bytes: vec2 pos @0, vec4 colour @8, vec2 texcoord @24; quadbuf::add vertex order TL,TR,BR,BL.
8. Textures: IMG_Load (also loads .tga); 4 bytes->RGBA/BGRA, 3->RGB/BGR, else TextureException; indexed->RGBA32 (texture.cpp:263-270); mipmaps via gluBuild2DMipmaps; mip filter LINEAR_MIPMAP_LINEAR (trilinear default true) else LINEAR_MIPMAP_NEAREST, mag LINEAR; no mipmaps LINEAR/LINEAR; wrap CLAMP_TO_EDGE default; no UNPACK_ALIGNMENT call.
9. Quadtree: root depth 1 (node depth=parent_depth+1, root parent_depth 0); `allowMoreItems = children.empty() && (depth>=max_depth || items.size()<max_items)` (quadtree.cpp:36-39). Gource: max_items=1; max_depth=1 unless `dir_bounds.area() > 10000.0` then `gGourceMaxQuadTreeDepth=6` (gource.cpp:23, 1305-1312 users, 1430-1437 dirs). Bounds padded +-1.0. Rebuilt from scratch every sim tick. Child order [0] TL (min,min+middle), [1] TR, [2] BL, [3] BR; item added to EVERY overlapping child (closed `overlaps`); item overlapping none silently dropped; existing items re-added in list order on split. Point query `getChildIndex`: first child containing pos (closed).
10. Quadtree callers: only `visitItemsInBounds` in interactUsers (gource.cpp:1326) and RDirNode::applyForces (dirnode.cpp:659) affect sim, ORDER-SENSITIVE (f32 accel sum non-associative; rand() consumption in user.cpp:105-108 dist<0.001, dirnode.cpp:622-626 posd<0.00001). Set-returning getItemsAt (gource.cpp:1899, 1915; mousetrace) only picks hover; pointer order irrelevant (last dir in pointer order with a hit wins fileSelection). Port rule: leaf Vec in insertion order, children TL,TR,BL,BR, closed overlap/contains, rebuild per tick from sorted-key map order, first-encounter dedupe via set when node_count != 1. Brute-force O(n^2) NOT equivalent.
11. Tick order (gource.cpp:1867-1873): `updateBounds; interactUsers; updateUsers; interactDirs; updateDirs`. Paused: `updateBounds; interactUsers; interactDirs` (:1694-1696). Dir recursion is children-first (dirnode.cpp:650-654).
12. `normalise` (vectors.cpp:7-23): `l=length(v); if(l>0.0) return v/l; return v;` (divide, zero returned unchanged). `rotate_vec2(v,s,c)=vec2(v.x*c - v.y*s, v.x*s + v.y*c)` (:3-5). glm::normalize used only in plane.cpp:36 (draw-only).
13. stringHash exact (stringhash.cpp:28-83): `val += str[i] * (gStringHashSeed ^ (n-i))` (XOR, signed char, i32 wrapping), `if(val<0) val=-val` (wrapping_neg). vec2Hash: x=((h/7)%255)-127, y=((h/3)%255)-127, normalise. vec3Hash adds z=h%255. colourHash: `if(h==0) h++; r=(h/7)%255; g=(h/3)%255; b=h%255; normalise(vec3)`. Seed 31 default (gource_settings.cpp:489), `--hash-seed` :1066, recolour sets `(rand()%10000)+1` (gource.cpp:1555). Callers dirnode.cpp:788/791, file.cpp:137, key.cpp:61, user.cpp:175, commitlog.cpp:315.
14. utf8 `replace_invalid` (checked.h:99-125) with '?' via `filter_utf8` (commitlog.cpp:24-35; callers commitlog.cpp:292, 354, gource.cpp:1112): UTF8_OK copy; NOT_ENOUGH_ROOM -> one '?' and stop; INVALID_LEAD -> '?', advance 1; INCOMPLETE/OVERLONG/INVALID_CODE_POINT -> one '?', skip following continuation bytes. Lead classes: <0x80:1; (b>>5)==6:2; (b>>4)==0xE:3; (b>>3)==0x1E:4. cp>0x10FFFF or D800-DFFF invalid. try/catch -> "???". Rust `from_utf8_lossy` NOT equivalent.
15. Frustum is presentation-only (constructed once per frame gource.cpp:2412; `in_frustum` set via checkFrustum 2437, read only by draw paths); cannot affect sim. Logger tags 7-wide: `"  ERROR"`,`"CONSOLE"`,`"   INFO"`,`" SCRIPT"`,`"  DEBUG"`,`"   WARN"`,`" PEDANT"`; default level ERROR to stderr; format `"%s: %s\n"`.
16. mktime_utc = TZ=UTC hack; Rust: chrono NaiveDateTime.and_utc().timestamp(). Used by settings.cpp:331,348; svn.cpp:117-144 own copy.

Gotchas: `char` signedness in stringHash (assume signed); VBO draw with non-final runs draws end-start+1 vertices (GL_QUADS truncates); mouse cursor draw leaks translate; hidden tab handling differs between VBO and immediate paths; `getItemsAt` returns whole leaf without point test; `utf8-cpp` only used for utf16to8 under _WIN32 (gource_settings.cpp:836).
Verification: tick 4 checked ~45 claims; 1 corrected (QuadNode::getItemsAt is quadtree.cpp:188-208 not 135-155). Unconfirmed: Makefile.am "8-32" start line (entries run ~11-32), gource.pro 65-87, dirnode.cpp in_frustum read sites 985/1012/1036/1058/1079/1152 not individually opened.
Open: frustum.h/normalise guard (since resolved in Tick 2); ui/ per-widget unneeded; accel->position integration not traced here (SCN); whether hover feeds camera-follow not traced here; libc rand() sequence per platform unspecified.

---
### 2. output-pipeline.md (364 lines)
Purpose: PPM/PNG export, frame cadence, render pass order, GL state, full shader text, assets, text/UI geometry, pawn formulas.
Map: 1 PPM stream (+Frame cadence, Stop flags, output-custom-log); 2 PNG screenshot; 3 Display/clear/projection; 4 Render pass order; 5 Shaders (full source); 6 Assets; 7 Text sizes/UI constants; 8 wgpu/visual-equivalence checklist; 9 Gaps; Tick 2 (T2.a uniforms, T2.b fonts, T2.c labels, T2.d UI geometry, T2.e pawn/file/user, T2.f display/DPI, T2.g textures, T2 gaps); Verification (tick 4).

Key facts:
1. PPM: header `"P6\n%d %d 255\n"` written before EVERY frame (ppm.cpp:175,186); body w*h*3 RGB8 top-to-bottom (GL readback flipped in writer thread); `glPixelStorei(GL_PACK_ALIGNMENT,1)`, `glReadPixels(0,0,w,h,GL_RGB,UNSIGNED_BYTE)` after scene+UI, BEFORE cursor, before swap (ppm.cpp:98-117; gource.cpp:271-281). One SDL thread "frame_exporter", double buffer; main blocks if worker busy (no drop, depth 1). `-` = stdout.
2. Framerates only 25/30/60 (settings.cpp:456-470), default 60. Tick calc: `gource_fps=video_fps; frameskip=0; while(gource_fps<60){gource_fps+=video_fps; frameskip++}`; `max_tick_rate=1/gource_fps` (gource.cpp:505-522). 60->60Hz skip0; 30->60Hz dump every 2nd; 25->75Hz dump every 3rd. Default `max_tick_rate=1/60`; `scaled_dt=min(real_dt,max_tick_rate)` (gource.cpp:156,248). With exporter `scaled_dt=max_tick_rate`, then `*=time_scale`; `runtime+=scaled_dt` unless paused (:246-259).
3. `Gource::update` order: stop_at_time check -> logic -> draw -> if exporter && commitlog && !shutdown && `framecount % (frameskip+1)==0` dump -> cursor (real dt) if !hide_mouse -> framecount++ (gource.cpp:246-286).
4. Stop flags: exporter present (and not --dont-stop/--loop/path "-") forces stop_at_end (gource_shell.cpp:219-226); multi-repo default stop_at_time 60 (:228-241); app ends only when `users.empty() && stop_position_reached` (gource.cpp:1401-1403) so fade-out frames exported; `--stop-on-idle` no-op (gource_settings.cpp:1376-1379).
5. `--output-custom-log`: no graphics; `"%lld|%s|%s|%s\n"` timestamp|username|action|filename; `-` stdout (gource.cpp:171-214; main.cpp:119-123).
6. PNG screenshot F12: `gource-%04d.png` first free 1..9999 in CWD else overwrites 9999 (gource.cpp:2234-2255); RGB or RGBA if --transparent; taken near END of draw() after UI, before transient message; 5 s message "Wrote screenshot %s". PNGExporter not instantiated.
7. Clear: background default vec3(0.1,0.1,0.1), alpha 1 (0 if transparent); 3D: gluPerspective fov 90, near 0.1, far max+1, start z=-300 (gource.cpp:140; zoomcamera.cpp:23-26,54,75-82); no depth test enabled; painter's order.
8. Pass order (draw gource.cpp:2401-2780): 0 clear/bg image; (loading screen if no commitlog: "Reading Log"/"Aborting" + `int(runtime*3)%4` dots, font 14*scale at (w/2-tw/2,h/2-10)); 1 cull/project/updateVBOs; 2 edges (ortho, beam.png; shadow shader strength 0.5 translate (+2,+2) px, then plain); quad per spline segment half-width 2.5 px; edge_detail=min(10,int(angle/PI*100)) min 1; 3 file shadows (+2,+2 world, 0.5); 4 actions (src half-width 0.3*(target size/2) alpha 0.1a, dest half-width target size/2 alpha a=1-progress); 5 files; 6 user shadows ((2,2)*user_scale); 7 users; 8 bloom blend ONE,ONE, bloom_vertex 64 B (pos vec2@0, colour vec4@8, texcoord vec4@24, 24 B pad); per dir R=dir_radius*2*bloom_multiplier, quad 2R, colour dir colour*bloom_intensity alpha 1, texcoord (R,pos.x,pos.y,0); 9 text (shader buffer: dir names, unselected users/files; then selected user/file with CPU shadow); 10 debug; 11 2D UI (logo, splash first 15 s bar colour (0,0.5,1,splash*0.015), "Gource" 42px, date fontmedium x=w/2-date_x_offset y=20, title (10,h-10), captions, key, slider, hover textbox, debug, screenshot, message at (1,3)); 12 PPM dump then cursor. All SRC_ALPHA/ONE_MINUS_SRC_ALPHA except bloom. Multi-repo transition: glCopyTexImage2D of last frame blended alpha=transition_interval 1.0 decreasing by dt (gource_shell.cpp:246-300).
9. Shaders: shadow/text vert = `gl_TexCoord[0]=gl_MultiTexCoord0; gl_FrontColor=gl_Color; gl_Position=ftransform();`. shadow.frag: `vec4(0,0,0, gl_Color.w*colour.w*shadow_strength)`. text.frag: `colour_alpha=tex(uv).w; shadow_alpha=tex(uv - vec2(texel_size)).w*shadow_strength; combined=1-(1-shadow_alpha)*(1-colour_alpha); if(combined>0) colour_alpha/=combined; gl_FragColor=gl_Color*vec4(vec3(colour_alpha),combined)`. bloom.vert: `pos=gl_Vertex.xyz-gl_MultiTexCoord0.yzw`. bloom.frag: `r=fract(sin(dot(pos.xy,vec2(11.3713,67.3219)))*2351.3718); offset=(0.5-r)*tc.x*0.045; intensity=min(1.0,cos((length(pos*2.0)+offset)/tc.x)); gradient=intensity*smoothstep(0.0,2.0,intensity); gradient*=smoothstep(1.0,0.67+r*0.33,1.0-intensity); gl_FragColor=gl_Color*gradient`. Uniforms: shadow_strength 0.5 (gource.cpp:2146-2147, 2307-2308, 2333-2334), text shadow_strength 0.7 (:2524), texel_size=1/min(512,GL_MAX_TEXTURE_SIZE) (:89-92, 2525), bloom none.
10. Shader gotchas: bloom `smoothstep(1.0, 0.67+r*0.33, x)` has edge0>edge1 (undefined in spec; implement `1 - smoothstep(0.67+r*0.33, 1.0, x)`); negative cos beyond pi/2 must be clamped >=0; text shadow = 1 texel up-left sample for all glyphs in the buffer regardless of dropShadow; FFP path shadow only when `shadow && !fontmanager.use_vbo`.
11. Assets: FreeSans.ttf 697 KB; file.png 256x256 RGBA (mipmaps, clamp); user.png 384x512 (graphic_ratio 4/3); beam.png 128x1 (no mipmaps, default REPEAT); bloom.tga 512x512 RGB RLE and bloom_alpha.tga RGBA RLE (FFP only; shader path ignores). Shader dir has exactly six files.
12. Font scale F = viewport_dpi_ratio.x if >1 else `1 + display.height/1600` (int div) only when default_font_scale (gource.cpp:40-47); `scaled_X=clamp((int)(X*F),1,100)` (gource_settings.cpp:522-527). Sizes: fontlarge 42*F (shadow, round); fontmedium scaled_font_size default 16; fontcaption 16 unscaled; HUD font 14*F; dirname scaled_dirname_font_size 14; user name scaled_user_font_size 14 (selected 18 unscaled); file name 14 (selected 18, colour (1,1,0.3)); key/slider 16*F, textbox 18*F.
13. Pawn formulas: base alpha min(elapsed/fadetime,1) fadetime 1.0 (pawn.h:93); shadow colour (0,0,0,alpha*0.5) (pawn.cpp:20,130-152); quad pawn.cpp:154-190; file size 8.0*1.05 (file.cpp:20,29); user size 20*user_scale (user.cpp:29); user colour colourHash then `c=c*0.6+0.4; c*=0.9` (user.cpp:219-220), selected white; user alpha after user_idle_time 3.0: `1-min(elapsed-last_action-idle,1)` (user.cpp:343-349); file alpha fade `1-clamp(elapsed-fade_start,0,1)`.
14. Name fade: `done=nametime-name_interval; done<1: a=done; done>1 && <nametime-1: 1; else nametime-done` (pawn.cpp:112-127; strict `>1.0` so done==1.0 gives 4); nametime 5.
15. UI geometry (T2.d): FileKey entry width 90*F, height scaled_font_size+4, shadow offset (3,3) alpha a*0.333, gradient left col*0.5 right col; rebuilt each 1.0 s; sort count desc then ext asc; max rows `max(0,(int)((height-150)/20))`; Slider bounds min (35,height-70) max (width-35,height-35), shadow (2,2) alpha 0.7*alpha; TextBox rect_width=max(getWidth)+6, rect_height=2+n*(fontsize+4), fill = pawn colour*0.5 (gource.cpp:143), shadow (3,3) 0.333.
16. Label placement: user name anchor (pos.x, pos.y - dims.y*0.5) gluProject y-flipped, x=sx-namewidth*0.5, y=sy-getMaxHeight (user.cpp:370-395); file name anchor absPos+(5.5, selected?-2:-1) (file.cpp:262-291); dir names: label_pos weights default dir_name_position 0.5: `s=0.5-abs(pos-0.5); p=1-s; label=pos1*(p*(1-pos))+pos2*(p*pos)+spos*s`; dir name alpha `highlight_dirs?1:max(0,5-since_change)/5`; skipped if root, hide_dirnames, depth rule, since_last_node_change>5.
17. Rust checklist (sec. 8): view-proj uniform fov 90; triangulate quads (0,1,2,0,2,3); offscreen Rgba8Unorm (not sRGB), 256-byte row padding on readback; no depth buffer; cursor, message text, and PPM alpha excluded; pixel exactness NOT required.
Gotchas: DPI branch: default high_dpi_aware=false so macOS 1x backbuffer unless --high-dpi (display.cpp:47); caption stacking floor/stack by fontcaption max height; screenshot includes UI excludes cursor/message.
Verification: tick 4 sampled 36 claims; corrections inline (pawn.cpp lines shifted, user colour tint lines, filename_colour not white, setting-driven font sizes, FFP shadow condition, name-fade boundary). Not re-verified: Windows DPI (display.cpp:549-593), frameless toggle, caption stacking (gource.cpp:1778-1846), GLU resampling.
Open: GLU rounding for 384-wide user.png; Bounds2D line width HiDPI; where --high-dpi calls enableHighDPIAwareness; file_scale effect; FXGlyphSet drawToVBO fully; FileKey/Slider/TextBox full draw geometry partially.

---
### 3. deps-and-rust-mapping.md (279 lines)
Purpose: build system, packaging, C++ dependency use, crate candidates (verified on crates.io 2026-10-29), license analysis, rand() port, GL stack recommendation, git quoted-path claim.
Map: 1 Version/license/history; 2 Repo layout; 3 Build artefacts/flags; 4 C++ deps table; 5 Install layout/resource lookup; 6 Bundled TinyXML; 7 Packaging (Windows/Linux/macOS/CI); 8 Crate candidates (+8.1 verified license table, cross-cutting risks); 9 Gaps; Tick 2: T2.1 rand() port, T2.2 legacy-GL options, T2.3 git quoted path.

Key facts:
1. Version `0.57` (`GOURCE_VERSION` gource_settings.h:21; printed `Gource v%s` first line of -h/-H gource_settings.cpp:50). NO `--version` option: exit 1 "unknown option" (corrected tick 4). configure.ac:6 AC_INIT(Gource, 0.57). License GPLv3+ (COPYING:1-2); port must stay GPL-3.0-or-later. FreeType: use FTL (GPLv2-only incompatible with GPLv3).
2. Bundled: TinyXML 2.6.2 (zlib), FTGL-derived fxfont, utf8-cpp (Boost), FreeSans.ttf (GPLv3+ font exception).
3. Build flags: `-std=gnu++0x -Wall ...` (Makefile.am:5); tga.cpp in gource.pro:84 not Makefile.am; tinyxml conditional on USE_BUNDLED_TINYXML (Makefile.am:57-63); `AC_CHECK_FUNCS([timegm unsetenv])` (configure.ac:17); `AX_BOOST_BASE([1.69])` (:49); SDL2_image needs PNG+JPEG (:45-46); freetype2 >= 9.0.3 (:36).
4. Install layout: `<prefix>/share/gource` data: `beam.png bloom.tga bloom_alpha.tga file.png user.png gource.style` (Makefile.am:69), shaders `{shadow,bloom,text}.{vert,frag}`, fonts FreeSans.ttf; man gource.1.gz (:74-79). Retain gource.style byte-for-byte (hg format uses it, hg.cpp:47).
5. SDL use: compat profile (display.cpp:254), SDL_Init(TIMER|VIDEO) (display.cpp:590), threads for logmill/ppm/png/tga, SDL_GetTicks 37 uses.
6. PCRE2: `pcre2_compile` flags=0 (regex.cpp:35-42); negative lookahead `(?!-)` (gource_settings.cpp:743); BOM `\xEF\xBB\xBF` (gource.cpp:1080); user filters (gource_settings.cpp:1527-1596). Highest compat risk (sec. 8 cross-cutting #1). Recommendation: `pcre2` crate for user filters; `regex::bytes` `(?-u)` for internal.
7. Crates (versions @2026-10-29): winit 0.30.13, glutin 0.32.3 (2025-04-30), glutin-winit 0.5.0 (2024-06-08), sdl2 0.38.0, sdl3 0.20.0, glow 0.18.0, wgpu 30.0.1, glam 0.34.1, image 0.25.10, png 0.18.1, fontdue 0.9.5, ab_glyph 0.2.32, freetype-rs 0.38.0, pcre2 0.2.11, fancy-regex 0.20.0, regex 1.13.1, roxmltree 0.21.2, quick-xml 0.42.0, chrono 0.4.45, jiff 0.2.38, clap 4.6.7, lexopt 0.3.2. All GPLv3-compatible (table 8.1). Font: unhinted fontdue/ab_glyph drift from FT hinted; freetype-rs gives identical glyphs.
8. DEP T2.2 recommended `sdl2`+`glow` GL 3.2 core `#version 150` (macOS caps at 3.2; naga GLSL frontend only 440/450/460). NOTE: LEDGER Decisions later chose winit+glutin+glow instead (see sec. 8/LEDGER) -- conflict, user decision wins.
9. rand() usage: dirnode.cpp:625, user.cpp:106,128, gource.cpp:1555; `srand(time(0))` only for `--start-position random` (gource_settings.cpp:1319). glibc TYPE_3 algorithm: seed s (0->1), r[0]=s, i=1..30 `hi=w/127773; lo=w-hi*127773; w=16807*lo-2836*hi; if w<0 w+=2147483647`; fptr=3,rptr=0; discard 310 steps (`r[f]+=r[r]` u32, output `>>1`, indices wrap at 31); each rand() one more step. First outputs `1804289383, 846930886, 1681692777, 1714636915` (from a Python model, NOT a real libc run). Correction: warm-up is 310 not 344. macOS = Park-Miller `16807*(x%127773)-2836*(x/127773)`, init 1 (zero state->123459876); EMPIRICAL on host: srand(1) -> `16807 282475249 1622650073 984943658`.
10. Git quoted paths (git.cpp:234-239): strip first/last byte only if file starts AND ends with `"`; size<=2 skipped; NO octal/escape unescaping. Status = line.substr(tab-1,1); file=substr(tab+1); rename lines not split (`old<TAB>new` one path); lines with no tab / tab at 0 or last skipped (:227,229-230).
11. TinyXML consumers: svn (svn.cpp:206-263) and cvs2cl (cvs2cl.cpp:96-147); lenient parsing; keep skip-bad-entry path. VCS invoked by `system()` into temp file (commitlog.cpp:94, 257-290) with shell-quoted args; preserve flags verbatim.
12. macOS: default window height 640 (settings.cpp:69-71); frameless toggle disabled (display.cpp:403-406); NFC normalise avatar names (gource_settings.cpp:608-610, 848-858); no CI in repo (.github has only FUNDING.yml).
Gotchas: Homebrew libs x86_64; qmake links boost_system (autoconf dropped in 0.56); data/cursor.png whitelisted in build_tar.pl:66 but nonexistent.
Verification: tick 2 verified all 40 crate versions/licenses vs crates.io API; configure.ac line cites were shifted (FreeType :36, SDL2_image :45-46, PCRE2 37, GLEW 38, SDL2 39, libpng 40, GLM 65, TinyXML 67-74, GLU 31). Tick 4: "--version" statement fixed. Transitive licenses unchecked.
Open: MSYS2 DLL list not in repo; ui/* Boost.Filesystem detail only grepped; README not inspected beyond license/high-dpi; Windows MSVC rand() not researched; apple Libc variant confirmed empirically only.

---
### 4. rust-architecture-map.md (398 lines)
Purpose: synthesis/start-here doc: crate layout, C++ file to Rust module map, data model, containers, threading, renderer boundary, settings/CLI model, regex, PRNG/timing, platform notes, phases with parity gates, decisions.
Map: 1 Workspace layout (1.1 crates, 1.2 module tree, 1.3 file map); 2 Core data model (2.1 commit types, 2.2 arena tree, 2.3 other structs, 2.4 BTreeMap vs Vec); 3 Threading (3.1 LogMill, 3.2 PPM); 4 Renderer boundary (4.1 trait, 4.2 stack, 4.3 pass checklist, 4.4 shader checklist); 5 Settings/CLI (5.1 hand-rolled, 5.2 ConfFile, 5.3 parseArgs-into-ConfFile); 6 Regex; 7 PRNG and timing; 8 Platform; 9 Phases; 10 Decisions; Verification (tick 4).

Key facts:
1. Crates: `gource-util` (prng, strhash, math, bounds, quadtree, utf8, regex, timeparse), `gource-conf` (conffile, args, settings, help), `gource-formats` (seeklog, commit, nine parsers, logmill), `gource-sim` (dir_tree arena, file, user, action, pawn, camera, gource, shell, dump), `gource-render` (Renderer trait, scene, text, ui, spline, bloom), `gource-gl` (backend), `gource` bin. Sim never imports renderer/GL/SDL types.
2. Arena DirTree: `DirTree{nodes: Vec<Option<DirNode>>, free: Vec<u32>, root: DirId, by_path: BTreeMap<String,DirId>}` with DirId/FileId newtypes; NOT Rc<RefCell>. Never `swap_remove` in children/files (insertion order drives f32 accumulation and rand() order); use `Vec::remove`/`retain`. C++ ctor shadows `parent` (always 0 until setParent) so Rust has no ctor parent param.
3. Container mapping: users `BTreeMap<String,UserId>`, files `BTreeMap<String,FileId>`, `gGourceDirMap` `BTreeMap<String,DirId>` (abspath with trailing '/'), tagusermap `BTreeMap<i32,..>`, FileKey keymap `BTreeMap`, ConfFile `BTreeMap<String,Vec<Section>>`; actions `VecDeque`; commitqueue `VecDeque<Commit>` refill when `queue.empty() || (back.timestamp <= currtime && size < 100)`. Commits NEVER sorted (gource.cpp:1131-1154, 1744-1774; commitlog.cpp:128,212-219); without `--no-time-travel`, `currtime = commit.timestamp` when `lasttime > commit.timestamp` (:1765-1767). NO HashMap in sim state. `getFilesRecursive` builds by prepending.
4. Process globals (gStringHashSeed, gGourceRemovedFiles, gGourceDirMap, gGourceForceGravity) become fields of `SimCtx{hash_seed:i32, removed_files, rng}`.
5. ZoomCamera: `toa = tan(fov*0.5*0.017453292)*2.0` with f64 literal narrowed to f32 = 1.99999988 not 2.0; dir file_area constant stored f32 = 50.26548386; `PI` f64 literal 3.14159265 not std PI. All f32; mirror per-expression f64 literal narrowing.
6. Threading: LogMill = std::thread returning Result<Box<dyn CommitLog+Send>>; AtomicU8 state; worker also runs format detection and `--start-date` scan (logmill.cpp:86-97); stdin must be non-blocking-semantics ("no line yet" != EOF; final unterminated line dropped) -> stdin pump thread + mpsc; PPM writer = `sync_channel(1)` + recycled buffers; header every frame; handle BrokenPipe; no stdout prints while `-o -`.
7. Renderer trait: begin_frame, set_camera, draw_batch, draw_text, text_metrics, read_back_rgb, present; `Pipeline{Textured,Shadow,Text,Bloom,Solid}`. Mouse picking is the only render-to-sim feedback (kept in `gource` app).
8. Stack: doc text says sdl2 0.38 + glow 0.18 + glam, GL 3.2 core `#version 150` (SUPERSEDED by LEDGER decision winit+glutin+glow). Shader checklist: #version 150, `in vec3 position; uniform mat4 mvp`, keep vec4 texcoord (bloom .yzw), A8 atlas as GL_R8, global VAO, glGenerateMipmap, beam.png REPEAT, thick lines as quads.
9. CLI: hand-rolled parser (not clap). parseArgs: all leading `-` stripped, lone `-` positional, `--` unknown option, no `--opt=value`, value option consumes next token even if starts with `-`, `-WxH` regex `^([0-9.]+)x([0-9.]+)(!)?$` -> `[display] viewport`. Help byte-exact via include_str!. Errors `gource: <msg>\nTry 'gource --help' for more information.\n\n` stderr exit 1. Command options (-h,-H,--load-config,--save-config,--log-command,--*-command,--output-custom-log,--log-level) run immediately in both passes.
10. ConfFile grammar: `#` only col 0; section `^\s*\[([^\]]+)\]\s*$`; entry `^\s*([^=\s]+)\s*=\s*([^\s].*)?$`; error `<file>, line <n>: could not parse line`; first entry of key wins for scalars; save sorted by section/key.
11. parseArgs flow: conf positional ending `.conf`(len>5)/`.cfg`/`.ini`(len>4) becomes load_config; if load_config `conf.clear(); load; parse_args again`; last positional = `path` set on EVERY `[gource]` section (or create one); per-option: Multi types add_entry, others set_entry on every section of target name. Per-repo GourceSettings reset: fresh defaults incl. `gStringHashSeed=31`; NOT reset: repo_count, shutdown, load_config, save_config, output_custom_filename, file_graphic, log_level, display settings. Fixups (gource_shell.cpp:221-242): exporter && !(dont_stop||loop||path=="-") -> stop_at_end; repo_count>1 && stop_at_time<=0 && stop_position<=0 -> 60.0; wrap to first repo only when repo_count>1 && exporter==0.
12. Regex wrapper semantics (regex.cpp:149-208): unanchored search; bytes; `match()` returns groups 1..=N highest participating, trailing non-participating dropped, middle empty; empty subject never matches (offset>=size -> -1, :174); PCRE2 `$` matches before final `\n` while Rust regex does not; flags=0.
13. PRNG: `trait SimRng{fn next_i31()}`; default documented Rust PRNG (xorshift) fixed seed; modes `GOURCE_RS_PRNG=xorshift|glibc|park|script` (`--rand-script FILE`); `vec2(rand()%100-50, rand()%100-50)` draw x then y (clang evaluates x first). 4 rand sites listed.
14. Timing (TIM authoritative; min delta 16 ms): interactive loop `delta = whole accumulated buffer`; GourceShell::update if no current Gource or finished: get_next() and return, no update/draw/dump (gource_shell.cpp:285-288). `time_inc` is f32: `(dt as f64 * 86400.0 * days_per_second as f64) as f32; seconds = time_inc as i32; subseconds += time_inc - seconds as f32; if subseconds>=1.0 { currtime += subseconds as i64; subseconds -= (subseconds as i32) as f32 }; currtime += seconds` (gource.cpp:1723-1733; corrected tick 4).
15. Phases 0-7 with gates: 0 util (cargo tests incl. 28 datetime vectors, stringhash, glibc outputs once confirmed); 1 conf (byte-exact -h/-H, exit codes, unknown option for --version); 2 formats (custom-log goldens exact); 3 headless sim + `--debug-dump-state` JSONL (exact tier topology/colours; position tier 1e-3 relative; `--hash-seed 31 --disable-auto-rotate --stop-at-time 10`); 4 camera/selection/captions/multi-section; 5 renderer (valid P6 stream, cadence, SSIM >= ~0.9 on every 30th frame under Xvfb/llvmpipe); 6 UI/input/interactive loop; 7 packaging/CI. Phase N+1 only after N gate green. Sabotage-check each gate.
Gotchas: `cvs-exp` unmatched by logmill (only "cvs", logmill.cpp:238; gource_settings.cpp:779,784); help says user-friction 0.67 but code default 1.0; timing.md sec. 2 last paragraph and DEP sec.1 `--version` wording are wrong and not edited in their own docs.
Verification: tick 4 cross-checked vs Makefile.am, source, timing/scene/app-core/log-formats/settings/misc-traces/verification-tick3-scene/parity-test-spec; list of "corrected tick 4" marks. Open: D1/D2 default/flag names (since decided in LEDGER); glibc rand unverified vs real libc (macOS Park-Miller since confirmed by a compiled test).

---
### 5. build-and-goldens.md (52 lines)
Purpose: record of building the C++ oracle on this machine and capturing goldens.
Map: Environment; Dependencies; Needed to proceed; Direct build; Goldens.
Facts:
1. macOS Darwin 25.5.0 arm64 shell; Homebrew at /usr/local is x86_64 -> must compile with `-arch x86_64` (runs via Rosetta); without it SDL.h fails with `raointintrin.h: __builtin_ia32_aadd32`. autoconf/automake/libtool MISSING (no pre-generated `configure`) so autotools blocked; direct clang++ build succeeded.
2. Deps found: sdl2 2.30.1, SDL2_image 2.8.2, glew 2.2.0, freetype2 26.6.20, pcre2 10.47, libpng 1.6.59, boost 1.84.0_1, glm 1.0.1.
3. Compile (53 files = src/main.cpp + Makefile.am sources + src/tinyxml/*.cpp, zero failures): `clang++ -arch x86_64 -std=c++17 -O1 -w -I src -I src/core -I src/tinyxml -DSDLAPP_RESOURCE_DIR=\"$S/data\" -DHAVE_TIMEGM -DHAVE_UNSETENV $(pkg-config --cflags sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -I/usr/local/include -I/usr/local/opt/boost/include -c FILE -o obj/FILE.o`. Link: `clang++ -arch x86_64 obj/*.o $(pkg-config --libs ...) -L/usr/local/opt/boost/lib -lboost_filesystem -lboost_system -framework OpenGL -framework CoreFoundation -o gource` -> 868 KB x86_64 Mach-O. Scripts: golden/scripts/{build.sh,link.sh,gold.sh,tbuild.sh} + README (parameterised BUILD=).
4. Goldens (golden/, 20 s background-kill timeout; no `timeout` binary): help-short.txt (`-h` rc0), help-long.txt (`-H` rc0), `--version` rc1 stderr "unknown option version" (version.stderr.txt), custom-log/{custom-dir-delete,file-removal,file-to-dir,utf8}.txt via `--log-format custom --output-custom-log - tests/logs/NAME.log`, custom-log/svn-dir-delete.txt via `--log-format svn`; exit-codes.txt.
5. utf8-caption.log with `--log-format custom`: rc1 "unsupported log format" (caption file, no golden). `SDL_VIDEODRIVER=dummy gource --stop-at-time 1 -o - tests/logs/utf8.log` -> rc1 "SDL initialization failed - OpenGL support is either not configured in SDL or not available in current SDL video driver (dummy)". `--output-custom-log` needs no window.
Gotchas: the file's "Needed to proceed" paragraph was truncated mid-sentence; it is fixed (committed in PR #2) and now reads "...automake libtool", with a new "Autotools route (not needed)" section. Verification: not independently re-verified in verification-tick2 (not sampled); LEDGER says build attempt succeeded. Open: real-window run not attempted.

---
### 6. tests-and-golden-strategy.md (84 lines)
Purpose: existing tests, fixtures, headless capability, determinism sources, 3-tier golden strategy.
Map: 1 Existing tests; 2 tests/test.conf; 3 Fixtures; 4 Headless capability; 5 Randomness; 6 Golden strategy (1 parser, 2 sim state, 3 visual); Gaps.
Facts:
1. Boost.Test: `check_PROGRAMS=gource_tests` (Makefile.am:92-103), sources src/test/{main,datetime_tests,regex_tests}.cpp. datetime test: `putenv("TZ=UTC")` l.27 then `TZ=Pacific/Auckland` l.31 (stays); vectors: UTC `2021-11-01`->1635724800; Auckland `2021-11-01`->1635678000; `2021-11-01Z`/`+0` 1635724800; `+13` 1635678000; `+5:30` 1635705000; `2021-11-01 12:01`->1635721260; `12:01Z` 1635768060; `12:01+13` 1635721260; `12:01+5:30` 1635748260; `12:01:59`->1635721319; `T12:01:59.123` 1635721319 (fraction ignored); `12:01:59Z` 1635768119; `+5:30` 1635748319; `+13` case listed twice (l.101/104). Zone-less input uses process TZ.
2. regex_tests: `^[A-Z]+$` ok; `ABC` true, `abc` false, `ABC123` false; copy valid; `^[A-Z+$` throws RegexCompilationException, with test=true isValid()==false; `([0-9]+)` on "ABC123" matches.size()==1, [0]=="123".
3. tests/test.conf: 8 repeated `[gource]` sections (File removal, file-to-dir, user-filter=Andrew, file-filter=\.cpp, file-show-filter=\.cpp, svn-dir-delete, custom-dir-delete, UTF-8 w/ caption-file=logs/utf8-caption.log user-image-dir=images/); no expected outputs; run from tests/ (relative paths); cases 3-5 have no path (need positional).
4. Fixtures: custom-dir-delete.log, file-removal.log (blank trailing line, last delete at +11s), file-to-dir.log, svn-dir-delete.log (attributes split across lines, empty kind=""), utf8.log (`Árvíztűrő tükörfúrógép`), utf8-caption.log (caption file `timestamp|text`), images/Árvíztűrő tükörfúrógép.png (NFC/NFD risk).
5. Headless: only `--output-custom-log` (gource_settings.cpp:113,250,329,583-585; main.cpp:119-122 exit(0)) loses commit grouping and colours. Full sim not headless: SDL_CreateWindow + compat GL (display.cpp:219-256); SDL dummy driver lacks GL; use Xvfb + llvmpipe (`LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe`). Exporter: `scaled_dt=max_tick_rate`, stop-at-time vs runtime.
6. Randomness: rand() at dirnode.cpp:625, user.cpp:106,128, gource.cpp:1555; srand(time) gource_settings.cpp:1319-1320; gStringHashSeed default 31 (:489), `--hash-seed` :1062-1066; SDL_GetTicks only profiling (gource.cpp:263-267).
7. Strategy (1) parser diff: `gource --output-custom-log - f` vs Rust, skip utf8-caption.log; also `--git-log-command` etc.; edge inputs empty/CRLF/blank/out-of-order/`|`/BOM. (2) `--debug-dump-state FILE` JSONL every N frames, floats `%.4f`, cmd `gource tests/logs/file-removal.log -o /dev/null -r 60 --seconds-per-day 1 --stop-at-time 10 --hash-seed 31 --disable-auto-rotate --debug-dump-state out.jsonl`; exact tier topology/frames/colours, tolerance ~1e-3 relative for positions unless glibc rand ported with identical f32 op order. (3) visual: xvfb-run 1280x720x24, SSIM >= ~0.9 every 30th frame.
Gotchas: no CI, nothing runs test.conf. Verification: tick 2 OK (rules :92-103, TZ lines, duplicate +13, rand sites, hash-seed 489). Open (gaps): multi-section sequencing (since traced in SET T2.1), approximate `~` line numbers (gource.cpp:171-215, 245-262), SDL offscreen driver unverified.

---
### 7. verification-tick2.md (54 lines)
Purpose: sample check of 32 citations across eight tick-1 docs.
Map: table (doc, claim, cite, status); Systemic issues.
Facts: ~30 OK, 8 SHIFTED, 0 WRONG on substance. Shifted lines: scene-model hash seed reset :490 -> gource_settings.cpp:489; log-formats logmill start_timestamp :86 -> :85; colourHash starts stringhash.cpp:71 (not 73); output-pipeline start camera z=-300 gource.cpp:138 -> :140; deps FreeType configure.ac:39 -> :36; SDL2_image :48-49 -> :45-46; configure.ac PCRE2/GLEW/SDL2/libpng/GLM/TinyXML/GLU actual 37,38,39,40,65,67-74,31. Confirmed substance: SplineEdge edge_detail=min(10,(int)(ang*100.0)) floor 1 (spline.cpp:23-69,37-39); FileKeyEntry shadow alpha*0.333, gradient *0.5 (key.cpp:113-152); interactDirs +/-1, area>10000 (gource.cpp:1418-1449); user.cpp:20-22 constants beam 100/action 50/personal space 100; user-friction help 0.67 vs code 1.0 (gource_settings.cpp:162,472,1411-1421); file-idle-time atoi invalid if <0 or (==0 and first char != '0') (:1222); padding invalid if <=0 or >=2, default 1.1 (:1466,431); `--user-idle-time` conf only (:1248); logmill auto-detect order git, hg, bzr, git-raw, cvs-exp, svn, cvs2cl, custom, apache (logmill.cpp:272-334); first-char checks git 'u', gitraw 'c', svn '<', cvs2cl '<'; all verbatim regexes OK char-for-char; regex.cpp match 149, empty-subject 174, trailing groups 195-208.
Not verified: external crate versions/licences (network facts), Windows/macOS-only paths. Cross-doc inconsistency noted: scene-model cites :490 vs tests doc :489 (patched). Open: none beyond above.

---
### 8. LEDGER.md (125 lines)
Purpose: discovery ledger: goal, loop config, doc index, agent budget, done checklist, cross-cutting findings, USER DECISIONS, reading order.
Map: header (goal, loop, stop rule, legend); Documents table; Agent budget; Done criterion checklist; Key findings tick 1; Key findings tick 2; Decisions flagged; Stop rule MET; Residual known-unverified; Decisions (user, 2026-10-30); D5 default pending; Reading order.
Facts:
1. Goal: Rust rewrite with same CLI/config/log-format API and same parsing + simulation state, rendering visually equivalent. Loop 5-minute ticks, 100-agent budget (29 spent, 71 unspent); started 2026-10-29T16:58:47Z; stopped 17:47Z after 4 ticks; stop rule MET (all checklist `[x]`, verifier pass over every doc, build produced goldens).
2. Docs index also lists timing.md (AUTHORITATIVE for timing), misc-traces.md, unit-tests.md (69 assertions pass; 28 datetime goldens), parity-test-spec.md (28-row matrix, JSONL schema, PRNG modes), verification-tick3-scene.md (9-row discrepancy table), golden/ (help, exit-codes, custom-log for 5 fixtures plus stdin variants, unit-tests.txt, glibc_rand.py).
3. Reading order: ARCH, parity-test-spec, timing, settings-cli + golden/help-*, log-formats + golden/custom-log, scene-model + verification-tick3-scene, app-core, core-lib, output-pipeline, deps, build-and-goldens + unit-tests, misc-traces.
4. Cross-cutting: legacy GL redraw; PCRE2 needed (regex crate insufficient); sim depends on rand(), sorted-map order, quadtree visit order (not std::set order), fixed-dt in export; timing min delta 16 ms; dump phase depends on log load time; CLI scalars apply to every [gource] section; utf8 replace_invalid != from_utf8_lossy.
5. USER DECISIONS (2026-10-30): Parity = same parsed commits + sim state, render visually equivalent. Rust workspace lives in this repo beside C++ (C++ stays until parity proven). First release targets macOS and Linux; Windows users via Docker, headless frame export only. Delivery: two PRs against master, docs first then phase 1 code. D1 validate every [gource] section up front. D2 export mode waits for log load (keep flag for old behaviour). D3 documented Rust PRNG default with glibc and macOS compat modes. D4 `--ffp` accepted and ignored. D5 fix all four known C++ bugs now: user-friction and dir-name-position help-text mismatches fixed by correcting HELP TEXT to match code (code change would alter sim state); `--log-format cvs-exp` made to work; `--stop-on-idle` pending (default: keep accepting, document as no effect, no invented semantics). Help goldens will therefore differ on those lines; record expected diff.
6. Renderer stack DECISION: `winit` + `glutin` + `glow` (chosen over recommended sdl2+glow). Consequences: input/IME/clipboard/cursor/high-DPI semantic differences need explicit parity tests; glutin (2025-04-30) and glutin-winit (2024-06-08) slow, pin exact versions, keep window/GL behind renderer trait; no C SDL2 needed.
7. D6: fork the C++ sources (chosen over patch file). Keep untouched C++ build as oracle for byte-exact goldens (help, exit codes, custom-log); fork only emits per-frame state dump; fork must reproduce same custom-log and help goldens as original before dump trusted. Fork dir location/name TBD.
Residual unverified: glibc rand outputs from Python model only; macOS Park-Miller confirmed by compiled test; MSVC rand not researched; `vec2(rand()%100-50, rand()%100-50)` eval order assumed clang left-to-right; stdin empty-startup and `--stop-at-end` with stdin read from source only; PPM pipe/SIGPIPE not exercised; no render golden (needs GL window); Windows-only paths from source only; transitive crate licences unchecked; tests/test.conf referenced by nothing.
Gotchas: ledger table rows ordered 1,4,total,3,2 (cosmetic); date of decisions 2026-10-30 vs discovery 2026-10-29. Conflicts: ARCH 4.2/DEP T2.2 recommend sdl2+glow; LEDGER Decisions override with winit+glutin+glow. ARCH 10 says `--stop-on-idle`: accept & ignore; LEDGER D5 pending call. ARCH 10 lists help-text mismatch "keep verbatim (golden), implement 1.0"; LEDGER D5 says fix the help text instead. ARCH 10 `--ffp` aligns with D4.

---
### 9. CONTEXT.md (9 lines, repo root of worktree)
Purpose: domain glossary. Content: "Gource Rust Rewrite": rewrite of Gource from C++ to Rust; first release matches C++ behaviour; breaking changes/new features only after proven. Term **Parity**: Rust program produces the same parsed commits and same simulation state as the C++ program for same inputs/settings; rendering only visually equivalent, not pixel-identical. Avoid: Compatibility, identical output, same output. (At that survey CONTEXT.md was untracked; it is committed now.)

---
### Cross-doc conflicts / gaps summary
- Stack: DEP T2.2 + ARCH 4.2 (sdl2+glow, GL 3.2 core #version 150) vs LEDGER Decisions (winit+glutin+glow). Latter is the user decision.
- Help text mismatches: ARCH 10 "keep help verbatim, implement 1.0" vs LEDGER D5 "correct help text to match code".
- `--version`: DEP sec.1 earlier wording "printed by --version" wrong (ARCH 5.3 and BLD correct: unknown option, rc 1).
- timing.md sec. 2 last paragraph ("only options present reset") wrong vs ARCH 5.3 item 8 / misc-traces (defaults fully reset).
- COR 0 says min delta 16 ms Gource vs SDLApp default 8 (consistent with TIM; OUT cites sdlapp.cpp:390-460 and COR :405-465 for the loop - same function, differing line range).
- COR says `fontmedium` etc. not covered; OUT T2.b is authoritative for font sizes.

---------------------------------------------------------------
## CROSS-CUTTING: the twenty most important facts for a Rust implementer
(Cites are the docs' own; detail lives in the named doc and in the per-doc sections above. Docs dir: <discovery-worktree>/docs/rust-rewrite/discovery/)

1. Minimum frame delta is 16 ms: GourceShell sets `min_delta_msec = 16` (gource_shell.cpp:29), overriding SDLApp default 8 (sdlapp.cpp:317); the whole accumulated buffer becomes dt, no remainder carried. [timing.md; core-lib.md's "8 ms" is wrong]
2. Sim dt: `scaled_dt = min(dt, max_tick_rate)` (gource.cpp:248), times `time_scale` (:256). Exporter: `scaled_dt = max_tick_rate` (:251-253), `max_tick_rate = 1.0f/gource_fps` (:519); gource_fps = smallest multiple of video fps >= 60 (30 -> 60, 25 -> 75, so dt 1/75 not 1/60; scene-model :358 is wrong). [timing.md, output-pipeline.md]
3. Exporter dumps a frame when `commitlog!=0 && !shutdown && framecount % (frameskip+1)==0` (gource.cpp:272-273); only framerates 25/30/60 are accepted (settings.cpp:456-470). Dump phase depends on async log-load time, so the port must define it explicitly (block on load, phase 0 at first loaded frame). [timing.md, output-pipeline.md]
4. Commits are never sorted: they are consumed in file order; non-monotonic timestamps are handled by the commit loop / time-travel branch (gource.cpp:1758-1769). Do not sort. [app-core.md T2.4, timing.md]
5. `rand()` is platform dependent and part of sim state: call sites gource.cpp:1555, user.cpp:106,128, dirnode.cpp:625; only `srand` is gource_settings.cpp:1319 (`--start-position random`, `srand(time(0))`). Default libc seed otherwise; port must reproduce libc rand() and call order. [app-core.md, deps-and-rust-mapping.md T2.1, parity-test-spec.md]
6. glibc rand() port: the generator discards 310 outputs (deps-and-rust-mapping T2.1, glibc_rand.py). parity-test-spec section 2.4 writes the same sequence as a recurrence whose first output is index r[344] (344 = 31 + 3 + 310), which is not a second discard count. Resolved; but the values are still only a Python model, never run against real glibc. macOS rand() is Park-Miller, confirmed by a compiled test. [deps-and-rust-mapping.md T2.1, parity-test-spec.md, golden/scripts/glibc_rand.py]
7. rand() argument evaluation order in `normalise(vec2(rand()%100-50, rand()%100-50))` is unspecified in C++ (gcc usually right-to-left, clang left-to-right) (user.cpp:106,128; dirnode.cpp:625); which compiler/libc is the parity reference is an open question. [scene-model.md, verification-tick3-scene.md]
8. Quadtree visit order affects f32 accumulation (non-associative) and rand consumption: root depth 1, `allowMoreItems = children.empty() && (depth >= max_depth || items.size() < max_items)` (quadtree.cpp:38); split TL,TR,BL,BR; an item goes into EVERY overlapping child (:96-104); max_items=1, max_depth=1 unless `dir_bounds.area() > 10000.0` then 6 (gource.cpp:23, 1305-1312); rebuilt every call, also while paused. Not std::set order. [scene-model.md, core-lib.md, LEDGER.md]
9. Container iteration order is sim-relevant: users `std::map<string,RUser*>` name-sorted; files fullpath-sorted; `gGourceDirMap` abspath-sorted with trailing '/'; `children`/`files` std::list insertion order. Use BTreeMap/VecDeque as mapped. [scene-model.md, rust-architecture-map.md]
10. CLI scalars apply to every `[gource]` section of a multi-repo config; settings are re-imported per repo (gource_shell.cpp:218) after `setGourceDefaults` (gource_settings.cpp:368-520), so per-repo options reset; misc-traces.md lists keys NOT reset. LEDGER decision D1: validate every `[gource]` section up front. [LEDGER.md, settings-cli.md, misc-traces.md]
11. Option ordering quirks: `--realtime` (:1360-1362) applied after seconds-per-day so it wins; `--disable-auto-skip` (:716-718) processed BEFORE `-a` (:1211) so `-a` re-enables; `-s` stored as `1.0/S`, `S<=0` invalid (:1203); time_scale valid `0 < S <= 4` (:1266-1268) but not enforced on key edits; `--stop-on-idle` is a no-op (:1376). [timing.md, settings-cli.md]
12. Stop conditions: `stop_position_reached` set by stop_at_time (:261), `stop_timestamp != 0 && commit.timestamp > stop_timestamp` (:1148-1150), `stop_at_end && commitlog->isFinished()` (:1172), stop_position (:1175); app finishes when `users.empty() && stop_position_reached` (:1401) or `shutdown && logmill->isFinished()` (:1570); exporter forces `stop_at_end=true` unless `dont_stop || loop || path=="-"` (gource_shell.cpp:224-225). [timing.md]
13. `runtime` is assigned only in the ctor (gource.cpp:157), not reset by reset()/seek; `stop_position_reached` is never reset by reset(); seekTo un-pauses (:1073). [timing.md, app-core.md]
14. Frame/loop order: logic() is a 25-step order and draw() a 24-step order; the paused path skips updateUsers but camera/bounds/quadtrees keep updating. Camera: updateCamera gource.cpp:1475-1550. [app-core.md Part 2, T2.3, T2.5]
15. Float/uninit semantics: accel/dest/touch_colour "uninitialised" are zero via `GLM_FORCE_CTOR_INIT` (src/core/vectors.h:34); `QuadItem::node_count` is genuinely uninit until addItem (quadtree.cpp:497); fade tests use `> 0.0f` so `fade_start == 0.0` never fades; `isVisible()` mutates a latch. Use f32 with no-FMA-contraction semantics for exact parity. [verification-tick3-scene.md, scene-model.md, parity-test-spec.md]
16. stringHash is exact and quirky: `val += str[i] * (gStringHashSeed ^ (n-i))` (XOR, signed char, i32 wrapping), `if(val<0) val=-val` (wrapping_neg); gStringHashSeed default 31, replaced by `(rand()%10000)+1` at gource.cpp:1555. vec2Hash/colourHash formulas in core-lib. [core-lib.md (stringhash.cpp:28-83), app-core.md]
17. Log parsing: custom log regex `^(?:\xEF\xBB\xBF)?([^|]+)\|([^|]*)\|([ADM]?)\|([^|]+)(?:\|#?([a-fA-F0-9]{6}))?` (custom.cpp:21); timestamp is date-parsed if len>1 and `-` at idx>=1 else atoll; the git log command is `git log --reverse --raw --encoding=UTF-8 --no-renames ... --pretty=format:user:%aN%n%ct` (git.cpp:88-128); git quoted paths: strip first/last quote only, NO unescaping (git.cpp:234-239). PCRE2 is needed (regex crate insufficient). [log-formats.md, LEDGER.md, deps-and-rust-mapping.md]
18. Config grammar: `#` only in column 0; section `^\s*\[([^\]]+)\]\s*$`; entry `^\s*([^=\s]+)\s*=\s*([^\s].*)?$`; error `<file>, line <n>: could not parse line`; first entry of a key wins for scalars; save sorted by section/key. Positional `.conf/.cfg/.ini` auto-detected as load_config (main.cpp:45-72). [core-lib.md, settings-cli.md]
19. Wall-clock affects the sim in only these places: interactive dt via SDL_GetTicks (sdlapp.cpp:414,424) and start/stop date parsing in local TZ (gource_settings.cpp:1280,1297,1306; core/settings.cpp:331-357); format parsers use mktime/timegm (cvs2cl:128, apache:81, cvs-exp:82, bzr:117, svn:239/241). Tests must pin TZ. [timing.md]
20. Project decisions (LEDGER 2026-10-30): parity = same parsed commits + sim state, render visually equivalent; Rust workspace beside C++ (C++ stays until parity proven); first release macOS+Linux, Windows via Docker headless export; two PRs (docs, then phase 1). Renderer stack DECIDED as winit+glutin+glow, overriding the sdl2 0.38 + glow 0.18, GL 3.2 core `#version 150` recommendation in deps-and-rust-mapping T2.2 / rust-architecture-map 4.2. [LEDGER.md, CONTEXT.md]

### Known conflicts between docs
- Renderer: deps-and-rust-mapping T2.2 + rust-architecture-map 4.2 (sdl2+glow) vs LEDGER Decisions (winit+glutin+glow). LEDGER wins.
- Help text: rust-architecture-map 10 "keep help verbatim, implement 1.0" vs LEDGER D5 "correct help text to match code".
- Exporter dt: scene-model :358 (1/60) vs timing.md (1/gource_fps). timing.md wins.
- Pause semantics/idle_time: app-core :233/:410/:429 vs timing.md (idle_time lives in updateUsers). timing.md wins.
- glibc rand discards: 310 (deps T2.1, glibc_rand.py) vs "344" (parity-test-spec 2.4). Resolved: 310 discarded outputs is correct; 344 is an index (31 + 3 + 310). Parity-test-spec's wording should say so.
- `--user-idle-time`: conf-only in settings-cli vs used on CLI in parity-test-spec. Resolved (Part 8): config-only; the built binary rejects it on the command line.
- core-lib "8 ms" min delta vs 16 ms (timing.md). 16 wins.

# Part 8: Conflicts found and how they were resolved

| Conflict | Resolution | Evidence |
|---|---|---|
| glibc `rand()` warm-up: 310 versus 344 discarded outputs | 310 discarded outputs. 344 is the recurrence index (31 + 3 + 310), not a discard count. Only a model in Python was used; it was never run against a real glibc | `discovery/deps-and-rust-mapping.md` tick 2 additions |
| `--user-idle-time`: config-only or also a CLI option | Config-only. Running the built C++ binary with `--user-idle-time 5` prints `gource: unknown option user-idle-time` and the Try line | Run on 2026-10-30 against the oracle build |
| Is the phase 1 worktree on a named branch | Yes. On 2026-10-30 the worktree HEAD was detached at `179b76e`, the same SHA as the branch; by 2026-10-31 the branch is checked out there | `git rev-parse`, `git branch -vv` |
| Was `build-and-goldens.md` truncated | Yes, at the "Needed to proceed" paragraph, with the next heading glued on. Fixed and relabelled "Autotools route (not needed)" | Edited 2026-10-30 |
| Did the user's merge of the discovery branch into `setup-agent-skills` complete | Neither completed nor aborted: still paused and uncommitted as of 2026-10-31. `.git/MERGE_HEAD` = `6a9d08f93e7ff292c7cff470f13aa5bfcae31f0f` (the OLD discovery tip), vim waits at the commit message, the branch is still `6ea1d6f` (already on `origin/master` as `216ec5d`, PR #1). `master` now has the docs, so finishing is pointless: the user should type `:cq`, then run `git merge --abort` | Part 1 B.5, Part 2 |
| PR #3 commit count: 6 not 5 | 6 commits not on master: `e2d0c64`, `69725e1`, `4692454`, `179b76e`, merge `f38481f` and `e66a9ee`. An earlier statement to the user said 5 | `gh pr view 3 --json commits` |
| Number of phases | The architecture map has eight (0 to 7), not nine. The first build session's "phase 1" covered map phases 0 and 2 | `discovery/rust-architecture-map.md`, `docs/rust-rewrite/phase1.md` on the phase 1 branch |
| Number of discovery documents | 17 markdown files, not 15, plus `golden/` | `ls docs/rust-rewrite/discovery` |
| Which Rust architecture the host runs | Shell is arm64 but Homebrew under `/usr/local` and the installed Rust toolchain are x86_64, so both the C++ oracle and Rust test builds run under Rosetta on this machine | Part 5 |

Further conflicts between the discovery documents themselves are listed at the end of Part 7.

