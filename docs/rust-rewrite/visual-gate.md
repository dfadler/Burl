# Visual-equivalence gate: procedure and measurements

Ticket [#12](https://github.com/dfadler/Burl/issues/12). Tools: `cargo xtask ssim` (PR #56), reference frames in
`tests/golden/visual/` (PR #62), `scripts/visual.sh`. Measured 2026-11-03 on an Apple-silicon Mac (Docker
Desktop): amd64 containers are emulated, arm64 native. Image `debian@sha256:f6b08b09...3ec4`, Mesa 25.0.7, LLVM 19.1.7.
Oracle: untouched `src/` built with clang 19, `-O1 -fsigned-char -ffp-contract=off`.

## Procedure

1. Render each fixture with the oracle: 640x360, 25 fps, structural flags (text, overlays and bloom hidden),
   Xvfb + llvmpipe, `LP_NUM_THREADS=1`, one warm-up render discarded.
2. Cut four frames per fixture; SSIM (11x11 Gaussian, luma) against the committed reference.
3. Self-SSIM: repeat step 1-2 N >= 10 times (`scripts/visual.sh self 10`); threshold = worst value - 0.02 (`cargo xtask ssim-threshold`).
4. Burl side: render the same fixture headless in the same image, compare frame by frame. Failure triage: simulation dump first, then render. Never loosen a threshold to get green.

## Measured: oracle self-SSIM, amd64, 10 runs x 16 frames (160 values)

| fixture | frames < 1.0 | worst | note |
|---|---|---|---|
| small-repo | 4 of 40 (run 10) | 0.979105 | |
| file-removal | 24 of 40 (6 runs) | 0.996925 | |
| file-to-dir | 28 of 40 (7 runs) | 0.996941 | |
| real-burl-git-250 | 0 of 40 | 1.000000 | |

Worst 0.979105, so the proposed structural threshold is **0.959105 (~0.959)**. Most runs are bit-identical.
The deviating runs repeat the *same* SSIM values (e.g. small-repo 0.979105 / 0.986697 / 0.997216 / 0.999780), so they
are a discrete mode, not rasteriser noise: the oracle loads the log asynchronously and its frame count jitters
(358/359/360 for one fixture, #16), shifting the simulation by one frame. The fork and Burl block on the load (D2),
so this spread belongs to the oracle's reference, and a fixed-index comparison cannot be tighter than it.
Warm-up and `LP_NUM_THREADS=1` removed all other variance.

## Measured: cross-CPU (limited)

No x86 hardware was available. What was run: arm64-native oracle (same source and flags, same image digest,
128-bit llvmpipe vectors) vs the amd64 references, 3 runs x 16 frames: worst 0.979123; real-burl-git-250 worst
0.999974 (no sim jitter there). Apart from the same one-frame mode, arm64 and amd64 agree to ~3e-5 SSIM,
so the rasteriser contributes far less than the timing mode. Not measured: a real x86 CPU with a different
AVX level (the amd64 container is emulated) and a second x86 microarchitecture, so R22 stays open for real
x86 hosts: CI's ubuntu-24.04 runner is the first real check (run `scripts/visual.sh self 10` there; expect >= 0.959).

## Measured: Burl vs oracle (non-blocking, informational)

`burl ... -o` in the same image vs the references at the same frame indices (SSIM): file-removal 0.987-0.993,
file-to-dir 0.985-0.996, small-repo 0.921-0.956, real-burl-git-250 0.879-0.924. **Not a valid comparison yet**:
Burl emits ~3x the frames (1051-1080 for fixtures where the oracle emits ~355; 3220 vs 1073 for the real log), so frame
index N is not the same moment, and the one pair of frames I looked at differs visibly in layout. Dark, sparse
frames score high regardless. The gate needs Burl's export to match the oracle's frame cadence and stop time first
(proposed deviation or bug for the render owner; not for this PR to decide).

## FMA (gcc on amd64)

Not measured. #16's probe emitted no `fmadd`, and the fork is the right subject. Needs a gcc build of the
fork with default flags on real x86; the oracle goldens use clang with `-ffp-contract=off`, so it only matters if gcc is ever the authority (excluded by M2).

## What blocks

| check | status |
|---|---|
| structural SSIM, threshold 0.959 | **non-blocking until** (a) Burl's frame cadence matches the oracle, (b) one real x86 run confirms the spread. Then blocking. |
| text pixels | none (#12) |
| bloom luminance within 5% | non-blocking |
| full-render SSIM | reported only |

Proposed CI wiring (a workflow edit, merged by the user): add `tests/golden/visual/` to the `golden-label` regex and a
heavy-tier step running Burl in the headless image and `cargo xtask ssim --min 0.959` per frame. Not included here.
The refs sit under `tests/golden/`, so the existing label gate already covers them.
