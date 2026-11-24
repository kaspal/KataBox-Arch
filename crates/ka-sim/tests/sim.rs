//! Simulator tests: goldens, determinism, behaviours (ADR 0006).

mod common;

use std::path::PathBuf;

use common::*;
use ka_core::scenario::{External, Fanout, KeySkew, Measure, Source};
use ka_core::{ComponentKind as K, Flavour, Protocol, registry};
use ka_sim::{RunResult, SimError, SimOptions, compile, simulate};

fn run(d: &ka_core::Design, s: &ka_core::scenario::Scenario, seed: u64) -> RunResult {
    simulate(d, &registry(), s, seed, &SimOptions::default()).expect("the design runs")
}

fn node<'a>(r: &'a RunResult, id: &str) -> &'a ka_sim::NodeResult {
    r.nodes.iter().find(|n| n.id == id).expect("node")
}

// --- Goldens ------------------------------------------------------------------------------

/// Compare with `tests/golden/<name>.json`; `KA_UPDATE_GOLDENS=1` rewrites it. A golden only
/// changes on purpose, and the commit says why.
fn golden(name: &str, r: &RunResult) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.json"));
    let got = serde_json::to_string_pretty(r).unwrap() + "\n";
    if std::env::var_os("KA_UPDATE_GOLDENS").is_some() {
        std::fs::write(&path, &got).unwrap();
    }
    let want = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        got == want,
        "golden {name} changed; review, then run with KA_UPDATE_GOLDENS=1"
    );
}

#[test]
fn golden_a_single_app_and_database() {
    let (d, s) = golden_a();
    let r = run(&d, &s, 42);
    assert_eq!(r.errors, 0.0);
    golden("a", &r);
}

#[test]
fn golden_b_cache_and_replicas() {
    let (d, s) = golden_b(12_000.0);
    let r = run(&d, &s, 42);
    assert_eq!(r.errors, 0.0);
    golden("b", &r);
}

#[test]
fn golden_c_stream_and_workers() {
    let (d, s) = golden_c();
    let r = run(&d, &s, 42);
    assert!(r.completion.is_some());
    golden("c", &r);
}

#[test]
fn golden_d_change_data_capture_through_a_stream_outage() {
    let (d, s) = golden_d();
    let r = run(&d, &s, 42);
    assert_eq!(r.errors, 0.0);
    assert_eq!(r.lost, 0.0, "changes wait in the database log, none lost");
    golden("d", &r);
}

// --- Determinism --------------------------------------------------------------------------

#[test]
fn the_same_seed_gives_byte_identical_results() {
    for (d, s) in [golden_a(), golden_b(12_000.0), golden_c()] {
        let a = serde_json::to_string(&run(&d, &s, 7)).unwrap();
        let b = serde_json::to_string(&run(&d, &s, 7)).unwrap();
        assert_eq!(a, b);
    }
    let (d, s) = golden_a();
    assert_ne!(
        serde_json::to_string(&run(&d, &s, 1)).unwrap(),
        serde_json::to_string(&run(&d, &s, 2)).unwrap()
    );
}

#[test]
fn snapshots_resume_identically() {
    // Clone the engine mid-run (a scrubbing snapshot): both copies finish the same.
    let (d, s) = golden_b(6_000.0);
    let c = std::sync::Arc::new(compile(&d, &registry(), &s, &SimOptions::default()).unwrap());
    let mut e = ka_sim::Engine::new(c, 3);
    e.advance_to(12_000_000);
    let mut copy = e.clone();
    e.run();
    copy.run();
    assert_eq!(
        serde_json::to_string(&e.result()).unwrap(),
        serde_json::to_string(&copy.result()).unwrap()
    );
}

#[test]
fn tracing_and_chunked_advance_never_change_a_run() {
    // Live runs advance in frame-sized steps and record packets: the result is the same
    // as one uninterrupted run.
    let (d, s) = golden_b(6_000.0);
    let c = std::sync::Arc::new(compile(&d, &registry(), &s, &SimOptions::default()).unwrap());
    let mut plain = ka_sim::Engine::new(c.clone(), 5);
    plain.run();
    let mut live = ka_sim::Engine::new(c, 5);
    live.set_trace(true);
    let mut packets = 0;
    let mut t = 0;
    while !live.finished() {
        t += 33_333;
        live.advance_to(t);
        packets += live.take_packets().len();
        let snap = live.snapshot();
        assert_eq!(snap.t_us, live.now_us());
    }
    assert!(packets > 100_000, "{packets} packets");
    assert_eq!(
        serde_json::to_string(&plain.result()).unwrap(),
        serde_json::to_string(&live.result()).unwrap()
    );
}

