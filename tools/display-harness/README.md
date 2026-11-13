# display-harness

Checks Burl's scale-factor handling on macOS: it creates a 2x and a 1x virtual display, runs the
real `burl` with `--log-level debug`, moves its window onto each display and asserts the
`display:` log lines and the captured pixel sizes, then compares the captured pixels. Part of map
#147.

```
cargo xtask display-harness
```

It builds the release `burl`, compiles `vd.m` and `wid.m` into `target/display-harness/`, and
leaves the logs and captures (`vd.out`, `burl.out`, `1x.png`, `2x.png`) there. Exit 1 with one
message per failed step or assertion. Not part of `cargo xtask ci`; other OSes print a message
and exit 1.

Assertions, per display: the new `display:` lines carry the display's scale with
logical = physical / scale, and a scale change shows `scale-changed` then `resized`; the
`screencapture` PNG is scale x the window's point size and has several distinct colours.

## Pixel comparison (#155)

- **Same frame:** burl's sim is time dependent, so the harness presses Space once (System Events
  key, read back from the pixels: the camera keeps settling for 10 to 45 s, then the frame stops
  changing) and every capture after that, on either display, is the same frame.
- **Same physical size, not same point size:** Burl lays out in physical pixels (fixed pixel font
  size, camera zoom), so a 320x180 pt window on the 2x display and a 640x360 pt window on the 1x
  display both show 640x360 px of the same frame, while a 320x180 pt window on 1x is framed
  differently. The harness resizes to exactly those, crops the title bar, and compares.
- **Primary check, no stored image:** the 2x capture must equal the 1x capture (MAE <= 0.25; real
  runs 0.04 and 0.05, the residue is the rounded window corners). A negative control built from
  the 1x capture (downscale by 2, upscale back) must be at least 2x worse than that limit (real
  runs 0.8 to 1.9), or the check fails as vacuous. Downscaling the 2x capture and comparing it
  to a 1x capture of the same point size was tried and rejected: it passes a window that
  was rendered at 1x and upscaled.
- **Secondary:** `baselines/` holds regression baselines from the runner, see its README.

## `--high-dpi` scenario (#162)

```
cargo xtask display-harness --high-dpi
```

Same flow, but burl gets `--high-dpi`, so `-640x360` is a logical-size window and font scale
follows the DPI ratio; artifacts go to `target/display-harness-high-dpi/`. Measured on macOS 26
(Apple Silicon): the window is 640x360 pt on both displays; physical 640x360 on 1x and 1280x720 on
2x; `display:` lines say logical 640x360 on both; the 2x capture is 1280x784 (title bar included).

- **Asserted:** the `display:` scale and logical = physical / scale (as above), logical size and
  window points equal 640x360 on both displays, physical = scale x logical, capture = scale x
  points and not blank.
- **No 1x/2x pixel comparison:** the 2x frame is 1280x720 px of a camera framed in physical pixels
  with font scale 2, so it is a different picture from the 1x one.
- **Crispness, golden-free:** `crispness` = how much finer-than-2px detail the capture has
  relative to its own box-downscaled-and-restretched copy. Genuine captures score 2.3 locally and
  2.9 to 3.3 on the hosted runners (limit 1.8, `MIN_CRISPNESS`); the blurred copy 1.4 locally and
  1.5 to 1.6 hosted; the 1x capture bilinear-stretched 1.28 (local); nearest-stretched 0. The synthetic blurry control (the blurred copy) is built from the real
  capture each run and printed beside the score; feeding the blurred copy to the check makes the
  real harness fail (1.43 < 1.8), as do the unit tests. Not reproduced: a real compositor upscale of a 1x render (its
  filter may differ from bilinear), and doubled lines.


## Cursor grab, `--grab`

```
cargo xtask display-harness --grab
```

No virtual display needed. Opens burl with the tree, files, users and slider hidden (so a click
lands on the background, which grabs) and **without** `--hide mouse` (that drops button presses).
`mouse.m` posts a real left press and release with `CGEventPost`, and the check reads burl's
`grab:` debug lines. Three phases:

