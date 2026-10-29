# Settings / CLI / config-file surface (Gource 0.57)

Sources: `src/main.cpp` (M), `src/gource_settings.cpp` (GS), `src/gource_settings.h` (GH), `src/core/settings.cpp` (CS), `src/core/settings.h`, `src/core/conffile.cpp` (CF), `src/core/sdlapp.cpp` (SA), `src/gource_shell.cpp` (SH), `README.md`, `data/gource.1`, `cmd/`, `tests/test.conf`.
Version string: `GOURCE_VERSION "0.57"` (GH:21).

## 0. Architecture in one paragraph

CLI args are never applied directly to settings. `parseArgs` (CS:178) converts each option into a **ConfFile entry** (section `gource` or `display`), except "command-line" options which run immediately. The ConfFile (loaded `--load-config` file + CLI overrides) is then **imported** by `importDisplaySettings` (CS:365) and `importGourceSettings` (GS:611), which do all type conversion and validation. `importGourceSettings` is re-run per `[gource]` section by the shell when multiple repos are configured (SH:218). A Rust port should model: `ConfFile = ordered map<section, Vec<Section>>`, `Section = map<key, Vec<Entry{value:String,line:int}>>`, and keep the CLI-to-conf-to-import split, because `--save-config` writes the ConfFile (not the settings struct).

## 1. Program flow (main.cpp)

| Step | Behaviour | Source |
|---|---|---|
| 1 | `SDLAppInit("Gource","gource",argv[0])` (exe path only on non-Windows). `gSDLAppExec="gource"` is the prefix of every error message | M:22-29, SA:178 |
| 2 | Windows only: `SDLApp::initConsole()` | M:31-33, SA:60 |
| 3 | `parseArgs(argc, argv, conf, &files)` first pass: fills `conf` + collects positional `files` | M:43 |
| 4 | If no `--load-config` and positional files exist: scan **all** positionals in order; first whose name ends in `.conf` (len>5), `.cfg` or `.ini` (len>4) AND loads successfully via `ConfFile::load` (any ConfFileException => "not a config") becomes `load_config` and is erased from `files`; break | M:45-72 |
| 5 | `Logger::getDefault()->setLevel(log_level)` (default `LOG_LEVEL_OFF`, GS:213) | M:75 |
| 6 | Windows only: if log level OFF and not an existing console => hide console | M:77-82 |
| 7 | If `load_config` set: `conf.clear()`, `conf.load(load_config)`, then **re-run `parseArgs` (no files ptr)** so CLI overrides the file | M:85-91 |
| 8 | If positionals remain: path = **last** positional; written (setEntry "path") into **every** `[gource]` section, or a new `[gource]` section if none | M:94-106 |
| 9 | `importDisplaySettings(conf)`, `importGourceSettings(conf)` (first/only `[gource]` section) - validates everything, errors throw ConfFileException | M:109-110 |
| 10 | `--save-config FILE`: `conf.save(FILE)` then `exit(0)` (after validation; no window) | M:113-116 |
| 11 | `--output-custom-log FILE` with non-empty path: `Gource::writeCustomLog(path, file)` then `exit(0)` | M:119-123 |
| 12 | Any `ConfFileException` in 3-11: `SDLAppQuit(what())` | M:125-128 |
| 13 | Display flags: frameless, multisample(4) if `multi-sampling`, alpha if `transparent`, vsync, resizable only if no ppm output | M:131-149 |
| 14 | High DPI: `high_dpi=true` always; on non-Windows set false if `viewport_specified && !--high-dpi` | M:151-163 |
| 15 | `display.init("Gource", w, h, fullscreen, screen)`; non-fullscreen + window_x/y >=0 => `SDL_SetWindowPosition`; hint MINIMIZE_ON_FOCUS_LOSS=0. SDLInitException => `SDLAppQuit("SDL initialization failed - <what>")` | M:165-183 |
| 16 | PPM exporter: failure => `SDLAppQuit("could not write to '<what>'")` | M:188-201 |
| 17 | `ResourceException` => `SDLAppQuit("failed to load resource '<what>'")`; `SDLAppException` => help if `showHelp()` else `SDLAppQuit(what)` | M:213-227 |

Exit codes: `SDLAppQuit` => exit(1); help / `*-command` / `--save-config` / `--output-custom-log` => exit(0); normal run returns 0 (M:237).

### Error output format (SA:135-153)
stderr, exactly:
```
gource: <message>
Try 'gource --help' for more information.
<blank line>
```
(second line followed by an extra `\n`). Windows: calls `showConsole(true)` first and, if `!existing_console`, prints `Press Enter` to stderr and waits for `getchar()`. Also `SDL_Quit()` first.

`SDLAppInfo(msg)` (SA:117-132) prints `msg\n` to **stdout**, Windows prints `\nPress Enter\n` + getchar if not existing console, then `exit(0)`.

## 2. Config-file location and loading

- **No default config path is searched.** Config is loaded only via `--load-config FILE` or a positional argument that looks like one (section 1 step 4). `gSDLAppConfDir` is computed (SA:182-256; cwd+"/" or exe dir on Windows, overridden by `SDLAPP_CONF_DIR` define) but never used to locate a gource config (grep: only definition). No env vars read for config (`getenv` used only for TERM in SA:70 and TZ in timezone.cpp:13).
- Relative paths are resolved against the process cwd.
- `--load-config` / `--save-config` / `--output-custom-log` with empty value fall through to `invalid <name> value` (GS:539-586,603).
- `--save-config`: ConfFile::save (CF:452-487) opens with `std::ofstream` (truncate), failure => `failed to write config to <file>`. Output order: sections sorted by name (std::map), repeated sections in insertion order; entries sorted by key (std::map), repeated keys in insertion order. Each section printed as `[name]\n` then `key=value\n` per entry then blank line (CF:395-411). Values are raw strings as they came from CLI/config (no normalisation; e.g. `1280x720` stays as typed). If no `[gource]` section existed, `importGourceSettings` adds an empty one (GS:617-619) so `[gource]` is always saved. `exportDisplaySettings` (CS:92) is **dead code** (never called).
- Because the saved file's first `[display]` section comes from CLI keys, saved config can contain keys `viewport, windowed, fullscreen, ...` under `[display]` and everything else under `[gource]`.

### 2.1 Grammar (CF:494-571)
Line-based (`std::getline`; line numbers 1-based):
1. Empty line (size 0) or first char `#` => skipped. **Comments only recognised at column 0**; `  # x` is not a comment (it fails to parse unless it contains `=`). A line of just `\r` (CRLF file blank line) is not empty and **fails** with "could not parse line" (trailing `\r` is otherwise trimmed from values).
2. Section: regex `^\s*\[([^\]]+)\]\s*$`. Starts a new section (previous one is added to the file). Sections with the same name are allowed and kept as a list (multi-repo).
3. Key/value: regex `^\s*([^=\s]+)\s*=\s*([^\s].*)?$`. Key = non-space, non-`=` run. Value = remainder starting at first non-space char (leading whitespace stripped by regex), trailing whitespace `" \t\f\v\n\r"` trimmed (CF:496-505). Value may be empty (`key=`). **No quoting, no escaping, no inline comments** (`#` inside a value is kept; `=` inside a value is fine because key excludes `=`). Entry before any section goes into an unnamed section `""` (CF:555) which is never read by settings.
4. Anything else: `ConfFileException("<file>, line <n>: could not parse line", file, n)`.
5. Repeated keys in one section: all kept in order (`addEntry`); single-valued settings read only `getEntry` = **first** one; multi-value settings read all.
6. Unknown keys / unknown sections are **silently ignored** by import (no validation of key names anywhere). Only CLI rejects unknown options.
7. Open failure: `failed to open config file <file>` (CF:520), lineno 0.
8. `ConfFile::load()` clears prior contents first (CF:510).

