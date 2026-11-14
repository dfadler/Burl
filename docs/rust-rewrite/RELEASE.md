# Release checklist

Implements the parity-release exit criterion ([#14](https://github.com/dfadler/Gnarl/issues/14) item 5)
and the release design ([#21](https://github.com/dfadler/Gnarl/issues/21)). The first release is
**0.58.0**, continuing Gource's numbering; the workspace stays at 0.57.0 until the release PR.

## Exit criteria, each as a command

| Criterion (#14) | Check | Where it runs |
|---|---|---|
| Every gate green on Linux amd64 | `cargo xtask ci` (fmt, clippy, tests, sim comparator) and the heavy tier: goldens and sim goldens reproduce, slow tests, headless render smoke | `ci.yml`, `heavy.yml`; `release.yml` re-runs the heavy tier and `cargo xtask ci` on the tag |
| Scene visual gate (SSIM, #12) | `scripts/visual-gnarl.sh` (`cargo xtask ssim --min 0.959` per frame), refs in `tests/golden/visual/` | `heavy.yml` `visual-ssim`, blocking (per-frame numbers in its step summary) |
| macOS native build, sim goldens within tolerance | the `macos` leg of `ci.yml`; `cargo xtask ci` on macOS | `ci.yml`, and `build-macos-arm64` in `release.yml` |
| `--features pcre2` leg | `cargo xtask ci --pcre2` | `ci.yml` (`linux-pcre2`) |
| Docker headless export gives a valid PPM stream for a fixture | `docker build --platform linux/amd64 -f docker/release.Dockerfile -t gnarl:release . && scripts/check-image.sh gnarl:release` | `image` job in `release.yml` |
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
4. Release PR (merged by the user; it touches no workflow): bump the workspace version to 0.58.0 **and the six `version = "=0.57.0"` requirements in the root `[workspace.dependencies]` to match** (a mismatch fails every `--locked` build, so CI catches a miss), change `VERSION` in `crates/gnarl-conf/src/help.rs` and the `help-*` expected-diff row, fill the sign-off block (`version`, `deviation-register` = the hash command above, one line per platform), tick the boxes.
5. Optional dry run: tag `v0.58.0-rc.1`. It skips the sign-off check, builds everything, pushes the image without `latest` and drafts a prerelease. Delete the draft and image tag afterwards.
6. **Re-check the crate names right before tagging**: `for c in gnarl gnarl-conf gnarl-formats gnarl-gl gnarl-render gnarl-sim gnarl-util; do printf '%s ' $c; curl -s -o /dev/null -w '%{http_code}\n' -A 'gnarl-release (github.com/dfadler/Gnarl)' https://crates.io/api/v1/crates/$c; done` must print 404 for all seven (all were 404 on 2026-11-14). A name is reserved only once published; a publish is permanent.
7. Tag `v0.58.0` on the merge commit on `master` (the gate rejects a tag not reachable from `master`). Approve the `release` environment when the jobs are green (the `publish` job pushes the image and drafts the release). Publish the draft release.
8. Approve the `release` environment a second time for the `crates` job. It publishes the seven crates to crates.io in dependency order (final tags only; an rc never publishes). If it fails part-way, fix the cause and re-run the failed job: crates already on crates.io are skipped. Afterwards check each crate's page and make the ghcr package public if it should be pullable.

## One-time repository setup (user)

`scripts/repo-setup.sh` (#180, #95) does the branch ruleset and the `release` environment below. The owner runs it with their own admin token: `check` (the default) only reads and prints the payloads it would send; `apply --confirm-apply` writes after a typed confirmation, then re-reads the settings. Defaults require only the fast-tier checks; `--require-heavy`, `--require-review` and `--admin-bypass` are opt-in decisions. GitHub has no dry-run, so the payloads are not proven accepted until the first `apply`. Making the ghcr package public after the first publish is believed to be UI-only; this is unverified.

- Settings, Environments: create `release`, add yourself as **required reviewer**, restrict to tags `v*`. Without reviewers the environment gate is not enforced (the sign-off script still is).
- Settings, Actions: workflow permissions can stay read-only; `publish` requests `contents: write` and `packages: write` itself.
- After the first push, make the ghcr package public if it should be pullable without login.
- crates.io: sign in to crates.io with the GitHub account that should own the crates and create an API token (Account Settings, API Tokens) with the `publish-new` and `publish-update` scopes. Add it as the secret **`CARGO_REGISTRY_TOKEN`** of the **`release`** environment (not a repository secret), so only the approved `crates` job can read it. A first publish cannot use trusted publishing: its docs list "Your crate must already be published to crates.io (initial publish requires an API token)" as a prerequisite (<https://crates.io/docs/trusted-publishing>). The scope names come from the crates.io source (`crates/crates_io_database/src/models/token/scopes.rs` in rust-lang/crates.io); there is no prose doc page for them that I could fetch. Set an expiry and delete the token after the release.
- Optional, after 0.58.0 is out: for each of the seven crates, crates.io Settings, Trusted Publishing, add GitHub repository `dfadler/Gnarl`, workflow `release.yml`, environment `release`; then replace the token in the `crates` job with `rust-lang/crates-io-auth-action` (pinned by SHA) plus `permissions: id-token: write`, and delete the secret. Tokens from trusted publishing last 30 minutes (same docs).

## Artifacts per release

- `gnarl-vX.Y.Z-linux-amd64.tar.gz`, `gnarl-vX.Y.Z-macos-arm64.tar.gz`, each with a `.sha256`: `gnarl`, `COPYING`, `NOTICE.md` (Gource attribution), `THIRD-PARTY-LICENSES.txt`.
- `ghcr.io/dfadler/gnarl:X.Y.Z` (the path comes from the repository name) (and `latest` for non-rc): headless, no ffmpeg; pipe the PPM stream to ffmpeg on the host.
- crates.io: `gnarl`, `gnarl-conf`, `gnarl-formats`, `gnarl-gl`, `gnarl-render`, `gnarl-sim`, `gnarl-util`, all at the same version (`=` requirements between them), published in that dependency order: util, formats, conf, sim, render, gl, gnarl. `cargo install gnarl --locked` installs the binary. The default build does not include `pcre2` ([G4-PCRE2-DECISION.md](G4-PCRE2-DECISION.md)).

## Packaging checks

- `cargo publish --workspace --exclude xtask --dry-run --locked` packages every crate and builds it from the package against its siblings. The `package` job in `ci.yml` runs it on every PR and the `crates` job runs it before publishing. Cargo's docs list `--workspace`, `--exclude` and `--dry-run` for `cargo publish`; the sibling-overlay behaviour (a dependent verifies against the packaged siblings, not the registry) was observed with cargo 1.99 and is not described on the doc page (<https://doc.rust-lang.org/cargo/commands/cargo-publish.html>).
- `cargo package --list -p <crate> --locked` shows what a crate ships. The crates.io limit is 10 MB per `.crate` file (<https://doc.rust-lang.org/cargo/reference/publishing.html>, "Packaging a crate"); the largest is `gnarl-render` at about 0.5 MB because it embeds the font.
- Each crate lists its files with `include`. Tests are not packaged: they read goldens from the repo root. Anything a crate embeds with `include_bytes!`/`include_str!` must live inside the crate (`crates/gnarl-render/assets/` holds copies of `data/`; a test pins them to the originals).
- Each crate carries a copy of `COPYING` as `LICENSE` (pinned by a test).
