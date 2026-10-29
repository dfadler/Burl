# Discovery ledger

Goal: gather everything needed to rewrite Gource in Rust with the same CLI/config/log-format
API and the same parsing + simulation state (rendering visually equivalent, not pixel-identical).

Loop: 5-minute ticks, Fable orchestrator, Sonnet (medium) children, depth 3, budget 100 total.
Started: 2026-10-29 (see scratchpad loop-start). Hard stop: 5 hours after start.

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

## Agent budget (100 total)

| Tick | Spent | Remaining | Notes |
|------|-------|-----------|-------|
| 1 | 10 | 90 | 8 first-wave children (app-core used 3, the rest 1 each). All returned. |

## Done criterion (checklist)

- [~] Every file in src/ catalogued — app-core.md + scene-model.md. gource.cpp sections written by sub-agents are unverified; input handlers and updateUsers after line 1400 only partly covered.
- [x] Every file in src/formats/ catalogued with exact parse rules and regexes — log-formats.md
- [~] Every file in src/core/ catalogued — core-lib.md; gaps in sdlapp dt loop, vectors normalise, quadtree query callers, utf8 semantics
- [x] Every CLI flag enumerated with source line, default, validation, error text — settings-cli.md (help text not golden-tested against a binary)
- [~] Every config-file key and section enumerated, incl. precedence — settings-cli.md; multi-section sequencing path not traced
- [x] Custom log format fully specified — log-formats.md
- [~] Simulation/layout algorithms captured — scene-model.md; out-of-order timestamps, quadtree std::set pointer-order effect, VBO/calcEdges state mutation unchecked
- [x] Output pipeline captured — output-pipeline.md
- [~] Each C++ dependency mapped to Rust crates — deps-and-rust-mapping.md; crate license strings and some claims unverified
- [x] Build/packaging/platform specifics noted — deps-and-rust-mapping.md, settings-cli.md
- [x] Test fixtures understood; golden-output strategy written — tests-and-golden-strategy.md
- [x] Exit codes and error messages catalogued — settings-cli.md, app-core.md (logmill errors)

## Key cross-cutting findings (tick 1)

- Rendering is legacy fixed-function GL + GLSL 1.10 compat; a Rust backend will be a redraw (wgpu or glow), visual equivalence only.
- Regex: PCRE2 byte-mode with look-around in internal patterns and raw user patterns for --file-filter/--user-filter. `regex` crate alone is insufficient; pcre2 or fancy-regex needed for parity.
- Simulation state depends on: unseeded libc `rand()` (overlap nudges, recolour), sorted-map iteration order, quadtree `std::set` pointer order, wall-clock dt unless exporting (fixed dt = 1/gource_fps). Rust port needs a documented PRNG and fixed-dt mode; bit-exact parity with the C++ binary is only achievable by porting glibc `rand()`.
- `--output-custom-log` is the only headless path today and is the natural parser golden.
- Bugs/quirks to preserve or decide on: `--log-format cvs-exp` unmatched in logmill; `--stop-on-idle` no-op; help text says user-friction 0.67 but code default 1.0; RDirNode ctor shadows `parent`.

## Open gaps (for tick 2)

1. Verify gource.cpp sections of app-core.md against source; finish keyPress/mouseMove/mouseClick, updateUsers (>1400), updateBloomVBO/updateFilesVBO/calcEdges state mutation, out-of-order timestamp handling, camera 1475-1550, gource.h member defaults.
2. core/sdlapp.cpp dt + min_delta_msec loop; core/vectors normalise zero-guard; Frustum fields; utf8 replace_invalid; quadtree.cpp:135-300 and whether std::set pointer order reaches sim state via gource.cpp query callers.
3. Config multi-section sequencing (main.cpp:73-105, gource_settings.cpp:618, conffile.cpp); data/gource.1 vs -h text; README.md:330-end.
4. Rendering detail: shader.cpp:262-330 + shader_common uniform plumbing; fxfont.cpp:200-430 advance/kerning; label placement user.cpp:384-395, dirnode.cpp ~900-940; key/slider/textbox geometry; display.cpp:551-588 Windows DPI/frameless.
5. Rust mapping: verify crate licenses and maintenance on crates.io; git quoted-path octal unescape; document glibc rand() algorithm for a bit-exact port option.
6. Try to build gource in this worktree with already-installed deps only (no installs) to golden-capture `-h`/`-H` and `--output-custom-log` on tests/logs fixtures.
