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

## Measured: Burl vs oracle (2026-11-03, after the cadence and PRNG fixes)

The earlier numbers were invalid for two reasons, both fixed on the Burl side. (1) Burl wrote a frame every tick;
Gource dumps every `frameskip + 1`-th (3rd at `-r 25`), so Burl emitted ~3x the frames. (2) Burl's default PRNG is
clock-seeded, so frame counts varied 355-387 between identical runs; the oracle never calls `srand`, so its `rand()`
is glibc `srand(1)`. `scripts/visual.sh burl` therefore runs Burl with `--prng glibc` (same base digest, amd64; the separate `burl-visual:amd64` image, not the oracle's `burl-golden:amd64`).

Structural set (bloom hidden), 16 frames, SSIM vs the committed references: 11 frames 0.9995-0.99997; the lowest are
real-burl-git-250 frames 100/300/500/800 = 0.9856/0.9968/0.9899/0.9996 and file-removal frame 30 = 0.9969 (the
oracle's own jitter value 0.9969 above). Minimum 0.9856 against threshold 0.959.
Bloom set (bloom drawn, same fixtures; committed as `tests/golden/visual/bloom-*.png`, **16 of 16 frames**, regenerated
2026-11-04 in the pinned amd64 image by `scripts/visual.sh refs bloom`; provenance in `PROVENANCE-bloom.txt`).
`scripts/visual.sh burl bloom` against them: small-repo 0.9991/0.9979/0.9981/0.9991; file-removal 0.99981/0.99975/0.99991/0.99996;
file-to-dir 0.99997/0.99982/0.99977/0.99979; real-burl-git-250 (frames 100/300/500/800) 0.9797/0.9923/0.9804/0.9944.
Minimum 0.9797 against the structural threshold 0.959 (no threshold changed; the bloom check only reports). The
real-repo shortfall from 1.0 is the oracle's one-frame jitter mode (its frame counts varied 351-360 / 1073 across the
reference renders), as in the structural set. **These 16 PNGs are report-only reference candidates, not gate inputs:**
they were rendered under QEMU amd64 emulation on Apple silicon, not native x86, and nothing in CI compares against them
yet. They become usable once regenerated on native amd64 (or compared on the runner) and the two agree. Mean
luminance, Burl vs oracle, small-repo frames 30/70, is from the earlier 2026-11-03 run and was **not re-measured against
these references**: 29.648 vs 29.702, 33.153 vs 33.106 (0.2%, inside the 5%).

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
