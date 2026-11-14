//! `cargo xtask display-harness --real-panel` (#210): box 79's real-Retina crispness check on the
//! machine's own main display. No virtual display, nothing is moved or pressed; burl opens where
//! it opens, `screencapture -l` takes the window, and the pure checks in [`crate::pixels`] and
//! [`crate::edges`] judge it. macOS only; never run by hosted CI (only its pure parts are tested).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::display_harness::{
    check_capture, parse_display_events, parse_window, poll, read, run, Proc, Window,
};
use crate::{edges, pixels};

/// The main display as `panel.m` printed it.
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    pub scale: f64,
    pub bounds: (u32, u32),
    pub pixels: (u32, u32),
    pub name: String,
}

pub fn parse_panel(line: &str) -> Option<Panel> {
    let rest = line.trim().strip_prefix("panel ")?;
    let f = |k: &str| rest.split_whitespace().find_map(|t| t.strip_prefix(k));
    let size = |s: &str| {
        let (w, h) = s.split_once('x')?;
        Some((w.parse().ok()?, h.parse().ok()?))
    };
    Some(Panel {
        scale: f("scale=")?.parse().ok()?,
        bounds: size(f("bounds=")?)?,
        pixels: size(f("pixels=")?)?,
        name: rest[rest.find("name=")? + 5..].to_string(),
    })
}

/// Only a 2x main display can answer box 79; say why when it is not one.
pub fn check_panel(p: &Panel) -> Result<(), String> {
    if p.scale != 2.0 {
        return Err(format!(
            "the main display ({}) has backingScaleFactor {}, not 2: --real-panel needs a Retina \
             panel as the main display (System Settings > Displays, not a mirrored or scaled-to-1x \
             mode)",
            p.name, p.scale
        ));
    }
    Ok(())
}

fn tool_out(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into())
}

