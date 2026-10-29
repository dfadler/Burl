//! Parser-parity gate (parity-test-spec.md part 1): every row of
//! `golden/exit-codes.txt` is replayed against the Rust binary and stdout, stderr and the
//! exit code must match byte for byte; then the stdin variants, auto-detection, and the
//! error shapes settings-cli.md records.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn golden_dir() -> PathBuf {
    repo_root().join("docs/rust-rewrite/discovery/golden")
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: i32,
}

fn run_with(args: &[&str], stdin: Option<&[u8]>, envs: &[(&str, &str)]) -> Run {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gource"));
    cmd.args(args).current_dir(repo_root());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() });
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    if let Some(input) = stdin {
        let mut pipe = child.stdin.take().unwrap();
        pipe.write_all(input).unwrap();
        drop(pipe);
    }
    let out = child.wait_with_output().unwrap();
    Run { stdout: out.stdout, stderr: out.stderr, code: out.status.code().unwrap_or(-1) }
}

fn run(args: &[&str]) -> Run {
    run_with(args, None, &[])
}

fn read(p: &Path) -> Vec<u8> {
    std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn assert_bytes(what: &str, got: &[u8], want: &[u8]) {
    assert!(
        got == want,
        "{what} mismatch\n--- got ---\n{}\n--- want ---\n{}",
        String::from_utf8_lossy(got),
        String::from_utf8_lossy(want)
    );
}

/// `rc=N  name  (gource args...)` rows; the probe needs SDL and is C++-only.
#[test]
fn exit_codes_txt_rows() {
    let text = String::from_utf8(read(&golden_dir().join("exit-codes.txt"))).unwrap();
    let mut checked = 0;
    for line in text.lines() {
        let (rc, rest) = line.trim_start_matches("rc=").split_once("  ").unwrap();
        let (name, cmd) = rest.split_once("  ").unwrap();
        if name.ends_with(".bin") {
            continue;
        }
        let cmd = cmd.trim_start_matches('(').trim_end_matches(')');
        let mut argv: Vec<&str> = cmd.split(' ').skip(1).collect();
        let mut stdin = None;
        if let Some(pos) = argv.iter().position(|a| *a == "<") {
            stdin = Some(read(&repo_root().join(argv[pos + 1])));
            argv.truncate(pos);
        }
        let out = run_with(&argv, stdin.as_deref(), &[]);
        let dir = if name.ends_with(".txt") && Path::new(name).file_stem().is_some() && golden_dir().join("custom-log").join(name).exists() {
            golden_dir().join("custom-log")
        } else {
            golden_dir()
        };
        let stem = name.trim_end_matches(".txt");
        assert_eq!(out.code, rc.parse::<i32>().unwrap(), "{line}");
        assert_bytes(&format!("{name} stdout"), &out.stdout, &read(&dir.join(name)));
        assert_bytes(&format!("{name} stderr"), &out.stderr, &read(&dir.join(format!("{stem}.stderr.txt"))));
        checked += 1;
    }
    // the committed exit-codes.txt has 10 rows: 9 replayable plus the SDL probe
    assert_eq!(checked, 9, "rows replayed");
}

/// `gource --log-format custom --output-custom-log - - < tests/logs/utf8.log` must equal
/// the file-argument golden (misc-traces.md, exercised evidence).
#[test]
fn stdin_variant_matches_file_golden() {
    let g = golden_dir().join("custom-log");
    let input = read(&repo_root().join("tests/logs/utf8.log"));
    let out = run_with(&["--log-format", "custom", "--output-custom-log", "-", "-"], Some(&input), &[]);
    assert_eq!(out.code, 0);
    assert_bytes("stdout", &out.stdout, &read(&g.join("utf8.stdin.txt")));
    assert_bytes("stderr", &out.stderr, &read(&g.join("utf8.stdin.stderr.txt")));
    assert_bytes("same as file run", &out.stdout, &read(&g.join("utf8.txt")));
}

#[test]
fn stdin_drops_unterminated_last_line() {
    let g = golden_dir().join("custom-log");
    let input = read(&g.join("stdin-unterminated.input.txt"));
    let out = run_with(&["--log-format", "custom", "--output-custom-log", "-", "-"], Some(&input), &[]);
    assert_eq!(out.code, 0);
    assert_bytes("stdout", &out.stdout, &read(&g.join("stdin-unterminated.txt")));
    assert_bytes("stderr", &out.stderr, &read(&g.join("stdin-unterminated.stderr.txt")));
}

#[test]
fn stdin_requires_log_format() {
    let out = run_with(&["--output-custom-log", "-", "-"], Some(b""), &[]);
    assert_eq!(out.code, 1);
    assert_bytes(
        "stderr",
        &out.stderr,
        b"gource: log-format required when reading from STDIN\nTry 'gource --help' for more information.\n\n",
    );
}

/// parity-test-spec 1.1: without --log-format the detection order must land on the same
/// parser (custom / svn) and produce identical stdout; the caption file fails identically.
#[test]
fn auto_detect_matches_explicit_format() {
    for (fixture, fmt) in [
        ("custom-dir-delete", "custom"),
        ("file-removal", "custom"),
        ("file-to-dir", "custom"),
        ("utf8", "custom"),
        ("svn-dir-delete", "svn"),
        ("utf8-caption", "custom"),
    ] {
        let log = format!("tests/logs/{fixture}.log");
        let explicit = run(&["--log-format", fmt, "--output-custom-log", "-", &log]);
        let detected = run(&["--output-custom-log", "-", &log]);
        assert_eq!(detected.code, explicit.code, "{fixture}");
        assert_bytes(&format!("{fixture} stdout"), &detected.stdout, &explicit.stdout);
        assert_bytes(&format!("{fixture} stderr"), &detected.stderr, &explicit.stderr);
    }
}

#[test]
fn error_shapes() {
    let try_line = "Try 'gource --help' for more information.\n\n";
    let cases: [(&[&str], &str); 8] = [
        (&["--version"], "unknown option version"),
        (&["--", "x"], "unknown option -"),
        (&["--log-format", "bogus", "--output-custom-log", "-", "tests/logs/utf8.log"], "invalid 'log-format' value"),
        (&["--log-format", "cvs", "--output-custom-log", "-", "tests/logs/utf8.log"], "please use either 'cvs2cl' or 'cvs-exp'"),
        (&["--output-custom-log", "-", "tests/logs/utf8.log", "--log-format"], "specify log-format (format)"),
        (&["--output-custom-log"], "invalid output-custom-log value"),
        (&["--log-format", "custom", "--output-custom-log", "-", "tests/logs/does-not-exist.log"], "unsupported log format (you may need to regenerate your log file)"),
        // explicit cvs-exp never matches in fetchLog (preserved C++ bug)
        (&["--log-format", "cvs-exp", "--output-custom-log", "-", "tests/logs/utf8.log"], "unsupported log format (you may need to regenerate your log file)"),
    ];
    for (args, msg) in cases {
        let out = run(args);
        assert_eq!(out.code, 1, "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert_bytes(&format!("{args:?}"), &out.stderr, format!("gource: {msg}\n{try_line}").as_bytes());
    }
    // a directory with no repository above it (tests/logs sits inside this git checkout,
    // so detection would walk up and find .git), with and without a format
    let dir = std::env::temp_dir().join(format!("gource-phase1-dir-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dir_s = dir.to_str().unwrap();
    let out = run(&["--output-custom-log", "-", dir_s]);
    assert_eq!(out.code, 1);
    assert_bytes("dir", &out.stderr, format!("gource: directory not supported\n{try_line}").as_bytes());
    let out = run(&["--log-format", "custom", "--output-custom-log", "-", &format!("{dir_s}/")]);
    assert_bytes("dir+fmt", &out.stderr, format!("gource: failed to generate log file\n{try_line}").as_bytes());
    let _ = std::fs::remove_dir_all(&dir);
    // value-taking options swallow the next token even if it starts with '-'
    let out = run(&["--seconds-per-day", "--output-custom-log", "-", "--log-format", "custom", "tests/logs/utf8.log"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.starts_with(b"gource: phase 1 build"), "{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn log_command_printers() {
    let out = run(&["--svn-log-command"]);
    assert_eq!(out.code, 0);
    assert_bytes("svn", &out.stdout, b"svn log -r 1:HEAD --xml --verbose --quiet\n");
    let out = run(&["--log-command", "bzr"]);
    assert_bytes("bzr", &out.stdout, b"bzr log --verbose -r 1..-1 --short -n0 --forward\n");
    let out = run(&["--log-command", "cvs"]);
    assert_eq!(out.code, 1);
}

/// `chrono::Local` must honour TZ like the C library: date-string timestamps in a custom
/// log use the local zone (unit-tests.md rows 1-2).
#[test]
fn local_timezone_from_env() {
    let log = b"2021-11-01|bob|A|a\n2021-11-01 12:01:59|bob|A|b\n2021-11-01T12:01:59Z|bob|A|c\n";
    let args = ["--log-format", "custom", "--output-custom-log", "-", "-"];
    let out = run_with(&args, Some(log), &[("TZ", "Pacific/Auckland")]);
    assert_bytes("auckland", &out.stdout, b"1635678000|bob|A|/a\n1635721319|bob|A|/b\n1635768119|bob|A|/c\n");
    let out = run_with(&args, Some(log), &[("TZ", "UTC")]);
    assert_bytes("utc", &out.stdout, b"1635724800|bob|A|/a\n1635768119|bob|A|/b\n1635768119|bob|A|/c\n");
}

#[test]
fn output_to_file_and_silent_open_failure() {
    let dir = std::env::temp_dir().join(format!("gource-phase1-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out_path = dir.join("out.txt");
    let out = run(&["--log-format", "custom", "--output-custom-log", out_path.to_str().unwrap(), "tests/logs/utf8.log"]);
    assert_eq!(out.code, 0);
    assert_bytes("file", &read(&out_path), &read(&golden_dir().join("custom-log/utf8.txt")));
    let out = run(&["--log-format", "custom", "--output-custom-log", dir.join("no/such/dir/x").to_str().unwrap(), "tests/logs/utf8.log"]);
    assert_eq!(out.code, 0);
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_text_matches_golden() {
    for (flag, file) in [("-h", "help-short.txt"), ("-H", "help-long.txt"), ("--help", "help-short.txt"), ("-?", "help-short.txt")] {
        let out = run(&[flag]);
        assert_eq!(out.code, 0);
        assert_bytes(flag, &out.stdout, &read(&golden_dir().join(file)));
        assert!(out.stderr.is_empty());
    }
}
