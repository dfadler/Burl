# Gource: build, packaging, platforms, dependency -> Rust crate mapping

Scope: build system, packaging, platforms, C++ deps and Rust crate candidates. Citations are `path:line` in this repo. Crate facts (version, last-update, downloads) come from the crates.io JSON API (`https://crates.io/api/v1/crates/<name>`) queried 2026-10-29; feature claims come from docs.rs pages cited inline. Fetched content was treated as data.

## 1. Version, license, history

| Item | Value | Source |
|---|---|---|
| Version string | `0.57` (single source: `#define GOURCE_VERSION "0.57"`, printed as `Gource v%s` on the first line of `-h`/`-H`; there is NO `--version` option, it exits 1 with "unknown option" (golden/exit-codes.txt) (corrected tick 4)) | src/gource_settings.h:21, src/gource_settings.cpp:50 |
| Also duplicated in | `AC_INIT(Gource, 0.57, ...)`; `dev/bin/build_tar.pl` and `build_win64.pl` regex-scrape the header; build_tar dies if configure.ac/ChangeLog lack the version | configure.ac:6, dev/bin/build_tar.pl:15-22,87-94, dev/bin/build_win64.pl:17-22 |
| Latest ChangeLog entry | 0.57: "Handle loading indexed colour images"; 0.56 added `--author-time`, Boost 1.69 floor, compat GL profile | ChangeLog:1-9 |
| History | ~928 commits on this branch; ChangeLog back to older releases | `git log` |
| License | GPLv3 or later ("either version 3 of the License, or (at your option) any later version") | COPYING:1-2, README.md:480-489 |
| Contact/author | acaudwell@gmail.com | configure.ac:6 |

