//! `src/formats/custom.cpp`: `timestamp|username|action|filename[|colour]`, grouped by
//! consecutive identical (timestamp, username). Also the `--output-custom-log` format.

use crate::commit::RCommit;
use crate::commitlog::{CommitLog, CommitLogBase};
use crate::seeklog::SeekLogError;
use crate::settings::LogSettings;
use gource_util::cstr::atoll;
use gource_util::math::Vec3;
use gource_util::regex::Regex;
use gource_util::timeparse::parse_date_time;
use std::sync::{Arc, LazyLock};

static CUSTOM_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::must(r"^(?:\xEF\xBB\xBF)?([^|]+)\|([^|]*)\|([ADM]?)\|([^|]+)(?:\|#?([a-fA-F0-9]{6}))?"));

pub struct CustomLog {
    base: CommitLogBase,
}

impl CustomLog {
    pub fn new(logfile: &str, settings: Arc<LogSettings>) -> Result<CustomLog, SeekLogError> {
        Ok(CustomLog { base: CommitLogBase::open(logfile, None, settings)? })
    }

    /// `parseColour`: `sscanf("%02x%02x%02x") / 255`; the regex guarantees 6 hex digits.
    pub fn parse_colour(cstr: &[u8]) -> Vec3 {
        let hex = |i: usize| u8::from_str_radix(std::str::from_utf8(&cstr[i..i + 2]).unwrap_or("zz"), 16);
        match (hex(0), hex(2), hex(4)) {
            (Ok(r), Ok(g), Ok(b)) => [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0],
            _ => [0.0, 0.0, 0.0],
        }
    }

    fn parse_commit_entry(&mut self, commit: &mut RCommit) -> bool {
        let mut line = Vec::new();
        if !self.base.next_line(&mut line) {
            return false;
        }
        let Some(entries) = CUSTOM_REGEX.captures(&line) else { return false };

        // Allow timestamp to be a string
        let ts = &entries[0];
        let timestamp = if ts.len() > 1 && ts[1..].contains(&b'-') {
            match parse_date_time(ts) {
                Some(t) => t,
                None => return false,
            }
        } else {
            let t = atoll(ts);
            if t == 0 && ts.as_slice() != b"0" {
                return false;
            }
            t
        };

        let username: &[u8] = if entries[1].is_empty() { b"Unknown" } else { &entries[1] };
        let action = if entries[2].is_empty() { "A" } else { std::str::from_utf8(&entries[2]).unwrap_or("A") };

        if commit.files.is_empty() {
            commit.timestamp = timestamp;
            commit.username = username.to_vec();
        } else if commit.timestamp != timestamp || commit.username != username {
            self.base.lastline = line;
            return false;
        }

        let settings = self.base.settings.clone();
        if entries.len() >= 5 && !entries[4].is_empty() {
            let colour = CustomLog::parse_colour(&entries[4]);
            commit.add_file_coloured(&entries[3], action, colour, &settings);
        } else {
            commit.add_file(&entries[3], action, &settings);
        }
        true
    }
}

impl CommitLog for CustomLog {
    super::impl_base!(CustomLog);

    fn parse_commit(&mut self, commit: &mut RCommit) -> bool {
        while self.parse_commit_entry(commit) {}
        !commit.files.is_empty()
    }

    fn log_command(&self) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commitlog::LogSource;
    use crate::seeklog::SeekLog;

    fn log(bytes: &[u8]) -> CustomLog {
        let mut base = CommitLogBase::open("/nonexistent/gource-test", None, Arc::new(LogSettings::default())).unwrap();
        base.logf = Some(LogSource::Seek(SeekLog::from_bytes(bytes.to_vec())));
        base.seekable = true;
        base.success = true;
        CustomLog { base }
    }

    fn commits(bytes: &[u8]) -> Vec<RCommit> {
        let mut l = log(bytes);
        assert!(l.check_format());
        let mut out = Vec::new();
        while !l.is_finished() {
            let mut c = RCommit::default();
            if l.next_commit(&mut c, true) {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn grouping_and_pushback() {
        let cs = commits(b"1|bob|A|a\n1|bob|M|b\n2|bob|D|c\n1|bob|A|d\n");
        assert_eq!(cs.len(), 3);
        assert_eq!(cs[0].files.len(), 2);
        assert_eq!(cs[0].files[1].action, "M");
        assert_eq!(cs[1].files[0].filename, "/c");
        assert_eq!(cs[2].files[0].filename, "/d");
    }

    #[test]
    fn bom_colour_defaults_and_trailing_text() {
        let cs = commits(b"\xEF\xBB\xBF1||\x7c/a.txt|#ff8000\n1||\x7c/b|zzz\n1||\x7c/c|abcdef junk\n");
        assert_eq!(cs.len(), 1);
        let c = &cs[0];
        assert_eq!(c.username, b"Unknown");
        assert_eq!(c.files[0].action, "A");
        assert_eq!(c.files[0].colour, [1.0, 128.0 / 255.0, 0.0]);
        assert_eq!(c.files[1].colour, RCommit::file_colour(b"/b", 31));
        assert_eq!(c.files[2].colour, [0xab as f32 / 255.0, 0xcd as f32 / 255.0, 0xef as f32 / 255.0]);
    }

    #[test]
    fn malformed_lines_end_and_drop() {
        // bad action, missing fields, non-numeric timestamp, "12abc" -> 12, "0" accepted, "-5" accepted
        let cs = commits(b"1|bob|A|a\n1|bob|R|a\n\n1|bob\n2|bob|A|b\nabc|bob|A|c\n12abc|bob|A|d\n0|bob|A|e\n-5|bob|A|f\n");
        let ts: Vec<i64> = cs.iter().map(|c| c.timestamp).collect();
        assert_eq!(ts, vec![1, 2, 12, 0, -5]);
        // a malformed FIRST line rejects the format
        assert!(!log(b"1|bob|R|a\n1|bob|A|a\n").check_format());
        // pipe in a filename truncates it; a blank first line rejects the format
        let cs = commits(b"1|bob|A|a|b|c\n");
        assert_eq!(cs[0].files[0].filename, "/a");
        assert!(!log(b"\n1|bob|A|a\n").check_format());
    }

    #[test]
    fn date_string_timestamps() {
        let cs = commits(b"2021-11-01Z|bob|A|a\n2021-11-01 12:01:59+5:30|bob|A|b\n");
        assert_eq!(cs[0].timestamp, 1635724800);
        assert_eq!(cs[1].timestamp, 1635748319);
        assert!(!log(b"2021-13|bob|A|a\n").check_format());
    }
}
