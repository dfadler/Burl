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
| golden/ | captured goldens: help-short/long, exit-codes, custom-log/* for 5 fixtures; scripts/ to rebuild and recapture |
| verification-tick2.md | citation sample check: 32 cites, no substantive errors, line shifts (now patched) |

## Agent budget (100 total)

| Tick | Spent | Remaining | Notes |
|------|-------|-----------|-------|
| 1 | 10 | 90 | 8 first-wave children (app-core used 3, the rest 1 each). All returned. |
| 2 | 9 | 81 | 8 children, started early at 17:14Z, done 17:40Z. Verifier 1, timing 1, build+goldens 1 (resumed once), gource.cpp finish 1, core gaps 1, config sequencing 1, rendering gaps 2, crate verification 1. |

## Done criterion (checklist)

- [x] Every file in src/ catalogued — app-core.md (gource.cpp Parts 1-2 corrected in tick 2, all flagged gaps covered) + scene-model.md
- [x] Every file in src/formats/ catalogued with exact parse rules and regexes — log-formats.md (all verbatim regexes verified char-for-char in tick 2)
- [x] Every file in src/core/ catalogued — core-lib.md + tick 2 additions (normalise, Frustum, quadtree visit order, utf8 replace_invalid, stringHash transcribed; ui/, tga.cpp, timer.cpp confirmed not built; vbo.cpp IS used)
- [x] Every CLI flag enumerated with source line, default, validation, error text — settings-cli.md; help text golden-captured from the real binary in golden/help-*.txt
- [x] Every config-file key and section enumerated, incl. precedence — settings-cli.md + tick 2 multi-section sequencing (ConfFile::save format still untraced)
- [x] Custom log format fully specified — log-formats.md (regex verified); goldens in golden/custom-log/
- [~] Simulation/layout algorithms captured — scene-model.md; tick 2 resolved out-of-order timestamps, quadtree ordering (visit order matters, set order does not), render-only functions. Remaining: independent verification of RUser::logic / RFile::logic / RDirNode::applyForces+move / ZoomCamera::adjust formulas; whether RCommitLog sorts commits
- [x] Output pipeline captured — output-pipeline.md + tick 2 (uniforms, fonts, labels, UI geometry, pawn formulas, DPI, textures)
- [x] Each C++ dependency mapped to Rust crates — deps-and-rust-mapping.md; 40 crates verified on crates.io, licenses GPLv3-compatible, GL stack recommendation (sdl2+glow GL 3.2 core), glibc vs macOS rand() documented
- [x] Build/packaging/platform specifics noted — deps-and-rust-mapping.md, settings-cli.md, build-and-goldens.md
- [~] Test fixtures understood; golden-output strategy written — tests-and-golden-strategy.md; parser + help goldens captured. utf8-caption.log is a --caption-file fixture (test.conf:39), not a log. Remaining: build+run src/test unit tests; no render golden (needs a real GL window; SDL dummy driver has no GL)
- [x] Exit codes and error messages catalogued — settings-cli.md, app-core.md; golden/exit-codes.txt from the binary
- [~] Citation-verifier pass over every doc — tick 2 pass covered the eight tick-1 docs (verification-tick2.md); tick 2 additions and timing.md not yet verified
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

## Open gaps (for tick 3)

1. Scene-model verification: independently re-derive RUser::logic, RFile::logic, RDirNode::applyForces/move/updateSplinePoint, ZoomCamera::adjust from source and diff against scene-model.md/app-core.md; confirm whether RCommitLog/logmill sorts commits or relies on log order.
2. Small untraced items: PPMExporter internals (ppm.cpp), commitlog isFinished for stdin, multi-repo settings re-import gource_settings.cpp:1280-1357, ConfFile::save format/ordering, hover->camera-follow path, loadCaptions regex group indexing, Bounds2D line width on HiDPI, --high-dpi -> enableHighDPIAwareness call site.
3. Build and run src/test unit tests (datetime, regex) with the same clang++ recipe; record outputs as goldens.
4. Parity-test spec: concrete debug-dump field list per entity (from scene-model member tables), fixed-dt mode, PRNG plan, and the exact golden commands; one doc.
5. Rust architecture map: C++ file/class -> Rust crate/module layout, threading model (logmill thread, PPM writer thread), renderer trait boundary, settings/config model; one doc the rewrite can start from.
6. Final verification pass (reserved 15): sample cites in all tick 2/3 additions, timing.md, and the architecture map.
