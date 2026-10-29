//! `src/formats/cvs2cl.cpp`: `cvs2cl --chrono --stdout --xml -g-q`. First byte `<`. Same
//! accumulate-then-parse structure as SVN with `<entry>`; the `Z` date is converted with
//! local `mktime` (inconsistent with svn, preserved).

use crate::commit::RCommit;
use crate::commitlog::{CommitLog, CommitLogBase};
use crate::formats::svn::{read_entry, tm_from_entries};
use crate::seeklog::SeekLogError;
use crate::settings::LogSettings;
use crate::xml;
use gource_util::regex::Regex;
use gource_util::timeparse::mktime_local;
use std::sync::{Arc, LazyLock};

pub const LOG_COMMAND: &str = "cvs2cl --chrono --stdout --xml -g-q";

static ENTRY_START: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^<entry"));
static ENTRY_END: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^</entry>"));
static TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})Z"));

pub struct CVS2CLCommitLog {
    base: CommitLogBase,
}

impl CVS2CLCommitLog {
    pub fn new(logfile: &str, settings: Arc<LogSettings>) -> Result<CVS2CLCommitLog, SeekLogError> {
        Ok(CVS2CLCommitLog { base: CommitLogBase::open(logfile, Some(b'<'), settings)? })
    }
}

impl CommitLog for CVS2CLCommitLog {
    super::impl_base!(CVS2CLCommitLog);

    fn parse_commit(&mut self, commit: &mut RCommit) -> bool {
        let Some(logentry) = read_entry(&mut self.base, &ENTRY_START, &ENTRY_END) else { return false };
        let settings = self.base.settings.clone();
        let Some(doc) = xml::parse_fragment(&logentry) else { return false };
        let root = doc.root_element();
        if root.tag_name().name() != "entry" {
            return false;
        }
        let Some(date) = xml::first_child_element(root, "isoDate") else { return false };
        let Some(ts) = xml::get_text(date) else { return false };
        let Some(entries) = TIMESTAMP.captures(ts.as_bytes()) else { return false };
        commit.timestamp = mktime_local(&tm_from_entries(&entries));
        if let Some(author) = xml::first_child_element(root, "author") {
            // std::string(NULL) would crash the C++
            let Some(mut name) = xml::get_text(author) else { return false };
            if name.is_empty() {
                name = "Unknown".to_string();
            }
            commit.username = name.into_bytes();
        }
        for file in xml::sibling_elements_from(root, "file") {
            let state = xml::first_child_element(file, "cvsstate");
            let name = xml::first_child_element(file, "name");
            let (Some(state), Some(name)) = (state, name) else { continue };
            let Some(state_text) = xml::get_text(state) else { return false };
            let status = if state_text == "dead" { "D" } else { "M" };
            let Some(file_name) = xml::get_text(name) else { return false };
            if file_name.is_empty() {
                continue;
            }
            commit.add_file(file_name.as_bytes(), status, &settings);
        }
        true
    }

    fn log_command(&self) -> String {
        LOG_COMMAND.to_string()
    }
}
