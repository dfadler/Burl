# Visual-equivalence reference frames (ticket #12)

`FIXTURE-N.png` is frame `N` of the untouched oracle (Gource 0.57, `src/`) rendering `FIXTURE`, 640x360, 25 fps,
cut from the `-o -` PPM stream. Four fixtures x four frames: `small-repo`, `file-removal`, `file-to-dir`
(the simulation-gate logs in `tests/golden/sim/`, 10 s) and `real-burl-git-250` (`tests/golden/real/`, 30 s).
This is the **structural set**: `--hide mouse,date,dirnames,filenames,usernames,progress,bloom`, so no text, no
overlays, no bloom. `PROVENANCE.txt` records the image digest, source sha, flags and frame indices.

- One pinned platform: the amd64 Debian trixie-slim image, Xvfb + Mesa llvmpipe, `LP_NUM_THREADS=1`, one warm-up
  render discarded first (llvmpipe output varies with CPU SIMD features and is unstable on a cold start, #16).
- Regenerate only in a `golden-refresh` PR: `scripts/visual.sh refs`.
- `bloom-FIXTURE-N.png` is the bloom set (bloom drawn, same fixtures and frames; `PROVENANCE-bloom.txt`): `scripts/visual.sh refs bloom`.
- Compare with `cargo xtask ssim REF.png ACTUAL.png [--min X]`. Thresholds, the measurement procedure and what
  blocks are in `docs/rust-rewrite/visual-gate.md`.
