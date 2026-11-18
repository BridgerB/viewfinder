//! The two places where results must match the JVM reference bit for bit: Java's rounding,
//! which differs from Rust's on negative halves, and `java.util.Random`, which picks the fixed
//! samples the golden test and the profiler use.

/// `Math.round(double)` in Java: round half toward positive infinity. Rust's `f64::round`
/// rounds half away from zero, so `-2.5` would differ.
#[must_use]
pub fn round(x: f64) -> i64 {
    if x.is_nan() {
        return 0;
    }
    let floor = x.floor();
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    rounded as i64
}

/// `java.util.Random`: the 48-bit linear congruential generator.
#[derive(Clone, Debug)]
pub struct JavaRandom {
    seed: u64,
}

const MULTIPLIER: u64 = 0x0005_DEEC_E66D;
const ADDEND: u64 = 0xB;
const MASK: u64 = (1 << 48) - 1;

impl JavaRandom {
    #[must_use]
    pub fn new(seed: i64) -> Self {
        Self { seed: (seed as u64 ^ MULTIPLIER) & MASK }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// `nextInt(bound)`, bound > 0. Returns 0 for a non-positive bound instead of throwing.
    pub fn next_int(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            return 0;
        }
        if bound & (bound - 1) == 0 {
            return ((i64::from(bound) * i64::from(self.next(31))) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let val = bits % bound;
            if bits.wrapping_sub(val).wrapping_add(bound - 1) >= 0 {
                return val;
            }
        }
    }

    /// The first `count` distinct values of `nextInt(bound)`, sorted: how the reference picks a
    /// fixed sample of places. Stops early if the bound has fewer distinct values.
    pub fn distinct_sorted(&mut self, bound: i32, count: usize) -> Vec<usize> {
        let want = count.min(usize::try_from(bound.max(0)).unwrap_or(0));
        let mut seen = std::collections::BTreeSet::new();
        let mut order = Vec::with_capacity(want);
        while order.len() < want {
            let v = self.next_int(bound);
            if seen.insert(v) {
                order.push(v as usize);
            }
        }
        order.sort_unstable();
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_matches_java() {
        assert_eq!(round(2.5), 3);
        assert_eq!(round(-2.5), -2);
        assert_eq!(round(-2.6), -3);
        assert_eq!(round(0.499_999_999_999_999_94), 0);
        assert_eq!(round(-0.5), 0);
        assert_eq!(round(f64::NAN), 0);
    }

    #[test]
    fn random_matches_java() {
        // Values printed by jshell (JDK 21): new Random(42).nextInt(100) five times.
        let mut r = JavaRandom::new(42);
        let got: Vec<i32> = (0..5).map(|_| r.next_int(100)).collect();
        assert_eq!(got, vec![30, 63, 48, 84, 70]);
        // new Random(7).nextInt(16) three times: the power-of-two path.
        let mut r = JavaRandom::new(7);
        let got: Vec<i32> = (0..3).map(|_| r.next_int(16)).collect();
        assert_eq!(got, vec![11, 10, 11]);
    }
}
