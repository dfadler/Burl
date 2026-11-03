//! `cargo xtask ci [--pcre2]`: the fast-tier checks, run exactly as `.github/workflows/ci.yml`
//! runs them (the workflow calls this command, so the two cannot drift).
//! `cargo xtask sim-compare GOLDEN ACTUAL [--floats exact|report]`: the simulation-gate comparator.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use xtask::simcmp::{self, Floats, Options};

/// The fast tier (ticket #13): fmt, clippy with warnings denied, locked tests.
fn steps(pcre2: bool) -> Vec<Vec<&'static str>> {
    let feat: &[&str] = if pcre2 { &["--features", "pcre2"] } else { &[] };
    let with = |base: &[&'static str]| [base, feat].concat();
    vec![
        vec!["fmt", "--all", "--check"],
        with(&["clippy", "--locked", "--workspace", "--all-targets"])
            .into_iter()
            .chain(["--", "-D", "warnings"])
            .collect(),
        with(&["test", "--locked", "--workspace"]),
    ]
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

/// `sim-compare`: exit 0 when the dumps agree under the tiers, 1 on a divergence, 2 on bad input.
fn sim_compare(args: &[String]) -> ExitCode {
    let usage = "usage: cargo xtask sim-compare GOLDEN ACTUAL [--floats exact|report]";
    let mut files = Vec::new();
    let mut floats = Floats::Exact;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--floats" => match it.next().map(String::as_str) {
                Some("exact") => floats = Floats::Exact,
                Some("report") => floats = Floats::Report,
                _ => {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                }
            },
            f => files.push(f),
        }
    }
    let [golden, actual] = files[..] else {
        eprintln!("{usage}");
        return ExitCode::from(2);
    };
    let root = workspace_root();
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let result = (|| {
        let ids = simcmp::deviation_ids(&read(&root.join("docs/rust-rewrite/DEVIATIONS.md"))?);
        let allow = simcmp::parse_allowlist(&read(&root.join(simcmp::ALLOWLIST))?, &ids)?;
        simcmp::compare(
            &read(Path::new(golden))?,
            &read(Path::new(actual))?,
            &Options { floats, allow },
        )
    })();
    match result {
        Ok(r) => {
            print!("{r}");
            ExitCode::from(u8::from(!r.ok()))
        }
        Err(e) => {
            eprintln!("sim-compare: {e}");
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("sim-compare") {
        return sim_compare(&args[1..]);
    }
    if args.first().map(String::as_str) != Some("ci") || args[1..].iter().any(|a| a != "--pcre2") {
        eprintln!("usage: cargo xtask ci [--pcre2]");
        return ExitCode::from(2);
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    for step in steps(args.len() > 1) {
        eprintln!("+ cargo {}", step.join(" "));
        match Command::new(&cargo).args(&step).status() {
            Ok(s) if s.success() => {}
            Ok(s) => return ExitCode::from(s.code().unwrap_or(1) as u8),
            Err(e) => {
                eprintln!("xtask: cannot run {cargo}: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::steps;

    #[test]
    fn pcre2_leg_adds_the_feature_to_clippy_and_test_only() {
        let s = steps(true);
        assert!(!s[0].contains(&"pcre2"));
        assert!(s[1].contains(&"pcre2") && s[2].contains(&"pcre2"));
        assert_eq!(s[1].last(), Some(&"warnings"));
        assert!(!steps(false).iter().flatten().any(|a| *a == "pcre2"));
    }
}
