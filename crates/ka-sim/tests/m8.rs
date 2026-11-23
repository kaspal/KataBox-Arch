//! M8 simulator features (ADR 0006, "M8 additions"): one golden per feature plus behaviour
//! tests. Every feature is opt-in, so goldens a–d (tests/sim.rs) stay byte-identical.

mod common;

use std::path::PathBuf;

use common::*;
use ka_core::scenario::{ChaosEvent, Scenario};
use ka_core::{ComponentKind as K, Design, Flavour, PropValue, Protocol, registry};
use ka_sim::{RunResult, SimOptions, simulate};

fn run(d: &Design, s: &Scenario, seed: u64) -> RunResult {
    simulate(d, &registry(), s, seed, &SimOptions::default()).expect("the design runs")
}

/// Compare with `tests/golden/<name>.json`; `KA_UPDATE_GOLDENS=1` rewrites it.
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

fn chaos(v: serde_json::Value) -> ChaosEvent {
    serde_json::from_value(v).expect("a chaos event")
}

fn served(r: &RunResult, id: &str) -> f64 {
    r.nodes.iter().find(|n| n.id == id).expect("node").served
}

// --- S8: client retries, timeouts, retry budget ---------------------------------------------

/// client → app (Go ×2) → PostgreSQL; the database slows down by `add_ms` from 10 s to 30 s.
fn slow_dependency(client: &[(&str, f64)], jitter: bool, add_ms: f64) -> (Design, Scenario) {
    let mut c = node("client", K::Client, Flavour::Web, client);
    if jitter {
        c.props
            .insert("jitter".into(), PropValue::Text("full".into()));
    }
    let d = design(
        vec![
            c,
            node("app", K::App, Flavour::Go, &[("instances", 2.0)]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "db", Protocol::Query),
        ],
    );
    let mut s = scenario(
        200.0,
        40.0,
        vec![rtype("read", 1.0, &["compute", "datastore"])],
    );
    s.chaos = vec![chaos(serde_json::json!({
        "latency": { "id": "db", "add_ms": add_ms, "at": 10, "for": 20 }
    }))];
    (d, s)
}

/// A timeout shorter than the slow database's answers: every request in the window fails,
/// and with retries the database sees the same request again and again (amplification).
#[test]
fn s8_retries_amplify_load_on_a_slow_dependency() {
    let short = [("timeout_ms", 300.0)];
    let (d0, s0) = slow_dependency(&short, false, 400.0);
    let none = run(&d0, &s0, 7);
    let (d1, s1) = slow_dependency(
        &[("timeout_ms", 300.0), ("retries", 3.0), ("backoff_ms", 0.0)],
        false,
        400.0,
    );
    let storm = run(&d1, &s1, 7);
    assert!(none.errors > 0.0, "the slow window times out");
    assert_eq!(none.retried, 0.0, "no retries by default");
    assert!(storm.retried > 0.0);
    let db_queue = |r: &RunResult| r.nodes.iter().find(|n| n.id == "db").unwrap().max_queue;
    assert!(
        db_queue(&storm) > db_queue(&none) + 10.0,
        "the retried load queues at the database: {} vs {}",
        db_queue(&storm),
        db_queue(&none)
    );
    assert!(
        served(&storm, "db") > served(&none, "db") * 1.1,
        "retries multiply the load on the slow database: {} vs {}",
        served(&storm, "db"),
        served(&none, "db")
    );
    // The user-visible request count doesn't change: retries are tries, not requests.
    assert!((storm.requests - none.requests).abs() / none.requests < 0.05);
}

/// A retry budget caps retries at a share of first tries.
#[test]
fn s8_a_retry_budget_caps_retries() {
    let free = [("timeout_ms", 300.0), ("retries", 3.0), ("backoff_ms", 0.0)];
    let budget = [
        ("timeout_ms", 300.0),
        ("retries", 3.0),
        ("backoff_ms", 0.0),
        ("retry_budget_pct", 10.0),
    ];
    let (d0, s0) = slow_dependency(&free, false, 400.0);
    let (d1, s1) = slow_dependency(&budget, false, 400.0);
    let (a, b) = (run(&d0, &s0, 3), run(&d1, &s1, 3));
    assert!(b.retried > 0.0);
    assert!(
        b.retried <= 0.12 * b.requests,
        "retries stay near 10% of first tries: {} of {}",
        b.retried,
        b.requests
    );
    assert!(b.retried < a.retried / 3.0);
}

