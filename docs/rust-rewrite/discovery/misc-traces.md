# Misc traces (nine small items)

All paths relative to repo root. Cites are `file:line`. "Verified" = run against the built binary
(a binary built with `golden/scripts/build.sh` + `link.sh`); everything else is read from source.

Correction to the brief: multi-repo re-import is NOT at gource_settings.cpp:1280-1357 (that range is
start/stop-date/position parsing). The real code is `GourceShell::getNext` (src/gource_shell.cpp:205-251),
`GourceSettings::setGourceDefaults` (src/gource_settings.cpp:368-520) and `importGourceSettings` (611-1725).

---

## 1. PPMExporter / FrameExporter (src/core/ppm.cpp, ppm.h)

**Construction** (`FrameExporter::FrameExporter`, ppm.cpp:39-63). Needs `display` already initialised.
`rowstride = display.width*3`; allocates three `char[height*rowstride]` buffers: `pixels1`, `pixels2`
(GL read targets, ping-pong) and `pixels_out` (flipped, written). Creates one SDL mutex, one SDL cond and
one SDL thread named "frame_exporter" running `dumpThr`. Buffers are raw `new[]` (uninitialised).
Size is fixed at construction; window resize is not handled (resizing is disabled when `-o` is set,
main.cpp:147).

**PPMExporter ctor** (ppm.cpp:156-176). Output `"-"` => `std::cout` (Windows: `_setmode(fileno(stdout), _O_BINARY)`,
ppm.cpp:159-161). Otherwise `std::ofstream(file, out|binary)`; on `fail()` it deletes and throws
`PPMExporterException(file)` (ppm.cpp:168-171), main.cpp:194-199 catches and calls
`SDLAppQuit("could not write to '<file>'")` (exit 1). Header is built once: `"P6\n%d %d 255\n"` with
`display.width display.height` (ppm.cpp:175). No `Content` beyond that (no comment lines).

**Per frame** (`Gource::update`, gource.cpp:271-276): only if exporter set, commitlog loaded and
`!shutdown`, and `framecount % (frameskip+1) == 0`, call `dump()`. `frameskip`/`max_tick_rate` come from
`setFrameExporter` (gource.cpp:505-522): `gource_framerate = fps; while(<60) { += fps; frameskip++ }`,
`max_tick_rate = 1/gource_framerate`; with an exporter `scaled_dt = max_tick_rate` (fixed step, gource.cpp:251-253).

**dump()** (ppm.cpp:96-119), main thread: `display.mode2D()`, enable TEXTURE_2D, disable BLEND,
`glPixelStorei(GL_PACK_ALIGNMENT,1)`, `glReadPixels(0,0,w,h,GL_RGB,GL_UNSIGNED_BYTE, next)` where `next` is
whichever of pixels1/pixels2 is NOT `pixels_shared_ptr`. The read happens OUTSIDE the lock. Then
`SDL_mutexP`, set `pixels_shared_ptr = next`, `state = DUMP`, `CondSignal`, `mutexV`.

**dumpThr()** (ppm.cpp:121-152): takes the mutex for its whole life. Loop: set `WAIT`, `CondWait` (releases
the mutex only while waiting) until state != WAIT; on EXIT break; else flip rows vertically from
`pixels_shared_ptr` into `pixels_out` (ppm.cpp:138-142, GL origin is bottom-left so the file is top-down),
then `dumpImpl()` — and it does all that WHILE HOLDING the mutex. Consequence = back-pressure: if the
consumer (pipe/disk) is slow, the next `dump()` blocks at `SDL_mutexP` on the main/render thread until the
previous frame has been fully written. No queue, no frame dropping; at most one frame in flight plus one
being read. A frame signalled while the thread is not yet waiting is not lost (state stays DUMP; thread's
`WAIT` reset at loop top only happens when it re-acquires).

