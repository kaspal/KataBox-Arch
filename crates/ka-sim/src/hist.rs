//! Latency histograms with fixed log buckets (1 µs to ~60 s, about 2% wide), so
//! percentiles are deterministic and cheap to merge.

use ka_core::detmath;

const GROWTH: f64 = 1.02;
pub const BUCKETS: usize = 910; // 1.02^910 µs ≈ 66 s

fn ln_growth() -> f64 {
    detmath::ln(GROWTH)
}

/// The bucket of a latency in µs.
pub fn bucket_of(us: f64) -> usize {
    if us <= 1.0 {
        return 0;
    }
    ((detmath::ln(us) / ln_growth()) as usize).min(BUCKETS - 1)
}

/// A representative value of a bucket, in µs (its geometric middle).
pub fn bucket_value(i: usize) -> f64 {
    detmath::exp((i as f64 + 0.5) * ln_growth())
}

/// Weighted counts per bucket. Empty histograms allocate nothing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Hist {
    counts: Vec<f64>,
    total: f64,
}

impl Hist {
    pub fn add(&mut self, us: f64, weight: f64) {
        if self.counts.is_empty() {
            self.counts = vec![0.0; BUCKETS];
        }
        self.counts[bucket_of(us)] += weight;
        self.total += weight;
    }

    pub fn total(&self) -> f64 {
        self.total
    }

    pub fn merge(&mut self, other: &Hist) {
        if other.counts.is_empty() {
            return;
        }
        if self.counts.is_empty() {
            self.counts = vec![0.0; BUCKETS];
        }
        for (a, b) in self.counts.iter_mut().zip(&other.counts) {
            *a += b;
        }
        self.total += other.total;
    }

    /// The q-quantile (0..=1) in milliseconds; 0 when empty.
    pub fn quantile_ms(&self, q: f64) -> f64 {
        if self.total <= 0.0 {
            return 0.0;
        }
        let want = q * self.total;
        let mut acc = 0.0;
        for (i, c) in self.counts.iter().enumerate() {
            acc += c;
            if acc >= want && *c > 0.0 {
                return bucket_value(i) / 1000.0;
            }
        }
        bucket_value(BUCKETS - 1) / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_are_about_two_percent_wide() {
        for &us in &[10.0, 1_000.0, 80_000.0, 5_000_000.0] {
            let v = bucket_value(bucket_of(us));
            assert!((v / us - 1.0).abs() < 0.02, "{us} -> {v}");
        }
    }

    #[test]
    fn quantiles() {
        let mut h = Hist::default();
        for i in 1..=100 {
            h.add(f64::from(i) * 1000.0, 1.0); // 1..100 ms
        }
        assert!((h.quantile_ms(0.5) - 50.0).abs() < 1.5);
        assert!((h.quantile_ms(0.95) - 95.0).abs() < 2.5);
        assert_eq!(Hist::default().quantile_ms(0.95), 0.0);
    }
}
