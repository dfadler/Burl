# Release readiness: 0.58.0

Snapshot of `master` at `ceb39a8` (2026-11-04) against the parity-release exit criteria
([#14](https://github.com/dfadler/Burl/issues/14) resolution item 5) and the table in
[RELEASE.md](RELEASE.md). Nothing was tagged, published or changed in the repository settings.

**Verdict: not ready.** Two exit criteria are not met on `master` today (the scene visual gate, until #115 and #120 merge,
and the unpinned register rows, until #117 and #118 merge), and the user-only steps (interactive checklist, deviation review,
repository setup, tag) have not started. Status values: **met** (a check ran and passed),
**not met**, **needs-user** (only the user can do it).

**Update 2026-11-04 (after the first draft of this report).** Facts that changed since the snapshot; the tables below
keep the `ceb39a8` wording and say where it is superseded.

- **Visual gate cause found: a procedure bug, not a Burl rendering gap.** `scripts/visual-burl.sh` omitted `--prng glibc`
  (unlike `scripts/visual.sh burl`). Fixed in draft [#115](https://github.com/dfadler/Burl/pull/115); its head run
  scored 16 of 16 frames on the real x86 runner, minimum 0.9789 against the 0.959 threshold (heavy run
  [41877743639](https://github.com/dfadler/Burl/actions/runs/41877743639)). Thresholds and references are untouched.
  [#120](https://github.com/dfadler/Burl/pull/120) (stacked on #115) makes the gate blocking (`shell: bash`, no
  `continue-on-error`). Both touch workflows or the gate and are draft; the user merges #115 first, then retargets #120.
- **Register rows pinned or documented in open PRs:** [#117](https://github.com/dfadler/Burl/pull/117) (D3 ADR, per-PR D9
  test, G1 test, G2 characterisation) and [#118](https://github.com/dfadler/Burl/pull/118) (G9 row, G4 decision note,
  `OPTION-COVERAGE.md` refresh). Per #10 the user's merge is the OK for the changed rows.
- **Selection (#92) is a 5-PR stack,** [#119](https://github.com/dfadler/Burl/pull/119) to
  [#124](https://github.com/dfadler/Burl/pull/124): oracle fork scripted pointer, selected look and Tab, sim picking and
  click, windowed pointer, hover box. The scope paper is [#105](https://github.com/dfadler/Burl/pull/105). Merge in order.
- **CI health:** [#116](https://github.com/dfadler/Burl/pull/116) adds heavy runs on `master` push and `merge_group`, and
  documents that `macos` CANCELLED/queued results are GitHub runner capacity, not our config.
- The release PR ([#114](https://github.com/dfadler/Burl/pull/114)) adds register row D12 (version line), so the
  selection paper's new rows are numbered D13 and D14.

## Exit criteria (#14 item 5)

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | Every gate green on Linux amd64 | **met**, with a caveat | `cargo xtask ci` run locally on this checkout (arm64 macOS): exit 0, 259 tests passed, 0 failed, 1 ignored (sim gate included). `ci` on `master` (`ceb39a8`): `linux`, `linux-pcre2`, `macos`, `deny` success ([run](https://github.com/dfadler/Burl/actions/runs/41849294439)). Heavy tier (`goldens-reproduce`, `sim-goldens-reproduce`, `slow-tests`, `render-smoke`, `visual-ssim`) last ran green on the PR that became `master`'s head ([run](https://github.com/dfadler/Burl/actions/runs/41842285254)). Caveat: `heavy.yml` runs on PRs touching its paths, weekly and on tags, **not on pushes to `master`**, and no weekly run exists yet, so no heavy run covers `master` itself. `visual-ssim` is "green" only because of the problem in the next row. |
| 2 | Scene visual gate (SSIM, #12) | **not met on `master`; cause fixed in draft #115, blocking in draft #120** (see the update above: 16/16 frames, min 0.9789, on x86 once `--prng glibc` is passed). Snapshot finding: | `visual-ssim` is wired in `heavy.yml` and `scripts/visual-burl.sh` exits 1 below 0.959, but in the last two PR runs 8 of 16 frames are below 0.959 on the real x86 runner: small-repo 0.907-0.947, real-burl-git-250 0.838-0.918 (file-removal and file-to-dir 0.984-0.994). [run 41842285254](https://github.com/dfadler/Burl/actions/runs/41842285254), [run 41839132242](https://github.com/dfadler/Burl/actions/runs/41839132242). `visual-gate.md` measured a minimum of 0.9856 on emulated amd64 (Apple-silicon Docker), so the cross-CPU spread (R22, #96) is real and large, or Burl differs from the oracle on x86. The cross-CPU reading below was wrong: the cause was the missing `--prng glibc`. Two things hide a failure: the job is `continue-on-error: true`, and the step runs `bash scripts/visual-burl.sh ... \| tee` with no `pipefail`, so the step reports success even when frames are BELOW. Making the gate blocking needs `shell: bash` (pipefail) in `heavy.yml` as well as dropping `continue-on-error`; both are workflow edits for the user to merge. |
| 3 | macOS native build, sim goldens within tolerance | **met** for `ci.yml`; release build **not exercised** | `macos` leg of `ci` green on `master` (run above) and `cargo xtask ci` green locally on macOS arm64. `build-macos-arm64` in `release.yml` has never run (no `release` workflow runs exist), so the tarball and checksum steps are untested until the rc dry run. |
| 4 | Docker headless export gives a valid PPM stream | **met** | Built `docker/release.Dockerfile` for `linux/amd64` on this checkout, then `scripts/check-image.sh`: `PPM OK: 867 frames of 320x240`, exit 0. It runs at release time only (`image` job), not per PR (register row D9). |
| 5 | Interactive checklist signed off by the user | **needs-user** | [INTERACTIVE-CHECKLIST.md](INTERACTIVE-CHECKLIST.md): 18 boxes unticked, three platform lines `TODO` ([#94](https://github.com/dfadler/Burl/issues/94)). `scripts/check-release.sh v0.58.0` on `master` fails with exactly those items plus the version mismatch (below). |
| 6 | Deviation register complete | **not met** (needs the user's review too) | See "Register rows" below: seven rows have no owning test or are marked open, the D3 ADR does not exist, and G4 (does `pcre2` ship by default) is undecided. The sign-off hash is for the user to record after the review. |

## RELEASE.md table, row by row

| Row | Command / check | Result |
|---|---|---|
| Every gate green on Linux amd64 | `cargo xtask ci` | Local: pass (see above). CI: pass on `master`. |
| Scene visual gate | `scripts/visual-burl.sh` / `heavy.yml` `visual-ssim` | **Fails** on the CI runner, masked (above). Not run locally: the Apple-silicon Docker numbers in `visual-gate.md` are the emulated ones. |
| macOS native build | `ci` `macos` leg | Pass. |
| `--features pcre2` leg | `cargo xtask ci --pcre2` | CI `linux-pcre2` pass on `master`. Not run locally. |
| Docker headless export | `docker build ... && scripts/check-image.sh burl:release` | Pass (above). |
| Interactive checklist signed off | `scripts/check-release.sh vX.Y.Z` | Script behaves. On `master`: `v0.58.0` fails 6 checks (tag != workspace version 0.57.0, 18 unticked boxes, 3 missing platform sign-offs, sign-off version `TODO`, register hash `TODO`). With a scratch copy of the checklist (all boxes ticked, version `0.57.0`, the real `git hash-object` of `DEVIATIONS.md`, three platform lines filled) `v0.57.0` and `v0.57.0-rc.1` pass; pointing the register at a different file fails with the hash message; `v0.58.0` against that copy fails on the version. Note `release.yml` skips this script for tags containing `-` (rc), by design. |
| Deviation register reviewed | same script, register hash | Mechanism works (above). The review itself is a user task. Current blob `67d95f8c779b1c51efd8566187a118a2cea169b1`; it changes whenever any session edits `DEVIATIONS.md`, so the user must take the hash last. |
| Licences | `cargo deny check` | Not run locally (`cargo-deny` is not installed here). CI `deny` job pass on `master`. |
| Reproducible dependencies | `--locked` everywhere | Met: `grep` over `.github/workflows`, `scripts`, `docker` and `xtask/src` finds no `cargo build/test/run/clippy/check` without `--locked`. |

## Register rows with no pin, or open

From [DEVIATIONS.md](DEVIATIONS.md) at the snapshot, with the open PRs that address them. RELEASE.md step 3 requires none of these at release time.

| Row | State |
|---|---|
| D3 | Pinning tests exist. The ADR is owed and `docs/adr/` does not exist on `master`. **[#117](https://github.com/dfadler/Burl/pull/117) adds `docs/adr/0001-prng-modes.md`.** |
| D9 | Check runs at release time only. **#117 adds the per-PR test `deviation_d9_export_from_an_empty_working_directory`.** |
| G1 | No owning test. **#117 adds `deviation_g1_closed_pipe_ends_the_export_with_an_error`.** |
| G2 | No owning test (not characterised). **#117 characterises it against compiled tinyxml (four differences) and pins them; it stays a Gap.** |
| G4 | Pinned by tests, but the decision "does `pcre2` ship by default" is open (RELEASE.md step 3). **[#118](https://github.com/dfadler/Burl/pull/118) adds `G4-PCRE2-DECISION.md` with facts and a recommendation (ship without for 0.58.0); it decides nothing, the user does.** |
| G7 | No owning test; needs a moving-camera visual fixture. |
| G8 | No owning test (proposed downgrade to a note). |
| G9 | No owning test, and the row text is stale: it says `export.rs` never loads avatars, but `fa38c17` ("avatars: ... in export") landed. The windowed half is [#91](https://github.com/dfadler/Burl/issues/91). **#118 rewrites the row and names the owning export tests.** |
| G6 | Proposed closed; closes when the user merges the row. |

## What remains unported

[OPTION-COVERAGE.md](OPTION-COVERAGE.md) was written at `33ead47`, 65 commits ago, so most of its "Real
gaps" table is out of date. Re-checked against the current tree (`rg` for the option names and the `not ported`
markers; the tree has no `not ported` / `unimplemented!` markers left):

| Gap (coverage-report number) | State on `master` |
|---|---|
| 1 windowed text and overlays, multi-section | Landed (`3c368c0`, `e6738ff`). Open follow-ups: a multi-section run stalls on a stdin section ([#99](https://github.com/dfadler/Burl/issues/99)); FrameKit and SceneSettings keep the initial size after a resize ([#98](https://github.com/dfadler/Burl/issues/98)). |
| 2, 3 start-position (random and N), `--loop`, `--follow-user`, camera mode track | Landed (`7b6e5e4`, `e1cc912`, `23e556b`). |
| 4 `--hide-root`, 5 `--fixed-user-size`, 6 `--font-file`, 8 `--transparent` | Landed. |
| 7 avatars | Export landed; windowed, NFC keys and the G8 mip check open ([#91](https://github.com/dfadler/Burl/issues/91)). |
| 9 `--selection-colour` and pointer selection | **Landed** (option B2 from [#105](https://github.com/dfadler/Burl/pull/105); stack #119-#124, [#92](https://github.com/dfadler/Burl/issues/92)). `--selection-colour` is wired: it reaches the selected-name pass (`crates/burl/src/frame.rs`, `crates/burl-render/src/labels.rs`), and the sim picks, selects and hovers (`crates/burl-sim/src/pointer.rs`). Tests: `sim_gate_small_repo_select`, `selected_names_are_drawn_last_in_the_selection_colour_whatever_the_fade_or_hide`. Not yet checked against a real display or any Gource run on macOS (no oracle there); the checklist rows in `INTERACTIVE-CHECKLIST.md` stay manual. |
| 10 `--multi-sampling`, `--screen` | Landed (`6f160b5`). |
| 11 `--log-level` | Landed (`6e4f4fb`). |
| 12 position slider | **Open** ([#93](https://github.com/dfadler/Burl/issues/93)). |
| 13 JPEG and GIF images | Landed (`4d189f8`). |
| 14 test gaps | Landed (`f6ad168`) apart from the items in the visual gate. |

The coverage report itself needs a refresh before the release so the shipped docs do not
describe landed work as missing; [#118](https://github.com/dfadler/Burl/pull/118) does that (to `ceb39a8`). Other open issues that gate the release: [#89](https://github.com/dfadler/Burl/issues/89) and
[#90](https://github.com/dfadler/Burl/issues/90) (golden refreshes, `ready-for-human`),
[#94](https://github.com/dfadler/Burl/issues/94) (interactive checklist),
[#95](https://github.com/dfadler/Burl/issues/95) (required checks and the `release` environment),
[#96](https://github.com/dfadler/Burl/issues/96) (cross-CPU SSIM, which row 2 above makes urgent),
[#97](https://github.com/dfadler/Burl/issues/97) (retire HANDOFF.md and plan/ after the map closes); #50-#54 are
older Phase 5 tickets, some of which the landed work above covers (not triaged here).

## Blocking 0.58.0 today

**Code and CI work for agents**

1. Visual gate: diagnosed (missing `--prng glibc`) and fixed in draft #115; blocking in draft #120. Remaining: the user merges both (workflow/gate edits), then optionally adds `visual-ssim` to the required checks. The cross-CPU question in [#96](https://github.com/dfadler/Burl/issues/96) is narrower now.
2. Register rows: D3, D9, G1, G2 in #117 and G9, G4 note in #118 (both open, user merges). Still open: tests or closure for G7 and G8.
3. `OPTION-COVERAGE.md` refresh: in #118.
4. Decide a trigger for `heavy.yml` on `master` pushes, or accept PR-only plus the tag run.
5. Windowed follow-ups #91, #93, #98, #99 (each pending its own triage or PR), and selection #92 (stack #119-#124).

**User tasks**

1. Run the interactive checklist on macOS Retina, macOS non-Retina and Linux X11 ([#94](https://github.com/dfadler/Burl/issues/94)).
2. Review every `DEVIATIONS.md` row, decide G4, approve the new D12 version row in the release PR (and the D13/D14 rows from the selection scope paper, #105), and take the register hash last.
3. Repository setup: required checks and the `release` environment ([#95](https://github.com/dfadler/Burl/issues/95)).
4. Merge the golden-refresh PRs ([#89](https://github.com/dfadler/Burl/issues/89), [#90](https://github.com/dfadler/Burl/issues/90)) and the workflow edits.
5. Fill the sign-off block in the release PR, optionally tag `v0.58.0-rc.1` to dry-run `release.yml` (it has never run), merge the release PR, tag `v0.58.0`, approve the `release` environment, publish the draft.
