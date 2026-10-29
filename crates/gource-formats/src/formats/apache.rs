//! `src/formats/apache.cpp`: Apache combined access log, one line = one commit with one
//! file, action `A`, username = host. `apache_entry_agent` and `apache_hostname_parts`
//! exist in the C++ but their results are unused, so they are not transcribed.

use crate::commit::RCommit;
use crate::commitlog::{CommitLog, CommitLogBase};
use crate::seeklog::SeekLogError;
use crate::settings::LogSettings;
use gource_util::cstr::atoi;
use gource_util::regex::Regex;
use gource_util::timeparse::{mktime_local, Tm};
use std::sync::{Arc, LazyLock};

const MONTHS: [&[u8]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

static ENTRY_START: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^(?:[^ ]+ )?([^ ]+) +[^ ]+ +([^ ]+) +\[(.*?)\] +(.*)$"));
static ENTRY_DATE: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"(\d+)/([A-Za-z]+)/(\d+):(\d+):(\d+):(\d+) ([+-])(\d+)"));
static ENTRY_REQUEST: LazyLock<Regex> = LazyLock::new(|| Regex::must(r#""([^ ]+) +([^ ]+) +([^ ]+)" +([^ ]+) +([^\s+]+)(.*)"#));

pub struct ApacheCombinedLog {
    base: CommitLogBase,
}

impl ApacheCombinedLog {
    pub fn new(logfile: &str, settings: Arc<LogSettings>) -> Result<ApacheCombinedLog, SeekLogError> {
        Ok(ApacheCombinedLog { base: CommitLogBase::open(logfile, None, settings)? })
    }
}

/// `parseCommit` on one line (pure, for tests).
pub fn parse_line(line: &[u8], commit: &mut RCommit, settings: &LogSettings) -> bool {
    let Some(matches) = ENTRY_START.captures(line) else { return false };
    if matches.len() != 4 {
        return false;
    }
    commit.username = matches[0].clone();
    let request_str = matches[3].clone();
    let datestr = matches[2].clone();
    let Some(m) = ENTRY_DATE.captures(&datestr) else { return false };
    if m.len() != 8 {
        return false;
    }
    let month = MONTHS.iter().position(|&name| name == m[1].as_slice()).unwrap_or(0);
    let tm = Tm {
        year: atoi(&m[2]) as i64,
        mon: month as i64,
        mday: atoi(&m[0]) as i64,
        hour: atoi(&m[3]) as i64,
        min: atoi(&m[4]) as i64,
        sec: atoi(&m[5]) as i64,
    };
    commit.timestamp = mktime_local(&tm);
    let Some(m) = ENTRY_REQUEST.captures(&request_str) else { return false };
    if m.len() < 5 {
        return false;
    }
    let mut file = m[1].clone();
    // remove args from url
    if let Some(argpos) = file.iter().rposition(|&b| b == b'?') {
        file.truncate(argpos);
    }
    if file.is_empty() {
        file = b"/".to_vec();
    }
    // name index pages
    if file.last() == Some(&b'/') {
        file.extend_from_slice(b"index.html");
    }
    commit.add_file(&file, "A", settings);
    true
}

impl CommitLog for ApacheCombinedLog {
    super::impl_base!(ApacheCombinedLog);

    fn parse_commit(&mut self, commit: &mut RCommit) -> bool {
        let mut line = Vec::new();
        if !self.base.raw_next_line(&mut line) {
            return false;
        }
        let settings = self.base.settings.clone();
        parse_line(&line, commit, &settings)
    }

    fn log_command(&self) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combined_line() {
        let s = LogSettings::default();
        let mut c = RCommit::default();
        let line = br#"127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif?x=1 HTTP/1.0" 200 2326 "http://ref/" "Mozilla""#;
        assert!(parse_line(line, &mut c, &s));
        assert_eq!(c.username, b"127.0.0.1");
        assert_eq!(c.files[0].filename, "/apache_pb.gif");
        assert_eq!(c.files[0].action, "A");
        let mut c = RCommit::default();
        let line = br#"vhost.example 10.0.0.1 - - [10/Oct/2000:13:55:36 +0000] "GET /dir/ HTTP/1.1" 200 5"#;
        assert!(parse_line(line, &mut c, &s));
        assert_eq!(c.username, b"10.0.0.1");
        assert_eq!(c.files[0].filename, "/dir/index.html");
        assert!(!parse_line(b"garbage", &mut RCommit::default(), &s));
    }
}
