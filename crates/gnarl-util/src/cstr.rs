//! C library conversions the parsers rely on (`atoi`, `atol`, `atoll`): skip leading
//! whitespace, optional sign, digits up to the first non-digit, no error reporting.
//! Overflow is undefined in C; we saturate.

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `atoll` / `atol` (both 64-bit on the LP64 platforms Gource targets).
pub fn atoll(s: &[u8]) -> i64 {
    let mut i = 0;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut v: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        v = if neg {
            v.saturating_mul(10).saturating_sub(d)
        } else {
            v.saturating_mul(10).saturating_add(d)
        };
        i += 1;
    }
    v
}

/// `atoi`.
pub fn atoi(s: &[u8]) -> i32 {
    atoll(s).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atoll_matches_c() {
        assert_eq!(atoll(b"12abc"), 12);
        assert_eq!(atoll(b"  -5"), -5);
        assert_eq!(atoll(b"abc"), 0);
        assert_eq!(atoll(b"+7"), 7);
        assert_eq!(atoll(b""), 0);
        assert_eq!(atoll(b"1277787455"), 1277787455);
        assert_eq!(atoi(b"99999999999"), i32::MAX);
    }
}
