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
That check has now run: see the next section.

## Measured: native x86 runner (2026-11-12, `visual-spread` run 43134255630)

Manual dispatch of `visual-spread.yml` on master f9ed949 (`fixture=all`, `runs=10`), GitHub-hosted `ubuntu-24.04`, 21 minutes
(the `real-burl-git-250` leg took 20 of them). Read from the run's artifacts (`self-*/self.txt`) and the regenerated bloom frames.

- **Self-SSIM, 160 values: worst 0.979105 (small-repo frame 30), above the 0.959 gate.** The same worst value as the emulated
  measurement above, so the proposed threshold is unchanged (0.979105 - 0.02 = 0.959105).
- **No run-to-run spread:** each of the 16 frames has one value across all 10 runs.
- **Which mode each fixture lands in differs from the emulated set.** The deviating values are the same discrete ones as above
  (small-repo 0.979105 / 0.986697 / 0.997216 / 0.999780), but here small-repo and file-removal (worst 0.996925) are in that mode
  on every run (40 of 40 values below 1.0 each), where the emulated runs were mostly bit-identical for small-repo (4 of 40) and
  mixed for file-removal (24 of 40). file-to-dir (worst 1.000000 here, 0.996941 emulated) and real-burl-git-250 (1.000000) match the
  references exactly on every run. This is consistent with the oracle's asynchronous-load timing mode described above; why this
  runner lands in it more often was not investigated.
- **Bloom references:** regenerating them on this runner (`scripts/visual.sh refs bloom`, fresh oracle build, source f9ed949)
  gave 16 of 16 frames **byte-identical** to the committed ones, which were rendered under QEMU amd64: SSIM 1.000000 and the same
  mean luma to three decimals. The comparison was checked not to be trivial (the workflow copies the committed set aside before
  regenerating; the run's provenance file shows the new build).

Not established: the CPU model of the runner (the workflow wrote `lscpu` only to the step summary, which the API does not
expose; read it on the run page, or from the next dispatch once the artifacts carry it), a second x86 microarchitecture, and
the decisions on keeping `visual-ssim` blocking and on making it a required check (#95). R22 stays open for those.

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
they were rendered under QEMU amd64 emulation on Apple silicon, not native x86. Since #176 the `visual-ssim` job in
`heavy.yml` renders Burl on the x86 runner and reports SSIM and mean luminance against them in the step summary (report
only, never fails the job). They become usable as gate inputs once regenerated on native amd64 and the two agree.
Mean luminance, oracle vs Burl (oracle first in each pair), 16 frames, from `heavy.yml` run 42942335614 (master 714da55, x86 runner): every frame
within 0.01-0.70% (limit 5%). small-repo frames 30/70/110/170 = 30.006 vs 30.025, 32.809 vs 32.832, 32.443 vs 32.463,
31.278 vs 31.293 (0.05-0.07%); file-removal and file-to-dir 0.01-0.02%; real-burl-git-250 frames 100/300/500/800 =
36.787 vs 37.045, 35.587 vs 35.678, 44.142 vs 44.337, 39.562 vs 39.716 (0.70/0.26/0.44/0.39%; the oracle's one-frame jitter
is a possible cause of the larger ones, not established for these frames). SSIM on the same run, min 0.9797 (real-burl-git-250 frame 100), agrees with the 2026-11-04
emulated-Burl figures above on all 16 frames (largest difference 0.0006, small-repo frame 170). The earlier 2026-11-03
numbers (29.648 vs 29.702, 33.153 vs 33.106) are superseded.

## Moving camera: no Burl gap, the cold first render is the outlier (2026-11-10, #172)

The #172 draft fixture (`camera-cull`: 194-line custom log, `--camera-mode track`, structural flags, 640x360 at `-r 25`, 559 frames)
reported Burl-vs-oracle SSIM 0.72/0.68/0.76/0.87 at frames 135/160/235/310. Re-measured in `burl-golden:amd64`, four oracle renders
in separate containers, Burl with `--prng glibc`: the two **warm** oracle renders (second render in a container) are bit-identical to each other
and to nothing else; Burl vs warm oracle over 111 frames (every 5th) has min **0.999876** (frames 135/160/235/310: 0.99990/0.99990/0.99991/0.99995).
The two **cold** renders (first in a container) differ from the warm one in all 559 frames, and score 0.66-0.94 against it, the same numbers
the draft reported. So the "gap" is the documented warm-up effect (#16, above): a cold oracle render is shifted in simulation time and a
moving camera turns that shift into a large SSIM loss, where static frames hide it. Camera interpolation, the frame step and the track target
agree with the oracle. A moving-camera fixture must render the oracle twice and keep the second (as `scripts/visual-in-container.sh stream` does).

## FMA (gcc on amd64)

Not measured. #16's probe emitted no `fmadd`, and the fork is the right subject. Needs a gcc build of the
fork with default flags on real x86; the oracle goldens use clang with `-ffp-contract=off`, so it only matters if gcc is ever the authority (excluded by M2).

## What blocks

| check | status |
|---|---|
| structural SSIM, threshold 0.959 | **blocking** in `heavy.yml` `visual-ssim` (real x86 run with `--prng glibc`: 16 frames, min 0.979, max 1.000). |
| text pixels | none (#12) |
| bloom luminance within 5% | non-blocking |
| full-render SSIM | reported only |

Proposed CI wiring (a workflow edit, merged by the user): add `tests/golden/visual/` to the `golden-label` regex and a
heavy-tier step running Burl in the headless image and `cargo xtask ssim --min 0.959` per frame. Not included here.
The refs sit under `tests/golden/`, so the existing label gate already covers them.
