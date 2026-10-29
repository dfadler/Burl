//! The slice of `GourceSettings` the log layer reads (gource_settings.cpp). Phase 1 never
//! sets anything but `log_format`; the rest keeps its C++ default.

use gource_util::regex::Regex;
use gource_util::strhash::DEFAULT_HASH_SEED;

#[derive(Clone, Debug)]
pub struct LogSettings {
    /// `--log-format`; empty = auto-detect.
    pub log_format: String,
    /// `--file-filter` (any match drops the file), raw filename, before the leading `/`.
    pub file_filters: Vec<Regex>,
    /// `--file-show-filter` (every one must match).
    pub file_show_filters: Vec<Regex>,
    /// `--user-filter` (any match drops the commit), UTF-8 filtered username.
    pub user_filters: Vec<Regex>,
    /// `--user-show-filter` (every one must match).
    pub user_show_filters: Vec<Regex>,
    /// `gStringHashSeed` for `fileColour`.
    pub hash_seed: i32,
    /// `--start-date` / `--stop-date` as `YYYY-MM-DD` strings for the VCS commands
    /// (gource_settings.cpp:1271-1311, stop +1 day when it has a time part).
    pub start_date: String,
    pub stop_date: String,
    pub start_timestamp: i64,
    pub stop_timestamp: i64,
    /// `--git-branch`, appended verbatim to the git command.
    pub git_branch: String,
    /// `--author-time`: `%at` instead of `%ct`.
    pub author_time: bool,
    /// `gSDLAppResourceDir` (trailing separator included), for `gource.style`.
    pub resource_dir: String,
}

impl Default for LogSettings {
    fn default() -> Self {
        LogSettings {
            log_format: String::new(),
            file_filters: Vec::new(),
            file_show_filters: Vec::new(),
            user_filters: Vec::new(),
            user_show_filters: Vec::new(),
            hash_seed: DEFAULT_HASH_SEED,
            start_date: String::new(),
            stop_date: String::new(),
            start_timestamp: 0,
            stop_timestamp: 0,
            git_branch: String::new(),
            author_time: false,
            // ponytail: the C++ binary bakes SDLAPP_RESOURCE_DIR in at build time; phase 1
            // has no resource lookup yet, so the source-tree layout is the default.
            resource_dir: "data/".to_string(),
        }
    }
}
