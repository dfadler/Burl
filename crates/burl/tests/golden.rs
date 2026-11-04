//! Parser-parity gate (parity-test-spec.md part 1): every row of
//! `golden/exit-codes.txt` is replayed against the Rust binary and stdout, stderr and the
//! exit code must match byte for byte; then the stdin variants, auto-detection, and the
//! error shapes settings-cli.md records.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
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
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_burl"));
    cmd.args(args).current_dir(repo_root());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.stdin(if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    if let Some(input) = stdin {
        let mut pipe = child.stdin.take().unwrap();
        pipe.write_all(input).unwrap();
        drop(pipe);
    }
    let out = child.wait_with_output().unwrap();
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code().unwrap_or(-1),
    }
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

/// The goldens come from the C++ original, which calls itself gource; this program is burl.
/// The program name is normalised on BOTH sides, and only at the anchored phrases below
/// (decision recorded in docs/rust-rewrite/discovery/LEDGER.md). Any other difference, including
/// the name anywhere else, still fails. `{n}` is the lowercase name and `{N}` the capitalised one.
const NAME_TEMPLATES: [&str; 5] = [
    "{N} v0.57",                              // first help line
    "Usage: {n} [options] [path]",            // usage line
    "a {n} config",                           // help prose
    "If omitted, {n} will attempt",           // help prose
    "Try '{n} --help' for more information.", // error trailer
];

fn replace_all(hay: &[u8], needle: &[u8], rep: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(hay.len());
    let mut i = 0;
    while i < hay.len() {
        if hay[i..].starts_with(needle) {
            out.extend_from_slice(rep);
            i += needle.len();
        } else {
            out.push(hay[i]);
            i += 1;
        }
    }
    out
}

fn normalise_name(text: &[u8], n: &str, cap: &str) -> Vec<u8> {
    let mut out = text.to_vec();
    for t in NAME_TEMPLATES {
        let from = t.replace("{n}", n).replace("{N}", cap);
        let to = t.replace("{n}", "@name@").replace("{N}", "@Name@");
        out = replace_all(&out, from.as_bytes(), to.as_bytes());
    }
    // error prefix: only a line that starts with "<name>: "
    let prefix = format!("{n}: ");
    let mut res = Vec::with_capacity(out.len());
    for line in out.split_inclusive(|b| *b == b'\n') {
        if line.starts_with(prefix.as_bytes()) {
            res.extend_from_slice(b"@name@: ");
            res.extend_from_slice(&line[prefix.len()..]);
        } else {
            res.extend_from_slice(line);
        }
    }
    res
}

/// `got` is this program's output (named burl), `want` a golden captured from gource.
fn assert_named(what: &str, got: &[u8], want: &[u8]) {
    assert_bytes(
        what,
        &normalise_name(got, "burl", "Burl"),
        &normalise_name(want, "gource", "Gource"),
    );
}

/// `rc=N  name  (gource args...)` rows; the probe needs SDL and is C++-only.
#[test]
fn exit_codes_txt_rows() {
    let text = String::from_utf8(read(&golden_dir().join("exit-codes.txt"))).unwrap();
    let mut checked = 0;
    let expected = text.lines().filter(|l| !l.contains(".bin  ")).count();
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
        let dir = if name.ends_with(".txt")
            && Path::new(name).file_stem().is_some()
            && golden_dir().join("custom-log").join(name).exists()
        {
            golden_dir().join("custom-log")
        } else {
            golden_dir()
        };
        let stem = name.trim_end_matches(".txt");
        assert_eq!(out.code, rc.parse::<i32>().unwrap(), "{line}");
        if name.starts_with("help-") {
            assert_named(
                &format!("{name} stdout"),
                &out.stdout,
                &golden_with_expected_diff(name),
            );
        } else {
            assert_bytes(
                &format!("{name} stdout"),
                &out.stdout,
                &read(&dir.join(name)),
            );
        }
        assert_named(
            &format!("{name} stderr"),
            &out.stderr,
            &read(&dir.join(format!("{stem}.stderr.txt"))),
        );
        checked += 1;
    }
    // every row except the SDL probe (.bin) was replayed
    assert!(
        expected > 0 && checked == expected,
        "rows replayed: {checked} of {expected}"
    );
}