### 2.2 Value syntaxes (CF:25-34, 126-223)
| Kind | Rule |
|---|---|
| bool | true if exactly one of `1 true True TRUE yes Yes YES`; **everything else is false** (including `on`, empty, garbage, no error). Missing key => false. isBool lists also `0 false False FALSE no No NO` but is unused by settings |
| int | `atoi` (garbage => 0, leading ws ok, `12abc`=>12) |
| float | `atof` |
| string | raw |
| colour | accepts `vec3(r, g, b)` (regex `^\s*vec3\(\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*\)\s*$`, components 0..1 floats, no clamping) **or** exactly 6 chars with `sscanf("%02x%02x%02x")` == 3 => /255 (e.g. `FF8800`; `#` prefix not accepted; sscanf %x tolerates e.g. `+1`/`0x`-style forms only within the 6 chars). Else `invalid '<key>' value` |
| vec2/vec4 | `vec2(x, y)`, `vec4(...)` same style; `getVec*` returns zeros if no match |
| written floats | `%.5f` ; written bools `yes`/`no` (CF:49-124; only used by dead exporter) |

### 2.3 ConfFileException messages (CF:752-838)
`entryException(entry, reason)` message = `<conf filename>[, line <n>]: <reason>`; the file/line prefix only if the ConfFile has a filename (i.e. a config file was loaded or `save` set one) and the entry's line != 0 (CLI-derived entries have line 0 so only `<file>: ` prefix appears when a config is loaded, nothing otherwise). Helpers:
- unknown option `'<name>'`; no value specified for `'<name>'` (unused by import except hide); invalid `'<name>'` value (`invalidValueException`); section `'<s>' missing required entry '<e>'` (unused by settings).
- CLI-time errors thrown directly from parseArgs have no prefix: `unknown option <name>` (CS:238, no quotes, name has hyphens/alias stripped).
- Note: with a loaded config and a CLI-provided bad value, the message is prefixed by the **config filename** (conffile non-empty) even though the value came from the CLI.

## 3. CLI parsing rules (CS:166-282)

