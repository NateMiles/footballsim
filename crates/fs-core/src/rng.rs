//! Deterministic random numbers.
//!
//! The generator is **xoshiro256\*\***, seeded through **SplitMix64**. Both
//! use only integer arithmetic, so they give identical streams on every
//! platform.
//!
//! Parallel work never shares a generator. Each unit of work derives its own
//! [`Rng`] from the world [`Seed`], a [`Stream`] naming the subsystem, and up
//! to two keys such as the day number and an entity ID. Results therefore do
//! not depend on thread scheduling or on the order work is processed.
//!
//! **Changing anything in this file changes every simulated outcome and
//! invalidates replays.** The golden-value tests below exist to catch that.

use crate::math;
use serde::{Deserialize, Serialize};

/// The root seed of a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seed(pub u64);

/// Names the subsystem a random stream belongs to, so that two subsystems
/// using the same keys still get independent streams.
///
/// The numeric values are part of the save format. Never renumber them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u64)]
pub enum Stream {
    /// Procedural world generation.
    Generation = 1,
    /// Per-entity daily updates (training, recovery, morale).
    DailyTick = 2,
    /// A single match.
    Match = 3,
    /// AI decision making.
    Ai = 4,
    /// Transfer market.
    Transfers = 5,
    /// Youth intake.
    Youth = 6,
    /// Injuries.
    Injury = 7,
    /// Monthly player development and ageing.
    Development = 8,
    /// Season rollover: retirements, contract decisions.
    Season = 9,
    /// Scouting estimates: what a club believes about a player.
    Scouting = 10,
    /// Club finances: attendances and other variable income.
    Finance = 11,
    /// Staff: generation, development and hiring.
    Staff = 12,
    /// Boards and the managers' job market.
    Board = 13,
}

impl Seed {
    /// Derives an independent generator for `stream` keyed by `a` and `b`.
    ///
    /// ```
    /// use fs_core::{Seed, Stream};
    /// let seed = Seed(7);
    /// let mut x = seed.derive(Stream::DailyTick, 10, 3);
    /// let mut y = seed.derive(Stream::DailyTick, 10, 3);
    /// assert_eq!(x.next_u64(), y.next_u64());
    /// ```
    pub fn derive(self, stream: Stream, a: u64, b: u64) -> Rng {
        let mut h = mix(self.0 ^ mix(stream as u64));
        h = mix(h ^ a.wrapping_mul(0xA076_1D64_78BD_642F));
        h = mix(h ^ b.wrapping_mul(0xE703_7ED1_A0B4_28DB));
        Rng::from_u64(h)
    }
}

