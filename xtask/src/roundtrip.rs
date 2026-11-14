//! `cargo xtask display-harness --roundtrip`: the window is paused, captured on the 1x display
//! (A), dragged by its title bar to the 2x display and back, and captured again (B). Layout
//! survived the scale changes iff B equals A: same display, same point size, same paused frame.
//! The 2x leg gets the `--drag` checks only (a 2x capture cannot be compared with A pixel for
//! pixel, see the README). See `tools/display-harness/README.md`.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::display_harness::{
    check_capture, check_move, check_size_kept, display_centre, drag_path, parse_display,
    parse_display_events, parse_window, title_bar_point, window_on, Display, Window,
};
use crate::pixels;
use crate::ssim::Image;

// ---- pure checks (unit-tested below) ----

/// A round trip on one display is the same paused frame, so only capture noise remains. The 1x/2x
/// pair of the default scenario measures 0.04 to 0.05 with the rounded window corners as residue;
/// same display and size has no such residue, so the same limit (`MAX_NATIVE_MAE`, 0.25, five
/// times the worst measured) is generous here.
pub const MAX_ROUNDTRIP_MAE: f64 = pixels::MAX_NATIVE_MAE;
/// A corrupted B (shifted or blurred) must be at least this many times worse than the limit.
pub const MIN_CONTROL_FACTOR: f64 = pixels::MIN_CONTROL_FACTOR;
/// Shift of the synthetic corrupted B, in pixels.
const SHIFT_PX: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundTrip {
    pub mae: f64,
    /// MAE(A, B shifted by `SHIFT_PX` pixels) and MAE(A, B blurred): the negative controls.
    pub shifted_mae: f64,
    pub blurred_mae: f64,
}

fn shifted(img: &Image) -> Image {
    let mut rgb = img.rgb.clone();
    rgb.rotate_right(SHIFT_PX * 3);
    Image {
        w: img.w,
        h: img.h,
        rgb,
    }
}

/// Frame size first (a size mismatch is a layout failure, not a pixel one), then MAE(A, B) against
/// the limit, then the controls: if neither a shifted nor a blurred B is `MIN_CONTROL_FACTOR` x
/// the limit away from A, the frame cannot tell a broken layout from a good one (flat, blank).
pub fn check_roundtrip(a: &Image, b: &Image) -> Result<RoundTrip, String> {
    if (a.w, a.h) != (b.w, b.h) {
        return Err(format!(
            "frame size differs after the round trip: before {}x{} px, after {}x{} px",
            a.w, a.h, b.w, b.h
        ));
    }
    let mae = pixels::mae(a, b)?;
    let shifted_mae = pixels::mae(a, &shifted(b))?;
    // `blurred` drops an odd last row/column: compare the common even part.
    let (w, h) = (b.w & !1, b.h & !1);
    let blurred_mae = pixels::mae(&pixels::crop(a, 0, 0, w, h)?, &pixels::blurred(b))?;
    let r = RoundTrip {
        mae,
        shifted_mae,
        blurred_mae,
    };
    if mae > MAX_ROUNDTRIP_MAE {
        return Err(format!(
            "the window shows a different frame after the 1x -> 2x -> 1x round trip: MAE {mae:.3} > {MAX_ROUNDTRIP_MAE} ({r:?})"
        ));
    }
    if shifted_mae.max(blurred_mae) < MAX_ROUNDTRIP_MAE * MIN_CONTROL_FACTOR {
        return Err(format!(
            "check is vacuous on this frame: corrupted copies have MAE {shifted_mae:.3} (shifted) and {blurred_mae:.3} (blurred), under {MIN_CONTROL_FACTOR}x the limit {MAX_ROUNDTRIP_MAE}"
        ));
    }
    Ok(r)
}

// ---- process plumbing (small duplicates of display_harness's private helpers) ----

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

fn colours(img: &Image) -> usize {
    let mut seen = std::collections::HashSet::new();
    for px in img.rgb.as_chunks::<3>().0 {
        seen.insert(*px);
        if seen.len() >= 16 {
            break;
        }
    }
    seen.len()
}

