//! `src/formats/gitraw.cpp`: deprecated `git log --pretty=raw` format. First byte `c`.

use crate::commit::RCommit;
use crate::commitlog::{CommitLog, CommitLogBase};
use crate::seeklog::SeekLogError;
use crate::settings::LogSettings;
use gource_util::cstr::atoll;
use gource_util::regex::Regex;
use std::sync::{Arc, LazyLock};

pub const LOG_COMMAND: &str = "git log --reverse --raw --pretty=raw";

static COMMIT: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^commit ([0-9a-z]+)"));
static TREE: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^tree ([0-9a-z]+)"));
static PARENT: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^parent ([0-9a-z]+)"));
static AUTHOR: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^author (.+) <([^@>]+)@?([^>]*)> (\d+) ([-+]\d+)"));
static COMMITTER: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^committer (.+) <([^@>]+)@?([^>]*)> (\d+) ([-+]\d+)"));
static FILE: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^:[0-9]+ [0-9]+ [0-9a-z]+\.* ([0-9a-z]+)\.* ([A-Z])[ \t]+(.+)"));

pub struct GitRawCommitLog {
    base: CommitLogBase,
}

impl GitRawCommitLog {
    pub fn new(logfile: &str, settings: Arc<LogSettings>) -> Result<GitRawCommitLog, SeekLogError> {
        Ok(GitRawCommitLog { base: CommitLogBase::open(logfile, Some(b'c'), settings)? })
    }
}

impl CommitLog for GitRawCommitLog {
    super::impl_base!(GitRawCommitLog);

    fn parse_commit(&mut self, commit: &mut RCommit) -> bool {
        let mut line = Vec::new();
        let settings = self.base.settings.clone();
        // commit
        if !self.base.raw_next_line(&mut line) || !COMMIT.is_match(&line) {
            return false;
        }
        // tree
        if !self.base.raw_next_line(&mut line) || !TREE.is_match(&line) {
            return false;
        }
        if !self.base.raw_next_line(&mut line) {
            return false;
        }
        // 0 or more parents
        while PARENT.is_match(&line) {
            if !self.base.raw_next_line(&mut line) {
                return false;
            }
        }
        // author - used for display name
        let Some(entries) = AUTHOR.captures(&line) else { return false };
        commit.username = entries[0].clone();
        // committer - used for time
        if !self.base.raw_next_line(&mut line) {
            return false;
        }
        let Some(entries) = COMMITTER.captures(&line) else { return false };
        commit.timestamp = atoll(&entries[3]);
        // blank line before message
        if !self.base.raw_next_line(&mut line) {
            return false;
        }
        // commit message
        while self.base.raw_next_line(&mut line) && !line.is_empty() {}
        // files
        while self.base.raw_next_line(&mut line) && !line.is_empty() {
            if let Some(e) = FILE.captures(&line) {
                let action = String::from_utf8_lossy(&e[1]).into_owned();
                commit.add_file(&e[2], &action, &settings);
            }
        }
        true
    }

    fn log_command(&self) -> String {
        LOG_COMMAND.to_string()
    }
}
