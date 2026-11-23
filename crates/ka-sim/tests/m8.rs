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

// --- S8: deploys, config pushes, control plane, DNS, health-check ejection -------------------

fn region(id: &str) -> ka_core::Group {
    ka_core::Group {
        id: id.into(),
        kind: ka_core::GroupKind::Region,
        label: id.into(),
        position: ka_core::Point { x: 0.0, y: 0.0 },
        size: ka_core::Size { w: 100.0, h: 100.0 },
        parent_id: None,
        scaling: None,
        region_tag: None,
        mesh: None,
    }
}

fn in_group(mut n: ka_core::Node, g: &str) -> ka_core::Node {
    n.group_id = Some(g.into());
    n
}

fn set(n: &mut ka_core::Node, k: &str, v: PropValue) {
    n.props.insert(k.into(), v);
}

fn simple_app(app: Vec<(&str, PropValue)>) -> (Design, Scenario) {
    let mut a = node("app", K::App, Flavour::Go, &[("instances", 4.0)]);
    for (k, v) in app {
        set(&mut a, k, v);
    }
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            a,
            node("db", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "db", Protocol::Query),
        ],
    );
    let mut s = scenario(
        400.0,
        60.0,
        vec![rtype("read", 1.0, &["compute", "datastore"])],
    );
    s.chaos = vec![chaos(serde_json::json!({
        "deploy": { "id": "app", "at": 10, "for": 40, "error_rate": 0.5 }
    }))];
    (d, s)
}

/// A bad build everywhere at once fails half the traffic until it's rolled back by hand; a
/// staged rollout with automatic rollback limits it to the first stage, briefly.
#[test]
fn s8_a_staged_rollout_with_auto_rollback_limits_a_bad_deploy() {
    let (d, s) = simple_app(vec![]);
    let big_bang = run(&d, &s, 31);
    let (d, s) = simple_app(vec![
        ("rollout_percent", PropValue::Number(10.0)),
        ("bake_s", PropValue::Number(60.0)),
        ("auto_rollback", PropValue::Bool(true)),
    ]);
    let staged = run(&d, &s, 31);
    assert!(big_bang.error_rate > 0.2, "{}", big_bang.error_rate);
    assert!(staged.error_rate < 0.002, "{}", staged.error_rate);
    assert!(staged.errors > 0.0, "the canary still hits a few");
}

/// Two Regions, each with its app; a bad config push reaches both at once, unless a staged
/// Config card sends it one Region at a time and its kill switch rolls it back.
fn two_regions(config: Option<&[(&str, bool)]>) -> (Design, Scenario) {
    let mut nodes = vec![
        node("client", K::Client, Flavour::Web, &[]),
        node("lb", K::Lb, Flavour::AwsAlb, &[]),
        in_group(node("app-a", K::App, Flavour::Go, &[]), "r-a"),
        in_group(node("app-b", K::App, Flavour::Go, &[]), "r-b"),
    ];
    if let Some(props) = config {
        let mut c = node(
            "flags",
            K::Config,
            Flavour::Launchdarkly,
            &[("bake_s", 30.0)],
        );
        for (k, v) in props {
            set(&mut c, k, PropValue::Bool(*v));
        }
        nodes.push(c);
    }
    let mut d = design(
        nodes,
        vec![
            edge("client", "lb", Protocol::Http),
            edge("lb", "app-a", Protocol::Http),
            edge("lb", "app-b", Protocol::Http),
        ],
    );
    d.groups = vec![region("r-a"), region("r-b")];
    let mut s = scenario(400.0, 60.0, vec![rtype("hit", 1.0, &["edge", "compute"])]);
    s.chaos = vec![chaos(serde_json::json!({
        "config_push": { "at": 10, "for": 30, "error_rate": 1.0 }
    }))];
    (d, s)
}

#[test]
fn s8_a_staged_config_push_with_a_kill_switch_limits_the_blast_radius() {
    let (d, s) = two_regions(None);
    let global = run(&d, &s, 32);
    let (d, s) = two_regions(Some(&[("staged", true)]));
    let staged = run(&d, &s, 32);
    let (d, s) = two_regions(Some(&[("staged", true), ("kill_switch", true)]));
    let both = run(&d, &s, 32);
    assert!(global.error_rate > 0.4, "{}", global.error_rate);
    assert!(
        staged.error_rate < global.error_rate * 0.7,
        "one Region only: {}",
        staged.error_rate
    );
    assert!(
        both.error_rate < 0.05,
        "rolled back in seconds: {}",
        both.error_rate
    );
}

