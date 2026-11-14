//! `cargo xtask crisp-control`: a real negative control for the crispness check. `lowres.m` draws
//! the same test card into a native 2x drawable and into a 1x drawable that the window server
//! magnifies on the 2x virtual display; both captures go through the existing
//! `pixels::check_crisp`. Answers what `screencapture -l` returns for a non-hi-res window and
//! whether the real upscale scores below `MIN_CRISPNESS`. macOS only; not part of `cargo xtask ci`.
//! See `tools/display-harness/README.md`.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::display_harness::{parse_display, parse_window, Display, Window};
use crate::pixels;
use crate::ssim::Image;

/// The window's content size in points (what `lowres.m` opens).
const CONTENT_PT: (u32, u32) = (640, 360);

// ---- pure parts (unit-tested below) ----

/// `lowres` takes `YES` (native drawable) or `NO` (1x drawable).
pub fn lowres_arg(native: bool) -> &'static str {
    if native {
        "YES"
    } else {
        "NO"
    }
}

/// `crisp-control` takes no arguments.
pub fn parse_args(args: &[String]) -> Result<(), String> {
    match args {
        [] => Ok(()),
        _ => Err("usage: cargo xtask crisp-control".into()),
    }
}

/// Capture pixels per window point (1 or 2), or `None` for anything else.
pub fn capture_scale(capture_w: usize, window_w: u32) -> Option<f64> {
    [1.0, 2.0]
        .into_iter()
        .find(|s| (f64::from(window_w) * s).round() as usize == capture_w)
}

/// What one capture measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score {
    /// Capture size in pixels, title bar included.
    pub capture: (usize, usize),
    pub scale: f64,
    pub crispness: f64,
    /// `check_crisp` accepted it.
    pub pass: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Finding {
    /// The compositor upscale scored below the limit: the control works.
    Rejected,
    /// The compositor upscale scored at or above the limit: the synthetic control was not
    /// representative; nothing is weakened, the numbers are the finding.
    NotRejected,
    /// `screencapture` returned the 1x backing, so no upscale is in the capture to judge.
    Backing1x,
}

/// Only what is true is asserted: the native render must be a 2x capture and pass `check_crisp`.
/// The low-res result is reported, not required, because what the compositor does is the question.
pub fn judge(native: &Score, low: &Score) -> Result<Finding, String> {
    if native.scale != 2.0 {
        return Err(format!(
            "native capture is {:?} px, scale {} not 2: the window was not on the 2x display, or the \
             drawable was not hi-res",
            native.capture, native.scale
        ));
    }
    if !native.pass {
        return Err(format!(
            "the native 2x card is not crisp (crispness {:.2}, limit {}): the card or the check is wrong, \
             so the low-res number means nothing",
            native.crispness,
            pixels::MIN_CRISPNESS
        ));
    }
    Ok(if low.scale == 1.0 {
        Finding::Backing1x
    } else if low.pass {
        Finding::NotRejected
    } else {
        Finding::Rejected
    })
}

// ---- process plumbing (small copies of display_harness's private helpers) ----

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

/// Opens `lowres` (native or 1x drawable), moves it onto `d2`, captures it and scores the content.
fn measure(dir: &Path, d2: &Display, native: bool) -> Result<Score, String> {
    let name = if native { "native" } else { "lowres" };
    let out = dir.join(format!("{name}.out"));
    let file = std::fs::File::create(&out).map_err(|e| e.to_string())?;
    let _app = Proc(
        Command::new(dir.join("lowres"))
            .arg(lowres_arg(native))
            .stdout(file)
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("lowres: {e}"))?,
    );
    let window = || -> Option<Window> {
        let o = Command::new(dir.join("wid")).arg("lowres").output().ok()?;
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .find_map(parse_window)
    };
    poll("lowres's window", 30, window)?;
    run(Command::new("osascript").args([
        "-e",
        &format!(
            "tell application \"System Events\" to tell (first process whose name is \"lowres\") to set position of window 1 to {{{}, {}}}",
            d2.x + 100,
            d2.y + 100
        ),
    ]))?;
    let on_d2 = |w: &Window| w.x >= d2.x && w.x < d2.x + d2.w && w.y >= d2.y && w.y < d2.y + d2.h;
    poll("the window to land on the 2x display", 20, || {
        window().filter(on_d2)
    })?;
    // Let the surface follow the move, then take two identical captures in a row.
    std::thread::sleep(Duration::from_secs(2));
    let png = dir.join(format!("{name}.png"));
    let mut prev: Option<Image> = None;
    let mut last = String::new();
    let (img, win) = poll("two identical captures", 30, || {
        let w = window()?;
        let ok = Command::new("screencapture")
            .args(["-x", "-o", "-l", &w.id.to_string()])
            .arg(&png)
            .status()
            .ok()?
            .success();
        if !ok {
            last = "screencapture failed (Screen Recording permission?)".into();
            return None;
        }
        let img = crate::ssim::decode(&std::fs::read(&png).ok()?).ok()?;
        let same = prev.as_ref().is_some_and(|p| p.rgb == img.rgb);
        prev = Some(Image {
            w: img.w,
            h: img.h,
            rgb: img.rgb.clone(),
        });
        same.then_some((img, w))
    })
    .map_err(|e| format!("{name}: {e}: {last}"))?;
    let scale = capture_scale(img.w, win.w).ok_or(format!(
        "{name}: capture is {}x{} px for a {}x{} pt window: neither 1x nor 2x",
        img.w, img.h, win.w, win.h
    ))?;
    let content = pixels::content(&img, CONTENT_PT, scale)?;
    crate::ssim::write_png(
        &dir.join(format!("{name}-content.png")),
        content.w,
        content.h,
        &content.rgb,
    )?;
    let crispness = pixels::crispness(&content)?;
    let pass = pixels::check_crisp(&content).is_ok();
    eprintln!(
        "{name}: window {}x{} pt, capture {}x{} px (scale {scale}), content {}x{}, sharpness {:.3}, \
         crispness {crispness:.2} (limit {}, {}); lowres reported: {}",
        win.w,
        win.h,
        img.w,
        img.h,
        content.w,
        content.h,
        pixels::sharpness(&content)?,
        pixels::MIN_CRISPNESS,
        if pass { "passes" } else { "fails" },
        read(&out).trim().replace('\n', "; ")
    );
    Ok(Score {
        capture: (img.w, img.h),
        scale,
        crispness,
        pass,
    })
}

