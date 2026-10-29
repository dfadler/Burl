# Discovery ledger

Goal: gather everything needed to rewrite Gource in Rust with the same CLI/config/log-format
API and the same parsing + simulation state (rendering visually equivalent, not pixel-identical).

Loop: 5-minute ticks, Fable orchestrator, Sonnet (medium) children, depth 3, budget 100 total.
Started: 2026-10-29 (see scratchpad loop-start). Hard stop: 5 hours after start.

## Agent budget (100 total)

| Tick | Spent | Remaining | Notes |
|------|-------|-----------|-------|

## Done criterion (checklist)

- [ ] Every file in src/ catalogued: purpose, public API, key state, behavior, constants
- [ ] Every file in src/formats/ catalogued with exact parse rules and regexes
- [ ] Every file in src/core/ (incl. ftgl/, ui/, utf8/) catalogued
- [ ] Every CLI flag enumerated with source line, default, validation, error text
- [ ] Every config-file key and section enumerated, incl. precedence vs CLI
- [ ] Custom log format fully specified, incl. edge cases from tests/logs
- [ ] Simulation/layout algorithms captured (tree layout, forces, user movement, timing, RNG use)
- [ ] Output pipeline captured (PPM stream, screenshot PNG, framerate, high-dpi)
- [ ] Each C++ dependency mapped to Rust crate candidates with tradeoffs
- [ ] Build/packaging/platform specifics noted (autotools, qmake, nsis, macOS, Windows)
- [ ] Test fixtures understood; golden-output strategy written
- [ ] Exit codes and error messages catalogued

## Open gaps (updated each tick)

(none recorded yet)
