//! The simulation-gate comparator (ticket #11, parity-test-spec 2.6): reads two schema-1 JSONL
//! dumps (the fork's golden and Burl's), aligns them on `frame - loaded_frame0`, and reports the
//! first divergent field path.
//!
//! Tiers:
//! - **exact**: everything that is not a float (integers, bools, strings, key sets, array
//!   lengths). Which numbers are integers is `burl_sim::dump::INT_KEYS`;
//! - **bit-exact f32**: every other number, compared as f32 bit patterns;
//! - **tolerance**: `burl_sim::dump::LIBM_KEYS` (`sinf`/`cosf` results) at 1e-5 relative.
//!
//! A difference listed in the allowlist (`ID path`, ids checked against `DEVIATIONS.md`) is
//! counted under its id instead of failing; there is no tolerance flag for deviations.

use burl_sim::dump::{BUILD_KEYS, INT_KEYS, LIBM_KEYS, SCHEMA};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The committed allowlist, relative to the workspace root.
pub const ALLOWLIST: &str = "tests/golden/sim/allowlist.txt";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Exact,
    BitF32,
    Tolerance,
}

/// What a float mismatch does. `Report` is the slice-1 relaxation (#11 item 3): listed, not failing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Floats {
    Exact,
    Report,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allow {
    pub id: String,
    /// A normalised field path (`g.loaded_frame0`, `users[].pos`): indices are `[]`, frame
    /// records have no prefix, other records are prefixed by their type (`section.path`).
    pub pattern: String,
}

pub struct Options {
    pub floats: Floats,
    pub allow: Vec<Allow>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            floats: Floats::Exact,
            allow: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Divergence {
    /// 0-based line in the golden file.
    pub record: usize,
    /// `frame` of the record, relative to `loaded_frame0`, when it is a frame record.
    pub frame: Option<i64>,
    pub path: String,
    pub tier: Tier,
    pub golden: String,
    pub actual: String,
}

impl fmt::Display for Divergence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "record {}", self.record)?;
        if let Some(fr) = self.frame {
            write!(f, " (frame {fr})")?;
        }
        write!(
            f,
            ": {} [{:?}] golden {} actual {}",
            self.path, self.tier, self.golden, self.actual
        )
    }
}

#[derive(Default, Debug)]
pub struct Report {
    pub compared: usize,
    pub failures: Vec<Divergence>,
    pub warnings: Vec<Divergence>,
    pub allowed: BTreeMap<String, usize>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} records compared", self.compared)?;
        for (id, n) in &self.allowed {
            writeln!(f, "  allowed by {id}: {n} difference(s)")?;
        }
        if !self.warnings.is_empty() {
            writeln!(
                f,
                "  {} float difference(s) reported, not failing; first:",
                self.warnings.len()
            )?;
            writeln!(f, "    {}", self.warnings[0])?;
        }
        if self.failures.is_empty() {
            return writeln!(f, "OK");
        }
        writeln!(f, "FAIL: {} divergence(s); first:", self.failures.len())?;
        for d in self.failures.iter().take(10) {
            writeln!(f, "  {d}")?;
        }
        Ok(())
    }
}

/// The ids of the `| Dn |` / `| Gn |` rows of `DEVIATIONS.md`.
pub fn deviation_ids(markdown: &str) -> BTreeSet<String> {
    markdown
        .lines()
        .filter_map(|l| l.strip_prefix('|'))
        .filter_map(|l| l.split('|').next())
        .map(str::trim)
        .filter(|c| {
            c.len() > 1
                && c.starts_with(['D', 'G'])
                && c[1..].chars().next().is_some_and(|ch| ch.is_ascii_digit())
        })
        .map(str::to_string)
        .collect()
}

/// `ID path` per line; blank lines and `#` comments ignored. Every id must be a register row.
pub fn parse_allowlist(text: &str, ids: &BTreeSet<String>) -> Result<Vec<Allow>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (id, pattern) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("allowlist line {}: expected 'ID path'", n + 1))?;
        if !ids.contains(id) {
            return Err(format!(
                "allowlist line {}: {id} is not a row of DEVIATIONS.md",
                n + 1
            ));
        }
        out.push(Allow {
            id: id.to_string(),
            pattern: pattern.trim().to_string(),
        });
    }
    Ok(out)
}

