# Timing: authoritative reference

Source of truth: `src/core/sdlapp.cpp`, `src/gource.cpp`, `src/gource_shell.cpp`, `src/gource_settings.cpp`, `src/core/settings.cpp`. All cites are `path:line` in this worktree. This section supersedes the dt claims in scene-model.md, core-lib.md, output-pipeline.md and app-core.md (see section 6 for the diffs).

## 1. Per-frame flow (SDL tick to simulation advance)

The app object is `GourceShell` (subclass of `SDLApp`). `GourceShell` sets `min_delta_msec = 16` in its ctor (gource_shell.cpp:29), overriding the `SDLApp` default of 8 (sdlapp.cpp:317). Effective minimum is 16 ms.

```
SDLApp::run()                                              sdlapp.cpp:405
  init(); SDL_StopTextInput()
  msec = last_msec = SDL_GetTicks()                        :414-415   (Uint32 ms)
  buffer_msec = total_msec = fps_updater = 0
  loop while !appFinished:                                 :422
    last_msec = msec; msec = SDL_GetTicks()                :423-424
    delta_msec = msec - last_msec                          :426
    buffer_msec += delta_msec                              :429
    if buffer_msec < min_delta_msec:                       :430   (16 for GourceShell)
        SDL_Delay(1); continue                             :431-432  (no events, no update)
    delta_msec = buffer_msec; buffer_msec = 0              :435-436  (delta can exceed 16, no remainder carried)
    total_msec += delta_msec                               :439
    t  = total_msec / 1000.0f      (f32)                   :441
    dt = delta_msec / 1000.0f      (f32)                   :442
    fps_updater += delta_msec; if >=1000: updateFramerate  :444-448  (fps display only)
    while SDL_PollEvent: handleEvent (keys/mouse/quit)     :452-455
    update(t, dt)  -> GourceShell::update                  :457 (corrected tick 4: was :458)
    display.update()   (swap; vsync via main.cpp:144)      :460 (corrected tick 4: was :461)
    frame_count++                                          :461 (corrected tick 4: was :462)  (SDLApp fps counter, not Gource::framecount)

GourceShell::update(t, dt)                                 gource_shell.cpp:283
  if gource==0 || gource->isFinished():                    :285
      if !getNext(): appFinished = true                    :286
      return                  // NO update/draw/dump this iteration (display.update still runs)
  gource->fps = fps                                        :291
  gource->update(t, dt)                                    :292  (t is IGNORED by Gource)
  toggle_delay -= dt (wall dt)                             :294
  transition texture copy / blendLastFrame(dt) (wall dt)   :297-306
  if next: delete gource                                   :308-313

Gource::update(t, dt)                                      gource.cpp:246
  scaled_dt = min(dt, max_tick_rate)        (f32)          :248   max_tick_rate=1/60 (f64 narrowed) :156
  if frameExporter != 0: scaled_dt = max_tick_rate         :251-253
  scaled_dt *= time_scale                                  :256
  if !paused: runtime += scaled_dt          (f32)          :259
  if stop_at_time > 0.0 && runtime >= stop_at_time:
        stop_position_reached = true                       :261
  logic(runtime, scaled_dt)                                :265
  draw(runtime, scaled_dt)                                 :269
  if exporter && commitlog && !shutdown && framecount % (frameskip+1) == 0:
        frameExporter->dump()                              :272-276  (after draw, before cursor)
  if !hide_mouse: cursor.logic(dt); cursor.draw()          :278-282  (REAL dt)
  framecount++                                             :284
```

`runtime` is only assigned in the Gource ctor (gource.cpp:157; grep shows no other assignment), so it is not reset by `reset()`/seek (verified tick 4). `seekTo` also un-pauses (`paused=false`, gource.cpp:1073), (verified tick 4) and advances (and `framecount` increments) while the log is still loading (`commitlog==0`, logic returns early at :1579-1581) and while paused (only the `runtime +=` is skipped when paused, :259).

Gource::logic(t=runtime, dt=scaled_dt), time-relevant part (gource.cpp:1568):

