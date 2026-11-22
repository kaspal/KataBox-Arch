//! `cargo run -p ka-sim --release --example bench`: the engine bench (ADR 0006).
#![allow(clippy::disallowed_types)]

#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    let (d, mut s) = common::golden_b(50_000.0);
    s.duration_s = 60.0;
    let t = std::time::Instant::now();
    let r = ka_sim::simulate(
        &d,
        &ka_core::registry(),
        &s,
        1,
        &ka_sim::SimOptions::default(),
    )
    .expect("runs");
    println!(
        "60 s at 50k rps: {:?} (sample factor {:.3}, p95 {:.1} ms, error rate {:.4})",
        t.elapsed(),
        r.sample_factor,
        r.latency.p95_ms,
        r.error_rate
    );
}
