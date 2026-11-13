//! `cargo xtask ci [--pcre2]`: the fast-tier checks, run exactly as `.github/workflows/ci.yml`
//! runs them (the workflow calls this command, so the two cannot drift).
//! `cargo xtask sim-compare GOLDEN ACTUAL [--floats exact|report]`: the simulation-gate comparator.
//! `cargo xtask display-harness`: the macOS virtual-display harness (ticket #152, macOS only).
//! `cargo xtask ssim A B [--min X]`, `ssim-frames STREAM.ppm PREFIX I,J,..`, `ssim-threshold V..`:
//! the visual-equivalence gate's comparator and helpers (ticket #12).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use xtask::simcmp::{self, Floats, Options};
use xtask::ssim;

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

/// The `ssim*` subcommands. Exit 0 ok, 1 below `--min`, 2 on bad input.
fn ssim_cmd(cmd: &str, args: &[String]) -> Result<ExitCode, String> {
    let load = |p: &String| ssim::decode(&std::fs::read(p).map_err(|e| format!("{p}: {e}"))?);
    match (cmd, args) {
        ("ssim", [a, b, rest @ ..]) => {
            let min = match rest {
                [] => None,
                [f, v] if f == "--min" => Some(v.parse::<f64>().map_err(|e| format!("--min: {e}"))?),
                _ => return Err("usage: cargo xtask ssim A B [--min X]".into()),
            };
            let s = ssim::ssim(&load(a)?, &load(b)?)?;
            println!("ssim={s:.6}");
            Ok(ExitCode::from(u8::from(min.is_some_and(|m| s < m))))
        }
        ("ssim-luma", [a, b]) => {
            let (la, lb) = (ssim::mean_luma(&load(a)?), ssim::mean_luma(&load(b)?));
            // a black reference has no relative difference: say so instead of printing NaN% / inf%
            let diff = if la > 0.0 { format!("{:.2}%", (lb - la).abs() / la * 100.0) } else { "n/a (zero-luma ref)".into() };
            println!("luma={la:.3}/{lb:.3} diff={diff}");
            Ok(ExitCode::SUCCESS)
        }
        ("ssim-frames", [stream, prefix, idx]) => {
            let bytes = std::fs::read(stream).map_err(|e| format!("{stream}: {e}"))?;
            let frames = ssim::ppm_frames(&bytes)?;
            for i in idx.split(',') {
                let n: usize = i.parse().map_err(|_| format!("bad frame index {i:?}"))?;
                let &(w, h, rgb) = frames.get(n).ok_or(format!("frame {n} of {}", frames.len()))?;
                ssim::write_png(Path::new(&format!("{prefix}-{n}.png")), w, h, rgb)?;
            }
            println!("frames={}", frames.len());
            Ok(ExitCode::SUCCESS)
        }
        ("ssim-threshold", vals) => {
            let v: Vec<f64> = vals.iter().map(|s| s.parse().map_err(|e| format!("{s}: {e}"))).collect::<Result<_, _>>()?;
            let t = ssim::threshold(&v).ok_or("no values")?;
            println!("runs={} worst={:.6} threshold={t:.6}", v.len(), t + ssim::MARGIN);
            Ok(ExitCode::SUCCESS)
        }
        _ => Err("usage: cargo xtask ssim A B [--min X] | ssim-luma REF NEW | ssim-frames STREAM.ppm PREFIX I,J,.. | ssim-threshold V..".into()),
    }
}

/// `display-harness`: macOS only (private CGVirtualDisplay API); exit 1 on a failed step or assertion.
fn display_harness_cmd(args: &[String]) -> ExitCode {
    let high_dpi = args.iter().any(|a| a == "--high-dpi");
    let grab = args.iter().any(|a| a == "--grab");
    if args.iter().any(|a| a != "--high-dpi" && a != "--grab") || (high_dpi && grab) {
        eprintln!("usage: cargo xtask display-harness [--high-dpi | --grab]");
        return ExitCode::from(2);
    }
    if !cfg!(target_os = "macos") {
        eprintln!("display-harness: needs macOS (virtual displays, System Events, screencapture)");
        return ExitCode::FAILURE;
    }
    let result = if grab {
        xtask::display_harness::run_grab_check(&workspace_root())
    } else {
        xtask::display_harness::run_harness(&workspace_root(), high_dpi)
    };
    match result {
        Ok(()) => {
            eprintln!("display-harness: all assertions passed");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("display-harness: FAIL: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(cmd) = args.first().filter(|c| c.starts_with("ssim")) {
        return ssim_cmd(cmd, &args[1..]).unwrap_or_else(|e| {
            eprintln!("{cmd}: {e}");
            ExitCode::from(2)
        });
    }
    if args.first().map(String::as_str) == Some("display-harness") {
        return display_harness_cmd(&args[1..]);
    }
    if args.first().map(String::as_str) == Some("sim-compare") {
        return sim_compare(&args[1..]);
    }
    if args.first().map(String::as_str) != Some("ci") || args[1..].iter().any(|a| a != "--pcre2") {
        eprintln!("usage: cargo xtask ci [--pcre2] | sim-compare .. | ssim* .. | display-harness [--high-dpi | --grab]");
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
