# Discovery ledger

Goal: gather everything needed to rewrite Gource in Rust with the same CLI/config/log-format
API and the same parsing + simulation state (rendering visually equivalent, not pixel-identical).

Loop: 5-minute ticks, Fable orchestrator, Sonnet (medium) children, depth 3, budget 100 total.
Started: 2026-10-29T16:58:47Z. Hard stop: 2026-10-29T21:58:47Z (5 hours).
Budget reserve: the last 15 agents are reserved for a final verification-only pass; gap ticks may not spend below 15 remaining.

Stop rule: the checklist counts as complete only when every item is `[x]` AND at least one citation-verifier pass has
covered every doc AND the build attempt either produced goldens or recorded exactly which dependency blocked it.
Legend: `[x]` verified against source or a binary, `[~]` catalogued from a child's report but not independently verified, `[ ]` not started.

## Documents

| File | Covers |
|------|--------|
| app-core.md | gource.cpp/.h, shell state machine, logmill, camera, key, slider, textbox, caption, bloom, spline |
| scene-model.md | dirnode, file, user, pawn, action: layout forces, lifecycles, colours, RNG, iteration order |
| settings-cli.md | every CLI flag, config grammar/keys, precedence, help text, errors/exit codes, platform quirks |
| log-formats.md | all 9 parsers with verbatim regexes, external commands, SeekLog, Regex wrapper, stringhash, custom format |
| core-lib.md | Core submodule: display, sdlapp, fxfont, shaders, texture, quadtree, timezone, resource, ui, utf8 |
| output-pipeline.md | PPM/PNG output, fixed-timestep cadence, render pass order + GL state, full shader text, assets |
| deps-and-rust-mapping.md | build system, packaging, dependency floors, Rust crate candidates with tradeoffs |
| tests-and-golden-strategy.md | existing tests/fixtures, headless status, 3-tier golden strategy |
| timing.md | AUTHORITATIVE per-frame timing flow, time settings table, exporter vs interactive, stop conditions, wall-clock entry points |
| build-and-goldens.md | how the C++ binary was built here without autotools (clang++ -arch x86_64), dependency versions, golden capture commands |
| golden/ | captured goldens: help-short/long, exit-codes, custom-log/* for 5 fixtures plus stdin variants, unit-tests.txt; scripts/ (parameterised by BUILD=, with README) rebuild the binary and recapture everything; glibc_rand.py model |
| verification-tick2.md | citation sample check: 32 cites, no substantive errors, line shifts (now patched) |
| verification-tick3-scene.md | independently re-derived pseudocode for every sim function (f32/f64 noted) and a 9-row discrepancy table (now patched into scene-model.md / app-core.md) |
| misc-traces.md | PPM exporter thread, stdin isFinished, multi-repo reset set, --save-config format, hover/follow, captions, HiDPI, custom-log writer |
| unit-tests.md | Boost tests built and run (69 assertions pass); 28 datetime parse goldens; golden/unit-tests.txt |
| parity-test-spec.md | executable parity plan: parser goldens, --debug-dump-state JSONL schema, PRNG modes, C++ dump patch, tolerances, visual SSIM recipe, 28-row matrix |
| rust-architecture-map.md | crate/module layout, arena tree model, threading, renderer trait, hand-rolled CLI, regex strategy, PRNG/timing, 9 phases with parity gates |

## Agent budget (100 total)

| Tick | Spent | Remaining | Notes |
|------|-------|-----------|-------|
| 1 | 10 | 90 | 8 first-wave children (app-core used 3, the rest 1 each). All returned. |
| 4 | 5 | 71 | Final verification pass (from the 15-agent reserve), started ~17:34Z, done ~17:40Z (times approximate): ~300 claims/cites checked across timing.md, all Tick 2 additions, misc-traces, unit-tests, parity spec, architecture map. No structural errors; line shifts and a handful of wrong statements corrected in place with "(corrected tick 4)" markers. |
| total | 29 | 71 | Loop stopped 2026-10-29T17:47Z after 4 ticks (48 min of the 5h window; ticks were fast-forwarded as soon as each wave returned). Stop rule met; 71 agents unspent. |
| 3 | 5 | 76 | 5 children, started ~17:25Z, done ~17:33Z (approximate): scene verification 1, misc traces 1, unit tests 1, parity spec 1, architecture map 1. |
| 2 | 9 | 81 | 8 children, started early at 17:14Z, done ~17:24Z (approximate). Verifier 1, timing 1, build+goldens 1 (resumed once), gource.cpp finish 1, core gaps 1, config sequencing 1, rendering gaps 2, crate verification 1. |

## Done criterion (checklist)

- [x] Every file in src/ catalogued — app-core.md (gource.cpp Parts 1-2 corrected in tick 2, all flagged gaps covered) + scene-model.md
- [x] Every file in src/formats/ catalogued with exact parse rules and regexes — log-formats.md (all verbatim regexes verified char-for-char in tick 2)
- [x] Every file in src/core/ catalogued — core-lib.md + tick 2 additions (normalise, Frustum, quadtree visit order, utf8 replace_invalid, stringHash transcribed; ui/, tga.cpp, timer.cpp confirmed not built; vbo.cpp IS used)
- [x] Every CLI flag enumerated with source line, default, validation, error text — settings-cli.md; help text golden-captured from the real binary in golden/help-*.txt
- [x] Every config-file key and section enumerated, incl. precedence — settings-cli.md + tick 2 multi-section sequencing (ConfFile::save format still untraced)
- [x] Custom log format fully specified — log-formats.md (regex verified); goldens in golden/custom-log/
- [x] Simulation/layout algorithms captured — scene-model.md + app-core.md, independently re-derived in verification-tick3-scene.md (9 minor discrepancies patched; commits never sorted)
- [x] Output pipeline captured — output-pipeline.md + tick 2 (uniforms, fonts, labels, UI geometry, pawn formulas, DPI, textures)
- [x] Each C++ dependency mapped to Rust crates — deps-and-rust-mapping.md; 40 crates verified on crates.io, licenses GPLv3-compatible, GL stack recommendation (sdl2+glow GL 3.2 core), glibc vs macOS rand() documented
- [x] Build/packaging/platform specifics noted — deps-and-rust-mapping.md, settings-cli.md, build-and-goldens.md
- [x] Test fixtures understood; golden-output strategy written — tests-and-golden-strategy.md + parity-test-spec.md; parser, help, exit-code and unit-test goldens captured. utf8-caption.log is a --caption-file fixture (test.conf:39). No render golden on this machine (needs a real GL window; SDL dummy driver has no GL); Linux Xvfb + llvmpipe recipe written
- [x] Exit codes and error messages catalogued — settings-cli.md, app-core.md; golden/exit-codes.txt from the binary
- [x] Citation-verifier pass over every doc — tick 2 (eight tick-1 docs), tick 3 (sim formulas re-derived), tick 4 (timing.md, every Tick 2 additions section, misc-traces, unit-tests, parity spec option spellings, architecture map cross-check). Every doc now ends with a Verification section naming what was and was not checked.
- [x] Authoritative timing section — timing.md (min_delta_msec is 16 in Gource; exporter step 1/gource_fps; frame-dump phase depends on log load time; contradictions in other docs patched)
- [x] Build attempt — succeeded without autotools (clang++ -arch x86_64, Rosetta); goldens captured; build-and-goldens.md

## Key cross-cutting findings (tick 1)

- Rendering is legacy fixed-function GL + GLSL 1.10 compat; a Rust backend will be a redraw (wgpu or glow), visual equivalence only.
- Regex: PCRE2 byte-mode with look-around in internal patterns and raw user patterns for --file-filter/--user-filter. `regex` crate alone is insufficient; pcre2 or fancy-regex needed for parity.
- Simulation state depends on: unseeded libc `rand()` (overlap nudges, recolour), sorted-map iteration order, quadtree `std::set` pointer order, wall-clock dt unless exporting (fixed dt = 1/gource_fps). Rust port needs a documented PRNG and fixed-dt mode; bit-exact parity with the C++ binary is only achievable by porting glibc `rand()`.
- `--output-custom-log` is the only headless path today and is the natural parser golden.
- Bugs/quirks to preserve or decide on: `--log-format cvs-exp` unmatched in logmill; `--stop-on-idle` no-op; help text says user-friction 0.67 but code default 1.0; RDirNode ctor shadows `parent`.

## Key cross-cutting findings (tick 2)

- The C++ binary builds here without autotools via direct clang++ (x86_64 under Rosetta because Homebrew libs are x86_64). Parser and help goldens exist. Render goldens need a real GL window; plan Xvfb + llvmpipe on Linux CI.
- Timing: Gource's main loop uses a 16 ms minimum delta (not 8), the whole accumulated buffer becomes dt. Exporter uses fixed 1/gource_fps and dumps every Nth frame, but the dump phase depends on how many frames elapsed during log loading.
- Quadtree: visit order (insertion order, TL/TR/BL/BR, rebuilt per tick) affects f32 accumulation and rand() consumption; the std::set pointer order does not reach sim state. An O(n^2) replacement is not equivalent.
- rand() is platform-dependent (glibc TYPE_3 vs macOS Park-Miller), so bit-exact parity with the C++ binary is per-platform; Rust should ship its own documented PRNG plus an optional glibc-compatible mode.
- Config: CLI scalars are applied to every [gource] section; only the first section is validated at startup; a later bad section terminates the process.
- utf8 replace_invalid (commitlog.cpp:29) is not equivalent to String::from_utf8_lossy; the transcribed algorithm must be ported.

## Decisions flagged (behaviour the architecture map proposes to change vs C++)

- Validate every [gource] section up front instead of only the first (C++ terminates mid-run on a bad later section). Deviation from parity; decide before implementing.
- Block on log load in export mode so the PPM frame-dump phase does not depend on load time (C++ advances runtime/framecount during load). Deviation from parity; decide.
- Accept --ffp and use the shader path anyway (C++ switches to fixed-function). Rendering-only.
- Both deviations should be behind flags or recorded as intentional differences in the parity spec.

## Stop rule: MET (tick 4)

Every checklist item is `[x]`; a citation-verifier pass has covered every doc; the build produced goldens.
Mechanical check: every `.cpp` under src/ (excluding vendored tinyxml) is named in a doc except src/core/ui/*.cpp, which Makefile.am does not build (confirmed in core-lib.md and rust-architecture-map.md).

## Residual, known-unverified items (not blockers; listed so the rewrite does not treat them as settled)

- glibc `rand()` TYPE_3 first outputs were reproduced from a Python model, not run against a real glibc; macOS Park-Miller was confirmed by a compiled test here. Windows MSVC `rand()` not researched.
- `vec2(rand()%100-50, rand()%100-50)` argument evaluation order is unspecified in C++; clang left-to-right assumed. Pin it in the parity spec when the reference binary is chosen.
- stdin log behaviour: parser path and unterminated-last-line drop were exercised against the built binary (misc-traces.md, last section); empty-stdin startup and `--stop-at-end` with stdin remain read from source only.
- PPM pipe / SIGPIPE behaviour not exercised.
- No render golden exists; SDL's dummy driver has no GL on macOS and opening a real window was avoided. Use the Xvfb + llvmpipe recipe in parity-test-spec.md on Linux.
- Windows-only paths (DPI awareness, console, HWND subclass, frameless) are catalogued from source only.
- Crate maintenance judged from release dates; transitive licenses unchecked.
- tests/test.conf is referenced by nothing in the build; it is a manual scenario file.

## Decisions

Decided by the user (2026-10-30; all nine questions answered):

- Parity: same parsed commits and simulation state; rendering visually equivalent (see /CONTEXT.md).
- Rust workspace lives in this repo beside the C++, which stays until parity is proven.
- First release targets macOS and Linux. Windows users run it in Docker, headless frame export only.
- Delivery: two PRs against master, docs first, then phase 1 code.
- D1: validate every [gource] section up front.
- D2: export mode waits for the log to load first (keep a flag for the old behaviour).
- D3: documented Rust PRNG by default, with glibc and macOS compatibility modes for parity tests.
- D4: `--ffp` is accepted and ignored.
- D5: fix all four known C++ bugs now. Interpretation accepted 2026-10-31: the two help-text mismatches (user-friction, dir-name-position) are fixed by correcting the help text to match the code, because changing the code would change simulation state and break parity. `--log-format cvs-exp` is made to work. `--stop-on-idle` stays an accepted option documented as having no effect, with no new semantics (decided 2026-10-31). The help-text goldens in golden/ will therefore differ from the Rust output on those lines. The expected diff is stored as a unified diff under `docs/rust-rewrite/discovery/golden/expected-diffs/` (does not exist yet; written when the help text is implemented); the golden harness compares Rust output against the golden with that diff applied, and the goldens themselves stay untouched.

- Renderer stack: `winit` + `glutin` + `glow` (chosen over the recommended `sdl2` + `glow`). Consequences: input, IME, clipboard, cursor and high-DPI semantics differ from SDL, so the parity spec needs explicit input and DPI tests; `glutin` (last release 2025-04-30) and `glutin-winit` (2024-06-08) are slow-moving, so pin exact versions and keep the window and GL context behind the renderer trait so a swap stays cheap. No C SDL2 library is needed.
- D6: fork the C++ sources (chosen over the recommended patch file). Consequence: the original program is no longer the only oracle. Keep the untouched C++ build as the oracle for every byte-exact golden (help text, exit codes, custom-log), and use the fork only to emit the per-frame state dump. Guard: the fork must reproduce the same custom-log and help goldens as the original before its state dump is trusted. Decided 2026-10-31: the directory is `reference/gource-cpp-dump/` (outside src/, so the autotools build and the Rust workspace never see it; does not exist yet). It holds a README naming the commit it was forked from, a copy of the needed sources with the src/core submodule's files copied in (not a nested submodule), and a script proving the fork reproduces the oracle's help and custom-log goldens before its state dump is trusted.

Decided 2026-10-31 (naming): Burl is the name of the project and its program (the Rust rewrite and what it grows into). Gource means only the original C++ program (0.57) that Burl was derived from and is checked against. Burl is our fork and will become a whole rewrite; it is never submitted back to Gource, and nothing is proposed upstream. The code stays GPL-3.0-or-later (COPYING), so Gource's copyright and licence notices are preserved. The glossary is in /CONTEXT.md.

Awaiting the user (recommendation only): the program's binary name. During parity the help header, error prefix and usage text must match the goldens byte for byte ("Gource v0.57", "gource: <msg>"), so the crate and binary are currently named `gource`. Recommendation: keep `gource` while parity goldens run, keep the program name in one constant, and rename to `burl` as the first deliberate break after parity.

## Reading order for the rewrite

1. rust-architecture-map.md (start here), 2. parity-test-spec.md, 3. timing.md, 4. settings-cli.md + golden/help-*.txt, 5. log-formats.md + golden/custom-log/, 6. scene-model.md + verification-tick3-scene.md, 7. app-core.md, 8. core-lib.md, 9. output-pipeline.md, 10. deps-and-rust-mapping.md, 11. build-and-goldens.md + unit-tests.md, 12. misc-traces.md.
