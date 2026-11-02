# gource-cpp-dump: the State-dump fork

A copy of the Gource 0.57 C++ sources (`src/`, with the `Core` submodule vendored at
`src/core`, pinned upstream commit `d8d880fd`) plus **only** dump hooks. It is the reference
side of the Burl simulation parity gate (issues #11, #18, #22). It is not the Oracle: byte-exact
goldens (help, exit codes, custom logs) still come from the untouched `src/` at the repo root.

The fork keeps Gource's behaviour, including its bugs, except:

- value-initialised fields that were uninitialised (`Pawn::accel`, `RFile::dest`,
  `RFile::touch_colour`, `QuadItem::node_count`/`quadItemBounds`), so the dump is defined;
- the three `vec2(rand()%100-50, rand()%100-50)` sites evaluate x then y explicitly (clang's
  order; gcc on x86-64 swaps them, issue #16 M2);
- all five `rand()` sites go through `gource_rand()` (`src/dumpstate.cpp`).

New files: `src/dumpstate.{h,cpp}`, `Dockerfile`, `scripts/dump-run.sh`, `tests/dump/`.
Hooks: `Gource::update` (after `logic`, before `draw`), `GourceShell::getNext` (section
boundary), `dumpJson` members on `RDirNode`, `RFile`, `RUser`, `ZoomCamera`, `RCaption`.

## Hidden options

`CommandLine` section, never in help, never saved by `--save-config`:

| Option | Meaning |
|---|---|
| `--debug-dump-state FILE` | write the JSONL dump to FILE |
| `--debug-dump-every N` | dump when `frame % N == 0` (default 1); the first loaded frame and the last frame are always dumped |
| `--debug-dump-max-frames N` | stop the run after frame N-1 (per section); 0 = no cap |
| `--prng default\|glibc\|park` | `default`: libc `rand()` untouched (unseeded = seed 1). `glibc`: libc `rand()` after an explicit `srand(1)` (or the seed). `park`: built-in Park-Miller (macOS libc), seed 1 |
| `--prng-seed N` | seed for any non-script mode |
| `--rand-script FILE` | Script mode: one integer per line, cyclic. Cannot be combined with `--prng` |

`start-position=random` no longer calls `srand(time(0))` unless `--prng default` is unseeded.

## Dump format (schema 1)

JSON Lines, one run per file, fixed key order, f32 as `%.9g`. Record types:

- `{"schema":1,"type":"header",...}` once: `prng`, `prng_seed`, `every`, `max_frames`, `compiler`,
  `char_signed`, `fma_target`, `flt_eval_method`.
- `{"schema":1,"type":"section","section":N,"repo_count":R,"hash_seed","stop_at_time",
  "stop_position","start_position","path"}` when a section's Gource starts (values after the
  per-section reset). The frame counter restarts here.
- `{"schema":1,"type":"frame","section","repo_count","frame","g":{...},"camera":{...},
  "dirs":[...],"files":[...],"users":[...],"captions":{"queued":[...],"active":[...]}}`
  as in `docs/rust-rewrite/discovery/parity-test-spec.md` section 2.2, with these fixes:
  `first_rand_frame` is the `framecount` of the first `gource_rand()` call (`-1` before; calls
  during option parsing count as frame 0), and non-finite floats are the strings `"nan"`/`"inf"`.
  Captions: `{"text","timestamp","elapsed","alpha"}`. Frames before the log has loaded are not dumped.

Arrays are sorted by key (`std::map` order): dirs by `abspath`, files by `fullpath`, users by
name; actions are queued first, then active.

## Build and run

```bash
docker build --platform linux/amd64 -t burl-gource-dump reference/gource-cpp-dump
docker run --rm --platform linux/amd64 -v "$PWD/out:/out" burl-gource-dump \
  /src/scripts/dump-run.sh /src/tests/logs/file-removal.log /out/run.jsonl
```

The image pins `debian:trixie-slim` by digest and builds with clang, `-fsigned-char`,
`-ffp-contract=off`, `-O1`. It runs under Xvfb with llvmpipe (`LP_NUM_THREADS=1`);
`dump-run.sh` discards one warm-up run first (the first llvmpipe run in a fresh container
differs, issue #16). Do not run `gold.sh` against this fork to overwrite goldens.
