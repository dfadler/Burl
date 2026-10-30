# Unit tests (Boost.Test)

Outcome: built and run, all pass (rc 0): 2 test cases, 69 assertions (parse_date_time_tests 58, regex_tests 11). Full log: golden/unit-tests.txt (ANSI stripped).

## Build (nothing installed; reuses gb/obj from build-and-goldens.md)
Build directory: a temp dir produced by `golden/scripts/build.sh`; the test build script is `golden/scripts/tbuild.sh` (takes `BUILD=`).
- src/test/main.cpp uses `boost/test/included/unit_test.hpp` (header-only), so -lboost_unit_test_framework is NOT needed.
- The test target links all `${sources}` but NOT src/main.cpp (the Boost header supplies main). So: all gb/obj/*.o except src_main.cpp.o.
- Compile each of the 3 test files (F = main, datetime_tests, regex_tests):
  `clang++ -arch x86_64 -std=c++17 -O1 -w -I src -I src/core -I src/tinyxml -I src/test -DSDLAPP_RESOURCE_DIR=\"$S/data\" -DHAVE_TIMEGM -DHAVE_UNSETENV $(pkg-config --cflags sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -I/usr/local/include -I/usr/local/opt/boost/include -c src/test/F.cpp -o tobj/F.o`
- Link: `clang++ -arch x86_64 tobj/*.o $(ls obj/*.o | grep -v src_main.cpp.o) $(pkg-config --libs sdl2 SDL2_image glew freetype2 libpcre2-8 libpng) -L/usr/local/opt/boost/lib -lboost_filesystem -lboost_system -framework OpenGL -framework CoreFoundation -o gource_tests`
- Run (no `timeout` cmd; background + kill): `./gource_tests --log_level=all --report_level=detailed > out 2>&1 & sleep 20; kill`. Finishes in ~26 ms; no window or GL context needed.

## regex_tests (all pass)
Regex("^[A-Z]+$"): isValid; match ABC true, abc false, ABC123 false; copy-constructed same; Regex("^[A-Z+$") throws RegexCompilationException; Regex("^[A-Z+$", true) (no-throw flag) gives isValid()==false; Regex("([0-9]+)").match("ABC123", &m) true, m.size()==1, m[0]=="123" (unanchored search, capture group 1 only).

## parseDateTime goldens
`SDLAppSettings::parseDateTime(str, time_t&)` returns true for every row; expected epoch below.

Quirk: the test does putenv TZ=UTC for the first row, then TZ=Pacific/Auckland (NZDT, +13 on 2021-11-01) for ALL later rows and never resets. Naive (no suffix) strings use the process-local TZ; `Z` and `+0` mean UTC; `+H` / `+H:MM` are explicit offsets. Auckland midnight 2021-11-01 = 1635678000. A port must set/emulate TZ per row.

| input | TZ in effect | expected epoch |
|---|---|---|
| `2021-11-01` | UTC | 1635724800 |
| `2021-11-01` | Pacific/Auckland | 1635678000 |
| `2021-11-01Z` | Pacific/Auckland | 1635724800 |
| `2021-11-01+0` | Pacific/Auckland | 1635724800 |
| `2021-11-01+13` | Pacific/Auckland | 1635678000 |
| `2021-11-01 +13` | Pacific/Auckland | 1635678000 |
| `2021-11-01+5:30` | Pacific/Auckland | 1635705000 |
| `2021-11-01 12:01` | Pacific/Auckland | 1635721260 |
| `2021-11-01 12:01Z` | Pacific/Auckland | 1635768060 |
| `2021-11-01 12:01+0` | Pacific/Auckland | 1635768060 |
| `2021-11-01 12:01+13` | Pacific/Auckland | 1635721260 |
| `2021-11-01 12:01 +13` | Pacific/Auckland | 1635721260 |
| `2021-11-01 12:01+5:30` | Pacific/Auckland | 1635748260 |
| `2021-11-01 12:01:59` | Pacific/Auckland | 1635721319 |
| `2021-11-01T12:01:59.123` | Pacific/Auckland | 1635721319 |
| `2021-11-01 12:01:59Z` | Pacific/Auckland | 1635768119 |
| `2021-11-01 12:01:59+0` | Pacific/Auckland | 1635768119 |
| `2021-11-01 12:01:59+13` | Pacific/Auckland | 1635721319 |
| `2021-11-01 12:01:59 +13` | Pacific/Auckland | 1635721319 |
| `2021-11-01 12:01:59+5:30` | Pacific/Auckland | 1635748319 |
| `2021-11-01T12:01:59` | Pacific/Auckland | 1635721319 |
| `2021-11-01T12:01:59.123` | Pacific/Auckland | 1635721319 |
| `2021-11-01T12:01:59Z` | Pacific/Auckland | 1635768119 |
| `2021-11-01T12:01:59+0` | Pacific/Auckland | 1635768119 |
| `2021-11-01T12:01:59+13` | Pacific/Auckland | 1635721319 |
| `2021-11-01T12:01:59+13` (duplicate in source) | Pacific/Auckland | 1635721319 |
| `2021-11-01T12:01:59+5:30` | Pacific/Auckland | 1635748319 |
| `2021-11-01T12:01:59.123+5:30` | Pacific/Auckland | 1635748319 |

Observations: fractional seconds are accepted and ignored; a space before the offset is allowed; space or `T` separates date and time; date-only, HH:MM and HH:MM:SS all work. No negative offsets, invalid input, or DST-gap cases are tested.

## Verification (tick 4)
All 28 parseDateTime rows checked programmatically against src/test/datetime_tests.cpp (regex-extracted inputs and expected epochs, in order): 28/28 inputs and epochs match verbatim. Source has 28 parseDateTime calls and 28 timestamp checks. Note: `2021-11-01T12:01:59.123` also appears twice (rows 15 and 22), besides the `+13` duplicate already noted. No corrections needed.
