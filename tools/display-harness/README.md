# display-harness

Checks Burl's scale-factor handling on macOS: it creates a 2x and a 1x virtual display, runs the
real `burl` with `--log-level debug`, moves its window onto each display and asserts the
`display:` log lines and the captured pixel sizes. Part of map #147; pixel comparison (SSIM) and
the CI workflow are separate tickets.

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
