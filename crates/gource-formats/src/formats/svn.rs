//! `src/formats/svn.cpp`: `svn log --xml --verbose`. First byte `<`. One `<logentry>` per
//! commit, accumulated line by line and parsed as an XML fragment.

use crate::commit::RCommit;
use crate::commitlog::{CommitLog, CommitLogBase};
use crate::seeklog::SeekLogError;
use crate::settings::LogSettings;
use crate::xml;
use gource_util::cstr::atoi;
use gource_util::regex::Regex;
use gource_util::timeparse::{timegm, Tm};
use std::path::Path;
use std::sync::{Arc, LazyLock};

static XML_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^<\??xml"));
static LOGENTRY_START: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^<logentry"));
static LOGENTRY_END: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^</logentry>"));
static TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})"));

/// `SVNCommitLog::logCommand()`.
pub fn log_command(settings: &LogSettings) -> String {
    let start = if settings.start_date.is_empty() { "1".to_string() } else { format!("{{{}}}", settings.start_date) };
    let stop = if settings.stop_date.is_empty() { "HEAD".to_string() } else { format!("{{{}}}", settings.stop_date) };
    format!("svn log -r {}:{} --xml --verbose --quiet", start, stop)
}

pub struct SVNCommitLog {
    base: CommitLogBase,
}

impl SVNCommitLog {
    pub fn new(logfile: &str, settings: Arc<LogSettings>) -> Result<SVNCommitLog, SeekLogError> {
        let mut base = CommitLogBase::open(logfile, Some(b'<'), settings)?;
        if base.logf.is_none() && base.is_dir {
            let svndir = Path::new(logfile).join(".svn");
            if std::fs::metadata(&svndir).map(|m| m.is_dir()).unwrap_or(false) {
                let cmd = log_command(&base.settings);
                base.run_log_command(&cmd, Some(Path::new(logfile)))?;
            }
        }
        Ok(SVNCommitLog { base })
    }
}

/// Lines from the first `^<logentry` (skipping past an `^<\??xml` header) to `^</logentry>`,
/// joined with `\n`. Shared with cvs2cl (different tag names).
pub(crate) fn read_entry(base: &mut CommitLogBase, start: &Regex, end: &Regex) -> Option<Vec<u8>> {
    let mut line = Vec::new();
    if !base.next_line(&mut line) {
        return None;
    }
    if !start.is_match(&line) {
        // is this the start of the document
        if !XML_TAG.is_match(&line) {
            return None;
        }
        let mut found = false;
        while base.next_line(&mut line) {
            if start.is_match(&line) {
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }
    let mut entry = line.clone();
    entry.push(b'\n');
    let mut endfound = false;
    while base.next_line(&mut line) {
        entry.extend_from_slice(&line);
        entry.push(b'\n');
        if end.is_match(&line) {
            endfound = true;
            break;
        }
    }
    if !endfound {
        return None;
    }
    Some(entry)
}

/// `(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})` groups to a `Tm`.
pub(crate) fn tm_from_entries(e: &[Vec<u8>]) -> Tm {
    Tm {
        year: atoi(&e[0]) as i64,
        mon: atoi(&e[1]) as i64 - 1,
        mday: atoi(&e[2]) as i64,
        hour: atoi(&e[3]) as i64,
        min: atoi(&e[4]) as i64,
        sec: atoi(&e[5]) as i64,
    }
}

impl CommitLog for SVNCommitLog {
    super::impl_base!(SVNCommitLog);

    fn parse_commit(&mut self, commit: &mut RCommit) -> bool {
        let Some(logentry) = read_entry(&mut self.base, &LOGENTRY_START, &LOGENTRY_END) else { return false };
        let settings = self.base.settings.clone();
        let Some(doc) = xml::parse_fragment(&logentry) else { return false };
        let root = doc.root_element();
        if root.tag_name().name() != "logentry" {
            return false;
        }
        // date (a <date> without text would crash the C++; treat as failure)
        let Some(date) = xml::first_child_element(root, "date") else { return false };
        let Some(ts) = xml::get_text(date) else { return false };
        let Some(entries) = TIMESTAMP.captures(ts.as_bytes()) else { return false };
        commit.timestamp = timegm(&tm_from_entries(&entries));
        // author
        if let Some(author) = xml::first_child_element(root, "author") {
            let mut name = xml::get_text(author).unwrap_or_default();
            if name.is_empty() {
                name = "Unknown".to_string();
            }
            commit.username = name.into_bytes();
        }
        // log entries sometimes dont have any paths
        let Some(paths) = xml::first_child_element(root, "paths") else { return true };
        for path in xml::sibling_elements_from(paths, "path") {
            let kind = path.attribute("kind");
            let Some(action) = path.attribute("action") else { continue };
            let mut is_dir = false;
            if kind == Some("dir") {
                // accept only deletes for directories
                if action != "D" {
                    continue;
                }
                is_dir = true;
            }
            let Some(mut file) = xml::get_text(path) else { return false };
            if file.is_empty() || action.is_empty() {
                continue;
            }
            if is_dir && !file.ends_with('/') {
                file.push('/');
            }
            commit.add_file(file.as_bytes(), action, &settings);
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
        assert_eq!(log_command(&LogSettings::default()), "svn log -r 1:HEAD --xml --verbose --quiet");
        let s = LogSettings { start_date: "2020-01-01".into(), stop_date: "2020-02-02".into(), ..LogSettings::default() };
        assert_eq!(log_command(&s), "svn log -r {2020-01-01}:{2020-02-02} --xml --verbose --quiet");
    }
}