/// Backoff spreads retries out: with a long enough backoff, a short slowdown is waited out
/// instead of failing, and full jitter is deterministic per seed.
#[test]
fn s8_backoff_with_jitter_waits_out_a_short_slowdown() {
    let policy = [
        ("timeout_ms", 300.0),
        ("retries", 4.0),
        ("backoff_ms", 2000.0),
    ];
    let (d, s) = slow_dependency(&policy, true, 400.0);
    let a = run(&d, &s, 11);
    let b = run(&d, &s, 11);
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap(),
        "deterministic per seed"
    );
    let (d0, s0) = slow_dependency(&[("timeout_ms", 300.0)], false, 400.0);
    let none = run(&d0, &s0, 11);
    assert!(a.errors < none.errors, "{} vs {}", a.errors, none.errors);
}

/// The client timeout is a prop; its default is the 5 s the simulator always used.
#[test]
fn s8_the_client_timeout_is_a_prop() {
    let (d, s) = slow_dependency(&[], false, 400.0);
    let default = run(&d, &s, 5);
    assert_eq!(default.errors, 0.0, "a 400 ms slowdown is well under 5 s");
    let (d, s) = slow_dependency(&[("timeout_ms", 300.0)], false, 400.0);
    assert!(run(&d, &s, 5).errors > 0.0);
}

/// Golden l: a retry storm (three instant retries on a 300 ms timeout) against a database
/// that slows down for 20 s.
#[test]
fn golden_l_retry_storm() {
    let (d, s) = slow_dependency(
        &[("timeout_ms", 300.0), ("retries", 3.0), ("backoff_ms", 0.0)],
        false,
        400.0,
    );
    golden("l", &run(&d, &s, 42));
}

// --- S8: service calls, call timeouts, fallbacks, circuit breakers, bulkheads, shedding ------

/// client → app (Go ×2, 64 slots each) → recs (a leaf service) and PostgreSQL. `checkout`
/// calls recs then the database; `browse` reads the database only. Recs slows down by
/// `add_ms` from 10 s to 30 s.
fn with_recs(
    app: &[(&str, f64)],
    bulkhead: bool,
    optional: bool,
    add_ms: f64,
) -> (Design, Scenario) {
    let mut a = node("app", K::App, Flavour::Go, app);
    if bulkhead {
        a.props.insert("bulkhead".into(), PropValue::Bool(true));
    }
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            a,
            node("recs", K::Micro, Flavour::Generic, &[("instances", 4.0)]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "recs", Protocol::Grpc),
            edge("app", "db", Protocol::Query),
        ],
    );
    let recs = if optional { "service?" } else { "service" };
    let mut s = scenario(
        600.0,
        40.0,
        vec![
            rtype("checkout", 0.5, &["compute", recs, "datastore"]),
            rtype("browse", 0.5, &["compute", "datastore"]),
        ],
    );
    s.chaos = vec![chaos(serde_json::json!({
        "latency": { "id": "recs", "add_ms": add_ms, "at": 10, "for": 20 }
    }))];
    (d, s)
}

fn type_errors(r: &RunResult, id: &str) -> f64 {
    r.per_type.iter().find(|t| t.id == id).expect("type").errors
}

/// A slow dependency holds every app slot (thread per request), so even requests that never
/// call it fail. A bulkhead gives each dependency its own share of slots.
#[test]
fn s8_a_bulkhead_keeps_a_slow_dependency_from_taking_everything_down() {
    let (d, s) = with_recs(&[("instances", 2.0)], false, false, 3000.0);
    let shared = run(&d, &s, 21);
    assert!(
        type_errors(&shared, "browse") > 0.0,
        "browse starves behind recs"
    );
    let (d, s) = with_recs(&[("instances", 2.0)], true, false, 3000.0);
    let isolated = run(&d, &s, 21);
    assert!(
        type_errors(&isolated, "browse") < type_errors(&shared, "browse") / 10.0,
        "browse keeps its own slots: {} vs {}",
        type_errors(&isolated, "browse"),
        type_errors(&shared, "browse")
    );
}

