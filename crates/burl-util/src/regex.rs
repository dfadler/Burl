//! Port of `src/core/regex.cpp` (PCRE2, compile flags 0: byte-wise, ASCII classes, no
//! caseless, `.` excludes `\n`, single-line anchors).
//!
//! Engine choice (rust-architecture-map.md sec. 6):
//! * [`Regex::new`]: `regex::bytes` with Unicode **off**. Every internal pattern (the nine
//!   log parsers, `parseDateTime`, the viewport and conffile grammars) uses only anchors,
//!   classes, counted/lazy repetition, non-capturing and optional groups, and the raw BOM
//!   bytes `\xEF\xBB\xBF`; no look-around or backreferences, so the `regex` crate matches
//!   PCRE2 exactly (same leftmost-first capture semantics).
//! * [`Regex::new_user`]: for user-supplied patterns (`--file-filter`, `--file-show-filter`,
//!   `--user-filter`, `--user-show-filter`) and the `--git-branch` check
//!   `^(?!-)[/\w.,;_=+{}\[\]-]+$` (negative look-ahead). These need real PCRE2, available
//!   behind the `pcre2` cargo feature; without it they fall back to the `regex` engine and
//!   patterns using look-around or backreferences fail to compile.
//!
//! Semantics every caller relies on (regex.cpp:149-208):
//! * `match` is an unanchored search, never a full match.
//! * An empty subject never matches, even `^$` (`matchOffset` returns -1 when
//!   `offset >= str.size()`).
//! * Captures are groups `1..rc-1` where `rc` is one more than the highest participating
//!   group: trailing non-participating groups are dropped, middle ones become `""`.
//!   Callers branch on `results.len()`.
//! * Known residual difference: PCRE2 `$` also matches before a final `\n`; `regex` does not.
//!   No internal pattern is applied to a subject that can end in `\n` (lines are split first).

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegexCompilationError(pub String);

impl fmt::Display for RegexCompilationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "regex compilation failed: {}", self.0)
    }
}

impl std::error::Error for RegexCompilationError {}

#[derive(Clone, Debug)]
pub struct Regex {
    engine: Engine,
}

#[derive(Clone, Debug)]
enum Engine {
    Bytes(regex::bytes::Regex),
    #[cfg(feature = "pcre2")]
    Pcre2(pcre2::bytes::Regex),
}

impl Regex {
    /// Internal pattern: `regex::bytes`, Unicode off.
    pub fn new(pattern: &str) -> Result<Regex, RegexCompilationError> {
        regex::bytes::RegexBuilder::new(pattern)
            .unicode(false)
            .build()
            .map(|r| Regex { engine: Engine::Bytes(r) })
            .map_err(|_| RegexCompilationError(pattern.to_string()))
    }

    /// Internal pattern that must compile; panics otherwise (statics).
    pub fn must(pattern: &str) -> Regex {
        Regex::new(pattern).unwrap_or_else(|e| panic!("{e}"))
    }

    /// PCRE2 with flags 0, for user patterns.
    #[cfg(feature = "pcre2")]
    pub fn new_pcre2(pattern: &str) -> Result<Regex, RegexCompilationError> {
        pcre2::bytes::RegexBuilder::new()
            .build(pattern)
            .map(|r| Regex { engine: Engine::Pcre2(r) })
            .map_err(|_| RegexCompilationError(pattern.to_string()))
    }

    /// User pattern entry point (filters, git-branch): PCRE2 when available.
    pub fn new_user(pattern: &str) -> Result<Regex, RegexCompilationError> {
        #[cfg(feature = "pcre2")]
        {
            Regex::new_pcre2(pattern)
        }
        #[cfg(not(feature = "pcre2"))]
        {
            Regex::new(pattern)
        }
    }

