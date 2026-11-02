//! `SDLAppSettings::parseDateTime` (core/settings.cpp:284-363), `mktime_utc`
//! (core/timezone.cpp:41) and the `struct tm` -> epoch conversions the parsers use
//! (`mktime` local, `timegm` UTC). C's `mktime`/`timegm` normalise out-of-range fields
//! (month 13, hour 99, day 0); so do these.
//!
//! Local time comes from `chrono::Local`, which honours the `TZ` environment variable the
//! way the C library does. The parsing itself is generic over the zone so tests can pin
//! `Utc` / `FixedOffset` instead of mutating the process environment.

use crate::cstr::atoi;
use crate::regex::Regex;
use chrono::{DateTime, LocalResult, NaiveDateTime, Offset, TimeDelta, TimeZone};
use std::sync::LazyLock;

/// Broken-down time with a full year and a 0-based month, like `struct tm`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tm {
    pub year: i64,
    pub mon: i64,
    pub mday: i64,
    pub hour: i64,
    pub min: i64,
    pub sec: i64,
}

// Howard Hinnant's days_from_civil; m in 1..=12.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// `timegm` / `mktime_utc` with field normalisation.
pub fn timegm(tm: &Tm) -> i64 {
    let y = tm.year + tm.mon.div_euclid(12);
    let m = tm.mon.rem_euclid(12) + 1;
    (days_from_civil(y, m, 1) + tm.mday - 1) * 86400 + tm.hour * 3600 + tm.min * 60 + tm.sec
}

/// `mktime` with `tm_isdst = -1` in zone `tz`. Ambiguous wall times take the earlier
/// instant; nonexistent ones (DST gap) use the offset in force just before the gap, which
/// is what glibc produces. Neither case is covered by a C++ golden.
pub fn mktime_tz<Tz: TimeZone>(tm: &Tm, tz: &Tz) -> i64 {
    let naive_secs = timegm(tm);
    let naive: NaiveDateTime = match DateTime::from_timestamp(naive_secs, 0) {
        Some(dt) => dt.naive_utc(),
        None => return naive_secs,
    };
    let offset = match tz.offset_from_local_datetime(&naive) {
        LocalResult::Single(o) => o.fix().local_minus_utc(),
        LocalResult::Ambiguous(a, _) => a.fix().local_minus_utc(),
        LocalResult::None => {
            let before = naive - TimeDelta::hours(1);
            match tz.offset_from_local_datetime(&before) {
                LocalResult::Single(o) | LocalResult::Ambiguous(o, _) => o.fix().local_minus_utc(),
                LocalResult::None => 0,
            }
        }
    };
    naive_secs - offset as i64
}

/// `mktime` in the process-local zone (`TZ`).
pub fn mktime_local(tm: &Tm) -> i64 {
    mktime_tz(tm, &chrono::Local)
}

static TIMESTAMP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::must(r"^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{1,2}):(\d{2})(?::(\d{2}(?:\.\d+)?))?)?(Z| ?([+-])(\d{1,2})(?::?(\d{2}))?)?$")
});

/// `parseDateTime` with an explicit local zone.
pub fn parse_date_time_tz<Tz: TimeZone>(datetime: &[u8], local: &Tz) -> Option<i64> {
    let results = TIMESTAMP_REGEX.captures(datetime)?;
    if results.len() < 3 {
        return None;
    }
    let mut tm = Tm {
        year: atoi(&results[0]) as i64,
        mon: atoi(&results[1]) as i64 - 1,
        mday: atoi(&results[2]) as i64,
        ..Tm::default()
    };
    if results.len() >= 5 && !results[3].is_empty() {
        tm.hour = atoi(&results[3]) as i64;
        tm.min = atoi(&results[4]) as i64;
        if results.len() >= 6 {
            tm.sec = atoi(&results[5]) as i64;
        }
    }
    let timestamp = if results.len() == 7 && results[6] == b"Z" {
        timegm(&tm)
    } else if results.len() >= 9 {
        let tz_hour = atoi(&results[8]) as i64;
        let tz_min = if results.len() >= 10 { atoi(&results[9]) as i64 } else { 0 };
        let mut tz_offset = tz_hour * 3600 + tz_min * 60;
        if results[7] == b"-" {
            tz_offset = -tz_offset;
        }
        timegm(&tm) - tz_offset
    } else {
        mktime_tz(&tm, local)
    };
    Some(timestamp)
}

