# Decisions

The live table is in `LEDGER.md` under "Decisions". All six were decided 2026-10-30/31; this
file collects the reasoning and records the final values.

| # | Default taken | Why this default | What changes if the user chooses otherwise |
| --- | --- | --- | --- |
| D1 | validate all `[gource]` sections up front | a bad later section in C++ terminates mid-run; failing before anything plays is the only behaviour a user can act on | decided, no flag |
| D2 | block on log load in export mode | makes the frame-dump phase a function of the input alone; the C++ phase depends on disk speed, which no golden can pin | decided; `loaded_frame0` alignment in the comparator covers both; an old-behaviour flag is optional and was not requested |
| D3 | documented Rust PRNG by default | the C++ is not equal to itself across platforms; a documented default plus selectable glibc/Park/Script modes gives parity where it is measurable | decided: documented Rust PRNG (xorshift) by default, glibc and macOS compatibility modes for parity tests |
| D4 | accept `--ffp` and ignore it | fixed-function GL does not exist on GL 3.2 core | decided; rejecting would break scripts that pass it |
| D5 | fix all four known bugs: explicit `cvs-exp` works, `--stop-on-idle` stays accepted with no effect, help text corrected to the code (friction 1.0, dir-name-position from 0.1) | the code defaults stay, so simulation state and parity are unchanged | decided; Gource goldens are never edited, the expected help diff lives under `discovery/golden/expected-diffs/` (does not exist yet) |
| D6 | fork the C++ sources into `reference/gource-cpp-dump/` (does not exist yet) for the state dump | keeps `src/` untouched; the untouched build stays the oracle for byte-exact goldens, and the fork must first reproduce the oracle's help and custom-log goldens | decided (chosen over the recommended patch file) |
