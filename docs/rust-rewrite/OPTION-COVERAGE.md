# Option coverage: Gource 0.57 settings vs Burl

Audit of every option and config key in Gource 0.57's settings against Burl. Written at `33ead47` (master after #68); **refreshed at `ceb39a8` (master after #103)**, see "Refresh" in Method. Report only: no code was changed. Deviation rows (`D*`, `G*`) are in [DEVIATIONS.md](DEVIATIONS.md); this file does not edit it.

## Headline

- **Parse: complete.** The C++ option table (`arg_types[...]` in `src/gource_settings.cpp` plus the display options in `reference/gource-cpp-dump/src/core/settings.cpp`) has **114** entries. Burl's `ARG_TYPES` (`crates/burl-conf/src/args.rs`) has **120**: the same 114 with identical types, plus the six hidden test controls of ticket #22 (`--debug-dump-state/-every/-max-frames`, `--prng`, `--prng-seed`, `--rand-script`). The only type differences are `--ffp` and `--stop-on-idle`: `bool` in C++, `Ignored` in Burl (D4, D5d, ticket #8). Aliases and command-line-section routing were compared the same way.
- **Store: complete.** Every gource option has a field in `GourceSettings` (`settings.rs:142`) or `DisplaySettings` (`settings.rs:28`) except the two `Ignored` ones.
- **Changes behaviour (at `ceb39a8`):** every option is wired except `--selection-colour` (**Stored-unused**: no pointer picking, scope in [#92](https://github.com/dfadler/Burl/issues/92)). Partial: `--hide-progress` (caption rows move, but the position slider is not drawn on `master`; PR #113), `--transparent` (no visible effect in `-o` by design). `--user-image-dir`, `--default-user-image` and `--colour-images` are wired in `-o` only until PR #107 (windowed, [#91](https://github.com/dfadler/Burl/issues/91)) merges. `--stop-on-idle` and `--ffp` are `Ignored` by design (D5d, D4). The earlier "Refused" options (`--start-position N`, `--loop`, `--follow-user`, `--camera-mode track`) and the silent `random` start position are all wired.
- **Windowed vs `-o`:** both paths now draw the same frame (`crates/burl/src/frame.rs`, `FrameKit`: scene, names, date, title, captions, key, logo, background image), so the Windowed column is `Y` for those options. Open on the windowed path: multi-section runs stall on a stdin section ([#99](https://github.com/dfadler/Burl/issues/99)); avatars (PR #107, #91); the position slider (PR #113); the size after a resize (PR #109, #98).

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
| `--camera-mode` | Str | `camera_mode` | Y | Y | Wired | main.rs:245 (`track_users`), sim.rs:392 | U | gource.cpp:528-541 |
| `--caption-duration` | Float | `caption_duration` | Y | Y | Wired | burl-sim/src/sim.rs:2146 | SR |  |
| `--caption-file` | Str | `caption_file` | Y | Y | Wired | burl-sim/src/sim.rs:2092 | UES |  |
| `--crop` | Str | `crop_vertical`, `crop_horizontal` | Y | Y | Wired | burl-sim/src/sim.rs:194-196 | U |  |
| `--disable-auto-rotate` | Bool | `disable_auto_rotate` | Y | Y | Wired | burl-sim/src/sim.rs:2008 | UER |  |
| `--disable-auto-skip` | Bool | `auto_skip_seconds` | Y | Y | Wired | settings.rs:556 sets -1; sim.rs:555 | UER |  |
| `--elasticity` | Float | `elasticity` | Y | Y | Wired | burl-sim/src/sim.rs:1932 | US |  |
| `--file-idle-time` | Str | `file_idle_time` | Y | Y | Wired | burl-sim/src/sim.rs:1290 | UES |  |
| `--file-idle-time-at-end` | Str | `file_idle_time_at_end` | Y | Y | Wired | burl-sim/src/sim.rs:643 | UES |  |
| `--filename-time` | Float | `filename_time` | Y | Y | Wired | burl-sim/src/sim.rs:701 | US |  |
| `--follow-user` | MultiValue | `follow_users` | Y | Y | Wired | sim.rs:1184; gate case `small-repo-follow` | UR S | gource.cpp:1390 |
| `--hash-seed` | Int | `hash_seed` | Y | Y | Wired | burl-sim/src/sim.rs:827, commit.rs:51 | UES |  |
| `--highlight-user` | MultiValue | `highlight_users` | Y | Y | Wired | burl-sim/src/sim.rs:781 | U |  |
| `--highlight-users` | Bool | `highlight_all_users` | Y | Y | Wired | burl-sim/src/sim.rs:777, labels.rs:166 | UR |  |
| `--loop` | Bool | `loop_` | Y | Y | Wired | sim.rs:610; gate case `small-repo-loop` | UR S | gource.cpp:1704-1712 |
| `--loop-delay-seconds` | Float | `loop_delay_seconds` | Y | Y | Wired | sim.rs:613 | US | gource.cpp:1708 |
| `--max-file-lag` | Str | `max_file_lag` | Y | Y | Wired | burl-sim/src/sim.rs:1108 | US |  |
| `--max-files` | Int | `max_files` | Y | Y | Wired | burl-sim/src/sim.rs:684 | US |  |
| `--max-user-speed` | Float | `max_user_speed` | Y | Y | Wired | burl-sim/src/sim.rs:835 | S |  |
| `--no-time-travel` | Bool | `no_time_travel` | Y | Y | Wired | burl-sim/src/sim.rs:570 | ES |  |
| `--output-framerate` | Int | `output_framerate` | Y | Y | Wired | burl-sim/src/sim.rs:345-347 (tick rate only; no ffmpeg) | UE |  |
| `--padding` | Float | `padding` | Y | Y | Wired | burl-sim/src/sim.rs:356 (camera) | U |  |
| `--seconds-per-day` | Float | `days_per_second` | Y | Y | Wired | burl-sim/src/sim.rs:534, command.rs:158 | UES |  |
| `--start-position` | Str | `start_position`, `start_position_random` | Y | Y | Wired (N and `random`) | sim.rs:418-421 (`random` draws `rand() % 1000 / 1000`) | URS | gource.cpp:1604; gource_settings.cpp:1318 |
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
| `--background-image` | Str | `background_image` | Y* | Y | Wired | burl/src/export.rs:137 | R* | PNG only (Gource also reads JPEG/GIF); export only. #51. |
| `--bloom-intensity` | Float | `bloom_intensity` | Y | Y | Wired | burl-render/src/scene.rs:196 | U |  |
| `--bloom-multiplier` | Float | `bloom_multiplier` | Y | Y | Wired | burl-render/src/scene.rs:194 | **none** |  |
| `--caption-colour` | Str | `caption_colour` | Y | Y | Wired | burl-render/src/overlays.rs:144 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--caption-offset` | Int | `caption_offset` | Y | Y | Wired | burl-render/src/overlays.rs:114 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--caption-size` | Int | `caption_size` | Y | Y | Wired | burl-render/src/overlays.rs:45, export.rs:168 | **none** | Export only: the windowed frame draws no text (see Gap 1). |
| `--colour-images` | Bool | `colour_user_images` | Y | N (#107) | Wired (export); windowed open (G9, #91) | burl-render/src/avatars.rs; export.rs:101-110 | R | user.cpp:197,210 |
| `--date-format` | Str | `date_format` | Y | Y | Wired | burl/src/export.rs:203 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--default-user-image` | Str | `default_user_image` | Y | N (#107) | Wired (export); windowed open (G9, #91) | burl-render/src/avatars.rs; export.rs:101-110 | R | user.cpp:209 |
| `--dir-colour` | Str | `dir_colour` | Y | Y | Wired | burl-render/src/labels.rs:27 via export.rs:155 | U | Export only: the windowed frame draws no text (see Gap 1). |
| `--dir-font-size` | Int | `dirname_font_size`, `scaled_dirname_font_size` | Y | Y | Wired | burl/src/export.rs:161 | **none** | Export only: the windowed frame draws no text (see Gap 1). |
| `--dir-name-depth` | Int | `dir_name_depth` | Y | Y | Wired | burl-render/src/labels.rs:116 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--dir-name-position` | Float | `dir_name_position` | Y | Y | Wired | burl-render/src/labels.rs:132 | U | Export only: the windowed frame draws no text (see Gap 1). |
| `--file-extension-fallback` | Bool | `file_extension_fallback` | Y | Y | Wired | burl-render/src/labels.rs:151, overlays.rs:73 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--file-extensions` | Bool | `file_extensions` | Y | Y | Wired | burl-render/src/labels.rs:150 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--file-font-size` | Int | `filename_font_size`, `scaled_filename_font_size` | Y | Y | Wired | burl/src/export.rs:162 | **none** | Export only: the windowed frame draws no text (see Gap 1). |
| `--filename-colour` | Str | `filename_colour` | Y | Y | Wired | burl-render/src/labels.rs:28 via export.rs:156 | **none** | Export only: the windowed frame draws no text (see Gap 1). |
| `--fixed-user-size` | Bool | `fixed_user_size` | Y | Y | Wired | scene.rs:172; export.rs:125, interactive.rs:311 | R | gource.cpp:2273 |
| `--font-colour` | Str | `font_colour` | Y | Y | Wired | burl-render/src/overlays.rs:21 via export.rs:177 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--font-file` | Str | `font_file` | Y | Y | Wired | export.rs:75-78, frame.rs `new_fonts`; `load_font_file` (font.rs:21) | UR | gource.cpp:50-67, file.cpp:55-62. Validation at import still runs. |
| `--font-scale` | Float | `font_scale`, `default_font_scale` | Y | Y | Wired | burl/src/export.rs:109-110 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--font-size` | Int | `font_size`, `scaled_font_size` | Y | Y | Wired | burl/src/export.rs:167; key.rs:55 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--hide` | Str | `hide_*` | Y | part | Wired | settings.rs:491-547 (comma list OR-ed with every --hide-X flag); consumers as the individual hide-* rows | UER | Windowed honours tree/users/files/bloom/mouse only. Tested by `hide_list_quirks`. |
| `--hide-bloom` | Bool | `hide_bloom` | Y | Y | Wired | burl-render/src/scene.rs:191 | R |  |
| `--hide-date` | Bool | `hide_date` | Y | Y | Wired | burl-render/src/overlays.rs:124 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--hide-dirnames` | Bool | `hide_dirnames` | Y | Y | Wired | burl-render/src/labels.rs:121 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--hide-filenames` | Bool | `hide_filenames` | Y | Y | Wired | burl-render/src/labels.rs:138 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--hide-files` | Bool | `hide_files` | Y | Y | Wired | burl-render/src/scene.rs:146, labels.rs:138 | R |  |
| `--hide-progress` | Bool | `hide_progress` | Y | N | Wired-partial | burl-render/src/overlays.rs:89 | R | Only moves the caption rows; the position slider it hides is drawn in the window only (`slider.rs`, #93). C++ shows the slider only when not exporting and one repo (gource.cpp:167-168), so this matters for the windowed path. |
| `--hide-root` | Bool | `hide_root` | Y | Y | Wired | scene.rs:148; export.rs:124, interactive.rs:310 (toggle `R`, interactive.rs:173) | UR | dirnode.cpp:1111,1124,1138 (spline to the root dir skipped); `h`-style toggle gource.cpp:784 |
| `--hide-tree` | Bool | `hide_tree` | Y | Y | Wired | burl-render/src/scene.rs:138 | R | Also the `T`/`U` toggles in the windowed path. |
| `--hide-usernames` | Bool | `hide_usernames` | Y | Y | Wired | burl-render/src/labels.rs:160 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--hide-users` | Bool | `hide_users` | Y | Y | Wired | burl-render/src/scene.rs:159, labels.rs:160 | R | Also the `T`/`U` toggles in the windowed path. |
| `--highlight-colour` | Str | `highlight_colour` | Y | Y | Wired | burl-render/src/labels.rs:29 via export.rs:157 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--highlight-dirs` | Bool | `highlight_dirs` | Y | Y | Wired | burl-render/src/labels.rs:118 | R | Export only: the windowed frame draws no text (see Gap 1). |
| `--key` | Bool | `show_key` | Y | Y | Wired | burl-render/src/overlays.rs:149 | UR | Export only: the windowed frame draws no text (see Gap 1). |
| `--logo` | Str | `logo` | Y* | Y | Wired | burl/src/export.rs:141 | R* | PNG only; export only. #51. |
| `--logo-offset` | Str | `logo_offset` | Y | Y | Wired | burl/src/export.rs:197 | UR* |  |
| `--selection-colour` | Str | `selection_colour` | Y | Y | Wired | burl-render/src/labels.rs (selected label), sim colours | `small-repo-pick`, `small-repo-select`, `names.rs` | file.cpp:58,141; user.cpp:330; needs pointer selection, so windowed-relevant |
| `--title` | Str | `title` | Y | Y | Wired | burl-render/src/overlays.rs:96 | UE | Export only: the windowed frame draws no text (see Gap 1). |
| `--user-font-size` | Int | `user_font_size`, `scaled_user_font_size` | Y | Y | Wired | burl/src/export.rs:163 | U | Export only: the windowed frame draws no text (see Gap 1). |
| `--user-image-dir` | Str | `user_image_dir`, `user_image_map` | Y | N (#107) | Wired (export); windowed open (G9, #91) | burl-render/src/avatars.rs; export.rs:101-110 | RE | user.cpp:183 |

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
| `--multi-sampling` | Bool | `multisample` | - | Y | Wired | burl-gl/src/window.rs:20 (`sample_attempts`); interactive.rs:212 | **none** | main.cpp:134 |
| `--no-vsync` | Bool | `vsync` | - | Y | Wired | burl-gl/src/window.rs:61, interactive.rs:151 | U |  |
| `--screen` | Int | `screen` | - | Y | Wired | burl-gl `monitor_index` (window.rs:30); interactive.rs:209 | UR | main.cpp:167 |
| `--transparent` | Bool | `transparent` | N | Y* | Wired | interactive.rs:46 (window attribute); export.rs `clear_colour` (alpha 0) | `clear_colour` unit test | No output effect in `-o` (PPM has no alpha; bloom textures are `--ffp`-only). gource.cpp:74,2028,2214,2250; main.cpp:139 |
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
| `--log-level` | Str | - | n/a | n/a | Wired | main.rs:119, 355 (`burl_util::log::set_level`) | E | main.cpp:75; gource_settings.cpp:588-598. Burl never prints diagnostics at any level. |
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

The 2026-11 refresh found two options with no test naming them: `--multi-sampling` (wired, window-attribute only; needs a display) and `--selection-colour` (stored, unused). Of the earlier thirteen, the rest are now named by a test or are wired with one (`fixed_user_size_scales_users_with_camera_distance`, `crates/burl/tests/log_level.rs`, the `export.rs` option tests); the per-option check is the script in Method step 6.

## Real gaps: state on `master` (`ceb39a8`)

The original eleven-row list is kept as history in `git log` for this file; what remains is below. The numbers are the original ones.

| # | Gap | State |
|---|---|---|
| 1 | Windowed text, overlays, multi-section | **Landed** (`3c368c0`, `e6738ff`). Open: multi-section stdin stall ([#99](https://github.com/dfadler/Burl/issues/99)), resize ([#98](https://github.com/dfadler/Burl/issues/98), PR #109) |
| 2, 3 | `--start-position` random and N, `--loop`, `--follow-user`, `--camera-mode track` | **Landed** (`7b6e5e4`, `e1cc912`, `23e556b`) |
| 4 | `--hide-root` | **Landed** (`scene.rs:148`) |
| 5 | `--fixed-user-size` | **Landed** (`scene.rs:172`) |
| 6 | `--font-file` | **Landed** (`export.rs:75`, `load_font_file`) |
| 7 | Avatars | Export **landed** (G9). Windowed: PR #107; NFC keys: PR #110; G8 mip check: evidence in a comment on [#91](https://github.com/dfadler/Burl/issues/91) |
| 8 | `--transparent` | Closed, no visible effect in `-o` (unchanged) |
| 9 | `--selection-colour`, pointer selection | **PRs open** ([#92](https://github.com/dfadler/Burl/issues/92)): #119, #121, #122, hover box #124 |
| 10 | `--multi-sampling`, `--screen` | **Landed** (`6f160b5`) |
| 11 | `--log-level` | **Landed** (`6e4f4fb`) |
| 12 | Position slider | Windowed only; **PR #113** (never drawn in `-o`) |
| 13 | JPEG and GIF for `--logo`, `--background-image` | **Landed** (`frame.rs` `load_image`, magic-number dispatch) |
| 14 | Test gaps | **Done** (PRs #85, #103) |

Not gaps, unchanged: `--dont-stop`, `--stop-on-idle`, `--ffp`, `--realtime`, `--hide-mouse`/`--disable-input`, `--windowed`.

## Method

Everything above is reproducible from the tree; no option was assumed.

1. **Parse parity:** extract `arg_types["x"] = "t"` from `src/gource_settings.cpp` and `reference/gource-cpp-dump/src/core/settings.cpp`, extract `("x", ArgType::T)` from `crates/burl-conf/src/args.rs`, `sort | join`, and diff names and types (114 vs 120, differences listed in the headline).
2. **Store/consume:** for every field of `GourceSettings` and `DisplaySettings`, a script counts word-boundary hits outside `burl-conf`, split into production code and tests (`#[cfg(test)]` blocks and `tests/`). Fields with zero production hits were then checked by hand against the C++ consumer (`grep -n` in `src/`), and every "Wired" citation was confirmed by reading the line, because bare names (`padding`, `screen`, `title`, `path`, `font_size`) collide with unrelated locals. Options whose only consumer lives in `burl-conf` (`dont-stop`, `realtime`, `windowed`, `disable-auto-skip`) were checked in `settings.rs`.
3. **Not-ported markers:** `grep -rniE "not ported|not honou?red|not yet|nyi|unimplemented"` over `crates/`: `sim.rs:5,326-342`, `scene.rs:6`, `labels.rs:5`, `key.rs:4`, `overlays.rs:4`, `export.rs:5,90`, `main.rs:113`.
4. **Tests:** a script matches each option name (`--name`, `"name"`, `name=` in conf strings) in `crates/burl-conf/{tests,src}`, `crates/burl/tests`, `tests/golden/sim/*.{args,conf}` + `tests/test.conf`, and its field name in render/sim/format tests.
5. **Windowed vs export:** read `interactive.rs` (`App::start`, `Live::frame`, `run`) against `export.rs::run`.
6. **Refresh (2026-11, `ceb39a8`):** for each option whose status was not `Wired` or whose Windowed column was `N`, `grep -rnE "<field>" crates/burl/src crates/burl-sim/src crates/burl-render/src crates/burl-gl/src` (excluding `burl-conf`, tests and `//!` comments) gave the consumer lines now cited; `grep -rniE "not ported|not honou?red|unimplemented|nyi" crates` shows the remaining markers (`scene.rs:6` and `labels.rs:5` culling, `key.rs:4`, `overlays.rs:4`, `settings.rs:5` NFC). The Windowed column for the rendering options was set from `interactive.rs` building `FrameKit` (`frame.rs`), the same frame builder `export.rs` uses. Only `ceb39a8` is checked; rows of the table that were `Wired` with a `Y` in both columns were not re-read.
