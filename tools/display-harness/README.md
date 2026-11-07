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
- Moving with System Events `set position` is not a mouse drag; a real cross-display drag stays
  manual.
- The window starts on whichever display macOS picks (often a Retina panel), so the harness moves
  to 1x first and only requires `scale-changed` when the scale actually differs.