```
1570  if shutdown && logmill->isFinished(): appFinished = true; return
1575  message_timer -= dt; splash -= dt
1579  if commitlog==0: if !logmill finished return; take log (throw on error); seekTo(start_position) if >0   :1604
1609  file_key.logic(dt); slider.logic(dt)    (run even when paused)
1693  if paused: updateBounds; interactUsers; interactDirs; updateCamera(dt); return   // no commits, no updateUsers/updateDirs, idle_time frozen, no commit/appFinished-from-users check (the `shutdown` check at :1570 still runs before this) (verified tick 4)
1702  if commitqueue.empty(): readLog()
1707  if loop && commitqueue.empty() && commitlog->isSeekable() && idle_time >= loop_delay_seconds:
          first_read=true; seekTo(0.0); readLog()
1715  if currtime==0 && !commitqueue.empty(): currtime=lasttime=queue[0].timestamp; subseconds=0; loadCaptions()
1723  time_inc = dt * 86400.0 * days_per_second   (f64 expr, stored f32)
1724  seconds = (int)time_inc ; subseconds += time_inc - (float)seconds   (f32)
1728  if subseconds >= 1.0: currtime += (int)subseconds; subseconds -= (int)subseconds
1733  currtime += seconds
1744  while !commitqueue.empty():
1749      if auto_skip_seconds>=0.0 && idle_time>=auto_skip_seconds && !stop_position_reached:
1750          currtime=lasttime=front.timestamp; idle_time=0.0
1754      if front.timestamp > currtime break
1756      processCommit(front, t)           // t = runtime; action->t used for max_file_lag
1758-1769 no_time_travel ? lasttime=max : (lasttime>ts ? currtime=ts : ..; lasttime=ts)
1771      subseconds = 0.0 ; pop_front
1867-1875 updateBounds; interactUsers; updateUsers(t,dt); interactDirs; updateDirs(dt); updateCamera(dt)
1877  updateTime(commitqueue non-empty ? currtime : lasttime)   // localtime() display only
updateUsers tail: if users.empty() && stop_position_reached: appFinished=true   :1401
                  idle_time = (idle_users==users.size()) ? idle_time+dt : 0      :1406-1410
```

`currtime`/`lasttime` are `time_t` (commit timestamps); `idle_time` is f32 wall-less sim seconds (scaled dt, so it runs faster/slower with time_scale).

## 2. Time-related settings

