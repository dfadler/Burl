//! `src/formats/cvs-exp.cpp`: output of `cvs-exp.pl -notree`. Reachable only through
//! auto-detection: `--log-format cvs-exp` is accepted by settings but `fetchLog` matches
//! the string `"cvs"`, which settings reject (log-formats.md sec. 1 item 7; kept for parity).

use crate::commit::RCommit;
use crate::commitlog::{CommitLog, CommitLogBase};
use crate::seeklog::SeekLogError;
use crate::settings::LogSettings;
use crate::formats::svn::tm_from_entries;
use gource_util::regex::Regex;
use gource_util::timeparse::mktime_local;
use std::sync::{Arc, LazyLock};

pub const LOG_COMMAND: &str = "cvs-exp.pl -notree";

static COMMITNO: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^([0-9]{6}):"));
static BRANCH: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^BRANCH \[(.+)\]$"));
static DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::must(r"^\(date: ([0-9]{4})[-/]([0-9]{2})[-/]([0-9]{2}) ([0-9]{2}):([0-9]{2}):([0-9]{2})(?: [+-][0-9]{4})?;(.+)$")
});
static DETAIL: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"author: ([^;]+);  state: ([^;]+);(.+)$"));
static ENTRY: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"\| (.+),v:([0-9.]+),?"));
static END: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^(=+)$"));

pub struct CVSEXPCommitLog {
    base: CommitLogBase,
}

impl CVSEXPCommitLog {
    pub fn new(logfile: &str, settings: Arc<LogSettings>) -> Result<CVSEXPCommitLog, SeekLogError> {
        Ok(CVSEXPCommitLog { base: CommitLogBase::open(logfile, None, settings)? })
    }
}

impl CommitLog for CVSEXPCommitLog {
    super::impl_base!(CVSEXPCommitLog);

    fn parse_commit(&mut self, commit: &mut RCommit) -> bool {
        let mut line = Vec::new();
        let settings = self.base.settings.clone();
        if !self.base.raw_next_line(&mut line) {
            return false;
        }
        // skip empty line if there is one
        if line.is_empty() && !self.base.raw_next_line(&mut line) {
            return false;
        }
        if !COMMITNO.is_match(&line) {
            return false;
        }
        if !self.base.raw_next_line(&mut line) {
            return false;
        }
        // should be a branch
        if BRANCH.is_match(&line) {
            if !self.base.raw_next_line(&mut line) {
                return false;
            }
            if !line.is_empty() {
                return false;
            }
            if !self.base.raw_next_line(&mut line) {
                return false;
            }
        }
        let Some(entries) = DATE.captures(&line) else { return false };
        commit.timestamp = mktime_local(&tm_from_entries(&entries));
        let rest = entries[6].clone();
        let Some(entries) = DETAIL.captures(&rest) else { return false };
        commit.username = entries[0].clone();
        let commit_action = if entries[1] == b"dead" { "D" } else { "M" };
        if !self.base.raw_next_line(&mut line) {
            return false;
        }
        while let Some(e) = ENTRY.captures(&line) {
            // ignore files in Attic - previously deleted file
            if !e[0].windows(7).any(|w| w == b"/Attic/") {
                commit.add_file(&e[0], commit_action, &settings);
            }
            if !self.base.raw_next_line(&mut line) {
                return false;
            }
        }
        // blank line, then message until blank, then skip to the ===== line or EOF
        if !self.base.raw_next_line(&mut line) {
            return false;
        }
        while self.base.raw_next_line(&mut line) && !line.is_empty() {}
        while self.base.raw_next_line(&mut line) {
            if END.is_match(&line) {
                break;
            }
        }
        true
    }

    fn log_command(&self) -> String {
        LOG_COMMAND.to_string()
    }
}