#[test]
fn snapshots_report_load_and_the_last_second() {
    let (d, s) = golden_b(6_000.0);
    let c = std::sync::Arc::new(compile(&d, &registry(), &s, &SimOptions::default()).unwrap());
    let mut e = ka_sim::Engine::new(c, 1);
    e.advance_to(10_500_000);
    let snap = e.snapshot();
    let last = snap.last_second.expect("ten seconds are done");
    assert_eq!(last.t, 9);
    assert!(
        (5_000.0..7_000.0).contains(&last.requests),
        "{}",
        last.requests
    );
    let app = &snap.nodes[2];
    let util = app.busy_us / (app.servers * 10_500_000.0);
    assert!((0.05..0.5).contains(&util), "{util}");
    assert!(snap.cost_month > 0.0);
}

// --- Behaviour ----------------------------------------------------------------------------

#[test]
fn an_overloaded_server_queues_then_drops() {
    let (mut d, s) = golden_a();
    // One instance, two workers, 100 ms per request: capacity 20 req/s for 1000 req/s.
    d.nodes[1] = node_with(
        "app",
        K::App,
        Flavour::Go,
        &[
            ("instances", 1.0),
            ("concurrency", 2.0),
            ("service_ms", 100.0),
        ],
    );
    let r = run(&d, &s, 1);
    assert!(r.error_rate > 0.9, "{}", r.error_rate);
    let app = node(&r, "app");
    assert!(app.drops > 0.0 && app.utilisation > 0.95);
}

fn node_with(
    id: &str,
    kind: ka_core::ComponentKind,
    flavour: Flavour,
    props: &[(&str, f64)],
) -> ka_core::Node {
    common::node(id, kind, flavour, props)
}

#[test]
fn a_slow_database_ties_up_the_app_through_held_slots() {
    let (mut d, s) = golden_a();
    d.nodes[2] = node_with("db", K::Sql, Flavour::Postgresql, &[("service_ms", 60.0)]);
    let slow = run(&d, &s, 1);
    let fast = run(&golden_a().0, &s, 1);
    assert!(node(&slow, "app").utilisation > 3.0 * node(&fast, "app").utilisation);
}

#[test]
fn a_cache_absorbs_reads_and_hotter_keys_hit_more() {
    let (d, mut s) = golden_b(4_000.0);
    let warm = run(&d, &s, 1);
    let reads_to_db = node(&warm, "db").served + node(&warm, "db-replica").served;
    assert!(
        reads_to_db < 0.3 * warm.requests,
        "the cache serves most reads"
    );
    s.key_skew_override = Some(KeySkew { zipf: 1.5 });
    let hot = run(&d, &s, 1);
    let hot_db = node(&hot, "db").served + node(&hot, "db-replica").served;
    assert!(hot_db < reads_to_db, "hotter keys hit more");
    s.key_skew_override = None;
    s.cold_caches = true;
    let cold = run(&d, &s, 1);
    assert!(
        node(&cold, "db").served + node(&cold, "db-replica").served > reads_to_db,
        "a cold cache misses more"
    );
}

#[test]
fn reads_spread_over_replicas_and_writes_stay_on_the_primary() {
    let (d, mut s) = golden_b(2_000.0);
    s.request_types[1].share = 0.5; // half writes
    let r = run(&d, &s, 1);
    // Writes only reach the primary: it serves far more than the replica.
    assert!(node(&r, "db").served > 2.0 * node(&r, "db-replica").served);
    assert!(node(&r, "db-replica").served > 0.0);
}

#[test]
fn load_balancers_spread_evenly() {
    let (d, s) = golden_b(2_000.0);
    let r = run(&d, &s, 1);
    let (a, b) = (node(&r, "app-1").served, node(&r, "app-2").served);
    assert!((a - b).abs() / (a + b) < 0.02, "{a} vs {b}");
}

#[test]
fn stream_partitions_cap_consumer_parallelism() {
    let (mut d, s) = golden_c();
    let wide = run(&d, &s, 1);
    d.nodes[2] = node_with("stream", K::Stream, Flavour::Kafka, &[("partitions", 2.0)]);
    let narrow = run(&d, &s, 1);
    // 2 partitions × 20 ms can't keep up with 400 msg/s: completion latency explodes.
    assert!(narrow.completion.unwrap().p95_ms > 10.0 * wide.completion.unwrap().p95_ms);
}

