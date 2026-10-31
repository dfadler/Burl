# Build and goldens attempt

Outcome: autotools path BLOCKED (autoconf/automake/libtool missing). Direct clang++ build SUCCEEDED (retry, nothing installed); goldens in golden/.

## Environment
- macOS (Darwin 25.5.0), arm64 machine, Homebrew at /usr/local (Intel-prefix layout; /opt/homebrew absent).
- Compiler: /usr/bin/g++ and /usr/bin/clang++ (Apple clang), /usr/bin/make, /usr/bin/m4.
- `timeout` / `gtimeout`: not present (would need `brew install coreutils`, or background+kill).

## Dependencies
| Dependency | Status |
|---|---|
| sdl2 | 2.30.1 |
| SDL2_image | 2.8.2 |
| glew | 2.2.0 |
| freetype2 | 26.6.20 (libtool version) |
| libpcre2-8 | 10.47 |
| libpng | 1.6.59 |
| boost | 1.84.0_1 (/usr/local/opt/boost) |
| glm | 1.0.1 (headers at /usr/local/include/glm) |
| pkg-config | /usr/local/bin/pkg-config |
| autoconf | MISSING |
| automake | MISSING |
| libtool (glibtoolize) | MISSING |

The repo has no pre-generated `configure` (only autogen.sh and configure.ac), so autotools are required.

## Autotools route (not needed)
Running `./autogen.sh` would need `brew install autoconf automake libtool`. This route was not taken: the direct build below worked without installing anything.

## Direct build (no autotools) - worked
Key finding: Homebrew under /usr/local is x86_64 (pkg-config, SDL2, boost dylibs are all x86_64) while the shell is arm64. Without `-arch x86_64`, SDL.h pulls in x86 intrinsics and every file fails with `raointintrin.h: use of undeclared identifier '__builtin_ia32_aadd32'`. Rosetta works (`arch -x86_64 /usr/bin/true`). Fixed by `-arch x86_64`.

Build copy of src/ and data/ in a temp directory; the reproducible scripts are `golden/scripts/build.sh`, `link.sh`, `gold.sh`, `tbuild.sh` (see `golden/scripts/README.md`; they take `BUILD=` and default `REPO` to this checkout). The original session-scoped build directory is gone.
Per-file compile (53 files: src/main.cpp + Makefile.am `sources` + src/tinyxml/*.cpp, all compiled clean, zero failures):

    clang++ -arch x86_64 -std=c++17 -O1 -w -I src -I src/core -I src/tinyxml -DSDLAPP_RESOURCE_DIR=\"$S/data\" -DHAVE_TIMEGM -DHAVE_UNSETENV $(pkg-config --cflags sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -I/usr/local/include -I/usr/local/opt/boost/include -c FILE -o obj/FILE.o

Link:

    clang++ -arch x86_64 obj/*.o $(pkg-config --libs sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -L/usr/local/opt/boost/lib -lboost_filesystem -lboost_system -framework OpenGL -framework CoreFoundation -o gource

Result: x86_64 Mach-O binary (868 KB). Run via Rosetta transparently.

## Goldens (golden/, produced by golden script gold.sh with 20s background-kill timeout; stderr beside each; exit codes in golden/exit-codes.txt)
- help-short.txt: `gource -h` (rc 0); help-long.txt: `gource -H` (rc 0). Line 1 carries "Gource v0.57" (GOURCE_VERSION in src/gource_settings.h).
- `gource --version` is NOT a valid option: rc 1, stderr "unknown option version" (version.stderr.txt). Version comes from -h output.
- custom-log/{custom-dir-delete,file-removal,file-to-dir,utf8}.txt: `gource --log-format custom --output-custom-log - tests/logs/NAME.log` (rc 0).
- custom-log/svn-dir-delete.txt: same with `--log-format svn` (rc 0).
- utf8-caption.log: rc 1, "unsupported log format" with --log-format custom (it is a caption file, not a commit log; no golden). See custom-log/utf8-caption.stderr.txt.
- Window probe: `SDL_VIDEODRIVER=dummy gource --stop-at-time 1 -o - tests/logs/utf8.log` -> rc 1, stderr "SDL initialization failed - OpenGL support is either not configured in SDL or not available in current SDL video driver (dummy)". The dummy driver cannot supply GL, so a real window/GL context is required for rendering (probe-dummy.stderr.txt). Real-window run not attempted (would open a window).
- Note: --output-custom-log works without creating a window.
