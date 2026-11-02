//! `RCommitLog` (src/formats/commitlog.cpp:40-286): the state every parser shares and the
//! `checkFormat` / `nextCommit` / `isFinished` logic. Parsers implement [`CommitLog`] by
//! providing `parse_commit` and access to their [`CommitLogBase`].

use crate::commit::RCommit;
use crate::seeklog::{peek_file, SeekLog, SeekLogError, StreamLog};
use crate::settings::LogSettings;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub enum LogSource {
    Seek(SeekLog),
    Stream(StreamLog),
}

impl LogSource {
    pub fn next_line(&mut self, line: &mut Vec<u8>) -> bool {
        match self {
            LogSource::Seek(l) => l.next_line(line),
            LogSource::Stream(l) => l.next_line(line),
        }
    }
}

#[derive(Debug)]
pub struct CommitLogBase {
    pub logf: Option<LogSource>,
    pub seekable: bool,
    pub success: bool,
    pub is_dir: bool,
    buffered: bool,
    last_commit: RCommit,
    /// Push-back line (custom, hg grouping).
    pub lastline: Vec<u8>,
    temp_file: Option<PathBuf>,
    pub settings: Arc<LogSettings>,
}

/// `checkFirstChar`: no expectation, or the peeked byte equals it.
fn check_first_char(expected: Option<u8>, actual: Option<u8>) -> bool {
    match expected {
        None => true,
        Some(e) => actual == Some(e),
    }
}

impl CommitLogBase {
    /// The `RCommitLog(logfile, firstChar)` constructor. `"-"` is stdin (non-seekable). A
    /// path that `stat`s as a directory sets `is_dir` for the parser's generator; a regular
    /// file is peeked and, if the first byte matches, loaded whole. An unreadable regular
    /// file surfaces as [`SeekLogError`] ("unable to read log file"); a missing path or a
    /// first-byte mismatch leaves `success == false` ("unsupported log format").
    pub fn open(logfile: &str, first_char: Option<u8>, settings: Arc<LogSettings>) -> Result<CommitLogBase, SeekLogError> {
        let mut base = CommitLogBase {
            logf: None,
            seekable: false,
            success: false,
            is_dir: false,
            buffered: false,
            last_commit: RCommit::default(),
            lastline: Vec::new(),
            temp_file: None,
            settings,
        };
        if logfile == "-" {
            if check_first_char(first_char, StreamLog::peek()) {
                base.logf = Some(LogSource::Stream(StreamLog));
                base.success = true;
            }
            return Ok(base);
        }
        if let Ok(meta) = std::fs::metadata(logfile) {
            base.is_dir = meta.is_dir();
            if !base.is_dir && check_first_char(first_char, peek_file(logfile)) {
                base.logf = Some(LogSource::Seek(SeekLog::open(logfile)?));
                base.seekable = true;
                base.success = true;
            }
        }
        Ok(base)
    }

    /// `RCommitLog::getNextLine`: the pushed-back line first, then the log.
    pub fn next_line(&mut self, line: &mut Vec<u8>) -> bool {
        if !self.lastline.is_empty() {
            line.clear();
            line.append(&mut self.lastline);
            return true;
        }
        self.raw_next_line(line)
    }

    /// `logf->getNextLine`: bypasses the push-back (git, gitraw, bzr, cvs-exp, apache).
    pub fn raw_next_line(&mut self, line: &mut Vec<u8>) -> bool {
        match self.logf.as_mut() {
            Some(l) => l.next_line(line),
            None => false,
        }
    }

    fn seek_log(&mut self) -> Option<&mut SeekLog> {
        match self.logf.as_mut() {
            Some(LogSource::Seek(l)) => Some(l),
            _ => None,
        }
    }

