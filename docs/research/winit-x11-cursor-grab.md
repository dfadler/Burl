# winit 0.30 X11: how `CursorGrabMode::Confined` / `Locked` work (issue #142)

Part of #136. Version checked: `winit 0.30.13` (Cargo.lock; `Cargo.toml` pins `=0.30.13`).
Primary sources: the published crate source `winit-0.30.13/src/platform_impl/linux/x11/window.rs`
(`set_cursor_grab`, ~L1493-1570) and `src/window.rs` (`CursorGrabMode` docs); the X11 protocol spec
(<https://xorg.freedesktop.org/archive/current/doc/xproto/x11protocol.html>); the winit changelog on
docs.rs (`winit::changelog::v0_30`).

## Answer

- **Confined uses core X11 `GrabPointer` (xcb `grab_pointer`), not XInput2.**
  Call: `owner_events = true`, `grab_window = confine_to = self.xwindow`, async pointer/keyboard
  modes, `cursor = 0`, `time = CURRENT_TIME`, event mask = button press/release, enter/leave,
  pointer motion (+hint), button1-5 motion, keymap state. XInput2 (`xinput::XIEventMask`,
  window.rs ~L492-503) is used only to *select events* (motion, buttons, enter/leave, focus,
  touch), never to grab.
- **Locked is not implemented on X11.** `set_cursor_grab(Locked)` returns
  `ExternalError::NotSupported` immediately (window.rs ~L1495). `src/window.rs` docs agree:
  "X11: Not implemented. Always returns `ExternalError::NotSupported`". No server state.
- Before any grab winit sends `ungrab_pointer(CURRENT_TIME)` ("to prevent passive grabs from
  causing `AlreadyGrabbed`"), releasing its own earlier grab first.
- Status mapping: `SUCCESS` -> Ok; `ALREADY_GRABBED`, `INVALID_TIME`, `NOT_VIEWABLE`, `FROZEN` ->
  `ExternalError::Os` ("Cursor could not be confined: ..."). The `grab_pointer(..).expect(..)` /
  `.reply().expect(..)` calls panic on connection/protocol errors.
- Changelog: the only X11 grab entry is 0.30.8 "On X11, fixed cursor grab mode state tracking on
  error." Nothing changes the X11 grab API; `Locked` was added on Windows only (0.30.10). The raw
  CHANGELOG.md is not in the crate tarball (it points at the docs.rs changelog module).

## Server-side state a second client can observe

(X11 spec, applied to the parameters winit uses.)

- **A second client's `GrabPointer` fails with `AlreadyGrabbed`** while winit's grab is active.
  Most direct, reliable observable.
- **`WarpPointer` cannot leave the confine window.** Spec: it "cannot be used to move the pointer
  outside the confine-to window of an active pointer grab"; it stops at the closest edge. Warp
  outside, then `QueryPointer` (root-relative coordinates) and check clamping to the window edge.
  `xdotool mousemove` is expected to use the warp path, but that was not verified here.
- If the pointer starts outside the confine window the server warps it to the nearest edge as the
  grab activates; reconfiguring the window warps the pointer to keep it inside.
- The server auto-releases the grab if the grab or confine window becomes unviewable
  (unmapped, or moved entirely off the root). winit does not re-grab: `cursor_grabbed_mode` is
  only read/written in `set_cursor_grab` (grep of the 0.30.13 x11 backend). Inference from the
  code, not tested: after an unmap/remap, `set_cursor_grab(Confined)` hits the
  `mode == *grabbed_lock` early return and does nothing; the app must set `None` then `Confined`.

## Implications for the Xvfb probe

1. Probe from a second X client (python-xlib / x11rb / xdotool): `GrabPointer` on the root should
   return `AlreadyGrabbed` while confined and `Success` after `set_cursor_grab(None)`. Release
   immediately on success so the probe does not disturb the app.
2. Or behavioural: warp the pointer outside the window, `QueryPointer` the root, assert it is
   clamped to the window rect. The window must be mapped and viewable. Xvfb has no WM, so geometry
   is whatever the app requested.
3. Do not look for XI2 grab state (`XIGrabDevice`): none is used. Do not probe `Locked` on X11;
   assert `NotSupported`.
4. `Confined` on an unmapped window yields `NOT_VIEWABLE` (Err). Request the grab after first map.
5. Unchecked: whether XTEST fake motion is clamped the same as `XWarpPointer`; verify in the job.
