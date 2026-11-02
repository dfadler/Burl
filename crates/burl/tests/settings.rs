//! The main.cpp flow end to end: two argument passes, positional config detection, path
//! override, `--save-config`, up-front validation of every `[gource]` section (D1) and the
//! hidden test-only options. Expected texts are derived from the C++ source (not oracle runs).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

struct Run {
    stdout: String,
    stderr: String,
    code: i32,
}

fn run_in(dir: &Path, args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_burl"))
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    Run {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code().unwrap_or(-1),
    }
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("burl-settings-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn try_line() -> &'static str {
    "Try 'burl --help' for more information.\n\n"
}

#[test]
fn save_config_end_to_end() {
    let d = scratch("save");
    let out = d.join("out.conf");
    let r = run_in(
        &d,
        &[
            "--save-config",
            out.to_str().unwrap(),
            "-1280x720",
            "--ffp",
            "--stop-on-idle",
            "-s",
            "2",
        ],
    );
    assert_eq!((r.code, r.stdout.as_str(), r.stderr.as_str()), (0, "", ""));
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "[display]\nviewport=1280x720\n\n[gource]\nffp=true\nseconds-per-day=2\nstop-on-idle=true\n\n"
    );
    // an unwritable target is an error, not a silent exit
    let r = run_in(&d, &["--save-config", "/nonexistent/dir/x.conf"]);
    assert_eq!(
        (r.code, r.stderr),
        (
            1,
            format!(
                "burl: failed to write config to /nonexistent/dir/x.conf\n{}",
                try_line()
            )
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn test_conf_imports_all_eight_sections_and_cli_reaches_each() {
    let tests = repo_root().join("tests");
    let d = scratch("testconf");
    let out = d.join("out.conf");
    let r = run_in(
        &tests,
        &[
            "test.conf",
            "-s",
            "2",
            "--user-filter",
            "bob",
            "--save-config",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!((r.code, r.stderr.as_str()), (0, ""));
    let saved = std::fs::read_to_string(&out).unwrap();
    let blocks: Vec<&str> = saved.split("\n\n").filter(|b| !b.is_empty()).collect();
    assert_eq!(blocks.len(), 8);
    assert!(blocks
        .iter()
        .all(|b| b.starts_with("[gource]\n") && b.contains("\nseconds-per-day=2")));
    // multi-value CLI filters are appended to the file's own entries, in section order
    assert_eq!(blocks[0], "[gource]\nfile-idle-time=0.0\npath=logs/file-removal.log\nseconds-per-day=2\nstop-at-time=10.0\ntitle=File removal\nuser-filter=bob");
    assert!(blocks[2].contains("\nuser-filter=Andrew\nuser-filter=bob"));
    assert!(
        blocks[7].contains("caption-file=logs/utf8-caption.log")
            && blocks[7].contains("user-image-dir=images/")
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn deviation_d1_bad_later_section_fails_before_anything_runs() {
    let d = scratch("d1");
    let log = repo_root().join("tests/logs/utf8.log");
    let conf = d.join("bad.conf");
    std::fs::write(&conf, "[gource]\ntitle=fine\n\n[gource]\nstop-at-time=-1\n").unwrap();
    // C++ imports only the first section here and would write the log; Burl rejects the file
    let r = run_in(
        &d,
        &[
            "--output-custom-log",
            "-",
            "--log-format",
            "custom",
            conf.to_str().unwrap(),
            log.to_str().unwrap(),
        ],
    );
    assert_eq!(r.stdout, "");
    assert_eq!(r.code, 1);
    assert_eq!(
        r.stderr,
        format!(
            "burl: {}, line 5: invalid 'stop-at-time' value\n{}",
            conf.display(),
            try_line()
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn config_loading_rules() {
    let d = scratch("load");
    let log = repo_root().join("tests/logs/utf8.log");
    let log = log.to_str().unwrap();
    let out = d.join("out.conf");
    let outs = out.to_str().unwrap();
    std::fs::write(
        d.join("a.conf"),
        "[gource]\ntitle=from file\nseconds-per-day=5\n[display]\nviewport=800x600\n",
    )
    .unwrap();
    // a positional .conf that loads is the config; the other positional is the path; CLI beats the file
    let r = run_in(&d, &["a.conf", log, "-s", "9", "--save-config", outs]);
    assert_eq!((r.code, r.stderr.as_str()), (0, ""));
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        format!("[display]\nviewport=800x600\n\n[gource]\npath={log}\nseconds-per-day=9\ntitle=from file\n\n")
    );
    // --load-config suppresses the positional detection: a.conf is then just the path
    std::fs::write(d.join("b.conf"), "[gource]\ntitle=b\n").unwrap();
    let r = run_in(
        &d,
        &["--load-config", "b.conf", "a.conf", "--save-config", outs],
    );
    assert_eq!(r.code, 0);
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "[gource]\npath=a.conf\ntitle=b\n\n"
    );
    // a .conf that does not parse is a path, not a config
    std::fs::write(d.join("bad.conf"), "not a config\n").unwrap();
    let r = run_in(&d, &["bad.conf", "--save-config", outs]);
    assert_eq!(r.code, 0);
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "[gource]\npath=bad.conf\n\n"
    );
    // load errors
    let r = run_in(&d, &["--load-config", "bad.conf"]);
    assert_eq!(
        r.stderr,
        format!(
            "burl: bad.conf, line 1: could not parse line\n{}",
            try_line()
        )
    );
    let r = run_in(&d, &["--load-config", "missing.conf"]);
    assert_eq!(
        r.stderr,
        format!(
            "burl: failed to open config file missing.conf\n{}",
            try_line()
        )
    );
    // an invalid value is prefixed with the config name even when it came from the CLI
    let r = run_in(&d, &["a.conf", "--time-scale", "9"]);
    assert_eq!(
        r.stderr,
        format!(
            "burl: a.conf: time-scale outside of range 0.0 - 4.0\n{}",
            try_line()
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn value_errors_and_empty_values() {
    let d = scratch("errors");
    for (args, msg) in [
        (
            &["--seconds-per-day", "0"][..],
            "invalid 'seconds-per-day' value",
        ),
        (&["--hide", "nonsense"], "unknown option hide nonsense"),
        (
            &["--output-framerate", "45"],
            "supported framerates are 25,30,60",
        ),
        (&["-0x7", "--viewport", "0x0"], "unknown option 0x7"),
        (&["--viewport", "0x0"], "invalid 'viewport' value"),
        (&["--load-config"], "invalid load-config value"),
        (&["--log-command", "nope"], "invalid log-command value"),
        (&["--title"], "specify title"),
    ] {
        let r = run_in(&d, args);
        assert_eq!((r.code, r.stdout.as_str()), (1, ""), "{args:?}");
        assert_eq!(r.stderr, format!("burl: {msg}\n{}", try_line()), "{args:?}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn hidden_options_are_accepted_never_saved_and_validated() {
    let d = scratch("hidden");
    let out = d.join("out.conf");
    let r = run_in(
        &d,
        &[
            "--debug-dump-state",
            "x.jsonl",
            "--debug-dump-every",
            "30",
            "--debug-dump-max-frames",
            "10",
            "--prng",
            "park",
            "--prng-seed",
            "5",
            "--save-config",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!((r.code, r.stderr.as_str()), (0, ""));
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "[gource]\n\n");
    let r = run_in(
        &d,
        &[
            "--rand-script",
            "s.txt",
            "--save-config",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(r.code, 0);
    for (args, msg) in [
        (
            &["--prng", "glibc", "--rand-script", "f"][..],
            "prng and rand-script cannot be combined",
        ),
        (
            &["--rand-script", "f", "--prng", "glibc"],
            "prng and rand-script cannot be combined",
        ),
        (&["--prng", "xorshift"], "invalid prng value"),
        (&["--prng-seed", "x"], "invalid prng-seed value"),
        (
            &["--debug-dump-every", "0"],
            "invalid debug-dump-every value",
        ),
    ] {
        let r = run_in(&d, args);
        assert_eq!(
            (r.code, r.stderr),
            (1, format!("burl: {msg}\n{}", try_line())),
            "{args:?}"
        );
    }
    // never in the help text
    for flag in ["-h", "-H"] {
        let h = run_in(&d, &[flag]).stdout;
        assert!(
            !h.contains("prng") && !h.contains("debug-dump") && !h.contains("rand-script"),
            "{flag}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn filters_reach_the_custom_log() {
    // the imported --user-filter is applied by the log layer, not just parsed
    let log = repo_root().join("tests/logs/utf8.log");
    let d = scratch("filters");
    let all = run_in(
        &d,
        &[
            "--log-format",
            "custom",
            "--output-custom-log",
            "-",
            log.to_str().unwrap(),
        ],
    );
    let user = all
        .stdout
        .lines()
        .next()
        .unwrap()
        .split('|')
        .nth(1)
        .unwrap()
        .to_string();
    let filtered = run_in(
        &d,
        &[
            "--log-format",
            "custom",
            "--output-custom-log",
            "-",
            "--user-filter",
            &format!("^{}$", user.replace('\\', "\\\\")),
            log.to_str().unwrap(),
        ],
    );
    assert_eq!(filtered.code, 0);
    assert!(
        filtered
            .stdout
            .lines()
            .all(|l| l.split('|').nth(1) != Some(user.as_str())),
        "{}",
        filtered.stdout
    );
    assert!(filtered.stdout.len() < all.stdout.len());
    let _ = std::fs::remove_dir_all(&d);
}
