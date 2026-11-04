//! G1 (`docs/rust-rewrite/DEVIATIONS.md`): Gource dies silently on SIGPIPE (exit 141 from a
//! shell) when the PPM consumer goes away; Burl's runtime ignores SIGPIPE, so the failed write
//! ends the export with an error message and exit 1. Needs a headless GL context; where there is
//! none it skips, except on macOS where a missing context is a failure.

use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn deviation_g1_closed_pipe_ends_the_export_with_an_error() {
    let log = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/logs/file-removal.log");
    let mut child = Command::new(env!("CARGO_BIN_EXE_burl"))
        .env("TZ", "UTC")
        .args(["--log-format", "custom", "--stop-at-time", "5", "-320x240"])
        .args(["-o", "-"])
        .arg(&log)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Close the read end before the first frame (a 230 KB write) is attempted.
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.contains("Broken pipe") && !cfg!(target_os = "macos") {
        eprintln!("SKIPPED: no headless GL context: {err}");
        return;
    }
    assert_eq!(
        out.status.code(),
        Some(1),
        "not SIGPIPE (141), not success: {err}"
    );
    assert!(err.contains("Broken pipe"), "{err}");
}