pub fn run_crisp_control(root: &Path) -> Result<(), String> {
    let dir = root.join("target/crisp-control");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let src = root.join("tools/display-harness");
    for (m, bin, fw) in [
        ("vd.m", "vd", "CoreGraphics"),
        ("wid.m", "wid", "CoreGraphics"),
        ("lowres.m", "lowres", "OpenGL"),
    ] {
        run(Command::new("clang")
            .args([
                "-fobjc-arc",
                "-Wno-deprecated-declarations",
                "-framework",
                "Cocoa",
                "-framework",
                fw,
            ])
            .arg(src.join(m))
            .arg("-o")
            .arg(dir.join(bin)))?;
    }
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
            "the display helper exited ({st}) before printing `ready`"
        )))
    })??;
    let d2 = text
        .lines()
        .filter_map(parse_display)
        .find(|d| d.name == "2x")
        .ok_or("helper reported no 2x display")?;
    eprintln!("2x display: {d2:?}");

    let native = measure(&dir, &d2, true)?;
    let low = measure(&dir, &d2, false)?;
    let finding = judge(&native, &low)?;
    eprintln!(
        "crisp-control: native crispness {:.2} (capture {:?}), low-res crispness {:.2} (capture {:?}), limit {}",
        native.crispness,
        native.capture,
        low.crispness,
        low.capture,
        pixels::MIN_CRISPNESS
    );
    eprintln!(
        "crisp-control: {}",
        match finding {
            Finding::Rejected => "the compositor-upscaled capture scores below the limit: the check rejects a real upscale",
            Finding::NotRejected => "FINDING: the compositor-upscaled capture scores AT OR ABOVE the limit: the check would not reject it",
            Finding::Backing1x => "FINDING: screencapture returned the 1x backing of the low-res window, not an upscale",
        }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(scale: f64, crispness: f64, pass: bool) -> Score {
        Score {
            capture: ((640.0 * scale) as usize, 750),
            scale,
            crispness,
            pass,
        }
    }

    #[test]
    fn lowres_gets_yes_for_native_and_no_for_one_x() {
        assert_eq!((lowres_arg(true), lowres_arg(false)), ("YES", "NO"));
    }

    #[test]
    fn takes_no_arguments() {
        assert_eq!(parse_args(&[]), Ok(()));
        assert!(parse_args(&["--high-dpi".to_string()]).is_err());
    }

    #[test]
    fn capture_scale_is_one_or_two_points_per_pixel() {
        assert_eq!(capture_scale(1280, 640), Some(2.0));
        assert_eq!(capture_scale(640, 640), Some(1.0));
        assert_eq!(capture_scale(960, 640), None);
    }

    #[test]
    fn native_must_be_a_crisp_2x_capture() {
        let low = score(2.0, 1.3, false);
        assert_eq!(judge(&score(2.0, 2.9, true), &low), Ok(Finding::Rejected));
        assert!(judge(&score(2.0, 1.4, false), &low).is_err());
        assert!(judge(&score(1.0, 2.9, true), &low).is_err());
    }

    #[test]
    fn the_low_res_result_is_reported_not_forced() {
        let native = score(2.0, 2.9, true);
        assert_eq!(
            judge(&native, &score(2.0, 2.5, true)),
            Ok(Finding::NotRejected)
        );
        assert_eq!(
            judge(&native, &score(1.0, 2.9, true)),
            Ok(Finding::Backing1x)
        );
    }
}
