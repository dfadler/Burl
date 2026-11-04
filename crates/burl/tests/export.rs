//! `-o -` frame export through the real binary (audit Gap 14): the PPM stream's shape, `-r`, and
//! one behavioural test per option the audit found with no test naming it. Each option test
//! renders the same fixed run twice, changing only that option, and requires different pixels:
//! it fails if the option's value stops reaching the renderer. Needs a headless GL context
//! (CGL on macOS, EGL elsewhere); where there is none it skips, except on macOS where a
//! missing context is a failure.

use std::path::{Path, PathBuf};
use std::process::Command;

const W: usize = 320;
const H: usize = 180;
const HEADER: &[u8] = b"P6\n320 180\n255\n";
/// Each frame carries its own header, as Gource's `PPMWriter` writes it.
const FRAME: usize = HEADER.len() + W * H * 3;

fn sim_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/sim")
}

/// The PPM stream of the small-repo fixture run for `frames` ticks, or None without a GL context.
/// `--prng glibc` and the fixed seeds make two runs with the same arguments identical.
fn stream(extra: &[&str], frames: usize) -> Option<Vec<u8>> {
    let out = Command::new(env!("CARGO_BIN_EXE_burl"))
        .current_dir(sim_dir())
        .env("TZ", "UTC")
        .args([
            "small-repo.log",
            "--log-format",
            "custom",
            "-320x180",
            "-o",
            "-",
        ])
        .args([
            "--seconds-per-day",
            "1",
            "--stop-at-time",
            "10",
            "--hash-seed",
            "31",
        ])
        .args([
            "--disable-auto-skip",
            "--disable-auto-rotate",
            "--hide",
            "mouse",
        ])
        .args([
            "--file-idle-time",
            "0",
            "--time-scale",
            "1",
            "--prng",
            "glibc",
        ])
        .args(["--debug-dump-max-frames", &frames.to_string()])
        .args(extra)
        .output()
        .unwrap();
    if out.status.success() {
        return Some(out.stdout);
    }
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !cfg!(target_os = "macos"),
        "burl -o - failed on macOS: {err}"
    );
    eprintln!("SKIPPED: no headless GL context: {err}");
    None
}

/// Run twice with only `a` and `b` differing and require different frames.
fn differs(a: &[&str], b: &[&str], frames: usize) {
    let (Some(x), Some(y)) = (stream(a, frames), stream(b, frames)) else {
        return;
    };
    assert_eq!(x.len(), y.len(), "{a:?} vs {b:?}: stream length");
    assert!(x != y, "{a:?} and {b:?} rendered the same {frames} frames");
}

const CAPTIONS: [&str; 4] = [
    "--caption-file",
    "small-repo-captions.txt",
    "--caption-duration",
    "2",
];

#[test]
fn ppm_stream_to_stdout_stops_at_max_frames() {
    let Some(s) = stream(&[], 3) else { return };
    assert!(s.starts_with(HEADER), "{:?}", &s[..20]);
    assert_eq!(s.len(), 3 * FRAME);
    // the default background is not pure black: a drawn frame, not an empty buffer
    assert!(s[HEADER.len()..].iter().any(|&b| b != 0));
}

/// `-r 30` is the export frame rate: Gource writes every (frameskip + 1)-th tick, so a run
/// holds half the frames of the `-r 60` run (here the run is the fixture's first 90 ticks).
#[test]
fn output_framerate_thins_the_stream() {
    let (Some(full), Some(half)) = (stream(&["-r", "60"], 90), stream(&["-r", "30"], 90)) else {
        return;
    };
    assert_eq!((full.len(), full.len() % FRAME), (90 * FRAME, 0));
    assert_eq!((half.len(), half.len() % FRAME), (45 * FRAME, 0));
}

#[test]
fn bloom_multiplier_changes_the_render() {
    differs(
        &["--bloom-multiplier", "1"],
        &["--bloom-multiplier", "6"],
        120,
    );
}

#[test]
fn filename_colour_changes_the_render() {
    differs(
        &["--filename-colour", "FFFFFF"],
        &["--filename-colour", "FF0000"],
        120,
    );
}

#[test]
fn file_font_size_changes_the_render() {
    differs(
        &["--file-font-size", "14"],
        &["--file-font-size", "30"],
        120,
    );
}

#[test]
fn dir_font_size_changes_the_render() {
    differs(&["--dir-font-size", "14"], &["--dir-font-size", "30"], 120);
}

#[test]
fn caption_size_changes_the_render() {
    let with = |size: &'static str| -> Vec<&'static str> {
        CAPTIONS
            .iter()
            .copied()
            .chain(["--caption-size", size])
            .collect()
    };
    differs(&with("16"), &with("40"), 180);
}
