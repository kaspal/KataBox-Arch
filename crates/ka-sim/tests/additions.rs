//! M5.0 simulator additions (ADR 0006): worker leases, change data capture, cost per shard,
//! key-hash balancing, read-from-primary, provider rate cuts and network splits.

mod common;

use std::collections::BTreeMap;
use std::sync::Arc;

use common::*;
use ka_core::scenario::{ChaosEvent, External, Measure, Selector};
use ka_core::{
    ComponentKind as K, Design, Flavour, Group, GroupKind, Point, PropValue, Protocol, Size,
    registry,
};
use ka_sim::{Engine, RunResult, SimError, SimOptions, compile, simulate};

fn run(d: &Design, s: &ka_core::scenario::Scenario) -> RunResult {
    simulate(d, &registry(), s, 7, &SimOptions::default()).expect("the design runs")
}

fn cost_of(r: &RunResult, id: &str) -> f64 {
    r.nodes
        .iter()
        .find(|n| n.id == id)
        .expect("node")
        .cost_month
}

fn kill(id: &str, at: f64, for_s: f64) -> ChaosEvent {
    ChaosEvent::Kill {
        target: Selector {
            id: Some(id.into()),
            ..Default::default()
        },
        at,
        for_s: Some(for_s),
    }
}

fn text_prop(d: &mut Design, id: &str, key: &str, v: &str) {
    let n = d.nodes.iter_mut().find(|n| n.id == id).expect("node");
    n.props.insert(key.into(), PropValue::Text(v.into()));
}

// --- Worker leases ----------------------------------------------------------------------

/// client → app → broker → 2 workers (1 s jobs) → PostgreSQL, measured at completion.
fn jobs(broker: ka_core::Node) -> (Design, ka_core::scenario::Scenario) {
    let b = broker.id.clone();
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            node("app", K::App, Flavour::Go, &[]),
            broker,
            node("w1", K::Worker, Flavour::Celery, &[("service_ms", 1000.0)]),
            node("w2", K::Worker, Flavour::Celery, &[("service_ms", 1000.0)]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", &b, Protocol::Publish),
            edge(&b, "w1", Protocol::Consume),
            edge(&b, "w2", Protocol::Consume),
            edge("w1", "db", Protocol::Query),
            edge("w2", "db", Protocol::Query),
        ],
    );
    let mut job = rtype(
        "job",
        1.0,
        &["compute", "broker", "worker", "datastore:write"],
    );
    job.measure = Measure::Completion;
    (d, scenario(10.0, 30.0, vec![job]))
}

#[test]
fn a_dead_workers_queue_messages_come_back_after_the_visibility_timeout() {
    let q = node(
        "q",
        K::Queue,
        Flavour::Sqs,
        &[("visibility_timeout_s", 12.0)],
    );
    let (d, mut s) = jobs(q);
    let calm = run(&d, &s);
    s.chaos.push(kill("w1", 5.0, 30.0));
    let r = run(&d, &s);
    // The ~5 jobs w1 held come back 12 s later (>1% of 300): nothing is lost.
    assert_eq!(r.lost, 0.0);
    let p99 = r.completion.unwrap().p99_ms;
    assert!(p99 > 12_000.0, "leased jobs wait out the timeout: {p99}");
    // Without the crash: 1 s jobs with a p99 tail of about 4 s.
    assert!(calm.completion.unwrap().p99_ms < 6_000.0);
}

#[test]
fn a_dead_stream_consumer_s_messages_come_back_after_the_session_timeout() {
    let st = node(
        "st",
        K::Stream,
        Flavour::Kafka,
        &[("partitions", 16.0), ("session_timeout_s", 20.0)],
    );
    let (d, mut s) = jobs(st);
    s.duration_s = 40.0;
    s.chaos.push(kill("w1", 5.0, 40.0));
    let r = run(&d, &s);
    assert_eq!(r.lost, 0.0);
    let p99 = r.completion.unwrap().p99_ms;
    assert!(p99 > 20_000.0, "{p99}");
}