/// With the control plane down, autoscaling can't add instances, so a ramp that it would
/// normally absorb overloads the tier (static stability is the fix).
#[test]
fn s8_no_scaling_while_the_control_plane_is_down() {
    let make = |cp: bool| {
        let mut d = design(
            vec![
                node("client", K::Client, Flavour::Web, &[]),
                in_group(
                    node(
                        "app",
                        K::App,
                        Flavour::Go,
                        &[("instances", 2.0), ("concurrency", 8.0)],
                    ),
                    "asg",
                ),
            ],
            vec![edge("client", "app", Protocol::Http)],
        );
        let mut g = region("asg");
        g.kind = ka_core::GroupKind::Autoscale;
        g.scaling = Some(ka_core::Scaling {
            flavour: Flavour::KubernetesHpa,
            props: [
                ("max_instances".to_owned(), PropValue::Number(20.0)),
                ("scale_delay_s".to_owned(), PropValue::Number(10.0)),
            ]
            .into_iter()
            .collect(),
        });
        d.groups = vec![g];
        let mut s = scenario(2000.0, 90.0, vec![rtype("hit", 1.0, &["compute"])]);
        s.traffic = ka_core::scenario::Curve::Ramp { from: 0.5, to: 3.0 };
        if cp {
            s.chaos = vec![chaos(serde_json::json!({ "control_plane": { "at": 5 } }))];
        }
        (d, s)
    };
    let (d, s) = make(false);
    let scales = run(&d, &s, 33);
    let (d, s) = make(true);
    let stuck = run(&d, &s, 33);
    assert!(
        stuck.errors > scales.errors * 5.0 + 100.0,
        "{} vs {}",
        stuck.errors,
        scales.errors
    );
}

/// Failover DNS with a long TTL keeps clients on a dead Region; a short TTL moves them.
fn dns_failover(ttl: f64) -> (Design, Scenario) {
    let mut d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            node("dns", K::Dns, Flavour::Route53, &[("ttl_s", ttl)]),
            in_group(node("app-a", K::App, Flavour::Go, &[]), "r-a"),
            in_group(node("app-b", K::App, Flavour::Go, &[]), "r-b"),
        ],
        vec![
            edge("client", "dns", Protocol::Http),
            edge("dns", "app-a", Protocol::Http),
            edge("dns", "app-b", Protocol::Http),
        ],
    );
    set(
        &mut d.nodes[1],
        "routing",
        PropValue::Text("failover".into()),
    );
    d.groups = vec![region("r-a"), region("r-b")];
    let mut s = scenario(300.0, 90.0, vec![rtype("hit", 1.0, &["edge", "compute"])]);
    s.chaos = vec![chaos(
        serde_json::json!({ "partition": { "zone": "r-a", "at": 10 } }),
    )];
    (d, s)
}

#[test]
fn s8_dns_ttl_decides_how_long_clients_stay_on_a_dead_region() {
    let (d, s) = dns_failover(86_400.0);
    let long = run(&d, &s, 34);
    let (d, s) = dns_failover(20.0);
    let short = run(&d, &s, 34);
    let last = |r: &RunResult| {
        r.series
            .iter()
            .rev()
            .take(20)
            .map(|x| x.errors)
            .sum::<f64>()
    };
    assert!(
        last(&long) > 1000.0,
        "still on the dead Region: {}",
        last(&long)
    );
    assert_eq!(last(&short), 0.0, "moved within the TTL");
    // Before the partition, everything goes to the first Region.
    let served = |r: &RunResult, id: &str| r.nodes.iter().find(|n| n.id == id).unwrap().served;
    assert!(served(&short, "app-a") > 0.0 && served(&short, "app-b") > 0.0);
}