fn parse(text: &str, what: &str) -> Result<Vec<Value>, String> {
    let mut v = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let rec: Value = serde_json::from_str(line)
            .map_err(|e| format!("{what} line {}: not JSON: {e}", n + 1))?;
        let schema = rec.get("schema").and_then(Value::as_u64);
        if schema != Some(SCHEMA as u64) {
            return Err(format!(
                "{what} line {}: schema {:?}, the comparator reads schema {SCHEMA}",
                n + 1,
                rec.get("schema")
            ));
        }
        v.push(rec);
    }
    if v.is_empty() {
        return Err(format!("{what}: no records"));
    }
    Ok(v)
}

fn rec_type(r: &Value) -> &str {
    r.get("type").and_then(Value::as_str).unwrap_or("")
}

/// `g.loaded_frame0` of the first frame record.
fn loaded_frame0(recs: &[Value]) -> i64 {
    recs.iter()
        .find(|r| rec_type(r) == "frame")
        .and_then(|r| r.pointer("/g/loaded_frame0"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

fn show(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 120 {
        format!("{}...", &s[..120])
    } else {
        s
    }
}

struct Walker<'a> {
    o: &'a Options,
    rep: &'a mut Report,
    record: usize,
    frame: Option<i64>,
    /// `section.` etc.; empty for frame records.
    prefix: &'a str,
}

impl Walker<'_> {
    fn norm(&self, path: &[String]) -> String {
        let mut s = String::from(self.prefix);
        for (i, p) in path.iter().enumerate() {
            if p.starts_with('[') {
                s.push_str("[]");
            } else {
                if i > 0 || !s.is_empty() {
                    s.push('.');
                }
                s.push_str(p);
            }
        }
        s
    }

    fn diverge(&mut self, path: &[String], tier: Tier, g: String, a: String) {
        let norm = self.norm(path);
        if let Some(al) = self.o.allow.iter().find(|al| al.pattern == norm) {
            *self.rep.allowed.entry(al.id.clone()).or_insert(0) += 1;
            return;
        }
        let d = Divergence {
            record: self.record,
            frame: self.frame,
            path: path_string(path),
            tier,
            golden: g,
            actual: a,
        };
        if tier != Tier::Exact && self.o.floats == Floats::Report {
            self.rep.warnings.push(d);
        } else {
            self.rep.failures.push(d);
        }
    }

    fn walk(&mut self, g: &Value, a: &Value, path: &mut Vec<String>) {
        match (g, a) {
            (Value::Object(go), Value::Object(ao)) => {
                let gk: Vec<&String> = go.keys().collect();
                let ak: Vec<&String> = ao.keys().collect();
                if gk != ak {
                    self.diverge(
                        path,
                        Tier::Exact,
                        format!("keys {gk:?}"),
                        format!("keys {ak:?}"),
                    );
                    return;
                }
                for (k, gv) in go {
                    if BUILD_KEYS.contains(&k.as_str()) {
                        continue;
                    }
                    path.push(k.clone());
                    // the section's `path` is where the run happened (a container path in the golden)
                    if self.prefix == "section." && k == "path" {
                        let base = |v: &Value| {
                            v.as_str()
                                .map(|s| s.rsplit('/').next().unwrap_or(s).to_string())
                        };
                        if base(gv) != base(&ao[k]) {
                            self.diverge(path, Tier::Exact, show(gv), show(&ao[k]));
                        }
                    } else {
                        self.walk(gv, &ao[k], path);
                    }
                    path.pop();
                }
            }
            (Value::Array(ga), Value::Array(aa)) => {
                if ga.len() != aa.len() {
                    self.diverge(
                        path,
                        Tier::Exact,
                        format!("{} elements", ga.len()),
                        format!("{} elements", aa.len()),
                    );
                    return;
                }
                for (i, (gv, av)) in ga.iter().zip(aa).enumerate() {
                    path.push(format!("[{i}]"));
                    self.walk(gv, av, path);
                    path.pop();
                }
            }
            (Value::Number(gn), Value::Number(an)) => {
                let key = path
                    .iter()
                    .rev()
                    .find(|p| !p.starts_with('['))
                    .map(String::as_str);
                if key.is_some_and(|k| INT_KEYS.contains(&k)) {
                    if gn != an {
                        self.diverge(path, Tier::Exact, gn.to_string(), an.to_string());
                    }
                    return;
                }
                let (gf, af) = (
                    gn.as_f64().unwrap_or(f64::NAN) as f32,
                    an.as_f64().unwrap_or(f64::NAN) as f32,
                );
                if key.is_some_and(|k| LIBM_KEYS.contains(&k)) {
                    if (gf - af).abs() > 1e-5 * f32::max(1.0, f32::max(gf.abs(), af.abs())) {
                        self.diverge(path, Tier::Tolerance, gn.to_string(), an.to_string());
                    }
                } else if gf.to_bits() != af.to_bits() {
                    self.diverge(path, Tier::BitF32, gn.to_string(), an.to_string());
                }
            }
            _ => {
                if g != a {
                    // a mismatch of kind or value (bool, string, null, "nan" vs number)
                    self.diverge(path, Tier::Exact, show(g), show(a));
                }
            }
        }
    }
}

fn path_string(path: &[String]) -> String {
    let mut s = String::new();
    for p in path {
        if !p.starts_with('[') && !s.is_empty() {
            s.push('.');
        }
        s.push_str(p);
    }
    s
}

/// Compare a golden dump with Burl's. `Err` is a malformed input, not a divergence.
pub fn compare(golden: &str, actual: &str, o: &Options) -> Result<Report, String> {
    let g = parse(golden, "golden")?;
    let a = parse(actual, "actual")?;
    let (gl0, al0) = (loaded_frame0(&g), loaded_frame0(&a));

    let mut rep = Report::default();

    // Frame records pair on `frame - loaded_frame0`; header and section records pair in order.
    let rel = |r: &Value, l0: i64| r.get("frame").and_then(Value::as_i64).map(|f| f - l0);
    let a_frames: BTreeMap<i64, usize> = a
        .iter()
        .enumerate()
        .filter(|(_, r)| rec_type(r) == "frame")
        .filter_map(|(i, r)| rel(r, al0).map(|f| (f, i)))
        .collect();
    let a_other: Vec<usize> = (0..a.len())
        .filter(|&i| rec_type(&a[i]) != "frame")
        .collect();
    let mut other_next = 0;

    for (gi, gr) in g.iter().enumerate() {
        let (ai, frame, prefix) = if rec_type(gr) == "frame" {
            let f = rel(gr, gl0).unwrap_or(-1);
            match a_frames.get(&f) {
                Some(&ai) => (ai, Some(f), ""),
                // sampled in one dump only: the cadence differs under a shifted alignment
                None if gl0 != al0 => continue,
                None => {
                    rep.failures.push(Divergence {
                        record: gi,
                        frame: Some(f),
                        path: "frame".into(),
                        tier: Tier::Exact,
                        golden: "present".into(),
                        actual: "missing".into(),
                    });
                    continue;
                }
            }
        } else {
            let Some(&ai) = a_other.get(other_next) else {
                rep.failures.push(Divergence {
                    record: gi,
                    frame: None,
                    path: rec_type(gr).into(),
                    tier: Tier::Exact,
                    golden: "present".into(),
                    actual: "missing".into(),
                });
                continue;
            };
            other_next += 1;
            let prefix = if rec_type(gr) == "section" {
                "section."
            } else {
                "header."
            };
            (ai, None, prefix)
        };

        // the absolute frame number is only meaningful relative to the load frame
        let (mut gv, mut av) = (gr.clone(), a[ai].clone());
        if frame.is_some() {
            gv["frame"] = Value::from(frame);
            av["frame"] = Value::from(frame);
        }
        let mut w = Walker {
            o,
            rep: &mut rep,
            record: gi,
            frame,
            prefix,
        };
        w.walk(&gv, &av, &mut Vec::new());
        rep.compared += 1;
    }

    if !g.iter().any(|r| rec_type(r) == "frame") || rep.compared == 0 {
        rep.failures.push(Divergence {
            record: 0,
            frame: None,
            path: "records".into(),
            tier: Tier::Exact,
            golden: "records to compare".into(),
            actual: "none paired".into(),
        });
    }
    let g_frames = g.iter().filter(|r| rec_type(r) == "frame").count();
    if gl0 == al0 && a_frames.len() != g_frames {
        rep.failures.push(Divergence {
            record: g.len(),
            frame: None,
            path: "frame records".into(),
            tier: Tier::Exact,
            golden: g_frames.to_string(),
            actual: a_frames.len().to_string(),
        });
    }
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(frame: i64, l0: i64, runtime: &str, rand_last: u32, dest: &str, pos: &str) -> String {
        format!(
            "{{\"schema\":1,\"type\":\"frame\",\"section\":1,\"repo_count\":1,\"frame\":{frame},\"g\":{{\"currtime\":1277788895,\"runtime\":{runtime},\"rand_last\":{rand_last},\"loaded_frame0\":{l0}}},\"files\":[{{\"path\":\"/a\",\"dest\":{dest},\"pos\":{pos}}}],\"users\":[]}}\n"
        )
    }

    fn run(g: &str, a: &str, o: &Options) -> Report {
        compare(g, a, o).unwrap()
    }

    const POS: &str = "[1.5,-2]";

    #[test]
    fn identical_dumps_pass() {
        let d = frame(0, 0, "0.0166666675", 7, "[0,1]", POS);
        let r = run(&d, &d, &Options::default());
        assert!(r.ok(), "{r}");
        assert_eq!(r.compared, 1);
    }

    #[test]
    fn integer_difference_fails_exactly_and_names_the_path() {
        let g = frame(0, 0, "1", 7, "[0,1]", POS);
        let a = frame(0, 0, "1", 8, "[0,1]", POS);
        let r = run(&g, &a, &Options::default());
        assert_eq!(r.failures.len(), 1);
        assert_eq!(r.failures[0].path, "g.rand_last");
        assert_eq!(r.failures[0].tier, Tier::Exact);
    }

    #[test]
    fn large_integers_are_not_compared_as_f32() {
        // 1277788895 and 1277788896 are the same f32
        let g = frame(0, 0, "1", 7, "[0,1]", POS);
        let a = g.replace("1277788895", "1277788896");
        assert_eq!(
            run(&g, &a, &Options::default()).failures[0].path,
            "g.currtime"
        );
    }

    #[test]
    fn one_ulp_float_difference_fails_bit_exact_but_only_warns_in_report_mode() {
        let g = frame(0, 0, "1", 7, "[0,1]", "[1.5,-2]");
        let a = frame(0, 0, "1", 7, "[0,1]", "[1.50000012,-2]"); // next f32 above 1.5
        let r = run(&g, &a, &Options::default());
        assert_eq!(r.failures[0].path, "files[0].pos[0]");
        assert_eq!(r.failures[0].tier, Tier::BitF32);
        let relaxed = Options {
            floats: Floats::Report,
            ..Options::default()
        };
        let r = run(&g, &a, &relaxed);
        assert!(r.ok() && r.warnings.len() == 1);
    }

    #[test]
    fn libm_fields_use_a_relative_tolerance() {
        let g = frame(0, 0, "1", 7, "[0.5,0.5]", POS);
        let near = frame(0, 0, "1", 7, "[0.500001,0.5]", POS);
        let far = frame(0, 0, "1", 7, "[0.51,0.5]", POS);
        assert!(run(&g, &near, &Options::default()).ok());
        let r = run(&g, &far, &Options::default());
        assert_eq!(
            (r.failures[0].path.as_str(), r.failures[0].tier),
            ("files[0].dest[0]", Tier::Tolerance)
        );
    }

    #[test]
    fn allowlisted_differences_are_counted_under_their_id() {
        let g = frame(0, 0, "1", 7, "[0,1]", POS);
        let ids = deviation_ids("| D2 | x |\n| D3 | y |\n");
        let allow = parse_allowlist("# c\nD2 g.loaded_frame0\n", &ids).unwrap();
        let o = Options {
            allow,
            ..Options::default()
        };
        // the fork loaded at frame 3, Burl at 0
        let a2 = frame(3, 3, "1", 7, "[0,1]", POS);
        let r = run(&g, &a2, &o);
        assert!(r.ok(), "{r}");
        assert_eq!(r.allowed.get("D2"), Some(&1));
        let r = run(&g, &a2, &Options::default());
        assert_eq!(r.failures[0].path, "g.loaded_frame0");
    }

    #[test]
    fn allowlist_ids_must_be_register_rows() {
        let ids = deviation_ids("| D2 | x |\n");
        assert!(parse_allowlist("D9 g.runtime\n", &ids).is_err());
        assert!(parse_allowlist("D2\n", &ids).is_err());
    }

    #[test]
    fn deviation_ids_reads_register_rows_only() {
        let md = "| id | Gource does |\n|----|----|\n| D1 | a |\n| D5a | b |\n| G2 | c |\nDx not a row\n";
        let ids: Vec<_> = deviation_ids(md).into_iter().collect();
        assert_eq!(ids, ["D1", "D5a", "G2"]);
    }

    #[test]
    fn frames_align_on_the_load_frame() {
        // the fork loaded at frame 2, Burl at frame 0: same relative frame, same content
        let g = frame(2, 2, "1", 7, "[0,1]", POS);
        let a = frame(0, 0, "1", 7, "[0,1]", POS);
        let o = Options {
            allow: vec![Allow {
                id: "D2".into(),
                pattern: "g.loaded_frame0".into(),
            }],
            ..Options::default()
        };
        let r = run(&g, &a, &o);
        assert!(r.ok(), "{r}");
        assert_eq!(r.allowed.get("D2"), Some(&1));
    }

    #[test]
    fn missing_frame_and_schema_mismatch() {
        let two = format!(
            "{}{}",
            frame(0, 0, "1", 7, "[0,1]", POS),
            frame(1, 0, "1", 7, "[0,1]", POS)
        );
        let one = frame(0, 0, "1", 7, "[0,1]", POS);
        assert!(!run(&two, &one, &Options::default()).ok());
        assert!(compare(
            &one.replace("\"schema\":1", "\"schema\":2"),
            &one,
            &Options::default()
        )
        .is_err());
    }

    #[test]
    fn extra_frames_and_extra_keys_in_burl_fail() {
        let one = frame(0, 0, "1", 7, "[0,1]", POS);
        let two = format!("{one}{}", frame(1, 0, "1", 7, "[0,1]", POS));
        let r = run(&one, &two, &Options::default());
        assert_eq!(r.failures[0].path, "frame records");
        let extra = one.replace("\"rand_last\":7", "\"rand_last\":7,\"surprise\":1");
        let r = run(&one, &extra, &Options::default());
        assert_eq!(r.failures[0].path, "g");
    }

    #[test]
    fn build_keys_and_the_section_path_are_not_compared() {
        let f = frame(0, 0, "1", 7, "[0,1]", POS);
        let h = |c: &str| {
            format!("{{\"schema\":1,\"type\":\"header\",\"prng\":\"script\",\"compiler\":\"{c}\"}}\n{f}")
        };
        assert!(run(&h("clang"), &h("rustc"), &Options::default()).ok());
        let s = |p: &str| format!("{{\"schema\":1,\"type\":\"section\",\"path\":\"{p}\"}}\n{f}");
        assert!(run(
            &s("/fx/a.log"),
            &s("tests/golden/sim/a.log"),
            &Options::default()
        )
        .ok());
        assert!(!run(
            &s("/fx/a.log"),
            &s("tests/golden/sim/b.log"),
            &Options::default()
        )
        .ok());
        let g = h("clang").replace("script", "park");
        assert!(
            !run(&g, &h("rustc"), &Options::default()).ok(),
            "prng mode is compared"
        );
    }
}
