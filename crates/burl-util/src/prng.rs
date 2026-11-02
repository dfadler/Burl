//! Simulation random numbers (decision D3, tickets #22, #11). The C++ calls libc `rand()` at four
//! sites (dirnode.cpp:625, user.cpp:106/128, gource.cpp:1555); the sim must draw at the same points
//! in the same order, `x` before `y` for `vec2(rand()%100-50, rand()%100-50)`.
//!
//! Modes (`--prng default|glibc|park`, `--rand-script FILE`; the CLI layer rejects combining them):
//! - **Default**: xorshift64* seeded through splitmix64, seed from the clock unless `--prng-seed N`
//!   is given, so unscripted runs vary like the C++. Algorithm is part of the D3 record: do not change.
//! - **Glibc**: glibc `rand()`, TYPE_3 additive feedback generator; `srand(0)` acts as seed 1.
//! - **Park**: the macOS libc `rand()`, Park-Miller minimal standard (16807, 2^31-1), with libc's
//!   substitution of 123459876 for seed 0.
//! - **Script**: integers read from a file, one per line, cycled; the primary parity-gate mode.
//!
//! Unseeded Glibc and Park behave like an unseeded libc (`srand(1)`).

use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrngMode {
    Default,
    Glibc,
    Park,
}

impl FromStr for PrngMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "default" => Ok(PrngMode::Default),
            "glibc" => Ok(PrngMode::Glibc),
            "park" => Ok(PrngMode::Park),
            _ => Err(format!(
                "unknown PRNG mode '{s}' (expected default, glibc or park)"
            )),
        }
    }
}

/// xorshift64* (Vigna): shifts 12/25/27, multiplier 0x2545F4914F6CDD1D. Output is the top 31 bits.
struct Xorshift64Star(u64);

impl Xorshift64Star {
    fn new(seed: u64) -> Self {
        // splitmix64 step so that small/zero seeds give a well-mixed non-zero state.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Xorshift64Star(if z == 0 { 0x9E37_79B9_7F4A_7C15 } else { z })
    }

    fn next(&mut self) -> i32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as i32
    }
}

/// glibc `random_r` TYPE_3: 31-word state, trinomial x^31 + x^3 + 1, 310 warm-up draws.
/// Same structure as `discovery/golden/scripts/glibc_rand.py`.
struct Glibc {
    r: [u32; 31],
    f: usize,
    b: usize,
}

impl Glibc {
    fn new(seed: u32) -> Self {
        let seed = if seed == 0 { 1 } else { seed };
        let mut r = [0u32; 31];
        let mut word = seed as i32 as i64;
        r[0] = word as u32;
        for slot in r.iter_mut().skip(1) {
            // Schrage's method; `/` and `%` truncate like C.
            let hi = word / 127773;
            let lo = word % 127773;
            word = 16807 * lo - 2836 * hi;
            if word < 0 {
                word += 2147483647;
            }
            *slot = word as u32;
        }
        let mut g = Glibc { r, f: 3, b: 0 };
        for _ in 0..310 {
            g.next();
        }
        g
    }

    fn next(&mut self) -> i32 {
        self.r[self.f] = self.r[self.f].wrapping_add(self.r[self.b]);
        let v = self.r[self.f] >> 1;
        self.f = (self.f + 1) % 31;
        self.b = (self.b + 1) % 31;
        v as i32
    }
}

/// macOS libc `rand()` (FreeBSD `do_rand` of that era): the state is the last output.
struct Park(i64);

impl Park {
    fn new(seed: u32) -> Self {
        Park(if seed == 0 { 123459876 } else { seed as i64 })
    }

    fn next(&mut self) -> i32 {
        let hi = self.0 / 127773;
        let lo = self.0 % 127773;
        let mut x = 16807 * lo - 2836 * hi;
        if x < 0 {
            x += 0x7fff_ffff;
        }
        self.0 = x;
        x as i32
    }
}