/// A DNS outage only hurts clients whose cached answer expired.
#[test]
fn s8_a_dns_outage_spares_cached_answers() {
    let make = |ttl: f64| {
        let d = design(
            vec![
                node("client", K::Client, Flavour::Web, &[]),
                node("dns", K::Dns, Flavour::Route53, &[("ttl_s", ttl)]),
                node("app", K::App, Flavour::Go, &[]),
            ],
            vec![
                edge("client", "dns", Protocol::Http),
                edge("dns", "app", Protocol::Http),
            ],
        );
        let mut s = scenario(300.0, 60.0, vec![rtype("hit", 1.0, &["edge", "compute"])]);
        s.chaos = vec![chaos(
            serde_json::json!({ "dns_outage": { "at": 10, "for": 20 } }),
        )];
        (d, s)
    };
    let (d, s) = make(0.0);
    let none = run(&d, &s, 35);
    let (d, s) = make(300.0);
    let cached = run(&d, &s, 35);
    assert!(none.error_rate > 0.3, "{}", none.error_rate);
    assert!(
        cached.error_rate < none.error_rate / 5.0,
        "{}",
        cached.error_rate
    );
}

/// Aggressive health checks eject overloaded targets, which overloads the rest (a spiral);
/// a minimum-healthy floor keeps capacity in rotation.
fn spiral(min_healthy: f64) -> (Design, Scenario) {
    let app = |id: &str| {
        node(
            id,
            K::App,
            Flavour::Go,
            &[("instances", 1.0), ("concurrency", 16.0)],
        )
    };
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            node(
                "lb",
                K::Lb,
                Flavour::AwsAlb,
                &[
                    ("health_timeout_ms", 20.0),
                    ("min_healthy_pct", min_healthy),
                ],
            ),
            app("a1"),
            app("a2"),
            app("a3"),
        ],
        vec![
            edge("client", "lb", Protocol::Http),
            edge("lb", "a1", Protocol::Http),
            edge("lb", "a2", Protocol::Http),
            edge("lb", "a3", Protocol::Http),
        ],
    );
    let mut s = scenario(7000.0, 60.0, vec![rtype("hit", 1.0, &["edge", "compute"])]);
    s.traffic = ka_core::scenario::Curve::Spike {
        at: 10.0,
        x: 1.6,
        for_s: 20.0,
    };
    (d, s)
}

#[test]
fn s8_health_check_ejection_spirals_without_a_floor() {
    let (d, s) = spiral(0.0);
    let no_floor = run(&d, &s, 36);
    let (d, s) = spiral(100.0);
    let floor = run(&d, &s, 36);
    assert!(
        no_floor.errors > floor.errors,
        "{} vs {}",
        no_floor.errors,
        floor.errors
    );
}

/// Golden l3: a bad deploy behind a staged rollout with auto-rollback.
#[test]
fn golden_l3_staged_rollout() {
    let (d, s) = simple_app(vec![
        ("rollout_percent", PropValue::Number(25.0)),
        ("bake_s", PropValue::Number(20.0)),
        ("auto_rollback", PropValue::Bool(true)),
    ]);
    golden("l3", &run(&d, &s, 42));
}

// --- S7: quorum, fencing, races ----------------------------------------------------------------

/// client → app → PostgreSQL; `book` checks then writes one hot key half the time.
fn booking(route: &[&str], cache_lock: bool) -> (Design, Scenario) {
    let mut nodes = vec![
        node("client", K::Client, Flavour::Web, &[]),
        node("app", K::App, Flavour::Go, &[("instances", 2.0)]),
        node("db", K::Sql, Flavour::Postgresql, &[]),
    ];
    let mut edges = vec![
        edge("client", "app", Protocol::Http),
        edge("app", "db", Protocol::Query),
    ];
    if cache_lock {
        nodes.push(node("locks", K::Cache, Flavour::Redis, &[]));
        edges.push(edge("app", "locks", Protocol::Query));
    }
    let mut t = rtype("book", 1.0, route);
    t.contended = Some(ka_core::scenario::Contended {
        keys: 1,
        share: 0.5,
    });
    (design(nodes, edges), scenario(300.0, 30.0, vec![t]))
}

#[test]
fn s7_check_then_write_races_unless_atomic_or_locked() {
    let (d, s) = booking(&["compute", "datastore", "datastore:write"], false);
    let racy = run(&d, &s, 41);
    assert!(racy.conflicts > 0.0, "two guests book the last room");
    let (d, s) = booking(&["compute", "datastore", "datastore:write:atomic"], false);
    assert_eq!(
        run(&d, &s, 41).conflicts,
        0.0,
        "a conditional write refuses the loser"
    );
    let (d, s) = booking(&["compute", "lock", "datastore", "datastore:write"], true);
    assert_eq!(run(&d, &s, 41).conflicts, 0.0, "the lock serialises them");
}

