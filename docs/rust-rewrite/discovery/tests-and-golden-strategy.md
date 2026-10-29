# Tests and golden-output strategy

## 1. Existing automated tests (Boost.Test, 2 cases)
- Build/run: `check_PROGRAMS = gource_tests` (Makefile.am:92), sources `src/test/{main,datetime_tests,regex_tests}.cpp` plus all `${sources}` (Makefile.am:97-101), `TESTS = gource_tests` (Makefile.am:103). Run with `make check`. There is no `--enable-tests` flag; configure.ac only requires Boost >= 1.69 and Boost Filesystem (configure.ac:48-53).
- `src/test/main.cpp:1-2`: `#define BOOST_TEST_MODULE Gource Test Suite` and `<boost/test/included/unit_test.hpp>`.
- `src/test/datetime_tests.cpp:23-112`, test `parse_date_time_tests`. Every call is `SDLAppSettings::parseDateTime(str, timestamp)` and must return true. `putenv("TZ=UTC")` at l.27, then `putenv("TZ=Pacific/Auckland")` at l.31 (stays set for all later cases). Input -> timestamp:
  - TZ=UTC: `2021-11-01` -> 1635724800
  - TZ=Auckland: `2021-11-01` -> 1635678000
  - `2021-11-01Z` 1635724800; `2021-11-01+0` 1635724800; `2021-11-01+13` 1635678000; `2021-11-01 +13` 1635678000; `2021-11-01+5:30` 1635705000
  - `2021-11-01 12:01` 1635721260; `12:01Z` 1635768060; `12:01+0` 1635768060; `12:01+13` 1635721260; `12:01 +13` 1635721260; `12:01+5:30` 1635748260 (all prefixed `2021-11-01 `)
  - `2021-11-01 12:01:59` 1635721319; `2021-11-01T12:01:59.123` 1635721319 (fraction ignored); ` 12:01:59Z` 1635768119; `+0` 1635768119; `+13` 1635721319; ` +13` 1635721319; `+5:30` 1635748319
  - `2021-11-01T12:01:59` 1635721319; `T12:01:59Z` 1635768119; `T12:01:59+0` 1635768119; `T12:01:59+13` 1635721319 (listed twice, l.101 and l.104); `T12:01:59+5:30` 1635748319; `T12:01:59.123+5:30` 1635748319
  - A zone-less input uses the process TZ (Auckland after l.31). Rust must read TZ or take it as a parameter.
- `src/test/regex_tests.cpp:22-45`, test `regex_tests` (core/regex.h): `Regex("^[A-Z]+$")` is valid; `match("ABC")` true, `match("abc")` false, `match("ABC123")` false; a copy-constructed regex is valid and matches "ABC"; `Regex("^[A-Z+$")` throws `RegexCompilationException`; `Regex("^[A-Z+$", true)` gives `isValid()==false` (no-throw ctor); `Regex("([0-9]+)").match("ABC123", &matches)` is true with `matches.size()==1` and `matches[0]=="123"` (unanchored search, capture group 1 only). The Rust `regex` crate lacks look-around and backrefs, so also test real user filter patterns.

## 2. Manual scenario suite: tests/test.conf
- Nothing references it (no build rule, script or doc found). It is a config with 8 repeated `[gource]` sections, meant for `gource --load-config tests/test.conf` run from `tests/` (paths are relative: `logs/...`, `images/`). There is no section-selection flag. Loading: main.cpp:73-80 loads the conf and re-parses argv over it; main.cpp:83-105 applies a trailing positional path to every `[gource]` section (`conf.getSections("gource")`); `default_section_name = "gource"` at gource_settings.cpp:219, `addSection("gource")` at gource_settings.cpp:618. Multiple `[gource]` sections are Gource's multi-repo mode (`repo_count`, used at gource.cpp:~165). The exact sequencing logic is not traced (see Gaps).
- Cases (test.conf lines):
  1. l.1-5 "File removal": file-removal.log, `file-idle-time=0.0`, stop 10
  2. l.7-10 "Convert 'file' to a directory": file-to-dir.log, stop 10
  3. l.12-15 "User filter (Andrew)": `user-filter=Andrew`, stop 5 (no path; needs a positional log)
  4. l.17-20 "Filename filter (.cpp)": `file-filter=\.cpp`, stop 5 (no path)
  5. l.22-25 "Filename show filter (.cpp)": `file-show-filter=\.cpp`, stop 5 (no path)
  6. l.27-30 "Directory delete (SVN)": svn-dir-delete.log, stop 5
  7. l.32-35 "Directory delete (Custom)": custom-dir-delete.log, stop 5
  8. l.37-42 "UTF-8 handling": `caption-file=logs/utf8-caption.log`, `user-image-dir=images/`, utf8.log, stop 5
