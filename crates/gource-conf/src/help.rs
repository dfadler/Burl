//! Help text printed by `-h` / `-H`. Must stay byte-identical to
//! `docs/rust-rewrite/discovery/golden/help-{short,long}.txt` (gource_settings.cpp:42-209).
//! Platform variants (Windows omits `--high-dpi`) are deferred.

pub const VERSION: &str = "0.57";
pub const HELP_SHORT: &str = include_str!("help-short.txt");
pub const HELP_LONG: &str = include_str!("help-long.txt");