const FRONT: &str = "tell application \"System Events\" to set frontmost of (first process whose name is \"burl\") to true";

struct Ctx {
    dir: PathBuf,
    burl_out: PathBuf,
}

impl Ctx {
    fn window(&self) -> Option<Window> {
        let o = Command::new(self.dir.join("wid"))
            .arg("burl")
            .output()
            .ok()?;
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .find_map(parse_window)
    }

    fn grab(&self, png: &Path) -> Option<(Window, Image)> {
        let w = self.window()?;
        let ok = Command::new("screencapture")
            .args(["-x", "-o", "-l", &w.id.to_string()])
            .arg(png)
            .status()
            .ok()?
            .success();
        let img = crate::ssim::decode(&std::fs::read(png).ok()?).ok()?;
        ok.then_some((w, img))
    }

    fn mouse(&self, args: &[String]) -> Result<(), String> {
        run(Command::new(self.dir.join("mouse")).args(args)).map(|_| ())
    }

    /// Press Space once (it only toggles) and wait until two consecutive captures are identical.
    fn pause(&self) -> Result<(), String> {
        let probe = self.dir.join("pause-probe.png");
        run(Command::new("osascript").args([
            "-e",
            FRONT,
            "-e",
            "delay 0.5",
            "-e",
            "tell application \"System Events\" to key code 49",
        ]))?;
        let (mut series, mut prev, mut still) = (Vec::new(), None::<Image>, 0);
        for _ in 0..90 {
            std::thread::sleep(Duration::from_secs(1));
            let Some((_, now)) = self.grab(&probe) else {
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
            return Err("burl never held still after pressing Space (still running, or still settling, or its window was off the on-screen list)".into());
        }
        Ok(())
    }

    /// A capture of burl's window on a display of `scale`, once its size is right, the scene fills
    /// it and two consecutive captures are identical. Returns the window, the content (title bar
    /// cropped) and the fill numbers.
    fn settled(
        &self,
        name: &str,
        scale: f64,
        logical: (u32, u32),
    ) -> Result<(Window, Image, String), String> {
        let png = self.dir.join(format!("{name}.png"));
        let (mut last, mut prev) = (String::new(), None::<Image>);
        poll("a settled capture of the right size", 30, || {
            let Some((w, img)) = self.grab(&png) else {
                last = "screencapture failed or no window".into();
                return None;
            };
            let r = check_capture(
                (img.w as u32, img.h as u32),
                (w.w, w.h),
                scale,
                colours(&img),
            )
            .and_then(|()| pixels::content(&img, logical, scale))
            .and_then(|c| pixels::check_scene_fills_window(&c, scale).map(|n| (c, n)));
            let (c, nums) = match r {
                Ok(v) => v,
                Err(e) => {
                    last = e;
                    prev = None;
                    return None;
                }
            };
            let same = prev.as_ref().is_some_and(|p| p.rgb == img.rgb);
            prev = Some(img);
            if !same {
                last = "the paused frame is still changing".into();
                return None;
            }
            Some((w, c, nums))
        })
        .map_err(|e| format!("{name}: {e}: {last} (capture kept at {})", png.display()))
    }

    /// One real title-bar drag `from` -> `to`, then the `--drag` assertions on the landing.
    /// Returns the landed window and burl's logical size.
    fn drag(
        &self,
        from: &Display,
        to: &Display,
        label: &str,
    ) -> Result<(Window, (u32, u32)), String> {
        let w = poll(
            &format!("the window on the {} display", from.name),
            20,
            || self.window().filter(|w| window_on(from, w)),
        )?;
        let before = parse_display_events(&read(&self.burl_out));
        let seen = before.len();
        let before_logical = before.last().ok_or("no display: line")?.logical;
        let (start, end) = (title_bar_point(&w), display_centre(to));
        eprintln!("drag {label}: window {w:?}, press at {start:?}, release at {end:?}");
        let s = |n: i32| n.to_string();
        run(Command::new("osascript").args(["-e", FRONT]))?;
        std::thread::sleep(Duration::from_secs(1));
        self.mouse(&["warp".into(), s(start.0), s(start.1)])?;
        std::thread::sleep(Duration::from_millis(300));
        self.mouse(&["down".into(), s(start.0), s(start.1)])?;
        let dragged = (|| -> Result<(), String> {
            for (x, y, dx, dy) in drag_path(start, end, 40) {
                self.mouse(&["drag".into(), s(x), s(y), s(dx), s(dy)])?;
                std::thread::sleep(Duration::from_millis(30));
            }
            std::thread::sleep(Duration::from_millis(500));
            Ok(())
        })();
        self.mouse(&["up".into(), s(end.0), s(end.1)])?;
        dragged?;
        let mut why = String::new();
        poll(
            &format!("burl's `display:` lines after the drag {label}"),
            20,
            || {
                let all = parse_display_events(&read(&self.burl_out));
                match check_move(&all[seen.min(all.len())..], from.scale, to.scale) {
                    Ok(()) => Some(()),
                    Err(e) => {
                        why = e;
                        None
                    }
                }
            },
        )
        .map_err(|e| format!("drag {label}: {e}: {why} (window at {:?})", self.window()))?;
        poll(
            &format!("the window to land on the {} display", to.name),
            20,
            || self.window().filter(|w| window_on(to, w)),
        )?;
        let mut why = String::new();
        poll(
            &format!("the window size to hold after the drag {label}"),
            20,
            || {
                let now = self.window()?;
                let logical = parse_display_events(&read(&self.burl_out)).pop()?.logical;
                match check_size_kept((w.w, w.h), (now.w, now.h), before_logical, logical) {
                    Ok(()) => Some((now, logical)),
                    Err(e) => {
                        why = e;
                        None
                    }
                }
            },
        )
        .map_err(|e| format!("drag {label}: {e}: {why}"))
    }
}

/// `cargo xtask display-harness --roundtrip`. Unverified until a hosted run: that a paused burl
/// redraws the same frame after two scale changes, and that the cursor resting on the title bar
/// does not change burl's drawing.
pub fn run_roundtrip_check(root: &Path) -> Result<(), String> {
    let dir = root.join("target/display-harness-roundtrip");
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
            "the display helper exited ({st}) before printing `ready` (a leftover `vd` process may still hold the virtual displays)"
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

    // The default scenario's flags (a paused, deterministic scene) without `--hide mouse`, which
    // drops the button presses a title-bar drag needs.
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
            .args([
                "--disable-auto-rotate",
                "--file-idle-time",
                "0",
                "--time-scale",
                "1",
            ])
            .args(["--log-level", "debug"])
            .stdout(out.try_clone().map_err(|e| e.to_string())?)
            .stderr(out)
            .spawn()
            .map_err(|e| format!("{}: {e}", burl_bin.display()))?,
    );
    let ctx = Ctx { dir, burl_out };
    poll("burl's window", 60, || ctx.window())?;
    poll("burl's `display: event=start` line", 30, || {
        parse_display_events(&read(&ctx.burl_out))
            .first()
            .map(|_| ())
    })?;