fn az(id: &str) -> ka_core::Group {
    let mut g = region(id);
    g.kind = ka_core::GroupKind::Az;
    g
}

/// Two AZs, an app in each, a PostgreSQL primary in a and its replica in b, with
/// `split_failover = auto`; AZ b is split off from 10 s to 30 s. `lock` is a Redis cache in
/// b (no quorum) or an etcd cluster in a (quorum).
fn split_brain(lock: Option<K>, fencing: bool) -> (Design, Scenario) {
    let mut db_a = in_group(
        node("db-a", K::Sql, Flavour::Postgresql, &[("failover_s", 5.0)]),
        "az-a",
    );
    let mut db_b = in_group(
        node("db-b", K::Sql, Flavour::Postgresql, &[("failover_s", 5.0)]),
        "az-b",
    );
    for db in [&mut db_a, &mut db_b] {
        set(db, "split_failover", PropValue::Text("auto".into()));
        set(db, "fencing", PropValue::Bool(fencing));
    }
    // Users on both sides of the split, each reaching the app in their own AZ.
    let mut nodes = vec![
        in_group(node("client-a", K::Client, Flavour::Web, &[]), "az-a"),
        in_group(node("client-b", K::Client, Flavour::Web, &[]), "az-b"),
        in_group(node("app-a", K::App, Flavour::Go, &[]), "az-a"),
        in_group(node("app-b", K::App, Flavour::Go, &[]), "az-b"),
        db_a,
        db_b,
    ];
    let mut edges = vec![
        edge("client-a", "app-a", Protocol::Http),
        edge("client-b", "app-b", Protocol::Http),
        edge("app-a", "db-a", Protocol::Query),
        edge("app-b", "db-a", Protocol::Query),
        edge("db-a", "db-b", Protocol::Replicate),
    ];
    let mut route = vec!["compute", "datastore:write"];
    match lock {
        Some(K::Cache) => {
            nodes.push(in_group(
                node("locks", K::Cache, Flavour::Redis, &[]),
                "az-b",
            ));
            edges.push(edge("app-a", "locks", Protocol::Query));
            edges.push(edge("app-b", "locks", Protocol::Query));
            route.insert(1, "lock");
        }
        Some(K::Coord) => {
            nodes.push(in_group(
                node("etcd", K::Coord, Flavour::Etcd, &[("instances", 3.0)]),
                "az-a",
            ));
            edges.push(edge("app-a", "etcd", Protocol::Query));
            edges.push(edge("app-b", "etcd", Protocol::Query));
            route.insert(1, "lock");
        }
        _ => {}
    }
    let mut d = design(nodes, edges);
    d.groups = vec![az("az-a"), az("az-b")];
    let mut s = scenario(300.0, 40.0, vec![rtype("write", 1.0, &route)]);
    s.chaos = vec![chaos(
        serde_json::json!({ "split": { "zone": "az-b", "at": 10, "for": 20 } }),
    )];
    (d, s)
}

#[test]
fn s7_split_brain_conflicts_unless_a_quorum_lock_fences_the_stale_side() {
    let (d, s) = split_brain(None, false);
    let brain = run(&d, &s, 42);
    assert!(brain.conflicts > 0.0, "both sides take writes");
    let (d, s) = split_brain(Some(K::Cache), true);
    assert!(
        run(&d, &s, 42).conflicts > 0.0,
        "a lock without quorum doesn't help"
    );
    let (d, s) = split_brain(Some(K::Coord), true);
    let fenced = run(&d, &s, 42);
    assert_eq!(
        fenced.conflicts, 0.0,
        "the minority side can't take the lock"
    );
    assert!(fenced.errors > 0.0, "its writes fail instead");
    // Without auto failover the old behaviour holds: the cut-off side's writes just fail.
    let (mut d, s) = split_brain(None, false);
    for n in d.nodes.iter_mut().filter(|n| n.kind == K::Sql) {
        n.props.remove("split_failover");
    }
    assert_eq!(run(&d, &s, 42).conflicts, 0.0);
}

/// Golden k: split-brain with a Redis lock (conflicts) and the same with a quorum etcd.
#[test]
fn golden_k_quorum_and_fencing() {
    let (d, s) = split_brain(Some(K::Cache), true);
    golden("k", &run(&d, &s, 42));
    let (d, s) = split_brain(Some(K::Coord), true);
    golden("k2", &run(&d, &s, 42));
}
