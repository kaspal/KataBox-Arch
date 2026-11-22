//! The one source of randomness for the simulator and the judge: xoshiro256** seeded by
//! splitmix64 (ADR 0006). No OS randomness anywhere: the same seed gives the same run.

use crate::detmath;

/// splitmix64: expands a seed into well-mixed state words.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// FNV-1a over bytes: a stable 64-bit hash for naming random streams (node ids, labels).
pub fn fnv64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// A xoshiro256** generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut st = seed;
        let s = [
            splitmix64(&mut st),
            splitmix64(&mut st),
            splitmix64(&mut st),
            splitmix64(&mut st),
        ];
        Rng { s }
    }

    /// An independent stream for `label` under `seed` (per node, per source…), so editing
    /// one part of a design doesn't reshuffle every other draw.
    pub fn stream(seed: u64, label: &str) -> Self {
        Rng::new(seed ^ fnv64(label.as_bytes()).rotate_left(17))
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform in [0, 1), 53 bits.
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }

    /// Uniform in (0, 1]: safe for logarithms.
    pub fn uniform_open(&mut self) -> f64 {
        1.0 - self.uniform()
    }

    /// Uniform integer in [0, n) (n > 0), without modulo bias worth caring about here
    /// (Lemire's multiply-shift).
    pub fn below(&mut self, n: u64) -> u64 {
        ((u128::from(self.next_u64()) * u128::from(n)) >> 64) as u64
    }

    /// Exponential with the given mean.
    pub fn exponential(&mut self, mean: f64) -> f64 {
        -detmath::ln(self.uniform_open()) * mean
    }

    /// Standard normal (Marsaglia's polar method: no trigonometry needed).
    pub fn normal(&mut self) -> f64 {
        loop {
            let u = 2.0 * self.uniform() - 1.0;
            let v = 2.0 * self.uniform() - 1.0;
            let s = u * u + v * v;
            if s > 0.0 && s < 1.0 {
                return u * (-2.0 * detmath::ln(s) / s).sqrt();
            }
        }
    }

    /// Lognormal with the given median and p99/median ratio (σ = ln(ratio) / 2.3263).
    pub fn lognormal(&mut self, median: f64, p99_ratio: f64) -> f64 {
        let sigma = detmath::ln(p99_ratio.max(1.0)) / 2.326_347_874;
        median * detmath::exp(sigma * self.normal())
    }
}

/// Zipf sampler over ranks 1..=n with exponent s > 0, by rejection-inversion
/// (Hörmann & Derflinger 1996): O(1) per draw, no tables.
#[derive(Debug, Clone, PartialEq)]
pub struct Zipf {
    n: f64,
    s: f64,
    h_x1: f64,
    h_n: f64,
    cut: f64,
}

impl Zipf {
    pub fn new(n: u64, s: f64) -> Self {
        let n = n.max(1) as f64;
        let s = if (s - 1.0).abs() < 1e-9 {
            1.0 + 1e-9
        } else {
            s.max(1e-6)
        };
        let z = Zipf {
            n,
            s,
            h_x1: 0.0,
            h_n: 0.0,
            cut: 0.0,
        };
        let h_x1 = z.h(1.5) - 1.0;
        let h_n = z.h(n + 0.5);
        let cut = 2.0 - z.h_inv(z.h(2.5) - detmath::powf(2.0, -s));
        Zipf {
            h_x1,
            h_n,
            cut,
            ..z
        }
    }

    fn h(&self, x: f64) -> f64 {
        (detmath::powf(x, 1.0 - self.s) - 1.0) / (1.0 - self.s)
    }

    fn h_inv(&self, x: f64) -> f64 {
        detmath::powf(1.0 + x * (1.0 - self.s), 1.0 / (1.0 - self.s))
    }

    /// A rank in 1..=n; rank 1 is the most popular key.
    pub fn sample(&self, rng: &mut Rng) -> u64 {
        loop {
            let u = self.h_n + rng.uniform() * (self.h_x1 - self.h_n);
            let x = self.h_inv(u);
            let k = (x + 0.5).floor().clamp(1.0, self.n);
            if k - x <= self.cut || u >= self.h(k + 0.5) - detmath::powf(k, -self.s) {
                return k as u64;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let a: Vec<u64> = (0..5)
            .map({
                let mut r = Rng::new(42);
                move |_| r.next_u64()
            })
            .collect();
        let b: Vec<u64> = (0..5)
            .map({
                let mut r = Rng::new(42);
                move |_| r.next_u64()
            })
            .collect();
        assert_eq!(a, b);
        assert_ne!(
            Rng::stream(42, "app-1").next_u64(),
            Rng::stream(42, "app-2").next_u64()
        );
    }

    #[test]
    fn known_first_values() {
        // Pinned so a change to the generator is noticed (it would move every golden).
        let mut r = Rng::new(0);
        assert_eq!(r.next_u64(), 0x99ec_5f36_cb75_f2b4);
    }

    #[test]
    fn distributions_have_the_right_means() {
        let mut r = Rng::new(7);
        let n = 200_000;
        let mean_exp = (0..n).map(|_| r.exponential(2.0)).sum::<f64>() / n as f64;
        assert!((mean_exp - 2.0).abs() < 0.03, "{mean_exp}");
        let mean_norm = (0..n).map(|_| r.normal()).sum::<f64>() / n as f64;
        assert!(mean_norm.abs() < 0.01, "{mean_norm}");
        let mut logs: Vec<f64> = (0..n).map(|_| r.lognormal(10.0, 4.0)).collect();
        logs.sort_by(f64::total_cmp);
        let (median, p99) = (logs[n / 2], logs[n * 99 / 100]);
        assert!((median - 10.0).abs() < 0.2, "median {median}");
        assert!(
            (p99 / median - 4.0).abs() < 0.2,
            "p99 ratio {}",
            p99 / median
        );
    }

    #[test]
    fn zipf_favours_low_ranks_and_stays_in_range() {
        let mut r = Rng::new(3);
        let z = Zipf::new(1000, 1.1);
        let mut counts = [0u32; 1001];
        for _ in 0..100_000 {
            let k = z.sample(&mut r);
            assert!((1..=1000).contains(&k));
            counts[k as usize] += 1;
        }
        assert!(counts[1] > counts[2] && counts[2] > counts[10] && counts[10] > counts[500]);
        // P(rank 1) for s = 1.1 over 1000 keys is about 0.17.
        let p1 = f64::from(counts[1]) / 100_000.0;
        assert!((p1 - 0.17).abs() < 0.02, "{p1}");
    }
}
