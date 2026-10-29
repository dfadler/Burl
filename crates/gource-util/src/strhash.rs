//! `src/core/stringhash.cpp:28-83`, exact. `str[i]` is a signed `char` (x86/x64 and Apple
//! arm64), the arithmetic is signed 32-bit and wraps in practice, `-INT_MIN` stays `INT_MIN`
//! (so the hash can be negative and `/`, `%` then truncate toward zero, as in Rust).

use crate::math::{normalise2, normalise3, Vec2, Vec3};

/// `gStringHashSeed` default; `--hash-seed` and the recolour key change it.
pub const DEFAULT_HASH_SEED: i32 = 31;

pub fn string_hash(s: &[u8], seed: i32) -> i32 {
    let n = s.len() as i32;
    let mut val: i32 = 0;
    for i in 0..n {
        let c = s[i as usize] as i8 as i32;
        val = val.wrapping_add(c.wrapping_mul(seed ^ (n - i)));
    }
    if val < 0 {
        val = val.wrapping_neg();
    }
    val
}

pub fn colour_hash(s: &[u8], seed: i32) -> Vec3 {
    let mut hash = string_hash(s, seed);
    if hash == 0 {
        hash += 1;
    }
    let r = (hash / 7) % 255;
    let g = (hash / 3) % 255;
    let b = hash % 255;
    normalise3([r as f32, g as f32, b as f32])
}

pub fn vec2_hash(s: &[u8], seed: i32) -> Vec2 {
    let hash = string_hash(s, seed);
    let x = ((hash / 7) % 255) - 127;
    let y = ((hash / 3) % 255) - 127;
    normalise2([x as f32, y as f32])
}

pub fn vec3_hash(s: &[u8], seed: i32) -> Vec3 {
    let hash = string_hash(s, seed);
    let x = ((hash / 7) % 255) - 127;
    let y = ((hash / 3) % 255) - 127;
    let z = hash % 255;
    normalise3([x as f32, y as f32, z as f32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_computed_vectors() {
        // "a": 97 * (31 ^ 1) = 97 * 30
        assert_eq!(string_hash(b"a", 31), 2910);
        // "ab": 97 * (31^2=29) + 98 * (31^1=30)
        assert_eq!(string_hash(b"ab", 31), 2813 + 2940);
        // "txt": 116*28 + 120*29 + 116*30
        assert_eq!(string_hash(b"txt", 31), 10208);
        // signed char: 0xFF is -1 -> -30 -> abs
        assert_eq!(string_hash(b"\xff", 31), 30);
        assert_eq!(string_hash(b"", 31), 0);
    }

    #[test]
    fn wrapping_and_int_min() {
        // 2^31 overflow must wrap, not panic
        let s = vec![0x7fu8; 100_000];
        let _ = string_hash(&s, 31);
        let _ = string_hash(&s, i32::MAX);
    }

    #[test]
    fn colour_hash_txt() {
        // hash 10208: r=(1458%255)=183, g=(3402%255)=87, b=10208%255=8, normalised
        let c = colour_hash(b"txt", 31);
        let v = normalise3([183.0, 87.0, 8.0]);
        assert_eq!(c, v);
        // empty name: hash 0 -> 1 -> (0,0,1)
        assert_eq!(colour_hash(b"", 31), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn vec2_hash_zero_vector_stays_zero() {
        // x == y == 0 when (h/7)%255 == 127 and (h/3)%255 == 127: find one by search
        let mut found = false;
        for h in 0..200_000i32 {
            if (h / 7) % 255 == 127 && (h / 3) % 255 == 127 {
                let v = normalise2([((h / 7) % 255 - 127) as f32, ((h / 3) % 255 - 127) as f32]);
                assert_eq!(v, [0.0, 0.0]);
                found = true;
                break;
            }
        }
        assert!(found);
    }
}
