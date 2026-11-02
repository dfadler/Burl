# Deviation register

Every intended difference between Burl and Gource 0.57 (the oracle). Policy: [#10](https://github.com/dfadler/Burl/issues/10).

## Categories

- **Change**: a deliberate, decided difference.
- **Gap**: a known, unfixed, low-impact difference. Every Gap row names a fix trigger.
- **Bug**: any difference not listed here. A Bug fails the build.

A new Change or Gap needs a row in this file **and the user's explicit OK** before it merges. An unexplained difference is a Bug by default. There is no `--legacy-log-load` flag.

## How a row is pinned

- **Text output** (help, stderr, custom log): a unified diff `expected-diffs/<golden-stem>.diff`, stored next to the oracle golden (today `docs/rust-rewrite/discovery/golden/expected-diffs/`, later `tests/golden/`). The harness applies it to a copy of the golden with `patch` and compares Burl's output with the result. Goldens are never edited. Paths in the diff are the golden's basename. The golden is written with the oracle's program name (`gource`); the harness normalises the name on both sides, so diffs are written against the golden as stored.
- **Behaviour**: a Rust test named after the row id, e.g. `deviation_d5_cvs_exp_explicit`. A Burl-only test has no oracle golden.
- **Simulation comparator**: a per-id allowlist keyed to the row id. There is no tolerance flag for deviations (numeric f32 tolerance is a separate matter, phase 3 gate).

## Register

| id | Gource does | Burl does | Why | Category | Pinned by |
|----|-------------|-----------|-----|----------|-----------|
| D1 | Validates each `[gource]` config section when it is reached; a bad later section ends the run mid-way | Validates every section up front, before anything plays | Failing before output starts is the only behaviour a user can act on | Change | `deviation_d1_*` (with the multi-section `ConfFile`, not yet written) |
| D2 | In export mode the frame-dump phase depends on how fast the log loads | Blocks on log load in export mode | Makes the frame dump a function of the input alone | Change | fork `loaded_frame0` alignment in the comparator; `deviation_d2_*` (not yet written) |
| D3 | `rand()`, platform dependent | Documented Rust PRNG by default; `--prng default\|glibc\|park` compat modes | Gource is not equal to itself across platforms | Change | PRNG unit tests, `--prng` fixtures; ADR when written |
| D4 | `--ffp` selects the fixed-function GL path | Accepted, no effect | No fixed-function GL on 3.2 core | Change | `deviation_d5_ignored_options` (covers `--ffp`) |
| D5a | Help says `--user-friction` default 0.67; code default is 1.0 | Help says 1.0 | Fix the text, not the code (code change would alter sim state) | Change | `expected-diffs/help-long.diff`; `help_text_matches_golden` |
| D5b | Help says `--dir-name-position` is "between 0.0 and 1.0"; code rejects below 0.1 | Help says 0.1 to 1.0 | As D5a | Change | `expected-diffs/help-long.diff`; `help_text_matches_golden` |
| D5c | Explicit `--log-format cvs-exp` fails with "unsupported log format" (`fetchLog` matches `"cvs"`) | Selects the CVS-EXP parser | Plain bug; auto-detect already worked | Change | `deviation_d5_cvs_exp_explicit` (Burl-only) |
| D5d | `--stop-on-idle` is accepted and does nothing | Same, via the `Ignored` option kind (#8); silent | Keeps scripts working; no invented semantics | Change | `deviation_d5_ignored_options` |
| D8 | `StreamLog` polls for the first byte of stdin; `SeekLog` fails on logs over 100 MiB | Blocking stdin read; no size limit | Fixed with the settings import | Change | Burl-only tests when the settings import lands (not yet written) |
| D9 | Resources found via a search path | Embedded with `include_bytes!` | Single-binary release (#21) | Change | phase 7 packaging tests |
| G1 | SIGPIPE behaviour on a closed PPM pipe | Not matched | Not exercised by any golden | Gap | Fix trigger: first user report or phase 5 export tests |
| G2 | TinyXML quirks (e.g. CDATA handling) in svn logs | `roxmltree` semantics | Edge cases only | Gap | Fix trigger: a real log that parses differently; Burl-only test when found |
| G3 | High-DPI window sizing on Retina | Small window for a viewport without `--high-dpi` (#20) | Exact-pixel window rule | Gap | Fix trigger: phase 6 manual checklist |

Rows D1, D2, D8, D9 and G1 to G3 are placeholders carried from the decision tickets; their pinning tests and exact wording are filled in by the PR that implements them.
