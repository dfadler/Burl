//! `src/formats/git.cpp`: the default format. First byte `u`. Generates its log from a
//! directory containing `.git` (dir or file) with cwd = repo.

use crate::commit::RCommit;
use crate::commitlog::{CommitLog, CommitLogBase};
use crate::seeklog::SeekLogError;
use crate::settings::LogSettings;
use gource_util::cstr::{atoi, atoll};
use gource_util::regex::Regex;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};

static GIT_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"([0-9]+)(?:\.([0-9]+))?(?:\.([0-9]+))?"));
static GIT_VERSION: Mutex<(i32, i32, i32)> = Mutex::new((0, 0, 0));

/// `readGitVersion`: `git --version > tmp`, cached once the major is known.
fn read_git_version() -> (i32, i32, i32) {
    let mut cached = GIT_VERSION.lock().unwrap_or_else(|e| e.into_inner());
    if cached.0 != 0 {
        return *cached;
    }
    let Some(temp) = CommitLogBase::create_temp_file() else { return *cached };
    let rc = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("git --version > {}", temp.display()))
        .status();
    let first_line = std::fs::read(&temp)
        .ok()
        .map(|b| b.split(|&c| c == b'\n').next().unwrap_or(&[]).to_vec());
    let _ = std::fs::remove_file(&temp);
    if !matches!(rc, Ok(st) if st.success()) {
        return *cached;
    }
    let Some(line) = first_line else { return *cached };
    let Some(entries) = GIT_VERSION_REGEX.captures(&line) else { return *cached };
    cached.0 = atoi(&entries[0]);
    if entries.len() > 1 {
        cached.1 = atoi(&entries[1]);
    }
    if entries.len() > 2 {
        cached.2 = atoi(&entries[2]);
    }
    *cached
}

/// `GitCommitLog::logCommand()` for a known git version (pure).
pub fn log_command_for(settings: &LogSettings, git_version: (i32, i32, i32)) -> String {
    let mut cmd = String::from("git log --reverse --raw --encoding=UTF-8 --no-renames");
    let (major, minor, _) = git_version;
    if major == 0 || major > 2 || (major == 2 && minor >= 10) {
        cmd.push_str(" --no-show-signature");
    }
    if settings.author_time {
        cmd.push_str(" --pretty=format:user:%aN%n%at");
    } else {
        cmd.push_str(" --pretty=format:user:%aN%n%ct");
    }
    if !settings.start_date.is_empty() {
        cmd.push_str(" --since ");
        cmd.push_str(&settings.start_date);
    }
    if !settings.stop_date.is_empty() {
        cmd.push_str(" --until ");
        cmd.push_str(&settings.stop_date);
    }
    if !settings.git_branch.is_empty() {
        cmd.push(' ');
        cmd.push_str(&settings.git_branch);
    }
    cmd
}

/// `GitCommitLog::logCommand()`: probes `git --version` (runs a process).
pub fn log_command(settings: &LogSettings) -> String {
    log_command_for(settings, read_git_version())
}

pub struct GitCommitLog {
    base: CommitLogBase,
}

impl GitCommitLog {
    pub fn new(logfile: &str, settings: Arc<LogSettings>) -> Result<GitCommitLog, SeekLogError> {
        let mut base = CommitLogBase::open(logfile, Some(b'u'), settings)?;
        if base.logf.is_none() && base.is_dir {
            let gitdir = Path::new(logfile).join(".git");
            let generatable = std::fs::metadata(&gitdir).map(|m| m.is_dir() || m.is_file()).unwrap_or(false);
            if generatable {
                let cmd = log_command(&base.settings);
                base.run_log_command(&cmd, Some(Path::new(logfile)))?;
            }
        }
        Ok(GitCommitLog { base })
    }
}

impl CommitLog for GitCommitLog {
    super::impl_base!(GitCommitLog);

    fn parse_commit(&mut self, commit: &mut RCommit) -> bool {
        let mut line = Vec::new();
        commit.username.clear();
        let settings = self.base.settings.clone();
        while self.base.raw_next_line(&mut line) && !line.is_empty() {
            if line.starts_with(b"user:") {
                commit.username = line[5..].to_vec();
                if !self.base.raw_next_line(&mut line) {
                    return false;
                }
                commit.timestamp = atoll(&line);
                // this isnt a commit we are parsing, abort
                if commit.timestamp == 0 {
                    return false;
                }
                continue;
            }
            // should see username before files
            if commit.username.is_empty() {
                return false;
            }
            let Some(tab) = line.iter().position(|&b| b == b'\t') else { continue };
            if tab == 0 || tab == line.len() - 1 {
                continue;
            }
            let status = String::from_utf8_lossy(&line[tab - 1..tab]).into_owned();
            let mut file = &line[tab + 1..];
            if file.is_empty() {
                continue;
            }
            // check for and remove double quotes
            if file[0] == b'"' && file[file.len() - 1] == b'"' {
                if file.len() <= 2 {
                    continue;
                }
                file = &file[1..file.len() - 1];
            }
            commit.add_file(file, &status, &settings);
        }
        !commit.username.is_empty()
    }

    fn log_command(&self) -> String {
        log_command(&self.base.settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commitlog::LogSource;
    use crate::seeklog::SeekLog;

    #[test]
    fn command_string() {
        let s = LogSettings::default();
        assert_eq!(
            log_command_for(&s, (2, 39, 0)),
            "git log --reverse --raw --encoding=UTF-8 --no-renames --no-show-signature --pretty=format:user:%aN%n%ct"
        );
        assert_eq!(
            log_command_for(&s, (2, 9, 5)),
            "git log --reverse --raw --encoding=UTF-8 --no-renames --pretty=format:user:%aN%n%ct"
        );
        let s = LogSettings {
            author_time: true,
            start_date: "2020-01-01".into(),
            stop_date: "2020-02-01".into(),
            git_branch: "main".into(),
            ..LogSettings::default()
        };
        assert_eq!(
            log_command_for(&s, (0, 0, 0)),
            "git log --reverse --raw --encoding=UTF-8 --no-renames --no-show-signature --pretty=format:user:%aN%n%at --since 2020-01-01 --until 2020-02-01 main"
        );
    }

    #[test]
    fn parses_blank_line_delimited_commits() {
        let text = b"user:Bob Smith\n1277787455\n:000000 100644 0000000 abcdef0 A\tpath/a.txt\n:100644 100644 abcdef0 1234567 M\t\"quoted name\"\n\nuser:Merge\n1277787460\n\nuser:Ann\n1277787470\n:000000 100644 0 1 D\tb\n";
        let mut base = CommitLogBase::open("/nonexistent/gource-test", Some(b'u'), Arc::new(LogSettings::default())).unwrap();
        base.logf = Some(LogSource::Seek(SeekLog::from_bytes(text.to_vec())));
        base.seekable = true;
        base.success = true;
        let mut l = GitCommitLog { base };
        assert!(l.check_format());
        let mut out = Vec::new();
        while !l.is_finished() {
            let mut c = RCommit::default();
            if l.next_commit(&mut c, true) {
                out.push(c);
            }
        }
        assert_eq!(out.len(), 2); // the header-only merge commit is skipped by validation
        assert_eq!(out[0].username, b"Bob Smith");
        assert_eq!(out[0].files[1].filename, "/quoted name");
        assert_eq!(out[0].files[1].action, "M");
        assert_eq!(out[1].files[0].action, "D");
    }
}