1. `argv[1..]` only. For each token: strip **all** leading `-` characters while `size>1` (so `-x`, `--x`, `---x` identical); `is_option` = at least one hyphen removed. A lone `-` stays `-` and is **positional** (STDIN path). `--` => becomes `-` with is_option=true => treated as option name `-` => `unknown option -`. Empty token skipped.
2. Positional (non-option): pushed to `files` (only in the first pass; second pass passes `files=0` and drops them).
3. Alias translation via `arg_aliases` (applied before the viewport check, case-sensitive; `-H` != `-h`).
4. **Magic viewport**: if token (post-alias) has size>1 and contains `x`, try `^([0-9.]+)x([0-9.]+)(!)?$` with both ints (atoi) >0 => sets `[display] viewport=<token as typed>` (e.g. `1280x720!`), `continue`. Non-match falls through to normal lookup (so `--max-files` etc. are unaffected). Executes even if `--viewport` was also given; last wins (setEntry).
5. Unknown option (not in `arg_types`) => throw `unknown option <name>` (exit 1 via main's catch).
6. Value: `bool` type => value `"true"`, consumes nothing. Any other type => **consumes the next argv token unconditionally** (even if it starts with `-`, e.g. `-p -0.5`); if there is none, value is `""`. No `--opt=value` syntax (token `--opt=value` => unknown option `opt=value`). No numeric/type validation at parse time: `int`/`float`/`string`/`multi-value` only differ in multi-value handling.
7. Section: `conf_sections[name]` if present else `default_section_name` (`gource` for Gource; base default `settings` is overridden in GS:219).
8. Section `command-line` => `commandLineOption(name,value)` runs immediately, **in both passes** (so e.g. `--log-command git` prints and exits during the first pass, before any config loading).
9. Otherwise applied to **all** existing sections of that name (creating one if none): `multi-value` => `addEntry` (append; so CLI filters are added to those in a loaded config), others => `setEntry` (replaces **all** entries of that key in that section). This is the CLI-over-config precedence rule.
10. Repeat-option last-wins for single values; `--follow-user`, `--highlight-user`, `--user-filter`, `--user-show-filter`, `--file-filter`, `--file-show-filter` accumulate.

### 3.1 Precedence summary
defaults (`setDisplayDefaults`/`setGourceDefaults`, re-applied at start of each import) < config file value < CLI value (setEntry overwrite) ; multi-value: config entries + CLI entries concatenated (config first). Positional path overrides `path=` in config and in CLI `--path` (positional applied last, M:94-106). `--path PATH` is just a normal `gource` string option (help-extended only). Multi `[gource]` sections: CLI options applied to each; positional path applied to each (so all repos become the same path).

### 3.2 Command-line-only options (never saved/loaded in config) - `commandLineOption` GS:529-605
| Option | Type | Behaviour | Source |
|---|---|---|---|
| `-h`, `-?`, `--help` | bool | `help(false)`; exits 0 | GS:531, aliases 228-229 |
| `-H`, `--extended-help` | bool | `help(true)`; exits 0 | GS:535, 230 |
| `--load-config FILE` | string | sets `load_config` (ignored if empty -> falls to `invalid load-config value`) | GS:539 |
| `--save-config FILE` | string | sets `save_config` | GS:544 |
| `--log-command VCS` | string | VCS in `git`,`cvs-exp`,`cvs2cl`,`svn`,`hg`,`bzr` => print that VCS's log command via `SDLAppInfo` (stdout, exit 0); `cvs` => error `please use either 'cvs2cl' or 'cvs-exp'`; any other value (incl. empty, `apache`, `custom`) falls through to `invalid log-command value` | GS:549-580,603 |
| `--git-log-command`, `--cvs-exp-command`, `--cvs2cl-command`, `--svn-log-command`, `--hg-log-command`, `--bzr-log-command` | bool | undocumented shortcuts for the above | GS:555-581,242-247,287-292 |
| `--output-custom-log FILE` | string | sets `output_custom_filename`; see flow step 11 | GS:583 |
| `--log-level LEVEL` | string | `warn`,`debug`,`info`,`error`,`pedantic`; any other value silently ignored (stays OFF). Undocumented | GS:588-601 |

Log command strings come from `GitCommitLog::logCommand()` etc. (formats/*.cpp; belongs to the format-parsers subtask), e.g. git: `git log --reverse --raw --encoding=UTF-8 --no-renames ...` (src/formats/git.cpp:88).

## 4. Display options (section `[display]`) - CS:26-65, 365-470

| CLI | Alias | Type | Conf key (section display) | Import rule / errors | Default | Source |
|---|---|---|---|---|---|---|
| `-WxH[!]` (magic) / `--viewport WxH[!]` | - | string | `viewport` | regex above; both >0 else `invalid 'viewport' value`; `!` => `resizable=false`; sets `viewport_specified` | 1024x768 (**1024x640 on `__APPLE__`**, CS:69-73) | CS:149-163,377-394 |
| `-f`, `--fullscreen` | f | bool | `fullscreen` | true => fullscreen | false | CS:416 |
| `-w`, `--windowed` | w | bool | `windowed` | true => fullscreen=false (evaluated **after** fullscreen so wins) | - | CS:420 |
| `--screen N` | - | int | `screen` | `atoi`; `<1` => `invalid 'screen' value` | -1 | CS:404 |
| `--window-position XxY` | - | string | `window-position` | regex `^([0-9.]+)x([0-9.]+)$` (atoi each; zero allowed) else invalid | -1,-1 (unset) | CS:396 |
| `--frameless` | - | bool | `frameless` | applied only if not fullscreen | false | CS:424 |
| `--transparent` | - | bool | `transparent` | | false | CS:434 |
| `--multi-sampling` | - | bool | `multi-sampling` | main uses MSAA 4 | false | CS:412, M:134 |
| `--no-vsync` | - | bool | `no-vsync` | vsync=false | true | CS:438 |
| `--high-dpi` | - | bool | `high-dpi` | see flow step 14 | false | CS:442 |
| `-o`, `--output-ppm-stream FILE` | o | string | `output-ppm-stream` | empty value => error `specify ppm output file or '-' for stdout`; `-` = stdout | "" | CS:446 |
| `-r`, `--output-framerate FPS` | r | int | `output-framerate` | empty => `specify framerate (25,30,60)`; not 25/30/60 => `supported framerates are 25,30,60` | 60 | CS:456 |

Fullscreen with no viewport => width/height set to 0,0 ("desktop resolution", CS:429-432). `resizable` is only re-enabled by `setDisplayDefaults` (called at the start of each import). Only the **first** `[display]` section is read by import, while CLI display options go to all of them. `importDisplaySettings` calls `setDisplayDefaults()` first (and returns early with defaults if no `[display]` section; note `fullscreen` stays false then).
`--high-dpi` is hidden from help on Windows (GS:59-61); on Windows main always enables high DPI awareness.
`-WIDTHxHEIGHT, --viewport` and the README doc: "Add ! to make the window non-resizable" (README:35).

## 5. Gource options (section `[gource]`) - GS:211-366 (types), 611-1667 (import)

Defaults from `setGourceDefaults` (GS:368-520). "Req value" = empty value raises the quoted error via `entryException`. All errors are `ConfFileException` => `SDLAppQuit` exit 1. Range errors use `invalidValueException` => `invalid '<key>' value` unless a custom text is shown.

### 5.1 Numeric / timing

| CLI (alias) | Type | Conf key | Setting (type) | Default | Validation / conversion / error | Src |
|---|---|---|---|---|---|---|
| `-s`, `--seconds-per-day` | float | `seconds-per-day` | `days_per_second = 1/x` | 0.1 (=10 s/day) | req `specify seconds-per-day (seconds)`; `<=0` invalid | 1197 |
| `--realtime` | bool | `realtime` | `days_per_second = 1/86400` | - | evaluated after seconds-per-day so wins | 1360 |
| `-a`, `--auto-skip-seconds` | float | `auto-skip-seconds` | `auto_skip_seconds` | 3.0 | req `specify auto-skip-seconds (seconds)`; `<=0` invalid | 1211 |
| `--disable-auto-skip` | bool | `disable-auto-skip` | `auto_skip_seconds=-1` | - | **evaluated before** auto-skip-seconds (GS:716 vs 1211) so a given auto-skip-seconds overrides it | 716 |
| `-t`, `--stop-at-time` | float | `stop-at-time` | `stop_at_time` | -1 | req `specify stop-at-time (seconds)`; `<=0` invalid | 1341 |
| `-p`, `--start-position` | string | `start-position` | `start_position` | 0 | req `specify start-position (float,random)`; `random` => `srand(time(0)); (rand()%1000)/1000.0`; else float must be `0<x<1` else `start-position outside of range 0.0 - 1.0 (non-inclusive)` | 1314 |
| `--stop-position` | string | `stop-position` | `stop_position` | 0 | req `specify stop-position (float)`; `<=0 or >1` => `stop-position outside of range 0.0 - 1.0 (inclusive)` | 1330 |
| `--start-date` | string | `start-date` | `start_timestamp`,`start_date`(YYYY-MM-DD local) | 0/"" | req `specify start-date (YYYY-MM-DD hh:mm:ss)`; `parseDateTime` fail => invalid | 1271 |
| `--stop-date` | string | `stop-date` | `stop_timestamp`,`stop_date` | 0/"" | req `specify stop-date (YYYY-MM-DD hh:mm:ss)`; invalid on parse fail; `stop_date` = local date, **+1 day if local time-of-day is non-zero** (rounds up) | 1288 |
| `-c`, `--time-scale` | float | `time-scale` | `time_scale` | 1.0 | req `specify time-scale (scale)`; `<=0 or >4` => `time-scale outside of range 0.0 - 4.0` | 1259 |
| `-e`, `--elasticity` | float | `elasticity` | `elasticity` | 0.0 | req `specify elasticity (float)`; `<=0` invalid (so 0 cannot be passed explicitly) | 974 |
| `-i`, `--file-idle-time` | string | `file-idle-time` | `file_idle_time` (float from atoi) | 0 | req `specify file-idle-time (seconds)`; `atoi` result `<0`, or `==0` with first char not `'0'` => invalid (truncates decimals: `2.5`=>2) | 1222 |
| `--file-idle-time-at-end` | string | `file-idle-time-at-end` | `file_idle_time_at_end` | 0 | same rule, `specify file-idle-time-at-end (seconds)` | 1235 |
| (conf only; no CLI) | float | `user-idle-time` | `user_idle_time` | 3.0 | req `specify user-idle-time (seconds)`; `<0` invalid. **Not in `arg_types`: CLI `--user-idle-time` => `unknown option user-idle-time`** | 1248 |
| `--loop-delay-seconds` | float | `loop-delay-seconds` | `loop_delay_seconds` | 3.0 | req `specify loop-delay-seconds (float)`; `<=0` invalid | 728 |
| `--max-files` | int | `max-files` | `max_files` | 0 (no limit) | req `specify max-files (number)`; `<0`, or `==0` and string != `"0"` => invalid | 1389 |
| `--max-file-lag` | string | `max-file-lag` | `max_file_lag` | 5.0 | req `specify max-file-lag (seconds)`; only `==0` invalid; **negative accepted, `-1` = no limit** (README:275; user.cpp:247 tests `>=0`) | 1400 |
| `--max-user-speed` | float | `max-user-speed` | `max_user_speed` | 500 | req `specify max-user-speed (units)`; `<=0` invalid | 1435 |
| `--user-friction` | float | `user-friction` | `user_friction = 1/x` | 1.0 | req `specify user-friction (seconds)`; `<=0` invalid. Default stored 1.0 (=1/1.0) although help says "default: 0.67" | 1411 |
| `--user-scale` | string | `user-scale` | `user_scale` | 1.0 | req `specify user-scale (scale)`; `<=0 or >100` invalid | 1424 |
| `--filename-time` | float | `filename-time` | `filename_time` | 4.0 | req `specify duration to keep files on screen (float)`; `<2.0` => `filename-time must be >= 2.0` | 941 |
| `--bloom-intensity` | float | `bloom-intensity` | `bloom_intensity` | 0.75 | req `specify bloom-intensity (float)`; `<=0` invalid | 952 |
| `--bloom-multiplier` | float | `bloom-multiplier` | `bloom_multiplier` | 1.0 | req `specify bloom-multiplier (float)`; `<=0` invalid | 963 |
| `--padding` | float | `padding` | `padding` | 1.1 | req `specify padding (float)`; `<=0 or >=2` invalid | 1466 |
| `--hash-seed` | int | `hash-seed` | global `gStringHashSeed` (reset to 31 by every `setGourceDefaults`, GS:489) | 31 | req `specify hash seed (integer)`; any int accepted | 1062 |
| `--dir-name-depth` | int | `dir-name-depth` | `dir_name_depth` | 0 | req `specify dir-name-depth (depth)`; `<=0` invalid | 1607 |
| `--dir-name-position` | float | `dir-name-position` | `dir_name_position` | 0.5 | req `specify dir-name-position (float)`; `<0.1 or >1.0` => `dir-name-position outside of range 0.1 - 1.0 (inclusive)` | 1618 |

### 5.2 Fonts, colours, text, images

| CLI | Type | Conf key | Setting | Default | Validation / error | Src |
|---|---|---|---|---|---|---|
| `--font-file` | string | `font-file` | `font_file` | `GOURCE_FONT_FILE` define else `FreeSans.ttf` (GS:35-37, 446) | req `specify font file`; must exist (`boost::filesystem::exists`) else invalid; stored as `canonical()` absolute path | 985 |
| `--font-size` | int | `font-size` | `font_size` | 16 | req `specify font size`; <1 or >100 invalid | 1004 |
| `--file-font-size` | int | `file-font-size` | `filename_font_size` | 14 | same | 1015 |
| `--dir-font-size` | int | `dir-font-size` | `dirname_font_size` | 14 | same | 1026 |
| `--user-font-size` | int | `user-font-size` | `user_font_size` | 14 | same | 1037 |
| `--font-scale` | float | `font-scale` | `font_scale`, `default_font_scale=false` | 1.0 / true | req `specify font scale`; `<0 or >10` invalid. Scaled sizes: `clamp((int)(size*scale),1,100)` for font/user/dir/file (GS:522-527); `setScaledFontSizes` is called in defaults and after font-scale only (sizes are read before font-scale, so scaled sizes are correct). If `font-scale` is absent (`default_font_scale`), `Gource::Gource` sets `font_scale` = display DPI ratio x if >1.0 else `1 + max(0, display.height/1600)` and calls `setScaledFontSizes()` (gource.cpp:39-48) | | 1048 |
| `--font-colour` | string | `font-colour` | `font_colour` | 1,1,1 | colour rule (2.2); req `specify font colour (FFFFFF)` | 1069 |
| `-b`, `--background-colour` (alias `--background`) | string | `background-colour` | `background_colour` | 0.1,0.1,0.1 | req `specify background colour (FFFFFF)` | 1087 |
| `--highlight-colour` | string | `highlight-colour` | `highlight_colour` | 1,1,1 | req `specify highlight colour (FFFFFF)` | 1105 |
| `--selection-colour` | string | `selection-colour` | `selection_colour` | 1,1,0.3 | req `specify selection colour (FFFFFF)` | 1123 |
| `--dir-colour` | string | `dir-colour` | `dir_colour` | 1,1,1 | req `specify dir colour (FFFFFF)` | 1141 |
| `--filename-colour` | string | `filename-colour` | `filename_colour` | 1,1,1 | req `specify filename colour (FFFFFF)` | 924 |
| `--caption-colour` | string | `caption-colour` | `caption_colour` | 1,1,1 | req `specify caption colour (FFFFFF)` | 906 |
| `--background-image` | string | `background-image` | `background_image` | "" | req `specify background image (image path)`; existence NOT checked here | 1159 |
| `--title` | string | `title` | `title` | "" | req `specify title` | 1166 |
| `--logo` | string | `logo` | `logo` | "" | req `specify logo (image path)`; not checked here | 1173 |
| `--logo-offset XxY` | string | `logo-offset` | `logo_offset` vec2 | 20,20 | req `specify logo-offset (XxY)`; regex `^([0-9.]+)x([0-9.]+)$` else invalid | 1180 |
| `--date-format` | string | `date-format` | `date_format` | `%A, %d %B, %Y %X` | req => `no value specified for 'date-format'` (missingValueException) | 705 |
| `--caption-file` | string | `caption-file` | `caption_file` | "" | req `specify caption file (filename)`; must exist else `caption file not found` | 866 |
| `--caption-size` | int | `caption-size` | `caption_size` | 16 | req `specify caption size`; <1 or >100 invalid | 888 |
| `--caption-duration` | float | `caption-duration` | `caption_duration` | 10.0 | req `specify caption duration (seconds)`; `<=0` invalid | 877 |
| `--caption-offset` | int | `caption-offset` | `caption_offset` | 0 | req `specify caption offset`; any int | 899 |
| `--user-image-dir` | string | `user-image-dir` | `user_image_dir` (+ `/` appended if missing last char), `user_image_map` | "" | req `specify user-image-dir (directory)`; not a dir => `specified user-image-dir is not a directory`; dir read failure => `error reading specified user-image-dir`. Scans entries with ext (lowercased) `.jpg`,`.jpeg`,`.png`; key = filename without extension (case preserved), value = dir+filename; **macOS**: key NFC-normalised via CoreFoundation (GS:848-858); **Windows**: filename converted UTF-16->UTF-8 (GS:834-837) | 803 |
| `--default-user-image` | string | `default-user-image` | `default_user_image` | "" | req `specify default-user-image (image path)`; existence not checked here | 796 |
| `--colour-images` | bool | `colour-images` | `colour_user_images` | false | | 754 |
| `--fixed-user-size` | bool | `fixed-user-size` | `fixed_user_size` | false | | 1381 |

### 5.3 Behaviour / camera / hide / filters

| CLI | Type | Conf key | Setting | Default | Validation / error | Src |
|---|---|---|---|---|---|---|
| `--hide LIST` | string | `hide` | `hide_*` flags | none | empty => `no value specified for 'hide'`; comma separated (empty fields skipped, trailing `,` ok); allowed: `date users tree files usernames filenames dirnames bloom progress mouse root`; unknown => `unknown option hide <field>`. `mouse` also sets `hide_progress`. Parsing quirk: repeated commas collapsed; list is not trimmed of spaces | 627-703 |
| `--hide-date` `--hide-files` `--hide-users` `--hide-tree` `--hide-usernames` `--hide-filenames` `--hide-dirnames` `--hide-progress` `--hide-bloom` `--hide-mouse` `--hide-root` | bool | same names | same flags | false | undocumented; any `arg_types` key starting `hide-` of type bool is scanned generically (GS:673-681). Aliases (CLI only): `--disable-bloom`=>`hide-bloom`, `--disable-progress`=>`hide-progress` | GS:233-235,263-273 |
| `--loop` | bool | `loop` | `loop` | false | | 724 |
| `--stop-at-end` | bool | `stop-at-end` | `stop_at_end` | false | shell forces true when recording (`-o`) unless `dont-stop`/`loop`/path `-` (SH:224) | 1372 |
| `--dont-stop` | bool | `dont-stop` | `dont_stop` | false | | 1368 |
| `--stop-on-idle` | bool | `stop-on-idle` | `stop_on_idle` | false | **parsed but does nothing** ("NOTE: this no longer does anything", GS:1376); undocumented | 1377 |
| `--no-time-travel` | bool | `no-time-travel` | `no_time_travel` | false | | 1364 |
| `--author-time` | bool | `author-time` | `author_time` | false | | 1385 |
| `--key` | bool | `key` | `show_key` | false | | 1352 |
| `--ffp` | bool | `ffp` | `ffp` (fixed-function pipeline) | false | undocumented; forced true at runtime if no GLEW 2.0 (gource.cpp:33) | 1356 |
| `--disable-auto-rotate` | bool | `disable-auto-rotate` | `disable_auto_rotate` | false | | 712 |
| `--disable-input` | bool | `disable-input` | `disable_input` | false | | 720 |
| `--camera-mode` | string | `camera-mode` | `camera_mode` | `overview` | req `specify camera-mode (overview,track)`; else invalid | 1455 |
| `--crop AXIS` | string | `crop` | `crop_vertical`/`crop_horizontal` | none | req `specify crop (vertical,horizontal)`; else invalid | 758 |
| `--highlight-users` (alias `--highlight-all-users`) | bool | `highlight-users` or `highlight-all-users` (both read in conf) | `highlight_all_users` | false | | 1446 |
| `--highlight-dirs` | bool | `highlight-dirs` | `highlight_dirs` | false | | 1451 |
| `--highlight-user USER` | multi | `highlight-user` | `highlight_users[]` | [] | each: req `specify highlight-user (user)` | 1479 |
| `--follow-user USER` | multi | `follow-user` | `follow_users[]` | [] | each: req `specify follow-user (user)` | 1493 |
| `--file-extensions` | bool | `file-extensions` | `file_extensions` | false | | 1507 |
| `--file-extension-fallback` | bool | `file-extension-fallback` | `file_extension_fallback` | false | | 1511 |
| `--file-filter REGEX` | multi | `file-filter` | `file_filters[]` | [] | each: req `specify file-filter (regex)`; compile test `Regex(str,1)` (PCRE2, `pcre2_compile` flags 0, unanchored) fail => `invalid file-filter regular expression` | 1515 |
| `--file-show-filter REGEX` | multi | `file-show-filter` | `file_show_filters[]` | [] | `specify file-show-filter (regex)`; `invalid file-show-filter regular expression` | 1538 |
| `--user-filter REGEX` | multi | `user-filter` | `user_filters[]` | [] | `specify user-filter (regex)`; `invalid user-filter regular expression` | 1561 |
| `--user-show-filter REGEX` | multi | `user-show-filter` | `user_show_filters[]` | [] | `specify user-show-filter (regex)`; `invalid user-show-filter regular expression` | 1584 |
| `--git-branch` | string | `git-branch` | `git_branch` | "" | req => `no value specified for 'git-branch'`; PCRE `^(?!-)[/\w.,;_=+{}\[\]-]+$` else invalid (cannot start with `-`) | 739 |
| `--log-format VCS` | string | `log-format` | `log_format` | "" | req `specify log-format (format)`; `cvs` => `please use either 'cvs2cl' or 'cvs-exp'`; allowed `git cvs-exp cvs2cl svn custom hg bzr apache` else invalid. Note `apache` valid but not in help text | 773 |
| `--path PATH` / positional | string | `path` | `path`, `default_path=false` | `.` / true | see section 6 | 1629 |

(`-h` help lists "git,svn,hg,bzr,cvs2cl,custom" for log-format and "git,svn,hg,bzr,cvs2cl" for log-command; `cvs-exp` and `apache` accepted but unlisted.)

## 6. Path / repo resolution (GS:1629-1666, M:94-106)

- Positional: the **last** non-option, non-config-file token. Earlier extra positionals are silently dropped. Or `--path X` (conf key `path`).
- Default (no path): `path="."`, `default_path=true` => source layers try to generate a log from the cwd.
- `path == "-"`: requires `log_format` non-empty else error `log-format required when reading from STDIN`; then blocks until stdin has data: POSIX loop `while(std::cin.peek()==EOF && !std::cin.fail()) SDL_Delay(100)`, Windows loop with `PeekNamedPipe` on `STD_INPUT_HANDLE` until bytes available; then `std::cin.clear()`.
- Else if path non-empty and not `.`: one trailing `/` or `\` removed; must exist (`boost::filesystem::exists`) else `'<path>' does not appear to be a valid file or directory`.
- Whether path is a log file, VCS dir, etc. is decided later in the Gource class (commit log layer; other subtask).
- A positional ending `.conf/.cfg/.ini` that loads as a valid ConfFile is treated as config (README:63); a file with those extensions that fails to parse is treated as a log path. A positional config disables `files` use, so path then comes from the config's `path=`.
- Multiple `[gource]` sections (config file only; tests/test.conf): shell iterates them, `repo_count = countSection("gource")`; with >1 repos and no stop condition `stop_at_time = 60.0` is forced; after the last repo it loops back to the first unless recording (SH:213-245). Section settings re-imported with fresh defaults each time.

## 7. Help text (GS:42-209) - output must match byte for byte

Printed to **stdout** with `printf`, then `exit(0)`. Windows: first `SDLApp::resizeConsole(1040); showConsole(true)`; at end if `!existing_console` prints `Press Enter\n` and `getchar()`.
Layout rules: two-space indent for options; short option form `-x, --long`; options without short form are indented 6 spaces (`      --long`) in the first block; description column is at col 35 in the main block (aligned with `  -h, --help                       Help`), but several sub-blocks are **ad hoc aligned** (e.g. `--max-files NUMBER      Max...`, `--log-command VCS       Show...`, `--load-config CONF_FILE  Load...`, `-o, --output-ppm-stream FILE    Output...`), and the Extended block uses its own per-group alignment. Blocks separated by one blank line. The `--high-dpi` line is omitted on `_WIN32`. Extended block only for `-H`; the closing hint line `To see the full command line options use '-H'` only for `-h`.

The text below is the literal source (each `printf` line, `\n` escapes resolved; `[WIN]` conditionals noted). Reproduce exactly:

```
Gource v0.57
Usage: gource [options] [path]

Options:
  -h, --help                       Help

  -WIDTHxHEIGHT, --viewport        Set viewport size
  -f, --fullscreen                 Fullscreen
      --screen SCREEN              Screen number
      --multi-sampling             Enable multi-sampling
      --high-dpi                   Request a high DPI display          [omitted on Windows]
      --no-vsync                   Disable vsync

  --start-date 'YYYY-MM-DD hh:mm:ss +tz'  Start at a date and optional time
  --stop-date  'YYYY-MM-DD hh:mm:ss +tz'  Stop at a date and optional time

  -p, --start-position POSITION    Start at some position (0.0-1.0 or 'random')
      --stop-position  POSITION    Stop at some position
  -t, --stop-at-time SECONDS       Stop after a specified number of seconds
      --stop-at-end                Stop at end of the log
      --dont-stop                  Keep running after the end of the log
      --loop                       Loop at the end of the log

  -a, --auto-skip-seconds SECONDS  Auto skip to next entry if nothing happens
                                   for a number of seconds (default: 3)
      --disable-auto-skip          Disable auto skip
  -s, --seconds-per-day SECONDS    Speed in seconds per day (default: 10)
      --realtime                   Realtime playback speed
      --no-time-travel             Use the time of the last commit if the
                                   time of a commit is in the past
      --author-time                Use the timestamp of the author instead of
                                   the timestamp of the committer
  -c, --time-scale SCALE           Change simulation time scale (default: 1.0)
  -e, --elasticity FLOAT           Elasticity of nodes (default: 0.0)

  --key                            Show file extension key

  --user-image-dir DIRECTORY       Dir containing images to use as avatars
  --default-user-image IMAGE       Default user image file
  --fixed-user-size                Use a fixed size throughout
  --colour-images                  Colourize user images

  -i, --file-idle-time SECONDS     Time files remain idle (default: 0)
  --file-idle-time-at-end SECONDS  Time files remain idle at end (default: 0)

  --max-files NUMBER      Max number of files or 0 for no limit
  --max-file-lag SECONDS  Max time files of a commit can take to appear

  --log-command VCS       Show the VCS log command (git,svn,hg,bzr,cvs2cl)
  --log-format  VCS       Specify the log format (git,svn,hg,bzr,cvs2cl,custom)

  --load-config CONF_FILE  Load a config file
  --save-config CONF_FILE  Save a config file with the current options

  -o, --output-ppm-stream FILE    Output PPM stream to a file ('-' for STDOUT)
  -r, --output-framerate  FPS     Framerate of output (25,30,60)

```
Exact whitespace (e.g. trailing blank lines, the "Options:" preceded by blank line) is defined by GS:50-105: after `Gource v0.57\n`, `Usage: gource [options] [path]\n`, then `\nOptions:\n`; the first option line is followed by `\n\n` (blank line); each "block" ends with `\n\n` on its last printf. For `-H` the following block is inserted after the last block above (the previous block ends with a blank line, GS:105-107) and starts with `Extended Options:\n\n`:

```
Extended Options:

  --window-position XxY    Initial window position
  --frameless              Frameless window

  --output-custom-log FILE  Output a custom format log file ('-' for STDOUT).

  -b, --background-colour  FFFFFF    Background colour in hex
      --background-image   IMAGE     Set a background image

  --bloom-multiplier       Adjust the amount of bloom (default: 1.0)
  --bloom-intensity        Adjust the intensity of the bloom (default: 0.75)

  --camera-mode MODE       Camera mode (overview,track)
  --crop AXIS              Crop view on an axis (vertical,horizontal)
  --padding FLOAT          Camera view padding (default: 1.1)

  --disable-auto-rotate    Disable automatic camera rotation

  --disable-input          Disable keyboard and mouse input

  --date-format FORMAT     Specify display date string (strftime format)

  --font-file FILE         Specify the font
  --font-scale SCALE       Scale the size of all fonts
  --font-size SIZE         Font size used by date and title
  --file-font-size SIZE    Font size for filenames
  --dir-font-size SIZE     Font size for directory names
  --user-font-size SIZE    Font size for user names
  --font-colour FFFFFF     Font colour used by date and title in hex

  --file-extensions          Show filename extensions only
  --file-extension-fallback  Use filename as extension if the extension
                             is missing or empty

  --git-branch             Get the git log of a particular branch

  --hide DISPLAY_ELEMENT   bloom,date,dirnames,files,filenames,mouse,progress,
                           root,tree,users,usernames

  --logo IMAGE             Logo to display in the foreground
  --logo-offset XxY        Offset position of the logo

  --loop-delay-seconds SECONDS Seconds to delay before looping (default: 3)

  --title TITLE            Set a title

  --transparent            Make the background transparent

  --user-filter REGEX      Ignore usernames matching this regex
  --user-show-filter REGEX Show only usernames matching this regex

  --file-filter REGEX      Ignore file paths matching this regex
  --file-show-filter REGEX Show only file paths matching this regex

  --user-friction SECONDS  Change the rate users slow down (default: 0.67)
  --user-scale SCALE       Change scale of users (default: 1.0)
  --max-user-speed UNITS   Speed users can travel per second (default: 500)

  --follow-user USER       Camera will automatically follow this user
  --highlight-dirs         Highlight the names of all directories
  --highlight-user USER    Highlight the names of a particular user
  --highlight-users        Highlight the names of all users

  --highlight-colour       Font colour for highlighted users in hex.
  --selection-colour       Font colour for selected users and files.
  --filename-colour        Font colour for filenames.
  --dir-colour             Font colour for directories.

  --dir-name-depth DEPTH    Draw names of directories down to a specific depth.
  --dir-name-position FLOAT Position along edge of the directory name
                            (between 0.0 and 1.0, default is 0.5).

  --filename-time SECONDS  Duration to keep filenames on screen (default: 4.0)

  --caption-file FILE         Caption file
  --caption-size SIZE         Caption font size
  --caption-colour FFFFFF     Caption colour in hex
  --caption-duration SECONDS  Caption duration (default: 10.0)
  --caption-offset X          Caption horizontal offset

  --hash-seed SEED         Change the seed of hash function.

  --path PATH

```
Blank-line rule in that block: the *second* printf of each pair (`--user-show-filter`, `--file-show-filter`) ends in `\n\n` (GS:157-160).

Tail, always printed (GS:193-199):
```
PATH may be a supported version control directory, a log file, a gource config
file, or '-' to read STDIN. If omitted, gource will attempt to generate a log
from the current directory.

To see the full command line options use '-H'      <- only for -h (non-extended), followed by blank line
```
Help mismatches vs implementation (keep as is for fidelity): `--user-friction` help says default 0.67 but code default is 1.0 (GS:162 vs 472); `--dir-name-position` help says "between 0.0 and 1.0" but code rejects <0.1; `--max-file-lag` help has no `-1` mention; `--seconds-per-day` "default: 10" matches `days_per_second=0.1`; README says `--start-date` "Start with the first entry after..." etc.

To be safe, the Rust port should generate the golden help text from `./gource -h > ... ; ./gource -H > ...` of the C++ binary and diff in tests (the transcription above is from reading source).

## 8. Complete CLI option index (alphabetical, every key in `arg_types` / `arg_aliases`)

Aliases (CS:46-49, GS:222-236): `f`=fullscreen, `w`=windowed, `o`=output-ppm-stream, `r`=output-framerate, `p`=start-position, `a`=auto-skip-seconds, `s`=seconds-per-day, `t`=stop-at-time, `i`=file-idle-time, `e`=elasticity, `h`=help, `?`=help, `H`=extended-help, `b`=background-colour, `c`=time-scale, `background`=background-colour, `disable-bloom`=hide-bloom, `disable-progress`=hide-progress, `highlight-all-users`=highlight-users. Single-letter aliases work with any number of hyphens (`-f`, `--f`).

Documented in README/-h/-H vs undocumented:
- Undocumented everywhere (code only): `--stop-on-idle` (no-op), `--ffp`, `--log-level`, `--git-log-command` family, `--hide-*` booleans, `--disable-bloom`, `--disable-progress`, `--highlight-all-users`, `--background`, `--windowed`/`-w` (README only: `-w, --windowed` at README:60; not in -h), `--user-idle-time` (conf-only), `-?`.
- In README but not in `-h`/`-H`: `--windowed`, `--stop-at-time` is in -h; `--hash-seed` in -H and README; `--git-branch`, ok. `--loop-delay-seconds` in README and -H.
- `--path` appears only in -H and README.

Boolean options (consume no argument): help, extended-help, stop-on-idle, stop-at-end, dont-stop, loop, realtime, no-time-travel, colour-images, hide-{date,files,users,tree,usernames,filenames,dirnames,progress,bloom,mouse,root}, highlight-users, highlight-dirs, file-extensions, file-extension-fallback, fixed-user-size, author-time, key, ffp, disable-auto-rotate, disable-auto-skip, disable-input, {git,cvs-exp,cvs2cl,svn,hg,bzr}-log-command, windowed, fullscreen, frameless, transparent, multi-sampling, no-vsync, high-dpi. (CS:51-63, GS:253-292)

Because bool values from the CLI are stored as `"true"`, a config can express them as `yes/true/1` etc.; saved configs therefore contain `true`.

Takes one value: everything else in sections 4-5, plus `load-config, save-config, output-custom-log, log-level, log-command, path` (GS:294-364).

## 9. Platform-specific behaviour (cumulative)

| Item | Detail | Source |
|---|---|---|
| Default window height | 640 on macOS (`__APPLE__`), 768 elsewhere (width 1024) | CS:69-73 |
| `--high-dpi` | omitted from `-h` on Windows; main forces high-DPI awareness on Windows, on others only disables it when a viewport is given without `--high-dpi` (MacOS pixel doubling) | GS:59, M:151-163 |
| Windows console | `initConsole`, console resize to height 1040 for help, `showConsole`, "Press Enter" prompts after help/info/error if console is not pre-existing; console hidden when log level OFF | M:31-33,77-82, GS:44-47,201-206, SA:49-153 |
| Exe path | only passed to `SDLAppInit` on non-Windows (resource dir relative to exe if `SDLAPP_RESOURCE_DIR` missing) | M:22-27, SA:178-256 |
| `user-image-dir` | Windows: UTF-16 filename -> UTF-8; macOS: NFC normalisation of the avatar name | GS:834-858 |
| STDIN wait | Windows `PeekNamedPipe` loop vs POSIX `cin.peek` loop | GS:1641-1651 |
| `cmd/` | `cmd/gource` (sh): `"$(dirname $0)/../gource.exe" "$@"`; `cmd/gource.cmd` (Windows batch): `"%~dp0\..\gource.exe" %*` - wrapper launchers only | cmd/ |
| SDL version | `SDL_SetWindowPosition`/hint only for SDL >= 2 | M:169-175 |
| `-WxH` magic | token containing `x` before option lookup (all platforms) | CS:208 |

## 10. tests/test.conf

Plain config used by the test harness: eight `[gource]` sections (multi-repo), keys used: `title`, `path`, `file-idle-time`, `stop-at-time`, `user-filter`, `file-filter`, `file-show-filter`, `caption-file`, `user-image-dir`. Titles: "File removal", "Convert 'file' to a directory", "User filter (Andrew)", "Filename filter (.cpp)", "Filename show filter (.cpp)", "Directory delete (SVN)", "Directory delete (Custom)", "UTF-8 handling". Note `title=` value contains `'` and `(`: quoting is literal. Sections without `path=` (e.g. user-filter case) get default path `.`. `file-filter=\.cpp` shows regexes are written unescaped.

## 11. Gaps / notes for the port

- `--save-config` uses the raw ConfFile (CLI strings preserved), not typed settings; implement conf as string maps.
- Keep first-entry-wins vs `setEntry` replace-all semantics and the double `parseArgs` pass (needed so `--load-config` can appear after other options).
- `parseDateTime` regex/semantics (CS:284-363) reproduced by `src/test/datetime_tests.cpp`; accepted forms listed in comments CS:288-297. Local-time vs UTC: no tz => `mktime` local; `Z`/offset => `mktime_utc` then minus offset. `mktime_utc` in `src/core/timezone.cpp:41`.
- Regex engine is PCRE2 (`src/core/regex.cpp`); filter semantics (search, unanchored) must be PCRE2-compatible (look-around used by git-branch regex).
- Unverified (not read): `data/gource.1` man page text; README lines 330-end beyond ppm option block; consumer-side semantics of settings fields (other subtasks).

## Tick 2 additions

Cites: M=src/main.cpp, GS=src/gource_settings.cpp, CS=src/core/settings.cpp, CF=src/core/conffile.cpp, SH=src/gource_shell.cpp.

### T2.1 Multi-section config sequencing

**Storage (CF).** `ConfFile` = `map<name, list<ConfSection*>>`; `addSection` appends (file order preserved per name). `ConfSection` = `map<key, list<ConfEntry*>>`. `getSection(name)` = FIRST of the list; `getSections(name)` = whole list or NULL. `ConfFile::load` (CF:507-570): `clear()`, each `[name]` header closes the previous section and starts a new one (a repeated `[gource]` header makes another list element, no merge); `key=value` before any header goes to an unnamed `""` section (never imported); `#` only at column 0 is a comment; blank lines skipped; any other line throws `ConfFileException` "could not parse line"; values right-trimmed. `ConfSection::addEntry` appends (multi-value); `setEntry` deletes ALL entries of that key then inserts one (CF:293-310). `getEntry` = first entry of the key.

**Load order in main (M:40-130).**

| Step | Behaviour |
|---|---|
| 1 | `parseArgs(argc,argv,conf,&files)` into an EMPTY conf: bare tokens -> `files`; `command-line` keys (help, extended-help, log-command, *-log-command, load-config, save-config, output-custom-log, log-level) are handled immediately and never stored (CS:255, GS:239-251) |
| 2 | If no `--load-config` and `files` non-empty: first file ending `.conf`/`.cfg`/`.ini` (name longer than ext) that loads without `ConfFileException` becomes `load_config` and is erased from `files` (M:44-76) |
| 3 | If `load_config`: `conf.clear(); conf.load(file); parseArgs(argc,argv,conf)` again (no files vec) - CLI re-applied OVER the file (M:~86-92) |
| 4 | If `files` non-empty: LAST file is `path`; `setEntry("path",path)` on EVERY `[gource]` section (M:98-103); no `[gource]` section -> `conf.setEntry("gource","path",..)` creates one |
| 5 | `importDisplaySettings(conf)` (first `[display]` only; defaults reset; returns early if absent, CS:365-371), then `importGourceSettings(conf)` with section = `getSection("gource")`, auto-adding an empty `[gource]` if none (GS:615-619). Only the FIRST `[gource]` is validated here |
| 6 | `--save-config`: `conf.save()` then `exit(0)`; `--output-custom-log` with non-empty path: write log then `exit(0)` |

**Routing of keys to sections.** `GourceSettings::default_section_name = "gource"` (GS:219). Keys routed to `[display]`: viewport, windowed, fullscreen, frameless, screen, window-position, multi-sampling, output-ppm-stream, output-framerate, transparent, no-vsync, high-dpi (CS:31-43). A `-WxH` token writes `display.viewport` via `setEntry` (CS:208-229). Everything else -> `[gource]`. Display settings are global: imported once, window created once.

**CLI override semantics per section (CS:262-280).** A CLI option is applied to EVERY section of its target name: `multi-value` types (e.g. follow-user, highlight-user, filters) use `addEntry` (appended to each `[gource]`, in addition to the file's own entries); all other types use `setEntry` (replaces the key in each `[gource]`). Bools are stored as the string `"true"`. So CLI always beats the file for scalar keys in every section; for multi-value keys CLI adds to each section's list. If the target section does not exist it is created. Pass 1 (into the empty conf) is discarded by `conf.clear()` when a config is loaded; otherwise it is the conf used.

**Unknown keys.** `ConfFile::unknownOptionException` exists but is never called, so unknown or misspelled keys in a config file are silently ignored. Unknown CLI options throw `unknown option X` (CS:238). Value errors use `entryException` (file + line number).

**Runtime iteration (SH).** Ctor: iterator = `getSections("gource")->begin()`, `gGourceSettings.repo_count = countSection("gource")` (SH:34-36). `repo_count` other uses: slider shown only if ==1 (gource.cpp:168); Enter advances only if >1 (SH:177). `update()` calls `getNext()` when there is no Gource or `isFinished()`. `getNext()` (SH:205-253):

1. delete current Gource, `transition_interval=1.0` (1 s cross-fade of the captured last frame);
2. return NULL (app finishes) if `shutdown` or iterator at end;
3. `importGourceSettings(*conf, *it)` runs `setGourceDefaults()` FIRST (GS:613): every gource-section setting (filters, hide flags, follow/highlight users, path, fonts, colours, hash seed, git-branch, etc.) is reset to defaults and rebuilt ONLY from that section's entries. Nothing per-repo leaks to the next section. Persisting across sections: `repo_count`, `log_level`, `shutdown`, `load_config`/`save_config`/`output_custom_filename`, all display settings, and the CLI overrides (because they were written into every section);
4. exporter present and not (`dont_stop` || `loop` || path=="-") -> `stop_at_end=true`;
5. `repo_count>1` and `stop_at_time<=0` and `stop_position<=0` -> `stop_at_time=60.0`;
6. `++it`; at end: if `repo_count>1 && exporter==0` wrap to `begin()` (cycles forever interactively); with an exporter, or a single repo, the run ends after the last section;
7. `new Gource(exporter)`.
ESC sets `shutdown`. Enter (without Alt) sets `next=true` (SH:177) -> current Gource deleted and next started.

**Gotchas.** (a) Sections after the first are validated lazily in `getNext()`; a `ConfFileException` there is not caught by main's run block, which catches only `ResourceException` and `SDLAppException` (M:210-226; `ConfFileException : std::exception`, conffile.h:30) -> uncaught -> `std::terminate`, after earlier repos already played. Port must pick: abort the same way, or validate all sections up front (behaviour change). (b) `path` existence is checked per section at import (GS:1629+).

**tests/test.conf** (no reference found in Makefile.am/configure.ac/.github; presumably manual or external harness). Eight `[gource]` sections, each with `title` and `stop-at-time` (10,10,5,5,5,5,5,5), plus: `file-idle-time=0.0`, `path=` (logs/*.log) on 1,2,6,7, `user-filter=Andrew`, `file-filter=\.cpp`, `file-show-filter=\.cpp`, `caption-file`+`user-image-dir` (section 8, with `path=logs/utf8.log`). Sections 3-5 have no `path` -> default `.` after the reset. It exercises: per-section reset (filters must not leak), relative paths resolved from CWD, multi-repo wrap (repo_count=8). Suggested port test: run each section, assert filters absent in the following one.

### T2.2 Help (-h/-H) vs man page vs README

Method: set diff of `--long` tokens (help = GS:42-209; man = data/gource.1 with `\-` unescaped; README), plus `arg_types` keys.

| Option | -h/-H | man | README |
|---|---|---|---|
| `--disable-auto-skip` | yes | NO | NO |
| `--dont-stop` | yes | NO | NO |
| `-f` / `--fullscreen` | `-f, --fullscreen` | `-f` only | `-f, --fullscreen` |
| `-w, --windowed` | NO | NO | yes (alias `w` exists, CS:47) |
| `--path PATH` | yes | yes | yes |
| `--ffp`, `--stop-on-idle`, `--hide-<elem>` (bloom,date,dirnames,files,filenames,mouse,progress,root,tree,users,usernames), `--disable-bloom`, `--disable-progress`, `--log-level`, `--extended-help`, `--git/svn/hg/bzr/cvs2cl/cvs-exp-command` | NO | NO | NO |

Every other long option in help appears in both man and README. Man-only tokens `--chrono --stdout --xml` are the cvs2cl example, not options. `-H`: documented in `-h` line ("'-H' for extended help") of man/README.

Wording and default mismatches:

| Item | help | man / README | Code truth |
|---|---|---|---|
| `--dir-name-position` range | "between 0.0 and 1.0" | "0.1 and 1.0" | rejects outside 0.1-1.0 (GS:1625) - help wrong |
| `--user-friction` | "(default: 0.67)" | no default | stored as `1/value` (GS:1421); internal default 1.0 (GS:472) |
| `--filename-time` | "default: 4.0" | "(>= 2.0)" | default 4.0 (GS:487); error if <2.0 (GS:947) |
| `--default-user-image` | "Default user image file" | man ".jpg"; README ".jpg or .png" | either accepted |
| `--max-file-lag` | no -1 note | "Use -1 for no limit." | default 5.0 |
| `--auto-skip-seconds` | "(default: 3)" | no default | 3.0 (GS:409) |
| `--seconds-per-day` | "(default: 10)" | no default | days_per_second=0.1 (GS:410) |
| `--loop-delay-seconds` | "(default: 3)" | no default; README omits SECONDS arg | 3.0 (GS:416), must be >0 |
| `--time-scale` | "(default: 1.0)" | no range | rejects <=0 or >4.0 (GS:1266), undocumented |
| `--stop-date` | - | man "Stop at the last entry prior to"; README "Stop after..." | wording only |
| `--output-framerate` | "(25,30,60)" | same | enforced, CS:464-466 |
| `--high-dpi` | omitted from help on _WIN32 (GS:57) | documented always | |
| bloom/padding/caption-duration defaults | in help (0.75/1.0/1.1/10.0) | absent | match code |
| README typos | | custom log `type`: "initial for the update type" (man: "Single character"); caption: "ISO 8601 or A unix timestamp" | text only |

### T2.3 README lines 300-end (previously uncaptured)

- No ffmpeg/PPM pipeline recipe in README; only a link to the wiki Videos page. Port needs only `--output-ppm-stream`/`-r` semantics (auto-hides progress, implies stop-at-end).
- Custom log: pipe-delimited `timestamp|username|type|file|colour`, type A/M/D, timestamp ISO 8601 or unix, colour optional hex. Caption log: `timestamp|caption`.
- cvs2cl recipe: `cvs2cl --chrono --stdout --xml -g-q`. `--log-command svn > f.log; gource f.log` round trip implies format auto-detection of saved logs.
- Mouse: middle button toggles camera mode, left-drag pans, right-drag rotates; SPACE pause/resume (mouse inspection while paused); TAB cycles users.
- Keys: V camera mode, C logo, K ext key, M mouse, N next log entry, S randomize colours, D/F/U name display modes, G users, T tree edges, R root edges, `<`/`>` time scale, `+`/`-` sim speed, keypad +/- zoom, TAB, F12 screenshot, Alt+Enter fullscreen, ESC quit. In code but not README: F11 toggles window frame (SH:160), Enter = next repo in multi-section.
- No environment variables are documented in README or man.

### T2.4 Environment variables and SDL hints (grep of src/)

| Name | Where | Effect |
|---|---|---|
| `TZ` | src/core/timezone.cpp:13-35; src/formats/svn.cpp:121-141 | read and saved, set to `TZ=UTC` via `putenv` during date parsing, then restored/unset (`TZ=` if none). Port: do UTC arithmetic directly, no env mutation |
| `TERM` | src/core/sdlapp.cpp:70 (Windows `initConsole` only) | presence => treat as existing console |
| `SDL_HINT_VIDEO_MINIMIZE_ON_FOCUS_LOSS="0"` | src/main.cpp:174 (SDL2) | hint set before window creation |
| others | - | no other getenv/setenv; no GOURCE_* vars; tests set only `TZ` (src/test/datetime_tests.cpp:27,31). Child VCS commands inherit the environment |

Gaps after tick 2: `ConfFile::save` output format (key/section order) not traced; behaviour of the unnamed `""` section beyond "ignored" not verified; test.conf harness not located.