/// `gource --log-format custom --output-custom-log - - < tests/logs/utf8.log` must equal
/// the file-argument golden (misc-traces.md, exercised evidence).
#[test]
fn stdin_variant_matches_file_golden() {
    let g = golden_dir().join("custom-log");
    let input = read(&repo_root().join("tests/logs/utf8.log"));
    let out = run_with(
        &["--log-format", "custom", "--output-custom-log", "-", "-"],
        Some(&input),
        &[],
    );
    assert_eq!(out.code, 0);
    assert_bytes("stdout", &out.stdout, &read(&g.join("utf8.stdin.txt")));
    assert_bytes(
        "stderr",
        &out.stderr,
        &read(&g.join("utf8.stdin.stderr.txt")),
    );
    assert_bytes("same as file run", &out.stdout, &read(&g.join("utf8.txt")));
}

#[test]
fn stdin_drops_unterminated_last_line() {
    let g = golden_dir().join("custom-log");
    let input = read(&g.join("stdin-unterminated.input.txt"));
    let out = run_with(
        &["--log-format", "custom", "--output-custom-log", "-", "-"],
        Some(&input),
        &[],
    );
    assert_eq!(out.code, 0);
    assert_bytes(
        "stdout",
        &out.stdout,
        &read(&g.join("stdin-unterminated.txt")),
    );
    assert_bytes(
        "stderr",
        &out.stderr,
        &read(&g.join("stdin-unterminated.stderr.txt")),
    );
}

#[test]
fn stdin_requires_log_format() {
    let out = run_with(&["--output-custom-log", "-", "-"], Some(b""), &[]);
    assert_eq!(out.code, 1);
    assert_bytes(
        "stderr",
        &out.stderr,
        b"burl: log-format required when reading from STDIN\nTry 'burl --help' for more information.\n\n",
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
        assert_bytes(
            &format!("{fixture} stdout"),
            &detected.stdout,
            &explicit.stdout,
        );
        assert_bytes(
            &format!("{fixture} stderr"),
            &detected.stderr,
            &explicit.stderr,
        );
    }
}