| Option (CLI) | Default | Units | Validation | Enters flow at |
|---|---|---|---|---|
| `--time-scale S` | 1.0 (gource_settings.cpp:413) | multiplier | `S<=0 || S>4` -> error (:1266-1268). Keys: `.` up (>=1: min(4,floor+1); <1: min(1, x*2)), `,` down (>1: max(0,floor-1) so 1.5 -> 0; else max(0.25, x*0.5)), `/` reset to 1 (gource.cpp:853-873) | multiplies scaled_dt, gource.cpp:256 (affects runtime, stop_at_time, all dt-driven sim, idle_time, days elapsed) |
| `-s, --seconds-per-day S` | days_per_second=0.1 (i.e. 10 s/day) (:410) | sim-seconds per sim-day | `S<=0` -> invalid (:1203) ; stored `1.0/S` (:1208). Keys `=`/`+`: >=1: min(30, floor+1) else min(1, x*2); `-`: >1: max(0, floor-1) else max(0, x*0.5) (gource.cpp:821-835) -> can reach 0 | time_inc, gource.cpp:1723 |
| `--realtime` | off | - | sets days_per_second = 1/86400 (:1360-1362); applied AFTER seconds-per-day so it wins | gource.cpp:1723 |
| `-a, --auto-skip-seconds S` | 3.0 (:409) | seconds of idle sim time | `S<=0` -> invalid (:1217); explicit negative impossible | idle_time threshold, gource.cpp:1749 |
| `--disable-auto-skip` | off | - | sets auto_skip_seconds = -1.0 (:716-718); processed BEFORE `-a` (line 1211) so `-a` re-enables | gource.cpp:1749 (`>=0.0` guard) |
| `--stop-at-time S` | -1.0 (:395) | sim seconds (runtime) | `S<=0` -> invalid (:1347) | gource.cpp:261 |
| `--stop-position F` | 0.0 (:394) | fraction of log | `F<=0 || F>1` -> error (:1336) | readLog gource.cpp:1175 |
| `--stop-at-end` | false (:397) | - | bool | readLog gource.cpp:1172 |
| `--dont-stop` | false (:398) | - | bool | ONLY gource_shell.cpp:224 (exporter auto stop_at_end) |
| `--stop-on-idle` | false (:396) | - | parsed, no effect ("no longer does anything", :1376) | none |
| `--loop` | false (:415) | - | bool | gource.cpp:1707; gource_shell.cpp:224 |
| `--loop-delay-seconds S` | 3.0 (:416) | sim seconds idle | `S<=0` -> invalid (:734) | gource.cpp:1708 |
| `--start-position F` | 0.0 (:393) | fraction | `random` -> `srand(time(0))`, `(rand()%1000)/1000.0f` (:1319-1320); else `F<=0 || F>=1` -> error (:1324) | seekTo, gource.cpp:1604 |
| `--start-date / --stop-date` | 0 (:387,390) | local-time date -> time_t | parseDateTime; stop date rounded to next day boundary (:1294-1306) (corrected tick 4: the rounding `stop_timestamp_rounded` only feeds the `stop_date` display string, :1299-1307; `stop_timestamp` used by readLog stays the raw parsed value) | logmill.cpp:83-93 (start; corrected tick 4: was 85-109, the start-timestamp skip loop is lines 83-93), gource.cpp:1148 (stop_timestamp: first commit with ts > stop -> stop_position_reached) |
| `--no-time-travel` | false (:399) | - | bool | gource.cpp:1758 |
| `-i, --file-idle-time S` | 0 (:411) | integer seconds (`atoi`) | `<0` or non-"0" parse-to-0 -> invalid (:1230) | file.cpp:201 vs `elapsed` (pawn.cpp:75, += scaled dt) |
| `--file-idle-time-at-end S` | 0 (:412) | integer seconds (`atoi`) | same (:1243) | copied into file_idle_time when `is_finished||stop_position_reached` (gource.cpp:1180-1182, mutates the global) |
| `--user-idle-time S` | 3.0 (:471) | seconds | `<0` invalid (:1254) | user.cpp:346-358 vs `elapsed` |
| `--max-file-lag S` | 5.0 (:469) | seconds of runtime | `==0` invalid (:1406); negative allowed and disables lag (user.cpp:247 `>=0.0` guard) | user.cpp:247, compares `action->t` (runtime at creation) with `t - lag` |
| `-r, --output-framerate F` | 60 (core/settings.cpp:89) | fps | must be 25, 30 or 60 (core/settings.cpp:464-466) | setFrameExporter, gource.cpp:505-522 |
| `-o, --output-ppm-stream` | none | - | - | main.cpp:188 creates exporter; window not resizable (main.cpp:147) |
| `--no-vsync`/vsync | vsync off by default in display (display.cpp:43), set from settings main.cpp:144 | - | - | swap interval display.cpp:265; affects wall-clock pacing only |
| commitqueue_max_size | 100 (gource.cpp:919) | commits | internal | readLog look-ahead, :1137 |
| `max_tick_rate` | 1/60 | s | internal | gource.cpp:156, 519 |

Multi-repo (`repo_count > 1`, from gource_shell.cpp:228-234): if `stop_at_time<=0 && stop_position<=0` then `stop_at_time = 60.0f`. Settings re-imported per repo (:218) so keys that mutated `gGourceSettings` (time_scale, days_per_second, file_idle_time) are reset on every re-import: importGourceSettings calls setGourceDefaults first (gource_settings.cpp:368-520), so each per-repo option returns to its default before the section's keys apply; misc-traces.md lists the keys that are NOT reset (corrected tick 4).

## 3. Exporter (fixed-dt) vs interactive, side by side

