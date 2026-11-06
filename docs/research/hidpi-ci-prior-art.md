# How Chromium, Electron and winit test HiDPI and multi-monitor in CI

Issue: dfadler/Burl#140 (part of #136). Primary sources only, read 2026-11-05.

## Short answer

- **Chromium** is the only one that sets a scale factor on a virtual display. It calls the private `CGVirtualDisplaySettings.hiDPI` and registers a mode at half the pixel size.
- **Electron** `@electron-ci/virtual-display` hard-codes `hiDPI = 0`. It makes 1x displays only.
- **winit** has no virtual-display, multi-monitor or pixel-capture CI. It builds on macOS and runs unit tests.
- Reusable: the private `CGVirtualDisplay*` recipe (Chromium's), Electron's lifecycle quirks, and the observed-event wait pattern. None of them moves a window or captures pixels.

## Chromium: `ui/display/mac/test/virtual_display_util_mac.mm`

Source: tag 138.0.7204.183, `ui/display/mac/test/virtual_display_util_mac.{mm,h}` at
https://chromium.googlesource.com/chromium/src/+/refs/tags/138.0.7204.183/ui/display/mac/test/virtual_display_util_mac.mm
(the Debian mirror URL in the issue returned 404, so the upstream tag was read instead; same version).

- **Private API, hand-declared.** The file declares `CGVirtualDisplay`, `CGVirtualDisplayDescriptor`, `CGVirtualDisplayMode` and `CGVirtualDisplaySettings` as ObjC interfaces "generated from CoreGraphics binaries" (lines 26-104). Nothing is public API.
- **Scale factor: yes.** `CreateVirtualDisplay` (line 115) sets `settings.hiDPI = hiDPI` and registers one mode of `(hiDPI ? width/2 : width) x (hiDPI ? height/2 : height)` at 60 Hz (lines 150-158). So the caller's `resolution` is the backing pixel size, and hiDPI gives scale 2.
- **How hiDPI is chosen.** `AddDisplay` passes `hiDPI = display_params.dpi.x() >= 200` (line 404). The presets are keyed by DPI (for example `k3840x2160` is 200 dpi, `k1920x1080` is 125 dpi), so 2x or 1x falls out of the DPI value.
- **Descriptor quirks for macOS 14** (lines 133-139): a unique non-zero `serialNum`/`serialNumber` per display and a non-zero `vendorID` (505). The physical size is derived from `ppi`: `25.4 * px / ppi` mm.
- **Wait for the system, not a sleep.** After create it waits for `display::Screen` to report the display, then calls `CGDisplaySetDisplayMode` (synchronous) and waits for a metrics-changed event covering bounds, work area and `DEVICE_SCALE_FACTOR` (lines 256-298).
- **Removal flake workaround.** The first removal is flaky with timeouts, so it adds a throwaway second display and removes both together (lines 331-351, crbug 40148077).
- **Sleep blocker.** `IOPMAssertionCreateWithName(kIOPMAssertionTypeNoDisplaySleep)` is held while displays exist (lines 501-511).
- **Headless is unsupported.** `IsAPIAvailable()` returns `!IsRunningHeadless()`; headless means every online display's model number is `'virt'` (lines 169-202, 436-440, with a `TODO ... Support headless bots`). In practice the harness needs at least one real or otherwise non-virtual display online. Whether GitHub-hosted macOS runners count as having one is **not verified here**.

## Electron: `@electron-ci/virtual-display`

Sources: https://electronjs.org/docs/latest/development/multi-monitor-testing and
`electron/electron@main` (0de2430) `spec/fixtures/native-addon/virtual-display/` (`src/VirtualDisplayBridge.m`, `src/addon.mm`, `lib/virtual-display.js`, `package.json`).

- **API** (docs): `create({width,height,x,y})` returns an id (0 on failure, default 1920x1080 at 0,0), `destroy(id)`, `forceCleanup()`. macOS only, built with node-gyp and node-addon-api.
- **Scale factor: no.** `VirtualDisplayBridge.m` sets `settings.hiDPI = 0`. The docs do not mention scale. Its modes are multiples of the requested size (`width * mult * step`), which are pixel sizes at scale 1.
- **Positioning:** a `positionDisplay:x:y:` step exists. The docs warn macOS keeps the desktop contiguous, so requested origins may be adjusted.
- **Limits and flake advice** (docs): 720x720 to 8192x8192 sizes; rapid create/destroy can corrupt the CoreGraphics display ID pool; call `forceCleanup()` in `beforeEach` and `destroy()` after use. The source's `forceCleanup` runs a permanent then session `CGDisplayConfiguration` reset with a 2 s sleep.
- It is an addon for Node and Electron specs. It has no pixel capture and no window-move logic.

## winit: CI

Sources: `rust-windowing/winit@master` (7ed7c20) and tag `v0.30.12`, `.github/workflows/ci.yml`.

- The matrix builds and runs `cargo test` on `macos-latest` (x86_64, aarch64, and a private-APIs feature leg), plus Windows, Linux, Android, iOS and Web. Test steps are `cargo test` on crates (`dpi` is tested on Linux only).
- No `xvfb`, virtual display, screenshot, or scale-factor step appears in either version. The tests in the tree are `winit/tests/{send_objects,serde_objects,sync_object}.rs`, the `dpi` crate's unit tests, and an X11 `xsettings.dat` fixture. HiDPI and multi-monitor behaviour is not exercised in CI.
- Nothing to reuse for the harness. (Absence claim: based on grep of the two workflow files and the master file tree, not on every workflow in the org.)

## What we can reuse for the Burl harness

1. Chromium's descriptor + settings + mode recipe, as our own thin shim: unique serial and non-zero vendor ID per display, `hiDPI = 1` with a mode at half the pixel size for the 2x display, `hiDPI = 0` full-size mode for the 1x one.
2. Chromium's event-driven waits (display added, mode applied) instead of sleeps. For us the equivalent signal is winit's `ScaleFactorChanged` / `NSScreen` change.
3. Electron's lifecycle lessons: reset display state between tests, and expect origin adjustments, so verify positions after creation instead of trusting the requested ones.
4. Not provided by any of the three: moving a winit window between displays, and capturing pixels. Those stay ours (capture API choice not researched here).

## Open risks

- The `CGVirtualDisplay` API is private and undocumented; both implementations depend on reverse-engineered headers and OS-version quirks (macOS 14 serial/vendor rules).
- Whether a hosted macOS CI runner has a display session suitable for this (Chromium treats virtual-only as headless) needs a spike.