    /// `Regex::match(str)`: unanchored search; empty subject never matches.
    pub fn is_match(&self, subject: &[u8]) -> bool {
        if subject.is_empty() {
            return false;
        }
        match &self.engine {
            Engine::Bytes(re) => re.is_match(subject),
            #[cfg(feature = "pcre2")]
            Engine::Pcre2(re) => re.is_match(subject).unwrap_or(false),
        }
    }

    /// `Regex::match(str, &results)`: groups 1..=highest participating group.
    pub fn captures(&self, subject: &[u8]) -> Option<Vec<Vec<u8>>> {
        if subject.is_empty() {
            return None;
        }
        match &self.engine {
            Engine::Bytes(re) => {
                let caps = re.captures(subject)?;
                let groups: Vec<Option<&[u8]>> =
                    (1..caps.len()).map(|i| caps.get(i).map(|m| m.as_bytes())).collect();
                Some(trim_trailing(groups))
            }
            #[cfg(feature = "pcre2")]
            Engine::Pcre2(re) => {
                let caps = re.captures(subject).ok()??;
                let groups: Vec<Option<&[u8]>> =
                    (1..caps.len()).map(|i| caps.get(i).map(|m| m.as_bytes())).collect();
                Some(trim_trailing(groups))
            }
        }
    }
}

