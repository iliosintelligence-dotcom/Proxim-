//! Seeded deterministic integer RNG (xorshift64* seeded through SplitMix64) and integer
//! approximately-Gaussian draws. No float, no clock, no OS entropy: a run is a pure function of
//! its seed, so every report is byte-identical on rerun and across operating systems.

/// Fixed-point scale of [`Rng::gauss_q16`]: one standard deviation is `GAUSS_ONE`.
pub const GAUSS_ONE: i64 = 1 << 16;

#[derive(Clone, Debug)]
pub struct Rng(u64);

/// SplitMix64 step: decorrelates small consecutive seeds (0, 1, 2, ...).
#[must_use]
pub const fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Rng {
    /// A generator for `seed`; `stream` separates independent uses of one seed (noise, jumps).
    #[must_use]
    pub const fn new(seed: u64, stream: u64) -> Self {
        let s = splitmix64(seed ^ splitmix64(stream));
        Self(if s == 0 { 0x2545_F491_4F6C_DD1D } else { s })
    }

    pub const fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `0..n`; `n == 0` gives 0. Rejection sampling, so there is no modulo bias.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let zone = u64::MAX - (u64::MAX % n);
        loop {
            let r = self.next_u64();
            if r < zone {
                return r % n;
            }
        }
    }

    /// Uniform in `lo..=hi` (returns `lo` if `hi < lo`).
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = hi.abs_diff(lo).saturating_add(1);
        let off = self.below(span);
        lo.saturating_add_unsigned(off)
    }

    /// True with probability `ppm / 1_000_000`.
    pub fn chance_ppm(&mut self, ppm: u32) -> bool {
        self.below(1_000_000) < u64::from(ppm)
    }

    /// Irwin-Hall approximation of a standard normal in Q16: the sum of 12 uniforms on
    /// `[0, 2^16)` minus `6 x 2^16`. Mean 0, standard deviation `2^16`, bounded by 6 sigma.
    pub fn gauss_q16(&mut self) -> i64 {
        let mut sum: i64 = 0;
        for _ in 0..3 {
            let r = self.next_u64();
            // four 16-bit uniforms per draw
            sum = sum
                .saturating_add(i64::from((r & 0xFFFF) as u16))
                .saturating_add(i64::from(((r >> 16) & 0xFFFF) as u16))
                .saturating_add(i64::from(((r >> 32) & 0xFFFF) as u16))
                .saturating_add(i64::from(((r >> 48) & 0xFFFF) as u16));
        }
        sum.saturating_sub(6 * GAUSS_ONE)
    }

    /// A normal draw with standard deviation `sigma` (any integer unit), truncated at 6 sigma.
    pub fn gauss(&mut self, sigma: i64) -> i64 {
        let g = i128::from(self.gauss_q16());
        let v = (g * i128::from(sigma)) >> 16;
        i64::try_from(v).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_seed_sensitive() {
        let mut a = Rng::new(7, 1);
        let mut b = Rng::new(7, 1);
        let mut c = Rng::new(8, 1);
        let mut d = Rng::new(7, 2);
        let xa: alloc::vec::Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let xb: alloc::vec::Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        let xc: alloc::vec::Vec<u64> = (0..8).map(|_| c.next_u64()).collect();
        let xd: alloc::vec::Vec<u64> = (0..8).map(|_| d.next_u64()).collect();
        assert_eq!(xa, xb);
        assert_ne!(xa, xc);
        assert_ne!(xa, xd);
    }

    #[test]
    fn gauss_moments_are_close_to_standard() {
        let mut r = Rng::new(42, 0);
        let n = 200_000_i64;
        let mut sum = 0_i128;
        let mut sq = 0_i128;
        for _ in 0..n {
            let g = i128::from(r.gauss(1000));
            sum += g;
            sq += g * g;
        }
        let mean = sum / i128::from(n);
        let var = sq / i128::from(n) - mean * mean;
        assert!(mean.abs() < 10, "mean {mean}");
        // sigma 1000 -> variance 1e6, within 2%
        assert!((980_000..=1_020_000).contains(&var), "var {var}");
    }

    #[test]
    fn below_and_range_stay_in_bounds() {
        let mut r = Rng::new(3, 3);
        for _ in 0..10_000 {
            assert!(r.below(7) < 7);
            let x = r.range_i64(-5, 5);
            assert!((-5..=5).contains(&x));
        }
        assert_eq!(r.below(0), 0);
        assert_eq!(r.range_i64(4, 2), 4);
    }
}