/// SplitMix64 finaliser: a fast, high-quality 64-bit mixing function.
#[inline]
pub const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A xoshiro256\*\* generator. Small (32 bytes), fast, serialisable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    /// Seeds the generator by expanding `seed` with SplitMix64, as
    /// recommended by the xoshiro authors.
    pub fn from_u64(seed: u64) -> Self {
        let mut sm = seed;
        let mut next = || {
            sm = sm.wrapping_add(0x9E37_79B9_7F4A_7C15);
            mix(sm)
        };
        let s = [next(), next(), next(), next()];
        // All-zero state is the one invalid state. SplitMix64 cannot produce
        // four zero outputs in a row, but guard anyway.
        debug_assert!(s != [0; 4]);
        Self { s }
    }

    /// Next 64 random bits.
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.s;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// Next 32 random bits (the high half, which has the best quality).
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform integer in `0..n`, without modulo bias (Lemire's method).
    ///
    /// # Panics
    /// Panics if `n` is zero.
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "Rng::below(0)");
        let mut m = u64::from(self.next_u32()) * u64::from(n);
        let mut low = m as u32;
        if low < n {
            let threshold = n.wrapping_neg() % n;
            while low < threshold {
                m = u64::from(self.next_u32()) * u64::from(n);
                low = m as u32;
            }
        }
        (m >> 32) as u32
    }

    /// Uniform index into a slice of length `len`.
    ///
    /// # Panics
    /// Panics if `len` is zero or does not fit in a `u32`.
    #[inline]
    pub fn index(&mut self, len: usize) -> usize {
        self.below(u32::try_from(len).expect("Rng::index length overflow")) as usize
    }

    /// Uniform integer in `lo..=hi`.
    ///
    /// # Panics
    /// Panics if `lo > hi`.
    #[inline]
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi, "Rng::range_i32 with lo > hi");
        let span = (i64::from(hi) - i64::from(lo) + 1) as u64;
        if span > u64::from(u32::MAX) {
            return self.next_u32() as i32;
        }
        (i64::from(lo) + i64::from(self.below(span as u32))) as i32
    }

    /// Uniform float in `[0, 1)` with 53 bits of precision.
    #[inline]
    pub fn unit_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform float in `[0, 1)` with 24 bits of precision.
    #[inline]
    pub fn unit_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / (1u32 << 24) as f32)
    }

    /// Uniform float in `[lo, hi)`.
    #[inline]
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit_f64()
    }

    /// `true` with probability `p`. Values outside `[0, 1]` saturate.
    #[inline]
    pub fn chance(&mut self, p: f64) -> bool {
        self.unit_f64() < p
    }

    /// Normally distributed value (Marsaglia polar method).
    ///
    /// Uses only `sqrt` and `ln` from [`crate::math`], so it is bit-exact
    /// across platforms.
    pub fn normal(&mut self, mean: f64, std_dev: f64) -> f64 {
        loop {
            let u = 2.0 * self.unit_f64() - 1.0;
            let v = 2.0 * self.unit_f64() - 1.0;
            let s = u * u + v * v;
            if s > 0.0 && s < 1.0 {
                let factor = math::sqrt(-2.0 * math::ln(s) / s);
                return mean + std_dev * u * factor;
            }
        }
    }

    /// Picks an index with probability proportional to `weights[i]`.
    /// Returns `None` if the slice is empty or every weight is zero or
    /// negative.
    pub fn weighted_index(&mut self, weights: &[f64]) -> Option<usize> {
        let total: f64 = weights.iter().copied().filter(|w| *w > 0.0).sum();
        if total <= 0.0 {
            return None;
        }
        let mut target = self.unit_f64() * total;
        let mut last_positive = None;
        for (i, &w) in weights.iter().enumerate() {
            if w > 0.0 {
                if target < w {
                    return Some(i);
                }
                target -= w;
                last_positive = Some(i);
            }
        }
        // Rounding can leave a sliver past the last bucket.
        last_positive
    }

    /// Shuffles a slice in place (Fisher-Yates).
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.index(i + 1);
            items.swap(i, j);
        }
    }

    /// A uniformly chosen element, or `None` if the slice is empty.
    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.index(items.len())])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden values computed with an independent reference implementation
    /// of SplitMix64 seeding + xoshiro256**. If this test fails, every
    /// simulated outcome in the game has changed.
    #[test]
    fn golden_stream() {
        let mut rng = Rng::from_u64(0);
        let got: Vec<u64> = (0..4).map(|_| rng.next_u64()).collect();
        assert_eq!(got, GOLDEN_SEED_0);
    }

    #[test]
    fn golden_derive() {
        let mut rng = Seed(12345).derive(Stream::Match, 7, 99);
        assert_eq!(rng.next_u64(), GOLDEN_DERIVE);
    }

    const GOLDEN_SEED_0: [u64; 4] = [
        11_091_344_671_253_066_420,
        13_793_997_310_169_335_082,
        1_900_383_378_846_508_768,
        7_684_712_102_626_143_532,
    ];
    const GOLDEN_DERIVE: u64 = 10_146_113_943_904_712_730;

    #[test]
    fn streams_are_independent() {
        let seed = Seed(1);
        let a = seed.derive(Stream::DailyTick, 1, 1).next_u64();
        let b = seed.derive(Stream::DailyTick, 1, 2).next_u64();
        let c = seed.derive(Stream::Match, 1, 1).next_u64();
        let d = seed.derive(Stream::DailyTick, 2, 1).next_u64();
        assert!(a != b && a != c && a != d && b != c && b != d && c != d);
    }

    #[test]
    fn below_is_in_range_and_roughly_uniform() {
        let mut rng = Rng::from_u64(9);
        let mut counts = [0u32; 6];
        for _ in 0..60_000 {
            counts[rng.below(6) as usize] += 1;
        }
        for c in counts {
            assert!((9_000..11_000).contains(&c), "{counts:?}");
        }
    }

    #[test]
    fn range_i32_bounds() {
        let mut rng = Rng::from_u64(3);
        for _ in 0..10_000 {
            let x = rng.range_i32(-3, 4);
            assert!((-3..=4).contains(&x));
        }
        assert_eq!(rng.range_i32(5, 5), 5);
        // Full range does not panic.
        let _ = rng.range_i32(i32::MIN, i32::MAX);
    }

    #[test]
    fn unit_floats_in_range() {
        let mut rng = Rng::from_u64(4);
        for _ in 0..10_000 {
            let a = rng.unit_f64();
            let b = rng.unit_f32();
            assert!((0.0..1.0).contains(&a));
            assert!((0.0..1.0).contains(&b));
        }
    }

    #[test]
    fn normal_has_expected_moments() {
        let mut rng = Rng::from_u64(5);
        let n = 100_000;
        let xs: Vec<f64> = (0..n).map(|_| rng.normal(10.0, 2.0)).collect();
        let mean = xs.iter().sum::<f64>() / n as f64;
        let var = xs.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / n as f64;
        assert!((mean - 10.0).abs() < 0.05, "mean {mean}");
        assert!((var - 4.0).abs() < 0.1, "var {var}");
    }

    #[test]
    fn weighted_index_respects_weights() {
        let mut rng = Rng::from_u64(6);
        let w = [0.0, 1.0, 3.0, -2.0];
        let mut counts = [0u32; 4];
        for _ in 0..40_000 {
            counts[rng.weighted_index(&w).unwrap()] += 1;
        }
        assert_eq!(counts[0], 0);
        assert_eq!(counts[3], 0);
        assert!((9_000..11_000).contains(&counts[1]), "{counts:?}");
        assert_eq!(rng.weighted_index(&[0.0, -1.0]), None);
        assert_eq!(rng.weighted_index(&[]), None);
    }

    #[test]
    fn shuffle_is_a_permutation() {
        let mut rng = Rng::from_u64(8);
        let mut v: Vec<u32> = (0..100).collect();
        rng.shuffle(&mut v);
        let mut sorted = v.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..100).collect::<Vec<_>>());
        assert_ne!(v, sorted);
    }

    #[test]
    fn serde_round_trip_preserves_stream() {
        let mut rng = Rng::from_u64(11);
        rng.next_u64();
        let bytes = postcard::to_allocvec(&rng).unwrap();
        let mut back: Rng = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(rng.next_u64(), back.next_u64());
    }
}