- There are no expected outputs; all checks are visual.

## 3. Fixtures (tests/logs, tests/images)
| file | content / edge case |
|---|---|
| custom-dir-delete.log | custom format: adds under `path/to/` and `path/to/blah/`, then `D|path/to/` (directory path delete, recursive removal) |
| file-removal.log | 5 adds in one second, 4 deletes at +1s, last delete at +11s (past stop-at-time 10); trailing blank line (must be skipped) |
| file-to-dir.log | `path/to/file/Not`, `.../Not/Not`, ... `.../Not/Not/Not/File`: name collisions between node names and a "file" turning into a directory |
| svn-dir-delete.log | SVN XML; `<path` with attributes split across lines (`kind="dir"` and `action="D"` on separate lines); delete of dir `/blah` removes all its files; fractional-second ISO date with `Z`; empty `kind=""` |
| utf8.log | custom format, non-ASCII UTF-8 username and path (`Árvíztűrő tükörfúrógép`), space inside the username |
| utf8-caption.log | caption file `timestamp|text`; first line epoch, second `YYYY-MM-DD HH:MM:SS`; UTF-8 text |
| images/Árvíztűrő tükörfúrógép.png | avatar found by UTF-8 username via `user-image-dir`; macOS NFC/NFD filename normalization is a risk |

## 4. Headless capability today
- Parser-only path is headless: `--output-custom-log FILE|-` (gource_settings.cpp:113, 250, 329, 583-585) makes main.cpp:119-122 call `Gource::writeCustomLog(path, file)` and `exit(0)` before any display or SDL window is created (display setup follows at main.cpp:128+). It needs a path. Output is one `%lld|%s|%s|%s\n` line per file (timestamp, username, action, filename; gource.cpp:~199-205). Limits: commit grouping and colours are lost; a TODO notes no exception handling (gource.cpp:~179); silently returns if the output file can't be opened (gource.cpp:~189).
- The full simulation is not headless. There is no `--headless` flag and no `SDL_VIDEODRIVER` handling. Display always calls `SDL_CreateWindow` and `SDL_GL_CreateContext` with the compatibility profile (core/display.cpp:219-256), so it needs fixed-function GL. The SDL `dummy` video driver has no GL, so use Xvfb with Mesa llvmpipe (`LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe`). SDL's `offscreen` driver is unverified; check the SDL2 docs.
- Frame dump without a screen: `-o file.ppm` / `--output-ppm-stream` writes P6 frames (core/ppm.cpp:175-186; option at gource_settings.cpp:104, core/settings.cpp:39,446). With an exporter, `scaled_dt = max_tick_rate` (gource.cpp:~250), so time advances in fixed steps; `runtime += scaled_dt` (gource.cpp:259) and `stop-at-time` compares against that simulated runtime (gource.cpp:261). Without an exporter, dt is real frame time and runs are not reproducible.

