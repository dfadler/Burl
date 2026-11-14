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
| Holding the right button confines the pointer to the window and releasing it frees the pointer (Linux X11 `Confined` grab; `Locked` is unsupported on X11 and not probed; hiding the cursor is not asserted) | `scripts/window-in-container.sh::holding a button confines the pointer to the window`, `scripts/window-in-container.sh::releasing the button releases the confinement` |
| Alt+Return enters and leaves fullscreen (Linux X11) | `scripts/window-in-container.sh::Alt+Return goes fullscreen`, `scripts/window-in-container.sh::Alt+Return leaves fullscreen` |
| A window at scale factor 2 on Linux X11 (`WINIT_X11_SCALE_FACTOR=2`, `--high-dpi`, `-640x360`): burl reports scale 2, logical 640x360, physical 1280x720 and the X window is 1280x720 pixels. This forces the scale winit reports; it is not a real multi-monitor move or a Retina panel, and it does not click anything at that scale (the pointer claim at any scale factor stays a unit test) | `scripts/window-in-container.sh::scale factor 2: burl reports scale=2 logical=640x360 physical=1280x720 and the X window is 1280x720 pixels` |
| Burl logs the scale and size it sees (`display:` lines at start, on a scale change and on resize; logical is physical over scale) | `crates/burl/src/interactive.rs::display_line_pins_the_log_format`, `crates/burl/src/interactive.rs::logical_size_divides_by_the_scale_and_rounds` |
| macOS, virtual 2x and 1x displays, a real mouse drag of the title bar (press, 40 drag steps, release via `CGEventPost`) from the 1x display to the 2x display and back: burl logs `scale-changed` then `resized` with the target scale and logical size, and the window lands on the target display; after each leg the window's point size and burl's logical size are unchanged and a `screencapture` of it is scale x its point size and not blank, and the scene fills the window (seven corner and edge probes match the field's median luma; the date caption's own band is checked by the caption test) with the near-white date caption present and centred within 5% of the width (coarse: it catches a surface that did not follow the move, not overlap, clipping or text size). The cursor position is not asserted, and visual layout (by eye), doubled lines and crispness are not judged. Same non-required workflow as the rows below | `.github/workflows/display-harness.yml::cargo xtask display-harness --drag`, `xtask/src/display_harness.rs::drag_path_ends_exactly_and_steps_sum_to_the_distance`, `xtask/src/display_harness.rs::the_press_is_in_the_title_bar_and_the_release_in_the_target_display`, `xtask/src/display_harness.rs::a_move_must_keep_the_point_and_logical_sizes`, `xtask/src/pixels.rs::a_filled_scene_with_centred_caption_passes_at_1x_and_2x`, `xtask/src/pixels.rs::a_scene_in_the_top_left_quarter_fails`, `xtask/src/pixels.rs::a_white_bar_on_one_edge_fails`, `xtask/src/pixels.rs::a_white_bar_on_the_top_edge_fails`, `xtask/src/pixels.rs::a_caption_covering_the_top_inset_row_is_not_an_unfilled_edge`, `xtask/src/pixels.rs::a_missing_caption_fails`, `xtask/src/pixels.rs::an_off_centre_caption_fails`, `xtask/src/pixels.rs::real_captures_fill_the_window_with_a_centred_caption` |
| macOS, virtual 2x and 1x displays: moving the window between them logs `scale-changed` then `resized` with the right scale and logical size, and the window capture is 2x its point size on the 2x display and 1x on the 1x display and is not blank. Layout and crispness are not judged. Runs in `display-harness.yml`, a non-required workflow `release.yml` does not re-run, so these rows pin that the checks exist, not that they passed on the tag | `.github/workflows/display-harness.yml::cargo xtask display-harness`, `xtask/src/display_harness.rs::a_scale_change_needs_scale_changed_then_resized`, `xtask/src/display_harness.rs::rejects_wrong_scale_and_inconsistent_logical_size`, `xtask/src/display_harness.rs::capture_must_be_scaled_and_not_blank` |
| macOS, virtual 2x and 1x displays, `--high-dpi` with a `-640x360` viewport (a logical-size window): both displays report logical 640x360 and the window is 640x360 pt, the physical size doubles on 2x (1280x720), the captures are scale x the point size, and each capture is crisp by a golden-free measure (finer-than-2px detail relative to its own blurred copy, with the blurred copy required to fail). Not a pixel comparison with the 1x capture (the 2x frame is framed differently), and a real compositor upscale of a 1x render was never reproduced, so this is evidence, not the Retina eyeball check. Same non-required workflow as the row above | `.github/workflows/display-harness.yml::cargo xtask display-harness --high-dpi`, `xtask/src/display_harness.rs::high_dpi_window_is_the_same_logical_size_and_doubles_physically`, `xtask/src/pixels.rs::a_native_render_is_crisp_and_its_blurred_copy_is_not`, `xtask/src/pixels.rs::a_stretched_1x_render_is_not_crisp`, `xtask/src/pixels.rs::real_capture_passes_and_its_blurred_copies_fail` |
| macOS, virtual 2x and 1x displays, default `cargo xtask display-harness` (not `--high-dpi`): the 2x window capture, at the same physical size as the 1x capture, equals the 1x capture pixel for pixel, and a blurred copy of the 2x frame is required to fail that same compare (negative control). So the 2x render is not a blurry upscale of the 1x one at that size; it is not the Retina eyeball check. Runs only in the default invocation, not `--high-dpi`, in the same non-required workflow as the rows above | `.github/workflows/display-harness.yml::cargo xtask display-harness`, `xtask/src/pixels.rs::identical_native_pair_passes`, `xtask/src/pixels.rs::a_blurred_2x_fails_against_the_native_1x_frame` |
| macOS, `--grab` (no virtual display): one real left press and release on the background logs exactly one `grab: event=on mode=locked` then one `grab: event=off` from burl, with a no-button motion control logging none. This checks only burl's own `grab:` log lines, NOT where the cursor goes (synthetic `CGEventPost` drag moved the cursor while burl held the lock, so a position check cannot tell a lock from none) and NOT that motion is detached from a real device, so the manual box "Cursor grab hides and confines the cursor..." stays for that. It DOES assert the cursor is hidden: a full-screen `screencapture -C` shows the sprite before the press and must not show it while the grab is held. Same non-required workflow as the rows above | `.github/workflows/display-harness.yml::cargo xtask display-harness --grab`, `xtask/src/display_harness.rs::one_press_is_one_locked_grab_then_one_release`, `xtask/src/display_harness.rs::parses_burls_grab_lines_and_ignores_others`, `xtask/src/pixels.rs::a_cursor_that_shows_before_and_is_gone_while_held_passes`, `xtask/src/pixels.rs::a_cursor_still_drawn_while_held_fails` |

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
| Tab selects the next user and wraps from the last to the first (ignored under `--disable-input`; also in track mode) | `crates/burl/src/interactive/input.rs::buttons_map_to_the_sims_and_tab_selects_the_next_user`, `crates/burl-sim/tests/pointer_script.rs::tab_walks_the_users_and_wraps_to_the_first`, `crates/burl-sim/tests/pointer_script.rs::scripted_tab_is_ignored_under_disable_input`, `crates/burl/tests/sim_gate.rs::sim_gate_small_repo_follow` |
| Arrows pan, held-key state, also while paused | `crates/burl/src/interactive/input.rs::arrows_are_held_state`, `crates/burl-sim/src/sim.rs::arrows_assign_in_cursor_order_and_make_the_camera_manual`, `crates/burl-sim/src/sim.rs::manual_camera_pans_by_cam_rate_and_stays_put` |
| Keypad `+`/`-` zoom (reverse of the character keys) | `crates/burl/src/interactive/input.rs::keypad_zoom_is_not_the_character_row` |