**dumpImpl()** (ppm.cpp:185-188): `*output << ppmheader; output->write(pixels_out, rowstride*height);`
Header and payload per frame, concatenated => a PPM stream (ffmpeg `-f image2pipe -c:v ppm`). No flush call
(stdout flushed by stream buffering/exit).

**Errors**: none checked. A failed write only sets failbit/badbit on the ostream; every later `<<`/`write`
is a silent no-op while the program keeps rendering at full cost. No message, no exit code. Broken pipe:
no SIGPIPE handling exists anywhere (`grep SIGPIPE src` = empty), so on POSIX the default action kills the
process silently with SIGPIPE (exit 141 from a shell) the first time the write hits a closed pipe. (Note:
glReadPixels + header both succeed before that; nothing is flushed or cleaned up.)

**Shutdown**: `Gource::~..`/quit path calls `frameExporter->stop()` (gource.cpp:1593 when logmill fails;
`~PPMExporter` -> `stop()`, ppm.cpp:178-183). `stop()` (ppm.cpp:78-94): if thread alive and not already
STOPPED/EXIT, lock, state=EXIT, signal, unlock, `SDL_WaitThread`. `~PPMExporter` closes the file only when
`filename` non-empty (so stdout is never closed). `~FrameExporter` then frees buffers.
Rust mapping: bounded(1) channel (or Mutex+Condvar) between render thread and writer thread with blocking
send; write errors should be surfaced (original silently continues / dies on SIGPIPE).

---

## 2. Stdin / StreamLog and "finished" semantics

**Selection**: path `"-"` => `RCommitLog` ctor (src/formats/commitlog.cpp:48-57): optional first-char peek on
`std::cin` (`checkFirstChar`, commitlog.cpp:109-120, e.g. git raw expects a given first char), then
`logf = new StreamLog()`, `seekable = false`, `success = true`. `log-format` is mandatory for stdin
(gource_settings.cpp:1635-1638, error "log-format required when reading from STDIN").

**Startup wait** (gource_settings.cpp:1649-1652, non-Windows): `while(std::cin.peek()==EOF && !std::cin.fail()) SDL_Delay(100);`
then `std::cin.clear()`. Only blocking/poll-with-sleep in the whole path: waits in 100 ms steps for the first
byte; (Windows: `PeekNamedPipe` loop, 1640-1647). Because `peek()` at EOF sets eofbit and the next `peek()`'s
sentry sets failbit, an immediately-closed/empty stdin ends the loop after ~1 iteration rather than hanging.

**StreamLog ctor** (src/core/seeklog.cpp:38-57): `stream=&std::cin`; POSIX: `fcntl(STDIN, F_SETFL, flags|O_NONBLOCK)`;
on failure logs and sets `fcntl_fail=true` (which makes `isFinished()` permanently true). Windows stores the
stdin handle.

**StreamLog::getNextLine** (seeklog.cpp:59-87): (1) `if(isFinished()) stream->clear()` — it always tries to
recover from EOF/fail, so EAGAIN-as-EOF is retried every call; (2) Windows only: `PeekNamedPipe`, return false
if 0 bytes available (never blocks); (3) `std::getline`; (4) strip one trailing `\r`; (5) if the stream is
now eof/fail (`isFinished()`), RETURN FALSE even if `line` holds text; else true.
`StreamLog::isFinished` = `fcntl_fail || fail() || eof()` (seeklog.cpp:89-95).

**Partial lines**: on a non-blocking pipe a read that returns EAGAIN mid-line makes `getline` return the
partial text with eofbit set; step (5) then returns false and the caller discards `line` => the first half of
a line split across pipe writes is LOST (the remainder is parsed next call as a malformed line). There is no
line-reassembly buffer. This is inferred from libstdc++/libc++ semantics, not exercised; unverified.

**RCommitLog::isFinished** (commitlog.cpp:238-242): `return seekable && logf->isFinished();` => for stdin it is
ALWAYS false. Gource therefore never decides a stdin log is finished; EOF on stdin just yields endless
`nextCommit()==false`. Where it matters:
- `Gource::readLog` (gource.cpp:1137-1148): loop condition uses `!isFinished()` (always true for stdin);
  on `nextCommit` false and `!isSeekable()` it `break`s (no spin, no sleep; polled again next frame).
