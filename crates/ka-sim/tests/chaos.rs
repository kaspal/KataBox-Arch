//! Chaos, health checks, failover and autoscaling (plan M1.5, ADR 0006).

mod common;

use std::collections::BTreeMap;

use common::*;
use ka_core::scenario::{ChaosEvent, External, Role, Selector};
use ka_core::{
    ComponentKind as K, Flavour, Group, GroupKind, Point, PropValue, Protocol, Scaling, Size,
    registry,
};
use ka_sim::{RunResult, SimOptions, simulate};

fn run(d: &ka_core::Design, s: &ka_core::scenario::Scenario) -> RunResult {
    simulate(d, &registry(), s, 1, &SimOptions::default()).expect("the design runs")
}

fn errors(r: &RunResult, from: usize, to: usize) -> f64 {
    r.series[from..to].iter().map(|s| s.errors).sum()
}

fn requests(r: &RunResult, from: usize, to: usize) -> f64 {
    r.series[from..to].iter().map(|s| s.requests).sum()
}

fn by_id(id: &str) -> Selector {
    Selector {
        id: Some(id.into()),
        ..Selector::default()
    }
}

#[test]
fn a_dead_app_server_fails_requests_until_health_checks_take_it_out() {
    let (d, mut s) = golden_b(6_000.0);
    s.duration_s = 40.0;
    s.chaos.push(ChaosEvent::Kill {
        target: by_id("app-1"),
        at: 10.0,
        for_s: Some(20.0),
    });
    let r = run(&d, &s);
    assert!(errors(&r, 10, 13) > 100.0, "{:?}", &r.series[9..14]);
    // Health checks noticed within 4 s; the other server carries the load, and the
    // revived one rejoins only after it is seen healthy again.
    assert_eq!(errors(&r, 14, 40), 0.0, "{:?}", &r.series[13..20]);
    assert_eq!(errors(&r, 0, 10), 0.0);
}

#[test]
fn a_dead_primary_fails_over_to_its_replica() {
    let (mut d, mut s) = golden_b(6_000.0);
    s.duration_s = 40.0;
    let db = d.nodes.iter_mut().find(|n| n.id == "db").unwrap();
    db.props.insert("failover_s".into(), PropValue::Number(5.0));
    s.chaos.push(ChaosEvent::Kill {
        target: Selector {
            kind: Some(K::Sql),
            role: Some(Role::Primary),
            ..Selector::default()
        },
        at: 10.0,
        for_s: None,
    });
    let r = run(&d, &s);
    // Writes fail until the replica is promoted (5 s), reads until health checks notice.
    assert!(errors(&r, 10, 15) > 0.0);
    assert_eq!(errors(&r, 16, 40), 0.0, "{:?}", &r.series[14..20]);
    let replica = r.nodes.iter().find(|n| n.id == "db-replica").unwrap();
    assert!(
        replica.util_series[30] > replica.util_series[5],
        "the replica now takes the writes"
    );
}

#[test]
fn added_latency_raises_p95_only_while_it_lasts() {
    let (d, mut s) = golden_b(6_000.0);
    s.duration_s = 30.0;
    s.chaos.push(ChaosEvent::Latency {
        target: Selector {
            kind: Some(K::App),
            all: true,
            ..Selector::default()
        },
        add_ms: 200.0,
        at: 10.0,
        for_s: Some(10.0),
    });
    let r = run(&d, &s);
    assert!(r.series[15].p95_ms > 200.0, "{}", r.series[15].p95_ms);
    assert!(r.series[5].p95_ms < 60.0);
    assert!(r.series[25].p95_ms < 60.0);
}

#[test]
fn a_flushed_cache_sends_its_reads_to_the_database() {
    let (d, mut s) = golden_b(6_000.0);
    s.duration_s = 30.0;
    s.chaos.push(ChaosEvent::Flush {
        target: by_id("cache"),
        at: 20.0,
    });
    let r = run(&d, &s);
    let db = |id: &str, t: usize| r.nodes.iter().find(|n| n.id == id).unwrap().util_series[t];
    let before = db("db", 18) + db("db-replica", 18);
    let after = db("db", 20) + db("db-replica", 20);
    assert!(after > 2.0 * before, "before {before}, after {after}");
}

#[test]
fn a_dead_cache_becomes_a_miss_once_health_checks_notice() {
    let (d, mut s) = golden_b(6_000.0);
    s.duration_s = 30.0;
    s.chaos.push(ChaosEvent::Kill {
        target: by_id("cache"),
        at: 10.0,
        for_s: None,
    });
    let r = run(&d, &s);
    // Reads fail until health checks notice (2-4 s), then go straight to the database.
    assert!(errors(&r, 10, 12) > 0.0);
    assert_eq!(errors(&r, 15, 30), 0.0, "{:?}", &r.series[12..17]);
    let db = |t: usize| {
        r.nodes.iter().find(|n| n.id == "db").unwrap().util_series[t]
            + r.nodes
                .iter()
                .find(|n| n.id == "db-replica")
                .unwrap()
                .util_series[t]
    };
    assert!(
        db(20) > 3.0 * db(5),
        "the database takes the reads: {} vs {}",
        db(20),
        db(5)
    );
}

