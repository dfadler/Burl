//! `utf8::replace_invalid(start, end, out, '?')` from the vendored utf8-cpp
//! (`src/core/utf8/checked.h:99-125`, `core.h:240-293`), as used by
//! `RCommitLog::filter_utf8` (commitlog.cpp:24-35). Not equivalent to
//! `String::from_utf8_lossy`: one `?` per bad sequence, and after an incomplete / overlong /
//! out-of-range sequence every following continuation byte is swallowed.

enum Utf8Err {
    NotEnoughRoom,
    InvalidLead,
    /// INCOMPLETE_SEQUENCE, OVERLONG_SEQUENCE, INVALID_CODE_POINT: all handled alike
    BadSequence,
}

fn is_trail(b: u8) -> bool {
    (b >> 6) == 2
}

fn sequence_length(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if (lead >> 5) == 0x6 {
        2
    } else if (lead >> 4) == 0xe {
        3
    } else if (lead >> 3) == 0x1e {
        4
    } else {
        0
    }
}

/// `validate_next`: returns the sequence length on success.
fn validate_next(s: &[u8]) -> Result<usize, Utf8Err> {
    let len = sequence_length(s[0]);
    if len == 0 {
        return Err(Utf8Err::InvalidLead);
    }
    let mut cp: u32 = match len {
        1 => s[0] as u32,
        2 => ((s[0] as u32) << 6) & 0x7ff,
        3 => ((s[0] as u32) << 12) & 0xffff,
        _ => ((s[0] as u32) << 18) & 0x1fffff,
    };
    for i in 1..len {
        if i >= s.len() {
            return Err(Utf8Err::NotEnoughRoom);
        }
        if !is_trail(s[i]) {
            return Err(Utf8Err::BadSequence);
        }
        let shift = 6 * (len - 1 - i);
        let mask: u32 = match (len, i) {
            (3, 1) => 0xfff,
            (4, 1) => 0x3ffff,
            (4, 2) => 0xfff,
            _ => 0x3f,
        };
        cp += ((s[i] as u32) << shift) & mask;
    }
    let valid = cp <= 0x10ffff && !(0xd800..=0xdfff).contains(&cp);
    if !valid {
        return Err(Utf8Err::BadSequence);
    }
    // is_overlong_sequence: chained else-if, not three independent tests
    let overlong = if cp < 0x80 {
        len != 1
    } else if cp < 0x800 {
        len != 2
    } else if cp < 0x10000 {
        len != 3
    } else {
        false
    };
    if overlong {
        return Err(Utf8Err::BadSequence);
    }
    Ok(len)
}

/// `replace_invalid(.., '?')` on raw bytes. The output is always valid UTF-8.
pub fn replace_invalid(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut start = 0;
    let end = input.len();
    while start != end {
        match validate_next(&input[start..]) {
            Ok(len) => {
                out.extend_from_slice(&input[start..start + len]);
                start += len;
            }
            Err(Utf8Err::NotEnoughRoom) => {
                out.push(b'?');
                start = end;
            }
            Err(Utf8Err::InvalidLead) => {
                out.push(b'?');
                start += 1;
            }
            Err(Utf8Err::BadSequence) => {
                out.push(b'?');
                start += 1;
                while start != end && is_trail(input[start]) {
                    start += 1;
                }
            }
        }
    }
    out
}

/// `RCommitLog::filter_utf8`: the `catch(...) { "???" }` arm cannot fire in utf8-cpp, but
/// `String::from_utf8` failing would be the equivalent internal error, so it maps there.
pub fn filter_utf8(input: &[u8]) -> String {
    String::from_utf8(replace_invalid(input)).unwrap_or_else(|_| "???".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(b: &[u8]) -> String {
        filter_utf8(b)
    }

    #[test]
    fn valid_passthrough() {
        let s = "Árvíztűrő tükörfúrógép/💩\0x";
        assert_eq!(f(s.as_bytes()), s);
    }

    #[test]
    fn vectors_distinguishing_from_lossy() {
        assert_eq!(f(b"\xC3\x28"), "?(");
        assert_eq!(f(b"\xC0\xAF"), "?"); // overlong: one ?, continuation swallowed
        assert_eq!(f(b"\xE2\x82"), "?"); // truncated at end: NOT_ENOUGH_ROOM
        assert_eq!(f(b"\xED\xA0\x80"), "?"); // surrogate
        assert_eq!(f(b"\xF5\x80\x80\x80"), "?"); // > U+10FFFF
        assert_eq!(f(b"a\x80\x80b"), "a??b"); // each stray continuation is an invalid lead
        assert_eq!(f(b"\xFFx"), "?x");
        assert_eq!(f(b"\xE2\x82x"), "?x"); // incomplete then non-trail: ? then x
        assert_eq!(f(b"\xE2\x82\xAC\x80"), "€?");
        assert_eq!(String::from_utf8_lossy(b"\xC0\xAF"), "\u{FFFD}\u{FFFD}"); // the difference
    }
}
