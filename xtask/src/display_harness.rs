//! `cargo xtask display-harness` (ticket #152): creates a 2x and a 1x virtual display with
//! `tools/display-harness/vd.m`, runs the real `burl`, moves its window onto each display and
//! asserts the `display:` debug lines and the captured pixel sizes. macOS only; not part of
//! `cargo xtask ci`. See `tools/display-harness/README.md`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::pixels;

// ---- pure parsing and checks (unit-tested below) ----

/// One `display:` debug line from burl (`interactive::display_line`).
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayEvent {
    pub event: String,
    pub scale: f64,
    pub physical: (u32, u32),
    pub logical: (u32, u32),
}

fn size(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// The `display: event=.. scale=.. physical=WxH logical=WxH` line anywhere in `line`.
pub fn parse_display_line(line: &str) -> Option<DisplayEvent> {
    let rest = &line[line.find("display: event=")? + "display: ".len()..];
    let field = |k: &str| rest.split_whitespace().find_map(|t| t.strip_prefix(k));
    Some(DisplayEvent {
        event: field("event=")?.to_string(),
        scale: field("scale=")?.parse().ok()?,
        physical: size(field("physical=")?)?,
        logical: size(field("logical=")?)?,
    })
}

pub fn parse_display_events(text: &str) -> Vec<DisplayEvent> {
    text.lines().filter_map(parse_display_line).collect()
}

/// A display the helper reported: `display name=2x id=.. x=.. y=.. w=.. h=.. scale=.. pixels=WxH`.
#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub scale: f64,
    pub pixels: (u32, u32),
}

pub fn parse_display(line: &str) -> Option<Display> {
    let rest = line.strip_prefix("display ")?;
    let f = |k: &str| rest.split_whitespace().find_map(|t| t.strip_prefix(k));
    Some(Display {
        name: f("name=")?.to_string(),
        x: f("x=")?.parse().ok()?,
        y: f("y=")?.parse().ok()?,
        w: f("w=")?.parse().ok()?,
        h: f("h=")?.parse().ok()?,
        scale: f("scale=")?.parse().ok()?,
        pixels: size(f("pixels=")?)?,
    })
}

/// A window the finder printed: `<id> <x> <y> <w> <h>` in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub fn parse_window(line: &str) -> Option<Window> {
    let mut it = line.split_whitespace();
    let mut n = || it.next()?.parse::<i32>().ok();
    Some(Window {
        id: n()? as u32,
        x: n()?,
        y: n()?,
        w: n()? as u32,
        h: n()? as u32,
    })
}

/// After moving onto a display of scale `want` (the window was on one of scale `before`): the new
/// `display:` lines must carry that scale with logical == physical / scale, and a scale change
/// must report `scale-changed` followed by `resized`.
pub fn check_move(new: &[DisplayEvent], before: f64, want: f64) -> Result<(), String> {
    if let Some(e) = new.iter().find(|e| e.scale != want && e.event != "resized") {
        return Err(format!("expected scale {want}, got {e:?}"));
    }
    for e in new {
        let l = |p: u32| (f64::from(p) / e.scale).round() as u32;
        if e.logical != (l(e.physical.0), l(e.physical.1)) {
            return Err(format!("logical != physical / scale in {e:?}"));
        }
    }
    if before != want {
        let at = new
            .iter()
            .position(|e| e.event == "scale-changed" && e.scale == want)
            .ok_or(format!(
                "no scale-changed scale={want} line (scale was {before}); got {new:?}"
            ))?;
        if !new[at..]
            .iter()
            .any(|e| e.event == "resized" && e.scale == want)
        {
            return Err(format!(
                "no resized scale={want} line after scale-changed; got {new:?}"
            ));
        }
    }
    Ok(())
}

/// A capture must be `scale` times the window's point size and not blank.
pub fn check_capture(
    pixels: (u32, u32),
    points: (u32, u32),
    scale: f64,
    colours: usize,
) -> Result<(), String> {
    let want = (
        (f64::from(points.0) * scale).round() as u32,
        (f64::from(points.1) * scale).round() as u32,
    );
    if pixels != want {
        return Err(format!(
            "capture is {}x{} px, expected {}x{} ({}x{} pt at scale {scale})",
            pixels.0, pixels.1, want.0, want.1, points.0, points.1
        ));
    }
    if colours < 4 {
        return Err(format!("capture looks blank ({colours} distinct colours)"));
    }
    Ok(())
}

/// What one display showed for a `--high-dpi` window, after the window settled there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HighDpiShot {
    /// The last `display:` line's scale, physical and logical size.
    pub scale: f64,
    pub physical: (u32, u32),
    pub logical: (u32, u32),
    /// The window frame in points, from the window list: the OS's own measurement, independent of
    /// the `display:` lines (winit's content size). Frame minus logical height is the title bar.
    pub frame: (u32, u32),
}

/// `--high-dpi` with a `-WxH` viewport is a logical-size window: `want` is that viewport. Both
/// displays must report it as the logical size and the window must be that many points, while
/// the physical size is `scale` times it (so it doubles on the 2x display).
pub fn check_high_dpi(
    one: &HighDpiShot,
    two: &HighDpiShot,
    want: (u32, u32),
) -> Result<(), String> {
    for (name, s, scale) in [("1x", one, 1.0), ("2x", two, 2.0)] {
        let phys = |l: u32| (f64::from(l) * scale).round() as u32;
        if s.scale != scale {
            return Err(format!("{name} display: scale {} is not {scale}", s.scale));
        }
        // Width: the frame is as wide as the content, so the OS and winit must agree. Height is
        // checked across the two displays below (the title bar is the difference).
        if s.logical != want || s.frame.0 != want.0 {
            return Err(format!(
                "{name} display: logical {:?} / window frame {:?} pt, expected {want:?} wide on both \
                 displays: not a logical-size window (exact-pixel, as without --high-dpi?)",
                s.logical, s.frame
            ));
        }
        if s.physical != (phys(want.0), phys(want.1)) {
            return Err(format!(
                "{name} display: physical {:?} is not {scale} x {want:?}",
                s.physical
            ));
        }
    }
    // The OS frame height minus winit's content height is the title bar. It must be the same on
    // both displays (and exist): a window mis-sized on one display would show a different bar.
    let bar = |s: &HighDpiShot| s.frame.1.checked_sub(s.logical.1).filter(|b| *b > 0);
    match (bar(one), bar(two)) {
        (Some(a), Some(b)) if a == b => Ok(()),
        (a, b) => Err(format!(
            "window frame height minus content height (the title bar) is {a:?} pt on 1x and {b:?} pt on 2x: \
             frames {:?} / {:?}, content {:?} / {:?}; expected the same non-zero bar on both",
            one.frame, two.frame, one.logical, two.logical
        )),
    }
}