/// A call timeout plus an optional dependency (`service?`) is a fallback: the checkout goes
/// on without recommendations instead of failing.
#[test]
fn s8_a_call_timeout_with_a_fallback_answers_without_the_dependency() {
    let (d, s) = with_recs(&[("instances", 2.0)], false, false, 3000.0);
    let none = run(&d, &s, 22);
    let (d, s) = with_recs(
        &[("instances", 2.0), ("call_timeout_ms", 200.0)],
        false,
        true,
        3000.0,
    );
    let fallback = run(&d, &s, 22);
    assert!(type_errors(&none, "checkout") > 0.0);
    assert_eq!(type_errors(&fallback, "checkout"), 0.0);
    assert_eq!(type_errors(&fallback, "browse"), 0.0);
    // Without `?` a timed-out call fails the request.
    let (d, s) = with_recs(
        &[("instances", 2.0), ("call_timeout_ms", 200.0)],
        false,
        false,
        3000.0,
    );
    assert!(type_errors(&run(&d, &s, 22), "checkout") > 0.0);
}

/// An open breaker fails calls at once instead of sending them: the dead dependency sees far
/// fewer calls while it is down.
#[test]
fn s8_a_circuit_breaker_stops_calling_a_failing_dependency() {
    let make = |app: &[(&str, f64)]| {
        let (d, mut s) = with_recs(app, false, true, 0.0);
        s.chaos = vec![chaos(serde_json::json!({
            "kill": { "id": "recs", "at": 10, "for": 20 }
        }))];
        (d, s)
    };
    let (d, s) = make(&[("instances", 2.0), ("call_timeout_ms", 200.0)]);
    let plain = run(&d, &s, 23);
    let (d, s) = make(&[
        ("instances", 2.0),
        ("call_timeout_ms", 200.0),
        ("breaker_errors_pct", 50.0),
        ("breaker_open_s", 5.0),
    ]);
    let breaker = run(&d, &s, 23);
    assert_eq!(
        type_errors(&breaker, "checkout"),
        0.0,
        "the fallback answers"
    );
    // While open, checkouts skip recs at once: their latency drops below the plain run's.
    let p95 = |r: &RunResult| {
        r.per_type
            .iter()
            .find(|t| t.id == "checkout")
            .unwrap()
            .latency
            .p95_ms
    };
    assert!(
        p95(&breaker) <= p95(&plain),
        "{} vs {}",
        p95(&breaker),
        p95(&plain)
    );
}

/// Load shedding turns requests away once a queue passes its limit, so the ones accepted are
/// served quickly instead of everyone waiting toward a timeout.
#[test]
fn s8_load_shedding_keeps_accepted_requests_fast() {
    let make = |shed: f64| {
        let d = design(
            vec![
                node("client", K::Client, Flavour::Web, &[]),
                node(
                    "app",
                    K::App,
                    Flavour::Go,
                    &[
                        ("instances", 1.0),
                        ("concurrency", 8.0),
                        ("shed_queue", shed),
                    ],
                ),
            ],
            vec![edge("client", "app", Protocol::Http)],
        );
        let mut s = scenario(1000.0, 30.0, vec![rtype("hit", 1.0, &["compute"])]);
        s.traffic = ka_core::scenario::Curve::Spike {
            at: 10.0,
            x: 3.0,
            for_s: 10.0,
        };
        (d, s)
    };
    let (d, s) = make(0.0);
    let queued = run(&d, &s, 24);
    let (d, s) = make(16.0);
    let shed = run(&d, &s, 24);
    assert!(shed.errors > 0.0, "the excess is turned away");
    assert!(
        shed.response.p95_ms < queued.response.p95_ms,
        "{} vs {}",
        shed.response.p95_ms,
        queued.response.p95_ms
    );
}

/// Golden l2: a slow dependency behind a bulkhead, a call timeout and a breaker, with a
/// fallback.
#[test]
fn golden_l2_resilience() {
    let (d, s) = with_recs(
        &[
            ("instances", 2.0),
            ("call_timeout_ms", 200.0),
            ("breaker_errors_pct", 50.0),
            ("breaker_open_s", 5.0),
        ],
        true,
        true,
        3000.0,
    );
    golden("l2", &run(&d, &s, 42));
}