#[test]
fn leases_still_count_attempts_toward_the_dlq() {
    // Workers that die and stay dead: each redelivery is an attempt; after max_retries the
    // message is lost (no DLQ wired).
    let q = node(
        "q",
        K::Queue,
        Flavour::Sqs,
        &[("visibility_timeout_s", 1.0), ("max_retries", 0.0)],
    );
    let (d, mut s) = jobs(q);
    s.chaos.push(kill("w1", 5.0, 3.0));
    let r = run(&d, &s);
    assert!(r.lost > 0.0, "no retries left: lost");
}

// --- Change data capture ----------------------------------------------------------------

/// client → app → PostgreSQL --CDC--> Kafka → indexer → Elasticsearch.
fn cdc() -> (Design, ka_core::scenario::Scenario) {
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            node("app", K::App, Flavour::Go, &[]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
            node("cdc", K::Stream, Flavour::Kafka, &[]),
            node(
                "indexer",
                K::Worker,
                Flavour::Celery,
                &[("service_ms", 5.0)],
            ),
            node("search", K::Search, Flavour::Elasticsearch, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "db", Protocol::Query),
            edge("db", "cdc", Protocol::Publish),
            edge("cdc", "indexer", Protocol::Consume),
            edge("indexer", "search", Protocol::Query),
        ],
    );
    let mut update = rtype(
        "update",
        1.0,
        &[
            "compute",
            "datastore:write:sql",
            "cdc",
            "worker",
            "datastore:write:search",
        ],
    );
    update.measure = Measure::Completion;
    update.key_space = 50;
    (d, scenario(200.0, 20.0, vec![update]))
}

#[test]
fn cdc_answers_at_the_commit_and_completes_through_the_stream() {
    let (d, s) = cdc();
    let r = run(&d, &s);
    assert_eq!(r.errors, 0.0);
    assert_eq!(r.lost, 0.0);
    let response = r.response.p95_ms;
    let completion = r.completion.unwrap().p95_ms;
    assert!(
        response < 30.0,
        "the answer leaves at the commit: {response}"
    );
    assert!(
        completion > response,
        "the index is updated later: {completion} vs {response}"
    );
    // The stream and the indexer really carried the changes.
    let served = |id: &str| r.nodes.iter().find(|n| n.id == id).unwrap().served;
    assert!(served("cdc") > 3000.0 && served("indexer") > 3000.0);
}

#[test]
fn a_cdc_route_needs_the_cdc_edge() {
    let (mut d, s) = cdc();
    d.edges.retain(|e| !(e.from == "db" && e.to == "cdc"));
    let err = simulate(&d, &registry(), &s, 1, &SimOptions::default()).unwrap_err();
    assert!(matches!(err, SimError::NoPath { .. }), "{err:?}");
}

#[test]
fn a_cdc_step_must_follow_a_write() {
    let (d, mut s) = cdc();
    s.request_types[0].route = ["compute", "cdc", "worker"].map(String::from).to_vec();
    assert!(matches!(
        simulate(&d, &registry(), &s, 1, &SimOptions::default()),
        Err(SimError::BadScenario(_))
    ));
}

#[test]
fn changes_wait_for_a_stream_that_is_down_and_keep_their_order() {
    let (d, mut s) = cdc();
    s.chaos.push(kill("cdc", 5.0, 8.0));
    let c = compile(&d, &registry(), &s, &SimOptions::default()).unwrap();
    let mut e = Engine::new(Arc::new(c), 3);
    e.set_cdc_audit(true);
    e.run();
    let r = e.result();
    assert_eq!(r.lost, 0.0, "delayed, not lost");
    assert!(r.completion.unwrap().p99_ms > 5_000.0, "changes waited");
    // Per key, the stream accepted changes in commit order.
    let mut last: BTreeMap<u64, u64> = BTreeMap::new();
    for &(key, at) in e.cdc_audit() {
        let prev = last.insert(key, at).unwrap_or(0);
        assert!(at >= prev, "key {key}: {at} accepted after {prev}");
    }
    assert!(e.cdc_audit().len() > 3000);
}