#[test]
fn error_shapes() {
    let try_line = "Try 'burl --help' for more information.\n\n";
    let cases: [(&[&str], &str); 8] = [
        (&["--version"], "unknown option version"),
        (&["--", "x"], "unknown option -"),
        (
            &[
                "--log-format",
                "bogus",
                "--output-custom-log",
                "-",
                "tests/logs/utf8.log",
            ],
            "invalid 'log-format' value",
        ),
        (
            &[
                "--log-format",
                "cvs",
                "--output-custom-log",
                "-",
                "tests/logs/utf8.log",
            ],
            "please use either 'cvs2cl' or 'cvs-exp'",
        ),
        (
            &[
                "--output-custom-log",
                "-",
                "tests/logs/utf8.log",
                "--log-format",
            ],
            "specify log-format (format)",
        ),
        (&["--output-custom-log"], "invalid output-custom-log value"),
        (
            &[
                "--log-format",
                "custom",
                "--output-custom-log",
                "-",
                "tests/logs/does-not-exist.log",
            ],
            "'tests/logs/does-not-exist.log' does not appear to be a valid file or directory",
        ),
        // explicit cvs-exp works in Burl (D5); a non-cvs-exp file is rejected by its parser
        (
            &[
                "--log-format",
                "cvs-exp",
                "--output-custom-log",
                "-",
                "tests/logs/utf8.log",
            ],
            "unsupported log format (you may need to regenerate your log file)",
        ),
    ];
    for (args, msg) in cases {
        let out = run(args);
        assert_eq!(out.code, 1, "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert_bytes(
            &format!("{args:?}"),
            &out.stderr,
            format!("burl: {msg}\n{try_line}").as_bytes(),
        );
    }
    // a directory with no repository above it (tests/logs sits inside this git checkout,
    // so detection would walk up and find .git), with and without a format
    let dir = std::env::temp_dir().join(format!("gource-phase1-dir-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dir_s = dir.to_str().unwrap();
    let out = run(&["--output-custom-log", "-", dir_s]);
    assert_eq!(out.code, 1);
    assert_bytes(
        "dir",
        &out.stderr,
        format!("burl: directory not supported\n{try_line}").as_bytes(),
    );
    let out = run(&[
        "--log-format",
        "custom",
        "--output-custom-log",
        "-",
        &format!("{dir_s}/"),
    ]);
    assert_bytes(
        "dir+fmt",
        &out.stderr,
        format!("burl: failed to generate log file\n{try_line}").as_bytes(),
    );
    let _ = std::fs::remove_dir_all(&dir);
    // value-taking options swallow the next token even if it starts with '-', so the value of
    // --seconds-per-day is "--output-custom-log" (atof 0) and the import rejects it
    let out = run(&[
        "--seconds-per-day",
        "--output-custom-log",
        "-",
        "--log-format",
        "custom",
        "tests/logs/utf8.log",
    ]);
    assert_eq!(out.code, 1);
    assert_bytes(
        "swallow",
        &out.stderr,
        format!("burl: invalid 'seconds-per-day' value\n{try_line}").as_bytes(),
    );
}

#[test]
fn log_command_printers() {
    let out = run(&["--svn-log-command"]);
    assert_eq!(out.code, 0);
    assert_bytes(
        "svn",
        &out.stdout,
        b"svn log -r 1:HEAD --xml --verbose --quiet\n",
    );
    let out = run(&["--log-command", "bzr"]);
    assert_bytes(
        "bzr",
        &out.stdout,
        b"bzr log --verbose -r 1..-1 --short -n0 --forward\n",
    );
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
    assert_bytes(
        "auckland",
        &out.stdout,
        b"1635678000|bob|A|/a\n1635721319|bob|A|/b\n1635768119|bob|A|/c\n",
    );
    let out = run_with(&args, Some(log), &[("TZ", "UTC")]);
    assert_bytes(
        "utc",
        &out.stdout,
        b"1635724800|bob|A|/a\n1635768119|bob|A|/b\n1635768119|bob|A|/c\n",
    );
}

#[test]
fn output_to_file_and_silent_open_failure() {
    let dir = std::env::temp_dir().join(format!("gource-phase1-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out_path = dir.join("out.txt");
    let out = run(&[
        "--log-format",
        "custom",
        "--output-custom-log",
        out_path.to_str().unwrap(),
        "tests/logs/utf8.log",
    ]);
    assert_eq!(out.code, 0);
    assert_bytes(
        "file",
        &read(&out_path),
        &read(&golden_dir().join("custom-log/utf8.txt")),
    );
    let out = run(&[
        "--log-format",
        "custom",
        "--output-custom-log",
        dir.join("no/such/dir/x").to_str().unwrap(),
        "tests/logs/utf8.log",
    ]);
    assert_eq!(out.code, 0);
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A golden with its `expected-diffs/<file>.diff` (deviation register, DEVIATIONS.md) applied;
/// the golden itself is never edited. No diff file means the golden is used as is.
fn golden_with_expected_diff(file: &str) -> Vec<u8> {
    let diff = golden_dir()
        .join("expected-diffs")
        .join(format!("{}.diff", file.trim_end_matches(".txt")));
    if !diff.exists() {
        return read(&golden_dir().join(file));
    }
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("burl-golden-{}-{n}-{file}", std::process::id()));
    std::fs::copy(golden_dir().join(file), &tmp).unwrap();
    let st = Command::new("patch")
        .arg("-s")
        .arg(&tmp)
        .arg(&diff)
        .status()
        .expect("patch(1) is required");
    assert!(
        st.success(),
        "expected diff {} does not apply",
        diff.display()
    );
    let out = read(&tmp);
    let _ = std::fs::remove_file(&tmp);
    out
}

/// D5a: the help says `--user-friction` defaults to 1.0 (Gource prints 0.67; the code default is 1.0).
#[test]
fn deviation_d5a_help_user_friction_default() {
    let out = String::from_utf8(run(&["-H"]).stdout).unwrap();
    assert!(
        out.contains("--user-friction SECONDS  Change the rate users slow down (default: 1.0)\n")
    );
    assert!(!out.contains("0.67"));
}

/// D5b: the help says `--dir-name-position` is between 0.1 and 1.0 (Gource says 0.0; the code
/// rejects below 0.1).
#[test]
fn deviation_d5b_help_dir_name_position_range() {
    let out = String::from_utf8(run(&["-H"]).stdout).unwrap();
    assert!(out.contains("(between 0.1 and 1.0, default is 0.5)."));
    let r = run(&["--dir-name-position", "0.05", "tests/logs/utf8.log"]);
    assert_eq!(r.code, 1);
    assert!(String::from_utf8_lossy(&r.stderr)
        .contains("dir-name-position outside of range 0.1 - 1.0 (inclusive)"));
}

#[test]
fn help_text_matches_golden() {
    for (flag, file) in [
        ("-h", "help-short.txt"),
        ("-H", "help-long.txt"),
        ("--help", "help-short.txt"),
        ("-?", "help-short.txt"),
    ] {
        let out = run(&[flag]);
        assert_eq!(out.code, 0);
        assert_named(flag, &out.stdout, &golden_with_expected_diff(file));
        assert!(out.stderr.is_empty());
    }
}

/// D5c: explicit `--log-format cvs-exp` selects the CVS-EXP parser (Gource: "unsupported log
/// format"). Burl-only test, no oracle golden.
#[test]
fn deviation_d5c_cvs_exp_explicit() {
    let envs = [("TZ", "UTC")];
    let explicit = run_with(
        &[
            "--log-format",
            "cvs-exp",
            "--output-custom-log",
            "-",
            "tests/logs/cvs-exp.log",
        ],
        None,
        &envs,
    );
    let detected = run_with(
        &["--output-custom-log", "-", "tests/logs/cvs-exp.log"],
        None,
        &envs,
    );
    assert_eq!(
        explicit.code,
        0,
        "{}",
        String::from_utf8_lossy(&explicit.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&explicit.stdout),
        "1073012645|bob|M|/src/a.c\n1073099045|amy|D|/src/a.c\n"
    );
    assert_bytes("detected", &detected.stdout, &explicit.stdout);
}

/// `--stop-on-idle` and `--ffp` are accepted and have no effect: output equals a run without them.
fn ignored_option_changes_nothing(opt: &str) {
    let args = |opt: Option<&str>| {
        let mut a: Vec<&str> = opt.into_iter().collect();
        a.extend([
            "--output-custom-log",
            "-",
            "--log-format",
            "custom",
            "tests/logs/utf8.log",
        ]);
        run(&a)
    };
    let (base, out) = (args(None), args(Some(opt)));
    assert_eq!(base.code, 0);
    assert_eq!(out.code, 0, "{opt}");
    assert_bytes(opt, &out.stdout, &base.stdout);
    assert_bytes(opt, &out.stderr, &base.stderr);
}

/// D4: `--ffp` is accepted and has no effect.
#[test]
fn deviation_d4_ffp_ignored() {
    ignored_option_changes_nothing("--ffp");
}

/// D5d: `--stop-on-idle` is accepted and has no effect.
#[test]
fn deviation_d5d_stop_on_idle_ignored() {
    ignored_option_changes_nothing("--stop-on-idle");
}

/// `tests/golden/{edge,real}/NAME.case` (ticket #17): each case runs as written and with
/// `--log-format X` removed (auto-detect, a required test) against goldens captured from the
/// oracle in the pinned image by `golden/scripts/gen-cases.sh`.
#[test]
fn scripted_cases() {
    let mut cases = 0;
    for dir in ["edge", "real"] {
        let d = repo_root().join("tests/golden").join(dir);
        let mut files: Vec<_> = std::fs::read_dir(&d)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        for case in files
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "case"))
        {
            let base = case.with_extension("");
            if base.with_extension("pending").exists() {
                continue; // oracle golden exists, Burl cannot match it yet; see the .pending file
            }
            let name = base.file_name().unwrap().to_str().unwrap().to_string();
            let words = String::from_utf8(read(case)).unwrap();
            let (mut envs, mut opts) = (Vec::new(), Vec::new());
            for w in words.split_whitespace() {
                match w.split_once('=') {
                    Some((k, v)) if opts.is_empty() => envs.push((k.to_string(), v.to_string())),
                    _ => opts.push(w.to_string()),
                }
            }
            let mut auto = Vec::new();
            let mut it = opts.iter();
            while let Some(o) = it.next() {
                if o == "--log-format" {
                    it.next();
                } else {
                    auto.push(o.clone());
                }
            }
            let log = format!("tests/golden/{dir}/{name}.log");
            for (suffix, base_args) in [("", &opts), (".auto", &auto)] {
                let mut args: Vec<&str> = base_args.iter().map(String::as_str).collect();
                args.extend(["--output-custom-log", "-", &log]);
                let env_refs: Vec<(&str, &str)> =
                    envs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                let out = run_with(&args, None, &env_refs);
                let want = |ext: &str| read(&d.join(format!("{name}{suffix}.{ext}")));
                let what = format!("{dir}/{name}{suffix}");
                let rc: i32 = String::from_utf8(want("rc"))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                assert_eq!(out.code, rc, "{what} exit code");
                assert_bytes(&format!("{what} stdout"), &out.stdout, &want("out"));
                assert_named(&format!("{what} stderr"), &out.stderr, &want("err"));
                cases += 1;
            }
        }
    }
    assert!(cases >= 2 * 17, "only {cases} runs");
}

/// D8: Gource polls non-blocking stdin and drops the first half of a line split across pipe
/// writes; Burl blocks and keeps it. Burl-only test, no oracle golden.
#[test]
fn deviation_d8_stdin_blocking_read() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_burl"));
    cmd.args(["--log-format", "custom", "--output-custom-log", "-", "-"]);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let mut pipe = child.stdin.take().unwrap();
    pipe.write_all(b"1000000000|ali").unwrap();
    pipe.flush().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    pipe.write_all(b"ce|A|a.txt\n").unwrap();
    drop(pipe);
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_bytes("stdout", &out.stdout, b"1000000000|alice|A|/a.txt\n");
}

/// D8: Gource's SeekLog goes through a different path above 100 MiB; Burl reads the whole
/// file. A 101 MiB log must come out complete. Burl-only test, no oracle golden.
#[test]
#[ignore = "101 MiB: slow in debug builds; run with --release -- --ignored (heavy tier)"]
fn deviation_d8_log_over_100_mib() {
    let dir = std::env::temp_dir().join(format!("burl-d8-big-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("big.log");
    let line = b"1000000000|alice|A|a.txt\n";
    let lines = 101 * 1024 * 1024 / line.len() + 1;
    let mut f = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    for _ in 0..lines {
        f.write_all(line).unwrap();
    }
    drop(f);
    let out = run(&[
        "--log-format",
        "custom",
        "--output-custom-log",
        "-",
        path.to_str().unwrap(),
    ]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(out.code, 0);
    assert_eq!(out.stdout.iter().filter(|b| **b == b'\n').count(), lines);
}
