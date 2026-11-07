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
pub fn run_harness(root: &Path) -> Result<(), String> {
    let dir = root.join("target/display-harness");
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
    let _vd = Proc(
        Command::new(dir.join("vd"))
            .stdout(vd_file)
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("vd: {e}"))?,
    );
    let text = poll("the helper's `ready` line", 60, || {
        let t = read(&vd_out);
        t.lines().any(|l| l == "ready").then_some(t)
    })?;
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
    let grab = || -> Option<Vec<u8>> {
        let w = window()?;
        let ok = Command::new("screencapture")
            .args(["-x", "-o", "-l", &w.id.to_string()])
            .arg(&probe)
            .status()
            .ok()?
            .success();
        let img = crate::ssim::decode(&std::fs::read(&probe).ok()?).ok()?;
        ok.then_some(img.rgb)
    };
    // Space only toggles, so the state is read back from the pixels: a running sim never shows
    // the same frame twice; a paused one stops changing once the camera has settled. Press again
    // whenever the frame is still moving after a generous wait.
    let mut paused = false;
    for _ in 0..5 {
        run(Command::new("osascript").args([
            "-e",
            "tell application \"System Events\" to set frontmost of (first process whose name is \"burl\") to true",
            "-e",
            "delay 0.5",
            "-e",
            "tell application \"System Events\" to key code 49",
        ]))?;
        let mut prev = None;
        let end = Instant::now() + Duration::from_secs(10);
        while Instant::now() < end && !paused {
            std::thread::sleep(Duration::from_millis(700));
            let now = grab();
            paused = now.is_some() && now == prev;
            prev = now;
        }
        if paused {
            break;
        }
    }
    if !paused {
        return Err("burl never held still after pressing Space (pause failed?)".into());
    }

    // 1x first: a window that starts on a 2x display (a Retina panel) then sees a real change on
    // each move; the check only requires `scale-changed` when the scale actually differs.
    let mut contents = Vec::new();
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

        // Resize so both displays show a 640x360 px frame: 640x360 pt at 1x, 320x180 pt at 2x.
        // `set size` is the frame, so add the title bar (window height minus content height).
        let content_pt = (640 / d.scale as u32, 360 / d.scale as u32);
        let bar = parse_display_events(&read(&burl_out))
            .last()
            .map_or(0, |e| w.h.saturating_sub(e.logical.1));
        let script = format!(
            "tell application \"System Events\" to tell (first process whose name is \"burl\") to set size of window 1 to {{{}, {}}}",
            content_pt.0,
            content_pt.1 + bar
        );
        run(Command::new("osascript").args(["-e", &script]))?;
        let w = poll(&format!("the window to be {content_pt:?} pt"), 20, || {
            window().filter(|w| w.w == content_pt.0 && w.h == content_pt.1 + bar)
        })?;

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
    let pair = pixels::measure_pair(c1, c2)?;
    eprintln!("pixels: 2x display vs 1x display, same paused frame at 640x360 px: {pair:?}");
    pixels::check_native_2x(&pair).map_err(|e| format!("Retina render: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