1. **Control:** motion with no button down logs no grab, so the `on` below is the press's.
2. **Press:** burl logs `grab: event=on mode=locked` and keeps it through drag motion. winit's macOS
   `Locked` is `CGAssociateMouseAndMouseCursorPosition(false)` and `Confined` is unsupported there,
   so `locked` also means the OS call returned `Ok`.
3. **Release:** burl logs `grab: event=off`; the whole cycle must be on, locked, off.

**Not asserted: where the cursor goes.** The first hosted runs (`macos-15`, `macos-26`) showed the
cursor following 50 points of synthetic drag while burl held the lock (`(412, 390) -> (462, 390)`),
so `CGEventPost` bypasses the detach and a position check cannot tell a lock from none. Also not
covered: that the cursor is hidden (`set_cursor_visible(false)` has no getter) and a real device's
relative motion, so the checklist's cursor-grab box stays manual.

## Real window drag, `--drag`

```
cargo xtask display-harness --drag
```

The real cross-display drag the earlier text said stayed manual. Needs the virtual displays.
Burl is moved to the 1x display with System Events once, then `mouse.m` does what a user does:
`warp` to the title bar (window origin + half the width across, 12 pt down), `down`, 40 `drag`
events stepping in a straight line to the centre of the other display (global bounds from `vd`,
so a negative x is fine), `up`. Then 2x back to 1x. Artifacts in `target/display-harness-drag/`.

- **Asserted, per crossing:** burl's `display:` lines after the drag pass `check_move` (target
  scale, logical = physical / scale, `scale-changed` then `resized`), and `wid` shows the window's
  origin on the target display.
- **Asserted, per crossing, once the window has settled on the target display** (`check_size_kept`,
  `check_capture`): the window's point size (`wid` frame) is the same as before the drag; burl's last
  `display:` logical size is the same as before the drag; a `screencapture -o -l` of the window is
  scale x its point size and has several distinct colours (not blank). This is burl without
  `--high-dpi` (exact-pixel `-640x360`); the logical-size-kept assertion is the expected winit
  behaviour, not a measurement: if a hosted run shows burl's logical size changing on the 2x display
  (deviation G3), record the measured numbers here and relax the assertion.
- **Not asserted:** the cursor position (synthetic events carry absolute locations, see `--grab`),
  layout by eye (overlap, clipping, text size), crispness, doubled lines, and pixel equality
  between a dragged capture and a `set position` one (the sim is not paused in `--drag`, so frames
  differ; reusing the Space-pause loop would mean refactoring `run_harness`).
- **Unverified until a hosted run:** that a `CGEventPost` title-bar drag across the virtual display
  boundary moves the window and makes winit fire `ScaleFactorChanged`. The path is a straight line
  between display centres, so it assumes the displays touch; a gap or offset may clamp the pointer.

## Needs

- macOS with a logged-in GUI session, `clang`, and a Rust toolchain.
- Screen Recording and Accessibility granted to the terminal/app that runs it (`screencapture`,
  `osascript` System Events). Hosted `macos-15` / `macos-26` runners already have both.
- It briefly adds two displays and opens and moves a burl window on your desktop.

## Files

- `vd.m`: creates the displays, pumps AppKit events, prints each display's global bounds, scale
  and pixels, then `ready`; runs until SIGTERM/SIGINT or its parent exits.
- `wid.m`: prints `id x y w h` of burl's on-screen windows taller than 100 points.

## Caveats

- `CGVirtualDisplay` is private API, declared by hand in `vd.m`; a macOS update can break it.
  macOS 14+ needs a unique non-zero vendor/serial per display.
- NSScreen never lists the displays unless AppKit events are pumped (`nextEventMatchingMask`).
- `hiDPI` advertises the 2x mode; `vd.m` still switches into 1920x1080 pt / 3840x2160 px with
  `CGConfigureDisplayWithDisplayMode`. The 1x display otherwise defaults to 1024x576, so it is
  switched to 1280x720 the same way.
- Display positions are read from the helper, not hard-coded (the 1x display can sit at a
  negative x).
- Moving with System Events `set position` is not a mouse drag; `--drag` is the real one.
- The window starts on whichever display macOS picks (often a Retina panel), so the harness moves
  to 1x first and only requires `scale-changed` when the scale actually differs.
