# Simulation goldens (phase 3 gate, ticket #11)

`NAME.log` is a custom-format log, `NAME.args` the fork arguments beyond the fixed inputs of
`reference/gource-cpp-dump/scripts/dump-run.sh` (`@auto-rotate` first or anywhere leaves auto-rotate on; `@conf:PATH` as the first word makes it a multi-section run of the conf at PATH relative to `tests/`, with no `NAME.log`, display pinned only, and the rest of the words passed through), and `NAME.jsonl` the fork's schema-1 state
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
| `small-repo-track` | `--camera-mode track` with nothing selected: the camera fits `active_user_bounds` while users are active and `dir_bounds` once idle |
| `small-repo-follow` | `--camera-mode track --follow-user bob --follow-user nobody`: `selectUser` on the first active frame (camera `lockon`, `lockon_time` decay), deselect when the user fades out, a name that never appears |
| `small-repo-select` | the fork's scripted pointer (`--debug-pointer-script`, `@mouse`): Tab (`selectNextUser`) in track mode: first user, next in tag order, a user that is not yet opaque, none eligible |
| `small-repo-pick` | scripted pointer: hover and click-select a user, Tab, hover and click-select a file, a same-frame down/up on the background (does nothing), `selectBackground` (deselect, grab, drag), a faded user that cannot be hovered |
| `small-repo-max-files` | `--max-files 6` |
| `small-repo-elasticity` | `--elasticity 0.5` |
| `small-repo-tuning` | `--user-friction 0.5 --user-scale 2 --max-user-speed 100 --max-file-lag 0.5 --filename-time 2 --highlight-all-users` |
| `small-repo-fps30` | `-r 30`: the output frame rate halves the dumped frames; `--debug-dump-every 5` counts those (the other fixtures run at `-r 60`) |
| `small-repo-stop-position` | `--stop-position 0.5` (run ends early) |
| `small-repo-start-position` | `--start-position 0.5`: the log seeks to byte 460 (mid-line, so the rest of that line is thrown away) before the first frame |
| `small-repo-loop` | `--loop --loop-delay-seconds 1`: after the log ends and the users idle, `Gource::reset` and a seek to 0 replay it from a live state, several times in the 10 s run |
| `auto-skip` | multi-day gaps with `--auto-skip-seconds 1` (the other fixtures run with auto-skip off) |
| `time-travel`, `no-time-travel` | a log whose timestamps go backwards, default and with `--no-time-travel` |
| `small-repo-captions` | `--caption-file` (`.txt` beside it): a BOM line, a date-string stamp (TZ=UTC), non-UTF-8 byte, ignored lines (too early, out of order, no bar, bad stamp), `--caption-duration 2` so fade-in and expiry show, one caption never reached |
| `real-auto-rotate` | the real-repo log with auto-rotate left on (`@auto-rotate`), stop 20 s, 600 frames: the 90-degree rotation runs at frames ~210 to 510, Script-mode PRNG, exact f32 incl. rotated positions |
| `test-conf` | `tests/test.conf` unchanged (eight sections); every section's `path` is replaced by `test-conf.log` (a positional path overrides all sections, as in `main.cpp`) because sections 3 to 5 use `path=.`, which needs a git checkout |
| `multi-reset` | purpose-built conf: no stop time (60-second rule), and section 1 sets hash seed, idle time, elasticity, friction, max files and captions that must not reach section 2 |
| `pcre2-filters` | look-behind, backreference and look-ahead filters; compared only in the `pcre2` leg (`cargo test --features pcre2`), register row G4 |

Multi-section dumps are one file: a `section` record per `[gource]` section, frame counters restart, the generator and `rand_calls` carry on. The comparator pairs frames on (section, frame - loaded_frame0).

The fork blocks on the async log load when dumping (`Gource::update`, dump mode only), so its frame 0 is the load frame, as in Burl (D2). Without that, a real log loaded one frame late (`loaded_frame0` 1), which shifts the fork's `runtime` and stop time by that frame and the every-N sampling by one.

A fixture whose `.args` contain `--prng` runs without `--rand-script` (`dump-run.sh` and `sim_gate.rs` both check).