pub fn run_real_panel(root: &Path) -> Result<(), String> {
    let dir = root.join("target/display-harness-real-panel");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let src = root.join("tools/display-harness");
    for (m, bin) in [("panel.m", "panel"), ("wid.m", "wid")] {
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
    // Refuse before building anything: a headless or non-Retina machine cannot answer.
    let out = Command::new(dir.join("panel"))
        .output()
        .map_err(|e| format!("panel: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "no usable display: {} (--real-panel needs a logged-in Mac session with a Retina main display)",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let panel = text
        .lines()
        .find_map(parse_panel)
        .ok_or(format!("unreadable display description {text:?}"))?;
    check_panel(&panel)?;
    eprintln!("panel: {panel:?}");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    eprintln!("+ cargo build --release -p burl");
    run(Command::new(&cargo)
        .args(["build", "--release", "-p", "burl"])
        .current_dir(root))?;
    let target =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let burl_bin = target.join("release/burl");

    let burl_out = dir.join("burl.out");
    let log = root.join("tests/golden/sim/small-repo.log");
    let f = std::fs::File::create(&burl_out).map_err(|e| e.to_string())?;
    let _burl = Proc(
        Command::new(&burl_bin)
            .arg(&log)
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
                "--disable-auto-skip",
                "--disable-auto-rotate",
            ])
            .args([
                "--file-idle-time",
                "0",
                "--time-scale",
                "1",
                "--hide",
                "mouse",
            ])
            .args(["--prng", "glibc", "--log-level", "debug"])
            .stdout(f.try_clone().map_err(|e| e.to_string())?)
            .stderr(f)
            .stdin(Stdio::null())
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
    let start = poll("burl's `display: event=start` line", 30, || {
        parse_display_events(&read(&burl_out)).first().cloned()
    })?;
    if start.scale != 2.0 || start.physical != (640, 360) {
        return Err(format!(
            "burl itself reports {start:?}, expected scale 2 and a 640x360 px window"
        ));
    }
    // Let the scene build up; the capture does not need a still frame (edge widths do not depend
    // on where the avatars are).
    std::thread::sleep(Duration::from_secs(8));
    let w = window().ok_or("burl's window vanished (minimized or on another Space?)")?;
    if w.x < 0 || w.y < 0 || w.x >= panel.bounds.0 as i32 || w.y >= panel.bounds.1 as i32 {
        return Err(format!(
            "burl's window at ({}, {}) is not on the main display ({}x{} pt): another display or \
             Space? Nothing is moved by this command",
            w.x, w.y, panel.bounds.0, panel.bounds.1
        ));
    }
    let png = dir.join("window.png");
    run(Command::new("screencapture")
        .args(["-x", "-o", "-l", &w.id.to_string()])
        .arg(&png))
    .map_err(|e| format!("{e} (Screen Recording permission for the terminal?)"))?;
    let img = crate::ssim::decode(&std::fs::read(&png).map_err(|e| e.to_string())?)?;
    let mut colours = std::collections::HashSet::new();
    for px in img.rgb.as_chunks::<3>().0 {
        colours.insert(*px);
        if colours.len() >= 16 {
            break;
        }
    }
    check_capture((img.w as u32, img.h as u32), (w.w, w.h), 2.0, colours.len())?;
    let logical = parse_display_events(&read(&burl_out))
        .last()
        .ok_or("no display: line")?
        .logical;
    let content = pixels::content(&img, logical, 2.0)?;
    crate::ssim::write_png(
        &dir.join("window-content.png"),
        content.w,
        content.h,
        &content.rgb,
    )?;

    let crisp = pixels::check_crisp(&content);
    let edge = edges::check_edges(&content);
    let mut report = format!(
        "real-panel report\nmacOS {} (build {}), {}\ndisplay: {} ({}x{} pt, {}x{} px, backingScaleFactor {})\n\
         capture: {}x{} px for a {}x{} pt window, content {}x{} px\n",
        tool_out("sw_vers", &["-productVersion"]),
        tool_out("sw_vers", &["-buildVersion"]),
        tool_out("uname", &["-m"]),
        panel.name,
        panel.bounds.0,
        panel.bounds.1,
        panel.pixels.0,
        panel.pixels.1,
        panel.scale,
        img.w,
        img.h,
        w.w,
        w.h,
        content.w,
        content.h
    );
    report += &match &crisp {
        Ok((c, control)) => format!(
            "crispness {c:.2} (limit {}, blurred copy {control:.2}): PASS\n",
            pixels::MIN_CRISPNESS
        ),
        Err(e) => format!("crispness: FAIL: {e}\n"),
    };
    report += &match &edge {
        Ok(s) => format!(
            "edges: {} edges, median rise {:.2} px (limit {}), {:.0}% within {} px (min {:.0}%): PASS\n",
            s.edges,
            s.median,
            edges::MAX_MEDIAN_PX,
            s.sharp * 100.0,
            edges::SHARP_PX,
            edges::MIN_SHARP * 100.0
        ),
        Err(e) => format!("edges: FAIL: {e}\n"),
    };
    eprint!("{report}");
    std::fs::write(dir.join("report.txt"), &report).map_err(|e| e.to_string())?;
    eprintln!("capture and report kept in {}", dir.display());
    crisp.map(|_| ()).map_err(|e| format!("crispness: {e}"))?;
    edge.map(|_| ()).map_err(|e| format!("edges: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str =
        "panel scale=2 bounds=1512x982 pixels=3024x1964 name=Built-in Liquid Retina XDR Display\n";

    #[test]
    fn parses_the_helper_line_with_a_spaced_name() {
        let p = parse_panel(LINE).unwrap();
        assert_eq!(
            p,
            Panel {
                scale: 2.0,
                bounds: (1512, 982),
                pixels: (3024, 1964),
                name: "Built-in Liquid Retina XDR Display".into()
            }
        );
        assert!(parse_panel("display name=2x").is_none());
        assert!(parse_panel("panel scale=2").is_none());
    }

    #[test]
    fn a_retina_panel_passes_and_a_1x_one_is_refused_with_its_name() {
        let mut p = parse_panel(LINE).unwrap();
        assert_eq!(check_panel(&p), Ok(()));
        p.scale = 1.0;
        let e = check_panel(&p).unwrap_err();
        assert!(e.contains("not 2") && e.contains("Built-in"), "{e}");
    }
}
