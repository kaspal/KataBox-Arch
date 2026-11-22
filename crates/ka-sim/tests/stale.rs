//! Stale reads (plan M1.7, ADR 0006): replicas behind their primary, and cache-aside copies
//! older than the last write.

mod common;

use common::*;
use ka_core::scenario::KeySkew;
use ka_core::{ComponentKind as K, Flavour, Protocol, registry};
use ka_sim::{RunResult, SimOptions, simulate};

fn run(d: &ka_core::Design, s: &ka_core::scenario::Scenario) -> RunResult {
    simulate(d, &registry(), s, 1, &SimOptions::default()).expect("the design runs")
}

/// Reads and writes over the same 1,000 hot keys.
fn mix(read: &[&str], write: &[&str]) -> Vec<ka_core::scenario::RequestType> {
    let mut r = rtype("read", 0.8, read);
    let mut w = rtype("write", 0.2, write);
    for t in [&mut r, &mut w] {
        t.key_space = 1_000;
        t.key_skew = Some(KeySkew { zipf: 1.1 });
    }
    vec![r, w]
}

fn replicated(lag_ms: f64) -> ka_core::Design {
    design(
        vec![
            node("client", K::Client, Flavour::Web, &[("rps", 2_000.0)]),
            node("app", K::App, Flavour::Go, &[("instances", 2.0)]),
            node(
                "db",
                K::Sql,
                Flavour::Postgresql,
                &[("replication_lag_ms", lag_ms)],
            ),
            node(
                "db-replica",
                K::Sql,
                Flavour::Postgresql,
                &[("replication_lag_ms", lag_ms)],
            ),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "db", Protocol::Query),
            edge("db", "db-replica", Protocol::Replicate),
        ],
    )
}

#[test]
fn replica_reads_are_stale_while_the_replica_lags() {
    let s = scenario(
        0.0,
        20.0,
        mix(&["compute", "datastore"], &["compute", "datastore:write"]),
    );
    let lagging = run(&replicated(500.0), &s);
    let tight = run(&replicated(1.0), &s);
    assert!(lagging.stale_rate > 0.05, "{}", lagging.stale_rate);
    assert!(
        tight.stale_rate < lagging.stale_rate / 10.0,
        "{}",
        tight.stale_rate
    );
}

#[test]
fn cache_aside_serves_stale_copies_and_write_through_mostly_does_not() {
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[("rps", 2_000.0)]),
            node("app", K::App, Flavour::Go, &[("instances", 2.0)]),
            node("cache", K::Cache, Flavour::Redis, &[("ttl_s", 300.0)]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "cache", Protocol::Query),
            edge("app", "db", Protocol::Query),
        ],
    );
    let read = ["compute", "cache?", "datastore"];
    let aside = run(
        &d,
        &scenario(0.0, 20.0, mix(&read, &["compute", "datastore:write"])),
    );
    let through = run(
        &d,
        &scenario(
            0.0,
            20.0,
            mix(&read, &["compute", "datastore:write", "cache:write"]),
        ),
    );
    // Hot keys are written every few seconds and cached for five minutes: without
    // invalidation, most hits after the first write are stale.
    assert!(aside.stale_rate > 0.2, "{}", aside.stale_rate);
    assert!(
        through.stale_rate < aside.stale_rate / 5.0,
        "{}",
        through.stale_rate
    );
}

#[test]
fn a_single_database_never_serves_stale_reads() {
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[("rps", 1_000.0)]),
            node("app", K::App, Flavour::Go, &[]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "db", Protocol::Query),
        ],
    );
    let r = run(
        &d,
        &scenario(
            0.0,
            10.0,
            mix(&["compute", "datastore"], &["compute", "datastore:write"]),
        ),
    );
    assert_eq!(r.stale_reads, 0.0);
}
