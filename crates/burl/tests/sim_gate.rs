//! Simulation parity gate (ticket #11, slice 1): run Burl on each fixture in
//! `tests/golden/sim/` with the fixed inputs of `reference/gource-cpp-dump/scripts/dump-run.sh`
//! and compare its `--debug-dump-state` with the fork's committed golden through the xtask
//! comparator. Goldens change only in a `golden-refresh` PR (`regen-sim.sh`).

use std::path::{Path, PathBuf};
use std::process::Command;
use xtask::simcmp::{self, Floats, Options};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn options(root: &Path) -> Options {
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    let ids = simcmp::deviation_ids(&read("docs/rust-rewrite/DEVIATIONS.md"));
    Options {
        floats: Floats::Exact,
        allow: simcmp::parse_allowlist(&read(simcmp::ALLOWLIST), &ids).unwrap(),
    }
}

/// Burl's dump of `name`, run with dump-run.sh's fixed inputs (no `-o`: the dump flag implies
/// the fixed tick) and the fixture's extra arguments.
fn burl_dump(root: &Path, name: &str, out: &Path) -> String {
    let dir = root.join("tests/golden/sim");
    let extra = std::fs::read_to_string(dir.join(format!("{name}.args"))).unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_burl"))
        .current_dir(&dir)
        .arg(format!("{name}.log"))
        .args([
            "--log-format",
            "custom",
            "-1280x720",
            "-r",
            "60",
            "--seconds-per-day",
            "1",
        ])
        .args([
            "--stop-at-time",
            "10",
            "--hash-seed",
            "31",
            "--disable-auto-skip",
        ])
        .args([
            "--disable-auto-rotate",
            "--file-idle-time",
            "0",
            "--time-scale",
            "1",
        ])
        .args(["--hide", "mouse"])
        .arg("--rand-script")
        .arg(root.join("reference/gource-cpp-dump/tests/dump/rand-script.txt"))
        .arg("--debug-dump-state")
        .arg(out)
        .args(["--debug-dump-max-frames", "4000"])
        .args(extra.split_whitespace())
        .status()
        .unwrap();
    assert!(status.success(), "burl failed on {name}");
    std::fs::read_to_string(out).unwrap()
}

fn gate(name: &str) {
    let root = repo_root();
    let out =
        std::env::temp_dir().join(format!("burl-sim-gate-{}-{name}.jsonl", std::process::id()));
    let actual = burl_dump(&root, name, &out);
    let _ = std::fs::remove_file(&out);
    let golden =
        std::fs::read_to_string(root.join(format!("tests/golden/sim/{name}.jsonl"))).unwrap();
    let report = simcmp::compare(&golden, &actual, &options(&root)).unwrap();
    assert!(report.ok(), "{name}: {report}");
    assert!(
        report.compared > 100,
        "{name}: too few records compared ({})",
        report.compared
    );
}

#[test]
fn sim_gate_small_repo() {
    gate("small-repo");
}

#[test]
fn sim_gate_file_removal() {
    gate("file-removal");
}

#[test]
fn sim_gate_file_to_dir() {
    gate("file-to-dir");
}

#[test]
fn sim_gate_custom_dir_delete() {
    gate("custom-dir-delete");
}

/// The second input set of the spec (2.1): file idle fade, `--file-idle-time-at-end`, hash seed 7.
#[test]
fn sim_gate_small_repo_idle() {
    gate("small-repo-idle");
}

/// The pinned real-repo run (#11 item 7, risk R13): this repository's first 250 commits as a git
/// log, `--stop-at-time 60`, dumped every 30 frames.
#[test]
fn sim_gate_real_burl_git_250() {
    gate("real-burl-git-250");
}