#[test]
fn a_consumer_s_calls_take_their_own_servers() {
    // The reserved consumer slot is the worker's, not the store's: the store behind the
    // workers counts its own busy servers (a regression found in M1.2).
    let (d, s) = golden_c();
    let r = run(&d, &s, 1);
    let store = node(&r, "store");
    // 400 DynamoDB writes/s × ~3 ms × 3 write slots over 4 partitions × 8 servers ≈ 0.12.
    assert!(
        (0.08..0.2).contains(&store.utilisation),
        "{}",
        store.utilisation
    );
}

#[test]
fn failing_async_work_is_retried_then_dead_lettered_or_lost() {
    let (mut d, mut s) = golden_c();
    // A queue gives up after `max_retries` (a stream keeps the record: see below).
    d.nodes[2] = node_with("stream", K::Queue, Flavour::Sqs, &[]);
    s.externals.push(External {
        name: "email".into(),
        median_ms: 5.0,
        p99_ms: 20.0,
        rate_limit: Some(50.0),
    });
    s.request_types[0].route = ["compute", "broker", "worker", "external:email"]
        .map(String::from)
        .to_vec();
    let lost = run(&d, &s, 1);
    assert!(
        lost.lost > 0.0 && lost.dead_lettered == 0.0,
        "no DLQ: messages are lost"
    );
    d.nodes
        .push(node_with("dlq", K::Dlq, Flavour::Generic, &[]));
    d.edges.push(edge("stream", "dlq", Protocol::Publish));
    let dl = run(&d, &s, 1);
    assert!(
        dl.dead_lettered > 0.0 && dl.lost == 0.0,
        "with a DLQ they are dead-lettered"
    );
}

#[test]
fn a_stream_keeps_retrying_a_failed_record_instead_of_losing_it() {
    let (d, mut s) = golden_c();
    s.externals.push(External {
        name: "email".into(),
        median_ms: 5.0,
        p99_ms: 20.0,
        rate_limit: Some(50.0),
    });
    s.request_types[0].route = ["compute", "broker", "worker", "external:email"]
        .map(String::from)
        .to_vec();
    let r = run(&d, &s, 1);
    assert_eq!(r.lost, 0.0, "the log keeps it; the consumer retries");
    assert!(r.retried > 0.0);
}

#[test]
fn the_caller_rule_writes_then_publishes() {
    // News Feed's post: the app writes the store, then publishes; a worker fills the cache.
    let d = design(
        vec![
            node_with("client", K::Client, Flavour::Web, &[]),
            node_with("app", K::App, Flavour::Go, &[]),
            node_with("db", K::Sql, Flavour::Postgresql, &[]),
            node_with("queue", K::Queue, Flavour::Sqs, &[]),
            node_with(
                "worker",
                K::Worker,
                Flavour::Celery,
                &[("service_ms", 50.0)],
            ),
            node_with("cache", K::Cache, Flavour::Redis, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "db", Protocol::Query),
            edge("app", "queue", Protocol::Publish),
            edge("queue", "worker", Protocol::Consume),
            edge("worker", "cache", Protocol::Query),
        ],
    );
    let mut post = rtype(
        "post",
        1.0,
        &[
            "compute",
            "datastore:write",
            "broker",
            "worker",
            "cache:write",
        ],
    );
    post.measure = Measure::Completion;
    post.fanout = Some(Fanout {
        mean: 3.0,
        heavy_share: 0.0,
        heavy_x: 1.0,
    });
    let r = run(&d, &scenario(200.0, 20.0, vec![post]), 1);
    assert_eq!(r.errors, 0.0);
    assert!(node(&r, "db").served > 0.0 && node(&r, "cache").served > 0.0);
    let comp = r.completion.unwrap();
    assert!(
        comp.p50_ms > r.response.p50_ms,
        "completion includes the async work: {} vs {}",
        comp.p50_ms,
        r.response.p50_ms
    );
}