    /// `createTempFile`: `mkstemp("/tmp/gource-XXXXXX")`.
    pub fn create_temp_file() -> Option<PathBuf> {
        let base = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        for attempt in 0..100u32 {
            let name = format!("gource-{:06x}", (nanos.wrapping_add(pid).wrapping_add(attempt * 7919)) & 0xffffff);
            let path = base.join(name);
            if std::fs::OpenOptions::new().write(true).create_new(true).open(&path).is_ok() {
                return Some(path);
            }
        }
        None
    }

    /// `generateLog` tail shared by every VCS: `system("<command> > <tmp>")` (optionally
    /// with the repository as cwd), then a `SeekLog` over the temp file. `Ok(false)` when
    /// the command could not run or exited non-zero; the temp file is removed on drop.
    pub fn run_log_command(&mut self, command: &str, cwd: Option<&Path>) -> Result<bool, SeekLogError> {
        let Some(temp) = CommitLogBase::create_temp_file() else { return Ok(false) };
        self.temp_file = Some(temp.clone());
        let shell_cmd = format!("{} > {}", command, temp.display());
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(&shell_cmd);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        let ok = matches!(cmd.status(), Ok(st) if st.success());
        if !ok {
            return Ok(false);
        }
        let temp_str = temp.to_string_lossy().into_owned();
        self.logf = Some(LogSource::Seek(SeekLog::open(&temp_str)?));
        self.seekable = true;
        self.success = true;
        Ok(true)
    }
}

impl Drop for CommitLogBase {
    fn drop(&mut self) {
        if let Some(t) = self.temp_file.take() {
            let _ = std::fs::remove_file(t);
        }
    }
}

pub trait CommitLog {
    fn base(&self) -> &CommitLogBase;
    fn base_mut(&mut self) -> &mut CommitLogBase;
    /// Format-specific `parseCommit`.
    fn parse_commit(&mut self, commit: &mut RCommit) -> bool;
    /// `getLogCommand`: the external command that would generate this format.
    fn log_command(&self) -> String;

    /// `checkFormat`: parse one commit unvalidated; seekable logs rewind, stdin keeps it
    /// buffered (so the first commit later bypasses `isValid`).
    fn check_format(&mut self) -> bool {
        if !self.base().success {
            return false;
        }
        let mut commit = RCommit::default();
        if !self.next_commit(&mut commit, false) {
            return false;
        }
        let base = self.base_mut();
        base.last_commit = commit;
        if base.seekable {
            if let Some(l) = base.seek_log() {
                l.seek_to(0.0);
            }
            base.lastline.clear();
        } else {
            base.buffered = true;
        }
        true
    }

    /// `nextCommit(commit, validate)`.
    fn next_commit(&mut self, commit: &mut RCommit, validate: bool) -> bool {
        {
            let base = self.base_mut();
            if base.buffered {
                *commit = base.last_commit.clone();
                base.buffered = false;
                return true;
            }
        }
        *commit = RCommit::default();
        if !self.parse_commit(commit) {
            return false;
        }
        commit.postprocess();
        if validate {
            return commit.is_valid(&self.base().settings);
        }
        true
    }

    /// `isFinished`: only a seekable log can finish.
    fn is_finished(&mut self) -> bool {
        let base = self.base_mut();
        base.seekable && matches!(base.logf.as_ref(), Some(LogSource::Seek(l)) if l.is_finished())
    }

    fn is_seekable(&self) -> bool {
        self.base().seekable
    }

    /// `seekTo(percent)`.
    fn seek_to(&mut self, percent: f32) {
        let base = self.base_mut();
        if !base.seekable {
            return;
        }
        base.lastline.clear();
        if let Some(l) = base.seek_log() {
            l.seek_to(percent);
        }
    }

    fn get_percent(&mut self) -> f32 {
        match self.base_mut().seek_log() {
            Some(l) => l.get_percent(),
            None => 0.0,
        }
    }

    /// `bufferCommit`.
    fn buffer_commit(&mut self, commit: RCommit) {
        let base = self.base_mut();
        base.last_commit = commit;
        base.buffered = true;
    }

    fn has_buffered_commit(&self) -> bool {
        self.base().buffered
    }
}