    // Start on the 1x display, let the scene build up, pause.
    let script = format!(
        "tell application \"System Events\" to tell (first process whose name is \"burl\") to set position of window 1 to {{{}, {}}}",
        d1.x + 100,
        d1.y + 100
    );
    run(Command::new("osascript").args(["-e", &script]))?;
    poll("the window on the 1x display", 20, || {
        ctx.window().filter(|w| window_on(&d1, w))
    })?;
    poll("burl at scale 1 on the 1x display", 20, || {
        let last = parse_display_events(&read(&ctx.burl_out)).pop()?;
        (last.scale == d1.scale).then_some(())
    })?;
    std::thread::sleep(Duration::from_secs(6));
    ctx.pause()?;

    // Capture A with the cursor where it will be after the drag back: on the title bar.
    let w0 = ctx.window().ok_or("burl's window vanished")?;
    let tb = title_bar_point(&w0);
    ctx.mouse(&["warp".into(), tb.0.to_string(), tb.1.to_string()])?;
    std::thread::sleep(Duration::from_millis(500));
    let logical = parse_display_events(&read(&ctx.burl_out))
        .pop()
        .ok_or("no display: line")?
        .logical;
    let (wa, a, nums) = ctx.settled("A-1x-before", d1.scale, logical)?;
    eprintln!(
        "ok   A: {}x{} pt on the 1x display, fills window: {nums}",
        wa.w, wa.h
    );

