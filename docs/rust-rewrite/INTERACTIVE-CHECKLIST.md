# Interactive checklist (parity release sign-off)

The windowed path (#20, #14 item 5), split into what CI proves and what still needs a person.
The user fills the sign-off block at the end **in the tagged commit**. `scripts/check-release.sh`
(run by `release.yml`) fails while any box below is unticked, any platform line is `TODO`,
`DEVIATIONS.md` changed after the recorded review, or **a proof named in the tables is gone**
(a renamed or deleted test, a reworded window check): every `` `path::name` `` in a table row must
still exist, so a covered item cannot silently lose its coverage.

Platforms: **macOS Retina**, **macOS non-Retina** (external display), **Linux X11**. Wayland is
best-effort, not a gate. Bindings are the app-core table (`discovery/app-core.md` T2.2).
Use any real log, e.g. `burl --seconds-per-day 0.5 <repo>`.

## Proved by CI (no action)

Proof format: `file::name`. A Rust name is a `fn`; any other text is matched literally in that file
(the `window-xvfb` job's check messages, the `heavy.yml` job names). The `window-xvfb` job runs
the real window under Xvfb + openbox and drives it with xdotool, Linux X11 only (`heavy.yml`,
which `release.yml` re-runs on the tag).

### Window and HiDPI

| Item | Proof |
|---|---|
| A viewport without `--high-dpi` opens a window of exactly that pixel size (G3); a default or `--high-dpi` window is logical; font scale follows the ratio | `crates/burl/src/interactive.rs::deviation_g3_viewport_without_high_dpi_is_exact_pixels`, `crates/burl/src/interactive.rs::default_and_high_dpi_windows_are_logical`, `crates/burl/src/interactive.rs::font_scale_follows_the_dpi_ratio_then_the_height` |
| The window opens, is shown without help, at 640x360 (Linux X11) | `scripts/window-in-container.sh::window is shown (mapped) without help`, `scripts/window-in-container.sh::window opens at the requested pixel size (G3)` |
| The scene is the right way up (the windowed blit once mirrored it) | `scripts/window-in-container.sh::the scene is upright` |
| Resize reaches the window and the scene redraws at the new size (far corner drawn); proportions are not measured | `scripts/window-in-container.sh::resize reaches the window`, `scripts/window-in-container.sh::resized scene is redrawn to the new size` |
| Alt+Return enters and leaves fullscreen (Linux X11) | `scripts/window-in-container.sh::Alt+Return goes fullscreen`, `scripts/window-in-container.sh::Alt+Return leaves fullscreen` |

### Keyboard

The key-to-action mapping is unit-tested; the thin glue that applies an action to the window
(`Live::act` in `interactive.rs`) is not, hence the smoke box below.

| Item | Proof |
|---|---|
| Esc quits, also under `--disable-input`; repeats ignored | `crates/burl/src/interactive/input.rs::shell_keys_ignore_repeat_and_escape_ignores_disable_input`, `scripts/window-in-container.sh::Esc quits cleanly` |
| Space pauses: maps to pause; a paused sim freezes the clock but not the camera | `crates/burl/src/interactive/input.rs::character_rows`, `crates/burl-sim/src/sim.rs::paused_freezes_the_clock_but_not_the_camera` |
| `=`/`+`, `-`, `.`, `,`, `/`, `[`, `]`, `N` map to their commands; gravity and skip-idle take effect | `crates/burl/src/interactive/input.rs::character_rows`, `crates/burl-sim/src/command.rs::arrows_and_gravity_rows`, `crates/burl-sim/src/sim.rs::force_gravity_scales_the_pull_toward_the_parent`, `crates/burl-sim/src/sim.rs::skip_idle_sets_the_idle_clock` |
| T, G, R map to the tree, users and root toggles; the hide flags empty their buffers | `crates/burl/src/interactive/input.rs::character_rows`, `crates/burl-render/tests/scene.rs::hide_flags_empty_their_buffers`, `crates/burl-render/tests/scene.rs::hide_root_skips_edges_to_the_roots_children` |
| U, D, F map to the oracle's three-state name cycles (hidden, highlighted, shown; files also extensions only), K to the key legend | `crates/burl/src/interactive/input.rs::character_rows`, `crates/burl/src/frame.rs::label_toggles_flip_their_own_flag` |
| M maps to hide-mouse; a hidden mouse drops presses but lets releases through, and `--disable-input` drops all | `crates/burl/src/interactive/input.rs::character_rows`, `crates/burl/src/interactive.rs::button_events_are_gated_by_disable_input_and_hidden_mouse_but_releases_pass` |
| Tab selects the next user (ignored under `--disable-input`; also in track mode) | `crates/burl/src/interactive/input.rs::buttons_map_to_the_sims_and_tab_selects_the_next_user`, `crates/burl-sim/tests/pointer_script.rs::scripted_tab_is_ignored_under_disable_input`, `crates/burl/tests/sim_gate.rs::sim_gate_small_repo_follow` |
| Arrows pan, held-key state, also while paused | `crates/burl/src/interactive/input.rs::arrows_are_held_state`, `crates/burl-sim/src/sim.rs::arrows_assign_in_cursor_order_and_make_the_camera_manual`, `crates/burl-sim/src/sim.rs::manual_camera_pans_by_cam_rate_and_stays_put` |
| Keypad `+`/`-` zoom (reverse of the character keys) | `crates/burl/src/interactive/input.rs::keypad_zoom_is_not_the_character_row` |

### Mouse

| Item | Proof |
|---|---|
| Wheel zooms in and out and respects the zoom clamp | `crates/burl/src/interactive/input.rs::wheel_sign_only`, `crates/burl-sim/src/command.rs::zoom_clamps` |
| Left-drag on the background pans; clicking empty background deselects (`selectBackground`, grab, drag), diffed against the oracle | `crates/burl/tests/sim_gate.rs::sim_gate_small_repo_select` |
| Right-drag rotates the tree about the camera (the right button's own event mapping is not asserted) | `crates/burl-sim/src/sim.rs::manual_rotation_turns_the_tree_about_the_camera` |
| Clicking a user or a file selects it; the selected sprite is white; its name is drawn last in `--selection-colour` (default yellow) | `crates/burl/tests/sim_gate.rs::sim_gate_small_repo_select`, `crates/burl-render/tests/scene.rs::selected_sprites_are_white`, `crates/burl-render/tests/names.rs::selected_names_are_drawn_last_in_the_selection_colour_whatever_the_fade_or_hide`, `crates/burl-render/tests/names.rs::the_default_selection_colour_is_yellow` |
| The click lands on what is under the cursor: pick and hover diffed against the oracle, overlapping users pick in name order, and the pointer is physical pixels at any scale factor. A fading-out user is skipped in code (`user_is_fading`) but no test pins it | `crates/burl/tests/sim_gate.rs::sim_gate_small_repo_pick`, `crates/burl-sim/tests/deviations.rs::deviation_d13_overlapping_users_pick_in_name_order`, `crates/burl/src/interactive.rs::the_pointer_is_physical_pixels_whatever_the_scale_factor` |
| Hover shows a user's name, a file's name then its directory, nothing for a selected pawn; the box sits above-left and flips inside the window | `crates/burl-sim/tests/deviations.rs::hover_text_names_the_pawn_under_the_mouse`, `crates/burl-render/src/textbox.rs::the_box_sits_above_left_of_the_cursor_and_flips_inside_the_window` |
| The cursor hides after 3 s without moving and a move brings it back; "a click then does nothing" is not asserted | `crates/burl-sim/tests/pointer_script.rs::the_cursor_hides_after_three_idle_seconds_and_a_move_brings_it_back` |
| The timeline slider: hit-test, click-to-percent seek, fade in on hover and out after 1 s, layout snapshot; the slider ignores a hidden or grabbed pointer | `crates/burl-render/tests/slider.rs::hit_test_and_percent`, `crates/burl-render/tests/slider.rs::click_seeks_only_on_the_slider`, `crates/burl-render/tests/slider.rs::fade`, `crates/burl-render/tests/slider.rs::slider_matches_snapshot`, `crates/burl-sim/src/sim.rs::slider_peek_and_seek`, `crates/burl/src/interactive.rs::the_slider_ignores_a_hidden_or_grabbed_pointer` |

### Render

| Item | Proof |
|---|---|
| Scene structure matches Gource (per-frame SSIM against the oracle). Its run hides text, date and bloom, so those are not covered | `.github/workflows/heavy.yml::visual-ssim:` |
| `--user-image-dir` and `--default-user-image`: the directory scan, own image then default then stock, `--colour-images` keeps the hash colour, a bad file is reported by name, and the render changes through the real binary | `crates/burl/tests/export.rs::user_image_dir_changes_the_render`, `crates/burl-render/tests/avatars.rs::scan_keeps_jpg_jpeg_png_keyed_by_stem_case_insensitively`, `crates/burl-render/tests/avatars.rs::own_image_then_default_then_stock`, `crates/burl-render/tests/avatars.rs::colour_images_keeps_the_hash_colour`, `crates/burl/src/frame.rs::avatar_hook_and_fatal_bad_file` |

## Still manual (per platform, tick each)

Only what needs a real display, real hardware or a person's eye. Tick on all three platforms
unless a line says otherwise.

- [ ] `--high-dpi` on Retina: line widths and text are crisp, not blurry or doubled (macOS Retina)
- [ ] Moving the window between displays of different scale does not break layout (macOS)
- [ ] Fullscreen toggle and resize keep the scene proportioned: no black bars or stretching (the Linux job checks only that it redraws)
- [ ] Smoke the key glue once in a real window: Space, T, G, R, U, D, F, K, M, `=`/`-`, arrows, Tab, keypad zoom, wheel
- [ ] Smoke the pointer glue once: click selects a user and a file, hover box follows the cursor and flips at the edges, background drag pans, right-drag rotates
- [ ] Cursor grab hides and confines the cursor during a drag and releases it when the last button goes up
- [ ] The slider's date caption is clamped to the window and the slider is absent with `--hide-progress` and with a stdin log
- [ ] Bloom, shadows, file and user textures, beams and the key legend look like Gource's at the same settings (SSIM hides bloom and text)
- [ ] Text (user, file, directory names, captions, title, date) is readable and positioned like Gource's
- [ ] Pause/resume and seeking leave no stale frame or flicker

## Not ported

F12, V and middle click, S, C, P and the debug keys Q, W, Y are not in the windowed path; that is
deviation G10 in `DEVIATIONS.md`, reviewed with the register rather than ticked here. Z is a
no-op in Gource too. `scripts/check-release.sh` fails if a `## Known gap` section with bullets
is ever added back.

## Sign-off

Replace each `TODO`. `deviation-register` is `git hash-object docs/rust-rewrite/DEVIATIONS.md`
after the user has reviewed every row; any later edit to the register invalidates it.

version: TODO
deviation-register: TODO
macos-retina: TODO
macos-non-retina: TODO
linux-x11: TODO