#[test]
fn changes_older_than_the_cdc_buffer_are_lost() {
    let (mut d, mut s) = cdc();
    d.nodes[2]
        .props
        .insert("cdc_buffer_s".into(), PropValue::Number(2.0));
    s.chaos.push(kill("cdc", 5.0, 8.0));
    let r = run(&d, &s);
    assert!(r.lost > 0.0);
}

// --- Cost per shard -----------------------------------------------------------------------

#[test]
fn sharded_stores_cost_per_shard_relative_to_their_default() {
    let (mut d, s) = golden_a();
    let one = cost_of(&run(&d, &s), "db");
    d.nodes[2]
        .props
        .insert("shards".into(), PropValue::Number(3.0));
    let three = cost_of(&run(&d, &s), "db");
    assert!((three - 3.0 * one).abs() < 0.01, "{one} → {three}");

    // Search defaults to 2 shards: 1 shard costs half the default card.
    let mut search = golden_a().0;
    search.nodes[2] = node("db", K::Search, Flavour::Elasticsearch, &[]);
    let s2 = scenario(
        100.0,
        10.0,
        vec![rtype("q", 1.0, &["compute", "datastore:search"])],
    );
    let two = cost_of(&run(&search, &s2), "db");
    search.nodes[2]
        .props
        .insert("shards".into(), PropValue::Number(1.0));
    let half = cost_of(&run(&search, &s2), "db");
    assert!((two - 2.0 * half).abs() < 0.01, "{two} vs {half}");
}

// --- Key hash, read from primary, provider rate cuts --------------------------------------

#[test]
fn key_hash_keeps_a_key_on_one_server() {
    let (mut d, mut s) = golden_b(1000.0);
    text_prop(&mut d, "lb", "algorithm", "key_hash");
    s.request_types[0].key_space = 1;
    s.request_types[0].key_skew = None;
    s.request_types[1].key_space = 1;
    let r = run(&d, &s);
    let served = |id: &str| r.nodes.iter().find(|n| n.id == id).unwrap().served;
    let (a, b) = (served("app-1"), served("app-2"));
    assert!(a == 0.0 || b == 0.0, "one key, one server: {a} / {b}");
    // The old spelling still means the same (files before v3).
    text_prop(&mut d, "lb", "algorithm", "ip_hash");
    let old = run(&d, &s);
    assert_eq!(old.nodes[2].served, r.nodes[2].served);
}

#[test]
fn reading_from_the_primary_avoids_stale_replica_reads() {
    let (mut d, mut s) = golden_b(2000.0);
    // No cache: every read hits the cluster; writes to a small hot key set.
    d.edges.retain(|e| e.to != "cache");
    for t in &mut s.request_types {
        t.key_space = 100;
        t.key_skew = None;
    }
    s.request_types[1].share = 0.3;
    d.nodes[6]
        .props
        .insert("replication_lag_ms".into(), PropValue::Number(500.0));
    let spread = run(&d, &s);
    assert!(spread.stale_rate > 0.01, "{}", spread.stale_rate);
    text_prop(&mut d, "db", "read_from", "primary");
    let primary = run(&d, &s);
    assert_eq!(primary.stale_reads, 0.0);
    let replica = primary.nodes.iter().find(|n| n.id == "db-replica").unwrap();
    assert_eq!(replica.served, 0.0);
}

