//! `cargo xtask ci [--pcre2]`: the fast-tier checks, run exactly as `.github/workflows/ci.yml`
//! runs them (the workflow calls this command, so the two cannot drift).

use std::process::{Command, ExitCode};

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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
