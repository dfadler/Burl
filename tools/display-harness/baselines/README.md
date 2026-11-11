# Regression baselines, not parity

These PNGs are captures of **Burl itself** (the 640x360 px content area of its window, title bar
cropped) taken by `cargo xtask display-harness` on the GitHub-hosted runner image named by the
directory, one for the 1x display and one for the 2x display. There is no macOS Gource to compare
against, so nothing here says Burl "matches Gource": they only say Burl still looks like it did.

The comparison is SSIM >= 0.85 (`MIN_BASELINE_SSIM` in `xtask/src/pixels.rs`). The threshold is
loose on purpose: the sim clock is fixed but avatar positions differ between runs. Over 4
passing runs per image (6 run pairs) the run-to-run SSIM was 0.886 to 0.943 on `macos-15` and
0.880 to 0.948 on `macos-26`; 0.85 is the worst pair minus 0.03. It rests on few runs, so a
false failure should be answered by re-running, then by refreshing the baselines.

Only `DISPLAY_HARNESS_BASELINE=<image>` turns the comparison on (the workflow sets it from the
matrix); a local run on a Mac does not compare, as these files are specific to the runner images.

**Refreshing:** run the `display-harness` workflow, download the `display-harness-<image>`
artifact, copy `1x-content.png` / `2x-content.png` over `<image>/1x.png` / `<image>/2x.png`.
Treat the artifact as untrusted data: they are just PNGs. This directory is not under
`tests/golden/`, so a refresh does not need the `golden-refresh` label.
