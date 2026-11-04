# PRNG: a documented default generator and three compatibility modes

Status: accepted (decided with the user, 2026-11-01). Register row: D3 in `docs/rust-rewrite/DEVIATIONS.md`.

Sources: [#22](https://github.com/dfadler/Burl/issues/22) (how the mode is selected), [#11](https://github.com/dfadler/Burl/issues/11) item 2 (which modes the parity gate uses). This record restates those resolutions and names the code that implements them; it adds no decision.

## Context

The Gource C++ calls libc `rand()` at four sites. Its output differs between platforms (glibc and macOS libc use different algorithms), so Gource is not equal to itself across platforms. The parity gate (#11) still needs a way to reproduce the oracle's draws exactly.

## Decision

1. **Default generator.** Burl uses a documented Rust generator by default, seeded from the clock like the C++, so unscripted runs vary. A hidden `--prng-seed N` reproduces a run. A fixed default seed was rejected because it would be a visible behaviour change; #22 says to flip it only if reproducible exports by default are wanted.
2. **Compatibility modes stay.** `--prng default|glibc|park` selects the generator; `--rand-script FILE` selects Script mode on its own. Combining the two is an error.
3. **Selection is a hidden CLI option, not an environment variable.** `--prng`, `--prng-seed`, `--rand-script` and the `--debug-dump-*` options sit in the option table's `CommandLine` section: run immediately, never stored, never saved by `--save-config`, never in help. The spec's `GOURCE_RS_PRNG` is dropped.
4. **Ungated in release builds.** All modes are available on every platform, so the binary the gates tested is the binary that ships.
5. **How the gate uses the modes** (#11): Script mode is the primary gate. glibc mode is checked in one supplementary fixture against the fork's real `rand()`. Park-Miller mode is unit-tested against the archived macOS vectors, not in the sim matrix.

## Implementation (`crates/burl-util/src/prng.rs`)

The default is xorshift64* (shifts 12/25/27, multiplier `0x2545F4914F6CDD1D`, top 31 bits of the output), with the seed run through one splitmix64 step. The tickets require a documented algorithm and name none; this one is what the module documents and freezes. Changing it changes every default-mode run for a given `--prng-seed`.

## Consequences

- Default-mode output is not comparable with the oracle; parity runs must pass `--prng glibc`, `--prng park` or `--rand-script`.
- The default algorithm is frozen by test, so a change is deliberate.

## Pinned by

`deviation_d3_default_prng_is_frozen`, `deviation_d3_glibc_matches_real_libc_*`, `deviation_d3_park_matches_archived_macos_vectors` (`burl-util`), `sim_gate_small_repo_glibc`.
