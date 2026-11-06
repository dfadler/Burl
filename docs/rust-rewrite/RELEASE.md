# Release checklist

Implements the parity-release exit criterion ([#14](https://github.com/dfadler/Burl/issues/14) item 5)
and the release design ([#21](https://github.com/dfadler/Burl/issues/21)). The first release is
**0.58.0**, continuing Gource's numbering; the workspace is 0.58.0 from the release PR on.

## Exit criteria, each as a command

| Criterion (#14) | Check | Where it runs |
|---|---|---|
| Every gate green on Linux amd64 | `cargo xtask ci` (fmt, clippy, tests, sim comparator) and the heavy tier: goldens and sim goldens reproduce, slow tests, headless render smoke | `ci.yml`, `heavy.yml`; `release.yml` re-runs the heavy tier and `cargo xtask ci` on the tag |
| Scene visual gate (SSIM, #12) | `scripts/visual-burl.sh` (`cargo xtask ssim --min 0.959` per frame), refs in `tests/golden/visual/` | `heavy.yml` `visual-ssim`, blocking (per-frame numbers in its step summary) |
| macOS native build, sim goldens within tolerance | the `macos` leg of `ci.yml`; `cargo xtask ci` on macOS | `ci.yml`, and `build-macos-arm64` in `release.yml` |
| `--features pcre2` leg | `cargo xtask ci --pcre2` | `ci.yml` (`linux-pcre2`) |
| Docker headless export gives a valid PPM stream for a fixture | `docker build --platform linux/amd64 -f docker/release.Dockerfile -t burl:release . && scripts/check-image.sh burl:release` | `image` job in `release.yml` |
| Interactive checklist: automated items proved, manual items signed off by the user | `scripts/check-release.sh vX.Y.Z` (every proof the checklist names still exists, every manual box ticked, a sign-off per platform, version matches); the proofs themselves are the unit tests and the `window-xvfb` and `visual-ssim` jobs | `gate` job in `release.yml`; `heavy.yml` |
| Deviation register complete and reviewed | the same script: the sign-off records `git hash-object docs/rust-rewrite/DEVIATIONS.md`; any later edit to the register makes the gate fail | `gate` job |
| Licences | `cargo deny check` (every PR); `cargo deny list` is shipped as `THIRD-PARTY-LICENSES.txt` | `ci.yml` `deny`; `build` jobs |
| Reproducible dependencies | all builds use `--locked` | everywhere |

`release.yml` runs on a tag `vX.Y.Z` (or `vX.Y.Z-rc.N`) only. The `publish` job (ghcr push and a
**draft** GitHub release) needs every job above green and then the `release` environment's
required reviewer. Nothing is public until the user also publishes the draft.

## Human steps, in order

1. All feature and gate PRs merged; `master` green (fast, heavy, visual).
2. Run the **manual** items of the interactive checklist ([INTERACTIVE-CHECKLIST.md](INTERACTIVE-CHECKLIST.md), "Still manual") on macOS Retina, macOS non-Retina and Linux X11. Everything else in it is proved by CI. Decide the "Known gap" list there (implement, or record each as a deviation) before step 3.
3. Review every row of `DEVIATIONS.md`. Every Change/Gap has the user's OK; no unpinned or "not yet written" test names; decide G4 (does `pcre2` ship by default).
4. Release PR (merged by the user; it touches no workflow): bump the workspace version to 0.58.0, change `VERSION` in `crates/burl-conf/src/help.rs` and the `help-*` expected-diff row, fill the sign-off block (`version`, `deviation-register` = the hash command above, one line per platform), tick the boxes.
5. Optional dry run: tag `v0.58.0-rc.1`. It skips the sign-off check, builds everything, pushes the image without `latest` and drafts a prerelease. Delete the draft and image tag afterwards.
6. Tag `v0.58.0` on the merge commit on `master` (the gate rejects a tag not reachable from `master`). Approve the `release` environment when the jobs are green. Publish the draft release.

## One-time repository setup (user)

- Settings, Environments: create `release`, add yourself as **required reviewer**, restrict to tags `v*`. Without reviewers the environment gate is not enforced (the sign-off script still is).
- Settings, Actions: workflow permissions can stay read-only; `publish` requests `contents: write` and `packages: write` itself.
- After the first push, make the ghcr package public if it should be pullable without login.

## Artifacts per release

- `burl-vX.Y.Z-linux-amd64.tar.gz`, `burl-vX.Y.Z-macos-arm64.tar.gz`, each with a `.sha256`: `burl`, `COPYING`, `NOTICE.md` (Gource attribution), `THIRD-PARTY-LICENSES.txt`.
- `ghcr.io/dfadler/burl:X.Y.Z` (and `latest` for non-rc): headless, no ffmpeg; pipe the PPM stream to ffmpeg on the host.
- No crates.io publish (`burl` is taken there).
