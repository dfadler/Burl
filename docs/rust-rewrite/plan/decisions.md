# Decisions

The live table is in `LEDGER.md` under "Decisions for the user". This file collects the
reasoning behind each default and is updated when the user answers.

| # | Default taken | Why this default | What changes if the user chooses otherwise |
| --- | --- | --- | --- |
| D1 | validate all `[gource]` sections up front | a bad later section in C++ terminates mid-run; failing before anything plays is the only behaviour a user can act on | a parity test of the lazy path runs with the restoring flag |
| D2 | block on log load in export mode | makes the frame-dump phase a function of the input alone; the C++ phase depends on disk speed, which no golden can pin | `loaded_frame0` alignment in the comparator covers both |
| D3 | documented Rust PRNG by default | the C++ is not equal to itself across platforms; a documented default plus selectable glibc/Park/Script modes gives parity where it is measurable | glibc default would make Linux goldens pass unmodified and macOS ones fail |
| D4 | accept `--ffp`, render through shaders | fixed-function GL does not exist on GL 3.2 core | rejecting breaks scripts that pass it |
| D5 | keep known bugs during the port | goldens captured from the C++ stay valid until the last phase | fixing early forces a golden update per bug |
| D6 | C++ dump reference via a patch file applied to a scratch checkout by the capture script | keeps `src/` untouched on this branch while still producing the reference | an instrumentation branch is equivalent; hand-derived vectors do not scale past the fixtures |
