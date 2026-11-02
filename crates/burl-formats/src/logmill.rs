//! `RLogMill::fetchLog` / `findRepository` / error selection (src/logmill.cpp:65-337),
//! synchronous. The logmill thread and the `start_timestamp` skip loop come with the sim.

use crate::commitlog::CommitLog;
use crate::formats::{apache, bzr, custom, cvs2cl, cvs_exp, git, gitraw, hg, svn};
use crate::seeklog::SeekLogError;
use crate::settings::LogSettings;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The message `SDLAppQuit` prints (`gource: <msg>`). Empty = no log and no message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogMillError(pub String);

/// `findRepository`: canonicalise, then walk up looking for `.git` (dir or file), `.hg`,
/// `.bzr`, `.svn` in that order; nearest ancestor wins.
pub fn find_repository(dir: &Path) -> Option<(PathBuf, &'static str)> {
    let mut dir = std::fs::canonicalize(dir).ok()?;
    while dir.is_dir() {
        let git = dir.join(".git");
        let fmt = if git.is_dir() || git.is_file() {
            Some("git")
        } else if dir.join(".hg").is_dir() {
            Some("hg")
        } else if dir.join(".bzr").is_dir() {
            Some("bzr")
        } else if dir.join(".svn").is_dir() {
            Some("svn")
        } else {
            None
        };
        if let Some(fmt) = fmt {
            return Some((dir, fmt));
        }
        dir = dir.parent()?.to_path_buf();
    }
    None
}

type Attempt = Result<Option<Box<dyn CommitLog>>, SeekLogError>;

fn try_log<L: CommitLog + 'static>(log: Result<L, SeekLogError>) -> Attempt {
    let mut log = log?;
    if log.check_format() {
        Ok(Some(Box::new(log)))
    } else {
        Ok(None)
    }
}

macro_rules! attempt {
    ($e:expr) => {
        if let Some(l) = try_log($e)? {
            return Ok(Some(l));
        }
    };
}

fn fetch(logfile: &str, log_format: &str, s: &Arc<LogSettings>) -> Attempt {
    if !log_format.is_empty() {
        match log_format {
            "git" => {
                attempt!(git::GitCommitLog::new(logfile, s.clone()));
                attempt!(gitraw::GitRawCommitLog::new(logfile, s.clone()));
            }
            "hg" => attempt!(hg::MercurialLog::new(logfile, s.clone())),
            "bzr" => attempt!(bzr::BazaarLog::new(logfile, s.clone())),
            // only "cvs" reaches cvs-exp here, and settings reject "cvs": preserved C++ bug
            "cvs" => attempt!(cvs_exp::CVSEXPCommitLog::new(logfile, s.clone())),
            "custom" => attempt!(custom::CustomLog::new(logfile, s.clone())),
            "apache" => attempt!(apache::ApacheCombinedLog::new(logfile, s.clone())),
            "svn" => attempt!(svn::SVNCommitLog::new(logfile, s.clone())),
            "cvs2cl" => attempt!(cvs2cl::CVS2CLCommitLog::new(logfile, s.clone())),
            _ => {}
        }
        return Ok(None);
    }
    // try different formats until one works
    attempt!(git::GitCommitLog::new(logfile, s.clone()));
    attempt!(hg::MercurialLog::new(logfile, s.clone()));
    attempt!(bzr::BazaarLog::new(logfile, s.clone()));
    attempt!(gitraw::GitRawCommitLog::new(logfile, s.clone()));
    attempt!(cvs_exp::CVSEXPCommitLog::new(logfile, s.clone()));
    attempt!(svn::SVNCommitLog::new(logfile, s.clone()));
    attempt!(cvs2cl::CVS2CLCommitLog::new(logfile, s.clone()));
    attempt!(custom::CustomLog::new(logfile, s.clone()));
    attempt!(apache::ApacheCombinedLog::new(logfile, s.clone()));
    Ok(None)
}

/// `RLogMill::run` minus the thread and the start-date skip: open `logfile` (a path or
/// `"-"`) in `settings.log_format` (empty = detect), or the exact C++ error message.
pub fn fetch_log(logfile: &str, settings: Arc<LogSettings>) -> Result<Box<dyn CommitLog>, LogMillError> {
    let mut logfile = logfile.to_string();
    let mut log_format = settings.log_format.clone();
    if log_format.is_empty() && logfile != "-" && Path::new(&logfile).is_dir() {
        if let Some((root, fmt)) = find_repository(Path::new(&logfile)) {
            logfile = root.to_string_lossy().into_owned();
            log_format = fmt.to_string();
        }
    }
    match fetch(&logfile, &log_format, &settings) {
        Ok(Some(log)) => Ok(log),
        Err(_) => Err(LogMillError("unable to read log file".to_string())),
        Ok(None) => {
            let msg = if Path::new(&logfile).is_dir() {
                if !log_format.is_empty() {
                    if settings.start_timestamp != 0 || settings.stop_timestamp != 0 {
                        "failed to generate log file for the specified time period"
                    } else {
                        "failed to generate log file"
                    }
                } else {
                    "directory not supported"
                }
            } else {
                "unsupported log format (you may need to regenerate your log file)"
            };
            Err(LogMillError(msg.to_string()))
        }
    }
}