| Aspect | Interactive (no exporter) | Exporter (`-o`) |
|---|---|---|
| Real-time throttle | `min_delta_msec=16` accumulation (sdlapp.cpp:429-436), vsync optional | same loop, same 16 ms throttle (wall-clock paced), vsync as set; Rust may skip it |
| `dt` entering `Gource::update` | wall delta (>=16 ms, f32 sec) | same value but discarded for sim |
| `scaled_dt` | `min(dt, 1/60) * time_scale` (:248,256). With 16 ms min the clamp bites only when frames take >16.67 ms | `max_tick_rate * time_scale`, `max_tick_rate = 1.0f/gource_fps` (:519) |
| gource_fps / frameskip | n/a (`max_tick_rate`=1/60) | `g=F; skip=0; while(g<60){g+=F; skip++}`: F=60 -> g60 skip0; F=30 -> g60 skip1; F=25 -> g75 skip2 (:507-517) |
| Frame dumped | never | when `commitlog!=0 && !shutdown && framecount % (frameskip+1)==0` (:272-273); framecount counts every update from setFrameExporter (called in ctor :165, resets to 0 :510) including log-loading frames |
| Cursor | `cursor.logic(dt)` real dt | same (real dt; irrelevant to sim/output unless mouse shown) |
| Auto stop at end | only if `stop_at_end`/`stop_position`/`stop_at_time` | `stop_at_end=true` forced unless `dont_stop || loop || path=="-"` (gource_shell.cpp:224-225) |
| Multi-repo | wraps around to first repo after last (gource_shell.cpp:239-243) | no wrap: runs each once, then `getNext()` returns 0 |
| Determinism | wall-clock dependent | sim deterministic EXCEPT: frames spent before `commitlog` becomes available (async logmill thread) advance `runtime` and `framecount` by an amount that depends on load time, so dump phase (`framecount % (skip+1)`) and runtime offset can differ run to run. A Rust port should define this explicitly (e.g. block on log load, framecount phase 0 at first loaded frame) and note the divergence. |

## 4. Stop conditions and post-stop frames

`stop_position_reached` (member, init false gource.cpp:107, never reset by `reset()`) is set by:
1. `stop_at_time > 0 && runtime >= stop_at_time` (:261), checked every update, including while paused/loading (runtime frozen when paused so no new trigger).
2. `stop_timestamp != 0 && commit.timestamp > stop_timestamp` in readLog (:1148-1150).
3. `stop_at_end && commitlog->isFinished()` (:1172).
4. `stop_position > 0 && commitlog->isSeekable() && (isFinished || last_percent >= stop_position)` (:1175).
(3 and 4 evaluated at the end of every `readLog()` call, which is only called when `commitqueue.empty()` and not paused, :1702-1704.)

Effects once set:
- `readLog()` returns immediately (:1132): no more commits are queued; commits already in `commitqueue` still replay as `currtime` advances.
- Auto-skip is disabled (:1749), so remaining queued commits only appear at real sim rate.
- `loop` seek can still fire if the queue empties and idle >= loop_delay (:1707-1712); readLog then returns immediately, so it loops visually but gets no commits (edge case; stop_position_reached stays true).
- `file_idle_time = file_idle_time_at_end` if that is `>0` (:1180-1182).

