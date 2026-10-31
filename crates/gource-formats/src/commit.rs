//! `RCommit` / `RCommitFile` (src/formats/commitlog.cpp:290-384).

use crate::settings::LogSettings;
use gource_util::math::Vec3;
use gource_util::strhash::colour_hash;
use gource_util::utf8::filter_utf8;

#[derive(Clone, Debug, PartialEq)]
pub struct RCommitFile {
    /// UTF-8 filtered, leading `/` prepended if missing (also for an empty name). No other
    /// normalisation: backslashes, `//`, `./` and a trailing `/` (directory marker) are kept.
    pub filename: String,
    /// Whatever the parser stored: `"A"`, `"D"`, `"M"`, or another letter (treated as modify).
    pub action: String,
    /// 0..1 components, not normalised when it came from a custom log colour field.
    pub colour: Vec3,
}

impl RCommitFile {
    pub fn new(filename: &[u8], action: &str, colour: Vec3) -> RCommitFile {
        let mut filename = filter_utf8(filename);
        if !filename.starts_with('/') {
            filename.insert(0, '/');
        }
        RCommitFile { filename, action: action.to_string(), colour }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RCommit {
    /// `time_t`.
    pub timestamp: i64,
    /// Raw bytes while parsing (grouping compares them raw); valid UTF-8 after
    /// [`RCommit::postprocess`], which `next_commit` always runs.
    pub username: Vec<u8>,
    /// Insertion order; no sorting anywhere in the log layer.
    pub files: Vec<RCommitFile>,
}

impl RCommit {
    pub fn username_str(&self) -> &str {
        std::str::from_utf8(&self.username).unwrap_or("???")
    }

    /// `RCommit::fileColour`: colour of the extension after the last `.` if that dot is after
    /// the last `/` and not the final character, else white. Computed on the raw filename.
    pub fn file_colour(filename: &[u8], hash_seed: i32) -> Vec3 {
        let slash = filename.iter().rposition(|&b| b == b'/');
        let dot = filename.iter().rposition(|&b| b == b'.');
        match dot {
            Some(dot) if dot + 1 < filename.len() && slash.is_none_or(|s| s < dot) => {
                colour_hash(&filename[dot + 1..], hash_seed)
            }
            _ => [1.0, 1.0, 1.0],
        }
    }

    /// `addFile(filename, action)`.
    pub fn add_file(&mut self, filename: &[u8], action: &str, settings: &LogSettings) {
        let colour = RCommit::file_colour(filename, settings.hash_seed);
        self.add_file_coloured(filename, action, colour, settings);
    }

    /// `addFile(filename, action, colour)`: file filters run on the raw filename.
    pub fn add_file_coloured(&mut self, filename: &[u8], action: &str, colour: Vec3, settings: &LogSettings) {
        if settings.file_filters.iter().any(|r| r.is_match(filename)) {
            return;
        }
        if settings.file_show_filters.iter().any(|r| !r.is_match(filename)) {
            return;
        }
        self.files.push(RCommitFile::new(filename, action, colour));
    }

    /// `postprocess`: UTF-8 filter the username.
    pub fn postprocess(&mut self) {
        self.username = filter_utf8(&self.username).into_bytes();
    }

    /// `isValid`: user filters, then at least one file.
    pub fn is_valid(&self, settings: &LogSettings) -> bool {
        if settings.user_filters.iter().any(|r| r.is_match(&self.username)) {
            return false;
        }
        if settings.user_show_filters.iter().any(|r| !r.is_match(&self.username)) {
            return false;
        }
        !self.files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gource_util::regex::Regex;

    #[test]
    fn commit_file_prepends_slash_and_filters_utf8() {
        let f = RCommitFile::new(b"path/to/1.txt", "A", [1.0, 1.0, 1.0]);
        assert_eq!(f.filename, "/path/to/1.txt");
        assert_eq!(RCommitFile::new(b"/x", "D", [0.0; 3]).filename, "/x");
        assert_eq!(RCommitFile::new(b"", "A", [0.0; 3]).filename, "/");
        assert_eq!(RCommitFile::new(b"a\xffb/", "A", [0.0; 3]).filename, "/a?b/");
        assert_eq!(RCommitFile::new(b".\\x//y/", "A", [0.0; 3]).filename, "/.\\x//y/");
    }

    #[test]
    fn file_colour_rules() {
        let white = [1.0, 1.0, 1.0];
        assert_eq!(RCommit::file_colour(b"a/b", 31), white);
        assert_eq!(RCommit::file_colour(b"a.txt/b", 31), white);
        assert_eq!(RCommit::file_colour(b"a/b.", 31), white);
        assert_eq!(RCommit::file_colour(b"a/b.txt", 31), colour_hash(b"txt", 31));
        assert_eq!(RCommit::file_colour(b".txt", 31), colour_hash(b"txt", 31));
    }

    #[test]
    fn filters() {
        let mut s = LogSettings::default();
        s.file_filters.push(Regex::new(r"\.cpp$").unwrap());
        let mut c = RCommit::default();
        c.add_file(b"a.cpp", "A", &s);
        c.add_file(b"a.h", "A", &s);
        assert_eq!(c.files.len(), 1);
        assert!(!RCommit::default().is_valid(&s));
        let mut s = LogSettings::default();
        s.user_show_filters.push(Regex::new("^bob").unwrap());
        s.user_show_filters.push(Regex::new("b$").unwrap());
        c.username = b"bobb".to_vec();
        assert!(c.is_valid(&s));
        c.username = b"bob smith".to_vec();
        assert!(!c.is_valid(&s));
        // empty username never matches any filter, even one that would match ""
        let mut s = LogSettings::default();
        s.user_filters.push(Regex::new("^$").unwrap());
        c.username.clear();
        assert!(c.is_valid(&s));
    }
}
