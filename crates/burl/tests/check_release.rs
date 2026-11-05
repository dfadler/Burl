//! `scripts/check-release.sh` on temporary sign-off files: a complete sign-off passes, and each way
//! a proof named in the checklist can go stale (a renamed test, a missing file, reworded text, no
//! table at all) fails it. The last test runs the gate on the real checklist, so a renamed test
//! breaks CI now and not only at tag time.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("burl-gate-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run the gate for the workspace version on `checklist` against a register with a known hash.
/// `signoff_hash` overrides the recorded hash (None = the right one).
fn gate(tag: &str, checklist: &str, signoff_hash: Option<&str>) -> Output {
    let dir = scratch(tag);
    let register = dir.join("DEVIATIONS.md");
    std::fs::write(&register, "register\n").unwrap();
    let hash = String::from_utf8(
        Command::new("git")
            .arg("hash-object")
            .arg(&register)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let hash = signoff_hash.unwrap_or(hash.trim());
    let body = checklist
        .replace("{VERSION}", env!("CARGO_PKG_VERSION"))
        .replace("{HASH}", hash);
    let file = dir.join("CHECKLIST.md");
    std::fs::write(&file, body).unwrap();
    let out = Command::new("bash")
        .arg(root().join("scripts/check-release.sh"))
        .arg(format!("v{}", env!("CARGO_PKG_VERSION")))
        .env("SIGNOFF_FILE", &file)
        .env("REGISTER_FILE", &register)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(dir);
    out
}

const SIGNED: &str = "\
- [x] a manual item

| Item | Proof |
|---|---|
| this test | `crates/burl/tests/check_release.rs::a_complete_signoff_passes` |
| a window check | `scripts/window-in-container.sh::Esc quits cleanly` |

version: {VERSION}
deviation-register: {HASH}
macos-retina: ok
macos-non-retina: ok
linux-x11: ok
";

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn a_complete_signoff_passes() {
    let o = gate("ok", SIGNED, None);
    assert!(o.status.success(), "{}", stderr(&o));
}

#[test]
fn a_renamed_test_fails_the_gate() {
    let c = SIGNED.replace("a_complete_signoff_passes", "a_test_that_was_renamed");
    let o = gate("renamed", &c, None);
    assert!(!o.status.success());
    assert!(stderr(&o).contains("proof test gone"), "{}", stderr(&o));
}

#[test]
fn a_missing_proof_file_fails_the_gate() {
    let c = SIGNED.replace(
        "crates/burl/tests/check_release.rs",
        "crates/burl/tests/nope.rs",
    );
    let o = gate("nofile", &c, None);
    assert!(!o.status.success());
    assert!(stderr(&o).contains("proof file missing"), "{}", stderr(&o));
}

#[test]
fn reworded_check_text_fails_the_gate() {
    let c = SIGNED.replace("Esc quits cleanly", "Esc quits tidily");
    let o = gate("reworded", &c, None);
    assert!(!o.status.success());
    assert!(stderr(&o).contains("proof text gone"), "{}", stderr(&o));
}

#[test]
fn a_checklist_with_no_proofs_fails_the_gate() {
    let c = "- [x] a manual item\n\nversion: {VERSION}\ndeviation-register: {HASH}\nmacos-retina: ok\nmacos-non-retina: ok\nlinux-x11: ok\n";
    let o = gate("noproofs", c, None);
    assert!(!o.status.success());
    assert!(stderr(&o).contains("no automated-check proofs"));
}

#[test]
fn the_existing_signoff_checks_still_apply() {
    let unticked = SIGNED.replace("- [x]", "- [ ]");
    assert!(!gate("unticked", &unticked, None).status.success());
    let todo = SIGNED.replace("linux-x11: ok", "linux-x11: TODO");
    assert!(!gate("todo", &todo, None).status.success());
    assert!(!gate("hash", SIGNED, Some("deadbeef")).status.success());
}

/// The real checklist: every proof it names must still exist (the unsigned sign-off is expected
/// to fail, for other reasons).
#[test]
fn every_proof_in_the_real_checklist_still_exists() {
    let o = Command::new("bash")
        .arg(root().join("scripts/check-release.sh"))
        .arg(format!("v{}", env!("CARGO_PKG_VERSION")))
        .output()
        .unwrap();
    let err = stderr(&o);
    assert!(!err.contains("proof "), "{err}");
    assert!(!err.contains("no automated-check proofs"), "{err}");
}