Termination: `appFinished` (the Gource's, from `SDLApp`) is set when `users.empty() && stop_position_reached` inside `updateUsers` (:1401), i.e. not while paused, not until every user has faded out (user inactivity after `user_idle_time`, plus fade; user.cpp:346-358) and been deleted (:1413). Also set by `shutdown && logmill->isFinished()` (:1570), where `shutdown` is set by `GourceShell::quit()` on SDL_QUIT/ESC (gource_shell.cpp:200-203).

Frames after stop: normal frames keep being simulated, drawn and (if exporting) dumped every `frameskip+1` updates while users fade out. The frame in which `appFinished` is set is still drawn and dumped (the dump at :272 follows logic/draw in the same `update`). The next `SDLApp` iteration calls `GourceShell::update`, sees `gource->isFinished()`, calls `getNext()` and returns without update/draw/dump (gource_shell.cpp:285-288); with a single repo or exporter, `getNext()` returns 0 so `appFinished=true` and the loop exits after one more `display.update()`. With interactive multi-repo it creates the next Gource (stop_at_time=60 default) and the transition texture blends the last frame for `transition_interval` (1.0 s wall dt, :208,311). With `shutdown` set, dumping stops immediately (:272). Frame exporter finalises in `~GourceShell`/main (main.cpp:232); `frameExporter->stop()` only on the no-log error path (gource.cpp:1593).

Edge: stopping at time `T` does not mean the last frame is at `runtime == T`; the run continues until users are gone, typically `user_idle_time` (3.0 s) + 1.0 s fade (user.cpp:346-347) later, scaled by time_scale since idle uses `elapsed += scaled dt`. Not stopping automatically on interactive single repo without any stop option (no end trigger).

## 5. Wall-clock / system-time entry points

| Where | What | Affects sim? |
|---|---|---|
| sdlapp.cpp:414,424 `SDL_GetTicks` | frame delta -> `dt`, `t` | Interactive: YES (dt, clamped to 1/60 then scaled). Exporter: no (fixed tick), but wall delta still reaches `GourceShell` transition (`toggle_delay`, `transition_interval`, gource_shell.cpp:294,280) and `cursor.logic(dt)` |
| sdlapp.cpp:431 `SDL_Delay(1)` | busy wait | pacing only |
| gource.cpp:263,267,1301,1332,1426,1448, 2066-2112, 2414-2545 `SDL_GetTicks` | perf counters for debug overlay | no |
| core/timer.cpp:37,49 `SDL_GetTicks` | query timers (debug) | no |
| gource.cpp:1465 `localtime` | date display text (TZ dependent) | no (display) |
| gource.cpp:298 `localtime` | slider hover date | no |
| gource_settings.cpp:1280,1297,1306 `localtime` ; core/settings.cpp:331-357 `mktime_utc`/`mktime` | start/stop date parsing: local TZ (settings.cpp:357 uses `mktime` local) | YES (filters commits; TZ dependent) |
| gource_settings.cpp:1319 `srand(time(0))` | seeds libc RNG only for `--start-position random` | YES (start position and the global `rand()` stream); otherwise `rand()` is never seeded (default seed 1) |
| gource.cpp:1555 `rand()` | `gStringHashSeed` | indirectly; also user.cpp:106,128, dirnode.cpp:625 |
| formats/*.cpp `mktime`/`timegm` (cvs2cl:128, apache:81, cvs-exp:82, bzr:117, svn:239/241) | commit timestamp parsing from log text | YES (timestamp values; TZ dependent for non-UTC parsers) |
| `fps` (sdlapp.cpp:322, updateFramerate :319-327; corrected tick 4: was :321) | measured fps, copied to `gource->fps` (gource_shell.cpp:291) | display only |

## 6. Contradictions in the four docs and the correct statement

1. core-lib.md (:17, :77, :234) "min frame delta 8 ms / accumulate >= 8 ms": 8 ms is only the `SDLApp` default (sdlapp.cpp:317). `GourceShell` overrides it to 16 ms (gource_shell.cpp:29), so the shipped app uses 16 ms. Also the delta passed on is the whole accumulated buffer (can be >16), with no remainder carry (sdlapp.cpp:435-436). app-core.md and output-pipeline.md :30 already say 16 and are correct.
2. scene-model.md (:7, :358) "dt = min(dt, 1/60)*time_scale" is only the interactive path; with an exporter it is `max_tick_rate*time_scale` where `max_tick_rate=1/gource_fps` (1/75 for 25 fps output), not necessarily 1/60. scene-model.md :358 ("fixed 1/60 ... the frame-exporter path already does this") is wrong for 25 fps output (1/75). Also in practice, with the 16 ms floor, the 1/60 clamp reduces dt only when a frame takes >16.67 ms.
3. output-pipeline.md :26 / task framing "gource_fps is output_framerate rounded up past 60": the loop is `while(g<60)`, so 30 -> exactly 60 (not past), 60 -> 60, 25 -> 75. Correct wording: smallest multiple of F that is >= 60. (The table's examples are right.) Also `--stop-at-time` cite should be gource_settings.cpp:1341-1350; output-pipeline.md :40 note "deterministic under export" holds for runtime but see section 3 on load-time frames.
4. output-pipeline.md :30 "loop is still wall-clock throttled; sim dt fixed": correct, but note it must not be read as "dt is irrelevant": wall `dt` still drives `cursor.logic`, shell `toggle_delay` and transition blend.
5. app-core.md :233 "logic()/draw() still run when paused with nonzero scaled_dt": true, but only `runtime` is frozen; `scaled_dt` still reaches camera/slider/key logic and the `paused` branch returns before `idle_time` accumulates (updateUsers not called), so idle_time, `appFinished` check and auto-skip are all frozen while paused (gource.cpp:1693-1699, :1401-1410). app-core.md :410 leaves "idle_time += dt in interactUsers/updateUsers?" unresolved: it is `updateUsers` (gource.cpp:1365-1410).
6. app-core.md :429 "verify in sdlapp.cpp whether dt fixed during frame export": it is NOT fixed at the SDLApp level; the fixing happens inside `Gource::update` (:251-253). Wall dt still flows to the shell and cursor.
7. app-core.md :299 states `=`/`+` caps days_per_second at 30 and `-` floors at 0; verified correct (gource.cpp:821-835). app-core.md :361/:304: time_scale key cap is 4, and `,` uses `max(0, floor-1)` for values >1 so 1.5 -> 0 (not 0.25); verified (:864-868). Note settings validation (0 < S <= 4, :1266) does not apply to key edits.
8. scene-model.md :344 and app-core.md :498 say `SDL_GetTicks` is used "only for debug timing" in gource.cpp: true for gource.cpp, but the frame dt itself comes from `SDL_GetTicks` in sdlapp.cpp (interactive sim depends on it). scene-model.md :344 states this correctly in the second column; app-core.md :498 omits it.
9. Missing from all four: (a) frames during log load advance `runtime`/`framecount` (nondeterministic in exporter mode, section 3; Rust waits for the log to load first, D2 decided 2026-10-31, see parity-test-spec.md 2.1 frame phase); (b) `--realtime` overrides `--seconds-per-day` and `--disable-auto-skip` is overridden by `-a` (order of processing in gource_settings.cpp:716, 1211, 1360); (c) `file_idle_time` / `file_idle_time_at_end` are `atoi` integers, not floats (gource_settings.cpp:1228,1241); (d) `--stop-on-idle` is a no-op (:1376); (e) `dont_stop` only has an effect in `GourceShell::getNext` (gource_shell.cpp:224).

## Verification (tick 4)

Checked against sdlapp.cpp, gource_shell.cpp, gource.cpp, gource_settings.cpp, plus core/settings.cpp, main.cpp, display.cpp, logmill.cpp, user/file/pawn/dirnode cites (spot-checked).

- Confirmed (verified tick 4): min_delta_msec 16 override (gource_shell.cpp:29) vs default 8 (sdlapp.cpp:317); whole accumulated buffer becomes dt, no remainder (sdlapp.cpp:435-436); exporter `scaled_dt = max_tick_rate` with `max_tick_rate = 1.0f/gource_fps` (gource.cpp:519), gource_fps loop `while(<60) += F` and frameskip (505-522; 60->skip0, 30->60/skip1, 25->75/skip2); output_framerate default 60 and 25/30/60 validation (core/settings.cpp:89,464-466); dump condition and phase (gource.cpp:272-276), framecount reset at setFrameExporter (:510) and incremented every update including log-loading frames; `runtime` advances while commitlog==0 and `logic` returns at :1579-1581; appFinished conditions (:1401, :1570, gource_shell.cpp:286); pause semantics (:259, :1693-1699, updateUsers idle_time :1406-1410); all time-setting defaults (gource_settings.cpp:387-416, 469, 471) and validations (:1203, 1217, 1230, 1243, 1254, 1266, 1319-1324, 1336, 1347, 1360, 1376, 1406, 734); key handlers (gource.cpp:821-873); max_tick_rate is f64 literal narrowed to `float` member (gource.h:141).
- Cites: about 110 path:line cites checked; about 105 correct; 6 corrected (sdlapp.cpp :457/:460/:461 shifts, sdlapp.cpp:322, logmill.cpp range) plus 1 wrong statement (stop-date rounding affects only the display string).
- Not confirmed: formats/*.cpp cites (cvs2cl:128, apache:81, cvs-exp:82, bzr:117, svn:239/241) confirmed by grep only, not read in context; gource.cpp:2414-2545 and the 11 hidden SDL_GetTicks matches not individually opened; keys block start lines 853/873 inferred from a contiguous read (not line-numbered).
