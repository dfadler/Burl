# Option coverage: Gource 0.57 settings vs Burl

Audit of every option and config key in Gource 0.57's settings against Burl at `33ead47` (master after #68). Report only: no code was changed. Deviation rows (`D*`, `G*`) are in [DEVIATIONS.md](DEVIATIONS.md); this file does not edit it.

## Headline

- **Parse: complete.** The C++ option table (`arg_types[...]` in `src/gource_settings.cpp` plus the display options in `reference/gource-cpp-dump/src/core/settings.cpp`) has **114** entries. Burl's `ARG_TYPES` (`crates/burl-conf/src/args.rs`) has **120**: the same 114 with identical types, plus the six hidden test controls of ticket #22 (`--debug-dump-state/-every/-max-frames`, `--prng`, `--prng-seed`, `--rand-script`). The only type differences are `--ffp` and `--stop-on-idle`: `bool` in C++, `Ignored` in Burl (D4, D5d, ticket #8). Aliases and command-line-section routing were compared the same way.
- **Store: complete.** Every gource option has a field in `GourceSettings` (`settings.rs:142`) or `DisplaySettings` (`settings.rs:28`) except the two `Ignored` ones.
- **Changes behaviour:** 101 of the 120 are wired (some only in `-o` export, see the Windowed column). Three are wired in part (`--transparent`, `--hide-progress`, `--camera-mode`), three are *refused* with an error (`--start-position N`, `--loop`, `--follow-user`; `--camera-mode track` as well), two are `Ignored` by design, and **eleven are silently stored and do nothing**: `--hide-root`, `--fixed-user-size`, `--selection-colour`, `--font-file`, `--loop-delay-seconds`, `--multi-sampling`, `--screen`, `--log-level`, `--colour-images`, `--user-image-dir`, `--default-user-image`. One more is silent *and wrong*: `--start-position random` plays from the start. These are the real gaps (end of this file).
- **Windowed vs `-o`:** the two paths consume different sets. The windowed frame (`crates/burl/src/interactive.rs:204-225`, `Live::frame`) draws only the scene (`build_frame` then `render`); names, date, title, captions, key, logo and background image are drawn only by `export.rs`. The windowed path also plays only the first `[gource]` section (`interactive.rs:344`, `repos.swap_remove(0)`); `-o` plays all of them.

## Status vocabulary

| Status | Meaning |
|---|---|
| **Wired** | Parsed, stored and read by simulation or rendering code (cited). |
| **Wired-partial** | Read, but part of the C++ behaviour is missing (see Note). |
| **Refused** | Parsed and stored; `Sim::new` returns an error (`sim.rs:326-342`, exit 1 with "... is not ported to the simulation yet"). Loud. |
| **Stored-unused** | Parsed, validated, stored (and saved by `--save-config`), then never read. Silent. |
| **Ignored (D4/D5d)** | Accepted by design as `ArgType::Ignored`; the D5 `Ignored` kind explains it. |
| **Deviation Gn** | A known Gap row in DEVIATIONS.md. |

