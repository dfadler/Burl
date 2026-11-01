# Golden capture scripts

These rebuild the C++ gource without autotools and recapture every golden in `..` (help text,
exit codes, `--output-custom-log` per fixture, unit-test output). They were written on macOS
with Homebrew x86_64 libraries under `/usr/local` (hence `-arch x86_64`, run under Rosetta);
set `ARCH=` and `BREW=/opt/homebrew` on an arm64 Homebrew install. Nothing is installed.

Required: clang++, pkg-config, sdl2, SDL2_image, glew, freetype2, libpcre2-8, libpng, boost, glm.
See `../../build-and-goldens.md` for the versions that were present when the goldens were captured.

```bash
cd docs/rust-rewrite/discovery/golden/scripts
BUILD=$(mktemp -d "${TMPDIR:-/tmp}/gource-build.XXXXXX")
BUILD=$BUILD ./build.sh      # compiles every source in Makefile.am into $BUILD/obj
BUILD=$BUILD ./link.sh       # links $BUILD/gource
BUILD=$BUILD ./gold.sh       # recaptures ../help-*.txt, ../exit-codes.txt, ../custom-log/*
BUILD=$BUILD ./tbuild.sh     # builds and runs the Boost unit tests into ../unit-tests.txt
```

`glibc_rand.py` is the Python model of glibc's TYPE_3 `rand()` referenced by
`../../deps-and-rust-mapping.md`; it has not been checked against a real glibc.

To build the state-dump fork `reference/gource-cpp-dump/` (does not exist yet), run the same scripts with `REPO` pointing at that directory and a separate `BUILD`; do not re-run `gold.sh` against the fork's output to overwrite goldens. The oracle build (the untouched `src/`) is never modified, and the goldens stay the oracle's truth.
