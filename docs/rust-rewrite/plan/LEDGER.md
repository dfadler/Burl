# Planning ledger

Goal: a plan for the Rust rewrite of Gource complete enough that a fresh implementer,
given only `docs/rust-rewrite/plan/` and one phase brief, builds that phase and passes
its parity gate without opening the C++ and without asking a question. Inputs: the
discovery pass (`../discovery/`, closed 2026-10-29), the phase-1 code and report
(`../phase1.md`, branch `claude/hungry-kilby-78e776`), the C++ sources (read only),
crate documentation at pinned versions.

Loop: dynamic ticks, orchestrator = the model running `LOOP.md`, children = Sonnet
(medium), verifiers = Sonnet (high), depth 2, `width` 6 per tick.
Budget: 150 agents total. Reserve: the last 20 are for the final verification pass; no
gap tick may spend below 20 remaining. Window: 8 hours from the first tick.
Started: (first tick writes this). Hard stop: (started + 8 h).
Shared notes folder for hand-over notes: the Claude desktop app's per-session `side-session-notes` folder (outside the repository; the path is given in the session's environment).

Legend: `[x]` holds and was checked this tick by the script or a verifier, `[~]` written
but unverified, `[ ]` not started.

## Done criterion (the loop stops only when all four layers hold)

### A. Coverage (mechanical, `scripts/plan-coverage.sh` reports zero gaps)

- [ ] Every translation unit in `Makefile.am` has a row in `map/modules.md` (Rust
      module, status, owner phase); unbuilt files are listed as such.
- [ ] Every method of the sim, formats, settings, shell and core data files has a row in
      `functions/<file>.md` with a Rust signature, pseudocode that notes f32/f64 and
      operation order, the vector or golden that pins it, and a cite.
- [ ] Every option in `golden/help-long.txt` and in the `ARG_TYPES` table has a row in
      `settings-spec.md` with type, default, validation, exact error text, section,
      owning phase and golden.
- [ ] Every row of `parity/matrix.md` names a golden file that exists or a generator
      script that exists.
- [ ] Every crate in `rust/crates.md` has a pinned version, a docs.rs citation with a
      date, and a compiled snippet under `rust/snippets/` that `cargo check` accepts.
- [ ] Every remaining phase (1, 3, 4, 5, 6, 7) has `phases/phase-N.md` with the required
      sections: Inputs, Outputs, Module tree, Gate, Acceptance, Size, Decisions touched.

### B. Executability (empirical, the implementer dry-run)

- [ ] For every remaining phase, the latest `dry-runs/phase-N.md` records
      `questions: 0`, `cpp_opens: 0`, and either the gate passing or, where the gate
      needs goldens that cannot be captured on this machine, every interface and test
      the brief specifies compiling under `cargo check` in a scratch copy.
- [ ] Each dry-run was produced by a fresh agent that received only the plan folder and
      the phase brief, after the last edit to any document that phase consumes.

### C. Verification

- [ ] Every document under `plan/` (except LEDGER, LOOP, BRIEFS) carries a
      `verified: <commit> <date>` line newer than its last content change, written by a
      verifier that sampled at least 20% of its citations and found no substantive error
      (line shifts corrected in place do not count as substantive).
- [ ] Every correction to a discovery document is recorded in that document's
      "Corrections" section with the C++ line that shows it.

### D. Convergence and hard stops

- [ ] Two consecutive ticks added no new gap, question, correction or decision.
- [ ] The final verification pass ran with the reserve and changed nothing of
      substance (or its changes were re-dry-run).
- Hard stops that end the loop without the criterion holding: agent budget spent; the
  8-hour window elapsed. Both leave the ledger marked "Stopped: <reason>" with the
  remaining gap list, so a later loop resumes from it.

## Research index (open topics; a topic closes when its output file is verified)

