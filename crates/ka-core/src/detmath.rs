//! Deterministic math for the simulator and the judge (ADR 0006).
//!
//! `f64::ln`, `exp` and friends call the platform's libm, which may differ in the last
//! bit between MSVC and glibc. One bit is enough to reorder two events and change a
//! golden. These versions use only `+ − × ÷`, comparisons and bit manipulation, which
//! IEEE 754 defines exactly, so the same inputs give the same bits on every platform.
//! They are accurate to about 1e-15 relative error (tested against std).

const LN2: f64 = std::f64::consts::LN_2;
/// ln 2 split in two (Cody–Waite, as in fdlibm): `k · LN2_HI` is exact for the k used here.
const LN2_HI: f64 = f64::from_bits(0x3fe6_2e42_fee0_0000);
const LN2_LO: f64 = f64::from_bits(0x3dea_39ef_3579_3c76);

/// The natural logarithm. `ln(0) = −∞`; negative or NaN inputs give NaN.
pub fn ln(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    if x.is_infinite() {
        return f64::INFINITY;
    }
    // x = m · 2^e with m in [1, 2) (subnormals scaled up first).
    let (mut bits, mut e) = (x.to_bits(), 0i64);
    if (bits >> 52) & 0x7ff == 0 {
        let scaled = x * f64::from_bits(0x4350_0000_0000_0000); // 2^54
        bits = scaled.to_bits();
        e -= 54;
    }
    e += ((bits >> 52) & 0x7ff) as i64 - 1023;
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000);
    // Centre m on 1: m in [√½, √2).
    if m > std::f64::consts::SQRT_2 {
        m *= 0.5;
        e += 1;
    }
    // ln(m) = 2·atanh(s) with s = (m − 1)/(m + 1), |s| < 0.1716: the series converges fast.
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    let mut term = s;
    let mut sum = 0.0;
    let mut k = 1.0;
    while k < 40.0 {
        sum += term / k;
        term *= s2;
        k += 2.0;
    }
    2.0 * sum + e as f64 * LN2
}

/// e^x. Overflows to +∞ above ~709.78 and underflows to 0 below ~−745.
pub fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x > 709.78 {
        return f64::INFINITY;
    }
    if x < -745.2 {
        return 0.0;
    }
    // x = k·ln2 + r, |r| ≤ ln2/2; e^x = 2^k · e^r.
    let k = round_half_away(x / LN2);
    let r = (x - k * LN2_HI) - k * LN2_LO;
    // Taylor series for e^r.
    let mut term = 1.0;
    let mut sum = 1.0;
    let mut n = 1.0;
    while n < 25.0 {
        term *= r / n;
        sum += term;
        n += 1.0;
    }
    scale_by_pow2(sum, k as i64)
}

/// x^y for x > 0 (all the simulator needs).
pub fn powf(x: f64, y: f64) -> f64 {
    if x == 1.0 || y == 0.0 {
        return 1.0;
    }
    exp(y * ln(x))
}

fn round_half_away(v: f64) -> f64 {
    let t = v.trunc();
    if (v - t).abs() >= 0.5 {
        t + v.signum()
    } else {
        t
    }
}

/// v · 2^k, exactly (in two steps so intermediate values stay normal).
fn scale_by_pow2(v: f64, k: i64) -> f64 {
    let pow2 = |e: i64| f64::from_bits(((e + 1023) as u64) << 52);
    if k > 1000 {
        v * pow2(1000) * pow2(k - 1000)
    } else if k < -1000 {
        v * pow2(-1000) * pow2(k + 1000)
    } else {
        v * pow2(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-14 * b.abs().max(1e-300)
    }

    #[test]
    fn ln_matches_std() {
        for &x in &[
            1e-300,
            1e-12,
            0.001,
            0.1,
            0.5,
            std::f64::consts::FRAC_1_SQRT_2,
            1.0,
            1.0001,
            2.0,
            std::f64::consts::E,
            10.0,
            12345.678,
            1e300,
        ] {
            assert!(close(ln(x), x.ln()), "ln({x}) = {} vs {}", ln(x), x.ln());
        }
        assert!(ln(-1.0).is_nan());
        assert_eq!(ln(0.0), f64::NEG_INFINITY);
        assert_eq!(ln(1.0), 0.0);
    }

    #[test]
    fn exp_matches_std() {
        for &x in &[
            -700.0,
            -50.0,
            -1.0,
            -1e-9,
            0.0,
            1e-9,
            0.5,
            1.0,
            std::f64::consts::LN_10,
            50.0,
            700.0,
        ] {
            assert!(
                close(exp(x), x.exp()),
                "exp({x}) = {} vs {}",
                exp(x),
                x.exp()
            );
        }
        assert_eq!(exp(1000.0), f64::INFINITY);
        assert_eq!(exp(-1000.0), 0.0);
    }

    #[test]
    fn powf_matches_std() {
        for &(x, y) in &[
            (2.0, 10.0),
            (10.0, -1.1),
            (1e6, 1.1),
            (0.3, 2.5),
            (7.0, 0.5),
        ] {
            assert!(
                (powf(x, y) - x.powf(y)).abs() <= 1e-13 * x.powf(y).abs(),
                "{x}^{y}"
            );
        }
    }
}
