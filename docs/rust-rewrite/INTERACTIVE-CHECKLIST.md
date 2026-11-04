# Interactive checklist (parity release sign-off)

Manual checks for the windowed path that no gate automates (#20, #14 item 5). The user runs
them on each platform below and fills the sign-off block at the end **in the tagged commit**.
`scripts/check-release.sh` (run by `release.yml`) fails while any box is unticked, any
platform line is `TODO`, or `DEVIATIONS.md` changed after the recorded review.

Run each section on: **macOS Retina**, **macOS non-Retina** (external display), **Linux X11**.
Wayland is best-effort, not a gate. Bindings are the app-core table
(`discovery/app-core.md` T2.2); compare against `gource` 0.57 side by side where unsure.

Use any real log, e.g. `burl --seconds-per-day 0.5 <repo>`.

## Window and HiDPI

- [ ] Default window opens; a viewport given without `--high-dpi` opens a window whose drawable matches (G3 gap: small on Retina is expected)
- [ ] `--high-dpi` on Retina: line widths and text are crisp, not blurry or doubled
- [ ] Fullscreen toggle and resize keep the scene proportioned; no black bars or stretching
- [ ] Moving the window between displays of different scale does not break layout

## Keyboard (each toggles/behaves as in Gource)

- [ ] Esc quits; F12 writes a screenshot
- [ ] Space pauses; `=`/`+` and `-` change days per second; `.` `,` `/` change time scale
- [ ] T, G, U, D, F, R, K toggle tree, users, user names, dir names, file names, root, key legend
- [ ] V toggles camera mode; Tab selects the next user; arrows pan (also while paused)
- [ ] Z gravity toggle, `[` `]` force gravity, S recolour, N skips idle time, C shows the splash, M hides the cursor
- [ ] Keypad `+`/`-` zoom

## Mouse

- [ ] Wheel zooms in/out and respects the zoom clamp
- [ ] Left-drag on the background pans; clicking empty background deselects the user
- [ ] Right-drag rotates the camera; middle click toggles camera mode
- [ ] Hover shows names; clicking a user selects it; the timeline slider (bottom strip, fades in on hover, out after 1 s) seeks on click, shows its date caption over the cursor clamped to the window, and is absent with `--hide-progress` or a stdin log
- [ ] Cursor grab starts on drag and releases when the last button is up

## Render

- [ ] Bloom, shadows, file and user textures, beams and the key legend look like Gource's at the same settings
- [ ] Text (user, file, directory names, captions, title, date) is readable and positioned like Gource's
- [ ] Pause/resume and seeking leave no stale frame or flicker
- [ ] `burl --user-image-dir DIR <repo>` (DIR holds `<user name>.png/.jpg`) shows each user as their own image, white unless `--colour-images`; `--default-user-image F` gives the rest F; a corrupt file in DIR exits with an error naming it (#91)

## Sign-off

Replace each `TODO`. `deviation-register` is `git hash-object docs/rust-rewrite/DEVIATIONS.md`
after the user has reviewed every row; any later edit to the register invalidates it.

version: TODO
deviation-register: TODO
macos-retina: TODO
macos-non-retina: TODO
linux-x11: TODO
