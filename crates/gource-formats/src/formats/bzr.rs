//! `src/formats/bzr.cpp`: `bzr log --verbose --short -n0 --forward`. Header line then file
//! lines until a blank line; date only, local midnight.

use crate::commit::RCommit;
use crate::commitlog::{CommitLog, CommitLogBase};
use crate::seeklog::SeekLogError;
use crate::settings::LogSettings;
use gource_util::cstr::atoi;
use gource_util::regex::Regex;
use gource_util::timeparse::{mktime_local, Tm};
use std::path::Path;
use std::sync::{Arc, LazyLock};

static COMMIT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::must(r"^ *([\d.]+) (.+)\t(\d{4})-(\d+)-(\d+)(?: \{[^}]+})?(?: \[merge\])?$"));
static FILE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^ *([AMDR])  (.*[^/])$"));

/// `BazaarLog::logCommand()`.
pub fn log_command(settings: &LogSettings) -> String {
    let start = if settings.start_date.is_empty() { "1".to_string() } else { format!("date:{}", settings.start_date) };
    let stop = if settings.stop_date.is_empty() { "-1".to_string() } else { format!("date:{}", settings.stop_date) };
    format!("bzr log --verbose -r {}..{} --short -n0 --forward", start, stop)
}

pub struct BazaarLog {
    base: CommitLogBase,
}

impl BazaarLog {
    pub fn new(logfile: &str, settings: Arc<LogSettings>) -> Result<BazaarLog, SeekLogError> {
        let mut base = CommitLogBase::open(logfile, None, settings)?;
        if base.logf.is_none() && base.is_dir {
            let bzrdir = Path::new(logfile).join(".bzr");
            if std::fs::metadata(&bzrdir).map(|m| m.is_dir()).unwrap_or(false) {
                let cmd = format!("{} {}", log_command(&base.settings), logfile);
                base.run_log_command(&cmd, None)?;
            }
        }
        Ok(BazaarLog { base })
    }
}

impl CommitLog for BazaarLog {
    super::impl_base!(BazaarLog);

    fn parse_commit(&mut self, commit: &mut RCommit) -> bool {
        let mut line = Vec::new();
        if !self.base.raw_next_line(&mut line) {
            return false;
        }
        let Some(entries) = COMMIT_REGEX.captures(&line) else { return false };
        commit.username = entries[1].clone();
        let tm = Tm {
            year: atoi(&entries[2]) as i64,
            mon: atoi(&entries[3]) as i64 - 1,
            mday: atoi(&entries[4]) as i64,
            ..Tm::default()
        };
        commit.timestamp = mktime_local(&tm);
        let settings = self.base.settings.clone();
        while self.base.raw_next_line(&mut line) && !line.is_empty() {
            let Some(e) = FILE_REGEX.captures(&line) else { continue };
            let action = String::from_utf8_lossy(&e[0]).into_owned();
            commit.add_file(&e[1], &action, &settings);
        }
        true
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
        assert_eq!(log_command(&LogSettings::default()), "bzr log --verbose -r 1..-1 --short -n0 --forward");
        let s = LogSettings { start_date: "2020-01-01".into(), stop_date: "2020-02-02".into(), ..LogSettings::default() };
        assert_eq!(log_command(&s), "bzr log --verbose -r date:2020-01-01..date:2020-02-02 --short -n0 --forward");
    }
}