- `is_finished` (gource.cpp:1167) is therefore false => `--stop-at-end` never fires from stdin EOF
  (`stop_position` needs seekable); gource keeps running until the user quits. Exporter case: main/shell
  deliberately does NOT force `stop_at_end` when `path == "-"` (gource_shell.cpp:221-226).
- `first_read && commitqueue.empty()` => `SDLAppException("no commits found")` (gource.cpp:1156-1158) — fires if
  the first frame's read finds nothing even though stdin is merely slow (checkFormat's one-commit
  read-ahead, commitlog.cpp:129-146, sets `buffered=true` so the first commit is normally present).
- `writeCustomLog` on stdin: `while(!isFinished())` + `if(!nextCommit) { if(!isSeekable()) break; }`
  (gource.cpp:194-203) => stops at the FIRST failed read, i.e. at EOF or at the first moment the pipe is
  momentarily empty. Equivalent to "read until first empty read".
- `start_timestamp` skip loop in logmill (logmill.cpp:90-97) relies on `isFinished()` => for stdin it loops
  until a commit >= start is found (blocking the logmill thread; CPU spin on EAGAIN).

SeekLog (files) for contrast: whole file read into an `istringstream` unless > `gSeekLogMaxBufferSize`
(100 MiB, seeklog.cpp:26-ish/`readFully`); `getNextLine` returns false iff `fail()`; finished = fail||eof.

---

## 3. Multi-repo settings re-import (GourceShell::getNext)

Flow (gource_shell.cpp:205-251): delete previous `Gource` (its dtor calls `gGourceSettings.setGourceDefaults()`,
gource.cpp:222-223, and sets `transition_interval=1.0`); stop if `shutdown` or iterator at end;
`importGourceSettings(*conf, *gource_settings)`; then fixups:
1. exporter present and not (`dont_stop || loop || path=="-"`) => `stop_at_end=true` (gource_shell.cpp:221-226);
2. `repo_count > 1` and `stop_at_time <= 0 && stop_position <= 0` => `stop_at_time = 60.0` (228-233);
3. advance iterator; if it hit the end and `repo_count>1 && exporter==0` wrap to begin (loops forever in
   interactive mode; with an exporter it ends after the last repo) (237-242);
4. `new Gource(exporter)`.
`repo_count = conf->countSection("gource")` is set once in the GourceShell ctor (gource_shell.cpp:32).

`importGourceSettings` first calls `setGourceDefaults()` (gource_settings.cpp:613) so every per-repo setting
is reset to its default before the section's entries are applied. Reset list = everything in
gource_settings.cpp:368-520: path ("."), default_path, ffp, all hide_*, start/stop timestamp+date,
start/stop_position, stop_at_time (-1), stop_on_idle, stop_at_end, dont_stop, no_time_travel, fixed_user_size,
author_time, show_key, disable_auto_rotate, disable_input, auto_skip_seconds(3), days_per_second(0.1),
file_idle_time(0), file_idle_time_at_end(0), time_scale(1), loop(false), loop_delay(3), logo+offset,
colour_user_images, default_user_image, user_image_dir, user_image_map.clear(), camera zoom min/default/max
(50/100/10000), camera_mode("overview"), padding(1.1), crop_*, bloom_*(1.0/0.75), background colour/image,
title, font_scale(1)+default_font_scale(true), font file/sizes, dir/font/highlight/selection colours,
dir_name_depth/position, elasticity, git_branch, log_format, date_format, max_files, max_user_speed(500),
max_file_lag(5), user_idle_time(3), user_friction(1), user_scale(1), `follow_users.clear()`,
`highlight_users.clear()`, highlight_all_users, highlight_dirs, all caption_* (duration 10, size 16, offset 0,
white), filename_colour/time(4), `gStringHashSeed=31` (a global!), file/user filters and show-filters
(Regex* deleted and cleared), file_extensions, file_extension_fallback; then `setScaledFontSizes()`.