#[test]
fn a_provider_can_cut_its_rate_limit() {
    let (d, mut s) = jobs(node("q", K::Queue, Flavour::Sqs, &[]));
    s.request_types[0].route = ["compute", "external:email"].map(String::from).to_vec();
    s.request_types[0].measure = Measure::Response;
    s.peak_rps = 100.0;
    s.externals.push(External {
        name: "email".into(),
        median_ms: 20.0,
        p99_ms: 80.0,
        rate_limit: None,
    });
    let calm = run(&d, &s);
    assert_eq!(calm.errors, 0.0);
    s.chaos.push(ChaosEvent::External {
        name: "email".into(),
        at: 5.0,
        for_s: Some(10.0),
        down: false,
        add_ms: 0.0,
        rate: Some(20.0),
    });
    let cut = run(&d, &s);
    // 100/s offered, 20/s allowed for 10 of 30 s: about 800 of 3000 fail.
    let share = cut.errors / cut.requests;
    assert!((0.2..0.35).contains(&share), "{share}");
}

// --- Network splits -----------------------------------------------------------------------

fn region(id: &str) -> Group {
    Group {
        id: id.into(),
        kind: GroupKind::Region,
        label: id.into(),
        position: Point { x: 0.0, y: 0.0 },
        size: Size { w: 400.0, h: 300.0 },
        parent_id: None,
        scaling: None,
    }
}

/// An LB (outside both regions) → an app in each region; the SQL primary in "us", its
/// replica in "eu". Users in "eu" also reach `app-eu` directly.
fn two_regions() -> (Design, ka_core::scenario::Scenario) {
    let mut d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            node("client-eu", K::Client, Flavour::Web, &[]),
            node("lb", K::Lb, Flavour::AwsAlb, &[]),
            node("app-us", K::App, Flavour::Go, &[("instances", 2.0)]),
            node("app-eu", K::App, Flavour::Go, &[("instances", 2.0)]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
            node("db-eu", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![
            edge("client", "lb", Protocol::Http),
            edge("client-eu", "app-eu", Protocol::Http),
            edge("lb", "app-us", Protocol::Http),
            edge("lb", "app-eu", Protocol::Http),
            edge("app-us", "db", Protocol::Query),
            edge("app-eu", "db", Protocol::Query),
            edge("db", "db-eu", Protocol::Replicate),
        ],
    );
    d.groups = vec![region("us"), region("eu")];
    for (n, g) in [
        ("app-us", "us"),
        ("db", "us"),
        ("app-eu", "eu"),
        ("db-eu", "eu"),
        ("client-eu", "eu"),
    ] {
        d.nodes.iter_mut().find(|x| x.id == n).unwrap().group_id = Some(g.into());
    }
    let mut read = rtype("read", 0.9, &["edge?", "compute", "datastore"]);
    read.key_space = 1000;
    let mut write = rtype("write", 0.1, &["edge?", "compute", "datastore:write"]);
    write.key_space = 1000;
    (d, scenario(500.0, 40.0, vec![read, write]))
}

#[test]
fn a_split_cuts_traffic_across_its_border_until_routing_learns() {
    let (d, mut s) = two_regions();
    let calm = run(&d, &s);
    assert_eq!(calm.errors, 0.0);
    s.chaos.push(ChaosEvent::Split {
        zone: "eu".into(),
        at: 10.0,
        for_s: Some(20.0),
    });
    let r = run(&d, &s);
    // Requests the LB sends into "eu" fail until two health checks pass (~2–4 s), then it
    // keeps to "us". Users inside "eu" are still served there, but their writes can't reach
    // the primary in "us", and the eu replica stops receiving writes: its reads go stale.
    let share = r.errors / r.requests;
    assert!(share > 0.0 && share < 0.15, "{share}");
    let down: u32 = r.nodes.iter().map(|n| n.down_seconds.len() as u32).sum();
    assert_eq!(down, 0, "a split kills nothing");
    assert!(
        r.stale_reads > calm.stale_reads + 100.0,
        "{} vs {}",
        r.stale_reads,
        calm.stale_reads
    );
}
