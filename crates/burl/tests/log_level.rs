//! `--log-level` (audit gap 11). Expected stderr is derived from `logger.cpp` / `logmill.cpp` /
//! `conffile.cpp` by reading; the oracle could not be run (amd64 image hung), so these are not
//! oracle-captured.

use std::process::Command;

fn stderr(args: &[&str], dir: &std::path::Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_burl"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8(out.stderr).unwrap()
}

#[test]
fn levels_filter_and_unknown_is_off() {
    let dir = std::env::temp_dir().join(format!("burl-loglevel-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("c.log"), "1300000000|alice|A|src/a.c\n").unwrap();
    let run = |lv: &str| {
        stderr(
            &["--log-level", lv, "--output-custom-log", "-", "c.log"],
            &dir,
        )
    };

    assert_eq!(run("off"), "");
    assert_eq!(run("error"), "");
    assert_eq!(run("info"), "");
    assert_eq!(run("bogus"), "");
    let want = "  DEBUG: trying git...\n  DEBUG: trying mercurial...\n  DEBUG: trying bzr...\n  DEBUG: trying git raw...\n  DEBUG: trying cvs-exp...\n  DEBUG: trying svn...\n  DEBUG: trying cvs2cl...\n  DEBUG: trying custom...\n";
    assert_eq!(run("debug"), want);
    // warn is above debug in the C++ enum, so it prints the same lines
    assert_eq!(run("warn"), want);

    std::fs::write(dir.join("x.conf"), "[gource]\nlog-level=debug\n").unwrap();
    let conf = stderr(
        &[
            "--load-config",
            "x.conf",
            "--log-level",
            "debug",
            "--log-format",
            "custom",
            "--output-custom-log",
            "-",
            "c.log",
        ],
        &dir,
    );
    assert!(
        conf.starts_with(
            "  DEBUG: ConfFile::load(x.conf)\n  DEBUG: x.conf: [gource] log-level => debug\n"
        ),
        "{conf}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