NOT reset (survive across repos): `repo_count`, `shutdown` (set in ctor, gource_settings.cpp:212-215),
`load_config`, `save_config`, `output_custom_filename`, `file_graphic` (texture grabbed once, gource.cpp:32-34),
`log_level`, and all display-level state (SDLAppSettings: viewport, fullscreen, vsync, `high_dpi`,
`output_ppm_filename`, `output_framerate`, transparency, etc., imported once by `importDisplaySettings`,
main.cpp:109) plus the `ConfFile` itself. CLI args apply to every `[gource]` section (parseArgs loops all
sections of that name, settings.cpp:268-277), so each repo sees CLI overrides; `path` from the positional arg
is set on every section (main.cpp:93-104). Per-section keys come from the loaded conf.
Gotcha: the `Gource` dtor also resets to defaults, so reset happens twice per transition; a Rust port needs
only "fresh struct from defaults + section" per repo, plus the global hash seed.

---

## 4. ConfFile::save format (src/core/conffile.cpp:457-488, ConfSection::print 395-411)

Verified by running `--save-config`. Algorithm: open file (truncate; failure => `ConfFileException("failed to write
config to <file>")`, exit 1 via SDLAppQuit); iterate `sectionmap` (a `std::map<string, list<ConfSection*>>`,
conffile.h:143) => sections in ALPHABETICAL name order (`display` before `gource`), repeated sections of the
same name in insertion order; each `ConfSection::print`: `[name]\n`, then entries iterated from
`entrymap` (`std::map<string, ConfEntryList*>`, conffile.h:96) => KEYS ALPHABETICAL (byte order), repeated
keys (multi-value options) in insertion order, each as `key=value\n` (no spaces around `=`, no quoting or
escaping, value is the raw string, `std::endl` = `\n`), then one blank line after every section (so file
ends with a blank line). Empty sections still print `[name]` + blank.
Values are the raw CLI strings (or the original file text): booleans stored as `true` (parseArgs
settings.cpp:241), numbers as typed, aliases resolved to the long name (`-s 2` => `seconds-per-day=2`,
`--disable-bloom` => `hide-bloom=true`, `-1280x720` => `[display] viewport=1280x720`). `--save-config` and
`--load-config` themselves are "command-line" options and are not stored. Typed setters (`%.5f`, `vec3(%.5f, ...)`,
`yes/no`, conffile.cpp ~95-125) exist but are only used by `exportDisplaySettings` (settings.cpp:92), which is
never called (dead code).

Save runs after import/validation but before any window: main.cpp:109-116 `importDisplaySettings; importGourceSettings;
if(save_config) { conf.save(file); exit(0); }`.

Typical output (`gource -1280x720 --hide filenames,dirnames -s 2 --camera-mode track --highlight-user "A B"
--highlight-user Zed --background-colour 202020 --disable-bloom --file-filter 'x.*' --high-dpi
--save-config out.conf tests/logs/utf8.log`):
```
[display]
high-dpi=true
viewport=1280x720

[gource]
background-colour=202020
camera-mode=track
file-filter=x.*
hide=filenames,dirnames
hide-bloom=true
highlight-user=A B
highlight-user=Zed
path=tests/logs/utf8.log
seconds-per-day=2

```
(Verified; the earlier run also saw `-s 2` here. Block above is the complete file, final blank line included.) A bare
`gource --save-config f log` yields exactly `[gource]\npath=log\n\n`. Loader side: key regex
`^\s*([^=\s]+)\s*=\s*([^\s].*)?$` (conffile.cpp:~30), sections `^\s*\[([^\]]+)\]\s*$`; values are
trimmed on the right. Round-tripping is value-preserving but drops comments and original ordering.

---

## 5. Hover, click, camera follow/selection (src/gource.cpp)

