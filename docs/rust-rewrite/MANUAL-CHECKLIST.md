# Manual interactive checklist (parity release)

Skeleton for ticket #20 item 5. **Unsigned**: the user fills in and signs. Rows derive from `Gource::keyPress`, `mouseMove`, `mouseClick`, `mouseWheel`, `updateCamera` (src/gource.cpp) and the display setup. Run each row against the C++ gource 0.57 where behaviour is in doubt. Platforms: macOS Retina, macOS non-Retina, Linux X11 (release gates); Wayland best-effort.

Command: `cargo run --release -p burl -- <repo>` (add `-800x600` for the exact-pixel rows).

| # | Row | Expect | mac Retina | mac non-Retina | Linux X11 |
|---|-----|--------|:---:|:---:|:---:|
| W1 | Window opens, scene animates | tree grows, users, beams, bloom | | | |
| W2 | Close button / Esc | exits 0 (Esc needs input wiring) | | | |
| W3 | Resize | keeps drawing, no crash | | | |
| W4 | `-800x600` without `--high-dpi` | drawable exactly 800x600 pixels (small on Retina: recorded Gap) | | | |
| W5 | No viewport / `--high-dpi` | sharp, logical size scaled by DPI | | | |
| W6 | `-f`, `--frameless`, `--window-position` | as flags | | | |
| W7 | `--no-vsync` | no refresh lock | | | |
| K1 | `=`/`+`, `-` | days/second steps (0.1,0.2..1,2..30) | | | |
| K2 | `.` `,` `/` | time scale 0.25..4, `/` resets | | | |
| K3 | Wheel, keypad `+`/`-` | zoom x1.1 per step, clamped | | | |
| K4 | Space | pause | | | |
| K5 | Arrows | pan camera | | | |
| K6 | Left drag / right drag | pan / rotate tree | | | |
| K7 | Middle click, `v` | toggle camera mode | | | |
| K8 | `t g u d f r k m c s z [ ] n p q w y` | each toggle as in gource.cpp keyPress | | | |
| K9 | Tab, click user/file | selection, tracking | | | |
| K10 | Timeline slider click | seeks | | | |
| K11 | F12 | screenshot | | | |

Rows K4-K11 are not implemented yet; mark N/A until their PRs land.

Signed: ______________  Date: ________  Burl commit: ________
