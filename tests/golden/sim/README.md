# Simulation goldens (phase 3 gate, ticket #11)

`NAME.log` is a custom-format log, `NAME.args` the fork arguments beyond the fixed inputs of
`reference/gource-cpp-dump/scripts/dump-run.sh`, and `NAME.jsonl` the fork's schema-1 state
dump (Script-mode PRNG, `reference/gource-cpp-dump/tests/dump/rand-script.txt`).

- Regenerate only in a `golden-refresh` PR: `docs/rust-rewrite/discovery/golden/scripts/regen-sim.sh`
  (pinned amd64 Debian image; `PROVENANCE.txt` records the image id and source sha).
- `crates/burl/tests/sim_gate.rs` runs Burl on each fixture and compares through
  `xtask::simcmp` (also `cargo xtask sim-compare GOLDEN ACTUAL`): integers, bools, strings and
  topology exact, floats bit-exact f32, `dest` (`sinf`/`cosf`) at 1e-5 relative.
- `allowlist.txt` lists accepted differences by `DEVIATIONS.md` id; an unknown id is an error.

| fixture | exercises |
|---|---|
| `small-repo` | 5 users, nested dirs (fork and common-prefix nodes), a 10-file ring layout, modify, file removal, directory delete, file-to-directory, user overlap (`rand`) |
| `file-removal` | removal actions, file fade-out, user fade-out, `appFinished` |
| `file-to-dir` | a file turning into a directory (`forced_removal`) |
| `custom-dir-delete` | `D` on a directory path removing every file below it |