    // 1x -> 2x: the `--drag` checks; the 2x frame is framed differently, so no comparison with A.
    let (_, logical2) = ctx.drag(&d1, &d2, "1x -> 2x")?;
    let (w2, _, nums) = ctx.settled("2x-between", d2.scale, logical2)?;
    eprintln!(
        "ok   drag 1x -> 2x: {}x{} pt, scene fills window: {nums}",
        w2.w, w2.h
    );

    // 2x -> 1x, then B.
    let (_, logical_b) = ctx.drag(&d2, &d1, "2x -> 1x")?;
    let (wb, b, nums) = ctx.settled("B-1x-after", d1.scale, logical_b)?;
    eprintln!(
        "ok   drag 2x -> 1x: {}x{} pt, scene fills window: {nums}",
        wb.w, wb.h
    );

    check_size_kept((wa.w, wa.h), (wb.w, wb.h), logical, logical_b)
        .map_err(|e| format!("round trip: {e}"))?;
    let r = check_roundtrip(&a, &b).map_err(|e| format!("round trip: {e}"))?;
    eprintln!(
        "ok   round trip: B equals A, MAE {:.3} (limit {MAX_ROUNDTRIP_MAE}); controls shifted {:.3}, blurred {:.3}",
        r.mae, r.shifted_mae, r.blurred_mae
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dup(i: &Image) -> Image {
        Image {
            w: i.w,
            h: i.h,
            rgb: i.rgb.clone(),
        }
    }

    /// Thin lines and a diagonal on a dark field, like glyphs.
    fn scene(w: usize, h: usize) -> Image {
        let mut rgb = vec![20; w * h * 3];
        for y in 0..h {
            for x in 0..w {
                if x % 8 == 0 || (x as i64 - y as i64).abs() < 2 {
                    rgb[(y * w + x) * 3..][..3].fill(235);
                }
            }
        }
        Image { w, h, rgb }
    }

    #[test]
    fn the_same_frame_passes_with_both_controls_above_twice_the_limit() {
        let a = scene(128, 96);
        let r = check_roundtrip(&a, &dup(&a)).unwrap();
        assert_eq!(r.mae, 0.0);
        assert!(r.shifted_mae.max(r.blurred_mae) >= MAX_ROUNDTRIP_MAE * MIN_CONTROL_FACTOR);
    }

    #[test]
    fn capture_noise_below_the_limit_passes() {
        let a = scene(128, 96);
        let mut b = dup(&a);
        b.rgb[300] = b.rgb[300].wrapping_add(10);
        assert!(check_roundtrip(&a, &b).is_ok());
    }

    #[test]
    fn a_shifted_or_blurred_b_fails() {
        let a = scene(128, 96);
        let e = check_roundtrip(&a, &shifted(&a)).unwrap_err();
        assert!(e.contains("different frame"), "{e}");
        let blur = pixels::blurred(&a);
        let e = check_roundtrip(&a, &blur).unwrap_err();
        assert!(e.contains("different frame"), "{e}");
    }

    #[test]
    fn unequal_frame_sizes_are_rejected_before_any_pixel_compare() {
        let e = check_roundtrip(&scene(128, 96), &scene(64, 48)).unwrap_err();
        assert!(e.contains("size differs"), "{e}");
    }

    #[test]
    fn a_flat_frame_is_vacuous_not_a_pass() {
        let flat = Image {
            w: 64,
            h: 48,
            rgb: vec![7; 64 * 48 * 3],
        };
        let e = check_roundtrip(&flat, &dup(&flat)).unwrap_err();
        assert!(e.contains("vacuous"), "{e}");
    }
}
