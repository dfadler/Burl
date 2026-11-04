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
- [ ] Clicking a user selects it: white sprite, larger name in `--selection-colour` drawn over everything; Tab moves to the next user and wraps
- [ ] Clicking a file selects it the same way (its name sits a little lower); clicking empty background deselects
- [ ] The click lands on what is under the cursor, also on Retina (pointer and drawable are both physical pixels in Burl: Gource mixes points and pixels with `--high-dpi`); a user that is fading out cannot be picked
- [ ] The cursor stays hidden until it moves; after 3 s without moving, hover ends (a click then does nothing until the mouse moves)
- [ ] The timeline slider seeks on click and shows its hover caption
- [ ] Cursor grab starts on drag and releases when the last button is up

## Render

- [ ] Bloom, shadows, file and user textures, beams and the key legend look like Gource's at the same settings
- [ ] Text (user, file, directory names, captions, title, date) is readable and positioned like Gource's
- [ ] Pause/resume and seeking leave no stale frame or flicker

## Sign-off

Replace each `TODO`. `deviation-register` is `git hash-object docs/rust-rewrite/DEVIATIONS.md`
after the user has reviewed every row; any later edit to the register invalidates it.

version: TODO
deviation-register: TODO
macos-retina: TODO
macos-non-retina: TODO
linux-x11: TODO