| Topic | Output | Status |
| --- | --- | --- |
| C++ function-level pass: `gource.cpp` (logic, readLog, processCommit, interact*, update*, camera, seek) | `functions/gource.md` | [ ] |
| C++ function-level pass: `dirnode.cpp`, `file.cpp`, `user.cpp`, `pawn.cpp`, `action.cpp`, `zoomcamera.cpp`, `spline.cpp`, `core/quadtree.cpp`, `core/bounds.h` | `functions/<file>.md` | [ ] |
| C++ function-level pass: `gource_shell.cpp`, `caption.cpp`, `key.cpp`, `slider.cpp`, `textbox.cpp` | `functions/<file>.md` | [ ] |
| C++ function-level pass: `gource_settings.cpp`, `core/settings.cpp`, `core/conffile.cpp`, `main.cpp` | `functions/<file>.md`, `settings-spec.md` | [ ] |
| C++ function-level pass: `core/{display,shader,shader_common,texture,fxfont,vbo,ppm,png_writer,frustum,plane,resource,sdlapp,logger,mousecursor}.cpp`, `bloom.cpp` | `functions/<file>.md` (render tier: signatures and behaviour, pseudocode only where state is produced) | [ ] |
| Rust: f32 determinism (FMA, `-C target-feature`, `mul_add`, libm variance), with compiled evidence | `rust/handbook.md` sec. 1 | [ ] |
| Rust: arena and index patterns, borrow-safe read-before-write, no `unsafe` | `rust/handbook.md` sec. 2 | [ ] |
| Rust: `sdl2` + `glow` GL 3.2 core on macOS and Linux, `freetype-rs`, `image`, `png` APIs the plan relies on, compile-checked | `rust/crates.md`, `rust/snippets/` | [ ] |
| Rust: `pcre2`, `regex`, `chrono`, `roxmltree` claims already used by phase 1, re-cited at versions | `rust/crates.md` | [ ] |
| Rust: testing and golden harness idioms (`CARGO_BIN_EXE_*`, insta or hand-rolled, `trycmd`), sabotage checks in CI | `rust/handbook.md` sec. 3 | [ ] |
| Rust: CI matrix for Linux/macOS/Windows with Xvfb + llvmpipe on Linux, `cargo deny` for transitive licences, MSRV policy, clippy gates | `rust/handbook.md` sec. 4 | [ ] |
| Rust: packaging (resources, `include_bytes!` vs install dir, NSIS replacement, `cargo dist` or not) | `rust/handbook.md` sec. 5 | [ ] |
| Parity: goldens each phase needs, which exist, which need the C++ binary, which need Linux; generator scripts; comparator design | `parity/infra.md`, `parity/matrix.md` | [ ] |
| Parity: the state-dump schema as adopted for Rust, and how the C++ side is produced given decision D6 | `parity/infra.md` | [ ] |
| Phase briefs 1, 3, 4, 5, 6, 7 | `phases/phase-N.md` | [ ] |
| Dry-runs 1, 3, 4, 5, 6, 7 | `dry-runs/phase-N.md` | [ ] |

## Document index

| File | Covers | Status |
| --- | --- | --- |
| (the loop fills this table as files appear) | | |

## Agent budget (150 total, reserve 20)

| Tick | Spent | Remaining | Children (role: output) | Coverage summary line |
| --- | --- | --- | --- | --- |
| 0 | 0 | 150 | kit written by hand, no agents | (first tick fills) |

## Decisions for the user (defaults taken; the loop never blocks on these)

| # | Question | Default taken | Restoring flag | Status |
| --- | --- | --- | --- | --- |
| D1 | validate every `[gource]` section up front vs C++ lazy | up front | `--strict-lazy-sections` (proposed name) | open |
| D2 | block on log load in export mode vs C++ load-time-dependent frames | block | `--legacy-log-load` (proposed name) | open |
| D3 | PRNG default | documented Rust PRNG; glibc / Park / Script selectable | `GOURCE_RS_PRNG` | open |
| D4 | `--ffp` | accept, shader path | none | open |
| D5 | keep known C++ bugs (`cvs-exp` unmatched, `--stop-on-idle` no-op, help friction 0.67 vs 1.0, dir-name-position range) | keep during the port | none | open |
| D6 | how the C++ state-dump reference is produced without modifying `src/` on this branch: a separate never-merged instrumentation branch, a patch file applied only in the golden-capture script, or hand-derived vectors | patch file under `parity/`, applied by the capture script to a scratch checkout, never committed to `src/` | none | open |

## Corrections to discovery docs found by this loop

| Doc | Claim | Evidence | Recorded in |
| --- | --- | --- | --- |
| LEDGER.md (discovery) | "9 phases" | rust-architecture-map.md sec. 9 defines phases 0-7 | ../phase1.md, this file |

## Residual, known-unverified items (carried from discovery; the loop retires or re-lists them)

- glibc TYPE_3 first outputs from a Python model only; macOS Park-Miller from one compiled test; Windows `rand()` not researched.
- `vec2(rand()%100-50, rand()%100-50)` argument evaluation order assumed left-to-right.
- No render golden; SDL dummy driver has no GL on macOS; macOS has no llvmpipe.
- PPM pipe / SIGPIPE on the C++ side not exercised.
- Multi-section `[gource]` sequencing untraced; `tests/test.conf` has no expected outputs.
- Captions absent from the dump schema.
- Transitive crate licences unchecked.

## Stop rule: NOT MET