Per frame in `draw` (gource.cpp:2405-2431): if mouse not hidden and `cursor.isVisible()` call `mousetrace(dt)`,
else clear `hoverUser/hoverFile` (`setMouseOver(false)`).

**mousetrace** (1880-1986):
1. Unproject: `projected_mouse = vec2(-(mx*2 - W)/H, 1 - 2*my/H) * cam_pos.z`, then `+= cam_pos.xy`
   (1884-1889; note the x sign flip and that it scales by camera z which is negative).
2. Users first (unless `hide_users`): `userTree->getItemsAt(set, pt)`; the first user in the (pointer-ordered)
   set with `!isFading()` and `quadItemBounds.contains(pt)` wins (1897-1911).
3. Files only if no user hit (unless `hide_files`): `dirNodeTree->getItemsAt`, for each dir, each file with
   `!isHidden() && file->overlaps(pt)`; inner `break` only leaves the file loop, so the LAST dir with a hit
   wins (1913-1936).
4. Hover state: file hit beats user hit (1938-1950 file branch is checked first, then user at 1953-1967): clears the
   other hover; swapping hover calls `setMouseOver(false/true)` on old/new. Neither hit => clear both.
5. `if(mouseclicked)` (set by left button DOWN in `mouseClick`, gource.cpp:490, cleared at end of frame 2759-2760):
   - hoverUser: `camera.lockOn(false); selectUser(hoverUser)`;
   - else hoverFile: `camera.lockOn(false); selectFile(hoverFile)`;
   - else `selectBackground()`.
   Hover is only a highlight + tooltip (draw 2674-2690: shows if different from the selected one); the camera
   follows only after a click (or follow-user / `n` key).

**selectUser** (560-590): no-op if already selected; deselects file and user (`setSelected(false)`); `user==0` =>
`camera.lockOn(false)` and return; else `selectedUser=user; setSelected(true); if(track_users) camera.lockOn(true)`.
**selectFile** (593-615): deselects user and file; sets `selectedFile` and `setSelected(true)`; NEVER locks the
camera. **selectBackground** (545-558): only if left button is held: `selectUser(0)` (also drops file),
`manual_camera=true`, `mousedragged=true`, `grabMouse(true)` => drag pans the camera (updateCamera 1482-1497:
`cursor_move * (-cam.z/5000) * 10` added to cam pos, `camera.setPos(..., true); camera.stop()`).
Other camera-affecting input: middle click `toggleCameraMode()` (466-469) => `setCameraMode(!track_users)` (528-538:
clears manual_rotate/zoom/camera, `lockOn` if a user is selected, writes `gGourceSettings.camera_mode`);
arrow keys set `manual_camera` (gource.cpp:1632-1651); right-drag rotates (360-376); wheel `zoom()` sets
`manual_zoom`, step x/÷1.1 clamped to [camera_zoom_min, camera_zoom_max] (426-441).

**Camera target each frame** (`updateCamera`, 1478-1522), when `!manual_camera`: if `track_users` and a user and/or file
is selected => bounds of {selectedUser->getPos(), selectedFile->getAbsolutePos()}; else if `track_users && idle_time==0`
=> `active_user_bounds`; else `dir_bounds` (whole tree). `camera.adjust(bounds, !manual_zoom)` sets dest xy to
bounds centre and, if distance allowed, z from bounds*`padding`, aspect and fov (zoomcamera.cpp:96-148, clamped to
min/max distance). `camera.logic(dt)` eases: `pos += (dest-pos)*dt*speed(1.0)`; while `lockon`, blends with
`lockon_time` (reset to 1.0 by `lockOn(true)`, decays 0.5/s) so the first second is a slow-start (zoomcamera.cpp:61-68,
170-186); target = (pos.x, pos.y, 0).
**follow-user setting** (1385-1397): each frame, for each non-idle user, if nothing selected and the user's name equals any
`follow_users` entry (exact string match) => `selectUser(u)`. Users fading out are deselected (1376-1379). A selected user's
`selectedUser = 0` reset happens in `reset()` (gource.cpp:905-911). `n`/next-user key uses `selectNextUser` (633-652,
iterates `tagusermap` by tag id, skipping inactive/not-fully-opaque users).
Without `--camera-mode track`, selecting a user does not move the camera at all (overview fits `dir_bounds`).