enum Gen {
    Default(Xorshift64Star),
    Glibc(Glibc),
    Park(Park),
    Script { values: Vec<i32>, pos: usize },
}

/// The sim's single random source. `next_i31` returns what libc `rand()` would: `0..=2^31-1`
/// (Script mode returns the file's values unchanged). Call counter and last value mirror the
/// fork's `g_rand_calls` / `g_rand_last` for the state dump.
pub struct SimRng {
    gen: Gen,
    calls: u64,
    last: i32,
}

impl SimRng {
    /// `seed` is `--prng-seed`. `None`: Default uses the clock; Glibc and Park behave as `srand(1)`.
    pub fn new(mode: PrngMode, seed: Option<u32>) -> Self {
        let gen = match mode {
            PrngMode::Default => Gen::Default(Xorshift64Star::new(match seed {
                Some(s) => s as u64,
                None => clock_seed(),
            })),
            PrngMode::Glibc => Gen::Glibc(Glibc::new(seed.unwrap_or(1))),
            PrngMode::Park => Gen::Park(Park::new(seed.unwrap_or(1))),
        };
        SimRng {
            gen,
            calls: 0,
            last: 0,
        }
    }

    /// Script mode from already-parsed values; see [`parse_script`]. Errors on an empty list.
    pub fn from_script(values: Vec<i32>) -> Result<Self, String> {
        if values.is_empty() {
            return Err("rand script contains no values".into());
        }
        Ok(SimRng {
            gen: Gen::Script { values, pos: 0 },
            calls: 0,
            last: 0,
        })
    }

    pub fn next_i31(&mut self) -> i32 {
        let v = match &mut self.gen {
            Gen::Default(g) => g.next(),
            Gen::Glibc(g) => g.next(),
            Gen::Park(g) => g.next(),
            Gen::Script { values, pos } => {
                let v = values[*pos];
                *pos = (*pos + 1) % values.len();
                v
            }
        };
        self.calls += 1;
        self.last = v;
        v
    }

    /// Number of draws so far.
    pub fn calls(&self) -> u64 {
        self.calls
    }

    /// The most recent draw (0 before the first).
    pub fn last(&self) -> i32 {
        self.last
    }
}

/// One integer per line; blank lines and surrounding whitespace are ignored.
pub fn parse_script(text: &str) -> Result<Vec<i32>, String> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| {
            l.trim()
                .parse::<i32>()
                .map_err(|_| format!("rand script line {}: not an integer: '{}'", i + 1, l.trim()))
        })
        .collect()
}

