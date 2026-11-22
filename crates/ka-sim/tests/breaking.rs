//! The breaking point (plan M1.6).

mod common;

use common::*;
use ka_core::registry;
use ka_sim::{FailureReason, Limits, find_limit};

#[test]
fn golden_a_breaks_where_its_app_tier_saturates() {
    // Client → app (Go ×2, 64 threads each) → PostgreSQL at 1,000 req/s. Each request holds
    // an app thread for its own ~5 ms plus the database call: the app tier gives out first,
    // somewhere near 10,000 req/s.
    let (d, s) = golden_a();
    let b = find_limit(&d, &registry(), &s, 1, Limits::default()).unwrap();
    assert_eq!(b.base_rps, 1000.0);
    assert!((5_000.0..15_000.0).contains(&b.limit_rps), "{b:?}");
    assert!(!b.beyond_ceiling);
    let first = b.first_failure.expect("a card gives way");
    assert_eq!(first.node, "app");
    assert!(matches!(
        first.reason,
        FailureReason::Saturated | FailureReason::QueueOverflow
    ));
    assert!(b.runs < 30, "{} windows", b.runs);
}

#[test]
fn the_search_is_deterministic_and_bisects_to_five_percent() {
    let (d, s) = golden_a();
    let a = find_limit(&d, &registry(), &s, 7, Limits::default()).unwrap();
    let b = find_limit(&d, &registry(), &s, 7, Limits::default()).unwrap();
    assert_eq!(a, b);
    // The failing multiple is within 5% above the reported one.
    let fail_x = a.multiplier / (1.0 - ka_sim::breaking::PRECISION);
    assert!(fail_x > a.multiplier);
}

#[test]
fn stricter_limits_break_sooner() {
    let (d, s) = golden_a();
    let loose = find_limit(&d, &registry(), &s, 1, Limits::default()).unwrap();
    let strict = find_limit(
        &d,
        &registry(),
        &s,
        1,
        Limits {
            p95_ms: 25.0,
            error_rate: 0.001,
        },
    )
    .unwrap();
    assert!(
        strict.limit_rps < loose.limit_rps,
        "{strict:?} vs {loose:?}"
    );
}
