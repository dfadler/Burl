# Simulation goldens (phase 3 gate, ticket #11)

`NAME.log` is a custom-format log, `NAME.args` the fork arguments beyond the fixed inputs of
`reference/gource-cpp-dump/scripts/dump-run.sh`, and `NAME.jsonl` the fork's schema-1 state
dump (Script-mode PRNG, `reference/gource-cpp-dump/tests/dump/rand-script.txt`).

- Regenerate only in a `golden-refresh` PR: `docs/rust-rewrite/discovery/golden/scripts/regen-sim.sh`
  (pinned amd64 Debian image; `PROVENANCE.txt` records the image id and source sha).
- `crates/burl/tests/sim_gate.rs` runs Burl on each fixture and compares through
  `xtask::simcmp` (also `cargo xtask sim-compare GOLDEN ACTUAL`): integers, bools, strings and
  topology exact, floats bit-exact f32, `dest` (`sinf`/`cosf`) and file `pos` (which chases `dest`) at 1e-5 relative.
- `allowlist.txt` lists accepted differences by `DEVIATIONS.md` id; an unknown id is an error.

| fixture | exercises |
|---|---|
| `small-repo` | 5 users, nested dirs (fork and common-prefix nodes), a 10-file ring layout, modify, file removal, directory delete, file-to-directory, user overlap (`rand`) |
| `small-repo-idle` | the same log with `--file-idle-time 2 --file-idle-time-at-end 1 --hash-seed 7` (idle fade, expiry, `gGourceRemovedFiles`) |
| `file-removal` | removal actions, file fade-out, user fade-out, `appFinished` |
| `file-to-dir` | a file turning into a directory (`forced_removal`) |
| `custom-dir-delete` | `D` on a directory path removing every file below it |
| `real-burl-git-250` | slice 2 real-repo run (#11 item 7): this repository's first 250 commits as a git log (`tests/golden/real/burl-git-250.log`, copied), `--stop-at-time 60`, dumped every 30 frames; ~110 files, quadtree forces over 2 to 7 users |
| `small-repo-glibc` | the small-repo log with `--prng glibc`: the fork's real glibc `rand()` (`srand(1)`) against Burl's `Glibc` generator (the only fixture not in Script mode; Park is unit-tested, #11 item 2) |
| `small-repo-max-files` | `--max-files 6` |
| `small-repo-elasticity` | `--elasticity 0.5` |
| `small-repo-tuning` | `--user-friction 0.5 --user-scale 2 --max-user-speed 100 --max-file-lag 0.5 --filename-time 2 --highlight-all-users` |
| `small-repo-stop-position` | `--stop-position 0.5` (run ends early) |
| `auto-skip` | multi-day gaps with `--auto-skip-seconds 1` (the other fixtures run with auto-skip off) |
| `time-travel`, `no-time-travel` | a log whose timestamps go backwards, default and with `--no-time-travel` |

The fork blocks on the async log load when dumping (`Gource::update`, dump mode only), so its frame 0 is the load frame, as in Burl (D2). Without that, a real log loaded one frame late (`loaded_frame0` 1), which shifts the fork's `runtime` and stop time by that frame and the every-N sampling by one.

A fixture whose `.args` contain `--prng` runs without `--rand-script` (`dump-run.sh` and `sim_gate.rs` both check).
