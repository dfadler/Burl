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

/// Script mode unless the fixture picks a generator itself with `--prng` (dump-run.sh does the same).
fn rand_args(root: &Path, extra: &str) -> Vec<std::ffi::OsString> {
    if extra.split_whitespace().any(|a| a == "--prng") {
        return Vec::new();
    }
    vec![
        "--rand-script".into(),
        root.join("reference/gource-cpp-dump/tests/dump/rand-script.txt")
            .into(),
    ]
}

/// Burl's dump of `name`, run as dump-run.sh runs the fork: a log fixture with the fixed
/// per-repo inputs (no `-o`: the dump flag implies the fixed tick), a conf fixture
/// (`@conf:PATH` first in the `.args`, PATH relative to `tests/`) with the display pinned only.
/// `@auto-rotate` leaves auto-rotate on. TZ is UTC, as in the image.
fn burl_dump(root: &Path, name: &str, out: &Path) -> String {
    let dir = root.join("tests/golden/sim");
    let args = std::fs::read_to_string(dir.join(format!("{name}.args"))).unwrap();
    let mut extra: Vec<&str> = args.split_whitespace().collect();
    let conf = extra
        .first()
        .and_then(|a| a.strip_prefix("@conf:"))
        .map(|c| {
            extra.remove(0);
            root.join("tests").join(c)
        });
    let auto_rotate = extra.contains(&"@auto-rotate");
    extra.retain(|a| *a != "@auto-rotate");
    let extra_line = extra.join(" ");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_burl"));
    cmd.env("TZ", "UTC");
    match &conf {
        Some(conf) => {
            cmd.current_dir(conf.parent().unwrap())
                .arg(conf.file_name().unwrap())
                .args(["-1280x720", "-r", "60"]);
        }
        None => {
            cmd.current_dir(&dir)
                .arg(format!("{name}.log"))
                .args(["--log-format", "custom", "-1280x720", "-r", "60"])
                .args(["--seconds-per-day", "1", "--stop-at-time", "10"])
                .args(["--hash-seed", "31", "--disable-auto-skip"])
                .args(["--file-idle-time", "0", "--time-scale", "1"])
                .args(["--hide", "mouse"]);
            if !auto_rotate {
                cmd.arg("--disable-auto-rotate");
            }
        }
    }
    let status = cmd
        .args(rand_args(root, &extra_line))
        .arg("--debug-dump-state")
        .arg(out)
        .args(["--debug-dump-max-frames", "4000"])
        .args(&extra)
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
        report.compared > 50,
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

/// Slice 3 fixtures: each exercises one sim setting the first five did not.
#[test]
fn sim_gate_small_repo_glibc() {
    gate("small-repo-glibc");
}

#[test]
fn sim_gate_small_repo_max_files() {
    gate("small-repo-max-files");
}

#[test]
fn sim_gate_small_repo_elasticity() {
    gate("small-repo-elasticity");
}

#[test]
fn sim_gate_small_repo_tuning() {
    gate("small-repo-tuning");
}

#[test]
fn sim_gate_small_repo_stop_position() {
    gate("small-repo-stop-position");
}

#[test]
fn sim_gate_auto_skip() {
    gate("auto-skip");
}

#[test]
fn sim_gate_time_travel() {
    gate("time-travel");
}

#[test]
fn sim_gate_no_time_travel() {
    gate("no-time-travel");
}

/// Phase 4 (ticket #18): captions (queued, active, fade, expiry; a BOM line, a date-string
/// stamp, ignored lines), auto-rotate left on over a real log, and the multi-section shell.
#[test]
fn sim_gate_small_repo_captions() {
    gate("small-repo-captions");
}

#[test]
fn sim_gate_real_auto_rotate() {
    gate("real-auto-rotate");
}

/// `tests/test.conf` unchanged (eight sections), every section's path replaced by one pinned log.
#[test]
fn sim_gate_test_conf() {
    gate("test-conf");
}

/// Purpose-built conf: no stop time (the 60-second rule) and options in section 1 that must not
/// reach section 2.
#[test]
fn sim_gate_multi_reset() {
    gate("multi-reset");
}

/// Look-around and backreference filters (#18 item 5): PCRE2 only. The fixture and the gate run
/// in the `pcre2` leg; the default build rejects the patterns at import (register row G4).
#[cfg(feature = "pcre2")]
#[test]
fn sim_gate_pcre2_filters() {
    gate("pcre2-filters");
}

#[cfg(not(feature = "pcre2"))]
#[test]
fn deviation_g4_default_build_rejects_pcre2_only_filters() {
    let root = repo_root();
    let dir = root.join("tests/golden/sim");
    let out = Command::new(env!("CARGO_BIN_EXE_burl"))
        .current_dir(&dir)
        .args(["pcre2-filters.conf", "--debug-dump-state", "/dev/null"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "burl: pcre2-filters.conf, line 3: invalid file-filter regular expression\nTry 'burl --help' for more information.\n\n"
    );
}