---

## 6. loadCaptions (gource.cpp:1080-1129; RCaption src/caption.cpp)

Regex (gource.cpp:1080): `^(?:\xEF\xBB\xBF)?([^|]+)\|(.+)$` (PCRE2, bytes; optional UTF-8 BOM only at line start).
Wrapper `Regex::match` returns capture groups only, 0-indexed (`results[0]` = group 1; regex.cpp:198-205), so
`matches[0]` = timestamp text (up to the FIRST `|`, so it cannot contain `|`), `matches[1]` = caption (everything
after the first `|`, may contain more `|`; `.+` requires >=1 char; no match => line skipped).

Procedure: called once, when the first commit is queued and `currtime==0` (gource.cpp:1717-1719) — i.e. after
`currtime` is known (not on `seekTo` or loop restart; `reset()` does not reload them). No file / unreadable =>
silently returns (existence was checked at settings import, gource_settings.cpp:872). For each line: `ConfFile::trim`
(RIGHT-trim only, conffile.cpp:~497-504; note `\r` stripped), regex, timestamp: if text length > 1 and contains
`-` after index 0 => `SDLAppSettings::parseDateTime` (accepts `YYYY-MM-DD[ T]hh:mm[:ss[.f]]` + `Z`/`+hh[:mm]` offsets, local
time by default; fail => skip line), else `atoll` as unix seconds (`0` only valid if text is literally "0"; negative
numbers parse). Caption text = `RCommitLog::filter_utf8` (invalid UTF-8 => replaced; "???" on exception). Then
DROP if `timestamp < currtime` ("ignore older"), DROP if `timestamp < last_timestamp` ("ignore out of order"); else
`last_timestamp = timestamp` and push `new RCaption(text, ts, fontcaption)`. Equal timestamps keep file order.
No sorting; whole file is read eagerly.

Playback (gource.cpp:1805-1857): each frame, while `captions.front()->timestamp <= currtime`: pick the lowest free slot
`y` starting at `caption_start_y` and moving up by `fontcaption.getMaxHeight()` per occupied slot, compute x
(`caption_offset==0` => centred `width/2 - textwidth/2`; `>0` => x = offset; `<0` => right aligned
`width + offset - textwidth`), `setPos`, move to `active_captions`. `caption_start_y = floor(slider.min.y - 35)` if
seekable, else `height - fontmedium.maxHeight - 20`; with a title, `min(that, height - 20 - fontmedium.maxHeight)`
(1778-1785). `RCaption::logic` (caption.cpp:~56-60): `elapsed += dt; fade_in = min(2, duration/3);
alpha = min(1, min(elapsed, max(0, duration-elapsed)) / fade_in)`; finished when `elapsed >= caption_duration`
(removed+deleted). Draw: font colour = caption colour with alpha, drop shadow on (gource.cpp:58-60).

Options (gource_settings.cpp:866-925): `--caption-file` (must exist, else error "caption file not found"),
`--caption-duration` float >0 (default 10), `--caption-size` int 1..100 (default 16; font created once at
`fontcaption = grab(font_file, caption_size)`, gource.cpp:58 — NOT multiplied by `font_scale`, unlike the other
fonts), `--caption-offset` int (default 0), `--caption-colour` either `vec3(r,g,b)` floats or 6 hex digits `RRGGBB`
(no `#`) (default white). Note a caption with a DATE-string timestamp is local-time; with `--stop-date` etc. unrelated.

---

## 7. Bounds2D line width / HiDPI scaling