### Mouse

| Item | Proof |
|---|---|
| Wheel zooms in and out and respects the zoom clamp | `crates/burl/src/interactive/input.rs::wheel_sign_only`, `crates/burl-sim/src/command.rs::zoom_clamps` |
| Left-drag on the background pans; clicking empty background deselects (`selectBackground`, grab, drag), diffed against the oracle | `crates/burl/tests/sim_gate.rs::sim_gate_small_repo_select` |
| Right-drag rotates the tree about the camera (the right button's own event mapping is not asserted) | `crates/burl-sim/src/sim.rs::manual_rotation_turns_the_tree_about_the_camera` |
| Clicking a user or a file selects it; the selected sprite is white; its name is drawn last in `--selection-colour` (default yellow) | `crates/burl/tests/sim_gate.rs::sim_gate_small_repo_select`, `crates/burl-render/tests/scene.rs::selected_sprites_are_white`, `crates/burl-render/tests/names.rs::selected_names_are_drawn_last_in_the_selection_colour_whatever_the_fade_or_hide`, `crates/burl-render/tests/names.rs::the_default_selection_colour_is_yellow` |
| The click lands on what is under the cursor: pick and hover diffed against the oracle, overlapping users pick in name order, and the pointer is physical pixels at any scale factor. A fading-out user is not pickable: the unit test pins it, and `small-repo-pick.pointer` clicks (616,275) at frame 460, near where it hovered and selected bob earlier (about (592,297) at frame 290); at frames 460-470 the oracle dump has bob fading at alpha 0 and `hover_user` and `selected_user` stay null (the click becomes a grab and drag). That the click is within bob's sprite was not checked against the camera projection | `crates/burl-sim/tests/pointer_script.rs::a_fading_user_is_not_pickable`, `crates/burl/tests/sim_gate.rs::sim_gate_small_repo_pick`, `crates/burl-sim/tests/deviations.rs::deviation_d13_overlapping_users_pick_in_name_order`, `crates/burl/src/interactive.rs::the_pointer_is_physical_pixels_whatever_the_scale_factor` |
| Hover shows a user's name, a file's name then its directory, nothing for a selected pawn; the box sits above-left and flips inside the window | `crates/burl-sim/tests/deviations.rs::hover_text_names_the_pawn_under_the_mouse`, `crates/burl-render/src/textbox.rs::the_box_sits_above_left_of_the_cursor_and_flips_inside_the_window` |
| In a real X11 window, real pointer input: Tab selects a user, a held background click deselects, hovering finds a pawn, clicking it selects it and the selected pawn gets no hover box. Read from the `pointer:` debug line, so box placement, text and edge flip are not judged | `scripts/window-in-container.sh::selection: Tab selects a user`, `scripts/window-in-container.sh::selection: a background click deselects`, `scripts/window-in-container.sh::selection: a click on the hovered pawn selects it`, `scripts/window-in-container.sh::selection: no hover box on the selected pawn`, `crates/burl/src/interactive.rs::pointer_line_pins_the_log_format` |
| The cursor hides after 3 s without moving and a move brings it back; "a click then does nothing" is not asserted | `crates/burl-sim/tests/pointer_script.rs::the_cursor_hides_after_three_idle_seconds_and_a_move_brings_it_back` |
| The same X11 window judged on screenshots, for what the state-based row above cannot see (no pointer is drawn in them; selecting and deselecting are that row's, here Tab and the clicks are setup). The scene is paused and left to come to rest, and every target is found in a screenshot (Tab picks a user, a 260 px wide window puts files at both side edges): after a click the user's sprite and the file's sprite are drawn white; an unselected user under the pointer gets a hover box and the selected user and the selected file get none; a file at the left edge gets its box right of the pointer and one at the right edge gets it left of the pointer (flipped); the hover box is gone 3.6 s after the last pointer movement (the sim's idle clock, which stops the hover; the OS cursor itself is never drawn in a screenshot, so whether it hides is not asserted). Only the side of the pointer the box lands on is measured, not above or below, nor its text or colours | `scripts/window-in-container.sh::clicking a user selects it`, `scripts/window-in-container.sh::clicking a file selects it`, `scripts/window-in-container.sh::hovering an unselected user shows a hover box`, `scripts/window-in-container.sh::no hover box over the selected pawn`, `scripts/window-in-container.sh::no hover box over the selected file`, `scripts/window-in-container.sh::hovering a file near the left edge shows a box to the right of the pointer`, `scripts/window-in-container.sh::hovering a file near the right edge flips the box to the left of the pointer`, `scripts/window-in-container.sh::the hover box goes after 3 s without pointer movement` |
| The timeline slider: hit-test, click-to-percent seek, fade in on hover and out after 1 s, layout snapshot; the slider ignores a hidden or grabbed pointer | `crates/burl-render/tests/slider.rs::hit_test_and_percent`, `crates/burl-render/tests/slider.rs::click_seeks_only_on_the_slider`, `crates/burl-render/tests/slider.rs::fade`, `crates/burl-render/tests/slider.rs::slider_matches_snapshot`, `crates/burl-sim/src/sim.rs::slider_peek_and_seek`, `crates/burl/src/interactive.rs::the_slider_ignores_a_hidden_or_grabbed_pointer` |

### Render

| Item | Proof |
|---|---|
| Scene structure matches Gource (per-frame SSIM against the oracle). Its run hides text, date and bloom, so those are not covered | `.github/workflows/heavy.yml::visual-ssim:` |
| `--user-image-dir` and `--default-user-image`: the directory scan, own image then default then stock, `--colour-images` keeps the hash colour, a bad file is reported by name, and the render changes through the real binary | `crates/burl/tests/export.rs::user_image_dir_changes_the_render`, `crates/burl-render/tests/avatars.rs::scan_keeps_jpg_jpeg_png_keyed_by_stem_case_insensitively`, `crates/burl-render/tests/avatars.rs::own_image_then_default_then_stock`, `crates/burl-render/tests/avatars.rs::colour_images_keeps_the_hash_colour`, `crates/burl/src/frame.rs::avatar_hook_and_fatal_bad_file`, `crates/burl-render/tests/avatars.rs::nfc_key_for_a_decomposed_file_name` |

## Still manual (per platform, tick each)

Only what needs a real display, real hardware or a person's eye. Tick on all three platforms
unless a line says otherwise.

- [ ] `--high-dpi` on Retina: line widths and text are crisp, not blurry or doubled (macOS Retina, real panel; the harness checks a golden-free sharpness ratio on virtual displays, not a person's eye on real hardware, and does not check doubled lines)
- [ ] Moving the window between displays of different scale does not break layout (macOS; on real displays; the harness only proves the scale events and capture sizes on virtual ones)
- [ ] Fullscreen toggle and resize keep the scene proportioned: no black bars or stretching (the Linux job checks only that it redraws)
- [ ] Smoke the key glue once in a real window: Space, T, G, R, U, D, F, K, M, `=`/`-`, arrows, Tab, keypad zoom, wheel
- [ ] `burl --user-image-dir DIR <repo>` in a window: each user whose name equals a file name in DIR (extension dropped; on macOS the file name is NFC-normalised first, the author name is not) shows that image (white tint, or the hash colour with `--colour-images`), the others the stock or `--default-user-image` avatar; in a separate run, a corrupt image named for an author who appears during playback exits naming it (an unused corrupt file is never decoded)
- [ ] Smoke the pointer glue once on a real display (Linux X11 also drives it from `window-in-container.sh`, see Mouse above, which cannot judge feel): clicking a user and a file feels right and picks the sprite under the cursor on a Retina display, the hover box follows the cursor and looks right (colour, text, above-left placement, not just the left/right flip), background drag pans, right-drag rotates, and Tab feels right
- [ ] Selection on the real display (#125): Tab cycles users and wraps; a user that has faded out cannot be picked; a still pointer hides after 3 s and a move brings it back; no hover box on the selected pawn; Retina picks land on the sprite under the cursor
- [ ] Cursor grab keeps the cursor from moving with the real mouse during a drag (hidden-ness is checked by `--grab`) and releases it when the last button goes up
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