Columns: **Export `-o`** and **Windowed** say whether that path consumes the option (`Y*` partial, `-` not applicable). **Tests** (scripted, see Method): `U` the option name appears in a `burl-conf` test, `E` in a CLI test under `crates/burl/tests`, `S` in a sim-gate case (`tests/golden/sim/*.args|conf`, `tests/test.conf`; compared against the fork's state dump, so only for sim, log and config options), `R` the Rust field name appears in a render/sim/format unit test. `*` marks an indirect hit (see Note). A match is a name match, not proof the assertion would fail without the option: the sabotage check was **not** run for each row.

Two generic tests cover every option's parse path and are not repeated per row: `every_option_has_an_import_arm` (`crates/burl-conf/tests/settings.rs:281`, a *text* check that `"name"` occurs in `settings.rs`, so it does not prove the arm reads it) and the saved/loaded ConfFile round trips (`save_config_end_to_end`, `first_end_to_end_save`). All `--hide-X` flags and `--hide a,b` share one import path (`settings.rs:491-547`), covered by `hide_list_quirks`; per-flag consumer tests are the `R` column.

## Per-option table

### Simulation and timing

| Option | Type | Stored in | Export `-o` | Windowed | Status | Consumer | Tests | Note / C++ ref |
|---|---|---|---|---|---|---|---|---|
| `--auto-skip-seconds` | Float | `auto_skip_seconds` | Y | Y | Wired | burl-sim/src/sim.rs:555 | USR |  |
| `--camera-mode` | Str | `camera_mode` | overview Y / track N | same | Wired-partial | sim.rs:340 refuses `track`; main.rs:236 | U | gource.cpp:528-541 |
| `--caption-duration` | Float | `caption_duration` | Y | Y | Wired | burl-sim/src/sim.rs:2146 | SR |  |
| `--caption-file` | Str | `caption_file` | Y | Y | Wired | burl-sim/src/sim.rs:2092 | UES |  |
| `--crop` | Str | `crop_vertical`, `crop_horizontal` | Y | Y | Wired | burl-sim/src/sim.rs:194-196 | U |  |
| `--disable-auto-rotate` | Bool | `disable_auto_rotate` | Y | Y | Wired | burl-sim/src/sim.rs:2008 | UER |  |
| `--disable-auto-skip` | Bool | `auto_skip_seconds` | Y | Y | Wired | settings.rs:556 sets -1; sim.rs:555 | UER |  |
| `--elasticity` | Float | `elasticity` | Y | Y | Wired | burl-sim/src/sim.rs:1932 | US |  |
| `--file-idle-time` | Str | `file_idle_time` | Y | Y | Wired | burl-sim/src/sim.rs:1290 | UES |  |
| `--file-idle-time-at-end` | Str | `file_idle_time_at_end` | Y | Y | Wired | burl-sim/src/sim.rs:643 | UES |  |
| `--filename-time` | Float | `filename_time` | Y | Y | Wired | burl-sim/src/sim.rs:701 | US |  |
| `--follow-user` | MultiValue | `follow_users` | N | N | Refused | sim.rs:337 | R | gource.cpp:1390 |
| `--hash-seed` | Int | `hash_seed` | Y | Y | Wired | burl-sim/src/sim.rs:827, commit.rs:51 | UES |  |
| `--highlight-user` | MultiValue | `highlight_users` | Y | Y | Wired | burl-sim/src/sim.rs:781 | U |  |
| `--highlight-users` | Bool | `highlight_all_users` | Y | Y | Wired | burl-sim/src/sim.rs:777, labels.rs:166 | UR |  |
| `--loop` | Bool | `loop_` | N | N | Refused | sim.rs:334 (`Sim::new` error) | UR | gource.cpp:1704-1712 |
| `--loop-delay-seconds` | Float | `loop_delay_seconds` | N | N | Stored-unused | none (only meaningful with --loop) | U | gource.cpp:1708 |
| `--max-file-lag` | Str | `max_file_lag` | Y | Y | Wired | burl-sim/src/sim.rs:1108 | US |  |
| `--max-files` | Int | `max_files` | Y | Y | Wired | burl-sim/src/sim.rs:684 | US |  |
| `--max-user-speed` | Float | `max_user_speed` | Y | Y | Wired | burl-sim/src/sim.rs:835 | S |  |
| `--no-time-travel` | Bool | `no_time_travel` | Y | Y | Wired | burl-sim/src/sim.rs:570 | ES |  |
| `--output-framerate` | Int | `output_framerate` | Y | Y | Wired | burl-sim/src/sim.rs:345-347 (tick rate only; no ffmpeg) | UE |  |
| `--padding` | Float | `padding` | Y | Y | Wired | burl-sim/src/sim.rs:356 (camera) | U |  |
| `--seconds-per-day` | Float | `days_per_second` | Y | Y | Wired | burl-sim/src/sim.rs:534, command.rs:158 | UES |  |
| `--start-position` | Str | `start_position`, `start_position_random` | Y/N | Y/N | Refused (N>0); **silent for `random`** | sim.rs:331 refuses `> 0`; `random` leaves 0 and plays from the start | UR | gource.cpp:1604; gource_settings.cpp:1318 |
| `--stop-at-end` | Bool | `stop_at_end` | Y | Y | Wired | burl-sim/src/sim.rs:635 | U* |  |
| `--stop-at-time` | Float | `stop_at_time` | Y | Y | Wired | burl-sim/src/sim.rs:489 | UES |  |
| `--stop-date` | Str | `stop_date`, `stop_timestamp` | Y | Y | Wired | burl-sim/src/sim.rs:617, logmill.rs:130 | UR |  |
| `--stop-position` | Str | `stop_position` | Y | Y | Wired | burl-sim/src/sim.rs:636 | US |  |
| `--time-scale` | Float | `time_scale` | Y | Y | Wired | burl-sim/src/sim.rs:482 | UE |  |
| `--user-friction` | Float | `user_friction` | Y | Y | Wired | burl-sim/src/sim.rs:1161 | US |  |
| `--user-scale` | Str | `user_scale` | Y | Y | Wired | burl-sim/src/sim.rs:836, scene.rs (ss.user_scale) | SR |  |

### Log input

| Option | Type | Stored in | Export `-o` | Windowed | Status | Consumer | Tests | Note / C++ ref |
|---|---|---|---|---|---|---|---|---|
| `--author-time` | Bool | `author_time` | Y | Y | Wired | burl-formats/src/formats/git.rs:60 | R |  |
| `--file-filter` | MultiValue | `file_filters` | Y | Y | Wired | burl-formats/src/commit.rs:76 | USR |  |
| `--file-show-filter` | MultiValue | `file_show_filters` | Y | Y | Wired | burl-formats/src/commit.rs:80 | US |  |
| `--git-branch` | Str | `git_branch` | Y | Y | Wired | burl-formats/src/formats/git.rs:73 | UR |  |
| `--log-format` | Str | `log_format` | Y | Y | Wired | burl-formats/src/logmill.rs:60 | UESR |  |
| `--path` | Str | `path` | Y | Y | Wired | burl-formats/src/formats/*.rs, sim.rs, main.rs | UES |  |
| `--start-date` | Str | `start_date`, `start_timestamp` | Y | Y | Wired | burl-formats/src/logmill.rs:123, formats/*.rs | UR |  |
| `--user-filter` | MultiValue | `user_filters` | Y | Y | Wired | burl-formats/src/commit.rs:97 | UESR |  |
| `--user-show-filter` | MultiValue | `user_show_filters` | Y | Y | Wired | burl-formats/src/commit.rs:104 | R |  |

### Rendering and text

| Option | Type | Stored in | Export `-o` | Windowed | Status | Consumer | Tests | Note / C++ ref |
|---|---|---|---|---|---|---|---|---|
| `--background-colour` | Str | `background_colour` | Y | Y | Wired | burl/src/export.rs:185, interactive.rs:181 | U |  |
| `--background-image` | Str | `background_image` | Y* | N | Wired | burl/src/export.rs:137 | R* | PNG only (Gource also reads JPEG/GIF); export only. #51. |
| `--bloom-intensity` | Float | `bloom_intensity` | Y | Y | Wired | burl-render/src/scene.rs:196 | U |  |
| `--bloom-multiplier` | Float | `bloom_multiplier` | Y | Y | Wired | burl-render/src/scene.rs:194 | **none** |  |
| `--caption-colour` | Str | `caption_colour` | Y | N | Wired | burl-render/src/overlays.rs:144 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--caption-offset` | Int | `caption_offset` | Y | N | Wired | burl-render/src/overlays.rs:114 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--caption-size` | Int | `caption_size` | Y | N | Wired | burl-render/src/overlays.rs:45, export.rs:168 | **none** | Export only: the windowed frame draws no text (see Gap 1). |
| `--colour-images` | Bool | `colour_user_images` | N | N | Stored-unused (G9) | none | **none** | user.cpp:197,210 |
| `--date-format` | Str | `date_format` | Y | N | Wired | burl/src/export.rs:203 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--default-user-image` | Str | `default_user_image` | N | N | Stored-unused (G9) | none | **none** | user.cpp:209 |
| `--dir-colour` | Str | `dir_colour` | Y | N | Wired | burl-render/src/labels.rs:27 via export.rs:155 | U | Export only: the windowed frame draws no text (see Gap 1). |
| `--dir-font-size` | Int | `dirname_font_size`, `scaled_dirname_font_size` | Y | N | Wired | burl/src/export.rs:161 | **none** | Export only: the windowed frame draws no text (see Gap 1). |
| `--dir-name-depth` | Int | `dir_name_depth` | Y | N | Wired | burl-render/src/labels.rs:116 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--dir-name-position` | Float | `dir_name_position` | Y | N | Wired | burl-render/src/labels.rs:132 | U | Export only: the windowed frame draws no text (see Gap 1). |
| `--file-extension-fallback` | Bool | `file_extension_fallback` | Y | N | Wired | burl-render/src/labels.rs:151, overlays.rs:73 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--file-extensions` | Bool | `file_extensions` | Y | N | Wired | burl-render/src/labels.rs:150 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--file-font-size` | Int | `filename_font_size`, `scaled_filename_font_size` | Y | N | Wired | burl/src/export.rs:162 | **none** | Export only: the windowed frame draws no text (see Gap 1). |
| `--filename-colour` | Str | `filename_colour` | Y | N | Wired | burl-render/src/labels.rs:28 via export.rs:156 | **none** | Export only: the windowed frame draws no text (see Gap 1). |
| `--fixed-user-size` | Bool | `fixed_user_size` | N | N | Stored-unused | none (scene.rs:7 lists it as not ported) | **none** | gource.cpp:2273 |
| `--font-colour` | Str | `font_colour` | Y | N | Wired | burl-render/src/overlays.rs:21 via export.rs:177 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--font-file` | Str | `font_file` | N | N | Stored-unused | none: export.rs:90 always uses bundled FreeSans | U | gource.cpp:50-67, file.cpp:55-62. Validation at import still runs. |
| `--font-scale` | Float | `font_scale`, `default_font_scale` | Y | N | Wired | burl/src/export.rs:109-110 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--font-size` | Int | `font_size`, `scaled_font_size` | Y | N | Wired | burl/src/export.rs:167; key.rs:55 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--hide` | Str | `hide_*` | Y | part | Wired | settings.rs:491-547 (comma list OR-ed with every --hide-X flag); consumers as the individual hide-* rows | UER | Windowed honours tree/users/files/bloom/mouse only. Tested by `hide_list_quirks`. |
| `--hide-bloom` | Bool | `hide_bloom` | Y | Y | Wired | burl-render/src/scene.rs:191 | R |  |
| `--hide-date` | Bool | `hide_date` | Y | N | Wired | burl-render/src/overlays.rs:124 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--hide-dirnames` | Bool | `hide_dirnames` | Y | N | Wired | burl-render/src/labels.rs:121 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--hide-filenames` | Bool | `hide_filenames` | Y | N | Wired | burl-render/src/labels.rs:138 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--hide-files` | Bool | `hide_files` | Y | Y | Wired | burl-render/src/scene.rs:146, labels.rs:138 | R |  |
| `--hide-progress` | Bool | `hide_progress` | Y | N | Wired-partial | burl-render/src/overlays.rs:89 | R | Only moves the caption rows; the position slider it hides is not drawn (overlays.rs:5). C++ shows the slider only when not exporting and one repo (gource.cpp:167-168), so this matters for the windowed path. |
| `--hide-root` | Bool | `hide_root` | N | N | Stored-unused | none | U | dirnode.cpp:1111,1124,1138 (spline to the root dir skipped); `h`-style toggle gource.cpp:784 |
| `--hide-tree` | Bool | `hide_tree` | Y | Y | Wired | burl-render/src/scene.rs:138 | R | Also the `T`/`U` toggles in the windowed path. |
| `--hide-usernames` | Bool | `hide_usernames` | Y | N | Wired | burl-render/src/labels.rs:160 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--hide-users` | Bool | `hide_users` | Y | Y | Wired | burl-render/src/scene.rs:159, labels.rs:160 | R | Also the `T`/`U` toggles in the windowed path. |
| `--highlight-colour` | Str | `highlight_colour` | Y | N | Wired | burl-render/src/labels.rs:29 via export.rs:157 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--highlight-dirs` | Bool | `highlight_dirs` | Y | N | Wired | burl-render/src/labels.rs:118 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--key` | Bool | `show_key` | Y | N | Wired | burl-render/src/overlays.rs:149 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--logo` | Str | `logo` | Y* | N | Wired | burl/src/export.rs:141 | R* | PNG only; export only. #51. |
| `--logo-offset` | Str | `logo_offset` | Y | N | Wired | burl/src/export.rs:197 | UR* |  |
| `--selection-colour` | Str | `selection_colour` | N | N | Stored-unused | none (labels.rs:5: selection not ported) | **none** | file.cpp:58,141; user.cpp:330; needs pointer selection, so windowed-relevant |
| `--title` | Str | `title` | Y | N | Wired | burl-render/src/overlays.rs:96 | UE | Export only: the windowed frame draws no text (see Gap 1). |
| `--user-font-size` | Int | `user_font_size`, `scaled_user_font_size` | Y | N | Wired | burl/src/export.rs:163 | U | Export only: the windowed frame draws no text (see Gap 1). |
| `--user-image-dir` | Str | `user_image_dir`, `user_image_map` | N | N | Stored-unused (G9) | none | UE | user.cpp:183 |

### Windowed-only and app

| Option | Type | Stored in | Export `-o` | Windowed | Status | Consumer | Tests | Note / C++ ref |
|---|---|---|---|---|---|---|---|---|
| `--disable-input` | Bool | `disable_input` | n/a | Y | Wired | burl/src/interactive.rs:271-285 | **none** |  |
| `--hide-mouse` | Bool | `hide_mouse`, `hide_progress` | n/a | Y | Wired | burl/src/interactive.rs:110,123; import also sets hide_progress | R | Windowed only (cursor). |
| `--output-ppm-stream` | Str | `output_ppm_filename` | Y | - | Wired | burl/src/main.rs:384, export.rs:93 | U |  |

### Display / window (`SDLAppSettings`)

| Option | Type | Stored in | Export `-o` | Windowed | Status | Consumer | Tests | Note / C++ ref |
|---|---|---|---|---|---|---|---|---|
| `--frameless` | Bool | `frameless` | - | Y | Wired | burl/src/interactive.rs:45 | U |  |
| `--fullscreen` | Bool | `fullscreen` | - | Y | Wired | burl/src/interactive.rs:47 | U |  |
| `--high-dpi` | Bool | `high_dpi` | - | Y | Wired | burl/src/interactive.rs:36 | UR |  |
| `--multi-sampling` | Bool | `multisample` | - | N | Stored-unused | none | **none** | main.cpp:134 |
| `--no-vsync` | Bool | `vsync` | - | Y | Wired | burl-gl/src/window.rs:61, interactive.rs:151 | U |  |
| `--screen` | Int | `screen` | - | N | Stored-unused | none (`window_attributes` never reads it) | U | main.cpp:167 |
| `--transparent` | Bool | `transparent` | N | Y* | Wired-partial | interactive.rs:46 (window attribute only) | **none** | Export clears with alpha 1 and writes P6 (3 channel); no `bloom_alpha.tga`. gource.cpp:74,2028,2250; main.cpp:139 |
| `--viewport` | Str | `width`, `height`, `viewport_specified` | Y | Y | Wired | burl/src/export.rs:86, interactive.rs:36 | UER |  |
| `--window-position` | Str | `window_x`, `window_y` | - | Y | Wired | burl/src/interactive.rs:50 | U |  |
| `--windowed` | Bool | `fullscreen`, `resizable` | - | Y | Wired | import sets fullscreen=false (settings.rs:106); interactive.rs:47 | U |  |

### Command-line-only

| Option | Type | Stored in | Export `-o` | Windowed | Status | Consumer | Tests | Note / C++ ref |
|---|---|---|---|---|---|---|---|---|
| `--bzr-log-command` | Bool | - | n/a | n/a | Wired | burl/src/main.rs:106 | E |  |
| `--cvs-exp-command` | Bool | - | n/a | n/a | Wired | burl/src/main.rs:91 | E |  |
| `--cvs2cl-command` | Bool | - | n/a | n/a | Wired | burl/src/main.rs:97 | E |  |
| `--debug-dump-every` | Str | - | n/a | n/a | Wired | burl/src/main.rs (DebugOptions), burl-conf settings.rs:1067-1144 | UE | Hidden test-only (#22); not in C++. |
| `--debug-dump-max-frames` | Str | - | n/a | n/a | Wired | burl/src/main.rs (DebugOptions), burl-conf settings.rs:1067-1144 | UE | Hidden test-only (#22); not in C++. |
| `--debug-dump-state` | Str | - | n/a | n/a | Wired | burl/src/main.rs (DebugOptions), burl-conf settings.rs:1067-1144 | UE | Hidden test-only (#22); not in C++. |
| `--extended-help` | Bool | - | n/a | n/a | Wired | burl/src/main.rs:50 | E |  |
| `--git-log-command` | Bool | - | n/a | n/a | Wired | burl/src/main.rs:88 | E |  |
| `--help` | Bool | - | n/a | n/a | Wired | burl/src/main.rs:50 (golden help text) | UE |  |
| `--hg-log-command` | Bool | - | n/a | n/a | Wired | burl/src/main.rs:103 | E |  |
| `--load-config` | Str | - | n/a | n/a | Wired | burl/src/main.rs:78, 337-348 | UE |  |
| `--log-command` | Str | - | n/a | n/a | Wired | burl/src/main.rs:88-112 | E |  |
| `--log-level` | Str | - | n/a | n/a | Stored-unused | main.rs:113 returns Ok; no logger | **none** | main.cpp:75; gource_settings.cpp:588-598. Burl never prints diagnostics at any level. |
| `--output-custom-log` | Str | - | n/a | n/a | Wired | burl/src/main.rs:109, write_custom_log 139 | UE |  |
| `--prng` | Str | - | n/a | n/a | Wired | burl/src/main.rs (DebugOptions), burl-conf settings.rs:1067-1144 | UE | Hidden test-only (#22); not in C++. |
| `--prng-seed` | Str | - | n/a | n/a | Wired | burl/src/main.rs (DebugOptions), burl-conf settings.rs:1067-1144 | UE | Hidden test-only (#22); not in C++. |
| `--rand-script` | Str | - | n/a | n/a | Wired | burl/src/main.rs (DebugOptions), burl-conf settings.rs:1067-1144 | UE | Hidden test-only (#22); not in C++. |
| `--save-config` | Str | - | n/a | n/a | Wired | burl/src/main.rs:82, 364-367 | E |  |
| `--svn-log-command` | Bool | - | n/a | n/a | Wired | burl/src/main.rs:100 | E |  |

### Config-only

| Option | Type | Stored in | Export `-o` | Windowed | Status | Consumer | Tests | Note / C++ ref |
|---|---|---|---|---|---|---|---|---|
| `--dont-stop` | Bool | `dont_stop` | Y | Y | Wired | burl-conf settings.rs:398 `apply_shell_fixups` (gource_shell.cpp:224); its only C++ consumer | U* | **Not a gap.** |
| `--ffp` | Ignored | - | - | - | Ignored (D4) | none by design | UE | `ArgType::Ignored` (args.rs:134) |
| `--realtime` | Bool | `days_per_second` | Y | Y | Wired | settings.rs:924 (import sets 1/86400, then as seconds-per-day) | U |  |
| `--stop-on-idle` | Ignored | - | - | - | Ignored (D5d) | none by design | UE | `ArgType::Ignored` (args.rs:109) |

## Other keys and inputs

| Item | Parse | Store | Behaviour | Tests / note |
|---|---|---|---|---|
| **ConfFile** (`conffile.rs`) | Sections, `key=value`, comments, multi-value repeats | `ConfFile` is the parser-to-settings seam (#8) | Two argument passes, positional `.conf/.cfg/.ini` detection, `path` override, CLI over file (`main.rs:329-352`) | `config_loading_rules`, `grammar`, `grammar_errors`; D1 (all sections validated up front) `deviation_d1_*` |
| `--load-config` / `--save-config` | Command-line section, never stored | `CommandLine` (`main.rs:60`) | `save-config` writes the ConfFile and exits 0 (`main.rs:364-367`); order is by name then key | `save_config_end_to_end`, `first_end_to_end_save`, `save_order_is_by_name_then_key_*`. Hidden `--debug-*`/`--prng*` are never saved (`hidden_options_are_accepted_never_saved_and_validated`) |
| `--output-custom-log` | yes | `CommandLine` | `write_custom_log` (`main.rs:139`), `-` for stdout, exit 0 | Oracle goldens `tests/golden/{edge,real}/*.out`, `scripted_cases`, `output_to_file_and_silent_open_failure` |
| `--output-ppm-stream` / `-o` | yes | `DisplaySettings.output_ppm_filename` | `export::run` writes one P6 per tick; `-` is stdout | **No CLI-level export test** (`crates/burl/tests` has none for `-o`); render-layer unit tests and the open visual gate (#54) only |
| `-r` / `--output-framerate` | alias in `ARG_ALIASES` | `output_framerate` (default 60) | Only the sim tick rate (`sim.rs:345-347`, `setFrameExporter` port); nothing is encoded | `U`/`E` for parsing; **no gate case uses `-r`**, so the tick-rate math for a value other than 60 is not compared with the fork |
| `--hide a,b,...` | yes (unknown field is an error) | `hide_*` | See the `--hide` row above | `hide_list_quirks` |
| `--key` | yes | `show_key` | `overlays.rs:149`, file-extension key; `FileKey::clear`/`colourize` not ported (`key.rs:4`) | `burl-render/tests/overlays.rs`, `crates/burl-render/src/key.rs` tests; export only |
| `--caption-file/-duration` | yes | `caption_*` | Sim reads the file and schedules captions (`sim.rs:2092,2146`); `caption-size/-colour/-offset` are render-only (`overlays.rs`) | Sim gate `small-repo-captions`; size/colour/offset only by `burl-render/tests/overlays.rs` |
| **stdin** (`path` `-`) | yes | `path == "-"` | Blocking first-byte wait then `StreamLog`; `--log-format` required (D8) | `stdin_variant_matches_file_golden`, `stdin_drops_unterminated_last_line`, `stdin_requires_log_format`, `deviation_d8_stdin_blocking_read`; `apply_shell_fixups` skips `stop_at_end` for `-` |
| **Log formats** (`--log-format`) | `git`, `svn`, `hg`, `bzr`, `cvs-exp`, `cvs2cl`, `custom`, `apache`, plus `git` raw-log auto path | `log_format` | `logmill.rs:60-74` matches the `logmill.cpp:216-262` set; auto-detect and `cvs-exp` (D5c) | Oracle goldens in `tests/golden/{edge,real}`, `auto_detect_matches_explicit_format`, `deviation_d5_cvs_exp_explicit` |
| `--*-log-command`, `--log-command` | yes | none (print and exit) | `main.rs:88-112`; `--log-command cvs` is the C++ error | `log_command_printers` (svn and bzr asserted by text; git/hg/cvs-exp/cvs2cl only by the shared code path) |
| `--help`, `-H` | yes | none | Golden help text, D5a/D5b corrected via `expected-diffs/help-long.diff` | `help_text_matches_golden` |
| Aliases (`ARG_ALIASES`) | 19 entries incl. `-r`, `-o`, `-w`, `-f` | | `-WxH` magic viewport | `parse_args_rules` |
| Environment | `TZ` for date formats | | `format_local` | `local_timezone_from_env` |

### Options with no test naming them

Scripted (`gen.py` in the Method section) found 13 options that no test in any column mentions by name: `--bloom-multiplier`, `--caption-size`, `--colour-images`, `--default-user-image`, `--dir-font-size`, `--disable-input`, `--file-font-size`, `--filename-colour`, `--fixed-user-size`, `--log-level`, `--multi-sampling`, `--selection-colour`, `--transparent`. Seven of them are Stored-unused (so there is nothing to test yet); the other six are wired and untested: `--bloom-multiplier`, `--caption-size`, `--dir-font-size`, `--file-font-size`, `--filename-colour`, `--disable-input`.

## Real gaps, prioritised

Sizes: **small** under a day and local to one or two files, **medium** a few days or touches both `-o` and windowed paths, **large** new subsystem or needs user decisions. "Ticket" names an existing open issue that already covers it (#51 body still lists items that have since landed: background image, logo, overlays and most `hide_*` are now in `export.rs`; its remaining scope is the windowed path, `fixed_user_size`, the slider and avatars).

| # | Gap | Options affected | C++ reference | Size | Ticket / owner |
|---|---|---|---|---|---|
| 1 | **Windowed frame draws no text or overlays and plays one section.** `interactive.rs::frame` calls `build_frame` only; `export.rs` additionally does `draw_names`, `Overlays::draw`, background, logo. `swap_remove(0)` drops later `[gource]` sections. A user running plain `burl` sees no labels, date, title, key, captions, logo or background image. | `hide-dirnames/-filenames/-usernames/-date/-progress`, `title`, `date-format`, `key`, `caption-size/-colour/-offset`, `font-*`, `*-font-size`, `dir-name-*`, `file-extension*`, `highlight-dirs`, `dir/filename/highlight/font-colour`, `logo`, `logo-offset`, `background-image`; multi-section configs | `gource.cpp:2495-2527` (names), `2655-2675` (overlays), `2025` (`drawBackground`), `2583-2592` (logo); `gource_shell.cpp:getNext` for sections | **medium** (share one frame-builder between `export.rs` and `interactive.rs`; section hand-off is its own medium) | Partly #51 (not the windowed half); needs its own ticket |
| 2 | **`--start-position random` silently plays from the start.** `settings.rs:899` sets `start_position_random` and leaves `start_position = 0`; nothing reads the flag, so `Sim::new`'s `> 0.0` refusal (`sim.rs:331`) is skipped. | `start-position random` | `gource_settings.cpp:1318-1321` (`srand(time(0))`, `rand() % 1000 / 1000.0`), `gource.cpp:1604-1605` (`seekTo`) | **small** to refuse loudly like the others; the real port is in 3 (note the C++ reseeds `srand(time(0))` at import: it interacts with the D3 PRNG design) | Fold into the `sim-start-loop` session; no ticket |
| 3 | **Refused (loud) in the sim**: start-position seek, `--loop`, `--follow-user`, `--camera-mode track`. | `start-position N`, `loop`, `loop-delay-seconds` (unused until `loop` lands), `follow-user`, `camera-mode track` | `gource.cpp:1604` (`seekTo`), `1704-1712` (loop restart and `loop_delay_seconds`), `1390` (follow users), `528-541,1507-1515` (track camera) | **medium** each | In flight in the `sim-start-loop` and `sim-follow-track` sessions; the refusal test is `unported_options_are_refused_not_ignored` |
| 4 | **`--hide-root` does nothing.** The root spline is always drawn. | `hide-root` | `dirnode.cpp:1111,1124,1138` (`spline` skipped when `parent->parent == 0`); toggle `gource.cpp:784` | **small** (one condition in the `scene.rs` edge building; `hide_root` is already in `GourceSettings`; add it to `SceneSettings`) | no ticket |
| 5 | **`--fixed-user-size` does nothing** (`scene.rs:7` says so). | `fixed-user-size` | `gource.cpp:2273` (`scaled_dims *= (-camera.getPos().z / -starting_z)`) | **small** | #51 |
| 6 | **`--font-file` ignored**: export always uses bundled FreeSans (`export.rs:90`); the path is validated at import, then dropped. | `font-file` | `gource.cpp:50-67`, `142`; `file.cpp:55-62` | **small** (`FontManager::new` takes bytes; read the file) | #50 area; no ticket for the option |
| 7 | **User avatars** (`colour-images`, `user-image-dir`, `default-user-image`): Stored-unused. | those three | `user.cpp:183-212` (`assignUserImage`) | **medium** (image decode for JPEG/GIF as well, per-user textures, texture cache) | DEVIATIONS G9, #51 |
| 8 | **`--transparent` is only a window flag.** Export clears with alpha 1, never loads `bloom_alpha.tga`; the PPM has no alpha anyway, so the visible effect is the bloom texture. | `transparent` | `gource.cpp:74` (`bloom_alpha.tga`), `2028` (clear alpha 0), `2250` (PNG alpha); `main.cpp:139` | **small** (bloom texture choice) after #53 | #53 |
| 9 | **`--selection-colour` and pointer selection**: no picking, hover or selected labels. | `selection-colour` | `file.cpp:58,141`, `user.cpp:330`, `gource.cpp:545` (`selectBackground`) and the mouse handlers | **large** (pointer picking, text box, selected-label drawing; windowed only) | #51 area; no ticket |
| 10 | **`--multi-sampling`, `--screen`** stored, never used by `WindowedGl`/`window_attributes`. | `multi-sampling`, `screen` | `main.cpp:134` (`SDL_GL_MULTISAMPLE*`), `167` (`display.init(..., screen)`) | **small** each (glutin config samples; winit monitor index) | no ticket |
| 11 | **`--log-level` accepted and ignored.** Burl never prints diagnostics at any level (`main.rs:113`). | `log-level` | `gource_settings.cpp:588-598`, `main.cpp:75` | **small** if only the call sites that log are wired; check what the oracle prints first | no ticket |
| 12 | **Position slider not drawn** though `--hide-progress` is honoured for caption placement. | `hide-progress` | `slider.cpp`; shown only when not exporting with one repo, `gource.cpp:167-168` | **medium** (windowed only) | #51 |
| 13 | **Images are PNG only** for `--logo` and `--background-image` (Gource decodes JPEG and GIF). | `logo`, `background-image` | `reference/gource-cpp-dump/src/core/texture.cpp` (`grabFile`) | **small** | #51 |
| 14 | **Test gaps.** No CLI test of `-o`; no gate case uses `-r`; `every_option_has_an_import_arm` is a text-contains check; six wired options have no test (see above); `--log-command git/hg/cvs-exp/cvs2cl` printers asserted only through shared code. | `-o`, `-r`, six options | n/a | **small** (a tiny `-o -` test with `--debug-dump-max-frames`, a gate case with `-r 30`) | #54 for the visual half |
| - | **Not gaps (do not ticket):** `--dont-stop` (its only C++ consumer is `gource_shell.cpp:224`, mirrored by `apply_shell_fixups`), `--stop-on-idle` and `--ffp` (D5d, D4, `Ignored`), `--realtime` (import sets the day rate), `--hide-mouse`/`--disable-input` (windowed-only by nature), `--windowed`. | | | | |

Ranking rationale: 1 and 2 are what a user hits first (blank windowed run; a silent wrong run). 3 is already owned. 4-6, 8, 10, 11, 13 are cheap and independent. 7 and 9 are the only sizeable ones.

## Method

Everything above is reproducible from the tree; no option was assumed.

1. **Parse parity:** extract `arg_types["x"] = "t"` from `src/gource_settings.cpp` and `reference/gource-cpp-dump/src/core/settings.cpp`, extract `("x", ArgType::T)` from `crates/burl-conf/src/args.rs`, `sort | join`, and diff names and types (114 vs 120, differences listed in the headline).
2. **Store/consume:** for every field of `GourceSettings` and `DisplaySettings`, a script counts word-boundary hits outside `burl-conf`, split into production code and tests (`#[cfg(test)]` blocks and `tests/`). Fields with zero production hits were then checked by hand against the C++ consumer (`grep -n` in `src/`), and every "Wired" citation was confirmed by reading the line, because bare names (`padding`, `screen`, `title`, `path`, `font_size`) collide with unrelated locals. Options whose only consumer lives in `burl-conf` (`dont-stop`, `realtime`, `windowed`, `disable-auto-skip`) were checked in `settings.rs`.
3. **Not-ported markers:** `grep -rniE "not ported|not honou?red|not yet|nyi|unimplemented"` over `crates/`: `sim.rs:5,326-342`, `scene.rs:6`, `labels.rs:5`, `key.rs:4`, `overlays.rs:4`, `export.rs:5,90`, `main.rs:113`.
4. **Tests:** a script matches each option name (`--name`, `"name"`, `name=` in conf strings) in `crates/burl-conf/{tests,src}`, `crates/burl/tests`, `tests/golden/sim/*.{args,conf}` + `tests/test.conf`, and its field name in render/sim/format tests.
5. **Windowed vs export:** read `interactive.rs` (`App::start`, `Live::frame`, `run`) against `export.rs::run`.