fn clock_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(s: &str) -> Vec<i32> {
        parse_script(s).unwrap()
    }

    fn draw(mut r: SimRng, n: usize) -> Vec<i32> {
        (0..n).map(|_| r.next_i31()).collect()
    }

    /// Real glibc 2.41 `rand()` output (ticket #16, M1; aarch64 and x86_64 identical).
    #[test]
    fn glibc_matches_real_libc_10000_values_seed1() {
        let want = fixture(include_str!("../tests/fixtures/glibc-seed1-10000.txt"));
        assert_eq!(want.len(), 10_000);
        assert_eq!(draw(SimRng::new(PrngMode::Glibc, Some(1)), 10_000), want);
        // unseeded libc rand() == srand(1)
        assert_eq!(draw(SimRng::new(PrngMode::Glibc, None), 10_000), want);
        assert_eq!(&want[..4], [1804289383, 846930886, 1681692777, 1714636915]);
    }

    #[test]
    fn glibc_matches_real_libc_other_seeds() {
        for (seed, text) in [
            (0u32, include_str!("../tests/fixtures/glibc-seed0-200.txt")),
            (
                2147483649,
                include_str!("../tests/fixtures/glibc-seed2147483649-200.txt"),
            ),
            (
                4294967295,
                include_str!("../tests/fixtures/glibc-seed4294967295-200.txt"),
            ),
            (
                3735928559,
                include_str!("../tests/fixtures/glibc-seed3735928559-200.txt"),
            ),
        ] {
            let want = fixture(text);
            assert_eq!(
                draw(SimRng::new(PrngMode::Glibc, Some(seed)), want.len()),
                want,
                "seed {seed}"
            );
        }
    }

    /// macOS 26.5.1 libc `rand()` archived from ticket #16 (first 1000 values per seed).
    #[test]
    fn park_matches_archived_macos_vectors() {
        for (seed, text) in [
            (0u32, include_str!("../tests/fixtures/macos-seed0-1000.txt")),
            (1, include_str!("../tests/fixtures/macos-seed1-1000.txt")),
            (
                2147483649,
                include_str!("../tests/fixtures/macos-seed2147483649-1000.txt"),
            ),
            (
                4294967295,
                include_str!("../tests/fixtures/macos-seed4294967295-1000.txt"),
            ),
            (
                3735928559,
                include_str!("../tests/fixtures/macos-seed3735928559-1000.txt"),
            ),
        ] {
            let want = fixture(text);
            assert_eq!(want.len(), 1000);
            assert_eq!(
                draw(SimRng::new(PrngMode::Park, Some(seed)), 1000),
                want,
                "seed {seed}"
            );
        }
        assert_eq!(
            draw(SimRng::new(PrngMode::Park, None), 3),
            [16807, 282475249, 1622650073]
        );
    }

    #[test]
    fn default_is_deterministic_per_seed_and_in_range() {
        let a = draw(SimRng::new(PrngMode::Default, Some(42)), 1000);
        assert_eq!(a, draw(SimRng::new(PrngMode::Default, Some(42)), 1000));
        assert_ne!(a, draw(SimRng::new(PrngMode::Default, Some(43)), 1000));
        assert!(a.iter().all(|&v| v >= 0));
        assert!(a.iter().any(|&v| v > 1 << 30), "top bit of the 31 is used");
        // Frozen so the documented algorithm cannot drift unnoticed (D3 record).
        assert_eq!(a[..2], [416839283, 1208242648]);
    }

    #[test]
    fn unseeded_default_varies_between_instances() {
        // 1 - 2^-31 chance per pair; two clock seeds differ unless the clock is frozen.
        let a = SimRng::new(PrngMode::Default, None);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = SimRng::new(PrngMode::Default, None);
        assert_ne!(draw(a, 4), draw(b, 4));
    }

    #[test]
    fn script_cycles_and_counts_calls() {
        let mut r = SimRng::from_script(parse_script("5\n\n 7 \n-3\n").unwrap()).unwrap();
        assert_eq!(r.calls(), 0);
        let got: Vec<i32> = (0..7).map(|_| r.next_i31()).collect();
        assert_eq!(got, [5, 7, -3, 5, 7, -3, 5]);
        assert_eq!((r.calls(), r.last()), (7, 5));
    }

    #[test]
    fn script_errors() {
        assert!(SimRng::from_script(vec![]).is_err());
        assert!(parse_script("1\nabc\n").unwrap_err().contains("line 2"));
        assert!(parse_script("99999999999\n").is_err());
    }

    #[test]
    fn mode_names() {
        assert_eq!("glibc".parse(), Ok(PrngMode::Glibc));
        assert_eq!("park".parse(), Ok(PrngMode::Park));
        assert_eq!("default".parse(), Ok(PrngMode::Default));
        assert!("xorshift".parse::<PrngMode>().is_err());
    }

    /// Pins the draw order the sim relies on: `vec2(rand()%100-50, rand()%100-50)` is x first.
    /// Reference: clang/macOS gives x=33,y=36 for glibc srand(1) (ticket #16, M2).
    #[test]
    fn overlap_jitter_pair_is_x_then_y() {
        let mut r = SimRng::new(PrngMode::Glibc, Some(1));
        let x = r.next_i31() % 100 - 50;
        let y = r.next_i31() % 100 - 50;
        assert_eq!((x, y), (33, 36));
    }
}