// ---- process plumbing ----

/// Kills and reaps its child on drop, so a failure or panic never leaves burl or the helper behind.
struct Proc(Child);
impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn poll<T>(what: &str, secs: u64, mut f: impl FnMut() -> Option<T>) -> Result<T, String> {
    let end = Instant::now() + Duration::from_secs(secs);
    loop {
        if let Some(v) = f() {
            return Ok(v);
        }
        if Instant::now() > end {
            return Err(format!("timed out after {secs}s waiting for {what}"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn run(cmd: &mut Command) -> Result<String, String> {
    let out = cmd.output().map_err(|e| format!("{cmd:?}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{cmd:?}: {}\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

fn distinct_colours(png_path: &Path) -> Result<((u32, u32), usize), String> {
    let bytes = std::fs::read(png_path).map_err(|e| format!("{}: {e}", png_path.display()))?;
    let img = crate::ssim::decode(&bytes)?;
    let mut seen = HashSet::new();
    for px in img.rgb.as_chunks::<3>().0 {
        seen.insert(*px);
        if seen.len() >= 16 {
            break;
        }
    }
    Ok(((img.w as u32, img.h as u32), seen.len()))
}

/// Runs the whole harness; `Err` is the message for the first failed step or assertion.
pub fn run_harness(root: &Path, high_dpi: bool) -> Result<(), String> {
    let dir = root.join(if high_dpi {
        "target/display-harness-high-dpi"
    } else {
        "target/display-harness"
    });
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let src = root.join("tools/display-harness");
    for (m, bin, fw) in [
        ("vd.m", "vd", "CoreGraphics"),
        ("wid.m", "wid", "CoreGraphics"),
    ] {
        run(Command::new("clang")
            .args(["-fobjc-arc", "-framework", "Cocoa", "-framework", fw])
            .arg(src.join(m))
            .arg("-o")
            .arg(dir.join(bin)))?;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    eprintln!("+ cargo build --release -p burl");
    run(Command::new(&cargo)
        .args(["build", "--release", "-p", "burl"])
        .current_dir(root))?;
    let target =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let burl_bin = target.join("release/burl");

    // The helper first: its `ready` line comes after both displays are listed.
    let vd_out = dir.join("vd.out");
    let vd_file = std::fs::File::create(&vd_out).map_err(|e| e.to_string())?;
    let mut vd = Proc(
        Command::new(dir.join("vd"))
            .stdout(vd_file)
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("vd: {e}"))?,
    );
    // Fail fast if the helper exits first (e.g. it printed "could not create the virtual
    // displays" because a leftover `vd` still holds them), instead of waiting out the timeout.
    let text = poll("the helper's `ready` line", 60, || {
        let t = read(&vd_out);
        if t.lines().any(|l| l == "ready") {
            return Some(Ok(t));
        }
        let st = vd.0.try_wait().ok().flatten()?;
        Some(Err(format!(
            "the display helper exited ({st}) before printing `ready` (its message is above; \
             a leftover `vd` process may still hold the virtual displays)"
        )))
    })??;
    let displays: Vec<Display> = text.lines().filter_map(parse_display).collect();
    let find = |name: &str| {
        displays
            .iter()
            .find(|d| d.name == name)
            .cloned()
            .ok_or(format!("helper reported no {name} display"))
    };
    let (d2, d1) = (find("2x")?, find("1x")?);
    eprintln!("displays: {d2:?} {d1:?}");

    let burl_out = dir.join("burl.out");
    let log = root.join("tests/golden/sim/small-repo.log");
    let out = std::fs::File::create(&burl_out).map_err(|e| e.to_string())?;
    let _burl = Proc(
        Command::new(&burl_bin)
            .arg(&log)
            .args(if high_dpi { &["--high-dpi"][..] } else { &[] })
            .args([
                "--log-format",
                "custom",
                "-640x360",
                "--seconds-per-day",
                "10",
                "--hash-seed",
                "31",
            ])
            .args([
                "--disable-auto-skip",
                "--disable-auto-rotate",
                "--file-idle-time",
                "0",
            ])
            .args([
                "--time-scale",
                "1",
                "--hide",
                "mouse",
                "--prng",
                "glibc",
                "--log-level",
                "debug",
            ])
            .stdout(out.try_clone().map_err(|e| e.to_string())?)
            .stderr(out)
            .spawn()
            .map_err(|e| format!("{}: {e}", burl_bin.display()))?,
    );
    let window = || -> Option<Window> {
        let o = Command::new(dir.join("wid")).arg("burl").output().ok()?;
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .find_map(parse_window)
    };
    poll("burl's window", 60, window)?;
    let mut scale = poll("burl's `display: event=start` line", 30, || {
        parse_display_events(&read(&burl_out))
            .first()
            .map(|e| e.scale)
    })?;

    // Freeze the sim so the captures on both displays show the same frame: Space pauses burl's
    // clock (it keeps drawing, so every resize still redraws). Let some scene build up first.
    std::thread::sleep(Duration::from_secs(6));
    let probe = dir.join("pause-probe.png");
    let grab = || -> Option<crate::ssim::Image> {
        let w = window()?;
        let ok = Command::new("screencapture")
            .args(["-x", "-o", "-l", &w.id.to_string()])
            .arg(&probe)
            .status()
            .ok()?
            .success();
        let img = crate::ssim::decode(&std::fs::read(&probe).ok()?).ok()?;
        ok.then_some(img)
    };
    // Space only toggles, so press it once and read the state back from the pixels: a paused sim
    // still moves the camera for a while, then the frame stops changing. The per-second change
    // (MAE between frames) is logged: a decay means "settling", a plateau means "still running".
    run(Command::new("osascript").args([
        "-e",
        "tell application \"System Events\" to set frontmost of (first process whose name is \"burl\") to true",
        "-e",
        "delay 0.5",
        "-e",
        "tell application \"System Events\" to key code 49",
    ]))?;
    let mut series = Vec::new();
    let mut prev: Option<crate::ssim::Image> = None;
    let mut still = 0;
    let mut missed = 0; // seconds the window was absent from the on-screen list (#169)
    for _ in 0..90 {
        std::thread::sleep(Duration::from_secs(1));
        let Some(now) = grab() else {
            missed += 1;
            continue;
        };
        if let Some(p) = &prev {
            let m = pixels::mae(p, &now).unwrap_or(f64::NAN);
            series.push(m);
            still = if m == 0.0 { still + 1 } else { 0 };
        }
        prev = Some(now);
        if still >= 2 {
            break;
        }
    }
    eprintln!("pause: frame-to-frame MAE per second after Space: {series:.3?}");
    if still < 2 {
        let why = if missed > 0 {
            format!("; burl's window was missing from the on-screen list for {missed} of the polled seconds (minimized, hidden, other Space, locked or asleep display?)")
        } else {
            String::new()
        };
        return Err(format!(
            "burl never held still after pressing Space (still running, or still settling){why}"
        ));
    }

    // 1x first: a window that starts on a 2x display (a Retina panel) then sees a real change on
    // each move; the check only requires `scale-changed` when the scale actually differs.
    let mut contents = Vec::new();
    let mut shots = Vec::new();
    for d in [&d1, &d2] {
        let seen = parse_display_events(&read(&burl_out)).len();
        let (px, py) = (d.x + 100, d.y + 100);
        let script = format!(
            "tell application \"System Events\" to tell (first process whose name is \"burl\") to set position of window 1 to {{{px}, {py}}}"
        );
        run(Command::new("osascript").args(["-e", &script]))?;
        let w = poll(
            &format!("the window to land on the {} display", d.name),
            20,
            || window().filter(|w| w.x >= d.x && w.x < d.x + d.w && w.y >= d.y && w.y < d.y + d.h),
        )?;
        let mut why = String::new();
        poll(
            &format!("burl's `display:` lines after the move to {}", d.name),
            20,
            || {
                let all = parse_display_events(&read(&burl_out));
                match check_move(&all[seen.min(all.len())..], scale, d.scale) {
                    Ok(()) => Some(()),
                    Err(e) => {
                        why = e;
                        None
                    }
                }
            },
        )
        .map_err(|e| format!("{} display: {e}: {why}", d.name))?;
        scale = d.scale;

        // Default scenario: resize so both displays show a 640x360 px frame (640x360 pt at 1x,
        // 320x180 pt at 2x). `set size` is the frame, so add the title bar (window height minus
        // content height). `--high-dpi`: the window is a logical size and stays as it is.
        let bar = parse_display_events(&read(&burl_out))
            .last()
            .map_or(0, |e| w.h.saturating_sub(e.logical.1));
        let (content_pt, w) = if high_dpi {
            ((w.w, w.h.saturating_sub(bar)), w)
        } else {
            let content_pt = (640 / d.scale as u32, 360 / d.scale as u32);
            let script = format!(
                "tell application \"System Events\" to tell (first process whose name is \"burl\") to set size of window 1 to {{{}, {}}}",
                content_pt.0,
                content_pt.1 + bar
            );
            run(Command::new("osascript").args(["-e", &script]))?;
            let w = poll(&format!("the window to be {content_pt:?} pt"), 20, || {
                window().filter(|w| w.w == content_pt.0 && w.h == content_pt.1 + bar)
            })?;
            (content_pt, w)
        };
        // Capture, retrying until burl has redrawn at the new size and the paused frame is still.
        let png = dir.join(format!("{}.png", d.name));
        let mut last = String::new();
        let mut prev: Option<crate::ssim::Image> = None;
        let mut still = None;
        let ok = poll("a settled capture of the right size", 30, || {
            let w = window().unwrap_or(w);
            let shot = Command::new("screencapture")
                .args(["-x", "-o", "-l", &w.id.to_string()])
                .arg(&png)
                .status();
            if !shot.is_ok_and(|s| s.success()) {
                last = "screencapture failed (Screen Recording permission?)".into();
                return None;
            }
            let img = match distinct_colours(&png)
                .and_then(|(px, c)| check_capture(px, (w.w, w.h), d.scale, c))
                .and_then(|()| std::fs::read(&png).map_err(|e| e.to_string()))
                .and_then(|b| crate::ssim::decode(&b))
            {
                Ok(i) => i,
                Err(e) => {
                    last = e;
                    prev = None;
                    return None;
                }
            };
            // Same pixels twice in a row: the paused frame has stopped moving.
            let same = prev.as_ref().is_some_and(|p| p.rgb == img.rgb);
            prev = Some(img);
            if !same {
                last = "the paused frame is still changing".into();
                return None;
            }
            still = prev.take();
            Some(())
        });
        ok.map_err(|e| {
            format!(
                "{} display: {e}: {last} (capture kept at {})",
                d.name,
                png.display()
            )
        })?;
        let content = pixels::content(&still.ok_or("no settled capture")?, content_pt, d.scale)?;
        crate::ssim::write_png(
            &dir.join(format!("{}-content.png", d.name)),
            content.w,
            content.h,
            &content.rgb,
        )?;
        contents.push(content);
        let last = parse_display_events(&read(&burl_out))
            .pop()
            .ok_or("no display: line")?;
        shots.push(HighDpiShot {
            scale: last.scale,
            physical: last.physical,
            logical: last.logical,
            frame: (w.w, w.h),
        });
        eprintln!(
            "ok   {} display: scale {} lines, capture {}",
            d.name,
            d.scale,
            png.display()
        );
    }
    let [c1, c2] = &contents[..] else {
        return Err("expected one content capture per display".into());
    };
    if high_dpi {
        // The 2x window shows a different frame (framed in physical pixels, font scale 2), so no
        // pixel comparison with the 1x one: assert the sizes, then crispness of each capture.
        let [one, two] = &shots[..] else {
            return Err("expected one shot per display".into());
        };
        check_high_dpi(one, two, (640, 360))?;
        eprintln!(
            "ok   --high-dpi: logical 640x360 on both, physical {:?} -> {:?}",
            one.physical, two.physical
        );
        for (d, c) in [("1x", c1), ("2x", c2)] {
            let (crisp, control) =
                pixels::check_crisp(c).map_err(|e| format!("{d} display, --high-dpi: {e}"))?;
            eprintln!(
                "ok   {d} display: crispness {crisp:.2} (blurred copy {control:.2}, limit {})",
                pixels::MIN_CRISPNESS
            );
        }
        return Ok(());
    }
    let pair = pixels::measure_pair(c1, c2)?;
    eprintln!("pixels: 2x display vs 1x display, same paused frame at 640x360 px: {pair:?}");
    pixels::check_native_2x(&pair).map_err(|e| format!("Retina render: {e}"))?;

    // Secondary: regression baselines taken from Burl itself on the hosted runner. Opt-in, because
    // they only mean something on the image that produced them; never a claim of parity.
    if let Some(name) = std::env::var_os("DISPLAY_HARNESS_BASELINE") {
        let base = root.join("tools/display-harness/baselines").join(name);
        for (d, got) in [("1x", c1), ("2x", c2)] {
            let path = base.join(format!("{d}.png"));
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let want = crate::ssim::decode(&bytes)?;
            let s = pixels::check_baseline(got, &want, pixels::MIN_BASELINE_SSIM)
                .map_err(|e| format!("{d} display, regression baseline {}: {e}", path.display()))?;
            eprintln!("ok   {d} display: SSIM {s:.4} against the regression baseline (not parity)");
        }
    }
    Ok(())
}

// ---- cursor grab (`--grab`) ----

/// One `grab:` debug line from burl (`interactive::grab_line`): `(on, mode)`.
pub fn parse_grab_line(line: &str) -> Option<(bool, String)> {
    let rest = &line[line.find("grab: event=")? + "grab: ".len()..];
    let field = |k: &str| rest.split_whitespace().find_map(|t| t.strip_prefix(k));
    let on = match field("event=")? {
        "on" => true,
        "off" => false,
        _ => return None,
    };
    Some((on, field("mode=")?.to_string()))
}

pub fn parse_grab_events(text: &str) -> Vec<(bool, String)> {
    text.lines().filter_map(parse_grab_line).collect()
}

/// One press and release must make exactly one `on` with mode `locked`, then one `off`: macOS only
/// has `Locked` (`Confined` is unsupported there), a second `on` would be a re-grab, and an `off`
/// before the release would be a grab that dropped while the button was still down.
pub fn check_grab_cycle(events: &[(bool, String)]) -> Result<(), String> {
    match events {
        [(true, mode), (false, _)] if mode == "locked" => Ok(()),
        _ => Err(format!(
            "expected `grab: on mode=locked` then `grab: off`, got {events:?}"
        )),
    }
}

fn cursor(mouse: &Path) -> Result<(i32, i32), String> {
    let out = run(Command::new(mouse).arg("pos"))?;
    let mut it = out.split_whitespace().map(|t| t.parse::<i32>());
    match (it.next(), it.next()) {
        (Some(Ok(x)), Some(Ok(y))) => Ok((x, y)),
        _ => Err(format!("mouse pos printed {out:?}")),
    }
}

fn mouse(bin: &Path, args: &[String]) -> Result<(), String> {
    run(Command::new(bin).args(args)).map(|_| ())
}

fn s(n: impl ToString) -> String {
    n.to_string()
}

/// Posts `steps` events of `kind` (`move` or `drag`) moving right by `step` points each, from `p`.
fn push_right(bin: &Path, kind: &str, p: (i32, i32), steps: i32, step: i32) -> Result<(), String> {
    for k in 1..=steps {
        let args = [kind, &s(p.0 + k * step), &s(p.1), &s(step), "0"].map(String::from);
        mouse(bin, &args)?;
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// `cargo xtask display-harness --grab`: a left press on burl's background grabs the cursor
/// (`Gource::grabMouse`) and the release lets go. A real press reaches the real window through the
/// window server, and burl's `grab:` line says what it asked winit for (on macOS `Locked`, which is
/// `CGAssociateMouseAndMouseCursorPosition(false)`; the call's `Ok` is what makes the mode `locked`).
/// Phases: no grab without a press, one grab on the press (kept through drag motion), one release.
///
/// It does NOT assert where the cursor goes: on hosted `macos-15` and `macos-26` the cursor followed
/// 50 points of synthetic `CGEventPost` drag while burl held a locked grab (`(412, 390) -> (462, 390)`),
/// so synthetic events bypass the detach and a position check cannot tell a lock from none. It needs
/// no virtual display.
pub fn run_grab_check(root: &Path) -> Result<(), String> {
    let dir = root.join("target/display-harness-grab");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let src = root.join("tools/display-harness");
    for (m, bin) in [("mouse.m", "mouse"), ("wid.m", "wid")] {
        run(Command::new("clang")
            .args([
                "-fobjc-arc",
                "-framework",
                "Cocoa",
                "-framework",
                "CoreGraphics",
            ])
            .arg(src.join(m))
            .arg("-o")
            .arg(dir.join(bin)))?;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    eprintln!("+ cargo build --release -p burl");
    run(Command::new(&cargo)
        .args(["build", "--release", "-p", "burl"])
        .current_dir(root))?;
    let target =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let burl_bin = target.join("release/burl");

    // Nothing is hidden but the mouse is NOT: `--hide mouse` makes burl drop button presses. With
    // the tree, files, users and slider hidden every click lands on the background, which is what
    // grabs.
    let burl_out = dir.join("burl.out");
    let out = std::fs::File::create(&burl_out).map_err(|e| e.to_string())?;
    let _burl = Proc(
        Command::new(&burl_bin)
            .arg(root.join("tests/golden/sim/small-repo.log"))
            .args([
                "--log-format",
                "custom",
                "-640x360",
                "--seconds-per-day",
                "10",
            ])
            .args([
                "--hash-seed",
                "31",
                "--prng",
                "glibc",
                "--disable-auto-skip",
            ])
            .args(["--disable-auto-rotate", "--log-level", "debug"])
            .args([
                "--hide",
                "tree,files,users,usernames,filenames,dirnames,bloom,progress",
            ])
            .stdout(out.try_clone().map_err(|e| e.to_string())?)
            .stderr(out)
            .spawn()
            .map_err(|e| format!("{}: {e}", burl_bin.display()))?,
    );
    let window = || -> Option<Window> {
        let o = Command::new(dir.join("wid")).arg("burl").output().ok()?;
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .find_map(parse_window)
    };
    let w = poll("burl's window", 60, window)?;
    run(Command::new("osascript").args([
        "-e",
        "tell application \"System Events\" to set frontmost of (first process whose name is \"burl\") to true",
    ]))?;
    std::thread::sleep(Duration::from_secs(3));

    let mouse_bin = dir.join("mouse");
    // Well inside the content area, clear of the title bar and the window edges.
    let p = (w.x + (w.w as i32) / 2 - 100, w.y + (w.h as i32) * 3 / 4);
    let place = || -> Result<(), String> {
        mouse(&mouse_bin, &["warp".into(), s(p.0), s(p.1)])?;
        std::thread::sleep(Duration::from_millis(300));
        let at = cursor(&mouse_bin)?;
        if (at.0 - p.0).abs() > 3 || (at.1 - p.1).abs() > 3 {
            return Err(format!(
                "the helper could not place the cursor at {p:?} (it is at {at:?})"
            ));
        }
        Ok(())
    };
    let seen = || parse_grab_events(&read(&burl_out)).len();

    // Phase A, control: motion with no button down grabs nothing, so the `on` below is the press's.
    place()?;
    let before = seen();
    push_right(&mouse_bin, "move", p, 5, 10)?;
    std::thread::sleep(Duration::from_secs(1));
    if seen() != before {
        return Err(format!(
            "burl logged a grab with no button down: {:?}",
            parse_grab_events(&read(&burl_out))
        ));
    }
    eprintln!("ok   control: motion alone does not grab");

    // Phase B, press on the background: one grab, locked, kept through drag motion.
    place()?;
    mouse(&mouse_bin, &["down".into(), s(p.0), s(p.1)])?;
    let held = (|| -> Result<(), String> {
        poll("burl's `grab: event=on` line after the press", 15, || {
            parse_grab_events(&read(&burl_out))
                .get(before..)?
                .iter()
                .any(|(on, _)| *on)
                .then_some(())
        })?;
        eprintln!("ok   burl reports grab on");
        push_right(&mouse_bin, "drag", p, 5, 10)?;
        let now = &parse_grab_events(&read(&burl_out))[before..];
        if now.len() != 1 {
            return Err(format!("the grab changed during the drag: {now:?}"));
        }
        eprintln!("ok   the grab holds through drag motion");
        Ok(())
    })();
    // Release whatever happened, so a failure above cannot leave the button down.
    mouse(&mouse_bin, &["up".into(), s(p.0), s(p.1)])?;
    held?;

    // Phase C, release: burl lets go, and the whole cycle is on, locked, off.
    poll(
        "burl's `grab: event=off` line after the release",
        15,
        || {
            parse_grab_events(&read(&burl_out))
                .get(before..)?
                .iter()
                .any(|(on, _)| !*on)
                .then_some(())
        },
    )?;
    check_grab_cycle(&parse_grab_events(&read(&burl_out))[before..])?;
    eprintln!("ok   burl reports grab off: one press, one locked grab, one release");
    Ok(())
}

// ---- real title-bar drag (`--drag`) ----

/// Points below the window's top edge where the drag grabs: inside the title bar (28 to 32 pt tall).
const TITLE_BAR_GRAB_DOWN: i32 = 12;

/// Where to press on `w`'s title bar: horizontally centred (clear of the traffic lights at the
/// left, and inside even the 320 pt wide window on the 2x display), a few points down.
pub fn title_bar_point(w: &Window) -> (i32, i32) {
    (w.x + (w.w as i32) / 2, w.y + TITLE_BAR_GRAB_DOWN)
}

/// The middle of a display, in the global coordinates the helper printed (may be negative).
pub fn display_centre(d: &Display) -> (i32, i32) {
    (d.x + d.w / 2, d.y + d.h / 2)
}

/// Is the window's origin inside display `d`? (The same test the `set position` flow uses.)
pub fn window_on(d: &Display, w: &Window) -> bool {
    w.x >= d.x && w.x < d.x + d.w && w.y >= d.y && w.y < d.y + d.h
}

/// `steps` points on the straight line `from` -> `to`, each with the delta from the previous point
/// (the first from `from`). The last point is exactly `to`; integer maths, so deltas sum exactly.
pub fn drag_path(from: (i32, i32), to: (i32, i32), steps: i32) -> Vec<(i32, i32, i32, i32)> {
    let mut prev = from;
    (1..=steps)
        .map(|k| {
            let p = (
                from.0 + (to.0 - from.0) * k / steps,
                from.1 + (to.1 - from.1) * k / steps,
            );
            let step = (p.0, p.1, p.0 - prev.0, p.1 - prev.1);
            prev = p;
            step
        })
        .collect()
}

/// `cargo xtask display-harness --drag`: the window is dragged by its title bar with real mouse
/// events (`mouse.m`) from the 1x display to the 2x display and back; each crossing must make burl
/// log `scale-changed` then `resized` at the target scale (`check_move`) and leave the window on
/// the target display. Replaces the `set position` move for this one thing: a user drags.
///
/// Unverified until a hosted run: that a synthetic title-bar drag across the virtual display
/// boundary makes the window server move the window and winit fire `ScaleFactorChanged`.
pub fn run_drag_check(root: &Path) -> Result<(), String> {
    let dir = root.join("target/display-harness-drag");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let src = root.join("tools/display-harness");
    for (m, bin) in [("vd.m", "vd"), ("wid.m", "wid"), ("mouse.m", "mouse")] {
        run(Command::new("clang")
            .args([
                "-fobjc-arc",
                "-framework",
                "Cocoa",
                "-framework",
                "CoreGraphics",
            ])
            .arg(src.join(m))
            .arg("-o")
            .arg(dir.join(bin)))?;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    eprintln!("+ cargo build --release -p burl");
    run(Command::new(&cargo)
        .args(["build", "--release", "-p", "burl"])
        .current_dir(root))?;
    let target =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let burl_bin = target.join("release/burl");

    let vd_out = dir.join("vd.out");
    let vd_file = std::fs::File::create(&vd_out).map_err(|e| e.to_string())?;
    let mut vd = Proc(
        Command::new(dir.join("vd"))
            .stdout(vd_file)
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("vd: {e}"))?,
    );
    let text = poll("the helper's `ready` line", 60, || {
        let t = read(&vd_out);
        if t.lines().any(|l| l == "ready") {
            return Some(Ok(t));
        }
        let st = vd.0.try_wait().ok().flatten()?;
        Some(Err(format!(
            "the display helper exited ({st}) before printing `ready` (a leftover `vd` process \
             may still hold the virtual displays)"
        )))
    })??;
    let displays: Vec<Display> = text.lines().filter_map(parse_display).collect();
    let find = |name: &str| {
        displays
            .iter()
            .find(|d| d.name == name)
            .cloned()
            .ok_or(format!("helper reported no {name} display"))
    };
    let (d2, d1) = (find("2x")?, find("1x")?);
    eprintln!("displays: {d2:?} {d1:?}");

    // `--hide mouse` is not passed (it drops button presses); the title bar is the OS's anyway.
    let burl_out = dir.join("burl.out");
    let out = std::fs::File::create(&burl_out).map_err(|e| e.to_string())?;
    let _burl = Proc(
        Command::new(&burl_bin)
            .arg(root.join("tests/golden/sim/small-repo.log"))
            .args([
                "--log-format",
                "custom",
                "-640x360",
                "--seconds-per-day",
                "10",
            ])
            .args([
                "--hash-seed",
                "31",
                "--prng",
                "glibc",
                "--disable-auto-skip",
            ])
            .args(["--disable-auto-rotate", "--log-level", "debug"])
            .stdout(out.try_clone().map_err(|e| e.to_string())?)
            .stderr(out)
            .spawn()
            .map_err(|e| format!("{}: {e}", burl_bin.display()))?,
    );
    let window = || -> Option<Window> {
        let o = Command::new(dir.join("wid")).arg("burl").output().ok()?;
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .find_map(parse_window)
    };
    poll("burl's window", 60, window)?;
    poll("burl's `display: event=start` line", 30, || {
        parse_display_events(&read(&burl_out)).first().map(|_| ())
    })?;
    let mouse_bin = dir.join("mouse");

    // Start on the 1x display (System Events move, as the other scenarios), then drag 1x -> 2x -> 1x.
    for (leg, (from, to)) in [(&d1, &d2), (&d2, &d1)].into_iter().enumerate() {
        if leg == 0 {
            let script = format!(
                "tell application \"System Events\" to tell (first process whose name is \"burl\") to set position of window 1 to {{{}, {}}}",
                from.x + 100,
                from.y + 100
            );
            run(Command::new("osascript").args(["-e", &script]))?;
        }
        let w = poll(
            &format!("the window to sit on the {} display", from.name),
            20,
            || window().filter(|w| window_on(from, w)),
        )?;
        // The resize after the positioning move must have settled so `seen` is a clean baseline.
        poll(
            &format!("burl at scale {} on the {} display", from.scale, from.name),
            20,
            || {
                let last = parse_display_events(&read(&burl_out)).pop()?;
                (last.scale == from.scale).then_some(())
            },
        )?;
        std::thread::sleep(Duration::from_secs(2));
        run(Command::new("osascript").args([
            "-e",
            "tell application \"System Events\" to set frontmost of (first process whose name is \"burl\") to true",
        ]))?;
        std::thread::sleep(Duration::from_secs(1));
        let w = window().unwrap_or(w);
        let before = parse_display_events(&read(&burl_out));
        let seen = before.len();
        let before_logical = before.last().ok_or("no display: line")?.logical;
        let start = title_bar_point(&w);
        let end = display_centre(to);
        let steps = drag_path(start, end, 40);
        eprintln!(
            "drag {}: window {w:?}, press at {start:?}, release at {end:?}",
            leg_label(leg)
        );

        mouse(&mouse_bin, &["warp".into(), s(start.0), s(start.1)])?;
        std::thread::sleep(Duration::from_millis(300));
        mouse(&mouse_bin, &["down".into(), s(start.0), s(start.1)])?;
        let dragged = (|| -> Result<(), String> {
            for (x, y, dx, dy) in steps {
                mouse(&mouse_bin, &["drag".into(), s(x), s(y), s(dx), s(dy)])?;
                std::thread::sleep(Duration::from_millis(30));
            }
            std::thread::sleep(Duration::from_millis(500));
            Ok(())
        })();
        // Release whatever happened, so a failure above cannot leave the button down.
        mouse(&mouse_bin, &["up".into(), s(end.0), s(end.1)])?;
        dragged?;

        let mut why = String::new();
        poll(
            &format!("burl's `display:` lines after the drag {}", leg_label(leg)),
            20,
            || {
                let all = parse_display_events(&read(&burl_out));
                match check_move(&all[seen.min(all.len())..], from.scale, to.scale) {
                    Ok(()) => Some(()),
                    Err(e) => {
                        why = e;
                        None
                    }
                }
            },
        )
        .map_err(|e| {
            format!(
                "drag {}: {e}: {why} (did the title-bar drag move the window? it is at {:?})",
                leg_label(leg),
                window()
            )
        })?;
        poll(
            &format!("the window to land on the {} display", to.name),
            20,
            || window().filter(|w| window_on(to, w)),
        )?;
        eprintln!(
            "ok   drag {}: scale-changed then resized at scale {}, window on the {} display",
            leg_label(leg),
            to.scale,
            to.name
        );

        // Layout judged after the move: the window keeps its point size and burl its logical size,
        // and a capture of the window is scale x its points and not blank.
        let mut why = String::new();
        let landed = poll(
            &format!("the window size to hold after the drag {}", leg_label(leg)),
            20,
            || {
                let now = window()?;
                let logical = parse_display_events(&read(&burl_out)).pop()?.logical;
                match check_size_kept((w.w, w.h), (now.w, now.h), before_logical, logical) {
                    Ok(()) => Some((now, logical)),
                    Err(e) => {
                        why = e;
                        None
                    }
                }
            },
        )
        .map_err(|e| format!("drag {}: {e}: {why}", leg_label(leg)))?;
        let png = dir.join(format!("drag-{}.png", to.name));
        let mut last = String::new();
        let mut fill = String::new();
        poll("a capture of the right size", 30, || {
            let w = window().unwrap_or(landed.0);
            let shot = Command::new("screencapture")
                .args(["-x", "-o", "-l", &w.id.to_string()])
                .arg(&png)
                .status();
            if !shot.is_ok_and(|s| s.success()) {
                last = "screencapture failed (Screen Recording permission?)".into();
                return None;
            }
            match distinct_colours(&png)
                .and_then(|(px, c)| check_capture(px, (w.w, w.h), to.scale, c))
                .and_then(|()| {
                    let img = crate::ssim::decode(
                        &std::fs::read(&png).map_err(|e| format!("{}: {e}", png.display()))?,
                    )?;
                    let c = pixels::content(&img, landed.1, to.scale)?;
                    pixels::check_scene_fills_window(&c, to.scale)
                }) {
                Ok(nums) => {
                    fill = nums;
                    Some(())
                }
                Err(e) => {
                    last = e;
                    None
                }
            }
        })
        .map_err(|e| {
            format!(
                "drag {}: {e}: {last} (capture kept at {})",
                leg_label(leg),
                png.display()
            )
        })?;
        eprintln!(
            "ok   drag {}: window {}x{} pt and logical {:?} unchanged, capture {}, scene fills window: {fill}",
            leg_label(leg),
            landed.0.w,
            landed.0.h,
            landed.1,
            png.display()
        );
    }
    Ok(())
}

/// A move must not resize the window: its point size (the OS's frame) and burl's logical size
/// (`display:` line) are the same after the move as before it.
pub fn check_size_kept(
    points_before: (u32, u32),
    points_after: (u32, u32),
    logical_before: (u32, u32),
    logical_after: (u32, u32),
) -> Result<(), String> {
    if points_before != points_after {
        return Err(format!(
            "window frame changed from {points_before:?} pt to {points_after:?} pt"
        ));
    }
    if logical_before != logical_after {
        return Err(format!(
            "burl's logical size changed from {logical_before:?} to {logical_after:?}"
        ));
    }
    Ok(())
}

fn leg_label(leg: usize) -> &'static str {
    ["1x -> 2x", "2x -> 1x"][leg]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disp(name: &str, x: i32, w: i32, scale: f64) -> Display {
        Display {
            name: name.into(),
            x,
            y: 0,
            w,
            h: 720,
            scale,
            pixels: (0, 0),
        }
    }

    #[test]
    fn drag_path_ends_exactly_and_steps_sum_to_the_distance() {
        let (from, to) = ((-1180, 112), (960, 540));
        let p = drag_path(from, to, 40);
        assert_eq!(p.len(), 40);
        assert_eq!((p[39].0, p[39].1), to);
        let (dx, dy) = p.iter().fold((0, 0), |a, s| (a.0 + s.2, a.1 + s.3));
        assert_eq!((dx, dy), (to.0 - from.0, to.1 - from.1));
        // from the 1x display (x in -1280..0) to the 2x one (x >= 0): some point is on each side
        assert!(p.iter().any(|s| s.0 < 0) && p.iter().any(|s| s.0 >= 0));
    }

    #[test]
    fn the_press_is_in_the_title_bar_and_the_release_in_the_target_display() {
        let w = Window {
            id: 1,
            x: -1180,
            y: 100,
            w: 640,
            h: 392,
        };
        assert_eq!(title_bar_point(&w), (-860, 112));
        let one = disp("1x", -1280, 1280, 1.0);
        assert_eq!(display_centre(&one), (-640, 360));
        assert!(window_on(&one, &w));
        assert!(!window_on(&disp("2x", 0, 1920, 2.0), &w));
    }

    #[test]
    fn parses_burls_grab_lines_and_ignores_others() {
        assert_eq!(
            parse_grab_line("  debug: grab: event=on mode=locked"),
            Some((true, "locked".into()))
        );
        assert_eq!(
            parse_grab_line("grab: event=off mode=none"),
            Some((false, "none".into()))
        );
        assert_eq!(parse_grab_line("grab: event=sideways mode=locked"), None);
        assert_eq!(parse_grab_line("pointer: selected=none hover=none"), None);
    }

    #[test]
    fn one_press_is_one_locked_grab_then_one_release() {
        let ev = |l: &[(bool, &str)]| -> Vec<(bool, String)> {
            l.iter().map(|(on, m)| (*on, m.to_string())).collect()
        };
        assert_eq!(
            check_grab_cycle(&ev(&[(true, "locked"), (false, "none")])),
            Ok(())
        );
        assert!(check_grab_cycle(&ev(&[])).is_err()); // the click never reached the grab
        assert!(check_grab_cycle(&ev(&[(true, "locked")])).is_err()); // never released
        assert!(check_grab_cycle(&ev(&[(true, "none"), (false, "none")])).is_err()); // grab failed
        assert!(check_grab_cycle(&ev(&[(true, "confined"), (false, "none")])).is_err());
        assert!(
            check_grab_cycle(&ev(&[(true, "locked"), (true, "locked"), (false, "none")])).is_err()
        );
        assert!(check_grab_cycle(&ev(&[(false, "none"), (true, "locked")])).is_err());
        // wrong order
    }

    const START: &str = "  debug: display: event=start scale=1 physical=640x360 logical=640x360";
    const CHANGED: &str =
        "  debug: display: event=scale-changed scale=2 physical=1280x720 logical=640x360";
    const RESIZED: &str =
        "  debug: display: event=resized scale=2 physical=1280x720 logical=640x360";

    fn ev(lines: &[&str]) -> Vec<DisplayEvent> {
        parse_display_events(&lines.join("\n"))
    }

    #[test]
    fn parses_burls_display_lines_and_ignores_others() {
        let e = ev(&["noise", START, "display: event=bogus"]);
        assert_eq!(e.len(), 1);
        assert_eq!(
            e[0],
            DisplayEvent {
                event: "start".into(),
                scale: 1.0,
                physical: (640, 360),
                logical: (640, 360)
            }
        );
    }

    #[test]
    fn a_scale_change_needs_scale_changed_then_resized() {
        assert_eq!(check_move(&ev(&[CHANGED, RESIZED]), 1.0, 2.0), Ok(()));
        assert!(check_move(&ev(&[RESIZED]), 1.0, 2.0).is_err()); // no scale-changed
        assert!(check_move(&ev(&[CHANGED]), 1.0, 2.0).is_err()); // no resized after it
        assert!(check_move(&ev(&[RESIZED, CHANGED]), 1.0, 2.0).is_err()); // wrong order
        assert_eq!(check_move(&[], 2.0, 2.0), Ok(())); // already at that scale
    }

    #[test]
    fn rejects_wrong_scale_and_inconsistent_logical_size() {
        assert!(check_move(&ev(&[CHANGED, RESIZED]), 1.0, 1.0).is_err());
        let bad = "display: event=scale-changed scale=2 physical=1280x720 logical=1280x720";
        assert!(check_move(&ev(&[bad, RESIZED]), 1.0, 2.0).is_err());
    }

    #[test]
    fn capture_must_be_scaled_and_not_blank() {
        assert_eq!(check_capture((1280, 720), (640, 360), 2.0, 9), Ok(()));
        assert!(check_capture((640, 360), (640, 360), 2.0, 9).is_err());
        assert!(check_capture((1280, 720), (640, 360), 2.0, 1).is_err());
    }

    fn shot(
        scale: f64,
        physical: (u32, u32),
        logical: (u32, u32),
        frame: (u32, u32),
    ) -> HighDpiShot {
        HighDpiShot {
            scale,
            physical,
            logical,
            frame,
        }
    }
    const V: (u32, u32) = (640, 360);
    /// The window frame for `V`: content plus a 32 pt title bar (as measured on macOS 26).
    const F: (u32, u32) = (640, 392);

    #[test]
    fn high_dpi_window_is_the_same_logical_size_and_doubles_physically() {
        let one = shot(1.0, V, V, F);
        let two = shot(2.0, (1280, 720), V, F);
        assert_eq!(check_high_dpi(&one, &two, V), Ok(()));
        // The exact-pixel window of G3 (no --high-dpi): 2x is 640x360 px = 320x180 pt.
        let exact = shot(2.0, V, (320, 180), (320, 212));
        assert!(check_high_dpi(&one, &exact, V).is_err());
        // Logical size right but the window did not double physically (macOS scaled a 1x surface).
        assert!(check_high_dpi(&one, &shot(2.0, V, V, F), V).is_err());
        // The OS frame is narrower than the logical width the line reports.
        assert!(check_high_dpi(&one, &shot(2.0, (1280, 720), V, (320, 212)), V).is_err());
        // Wrong scale on a display.
        assert!(check_high_dpi(&shot(2.0, V, V, F), &two, V).is_err());
    }

    #[test]
    fn a_mis_sized_window_height_shows_as_a_different_title_bar() {
        let one = shot(1.0, V, V, F);
        // Same logical height reported, but the 2x frame is 28 pt taller: not the same title bar.
        assert!(check_high_dpi(&one, &shot(2.0, (1280, 720), V, (640, 420)), V).is_err());
        // No title bar at all (frame height equals the content height): the OS frame and winit's
        // size cannot both be right.
        assert!(check_high_dpi(&one, &shot(2.0, (1280, 720), V, V), V).is_err());
        // Frame shorter than the content.
        assert!(check_high_dpi(&one, &shot(2.0, (1280, 720), V, (640, 300)), V).is_err());
    }

    #[test]
    fn a_move_must_keep_the_point_and_logical_sizes() {
        let v = (640, 360);
        assert_eq!(check_size_kept((640, 392), (640, 392), v, v), Ok(()));
        assert!(check_size_kept((640, 392), (320, 212), v, v).is_err());
        assert!(check_size_kept((640, 392), (640, 392), v, (320, 180)).is_err());
    }

    #[test]
    fn parses_helper_and_window_lines() {
        let d = parse_display(
            "display name=1x id=61 x=-1280 y=0 w=1280 h=720 scale=1.0 pixels=1280x720",
        )
        .unwrap();
        assert_eq!(
            (d.x, d.w, d.scale, d.pixels),
            (-1280, 1280, 1.0, (1280, 720))
        );
        assert_eq!(
            parse_window("42 100 -20 640 360"),
            Some(Window {
                id: 42,
                x: 100,
                y: -20,
                w: 640,
                h: 360
            })
        );
        assert_eq!(parse_window("ready"), None);
    }
}