#[test]
fn a_design_without_a_path_is_a_lint_error() {
    let d = design(
        vec![
            node_with("client", K::Client, Flavour::Web, &[]),
            node_with("db", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![],
    );
    let s = scenario(
        100.0,
        10.0,
        vec![rtype("read", 1.0, &["compute", "datastore"])],
    );
    match compile(&d, &registry(), &s, &SimOptions::default()) {
        Err(SimError::NoPath { request_type, .. }) => assert_eq!(request_type, "read"),
        other => panic!("expected NoPath, got {:?}", other.err()),
    }
}

#[test]
fn a_rate_limiter_sheds_abusive_traffic_without_counting_errors() {
    let d = design(
        vec![
            node_with("client", K::Client, Flavour::Web, &[]),
            node_with(
                "rl",
                K::Ratelimit,
                Flavour::Envoy,
                &[("limit_rps", 20.0), ("burst", 20.0)],
            ),
            node_with("app", K::App, Flavour::Go, &[]),
        ],
        vec![
            edge("client", "rl", Protocol::Http),
            edge("rl", "app", Protocol::Http),
        ],
    );
    let mut api = rtype("api", 0.5, &["edge", "compute"]);
    api.api_keys = Some(50);
    let mut abuse = rtype("abuse", 0.5, &["edge", "compute"]);
    abuse.api_keys = Some(1);
    abuse.abusive = true;
    let r = run(&d, &scenario(400.0, 10.0, vec![api, abuse]), 1);
    assert!(r.shed > 1000.0, "the abusive key is limited: {}", r.shed);
    assert!(
        r.error_rate < 0.01,
        "legit keys stay under their limit: {}",
        r.error_rate
    );
}

#[test]
fn scheduler_sourced_jobs_complete_through_a_queue() {
    let d = design(
        vec![
            node_with("cron", K::Scheduler, Flavour::Cron, &[]),
            node_with("queue", K::Queue, Flavour::Sqs, &[]),
            node_with(
                "worker",
                K::Worker,
                Flavour::Sidekiq,
                &[("service_ms", 10.0)],
            ),
            node_with("db", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![
            edge("cron", "queue", Protocol::Publish),
            edge("queue", "worker", Protocol::Consume),
            edge("worker", "db", Protocol::Query),
        ],
    );
    let mut job = rtype("job", 1.0, &["broker", "worker", "datastore:write"]);
    job.source = Source::Scheduler;
    job.measure = Measure::Completion;
    let r = run(&d, &scenario(100.0, 10.0, vec![job]), 1);
    assert!(r.ok > 900.0 && r.errors == 0.0, "{} ok", r.ok);
}

#[test]
fn sampling_keeps_utilisation_close_to_a_full_run() {
    // A small cap forces sampling at a load a full run handles quickly.
    let (d, s) = golden_b(3_000.0);
    let full = simulate(&d, &registry(), &s, 5, &SimOptions { sample_cap: 1e9 }).unwrap();
    let sampled = simulate(&d, &registry(), &s, 5, &SimOptions { sample_cap: 1000.0 }).unwrap();
    assert!(sampled.sample_factor < 0.5);
    for id in ["app-1", "cache", "db"] {
        let (a, b) = (node(&full, id).utilisation, node(&sampled, id).utilisation);
        assert!(
            (a - b).abs() <= 0.02 + 0.1 * a,
            "{id}: full {a} vs sampled {b}"
        );
    }
    assert!((full.requests - sampled.requests).abs() / full.requests < 0.03);
}

/// Review A1: a sampled run agrees with a full one at peak load (p95 and utilisation within
/// ±10%, same pass/fail). Release only: `cargo test -p ka-sim --release -- --ignored`.
#[test]
#[ignore]
fn accuracy_sampled_vs_full_at_peak() {
    for (name, (d, s)) in [
        ("a", golden_a()),
        ("b", golden_b(12_000.0)),
        ("b50k", golden_b(50_000.0)),
        ("c", golden_c()),
    ] {
        let full = simulate(&d, &registry(), &s, 11, &SimOptions { sample_cap: 1e12 }).unwrap();
        assert_eq!(full.sample_factor, 1.0);
        let sampled = simulate(&d, &registry(), &s, 11, &SimOptions::default()).unwrap();
        let p = (sampled.latency.p95_ms - full.latency.p95_ms).abs() / full.latency.p95_ms;
        println!(
            "{name}: f={:.3} p95 full {:.3} sampled {:.3} ({:+.1}%); requests {:.0} vs {:.0}",
            sampled.sample_factor,
            full.latency.p95_ms,
            sampled.latency.p95_ms,
            p * 100.0,
            full.requests,
            sampled.requests
        );
        for (a, b) in full.nodes.iter().zip(&sampled.nodes) {
            println!(
                "    {:12} util full {:.4} sampled {:.4}",
                a.id, a.utilisation, b.utilisation
            );
        }
        let verdict = |r: &RunResult| r.error_rate <= 0.001 && r.latency.p95_ms <= 80.0;
        assert_eq!(
            verdict(&full),
            verdict(&sampled),
            "{name}: the verdict must not depend on sampling"
        );
        assert!(p <= 0.10, "{name}: p95 off by {:.1}%", p * 100.0);
        for (a, b) in full.nodes.iter().zip(&sampled.nodes) {
            if a.utilisation > 0.05 {
                let du = (a.utilisation - b.utilisation).abs() / a.utilisation;
                assert!(
                    du <= 0.10,
                    "{name}/{}: utilisation {} vs {}",
                    a.id,
                    a.utilisation,
                    b.utilisation
                );
            }
        }
    }
}

/// The engine bench (plan M1.1): 60 s of golden (b) at 50k rps in under 1 s, release.
#[test]
#[ignore]
#[allow(clippy::disallowed_types)]
fn bench_60s_at_50k_rps() {
    let (d, mut s) = golden_b(50_000.0);
    s.duration_s = 60.0;
    let t = std::time::Instant::now();
    let r = simulate(&d, &registry(), &s, 1, &SimOptions::default()).unwrap();
    let el = t.elapsed();
    println!(
        "60 s at 50k rps: {el:?} (sample factor {:.3}, {} requests)",
        r.sample_factor, r.requests
    );
    assert!(el.as_secs_f64() < 1.0, "{el:?}");
}

#[test]
fn poison_messages_fail_every_attempt_and_need_a_dead_letter_queue() {
    let (mut d, mut s) = golden_c();
    s.request_types[0].poison_share = 0.05;
    // A queue gives up after its retries: without a DLQ, poison messages are lost.
    d.nodes[2] = node_with("stream", K::Queue, Flavour::Sqs, &[]);
    let lost = run(&d, &s, 1);
    assert!(
        lost.lost > 0.0 && lost.dead_lettered == 0.0,
        "{}",
        lost.lost
    );
    d.nodes
        .push(node_with("dlq", K::Dlq, Flavour::Generic, &[]));
    d.edges.push(edge("stream", "dlq", Protocol::Publish));
    let kept = run(&d, &s, 1);
    assert!(kept.dead_lettered > 0.0 && kept.lost == 0.0);
    // Good messages are unaffected.
    assert!(kept.dead_lettered < 0.08 * kept.requests);
}

#[test]
fn a_limiter_that_fails_open_lets_traffic_through_while_it_is_down() {
    let make = |on_failure: &str| {
        let mut rl = node_with(
            "rl",
            K::Ratelimit,
            Flavour::RedisTokenBucket,
            &[("limit_rps", 10_000.0)],
        );
        rl.props.insert(
            "on_failure".into(),
            ka_core::PropValue::Text(on_failure.into()),
        );
        let d = design(
            vec![
                node_with("client", K::Client, Flavour::Web, &[]),
                rl,
                node_with("app", K::App, Flavour::Go, &[]),
            ],
            vec![
                edge("client", "rl", Protocol::Http),
                edge("rl", "app", Protocol::Http),
            ],
        );
        let mut s = scenario(200.0, 30.0, vec![rtype("api", 1.0, &["edge", "compute"])]);
        s.chaos = vec![
            serde_json::from_value(
                serde_json::json!({ "kill": { "id": "rl", "at": 10, "for": 10 } }),
            )
            .unwrap(),
        ];
        (d, s)
    };
    let (d, s) = make("closed");
    let closed = run(&d, &s, 1);
    let (d, s) = make("open");
    let open = run(&d, &s, 1);
    assert!(closed.error_rate > 0.25, "{}", closed.error_rate);
    assert!(open.error_rate < 0.01, "{}", open.error_rate);
}

#[test]
fn a_priority_stream_dispatches_live_messages_before_bulk_ones() {
    let make = |priority: bool| {
        let (mut d, mut s) = golden_c();
        if priority {
            d.nodes[2]
                .props
                .insert("priority".into(), ka_core::PropValue::Bool(true));
        }
        // A backfill as heavy as the live traffic: together they outrun the workers.
        let live = s.request_types[0].clone();
        let mut backfill = live.clone();
        backfill.id = "backfill".into();
        backfill.bulk = true;
        s.request_types = vec![live, backfill];
        s.request_types[0].share = 0.5;
        s.request_types[1].share = 0.5;
        s.peak_rps *= 1.6;
        (d, s)
    };
    let p95 = |r: &RunResult, id: &str| {
        r.per_type
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.completion)
            .map(|l| l.p95_ms)
            .unwrap_or(0.0)
    };
    let (d, s) = make(false);
    let fifo = run(&d, &s, 1);
    let (d, s) = make(true);
    let prio = run(&d, &s, 1);
    let live_id = s.request_types[0].id.clone();
    assert!(
        p95(&prio, &live_id) < p95(&fifo, &live_id) / 3.0,
        "{} vs {}",
        p95(&prio, &live_id),
        p95(&fifo, &live_id)
    );
}