## 5. Where randomness and non-determinism enter
- Unseeded `rand()`: dirnode.cpp:625 (directory jitter), user.cpp:106 and user.cpp:128 (user acceleration), gource.cpp:1555 (`changeColours`, key-press only). `srand(time(0))` only for `--start-position random` (gource_settings.cpp:1319-1320).
- String hash seed `gStringHashSeed`: default 31 (gource_settings.cpp:489), `--hash-seed` (gource_settings.cpp:1062-1066). Drives colours and dir placement ("pseudo-randomness", dirnode.cpp:785). Deterministic given the seed; pin it.
- Wall clock: `SDL_GetTicks()` is used only for profiling timers (gource.cpp:263-267 and others), not state.
- Without a fixed seed the stream is libc-dependent (glibc default seed 1; macOS differs). To get bit-exact state, Rust must port the glibc TYPE_3 `rand()` (about 15 lines) and call it in the same order. Otherwise accept tolerance-based comparison.

## 6. Golden-output strategy
### (1) Parser equivalence (exact text diff, no GL)
```
# reference, once, commit to tests/golden/
for f in tests/logs/*.log; do gource --output-custom-log - "$f" > "tests/golden/$(basename "$f").custom"; done
# Rust
for f in tests/logs/*.log; do gource-rs --output-custom-log - "$f" | diff -u "tests/golden/$(basename "$f").custom" -; done
```
- Skip utf8-caption.log (caption format; add a separate caption dump check).
- Run real logs: `gource --git-log-command` prints the exact git command (also `--svn-log-command`, `--hg-log-command`, `--bzr-log-command`, gource_settings.cpp:~565-580); run it on a pinned repo and commit a small sample of git, `svn log --xml -v` and hg output; also feed each through `--log-format`.
- Edge inputs: empty file (compare exit code and stderr), CRLF, blank lines, out-of-order timestamps, `|` in names, UTF-8 BOM.
- Port the section 1 date vectors straight into `cargo test` (set TZ per case).

### (2) Simulation-state equivalence
Add `--debug-dump-state FILE` to the C++ reference and to Rust, writing JSON lines at the end of `Gource::logic` (called at gource.cpp:265) every N frames (0, 30, 60, ...): frame, runtime, current log time/commit cursor, counts of users/files/dirs, then entities sorted by path or name: file {path, pos, dest, vel, alpha, colour, removing}, user {name, pos, vel, action count}, dir {path, pos, parent angle, radius}, camera {pos, zoom}. Print floats `%.4f`.
```
gource tests/logs/file-removal.log -o /dev/null -r 60 --seconds-per-day 1 --stop-at-time 10 --hash-seed 31 --disable-auto-rotate --debug-dump-state out.jsonl
```
Compare in two tiers. Exact: topology, file add/remove/idle-expiry frames, user set, colour hashes. Tolerance (about 1e-3 relative): positions and velocities, unless glibc `rand` is ported with identical f32 operation order.

### (3) Visual equivalence (Linux CI)
```
xvfb-run -a -s "-screen 0 1280x720x24" env LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe \
  gource tests/logs/file-removal.log -1280x720 --seconds-per-day 1 --stop-at-time 10 -r 30 \
  --hash-seed 31 --disable-auto-rotate -o ref.ppm
# split the P6 stream into PNG frames (e.g. ffmpeg -f image2pipe -c:v ppm -i ref.ppm f%04d.png)
```
Run the Rust build the same way (wgpu with a Mesa lavapipe or other software adapter). Compare every 30th frame with ImageMagick `compare -metric SSIM` (pass at >= ~0.9, tune), plus the section 2 dump as the primary gate. Reduce noise by disabling bloom/shadows when checking structure, and use the bundled font data/fonts/FreeSans.ttf. macOS has no llvmpipe, so use the dump as the gate there.

## Gaps
- Sequencing of multiple `[gource]` sections: src/main.cpp:73-105, src/gource_settings.cpp:618, src/core/conffile.cpp (not traced).
- Exporter dt and writeCustomLog line numbers marked `~` are approximate: src/gource.cpp:171-215, 245-262.
- SDL offscreen driver and Mesa behaviour unverified against docs.
- No tool in the repo currently runs test.conf or compares output; no CI config found.