fn trim_trailing(mut groups: Vec<Option<&[u8]>>) -> Vec<Vec<u8>> {
    while matches!(groups.last(), Some(None)) {
        groups.pop();
    }
    groups
        .into_iter()
        .map(|g| g.map(|b| b.to_vec()).unwrap_or_default())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unanchored_search_and_group_one_only() {
        // regex_tests.cpp: Regex("([0-9]+)").match("ABC123") -> ["123"]
        let r = Regex::new("([0-9]+)").unwrap();
        assert_eq!(r.captures(b"ABC123"), Some(vec![b"123".to_vec()]));
        let r = Regex::new("^[A-Z]+$").unwrap();
        assert!(r.is_match(b"ABC"));
        assert!(!r.is_match(b"abc"));
        assert!(!r.is_match(b"ABC123"));
        assert!(Regex::new("^[A-Z+$").is_err());
    }

    #[test]
    fn empty_subject_never_matches() {
        let r = Regex::new("^$").unwrap();
        assert!(!r.is_match(b""));
        assert_eq!(r.captures(b""), None);
        assert!(Regex::new("").unwrap().captures(b"").is_none());
    }

    #[test]
    fn trailing_groups_dropped_middle_groups_empty() {
        let r = Regex::new("(a)(b)?(c)?(d)?").unwrap();
        assert_eq!(r.captures(b"a"), Some(vec![b"a".to_vec()]));
        assert_eq!(r.captures(b"ac"), Some(vec![b"a".to_vec(), vec![], b"c".to_vec()]));
        assert_eq!(
            r.captures(b"ad"),
            Some(vec![b"a".to_vec(), vec![], vec![], b"d".to_vec()])
        );
    }

    #[test]
    fn byte_mode_matches_invalid_utf8_and_bom_bytes() {
        let r = Regex::new(r"^(?:\xEF\xBB\xBF)?([^|]+)\|").unwrap();
        assert_eq!(
            r.captures(b"\xEF\xBB\xBFab|"),
            Some(vec![b"ab".to_vec()])
        );
        // PCRE2 without UTF treats 0xFF as an ordinary byte
        assert_eq!(r.captures(b"a\xFFb|"), Some(vec![b"a\xFFb".to_vec()]));
        // ASCII-only classes and `.` excluding newline
        assert!(!Regex::new(r"^\d$").unwrap().is_match("٣".as_bytes()));
        assert!(!Regex::new("^a.b$").unwrap().is_match(b"a\nb"));
    }

    #[test]
    fn every_internal_pattern_compiles() {
        // Verbatim from log-formats.md / settings.cpp; a failure here must surface in tests.
        let patterns = [
            r"^(?:\xEF\xBB\xBF)?([^|]+)\|([^|]*)\|([ADM]?)\|([^|]+)(?:\|#?([a-fA-F0-9]{6}))?",
            r"^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{1,2}):(\d{2})(?::(\d{2}(?:\.\d+)?))?)?(Z| ?([+-])(\d{1,2})(?::?(\d{2}))?)?$",
            r"([0-9]+)(?:\.([0-9]+))?(?:\.([0-9]+))?",
            r"^commit ([0-9a-z]+)",
            r"^tree ([0-9a-z]+)",
            r"^parent ([0-9a-z]+)",
            r"^author (.+) <([^@>]+)@?([^>]*)> (\d+) ([-+]\d+)",
            r"^committer (.+) <([^@>]+)@?([^>]*)> (\d+) ([-+]\d+)",
            r"^:[0-9]+ [0-9]+ [0-9a-z]+\.* ([0-9a-z]+)\.* ([A-Z])[ \t]+(.+)",
            r"^<\??xml",
            r"^<logentry",
            r"^</logentry>",
            r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})",
            r"^([0-9]+) -?[0-9]+\|([^|]+)\|([ADM]?)\|(.+)$",
            r"^ *([\d.]+) (.+)\t(\d{4})-(\d+)-(\d+)(?: \{[^}]+})?(?: \[merge\])?$",
            r"^ *([AMDR])  (.*[^/])$",
            r"^([0-9]{6}):",
            r"^BRANCH \[(.+)\]$",
            r"^\(date: ([0-9]{4})[-/]([0-9]{2})[-/]([0-9]{2}) ([0-9]{2}):([0-9]{2}):([0-9]{2})(?: [+-][0-9]{4})?;(.+)$",
            r"author: ([^;]+);  state: ([^;]+);(.+)$",
            r"\| (.+),v:([0-9.]+),?",
            r"^(=+)$",
            r"^<entry",
            r"^</entry>",
            r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})Z",
            r"^(?:[^ ]+ )?([^ ]+) +[^ ]+ +([^ ]+) +\[(.*?)\] +(.*)$",
            r"(\d+)/([A-Za-z]+)/(\d+):(\d+):(\d+):(\d+) ([+-])(\d+)",
            r#""([^ ]+) +([^ ]+) +([^ ]+)" +([^ ]+) +([^\s+]+)(.*)"#,
            r#" +"([^"]+)" +"([^"]+)" +"([^"]+)""#,
            r"([^.]+)(?:\.([^.]+))?(?:\.([^.]+))?(?:\.([^.]+))?(?:\.([^.]+))?(?:\.([^.]+))?(?:\.([^.]+))?(?:\.([^.]+))?$",
            r"^([0-9.]+)x([0-9.]+)(!)?$",
            r"^\s*\[([^\]]+)\]\s*$",
            r"^\s*([^=\s]+)\s*=\s*([^\s].*)?$",
        ];
        for p in patterns {
            Regex::new(p).unwrap_or_else(|e| panic!("{e}"));
        }
        // bzr's bare `}` must be a literal
        let bzr = Regex::new(r"^ *([\d.]+) (.+)\t(\d{4})-(\d+)-(\d+)(?: \{[^}]+})?(?: \[merge\])?$").unwrap();
        let c = bzr.captures(b"    3 Bob Smith\t2010-06-29 {tag-1} [merge]").unwrap();
        assert_eq!(c[1], b"Bob Smith");
        assert_eq!(c.len(), 5);
    }

    #[cfg(feature = "pcre2")]
    #[test]
    fn pcre2_engine_handles_lookaround() {
        let r = Regex::new_pcre2(r"^(?!-)[/\w.,;_=+{}\[\]-]+$").unwrap();
        assert!(r.is_match(b"main"));
        assert!(!r.is_match(b"-bad"));
        assert!(!r.is_match(b""));
        assert_eq!(
            Regex::new_pcre2("(a)(b)?").unwrap().captures(b"a"),
            Some(vec![b"a".to_vec()])
        );
    }
}