`Bounds2D` (src/core/bounds.h:34-135) is pure geometry: min/max/first, `update`, `contains`, `overlaps`, `centre`,
`width/height/area`. Its `draw()` (bounds.h:127-135) emits a `GL_LINE_STRIP` rectangle outline and sets NO line
width or colour; it inherits the current GL state. Only call: debug mode (the `debug` flag) in gource.cpp:2546-2552,
which sets `glLineWidth(2.0)`, white, and draws `active_user_bounds` (track) or `dir_bounds`. `Bounds3D::draw`
(bounds.h:205-211) draws one `GL_LINES` diagonal, white.
No HiDPI scaling of any line width anywhere: all `glLineWidth` calls are constants — ui/element.cpp:45,88 (1.0),
ui/slider.cpp:30 (1.0), slider.cpp:124,128 (2.0, timeline), gource.cpp:2549 (2.0), 2558 (1.0 quadtree debug).
Tree branch splines are drawn by `spline.draw()` (dirnode.cpp:1138) with their own geometry, not GL line width.
What IS DPI-aware: (a) `display.viewport_dpi_ratio` = drawable/window size (display.cpp:173-182), (b) mouse events are
multiplied by it (sdlapp.cpp:352-355, 387-388), (c) `font_scale` defaults to the ratio when `x>1`, else
`1 + height/1600` (integer division) (gource.cpp:39-49) — this is the only DPI-driven visual scale; sizes of fonts
get `scaled_*` variants; caption font does not. Everything else is in drawable-pixel units, so on a 2x display
geometry is drawn in the doubled pixel space with unchanged world-to-screen scale (the camera fits to
`display.width/height`, which are drawable pixels, display.cpp:383/530).

---

## 8. --high-dpi path

- CLI: `arg_types["high-dpi"]="bool"`, section `display` (settings.cpp:43,61) => `[display] high-dpi=true`.
- Import: `display_settings->getBool("high-dpi")` => `SDLAppSettings::high_dpi = true` (settings.cpp:442-444;
  default false, settings.cpp:81).
- Decision in main.cpp:153-163: `bool high_dpi = true;` then (non-Windows only, `#ifndef _WIN32`)
  `if(viewport_specified && !gGourceSettings.high_dpi) high_dpi = false;` then
  `display.enableHighDPIAwareness(high_dpi)`. So the DEFAULT is high-DPI AWARE; the flag only matters to
  override the "explicit viewport" exception (a `WxH` viewport turns it off unless `--high-dpi` is given). On
  Windows it is always true.
- `SDLAppDisplay::enableHighDPIAwareness` just stores `high_dpi_aware` (display.cpp:118-120; default false, :47).
- Effect: `SDLWindowFlags` ORs `SDL_WINDOW_ALLOW_HIGHDPI` when set, non-Windows only (display.cpp:88-91, SDL2 branch).
  On Windows `init()` instead calls `setWindowsHighDPIAwareness()` (display.cpp:551-590, 595) which loads
  `SetProcessDpiAwareness(PER_MONITOR)` or falls back to `SetProcessDPIAware()` from USER32/SHCORE.
  After window creation, `SDL_GL_GetDrawableSize` provides `display.width/height` and `updateViewportDPIRatio()`
  records the ratio (display.cpp:383-385, 530-532).
Rust/winit: request a HiDPI-capable surface by default, use physical (drawable) pixels for `width/height`, keep the
logical->physical ratio for mouse input, and disable HiDPI when an explicit viewport is given without `--high-dpi` (macOS/Linux).

---

## 9. --output-custom-log writer (Gource::writeCustomLog, gource.cpp:171-214)