### License implications for the Rust port
- The port is a derivative (or at least a reimplementation of GPL code); keep it GPL-3.0-or-later. Every candidate crate's license string was verified against the crates.io API on 2026-10-29 (table in section 8.1); all are MIT / Apache-2.0 / Zlib / Unlicense / CC0 / MIT-0 / Unicode-3.0 combinations, i.e. GPLv3-compatible (Apache-2.0 is compatible with GPLv3 but NOT GPLv2-only, irrelevant here since Gource is GPLv3+). Re-run `cargo deny`/`cargo license` on the resolved tree when pinned (transitive deps are not covered by this check).
- Static linking of C libs (PCRE2 BSD, FreeType FTL/GPLv2, libpng zlib-style, SDL2 zlib) is fine under GPLv3. FreeType is dual FTL/GPLv2 (GPLv2-only is NOT compatible with GPLv3; use FTL: the FreeType site states the FTL is "compatible with the GNU Public License (GPL) version 3" but not v2, https://www.freetype.org/license.html) - applies to `freetype-rs`/`freetype-sys` if used.
- Bundled third-party code that needs notices carried over if its logic is ported or its files are shipped:
  - TinyXML 2.6.2, zlib license (src/tinyxml/tinyxml.h:1-20, version consts :92-94).
  - FTGL (Henry Maddocks) portions in src/core/fxfont.{h,cpp}:27-31 - the glyph atlas/texture-font logic is derived from FTGL (MIT-style); retain the notice if the algorithm is ported.
  - utf8-cpp (Nemanja Trifunovic, Boost Software License) at src/core/utf8/ (src/core/utf8/utf8.h:1-5). Rust has native UTF-8; no port needed, except Windows UTF-16 -> UTF-8 (`std::os::windows::ffi::OsStrExt` / `OsString::to_string_lossy`).
  - GNU FreeFont `FreeSans.ttf` (GPLv3+ with font-exception) in data/fonts/ (README at data/fonts/README); redistribute with the exception text.
- Windows installer shows an MIT-style "disclaimer" page (dev/nsis/disclaimer.txt) - not the GPL; installer also ships COPYING.txt (dev/bin/build_win64.pl:198-204).

## 2. Repository layout relevant to build

| Path | What | Notes |
|---|---|---|
| configure.ac | Autoconf 2.61+ script (`AC_PREREQ(2.61)`, `AM_INIT_AUTOMAKE([dist-bzip2 foreign subdir-objects])`) | configure.ac:4-9 |
| Makefile.am | Automake rules, source list, data install | Makefile.am:1-103 |
| autogen.sh | `autoreconf -f -i`, then `git submodule init/update --init --recursive` | autogen.sh:2-8 |
| m4/ | ax_boost_base, ax_boost_filesystem, ax_check_gl/glu/glut, ax_pthread, ax_save/restore_flags, pkg.m4 | m4/ |
| gource.pro | qmake project; Windows/MSYS2 mingw-w64 only (comment says so) | gource.pro:1-2 |
| INSTALL | Dependency list and options | INSTALL:10-76 |
| src/core | Git submodule `https://github.com/acaudwell/Core.git` (shared with Logstalgia): display, SDL app, fonts, shaders, regex, conffile, png/ppm/tga export, UI | .gitmodules, src/core/README:1 |
| src/tinyxml | Bundled TinyXML 2.6.2 (5 files) | see section 6 |
| src/test | Boost.Test: main.cpp, datetime_tests.cpp, regex_tests.cpp | Makefile.am:92-103 |
| data/ | Runtime assets (images, TGAs, shaders, fonts, style, man page) | section 5 |
| cmd/ | `gource` (sh) and `gource.cmd` wrappers, installed in `\cmd` on Windows and added to PATH | cmd/gource, cmd/gource.cmd |
| dev/bin | `build_tar.pl` (source tarball), `build_win64.pl` (NSIS installer + zip) | section 7 |
| dev/nsis | `disclaimer.txt`, `welcome.bmp`, `welcome.xcf` | |
| scripts/gource-ps.pl | Perl helper (VCS log helper; not built) | |
| contrib/svn-gource.py | Python helper for SVN (not built) | |
| resources/*.xcf | GIMP sources of file/user icons | |
| .github/ | ONLY `FUNDING.yml` (`ko_fi: acaudwell`). **No CI workflows exist.** | .github/FUNDING.yml:1 |
| .gitattributes | `*.cmd text eol=crlf` | |

Divergences between build files (porting traps):
- `src/core/tga.cpp` (TGAExporter, used with screenshots/`--output-*`?) is in gource.pro:84 but NOT in Makefile.am sources (Makefile.am:7-55); `src/core/timer.cpp` is in neither list (header-only use / dead). Autotools builds therefore have no TGA export path linked in.
- gource.pro bundles tinyxml unconditionally (gource.pro:61-64); Makefile.am conditional on `USE_BUNDLED_TINYXML` (Makefile.am:57-63).
- qmake links `-lboost_system-mt` (gource.pro:26) while autoconf/ChangeLog 0.56 dropped boost_system.

## 3. Build artefacts and flags

| Artefact | Rule |
|---|---|
| `gource` binary | `bin_PROGRAMS = gource`, sources `src/main.cpp` + list (Makefile.am:3,65) |
| `gource_tests` | `check_PROGRAMS`, Boost.Test (header-only `included/unit_test.hpp`, src/test/main.cpp:2), `TESTS = gource_tests` (Makefile.am:92-103) |
| Man page | `data/gource.1` gzipped to `$(mandir)/man1/gource.1.gz` by `install-data-hook` (Makefile.am:74-79) |
| Source tarball | `gource-<ver>.tar.gz` via `dev/bin/build_tar.pl` (inclusion/exclusion regex lists, dev/bin/build_tar.pl:24-75) into `dev/builds/` |
| Windows | `gource-<ver>.win64-setup.exe` and `gource-<ver>.win64.zip` (dev/bin/build_win64.pl:258-259) |

Compiler/flags: `-std=gnu++0x -Wall -Wno-sign-compare -Wno-reorder -Wno-unused-but-set-variable -Wno-unused-variable` (Makefile.am:5); qmake `c++11` (gource.pro:10); INSTALL says GCC 4.6+ or Clang (INSTALL:33-35). Debug qmake defines `-DASSERTS_ENABLED` (gource.pro:15). Windows: `-Dmain=SDL_main`, `-mconsole`, static libgcc/libstdc++, `-lcomdlg32` (gource.pro:19-28). INSTALL: "Gource expects SDL to have been built with the NO_STDIO_REDIRECT flag" (INSTALL:55).

configure checks (configure.ac): `AC_CHECK_FUNCS([timegm unsetenv])` (:17) -> `HAVE_TIMEGM`/`HAVE_UNSETENV` used in src/formats/svn.cpp:113,138,238 (Rust: `chrono`/`time` `Utc.from_utc_datetime`; no libc timegm needed). macOS: `with_x=no`, `LIBS += -framework CoreFoundation` (:19-26). SDL2_image must export `IMG_LoadPNG_RW` and `IMG_LoadJPG_RW` (:48-49). `AX_BOOST_BASE([1.69])` (:52). Custom macros: `--enable-font-file=FILE` -> `GOURCE_FONT_FILE` (:57-59, Makefile.am:81-83); `--enable-ttf-font-dir=DIR` -> `SDLAPP_FONT_DIR` (:62-64, Makefile.am:84-85); `--with-tinyxml` -> `HAVE_LIBTINYXML` (:70-76). `AM_CPPFLAGS = -DSDLAPP_RESOURCE_DIR="$(pkgdatadir)"` (Makefile.am:67). `HAVE_PTHREAD` is used in src/logmill.h:30 and logmill.cpp:68 (from AX_PTHREAD in m4; not invoked in configure.ac so effectively undefined in autotools builds).

## 4. C++ dependencies (version floor, users, features touched)

| Dependency | Floor | Source (declared) | Files using it | Features actually used |
|---|---|---|---|---|
| SDL2 | any 2.x (`pkg sdl2`); SDL1.2 code paths survive behind `SDL_VERSION_ATLEAST(2,0,0)` (41 occurrences, dead for 2.x) | configure.ac:39, INSTALL:15 | src/core/display.cpp, sdlapp.cpp, mousecursor.cpp, ppm.cpp, tga.cpp, png_writer.cpp, timer.cpp; src/main.cpp, gource.cpp, gource_shell.cpp, logmill.cpp, gource_settings.cpp, core/ui/* | Window/GL context: `SDL_CreateWindow`, `SDL_GL_CreateContext`, profile **compatibility** (display.cpp:254), `SDL_GL_SetAttribute` double buffer / depth / alpha 8 / multisample (display.cpp:187-196), `SDL_GL_SetSwapInterval` (:265-266), fullscreen (+DESKTOP), borderless, resizable, `ALLOW_HIGHDPI`, `SDL_GL_GetDrawableSize`, `SDL_GetDisplayBounds`, `SDL_WINDOWPOS_*`; events: key/mouse/wheel/text-input/window-resize/quit via `SDL_PollEvent` (single poll site); `SDL_GetKeyboardState` (modifier scancodes), relative mouse mode, `SDL_WarpMouseInWindow`, cursors (`SDL_CreateCursor` hidden 32x32 cursor, mousecursor.cpp:51); clipboard get/set (sdlapp.cpp:158,174); `SDL_SetHint(MINIMIZE_ON_FOCUS_LOSS,"0")` (main.cpp:174); timing `SDL_GetTicks` (37 uses), `SDL_Delay`; **threads/mutex/cond**: `SDL_CreateThread/WaitThread/CreateMutex/CreateCond/CondWait/CondSignal` for log reader (logmill.cpp:52,135,156), PPM exporter (ppm.cpp:55-61,91), PNG exporter (png_writer.cpp:134-150,210), TGA exporter (tga.cpp:222-238,298); Windows only: `SDL_GetWindowWMInfo` -> HWND subclass to suppress Alt-key "ding" (display.cpp:310-322) and frame-size math (:423-470), `SDL_LoadObject("USER32.DLL"/"SHCORE.DLL")` for DPI awareness (:567-572); X11 clipboard event filter declared (`USE_X11`, sdlapp.h:111-113); `SDL_Init(TIMER|VIDEO)` (display.cpp:590) |
| SDL2_image | any 2.x built with PNG **and JPEG** | configure.ac:39,45-46, INSTALL:16,28 | src/core/texture.{h,cpp} | Single call `IMG_Load(filename)` (texture.cpp:258) for PNG, JPEG, and TGA (bloom.tga / bloom_alpha.tga load through the same call, data/); indexed/palette surfaces converted to RGBA32 (texture.cpp:263-270, ChangeLog:2); user avatar dir accepts .jpg/.jpeg/.png only (gource_settings.cpp:843) |
| OpenGL (fixed function + GLSL 1.10) | GL 1.x baseline, GL 2.0 for shaders; falls back to fixed-function (`ffp`) when `!GLEW_VERSION_2_0` | configure.ac:31-34 (AX_CHECK_GL) | all rendering: dirnode.cpp, user.cpp, file.cpp, gource.cpp, bloom.cpp, textbox.cpp, slider.cpp, key.cpp, caption.cpp, core/* | Immediate mode: `glBegin/End` (29 sites), `glVertex2f` (85), `glTexCoord2f` (52), `glColor4f`, matrix stack (`glMatrixMode`, `glPushMatrix`, `glTranslatef`, `glOrtho`, `glLoadIdentity`), `glLineWidth`, `glPolygonMode`, display lists (`glNewList`, `glGenLists`, `glCallList`), `glLoadName`, client arrays (`glVertexPointer`, `glColorPointer`, `glTexCoordPointer`) + VBOs (`glGenBuffers/BufferData/BufferSubData/DrawArrays`, core/vbo.cpp), `glCopyTexImage2D` (bloom), `glReadPixels` (screenshots/video), `glScissor`, blend modes, **GL timer queries** (`glGenQueries`, `glGetQueryObjectui64v`), `glGetDoublev` matrices for projection. Shaders use legacy built-ins (`gl_TexCoord`, `gl_Color`, `ftransform()`, `varying`): data/shaders/*.vert, *.frag. Shader preprocessing (`#version`, `#extension`, `#include`, uniform injection via boost::format) in core/shader_common.cpp. |
| GLU | any (from AX_CHECK_GLU) | configure.ac:32 | dirnode.cpp:949,954; zoomcamera.cpp:75; file.cpp:274; user.cpp:376; core/texture.cpp:232; core/display.cpp:691,738,759; core/gl.h:11 | `gluPerspective`, `gluLookAt`, `gluProject`, `gluUnProject` (screen<->world picking and label placement), `gluBuild2DMipmaps`, `gluErrorString`. All trivially reimplemented with glam. |
| GLEW | any (`pkg glew`) | configure.ac:38, INSTALL:19 | src/core/gl.h:4, display.cpp:136-150, gource.cpp:33,98,800, gource_shell.cpp:42 | `glewInit` (experimental flag option), tolerate `GLEW_ERROR_NO_GLX_DISPLAY` for Wayland (display.cpp:140-142); runtime caps checks `GLEW_VERSION_2_0`, `GLEW_ARB_texture_non_power_of_two`; `ARB` program-object entry points (`glUseProgramObjectARB`, 5 uses) |
| FreeType 2 | `freetype2 >= 9.0.3` (libtool version) | configure.ac:36 | src/core/fxfont.{h,cpp}, ftgl/FTUnicode.h | `FT_Init_FreeType`, `FT_New_Face`, `FT_Set_Char_Size(size, dpi)`, `FT_Get_Char_Index`, `FT_Load_Glyph` with flags (default `FT_LOAD_DEFAULT`, i.e. hinting on), `FT_Get_Glyph`, `FT_Glyph_To_Bitmap(FT_RENDER_MODE_NORMAL)` (8-bit AA, fxfont.cpp:81), glyph metrics (advance/bearing/bbox), kerning via FT; glyph bitmaps packed into GL textures (FTGL-derived texture-font logic, fxfont.cpp:148). Fonts loaded from a filesystem path (`FreeSans.ttf` default, gource_settings.cpp:35-36, 446). Text layout, width measurement and shadows done in fxfont.cpp. |
| PCRE2 | `libpcre2-8` (8-bit), no min | configure.ac:37 | src/core/regex.{h,cpp} (only direct user); wrapper `Regex` used by core/conffile.cpp:23-34, core/shader_common.cpp (9 regexes), formats/*.cpp (e.g. svn.cpp:29-30), gource.cpp:1080, gource_settings.cpp:743,1527-1596 | `pcre2_compile` with **flags=0** (no UTF, no UCP, no CASELESS; regex.cpp:35-42), `pcre2_match` from byte offset, ovector captures, `pcre2_code_copy`; no JIT, no named groups used. User-supplied patterns for `--file-filter`, `--file-show-filter`, `--user-filter`, `--user-show-filter` go straight to PCRE2 (gource_settings.cpp:1527-1596, 4 call sites, compile errors reported as conf errors via `Regex(str, test=true)`). Internal patterns use **negative look-ahead** `(?!-)` (gource_settings.cpp:743) and **byte escapes** `\xEF\xBB\xBF` (BOM, gource.cpp:1080), lazy/greedy classes `\s \d \w`. |
| Boost.Filesystem | `>= 1.69` (header + lib; boost_system no longer linked since 0.56) | configure.ac:52-53, INSTALL:21, ChangeLog:5-6 | src/logmill.{h,cpp}:106-208, gource_settings.cpp:816-997,1663, core/resource.cpp:55-59, core/ui/file_selector.cpp (many) | `exists`, `is_directory`, `is_regular_file`, `canonical` (font path, gource_settings.cpp:997), `directory_iterator` + `filename()`/`extension()`/`parent_path()`/`operator/`, `filesystem_error`; Windows `wstring()` filename + utf16->utf8 (gource_settings.cpp:834-838). |
| Boost header-only extras | (part of Boost >= 1.69) | configure.ac:52 | `boost/format.hpp` (shader_common.cpp ~50 uses; shader.cpp; formats/hg.cpp:34-47, bzr.cpp:33-35, svn.cpp:37-44 build the VCS command lines; gource_settings.cpp:1664), `boost/algorithm/string.hpp` (`to_lower`, `ilexicographical_compare`, file_selector.cpp:43-49,211; gource_settings.cpp:841), `boost/assign/list_of.hpp` (static maps, core/logger.cpp:35, core/ui/element.cpp:5), Boost.Test (src/test/*) | All replaced by `format!`, `str::to_lowercase`, array/`HashMap::from`, `#[test]`. Note `%e` in boost::format for GLSL float literals: formatting parity matters only for generated shader text, not simulation. |
| GLM | `>= 0.9.3` (header check `glm/glm.hpp`) | configure.ac:65, INSTALL:20 | src/core/vectors.h:37-48 (only include site) | `vec2/3/4`, `mat3/4`, `glm::length`, `length2` (gtx/norm), `min/max/floor`, `value_ptr`, `translate/scale/rotate` (gtc/matrix_transform), matrix column access (gtc/matrix_access). Wrapped by core/vectors.h (`normalise`, `lerp2`...). |
| libpng | `>= 1.2` | configure.ac:40, INSTALL:22 | src/core/png_writer.{h,cpp}, gource.cpp:19,2237 | **Encode only**: `png_create_write_struct`, `png_set_write_fn` to `std::ostream`, `png_set_IHDR` 8-bit RGB/RGBA, non-interlaced, `png_write_image` with rows flipped (bottom-up GL readback, png_writer.cpp:109-118). Used by `--output-ppm-stream`-adjacent screenshot code (`pngname`, gource.cpp:2237). PNG **decoding** goes through SDL2_image, not libpng. |
| TinyXML | bundled 2.6.2; system `libtinyxml` optional | configure.ac:67-74, INSTALL:26,72-76 | src/formats/svn.cpp:23-27,206-263 and src/formats/cvs2cl.cpp:22-24,96-147 | See section 6. |
| POSIX/Win32 | n/a | | commitlog.cpp (`system()`, `mkstemp`, `/tmp/gource-XXXXXX`, `GetTempPath`, `SearchPath`), seeklog.cpp (`fcntl(O_NONBLOCK)` on stdin, `PeekNamedPipe` on Windows), timezone.cpp (`setenv/unsetenv TZ`), main.cpp, logmill.cpp | VCS integration is by spawning shell commands (`git log ...`, `svn log --xml`, `hg log --style`, `bzr log`, cvs) via `system()` into a temp file (commitlog.cpp:94 and :257-290); Rust: `std::process::Command` with piped stdout (but preserve shell-quoting semantics since format strings embed `--date '...'` quoting, hg.cpp:34-47). |

## 5. Install layout and runtime resource lookup

| Item | Detail | Source |
|---|---|---|
| Prefix data dir | `$(pkgdatadir)` = `<prefix>/share/gource` (compiled in as `SDLAPP_RESOURCE_DIR`) | Makefile.am:67 |
| Data files | `beam.png bloom.tga bloom_alpha.tga file.png user.png gource.style` | Makefile.am:69 |
| Shaders | `<pkgdatadir>/shaders/{shadow,bloom,text}.{vert,frag}` | Makefile.am:71-72 |
| Fonts | `<pkgdatadir>/fonts/{README,FreeSans.ttf}`; skipped if `--enable-font-file` / `--enable-ttf-font-dir` | Makefile.am:81-90 |
| Man page | `<mandir>/man1/gource.1.gz` | Makefile.am:74-79 |
| Binary | `<bindir>/gource` | Makefile.am:3 |
| Other unrelated data | `data/gource.style` is the Mercurial template file; `hg` format passes its path (`gource_style_path`, hg.cpp:47) | |
| Lookup order (non-Windows) | defaults `data/`, `data/fonts/`, `data/shaders/` relative to cwd (sdlapp.cpp:204-211); if `SDLAPP_RESOURCE_DIR` exists use it, else directory of the executable + `data/` (sdlapp.cpp:221-236); `SDLAPP_FONT_DIR` overrides fonts (:240-244); `SDLAPP_CONF_DIR` optional (:216-219) | src/core/sdlapp.cpp |
| Lookup (Windows) | exe dir (`GetModuleFileName`) + `data\`, `data\fonts\`, `data\shaders\` (sdlapp.cpp:190-201) | src/core/sdlapp.cpp |
| Rust implication | Embed (via `include_bytes!`/`rust-embed`) or install under `<prefix>/share/gource` and mimic the same fallback chain; shaders are legacy GLSL 1.10 and will be rewritten (rendering only needs to be visually equivalent). Retain `gource.style` byte-for-byte (hg parsing depends on it) and the font (FreeSans.ttf, 697.7K). |

## 6. Bundled TinyXML

- Files: `src/tinyxml/{tinystr.cpp,tinystr.h,tinyxml.cpp,tinyxml.h,tinyxmlerror.cpp,tinyxmlparser.cpp}`, version 2.6.2 (tinyxml.h:92-94), zlib license (tinyxml.h:3-20).
- Selection: `#ifdef HAVE_LIBTINYXML #include <tinyxml.h> #else #include "../tinyxml/tinyxml.h"` (src/formats/svn.cpp:23-27, cvs2cl.cpp:22-26). configure: `--with-tinyxml` -> `AC_CHECK_LIB([tinyxml])`, `AM_CONDITIONAL(USE_BUNDLED_TINYXML ...)` (configure.ac:70-78).
- Consumers (only two formats):
  - **SVN** `svn log --xml --verbose --quiet`: each `<logentry>` is assembled line-by-line (regex `^<logentry`) then parsed: `doc.Parse`, `FirstChildElement("logentry")`, `<date>` (ISO 8601 UTC, parsed with timegm), `<author>`, `<paths>/<path action=...>` (svn.cpp:206-263).
  - **cvs2cl** XML (`<entry>` with `<isoDate>`, `<author>`, `<file><name>/<cvsstate>`) (cvs2cl.cpp:96-147).
- Usage is DOM-style navigation of small fragments (one commit at a time) - no namespaces, no writing, no DTD. Rust: `roxmltree` fits (read-only DOM, small input) and is simplest; `quick-xml` for streaming. Behaviour risk: TinyXML is lenient about malformed/partial XML and entity handling (it tolerates `&` oddities); strict parsers (roxmltree/xml-rs) will error. Port should keep the error-tolerant path (skip bad entry, as C++ does on parse failure) and add golden tests from svn/cvs samples.

## 7. Packaging

### Windows (dev/bin/build_win64.pl)
- Prereqs: MSYS2 64-bit build in `../build-gource-Msys2_64_bit-Release/release/gource.exe`, `cygcheck` to find DLLs (`grep msys`), NSIS at `C:\Program Files (x86)\NSIS\makensis.exe` with long-string build (`NSIS_MAX_STRLEN=8192` asserted), `zip`, plugins `SafeEnvVarUpdate.nsh`, `MultiUser.nsh`, `MUI2.nsh`, `x64.nsh`, `LogicLib.nsh` (build_win64.pl:10-15,54,80-83,304).
- Installer: 64-bit only (aborts otherwise, :107-110), MultiUser (per-user or all-users; default dir name `Gource`, registry `Software\Gource`; "Install_Mode" / "Install_Dir" values, :70-78), uninstalls previous version if present (:112-122), writes Add/Remove Programs entry (DisplayName, DisplayVersion, Publisher `acaudwell`, UninstallString, NoModify/NoRepair, :141-147), optional section "Add to PATH" appends `$INSTDIR\cmd` to HKLM/HKCU PATH (:151-159) - that is why `cmd/gource.cmd` and `cmd/gource` (calling `..\gource.exe`) exist. Welcome bitmap `dev/nsis/welcome.bmp`, disclaimer page `dev/nsis/disclaimer.txt`.
- Installed contents: `gource.exe` + all required MSYS2 DLLs; `data/{beam.png,file.png,user.png,bloom.tga,bloom_alpha.tga,gource.style}`, `data/fonts/FreeSans.ttf`, `data/shaders/{bloom,shadow,text}.{vert,frag}`, `cmd/gource.cmd`, `cmd/gource`; text files converted to CRLF `.txt`: README.txt, ChangeLog.txt, data/fonts/README->`data/fonts/README.txt`, COPYING.txt, THANKS.txt (:180-204,248-254). Output names: `gource-<ver>.win64-setup.exe` and `gource-<ver>.win64.zip`.
- Notes for Rust: a statically linked MSVC/GNU binary removes the DLL harvesting step; the NSIS script can be reused with the file list updated (data may be embedded). Gource's Windows-specific behaviours to preserve: console attach/resize (`SDLApp::showConsole/resizeConsole`, gource_settings.cpp:44-47,201-203), "Press Enter" when launched without an existing console, `PeekNamedPipe` stdin polling (gource_settings.cpp:1641-1650, seeklog.cpp:44-70), `./gource.exe` default path check (logmill.cpp:114-116), `WM`-level Alt-key suppression, DPI awareness (`high-dpi` option hidden on Windows, gource_settings.cpp:59-61).

### Linux / source tarball
- `dev/bin/build_tar.pl`: verifies version in configure.ac and ChangeLog, runs `make distclean` + `autoreconf -f -i -v`, copies whitelisted files (src/*.cpp/h, data, m4, configure, build-aux scripts...), excludes cmd/ resources/ tests/ scripts/ contrib/ dev/, tars to `/var/tmp/gource-<ver>.tar.gz` then moves to `dev/builds/`. Note: `data/cursor.png` is whitelisted (build_tar.pl:66) but no longer exists in data/.
- Distro packages (package names in INSTALL:15-26): libsdl2-dev libsdl2-image-dev libpcre2-dev libfreetype6-dev libglew-dev libglm-dev libboost-filesystem-dev libpng-dev libtinyxml-dev.

### macOS specifics
- No `.app` bundle/dmg tooling in this repo (distributed through Homebrew/MacPorts, not shown here).
- Build: `with_x=no`; link `-framework CoreFoundation` (configure.ac:19-26). qmake project is Windows-only.
- Code: default window height 640 instead of 768 (core/settings.cpp:69-71); `toggleFrameless` unsupported (display.cpp:403-406); window sizes are points not pixels, `--high-dpi` (README.md:43, ChangeLog 0.53 Retina fixes); filename NFC normalisation via CoreFoundation for avatar image names (HFS+ stores NFD) (gource_settings.cpp:608-610,848-858) - Rust: `unicode-normalization::UnicodeNormalization::nfc` replaces it; compat GL profile requested (display.cpp:254; on macOS this yields a GL 2.1 legacy context, so legacy immediate-mode GL stops at GL 2.1/4.1 deprecated - one more reason to redraw with glow/wgpu and Metal); Apple M1 autoconf macro fix (ChangeLog 0.54).

### Linux specifics
- X11 and experimental Wayland (ChangeLog 0.54); GLEW tolerates `GLEW_ERROR_NO_GLX_DISPLAY` (display.cpp:140-142); X11 clipboard event filter code gated by `USE_X11` (sdlapp.h:111-113).

### CI
- **None in-repo.** `.github/` contains only `FUNDING.yml`. No Dockerfile, no tox/Makefile CI, no Travis/AppVeyor. The `tests/` dir holds sample logs/images/test.conf (see sibling discovery doc `tests-and-golden-strategy.md`). New Rust port should add: `cargo fmt/clippy/test`, a matrix (ubuntu, macos, windows), and golden-log parse tests; cross-platform release job to produce the zip/NSIS installer.

## 8. Crate candidates

Data from crates.io API, 2026-10-29: version / last update / recent (90-day) downloads.

| Need | Candidate | Version / updated | Pros | Cons / risk |
|---|---|---|---|---|
| **Windowing + GL context + input (SDL2 role)** | A. `winit` 0.30.13 (2026-10-24, 14.4M recent) https://crates.io/crates/winit + `glutin` 0.32.3 (2025-04-30) + `glutin-winit` 0.5.0 (2024-06-08) https://crates.io/crates/glutin | Pure Rust, active; Wayland/X11/Win/mac; high-DPI; text input/IME events; relative mouse via DeviceEvent | glutin/glutin-winit updates slower; multisample/swap-interval via `ConfigTemplateBuilder`; pairing of versions fiddly; no SDL-style clipboard (add `arboard`); no threads/cond (use `std`); fullscreen "DESKTOP" modes slightly different |
| | B. `sdl2` 0.38.0 (2025-07-12, 0.8M recent) https://crates.io/crates/sdl2 | 1:1 API with current code (events, GL attributes incl. compat profile, clipboard, cursors, text input, `video().gl_set_swap_interval`) ; lowest behaviour risk for keyboard/mouse semantics; `bundled`/`static-link` features | Needs SDL2 C lib (or bundled build requires cmake); not pure Rust; SDL2 is in maintenance as SDL3 supersedes |
| | C. `sdl3` 0.20.0 (2026-09-07) https://crates.io/crates/sdl3 | SDL3 forward path | Younger crate (0.2M downloads), API still moving |
| **Rendering** | A. `glow` 0.18.0 (2026-07-09, 18M recent) https://crates.io/crates/glow | Thin GL/GLES wrapper; runs the same fixed pipeline concepts; works with glutin or SDL | No fixed-function (must rewrite immediate mode as batched VBOs + GLSL 330/ES); legacy compat profile unsupported on macOS 3.2+ core only |
| | B. `wgpu` 30.0.1 (2026-08-22, 15.6M recent) https://crates.io/crates/wgpu | Metal/Vulkan/DX12 modern; no GL driver quirks; visually-equivalent goal permits it | Large rewrite of bloom/shadow/text shaders to WGSL; MSAA/line-width (`glLineWidth`, `glPolygonMode` lines) need geometry-based lines; heavier binary |
| | C. `gl` 0.14.0 (2019) / `glium` 0.36.0 (2024-10) | Generated raw bindings (`gl`, stale) | `gl` is unmaintained since 2019; `glium` is high-level, slower updates; prefer glow |
| **Math (GLM role)** | A. `glam` 0.34.1 (2026-10-06, 58.6M recent) https://crates.io/crates/glam | API shape closest to GLM (`Vec2/3/4`, `Mat3/4`, `length`, `length_squared`, `look_at_rh`, `perspective_rh_gl`, `project/unproject` manual) | f32 only by default, matches C++ float semantics; `gluProject/gluUnProject` need hand-written (small) |
| | B. `nalgebra` 0.35.0 (2026-05-24) https://crates.io/crates/nalgebra | Rich, has `Perspective3`, `unproject_point` | Heavy; more generic API; slower compile |
| | C. `cgmath` 0.18.0 (2021) | Closest to old GLM style | Unmaintained since 2021 - avoid |
| **Image decode (SDL2_image role)** | A. `image` 0.25.10 (2026-03-10) https://crates.io/crates/image with features `png`,`jpeg`,`tga` | One call replaces `IMG_Load`; indexed PNG expands via `to_rgba8()` (matches 0.57 fix); TGA covers bloom.tga | Orientation/channel order differences vs SDL surfaces (bottom-up TGA handled by crate); JPEG decoder (zune-jpeg) output may differ by +-1 LSB from libjpeg - fine for avatars |
| | B. `png` 0.18.1 (2026-02-14) https://crates.io/crates/png + `zune-jpeg` | Smaller deps | Need separate TGA loader (write 30 lines) |
| **Image encode (libpng role)** | `png` 0.18.1 (also used by `image`) https://crates.io/crates/png | `Encoder::new(Write, w, h)` + `set_color(Rgb/Rgba)`, 8-bit, non-interlaced writes to any `Write`; trivial | PNG byte output differs from libpng (filter/compression choices) - pixel-identical decode, not byte-identical file (acceptable; no byte-exact requirement stated for screenshots) |
| **Font rasteriser (FreeType role)** | A. `fontdue` 0.9.5 (2026-10-19, 3.8M recent) https://crates.io/crates/fontdue | Pure Rust; fast raster; per-glyph coverage bitmap + metrics + kerning; easy to build the same glyph atlas | **No hinting**: glyph shapes/advance differ from FT `FT_LOAD_DEFAULT` hinted rendering - small visual differences, text width measurement will drift by fractions of pixels (affects label placement slightly, not simulation state) |
| | B. `ab_glyph` 0.2.32 (2025-09-28) https://crates.io/crates/ab_glyph | Pure Rust; rasterises outlines (coverage callback); has scale/kerning; OTF/TTF; very popular (13M recent) | Also unhinted; slightly slower than fontdue; API for metrics at px scale straightforward |
| | C. `freetype-rs` 0.38.0 (2025-01-08) https://crates.io/crates/freetype-rs / `freetype-sys` 0.23.0 | Same library -> **identical glyph output** (hinting, `FT_Set_Char_Size` at dpi); lowest visual risk | C dependency (build.rs vendored cmake option); FT license GPLv2/FTL (use FTL); bindings less actively maintained |
| | (shaping libs) `swash` 0.2.10, `cosmic-text` 0.19.0 | Complex-script shaping/layout | Overkill: Gource does per-codepoint layout, no shaping (fxfont.cpp:64-99 `FT_Get_Char_Index` per char) |
| **Regex (PCRE2 role)** | A. `pcre2` 0.2.11 (2025-09-25) https://crates.io/crates/pcre2 (with `pcre2-sys` 0.2.10) | **Exact semantic parity** with C++ (same engine incl. look-ahead, byte escapes, backtracking, leftmost-first, `\w` ASCII-only in non-UCP mode); safe to use for user-supplied `--file-filter` etc. | Needs libpcre2 or build via `pcre2-sys` vendored; maintained by BurntSushi (low churn); non-Rust dep; `Regex::new` via `RegexBuilder` can set `utf`/`ucp` off to match flags=0 |
| | B. `fancy-regex` 0.20.0 (2026-10-24, 90M recent) https://crates.io/crates/fancy-regex | Pure Rust; supports look-ahead `(?=)`, `(?!)`, backrefs; docs: https://docs.rs/fancy-regex/latest/fancy_regex/ | `\w \d \s` Unicode by default (differs from PCRE2 flags=0 ASCII-only); haystack is `&str`/UTF-8 so BOM `\xEF\xBB\xBF` pattern must be rewritten as `\u{FEFF}` and non-UTF-8 log bytes need lossy conversion; syntax gaps (e.g. possessive quantifiers, recursion, some escapes); different backtracking-limit behaviour |
| | C. `regex` 1.13.1 (2026-07-15) https://crates.io/crates/regex ; docs https://docs.rs/regex/latest/regex/ | Fast, safe; `regex::bytes` + `(?-u)` gives ASCII `\w` and byte matching | **No look-around or backrefs** (docs state so) -> `(?!-)` branch regex (gource_settings.cpp:743) must be rewritten (check first char separately); user filter patterns that use lookaheads/backrefs would now fail to compile - observable CLI/behaviour change; Unicode `\w` default |
| | Recommendation | Use `pcre2` (or `fancy-regex` with `.unicode(false)`-equivalent pattern prefixes) for **user-supplied filters** to keep config/CLI compatibility; internal fixed patterns (conffile, format line parsers) can be hand-written parsers or `regex::bytes` for speed. Add differential tests (C++ binary vs Rust) on the 4 filter options. |
| **XML (TinyXML role)** | A. `roxmltree` 0.21.2 (2026-10-24, 33M recent) https://crates.io/crates/roxmltree | Read-only DOM, tiny API, fits per-commit fragments | Strict: malformed XML errors; zero-copy borrow lifetimes |
| | B. `quick-xml` 0.42.0 (2026-08-22, 154M recent) https://crates.io/crates/quick-xml | Streaming, fastest; could parse whole `svn log --xml` incrementally and drop line-grouping hack | Event API verbose; need own tree handling |
| | C. `xml-rs` 1.0.0 (2026-08-06) / `xmltree` 0.12.0 | Simple | Slower, xmltree lighter maintenance |
| **Paths/FS (Boost.Filesystem role)** | `std::path` + `std::fs` (no crate) ; optional `walkdir` 2.5.0 (2024-03-01) https://crates.io/crates/walkdir ; `camino` 1.2.6 for UTF-8 paths | `exists/is_dir/is_file/canonicalize/read_dir/extension/file_name/join` all in std | `fs::canonicalize` on Windows returns `\\?\` UNC paths (strip with `dunce` crate); Boost `parent_path()` of `"a/b/"` differs from std (`file_selector.cpp:135` comment about trailing slash) - port that logic explicitly; `OsString` vs UTF-8: Windows wide-char handling (gource_settings.cpp:834-838) replaced by `to_string_lossy` |
| **Dates/timezones (timegm/TZ role)** | A. `chrono` 0.4.45 (2026-06-04) https://crates.io/crates/chrono (+ `chrono-tz` 0.10.4 https://crates.io/crates/chrono-tz) | Mature; strptime-like parsing | Locale/`TZ`-env behaviour of `mktime`/`localtime` differs; C++ `setenv("TZ")` hack (timezone.cpp) is replaced by explicit tz |
| | B. `jiff` 0.2.38 (2026-10-06) https://crates.io/crates/jiff | Modern tz-aware, uses system tzdb, good ISO 8601 parsing | Newer API |
| | C. `time` 0.3.55 https://crates.io/crates/time | Small, no tz database (needs `time-tz`) | Local offset lookup is restricted in multi-threaded programs |
| **CLI** | `clap` 4.6.7 (2026-09-14) https://crates.io/crates/clap or `lexopt` 0.3.2 | Gource options like `-1280x720`, `-WIDTHxHEIGHT`, `--key`, repeated flags and a `[path]` positional, plus custom conf-file/CLI duplication (`ConfFile`, arg_types map) need exact match; `clap` handles long/short; the `-WIDTHxHEIGHT` form needs a custom pre-pass | clap error text differs from C++ help; keep hand-written `help()` output verbatim (gource_settings.cpp:41+) for API compatibility; `lexopt` gives full control, lower risk |
| **Threads** | `std::thread` + `std::sync::{Mutex,Condvar,mpsc}`; optionally `crossbeam-channel` 0.5.17 | Replaces `SDL_CreateThread/mutex/cond` 1:1 | none |

### 8.1 Verified license / freshness table (tick 2)

Source for every row: `https://crates.io/api/v1/crates/<name>` (fields `max_stable_version`, `updated_at`, latest version's `license`, `recent_downloads`), queried 2026-10-29. All versions/dates in the table above matched the API (no numeric corrections needed). Corrections/additions: `glium` last update is 2024-10-11; `glutin` license is Apache-2.0 only and `glutin-winit` MIT; `ab_glyph` is Apache-2.0 only; `freetype-rs`/`freetype-sys` are MIT wrappers (the linked FreeType C library is FTL/GPLv2, use FTL); `walkdir` is Unlicense/MIT. Newly checked helper crates: `zune-jpeg` 0.5.15, `dunce` 1.0.5, `arboard` 3.7.0, `rust-embed` 8.13.0, `unicode-normalization` 0.1.26.

| Crate | Latest | Updated | License (crates.io) | GPLv3-compatible | Maintained? |
|---|---|---|---|---|---|
| winit | 0.30.13 | 2026-10-24 | Apache-2.0 | yes | yes |
| glutin | 0.32.3 | 2025-04-30 | Apache-2.0 | yes | slow (6 months) |
| glutin-winit | 0.5.0 | 2024-06-08 | MIT | yes | stale-ish (2+ years) |
| sdl2 | 0.38.0 | 2025-07-12 | MIT | yes | yes, slow |
| sdl3 | 0.20.0 | 2026-09-07 | MIT | yes | yes, pre-1.0 |
| glow | 0.18.0 | 2026-07-09 | MIT OR Apache-2.0 OR Zlib | yes | yes |
| wgpu | 30.0.1 | 2026-08-22 | MIT OR Apache-2.0 | yes | yes |
| gl | 0.14.0 | 2019-10-03 | Apache-2.0 | yes | no (unmaintained) |
| glium | 0.36.0 | 2024-10-11 | Apache-2.0 | yes | slow |
| glam | 0.34.1 | 2026-10-06 | MIT OR Apache-2.0 | yes | yes |
| nalgebra | 0.35.0 | 2026-05-24 | Apache-2.0 | yes | yes |
| cgmath | 0.18.0 | 2021-01-03 | Apache-2.0 | yes | no |
| image | 0.25.10 | 2026-03-10 | MIT OR Apache-2.0 | yes | yes |
| png | 0.18.1 | 2026-02-14 | MIT OR Apache-2.0 | yes | yes |
| zune-jpeg | 0.5.15 | 2026-10-19 | MIT OR Apache-2.0 OR Zlib | yes | yes |
| fontdue | 0.9.5 | 2026-10-19 | MIT OR Apache-2.0 OR Zlib | yes | yes |
| ab_glyph | 0.2.32 | 2025-09-28 | Apache-2.0 | yes | yes, low churn |
| freetype-rs | 0.38.0 | 2025-01-08 | MIT (+FreeType FTL) | yes (via FTL) | slow |
| freetype-sys | 0.23.0 | 2024-10-20 | MIT (+FreeType FTL) | yes (via FTL) | slow |
| swash | 0.2.10 | 2026-07-17 | Apache-2.0 OR MIT | yes | yes |
| cosmic-text | 0.19.0 | 2026-04-22 | MIT OR Apache-2.0 | yes | yes |
| pcre2 / pcre2-sys | 0.2.11 / 0.2.10 | 2025-09-25 / 09-21 | Unlicense OR MIT | yes | yes, low churn (PCRE2 C lib itself is BSD) |
| fancy-regex | 0.20.0 | 2026-10-24 | MIT | yes | yes |
| regex | 1.13.1 | 2026-07-15 | MIT OR Apache-2.0 | yes | yes |
| roxmltree | 0.21.2 | 2026-10-24 | MIT OR Apache-2.0 | yes | yes |
| quick-xml | 0.42.0 | 2026-08-22 | MIT | yes | yes |
| xml-rs | 1.0.0 | 2026-08-06 | MIT | yes | yes |
| xmltree | 0.12.0 | 2025-11-10 | MIT | yes | slow |
| walkdir | 2.5.0 | 2024-03-01 | Unlicense/MIT | yes | feature-complete, stable |
| camino | 1.2.6 | 2026-09-15 | MIT OR Apache-2.0 | yes | yes |
| dunce | 1.0.5 | 2024-08-04 | CC0-1.0 OR MIT-0 OR Apache-2.0 | yes | stable |
| chrono | 0.4.45 | 2026-06-04 | MIT OR Apache-2.0 | yes | yes |
| chrono-tz | 0.10.4 | 2025-07-11 | MIT OR Apache-2.0 | yes | yes |
| jiff | 0.2.38 | 2026-10-06 | Unlicense OR MIT | yes | yes |
| time | 0.3.55 | 2026-08-01 | MIT OR Apache-2.0 | yes | yes |
| clap | 4.6.7 | 2026-09-14 | MIT OR Apache-2.0 | yes | yes |
| lexopt | 0.3.2 | 2026-02-28 | MIT | yes | yes, tiny/stable |
| crossbeam-channel | 0.5.17 | 2026-09-05 | MIT OR Apache-2.0 | yes | yes |
| arboard | 3.7.0 | 2026-10-15 | MIT OR Apache-2.0 | yes | yes |
| rust-embed | 8.13.0 | 2026-10-07 | MIT | yes | yes |
| unicode-normalization | 0.1.26 | 2026-10-16 | (MIT OR Apache-2.0) AND Unicode-3.0 | yes | yes |

"Maintained" is judged only from last-release date and download volume on crates.io, not from issue-tracker activity. No crate was incompatible. Caveat: `sdl2`'s `bundled`/`static-link` and `pcre2-sys`'s vendored build pull in SDL2 (zlib) and PCRE2 (BSD) C code, also compatible.

### Cross-cutting risk summary
1. **PCRE2 vs regex semantics** - highest compatibility risk (user-supplied filter patterns are public CLI/config API). See rows above.
2. **Font hinting** - FreeType hinted vs unhinted rasterisers change text pixels/advance; acceptable under "visually equivalent", but pick FreeType bindings if label metrics must match.
3. **GL fixed-function** - macOS and modern drivers; Rust renderer cannot reuse immediate mode; plan rewrite (VBO batches) and reimplement `gluProject`-based text placement (`file.cpp:274`, `user.cpp:376`, `dirnode.cpp:949`) in glam so screen-space labels remain consistent with simulation.
4. **TinyXML leniency** vs strict XML crates.
5. **Windows**: HWND-level hacks (alt-key beep, frame-size math, console handling) have no direct winit equivalent; need `windows-sys`/winit extensions or accept minor differences.
6. **VCS invocation by shell string** (system() + temp file, commitlog.cpp:94) - Rust should reproduce exact commands (hg/bzr/svn/git/cvs flags in formats/*.cpp) and quoting, including `--date` ranges from `--start-date/--stop-date`.
7. **Missing CI** - nothing to port; define new.
8. **Version string** must stay `0.57` (or the next release) and print `Gource v0.57` as the help header; do not add a `--version` option unless parity with the C++ "unknown option" exit is deliberately dropped (corrected tick 4) (gource_settings.cpp:50).

## 9. Gaps
- (Resolved in tick 2) Per-crate license strings now verified, see 8.1. Transitive-dependency licenses still unchecked.
- Exact bytes of Windows DLL list at build time depend on MSYS2 (`cygcheck`), not in repo.
- src/core/ui/* (UI widgets, file selector) dependency detail only grepped (Boost.Filesystem heavy in file_selector.cpp), not deeply traced.
- The `README.md` was not inspected for packaging/platform statements beyond license and high-dpi note.

## Tick 2 additions

### T2.1 glibc `rand()` bit-exact port (and why macOS differs)

Where Gource uses libc `rand()`: simulation jitter `rand() % 100` in src/dirnode.cpp:625, src/user.cpp:106,128; string-hash seed `(rand() % 10000) + 1` in src/gource.cpp:1555. `srand` is called only for `--start-position random`: `srand(time(0))` at src/gource_settings.cpp:1319 (time-seeded, so non-reproducible by design). In every other run the generator is **unseeded = default seed 1**, so output of a given binary is deterministic per libc.

**Correction to the brief:** glibc's TYPE_3 warm-up is **310 discarded outputs (10 x degree 31)**, not 344. 344 = 31 (state) + 3 (separation) + 310, the index of the first returned value when the generator is described as a recurrence `r[i] = r[i-31] + r[i-3]` over a seed array (the well-known "r[344]" description). Both describe the same sequence. Source (glibc `stdlib/random_r.c`, `__srandom_r`/`__random_r`, fetched 2026-10-29): https://raw.githubusercontent.com/bminor/glibc/master/stdlib/random_r.c . glibc `rand()` calls `random()`, whose default state is TYPE_3 (`DEG_3 31`, `SEP_3 3`); the `rand()` -> `random()` forwarding is from memory of stdlib/rand.c, not fetched.

Algorithm (seed `s`, `uint32`; if `s == 0` then `s = 1`):
1. `r[0] = (int32) s`.
2. For `i = 1..30`: `w = r[i-1]` (signed); `hi = w / 127773` (C truncating division); `lo = w - hi*127773` (C remainder); `w = 16807*lo - 2836*hi`; `if w < 0 { w += 2147483647 }`; `r[i] = w` (stored as u32). (Schrage's method for `16807*w mod 2^31-1`; matches the quoted glibc code.)
3. `fptr = 3` (SEP_3), `rptr = 0`.
4. Run the generator step 310 times and discard: `r[fptr] = r[fptr].wrapping_add(r[rptr])` (u32); output `r[fptr] >> 1`; then `fptr += 1; rptr += 1`, each wrapping to 0 at 31.
5. Each subsequent `rand()` is one more step; the result is the 31-bit value `>> 1` (RAND_MAX = 2147483647).

Validation: reference implementation `golden/scripts/glibc_rand.py` (shipped with the discovery docs; Python model only) with seed 1 yields `1804289383, 846930886, 1681692777, 1714636915`, the widely known first outputs of glibc `rand()`. No glibc binary was run on this macOS host; the match is against that remembered sequence. For seeds >= 2^31 step 1 casts to negative `int32`, so use `i32` in step 2.

**macOS differs.** Apple's libc `rand()` is the FreeBSD one: Park-Miller LCG `x = 16807*(x % 127773) - 2836*(x / 127773)`, `if x < 0 { x += 0x7fffffff }`, return `x % (RAND_MAX + 1)`, initial `next = 1`, a zero state replaced by `123459876`; a `USE_WEAK_SEEDING` build variant instead uses `x*1103515245 + 12345`. Source: https://github.com/apple-oss-distributions/Libc/blob/main/stdlib/FreeBSD/rand.c (fetched 2026-10-29 via a summarising tool, so which variant Apple compiles is confirmed only by the empirical check below). **Empirical check on this host (Darwin 25.5.0, C program `srand(1)` then four `rand()`):** `16807 282475249 1622650073 984943658`, RAND_MAX 2147483647. That is 16807^n mod (2^31-1), i.e. the default Park-Miller variant, not glibc's additive-feedback generator. Windows MSVC `rand()` is a third algorithm (RAND_MAX 32767), not researched. Consequence: Gource's jitter and `gStringHashSeed` (hence hash-derived colours) already differ per platform, so "bit-exact against the C++ binary" is well-defined only per platform. Recommendation: a selectable `Rng` trait with `Glibc` (golden tests vs a Linux C++ build), `AppleLcg`, and a platform-independent default; real time seeding only for `--start-position random`.

### T2.2 Driving a legacy-GL-equivalent pipeline from Rust

Existing shaders (data/shaders/*): no `#version` line (GLSL 1.10 default); legacy built-ins `gl_Vertex`, `gl_MultiTexCoord0`, `gl_TexCoord[0]`, `gl_FrontColor`/`gl_Color`, `ftransform()` (bloom.vert:6-9, shadow.vert:3-5, text.vert:3-5), `varying vec3 pos` (bloom.vert:2, bloom.frag:2), `texture2D`, `gl_FragColor` (shadow.frag:6-8, text.frag:7-14, bloom.frag:15). Uniforms: `tex`, `shadow_strength`, `texel_size` (shadow.frag:1-2, text.frag:1-3). They are tiny (each < 20 lines), so the shader rewrite is cheap; the real cost is replacing the fixed-function host code (immediate mode `glBegin` 29 sites, display lists, matrix stack, `glLoadName` picking, client arrays; section 4).

Required rewrite regardless of stack:
- Vertex: explicit attributes `in vec3 position; in vec4 texcoord; in vec4 color;` plus `uniform mat4 mvp` (what `ftransform()` computed; keep the matrix stack in `glam` on the CPU). `gl_TexCoord[0]` -> `out vec4 v_texcoord`; `gl_FrontColor`/`gl_Color` -> `out vec4 v_color`. bloom.vert reads `gl_MultiTexCoord0.yzw` as extra data, so keep a full vec4 texcoord attribute.
- Fragment: `texture2D` -> `texture`; `gl_FragColor` -> `out vec4 frag_color`; `varying` -> `in`/`out`.
- Host: immediate mode becomes per-frame batched VBO/VAO; display lists become cached VBOs; `gluProject/gluUnProject/gluPerspective/gluLookAt` become `glam`; `glLoadName` selection (if used for picking) becomes CPU picking. Wide lines (`glLineWidth > 1`) are not guaranteed in a core profile: draw thick lines as quads.
- Version target: Apple's page says macOS OpenGL runs "from 1.x through OpenGL 3.2" and "OpenGL was deprecated in macOS 10.14" (https://developer.apple.com/opengl/). The portable target is therefore **GL 3.2 core, `#version 150`**, not 3.30 (3.30 is fine on Linux/Windows only). Gource already has a shader preprocessor (src/core/shader_common.cpp) that can switch keywords per target.

Option A: **`sdl2` 0.38.0 + `glow` 0.18.0** (+ `glam`). Same SDL semantics as the C++ app (text input, relative mouse, clipboard, cursors, GL attributes incl. multisample and swap interval, drawable size/high-DPI), so the largest behaviour surface (SDL row in section 4) ports 1:1; `glow` is a thin binding "to run GL anywhere (Open GL, OpenGL ES, and WebGL)" (https://github.com/grovesNL/glow) so draw order and blend modes stay recognisable for visual equivalence; `glReadPixels` export maps directly. Cons: C SDL2 dependency (`bundled` feature needs cmake); SDL2 is being superseded by SDL3 (`sdl3` crate 0.20, pre-1.0); depends on OpenGL, deprecated on macOS; Windows HWND hacks need `raw-window-handle`.

Option B: **`winit` 0.30.13 + `glutin` 0.32.3 / `glutin-winit` 0.5.0 + `glow`**. Pure Rust, same GL path, but glutin-winit last released 2024-06-08 and IME/clipboard/cursor-grab semantics differ from SDL, so more input-behaviour tests.

Option C: **`wgpu` 30.0.1 + `winit`**. Metal/Vulkan/DX12, no deprecated-API risk, offscreen bloom natural. Costs: rewrite of host rendering and shaders to WGSL; the `naga` GLSL frontend lists only GLSL 440/450/460 (Vulkan GLSL) as supported (https://docs.rs/naga/latest/naga/front/glsl/index.html), so legacy GLSL 1.10 cannot be auto-ingested and each shader is hand-ported; no line width, `glCopyTexImage2D` bloom becomes an explicit render-to-texture pass; far more pipeline boilerplate for a small renderer.

**Recommendation: A, `sdl2` + `glow` targeting GL 3.2 core / `#version 150`**, with all GL calls behind one `Renderer` module so a wgpu backend can be added if macOS OpenGL is removed. Reasons: (1) lowest behavioural risk since window/input/DPI/GL-attribute semantics equal the C++ app; (2) shaders are trivial and the immediate-mode batching work is needed by every option; (3) wgpu buys future-proofing, not fidelity, and can be deferred; (4) `glow`, `glam`, `sdl2` are permissively licensed and current (8.1). Revisit if Apple removes OpenGL, SDL2 shows Wayland/HiDPI bugs, or pure-Rust packaging becomes a requirement (then B).

### T2.3 Git quoted-path handling claim (log-formats.md) verified

Claim at docs/rust-rewrite/discovery/log-formats.md:124: "No unescaping of git C-quoted `\NNN` octal". **Confirmed.** The only quote handling in src/formats/git.cpp is lines 234-239:
- Condition: `file.find('"') == 0 && file.rfind('"') == file.size()-1`, i.e. first byte is `"` and last byte is `"` (a path that only starts or only ends with a quote is untouched).
- If `file.size() <= 2` (a lone `"` or `""`) the entry is skipped.
- Otherwise the first and last bytes are stripped (`file.substr(1, file.size()-2)`).
- Nothing else is interpreted: no `\NNN` octal, no `\"`, `\\`, `\t`, `\n`, `\a`, `\b`, `\f`, `\r`, `\v`. A quoted name like `"caf\303\251.txt"` is stored literally as `caf\303\251.txt`; an embedded `\"` stays `\"`.
Context: `status = line.substr(tab-1, 1)`, `file = line.substr(tab+1)` (lines 229-230); lines with no tab, tab at position 0 or last are skipped (227). Rename/copy lines (`R100<TAB>old<TAB>new`) are not split, so `old<TAB>new` becomes one path. For bit-compat the Rust port should replicate exactly this; proper unescaping is an optional improvement (git quotes only when `core.quotePath` applies to non-ASCII or special bytes).
