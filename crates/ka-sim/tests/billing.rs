//! Billing (ADR 0006, M7b): reads and writes priced apart, per-lane prices, billing units,
//! DNS `query_ratio`, and the live cost equal to the judged cost once a run ends.

mod common;

use std::sync::Arc;

use common::*;
use ka_core::{ComponentKind as K, Flavour, Protocol, registry};
use ka_sim::{RunResult, SimOptions, compile, simulate};

const MONTH_S: f64 = 2_629_746.0;

fn run(d: &ka_core::Design, s: &ka_core::scenario::Scenario) -> RunResult {
    simulate(d, &registry(), s, 42, &SimOptions::default()).expect("the design runs")
}

fn cost(r: &RunResult, id: &str) -> f64 {
    r.nodes
        .iter()
        .find(|n| n.id == id)
        .expect("node")
        .cost_month
}

fn per_month(per_million: f64, count: f64, secs: f64) -> f64 {
    per_million * count / secs * MONTH_S / 1e6
}

/// A finished live run shows what the judge bills: the drain after the test isn't traffic.
#[test]
fn the_live_cost_at_the_end_equals_the_judged_cost() {
    for (d, s) in [golden_a(), golden_b(6_000.0), golden_c(), golden_d()] {
        let c = Arc::new(compile(&d, &registry(), &s, &SimOptions::default()).unwrap());
        let mut e = ka_sim::Engine::new(c, 5);
        let mut t = 0;
        while !e.finished() {
            t += 33_333;
            e.advance_to(t);
        }
        assert!(e.now_us() > e.duration_us(), "the run drained");
        let live = e.snapshot().cost_month;
        let judged = e.result().cost_month;
        assert!(
            (live - judged).abs() <= 0.02,
            "live ${live} vs judged ${judged}"
        );
    }
}

#[test]
fn dynamodb_bills_write_units_and_kafka_bills_brokers() {
    // golden c: every event is a write to DynamoDB through Kafka.
    let (d, s) = golden_c();
    let r = run(&d, &s);
    let store = r.nodes.iter().find(|n| n.id == "store").unwrap();
    let want = per_month(0.625, store.served, s.duration_s);
    assert!(
        (store.cost_month - want).abs() < 0.02,
        "{} vs {want}",
        store.cost_month
    );
    // MSK: one broker, partitions free, no per-request fee.
    assert_eq!(cost(&r, "stream"), 153.0);
}

#[test]
fn dns_bills_only_the_query_ratio() {
    let mk = |ratio: Option<f64>| {
        let mut dns = node("dns", K::Dns, Flavour::Route53, &[]);
        if let Some(q) = ratio {
            dns.props
                .insert("query_ratio".into(), ka_core::PropValue::Number(q));
        }
        design(
            vec![
                node("client", K::Client, Flavour::Web, &[]),
                dns,
                node("app", K::App, Flavour::Go, &[]),
            ],
            vec![
                edge("client", "dns", Protocol::Http),
                edge("dns", "app", Protocol::Http),
            ],
        )
    };
    let s = scenario(500.0, 20.0, vec![rtype("page", 1.0, &["edge", "compute"])]);
    let r = run(&mk(None), &s);
    let served = r.nodes.iter().find(|n| n.id == "dns").unwrap().served;
    let want = per_month(0.4, served, s.duration_s) * 0.01;
    assert!(
        (cost(&r, "dns") - want).abs() < 0.02,
        "{} vs {want}",
        cost(&r, "dns")
    );
    let all = run(&mk(Some(1.0)), &s);
    assert!((cost(&all, "dns") - want * 100.0).abs() < 1.0);
}

#[test]
fn per_lane_prices_follow_partitions() {
    let mk = |flavour: Flavour, partitions: f64| {
        design(
            vec![
                node("client", K::Client, Flavour::Web, &[]),
                node("app", K::App, Flavour::Go, &[]),
                node("db", K::Nosql, flavour, &[("partitions", partitions)]),
            ],
            vec![
                edge("client", "app", Protocol::Http),
                edge("app", "db", Protocol::Query),
            ],
        )
    };
    let s = scenario(
        200.0,
        10.0,
        vec![rtype("read", 1.0, &["compute", "datastore"])],
    );
    assert_eq!(cost(&run(&mk(Flavour::Cassandra, 4.0), &s), "db"), 1000.0);
    assert_eq!(cost(&run(&mk(Flavour::Cassandra, 16.0), &s), "db"), 4000.0);
    assert_eq!(cost(&run(&mk(Flavour::Mongodb, 2.0), &s), "db"), 788.0);
    // DynamoDB on demand: no partition fee, only read units.
    let r = run(&mk(Flavour::Dynamodb, 16.0), &s);
    let served = r.nodes.iter().find(|n| n.id == "db").unwrap().served;
    let want = per_month(0.125, served, s.duration_s);
    assert!((cost(&r, "db") - want).abs() < 0.02);
}
