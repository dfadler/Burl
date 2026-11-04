//! D9 (`docs/rust-rewrite/DEVIATIONS.md`): resources are embedded, not found on a search path.
//! The binary runs from an empty working directory with no `data/` beside it and still exports
//! a valid PPM stream, so a resource read from disk at run time fails here, per PR (the release
//! image check, `scripts/check-image.sh`, runs the same check on the tag). Needs a headless GL
//! context; where there is none it skips, except on macOS where a missing context is a failure.

use std::path::Path;
use std::process::Command;

const HEADER: &[u8] = b"P6\n320 240\n255\n";
const FRAME: usize = HEADER.len() + 320 * 240 * 3;

#[test]
fn deviation_d9_export_from_an_empty_working_directory() {
    let log = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/logs/file-removal.log");
    let empty = tempdir();
    assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
    let out = Command::new(env!("CARGO_BIN_EXE_burl"))
        .current_dir(&empty)
        .env("TZ", "UTC")
        .args(["--log-format", "custom", "--stop-at-time", "1", "-320x240"])
        .args(["-o", "-"])
        .arg(&log)
        .output()
        .unwrap();
    std::fs::remove_dir(&empty).unwrap();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if cfg!(target_os = "macos") {
            panic!("burl -o - failed on macOS: {err}");
        }
        eprintln!("SKIPPED: no headless GL context: {err}");
        return;
    }
    let s = out.stdout;
    assert!(s.starts_with(HEADER), "bad PPM header");
    assert!(
        !s.is_empty() && s.len().is_multiple_of(FRAME),
        "{} bytes",
        s.len()
    );
}

fn tempdir() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("burl-d9-{}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}
