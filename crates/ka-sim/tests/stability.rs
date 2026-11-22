//! Gate checks (plan M1.8, `docs/sim-stability.md`), run in release:
//! `cargo test -p ka-sim --release --test stability -- --ignored --nocapture`.

mod common;

use common::*;
use ka_core::registry;
use ka_sim::{SimOptions, simulate};

fn cases() -> Vec<(&'static str, ka_core::Design, ka_core::scenario::Scenario)> {
    let (a, sa) = golden_a();
    let (b, sb) = golden_b(12_000.0);
    let (c, sc) = golden_c();
    vec![("a", a, sa), ("b (12k)", b, sb), ("c", c, sc)]
}

#[test]
#[ignore = "release gate check"]
fn a_hundred_runs_of_one_seed_are_byte_identical() {
    for (name, d, s) in cases() {
        let first = serde_json::to_string(
            &simulate(&d, &registry(), &s, 42, &SimOptions::default()).unwrap(),
        )
        .unwrap();
        for i in 1..100 {
            let again = serde_json::to_string(
                &simulate(&d, &registry(), &s, 42, &SimOptions::default()).unwrap(),
            )
            .unwrap();
            assert_eq!(first, again, "{name}: run {i} differs");
        }
        println!("{name}: 100 identical runs ({} bytes)", first.len());
    }
}

#[test]
#[ignore = "release gate check"]
fn p95_and_utilisation_spread_across_a_hundred_seeds() {
    println!(
        "| Golden | p95 mean (ms) | p95 min–max | p95 CV | busiest util mean | util min–max |"
    );
    println!("|---|---|---|---|---|---|");
    for (name, d, s) in cases() {
        let mut p95 = Vec::new();
        let mut util = Vec::new();
        for seed in 1..=100 {
            let r = simulate(&d, &registry(), &s, seed, &SimOptions::default()).unwrap();
            p95.push(r.latency.p95_ms);
            util.push(r.nodes.iter().map(|n| n.utilisation).fold(0.0, f64::max));
        }
        let stats = |v: &[f64]| {
            let n = v.len() as f64;
            let mean = v.iter().sum::<f64>() / n;
            let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / n;
            let min = v.iter().copied().fold(f64::INFINITY, f64::min);
            let max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (mean, min, max, var.sqrt() / mean)
        };
        let (pm, pmin, pmax, pcv) = stats(&p95);
        let (um, umin, umax, _) = stats(&util);
        println!(
            "| {name} | {pm:.2} | {pmin:.2}–{pmax:.2} | {:.1}% | {um:.3} | {umin:.3}–{umax:.3} |",
            pcv * 100.0
        );
        // A verdict shouldn't hinge on the seed: the spread stays narrow.
        assert!(
            pcv < 0.1,
            "{name}: p95 varies {:.1}% across seeds",
            pcv * 100.0
        );
    }
}
