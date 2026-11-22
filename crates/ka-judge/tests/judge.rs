//! The judge: verdicts, score, bottleneck, redaction, determinism (ADR 0007).

use ka_core::problem::ProblemDef;
use ka_core::scenario::PassCriteria;
use ka_core::{
    ComponentKind as K, Design, Edge, Flavour, Node, Point, PropValue, Protocol, registry,
};
use ka_judge::{BottleneckReason, Score, TestResult, Verdict, judge, score};

const PROBLEM: &str = r#"
id = "demo"
number = 1
set = 1
difficulty = "easy"
title = "Demo"
title_es = "Demo"
teaches = "Scaling"
teaches_es = "Escalar"

[nfr]
peak_rps = 2000
p95_ms = 100
availability = 0.999
budget_usd = 2000

[[request_types]]
id = "read"
share = 1.0
route = ["compute", "datastore"]

[[tests]]
id = "normal"
name = "Normal day"
name_es = "Un día normal"
duration_s = 15

[[tests]]
id = "spike"
name = "Spike"
name_es = "Pico"
hidden = true
duration_s = 15
traffic = { spike = { at = 3, x = 4, for = 8 } }

[pass]
error_rate = 0.001
"#;

fn problem(edit: impl Fn(&str) -> String) -> ProblemDef {
    toml::from_str(&edit(PROBLEM)).unwrap()
}

fn node(id: &str, kind: K, flavour: Flavour, props: &[(&str, f64)]) -> Node {
    Node {
        id: id.into(),
        kind,
        flavour,
        label: String::new(),
        position: Point { x: 0.0, y: 0.0 },
        props: props
            .iter()
            .map(|(k, v)| ((*k).to_owned(), PropValue::Number(*v)))
            .collect(),
        group_id: None,
    }
}

/// Client → app → PostgreSQL. `threads` per instance × `instances`.
fn design(instances: f64, threads: f64) -> Design {
    let mut d = Design::empty("d", "D");
    d.nodes = vec![
        node("client", K::Client, Flavour::Web, &[]),
        node(
            "app",
            K::App,
            Flavour::Go,
            &[("instances", instances), ("concurrency", threads)],
        ),
        node("db", K::Sql, Flavour::Postgresql, &[]),
    ];
    let e = |id: &str, from: &str, to: &str, protocol| Edge {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        protocol,
    };
    d.edges = vec![
        e("e1", "client", "app", Protocol::Http),
        e("e2", "app", "db", Protocol::Query),
    ];
    d
}

#[test]
fn a_sized_design_is_accepted_on_every_test_and_scores_full_marks_against_itself() {
    let p = problem(str::to_owned);
    let d = design(4.0, 64.0);
    let first = judge(&p, &d, &registry(), 1.0);
    // Scored against its own cost: full cost points.
    let j = judge(&p, &d, &registry(), first.cost_month);
    assert_eq!(j.verdict, Verdict::Accepted, "{j:#?}");
    assert!(j.tests.iter().all(|t| t.verdict == Verdict::Accepted));
    assert_eq!(j.score.correctness, 70);
    assert_eq!(j.score.cost, 20);
    assert!(j.score.latency >= 8, "{:?}", j.score);
    assert!(
        j.tests
            .iter()
            .all(|t| t.bottleneck.is_none() && t.replay_at_s.is_none())
    );
}

#[test]
fn an_undersized_design_drops_requests_in_the_spike_and_names_its_bottleneck() {
    let p = problem(str::to_owned);
    // 1 × 32 threads (~11 ms a request, ~2,800 req/s): fine at 2,000, overwhelmed at 8,000.
    let j = judge(&p, &design(1.0, 32.0), &registry(), 100.0);
    let normal = &j.tests[0];
    let spike = &j.tests[1];
    assert_eq!(normal.verdict, Verdict::Accepted, "{normal:#?}");
    assert_eq!(spike.verdict, Verdict::DroppedRequests, "{spike:#?}");
    assert_eq!(j.verdict, Verdict::DroppedRequests);
    let b = spike.bottleneck.as_ref().expect("a bottleneck");
    assert_eq!(b.node, "app");
    assert!(matches!(
        b.reason,
        BottleneckReason::QueueOverflow | BottleneckReason::Saturated
    ));
    assert!(b.from_s >= 3, "{b:?}");
    assert_eq!(spike.replay_at_s, Some(f64::from(b.from_s) - 2.0));
    assert_eq!(j.score.correctness, 35);
}