#[test]
fn a_partitioned_zone_is_unreachable_while_it_lasts() {
    let mut d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[("rps", 500.0)]),
            node("app", K::App, Flavour::Go, &[]),
        ],
        vec![edge("client", "app", Protocol::Http)],
    );
    d.groups.push(Group {
        id: "az-1".into(),
        kind: GroupKind::Az,
        label: "AZ a".into(),
        position: Point { x: 0.0, y: 0.0 },
        size: Size { w: 400.0, h: 300.0 },
        parent_id: None,
        scaling: None,
        region_tag: None,
        mesh: None,
    });
    d.nodes[1].group_id = Some("az-1".into());
    let mut s = scenario(0.0, 20.0, vec![rtype("call", 1.0, &["compute"])]);
    s.chaos.push(ChaosEvent::Partition {
        zone: "AZ a".into(),
        at: 5.0,
        for_s: Some(5.0),
    });
    let r = run(&d, &s);
    assert!(errors(&r, 6, 9) > 0.9 * requests(&r, 6, 9));
    assert_eq!(errors(&r, 11, 20), 0.0);
}

#[test]
fn an_external_outage_fails_calls_to_it() {
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[("rps", 300.0)]),
            node("app", K::App, Flavour::Go, &[]),
        ],
        vec![edge("client", "app", Protocol::Http)],
    );
    let mut s = scenario(
        0.0,
        20.0,
        vec![rtype("pay", 1.0, &["compute", "external:payment"])],
    );
    s.externals.push(External {
        name: "payment".into(),
        median_ms: 50.0,
        p99_ms: 200.0,
        rate_limit: None,
    });
    s.chaos.push(ChaosEvent::External {
        name: "payment".into(),
        at: 5.0,
        for_s: Some(5.0),
        down: true,
        add_ms: 0.0,
        rate: None,
    });
    let r = run(&d, &s);
    // Each call to the dead provider holds an app thread for the 1 s connect timeout: the
    // 128 threads fail 128 calls a second, nothing succeeds, and the rest queue behind them
    // (thread-pool exhaustion from a dead dependency).
    let ok: f64 = r.series[6..9].iter().map(|s| s.ok).sum();
    assert_eq!(ok, 0.0);
    assert!(errors(&r, 6, 10) >= 4.0 * 120.0, "{:?}", &r.series[5..11]);
    assert!(requests(&r, 6, 9) > 0.0);
    assert_eq!(errors(&r, 12, 20), 0.0);
}

#[test]
fn an_autoscaling_group_grows_under_load_and_costs_more() {
    let mut d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[("rps", 600.0)]),
            node(
                "app",
                K::App,
                Flavour::Go,
                &[
                    ("instances", 1.0),
                    ("concurrency", 8.0),
                    ("service_ms", 20.0),
                ],
            ),
        ],
        vec![edge("client", "app", Protocol::Http)],
    );
    let props: BTreeMap<String, PropValue> = [
        ("min_instances", 1.0),
        ("max_instances", 8.0),
        ("target_util", 60.0),
        ("scale_delay_s", 5.0),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), PropValue::Number(v)))
    .collect();
    d.groups.push(Group {
        id: "asg".into(),
        kind: GroupKind::Autoscale,
        label: "web tier".into(),
        position: Point { x: 0.0, y: 0.0 },
        size: Size { w: 400.0, h: 300.0 },
        parent_id: None,
        scaling: Some(Scaling {
            flavour: Flavour::KubernetesHpa,
            props,
        }),
        region_tag: None,
        mesh: None,
    });
    d.nodes[1].group_id = Some("asg".into());
    let s = scenario(0.0, 60.0, vec![rtype("call", 1.0, &["compute"])]);
    let r = run(&d, &s);
    // 600 req/s × 20 ms needs 12 busy servers; one instance has 8. It overflows at first,
    // then scales out (5 s tick + 5 s delay) and settles.
    assert!(errors(&r, 0, 10) > 0.0);
    assert_eq!(errors(&r, 30, 60), 0.0, "{:?}", &r.series[25..35]);
    let app = r.nodes.iter().find(|n| n.id == "app").unwrap();
    assert!(app.util_series[50] < 0.8, "{}", app.util_series[50]);
    assert!(
        app.cost_month > 70.0 * 1.5,
        "mean instances above 1: {}",
        app.cost_month
    );
}

#[test]
fn chaos_runs_are_deterministic() {
    let (d, mut s) = golden_b(12_000.0);
    s.duration_s = 30.0;
    s.chaos.push(ChaosEvent::Kill {
        target: by_id("app-2"),
        at: 5.0,
        for_s: Some(10.0),
    });
    s.chaos.push(ChaosEvent::Flush {
        target: by_id("cache"),
        at: 12.0,
    });
    s.chaos.push(ChaosEvent::Traffic {
        x: 2.0,
        at: 15.0,
        for_s: Some(5.0),
    });
    let a = serde_json::to_string(&run(&d, &s)).unwrap();
    let b = serde_json::to_string(&run(&d, &s)).unwrap();
    assert_eq!(a, b);
}
