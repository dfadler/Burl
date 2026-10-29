//! `src/formats/hg.cpp`: `hg log --style gource.style`, one file per line, grouped by
//! consecutive identical (timestamp, username) like the custom format.

use crate::commit::RCommit;
use crate::commitlog::{CommitLog, CommitLogBase};
use crate::seeklog::SeekLogError;
use crate::settings::LogSettings;
use gource_util::cstr::atoll;
use gource_util::regex::Regex;
use std::path::Path;
use std::sync::{Arc, LazyLock};

static HG_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^([0-9]+) -?[0-9]+\|([^|]+)\|([ADM]?)\|(.+)$"));

/// `MercurialLog::logCommand()`. Note the double space when no date range is given.
pub fn log_command(settings: &LogSettings) -> String {
    let style = format!("{}gource.style", settings.resource_dir);
    let range = match (settings.start_date.is_empty(), settings.stop_date.is_empty()) {
        (false, false) => format!("--date '{} to {}'", settings.start_date, settings.stop_date),
        (false, true) => format!("--date '>{}'", settings.start_date),
        (true, false) => format!("--date '<{}'", settings.stop_date),
        (true, true) => String::new(),
    };
    let cmd = format!("hg log {} -r 0:tip --style '{}'", range, style);
    if cfg!(windows) {
        cmd.replace('\'', "\"")
    } else {
        cmd
    }
}

pub struct MercurialLog {
    base: CommitLogBase,
}

impl MercurialLog {
    pub fn new(logfile: &str, settings: Arc<LogSettings>) -> Result<MercurialLog, SeekLogError> {
        let mut base = CommitLogBase::open(logfile, None, settings)?;
        if base.logf.is_none() && base.is_dir {
            let hgdir = Path::new(logfile).join(".hg");
            if std::fs::metadata(&hgdir).map(|m| m.is_dir()).unwrap_or(false) {
                let cmd = format!("{} -R \"{}\"", log_command(&base.settings), logfile);
                base.run_log_command(&cmd, None)?;
            }
        }
        Ok(MercurialLog { base })
    }

    fn parse_commit_entry(&mut self, commit: &mut RCommit) -> bool {
        let mut line = Vec::new();
        if !self.base.next_line(&mut line) {
            return false;
        }
        let Some(entries) = HG_REGEX.captures(&line) else { return false };
        let timestamp = atoll(&entries[0]);
        let username = &entries[1];
        if commit.files.is_empty() {
            commit.timestamp = timestamp;
            commit.username = username.clone();
        } else if commit.timestamp != timestamp || &commit.username != username {
            self.base.lastline = line;
            return false;
        }
        let action = if entries[2].is_empty() { "A" } else { std::str::from_utf8(&entries[2]).unwrap_or("A") };
        let settings = self.base.settings.clone();
        commit.add_file(&entries[3], action, &settings);
        true
    }
}

impl CommitLog for MercurialLog {
    super::impl_base!(MercurialLog);

    fn parse_commit(&mut self, commit: &mut RCommit) -> bool {
        while self.parse_commit_entry(commit) {}
        !commit.files.is_empty()
    }

    fn log_command(&self) -> String {
        log_command(&self.base.settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_string() {
        let s = LogSettings { resource_dir: "/usr/share/gource/".into(), ..LogSettings::default() };
        assert_eq!(log_command(&s), "hg log  -r 0:tip --style '/usr/share/gource/gource.style'");
        let s = LogSettings { start_date: "2020-01-01".into(), ..s };
        assert_eq!(log_command(&s), "hg log --date '>2020-01-01' -r 0:tip --style '/usr/share/gource/gource.style'");
        let s = LogSettings { stop_date: "2020-02-01".into(), ..s };
        assert_eq!(log_command(&s), "hg log --date '2020-01-01 to 2020-02-01' -r 0:tip --style '/usr/share/gource/gource.style'");
    }
}
