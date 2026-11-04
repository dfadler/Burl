//! `Logger` (`src/core/logger.cpp`): one process-wide level and the `%7s: msg` stderr line.
//!
//! The levels are the C++ enum in its own order, which is not severity order: `warn` (6) sits
//! above `debug` (5), so `--log-level warn` also prints every debug and info message. A message
//! prints when the configured level is at or above its own. Before `--log-level` is applied the
//! level is `error`, the C++ default logger's.

use std::sync::atomic::{AtomicU8, Ordering};

pub const OFF: u8 = 0;
pub const ERROR: u8 = 1;
pub const INFO: u8 = 3;
pub const DEBUG: u8 = 5;
pub const WARN: u8 = 6;
pub const PEDANTIC: u8 = 7;

static LEVEL: AtomicU8 = AtomicU8::new(ERROR);

/// `--log-level` value to level; anything else is ignored by the C++ (level stays `off`).
pub fn parse_level(value: &str) -> Option<u8> {
    Some(match value {
        "warn" => WARN,
        "debug" => DEBUG,
        "info" => INFO,
        "error" => ERROR,
        "pedantic" => PEDANTIC,
        _ => return None,
    })
}

pub fn set_level(level: u8) {
    LEVEL.store(level, Ordering::Relaxed);
}

/// The line `Logger::message` writes for `msg` at `level`, if the configured level lets it through.
pub fn format_line(configured: u8, level: u8, msg: &str) -> Option<String> {
    if level == OFF || configured < level {
        return None;
    }
    let name = match level {
        ERROR => "ERROR",
        2 => "CONSOLE",
        INFO => "INFO",
        4 => "SCRIPT",
        DEBUG => "DEBUG",
        WARN => "WARN",
        _ => "PEDANT",
    };
    Some(format!("{name:>7}: {msg}"))
}

pub fn log(level: u8, msg: &str) {
    if let Some(line) = format_line(LEVEL.load(Ordering::Relaxed), level, msg) {
        eprintln!("{line}");
    }
}

/// `debugLog`. Takes a closure so the message is only built when it will print.
pub fn debug(msg: impl FnOnce() -> String) {
    if LEVEL.load(Ordering::Relaxed) >= DEBUG {
        log(DEBUG, &msg());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!(parse_level("warn"), Some(WARN));
        assert_eq!(parse_level("pedantic"), Some(PEDANTIC));
        assert_eq!(parse_level("off"), None);
        assert_eq!(parse_level("bogus"), None);
    }

    #[test]
    fn filter_follows_enum_order_not_severity() {
        assert_eq!(format_line(OFF, ERROR, "x"), None);
        assert_eq!(format_line(INFO, DEBUG, "x"), None);
        assert_eq!(format_line(DEBUG, INFO, "x").as_deref(), Some("   INFO: x"));
        // warn is above debug in the enum, so it prints debug messages
        assert_eq!(format_line(WARN, DEBUG, "x").as_deref(), Some("  DEBUG: x"));
        assert_eq!(
            format_line(PEDANTIC, WARN, "x").as_deref(),
            Some("   WARN: x")
        );
        assert_eq!(
            format_line(ERROR, ERROR, "x").as_deref(),
            Some("  ERROR: x")
        );
    }
}
