//! S8 metastable check: `recovered_by_s` (M8).

use ka_core::scenario::PassCriteria;
use ka_sim::RunResult;

fn run_with(seconds: &[(f64, f64, f64)]) -> RunResult {
    let series: Vec<serde_json::Value> = seconds
        .iter()
        .enumerate()
        .map(|(t, (ok, errors, p95))| {
            serde_json::json!({ "t": t, "requests": ok + errors, "ok": ok, "errors": errors, "p95_ms": p95 })
        })
        .collect();
    serde_json::from_value(serde_json::json!({
        "seed": 1, "duration_s": seconds.len() as f64, "sample_factor": 1.0,
        "requests": 0.0, "ok": 0.0, "errors": 0.0, "shed": 0.0, "error_rate": 0.0,
        "availability": 1.0, "throughput_rps": 0.0,
        "latency": { "p50_ms": 0.0, "p95_ms": 0.0, "p99_ms": 0.0 },
        "response": { "p50_ms": 0.0, "p95_ms": 0.0, "p99_ms": 0.0 },
        "completion": null, "lost": 0.0, "dead_lettered": 0.0, "retried": 0.0,
        "stale_reads": 0.0, "stale_rate": 0.0, "cost_month": 0.0,
        "per_type": [], "nodes": [], "series": series
    }))
    .expect("a run result")
}

#[test]
fn a_system_that_recovers_by_the_deadline_passes() {
    // Bad from 2 s to 4 s, fine after.
    let r = run_with(&[
        (100.0, 0.0, 20.0),
        (100.0, 0.0, 20.0),
        (10.0, 90.0, 900.0),
        (10.0, 90.0, 900.0),
        (100.0, 0.0, 20.0),
        (100.0, 0.0, 20.0),
    ]);
    let limits = |by: f64| PassCriteria {
        p95_ms: Some(100.0),
        error_rate: Some(0.01),
        recovered_by_s: Some(by),
        ..PassCriteria::default()
    };
    assert!(ka_judge::recovered(&r, &limits(4.0)));
    assert!(
        !ka_judge::recovered(&r, &limits(3.0)),
        "still failing at 3 s"
    );
    assert!(
        ka_judge::recovered(&r, &PassCriteria::default()),
        "unset: not checked"
    );
}

#[test]
fn a_metastable_system_never_recovers() {
    let r = run_with(&[
        (100.0, 0.0, 20.0),
        (10.0, 90.0, 900.0),
        (10.0, 90.0, 900.0),
        (10.0, 90.0, 900.0),
    ]);
    let limits = PassCriteria {
        recovered_by_s: Some(2.0),
        ..PassCriteria::default()
    };
    assert!(!ka_judge::recovered(&r, &limits), "1% default error limit");
}