Trigger: main.cpp:119-123 after settings import: `if(!output_custom_filename.empty() && !path.empty()) { writeCustomLog(path, file); exit(0); }`
(no window, runs before display init). Option stored by `commandLineOption` (gource_settings.cpp:583-585).
Procedure: build `RLogMill(path)` (which runs the full log detection/`log-format` handling and `start-date` skip,
logmill.cpp:90-97); null log => print error via `SDLAppQuit` if one exists, else silently return (exit 0). Output
`stdout` for `-`, else `fopen(file,"w")` (failure => silent return, still exit 0 — no message). Loop
`while(!commitlog->isFinished())`: `nextCommit(commit)` (validate=true: user filter / show-filter / non-empty files,
commitlog.cpp:357-383; RCommit::postprocess filter_utf8 on username); on failure: seekable => `continue`,
non-seekable => `break`.
Line format, per FILE of the commit, in the order the parser produced them:
`fprintf(fh, "%lld|%s|%s|%s\n", (long long)commit.timestamp, username, cf.action, cf.filename)` (gource.cpp:207) — i.e.
`<unix-seconds>|<user>|<A|M|D>|<path>\n`, LF only, no header, no trailing blank line.
- Timestamp: integer epoch seconds (any date-string input is parsed to epoch first); same commit's files repeat it.
- Path: always starts with `/` (RCommitFile ctor prepends one if missing, commitlog.cpp:292-296); directory deletes keep their
  trailing `/` (golden svn-dir-delete: `D|/blah/`); text is `filter_utf8`'d.
- Action: whatever the parser stored (`A`, `M`, `D`; custom logs default to `A` if the field is empty, custom.cpp:~75).
- Username default `"Unknown"` for custom logs with empty user.
- Colour: NOT emitted (the custom format's optional 5th field is dropped; file colours are lost). `--stop-date` /
  `--stop-position` are not applied; `file-filter`/`user-filter` are (addFile/isValid).
- Ordering: commit order as parsed (no sorting); within a commit, file order of the parser.
- No flush/close checks; `fclose` only for real files.

Comparison with goldens (docs/rust-rewrite/discovery/golden/custom-log/): all four log goldens match this format exactly:
custom-dir-delete, file-removal, file-to-dir, utf8 (`1277787455|Árvíztűrő tükörfúrógép|A|/Árvíztűrő/tükörfúrógép/Árvíztűrő tükörfúrógép.txt`,
UTF-8 passes through unchanged), svn-dir-delete (svn log, date => `1263976518`). Inputs in tests/logs lack the leading
`/` (e.g. `path/to/file.1`) and the goldens show it added (`/path/to/file.1`) — confirms the ctor prepend.
`utf8-caption.txt` is empty with stderr `gource: unsupported log format ...`: that file is a caption file (`ts|text`), not a
commit log; the golden records the error path (exit code in exit-codes.txt). Stderr goldens are otherwise empty.

## Verification (tick 4)
Sampled against source: ppm.cpp:45-61 (rowstride, 3 buffers, SDL_CreateThread "frame_exporter"), 139-140 (row flip), 175 (P6 header), 186 (header write); commitlog.cpp:238-242 (isFinished = seekable && logf->isFinished()); gource_shell.cpp:225 (stop_at_end) and 232 (60.0f); gource_settings.cpp:368/613 and gource.cpp:223 (setGourceDefaults); conffile.cpp save() map iteration and conffile.h:96,143 (std::map => alphabetical); gource.cpp:545/560/590/1880 (selectBackground/User/File, mousetrace), :1080 caption regex (loadCaptions itself starts at :1082, minor), :171/207 (writeCustomLog, fprintf format); display.cpp:90,118,180; main.cpp:150-163 (high_dpi logic). All confirmed; no corrections needed (loadCaptions starts at 1082 not 1080; range cite is acceptable).

## Exercised evidence (tick 4, built binary)

- `gource --log-format custom --output-custom-log - - < tests/logs/utf8.log` exits 0 and its output is byte-identical to the file-argument golden (`golden/custom-log/utf8.stdin.txt` vs `utf8.txt`).
- Feeding two custom-format lines where the second has no trailing newline (`golden/custom-log/stdin-unterminated.input.txt`) emits only the first line (`stdin-unterminated.txt`, exit 0, empty stderr): the unterminated last line on stdin is dropped, as section 2 inferred.
- Still not exercised: empty stdin at startup, `--stop-at-end` with stdin in the interactive path, PPM pipe / SIGPIPE.