#[test]
fn a_tight_latency_target_is_an_slo_breach_and_a_small_budget_is_over_budget() {
    let slo = problem(|s| s.replace("p95_ms = 100", "p95_ms = 2"));
    let j = judge(&slo, &design(4.0, 64.0), &registry(), 100.0);
    assert!(
        j.tests.iter().all(|t| t.verdict == Verdict::SloBreach),
        "{j:#?}"
    );
    assert_eq!(j.score.total, 0);

    let poor = problem(|s| s.replace("budget_usd = 2000", "budget_usd = 10"));
    let j = judge(&poor, &design(4.0, 64.0), &registry(), 100.0);
    assert!(
        j.tests.iter().all(|t| t.verdict == Verdict::OverBudget),
        "{j:#?}"
    );
}

#[test]
fn a_design_without_a_path_is_invalid_with_a_reason() {
    let p = problem(str::to_owned);
    let mut d = design(4.0, 64.0);
    d.edges.pop();
    let j = judge(&p, &d, &registry(), 100.0);
    assert_eq!(j.verdict, Verdict::InvalidDesign);
    assert!(j.lint[0].contains("no path"), "{:?}", j.lint);
    assert!(j.tests.iter().all(|t| t.verdict == Verdict::InvalidDesign));
    assert_eq!(
        j.score,
        Score {
            correctness: 0,
            cost: 0,
            latency: 0,
            total: 0
        }
    );
    let req = problem(|s| {
        s.replace(
            "teaches_es = \"Escalar\"",
            "teaches_es = \"Escalar\"\nrequires = [\"metrics\"]",
        )
    });
    let j = judge(&req, &design(4.0, 64.0), &registry(), 100.0);
    assert_eq!(j.verdict, Verdict::InvalidDesign);
    assert!(j.lint[0].contains("metrics"), "{:?}", j.lint);
}

fn result(verdict: Verdict, p95: f64, target: f64) -> TestResult {
    TestResult {
        id: "t".into(),
        name: "t".into(),
        name_es: "t".into(),
        hidden: false,
        verdict,
        p95_ms: p95,
        error_rate: 0.0,
        availability: 1.0,
        stale_rate: 0.0,
        lost: 0.0,
        throughput_rps: 0.0,
        cost_month: 0.0,
        limits: PassCriteria {
            p95_ms: Some(target),
            ..PassCriteria::default()
        },
        bottleneck: None,
        replay_at_s: None,
        seed: 0,
    }
}

#[test]
fn the_score_matches_the_mockup_arithmetic() {
    // 3 of 4 tests: 52.5 rounds half to even, 52 / 70 (the mockup).
    let a = |p95| result(Verdict::Accepted, p95, 80.0);
    let tests = [
        a(20.0),
        a(40.0),
        a(60.0),
        result(Verdict::SloBreach, 200.0, 80.0),
    ];
    // Cost: ref 1,500 vs 2,000 → 0.75 × 20 × 3/4 = 11.25 → 11.
    // Latency over the accepted: headroom 0.75, 0.5, 0.25 → mean 0.5 → 10 × min(1, 1) = 10.
    let s = score(&tests, 2000.0, 1500.0);
    assert_eq!(
        s,
        Score {
            correctness: 52,
            cost: 11,
            latency: 10,
            total: 73
        }
    );
    // Cheaper than the reference never earns more than 20.
    assert_eq!(score(&[a(40.0)], 500.0, 1500.0).cost, 20);
    // Nothing accepted: no cost or latency points either (review A3).
    let none = score(
        &[result(Verdict::DroppedRequests, 10.0, 80.0)],
        100.0,
        1500.0,
    );
    assert_eq!(none.total, 0);
}

#[test]
fn hidden_tests_report_results_but_never_their_definition() {
    let p = problem(str::to_owned);
    let j = judge(&p, &design(1.0, 32.0), &registry(), 100.0);
    let json = serde_json::to_string(&j).unwrap();
    assert!(json.contains("\"hidden\":true"));
    for secret in ["traffic", "chaos", "spike\":{", "type_multipliers"] {
        assert!(!json.contains(secret), "judgement leaks {secret}");
    }
}

#[test]
fn judging_is_deterministic() {
    let p = problem(str::to_owned);
    let a = judge(&p, &design(1.0, 32.0), &registry(), 100.0);
    let b = judge(&p, &design(1.0, 32.0), &registry(), 100.0);
    assert_eq!(a, b);
    assert_ne!(
        a.tests[0].seed, a.tests[1].seed,
        "each test has its own seed"
    );
}
