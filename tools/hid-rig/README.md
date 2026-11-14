# HID rig: a real mouse for the macOS cursor-grab check (#211)

`cargo xtask display-harness --grab-device [--port PATH]` proves what hosted runners cannot (#205):
that a **real device's relative motion** does not move the cursor while burl holds a `Locked` grab.
Synthetic `CGEventPost` events carry absolute locations and bypass the lock, delta-only events do
not move the cursor at all, and a virtual HID device cannot be created from an ad-hoc signed
binary. So the input has to come from a device the OS sees as hardware.

**Status: the scenario has never run.** The protocol, the verdict and the line transport are unit
tested (`cargo test -p xtask hid_rig`); the firmware, the serial setup, the Karabiner route and the
scenario itself are UNRUN until a rig exists. Expect a first-run fix or two.

## What the scenario does

1. Starts burl as `--grab` does and parks the cursor on its background (`mouse warp`, the only
   synthetic input).
2. **Control, no grab:** the device sends 100 counts right; the cursor must move at least 20 pt the
   same way. If not, the device does not reach the system and the check fails as such.
3. **Press and hold** the device's left button on the background: burl logs `grab: event=on
   mode=locked`. Send the same 100 counts; the cursor must move at most 3 pt.
4. Release; burl logs `grab: event=off`.

No backend found: it prints `SKIPPED (not a pass)` and **exits 0**. A skip is not evidence; the
checklist box stays open until a run prints `ok`. A device you named (`--port`, or
`BURL_VHID_HELPER`) that does not answer is a failure (exit 1), not a skip.

## Route 1: Pico rig (any Mac, a self-hosted runner)

Bill of materials (about 10 USD):

- 1x Raspberry Pi Pico (RP2040; a Pico W or a clone also works), headers optional.
- 1x USB **data** cable (not charge-only) from the Pico's micro-USB to the Mac (USB-C to micro-B,
  or an adapter). The one connection carries both the mouse and the serial port.
- Nothing else; no wiring.

Flash (Arduino IDE or `arduino-cli`):

1. Install the **arduino-pico** core (Earle Philhower), board "Raspberry Pi Pico".
2. `Tools > USB Stack` = **Pico SDK** (the Adafruit TinyUSB stack has no `Mouse` library). Docs:
   <https://arduino-pico.readthedocs.io/en/latest/usb.html>. With the Pico SDK stack `Serial` and
   `Mouse` run together as one composite device.
3. Open `hid-rig.ino`, upload (hold BOOTSEL while plugging in the first time).

Check: `ls /dev/cu.usbmodem*` shows a port; `screen /dev/cu.usbmodem* 115200`, then typing `ping`
and Enter, answers `ok burl-hid-rig 1` (quit screen with Ctrl-A K). If macOS blocks the new mouse,
the control phase fails and says so.

### Protocol

One command per line, `\n` terminated. Replies `ok [info]` or `err MSG`.

| command | effect |
|---|---|
| `ping` | `ok burl-hid-rig 1` (how the host recognises the rig) |
| `rel DX DY` | relative move, each part -127..127 counts, no wheel |
| `down`, `up` | left button press, release |

Host code: `xtask/src/hid_rig.rs` (`encode`, `parse`, `parse_reply`; `parse` is the spec of the
sketch's parser). The serial port is opened raw at 115200 with a 2 s read timeout via `stty -f`.

### Safety

- The rig moves the **real cursor** of the Mac it is plugged into. Run it on a Mac nobody is
  using, or accept that the cursor jumps (the harness warps it to burl's window, then the rig
  pushes it 100 counts). Unplug the Pico to stop it instantly.
- The sketch releases a held button after 5 s without a command, so a crashed run cannot leave the
  button stuck down.
- It sends only motion and the left button. It cannot type (no keyboard descriptor).
- Treat a serial port as untrusted input only if something else can write to it; this one takes
  numbers and three words.

### Self-hosted runner

On a Mac mini runner (not a hosted runner: those have no USB):

- Plug the Pico in permanently; log the runner in to a GUI session (the harness needs a window
  server and Accessibility for `CGEventPost`/`osascript`, as `--grab` does).
- A job on the self-hosted label runs `cargo xtask display-harness --grab-device`. With the rig
  unplugged it exits 0 as `SKIPPED`, so make the job fail on a missing `ok` line, not on the exit code:
  `... 2>&1 | tee grab-device.log; grep -q '^ok ' grab-device.log`.
- No workflow is committed for this; add one when the runner exists (workflow files are reviewed
  by a human).

## Route 2: Karabiner-DriverKit-VirtualHIDDevice (no hardware, a LOCAL Mac only)

Evaluated from its documentation, **not built or run**:

- Repo and README: <https://github.com/pqrs-org/Karabiner-DriverKit-VirtualHIDDevice>.
- Install the `.pkg`, then activate the system extension with
  `/Applications/.Karabiner-VirtualHIDDevice-Manager.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Manager activate`
  and **approve the prompt in System Settings** (a one-time user action; a restart may be
  needed). This cannot be done unattended, so it cannot run on a hosted runner and is a poor fit
  for an unattended self-hosted one.
- A **root** `Karabiner-VirtualHIDDevice-Daemon` must be running
  (`/Library/Application Support/org.pqrs/Karabiner-DriverKit-VirtualHIDDevice/Applications/Karabiner-VirtualHIDDevice-Daemon.app/Contents/MacOS/`).
- The client is a **header-only C++23 library** over a root-only Unix socket
  (`/Library/Application Support/org.pqrs/tmp/rootonly/karabiner_virtual_hid_device_service.sock`);
  the devices accept commands only from root processes. There is **no CLI that sends pointing
  input**. The example `examples/virtual-hid-device-service-client/src/main.cpp` initialises
  with `async_virtual_hid_pointing_initialize()`, waits for `virtual_hid_pointing_ready`, and posts
  `virtual_hid_device_driver::hid_report::pointing_input` (fields `x`, `y`, and buttons/wheels
  whose exact names this evaluation did not confirm) with `async_post_report`.
- The report struct is a HID report, so the events reach the real HID stack (IOHIDFamily), which
  is the property the check needs. Whether a Karabiner device counts as "real" for the `Locked`
  grab was not measured.

How it plugs in: `BURL_VHID_HELPER=/path/to/helper` makes the backend spawn that executable and
speak the same line protocol on stdin/stdout. **The helper is not written**: it is a small C++
program (read lines, call `async_post_report`) built against the Karabiner headers and run as
root, for example through a `sudo` wrapper script on the owner's Mac. Write it once the Pico route
shows whether the check is worth a second backend.

## Why not other routes

- A virtual HID device from our own binary: needs the `com.apple.developer.hid.virtual.device`
  entitlement; AMFI kills an ad-hoc signed binary that claims it (#205, `hid.m`).
- `CGEventPost` with absolute or delta-only events: measured in #205, see
  `tools/display-harness/README.md`.