/// `parseDateTime` in the process-local zone.
pub fn parse_date_time(datetime: &[u8]) -> Option<i64> {
    parse_date_time_tz(datetime, &chrono::Local)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, Utc};

    #[test]
    fn timegm_normalises() {
        assert_eq!(timegm(&Tm { year: 1970, mon: 0, mday: 1, ..Tm::default() }), 0);
        assert_eq!(timegm(&Tm { year: 2021, mon: 10, mday: 1, ..Tm::default() }), 1635724800);
        // month 13 -> next January; day 0 -> last day of previous month; hour 25 -> next day
        assert_eq!(
            timegm(&Tm { year: 2021, mon: 12, mday: 1, ..Tm::default() }),
            timegm(&Tm { year: 2022, mon: 0, mday: 1, ..Tm::default() })
        );
        assert_eq!(
            timegm(&Tm { year: 2021, mon: 2, mday: 0, ..Tm::default() }),
            timegm(&Tm { year: 2021, mon: 1, mday: 28, ..Tm::default() })
        );
        assert_eq!(
            timegm(&Tm { year: 2021, mon: 0, mday: 1, hour: 25, ..Tm::default() }),
            timegm(&Tm { year: 2021, mon: 0, mday: 2, hour: 1, ..Tm::default() })
        );
        assert_eq!(timegm(&Tm { year: 2010, mon: 0, mday: 20, hour: 8, min: 35, sec: 18 }), 1263976518);
    }

    /// The 28 vectors from src/test/datetime_tests.cpp (unit-tests.md). Row 1 runs with
    /// TZ=UTC, all later rows with TZ=Pacific/Auckland, which is a constant +13:00 on
    /// 2021-11-01 (no transition that day), so a fixed offset reproduces it exactly.
    #[test]
    fn datetime_vectors() {
        let nz = FixedOffset::east_opt(13 * 3600).unwrap();
        assert_eq!(parse_date_time_tz(b"2021-11-01", &Utc), Some(1635724800));
        let rows: [(&str, i64); 27] = [
            ("2021-11-01", 1635678000),
            ("2021-11-01Z", 1635724800),
            ("2021-11-01+0", 1635724800),
            ("2021-11-01+13", 1635678000),
            ("2021-11-01 +13", 1635678000),
            ("2021-11-01+5:30", 1635705000),
            ("2021-11-01 12:01", 1635721260),
            ("2021-11-01 12:01Z", 1635768060),
            ("2021-11-01 12:01+0", 1635768060),
            ("2021-11-01 12:01+13", 1635721260),
            ("2021-11-01 12:01 +13", 1635721260),
            ("2021-11-01 12:01+5:30", 1635748260),
            ("2021-11-01 12:01:59", 1635721319),
            ("2021-11-01T12:01:59.123", 1635721319),
            ("2021-11-01 12:01:59Z", 1635768119),
            ("2021-11-01 12:01:59+0", 1635768119),
            ("2021-11-01 12:01:59+13", 1635721319),
            ("2021-11-01 12:01:59 +13", 1635721319),
            ("2021-11-01 12:01:59+5:30", 1635748319),
            ("2021-11-01T12:01:59", 1635721319),
            ("2021-11-01T12:01:59.123", 1635721319),
            ("2021-11-01T12:01:59Z", 1635768119),
            ("2021-11-01T12:01:59+0", 1635768119),
            ("2021-11-01T12:01:59+13", 1635721319),
            ("2021-11-01T12:01:59+13", 1635721319),
            ("2021-11-01T12:01:59+5:30", 1635748319),
            ("2021-11-01T12:01:59.123+5:30", 1635748319),
        ];
        for (input, expected) in rows {
            assert_eq!(parse_date_time_tz(input.as_bytes(), &nz), Some(expected), "{input}");
        }
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_date_time_tz(b"", &Utc), None);
        assert_eq!(parse_date_time_tz(b"2021-11-01x", &Utc), None);
        assert_eq!(parse_date_time_tz(b"1277787455", &Utc), None);
        assert_eq!(parse_date_time_tz(b"2010-06-29 16:57:45", &Utc), Some(1277830665));
    }
}
