//! The judge (ADR 0007): runs a problem's tests on a design and returns verdicts, a score,
//! the bottleneck of each failing test and a point to replay it from. Deterministic: every
//! test has its own seed, `fnv("<problem>/<test>")`, so the same design always scores the
//! same. The AI review (M4) explains a judgement; it never changes one.

use ka_core::problem::{ProblemDef, TestDef};
use ka_core::rng::fnv64;
use ka_core::scenario::PassCriteria;
use ka_core::{ComponentKind, Design, Registry};
use ka_sim::{RunResult, SimError, SimOptions};
use serde::{Deserialize, Serialize};

/// A test's outcome, in the order the checks run: the first that fails is the verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The design can't run the problem (validation, a missing path, a required card).
    InvalidDesign,
    /// Too many errors, or messages lost.
    DroppedRequests,
    /// p95, availability or stale reads outside the limits.
    SloBreach,
    /// Costs more per month than the budget.
    OverBudget,
    Accepted,
}

/// Why the bottleneck card is the bottleneck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum BottleneckReason {
    /// Every server busy and its queue growing.
    Saturated,
    /// Its queue full: it dropped requests.
    QueueOverflow,
    /// It was down (chaos) while requests needed it.
    Down,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Bottleneck {
    pub node: String,
    pub kind: ComponentKind,
    pub from_s: u32,
    pub utilisation: f64,
    pub reason: BottleneckReason,
}

/// The result of one test. Hidden tests carry the same numbers, but never their traffic or
/// chaos: the definition stays in Rust.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TestResult {
    pub id: String,
    pub name: String,
    pub name_es: String,
    pub hidden: bool,
    pub verdict: Verdict,
    pub p95_ms: f64,
    pub error_rate: f64,
    pub availability: f64,
    pub stale_rate: f64,
    pub lost: f64,
    pub throughput_rps: f64,
    pub cost_month: f64,
    /// The limits this test was held to.
    pub limits: PassCriteria,
    pub bottleneck: Option<Bottleneck>,
    /// Replay at failure: start the live run of this test here (two seconds before the
    /// bottleneck appears, or at the start).
    pub replay_at_s: Option<f64>,
    pub seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Score {
    pub correctness: u32,
    pub cost: u32,
    pub latency: u32,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Judgement {
    pub problem_id: String,
    /// Accepted when every test is; otherwise the most severe failing verdict.
    pub verdict: Verdict,
    /// Why the design is invalid (only for Invalid Design).
    pub lint: Vec<String>,
    pub tests: Vec<TestResult>,
    pub score: Score,
    /// The design's monthly cost on the first test, and the reference's.
    pub cost_month: f64,
    pub ref_cost: f64,
}

/// The seed of one test: stable across versions and machines.
pub fn test_seed(problem: &str, test: &str) -> u64 {
    fnv64(format!("{problem}/{test}").as_bytes())
}

/// Problems with the design that make it unjudgeable.
pub fn lint(problem: &ProblemDef, design: &Design, registry: &Registry) -> Vec<String> {
    let mut out: Vec<String> = ka_core::validate(design, registry)
        .iter()
        .map(|i| format!("{i:?}"))
        .collect();
    if out.is_empty() {
        if let Some(t) = problem.tests.first()
            && let Err(e) = ka_sim::compile(
                design,
                registry,
                &problem.scenario(t),
                &SimOptions::default(),
            )
        {
            out.push(e.to_string());
        }
        for k in &problem.requires {
            if !design.nodes.iter().any(|n| n.kind == *k) {
                out.push(format!("the problem requires a {} card", k.id()));
            }
        }
    }
    out
}

fn verdict(r: &RunResult, limits: &PassCriteria, cost: f64, budget: f64) -> Verdict {
    let over = |v: f64, l: Option<f64>| l.is_some_and(|l| v > l);
    if over(r.error_rate, limits.error_rate) || r.lost > limits.lost.unwrap_or(0.0) {
        return Verdict::DroppedRequests;
    }
    if over(r.latency.p95_ms, limits.p95_ms)
        || limits.availability.is_some_and(|a| r.availability < a)
        || over(r.stale_rate, limits.stale_rate)
    {
        return Verdict::SloBreach;
    }
    if cost > budget {
        return Verdict::OverBudget;
    }
    Verdict::Accepted
}

/// The card that gave way first: the earliest second in which a card is down, or at least
/// 95% busy with its queue growing or dropping. Ties go to the busiest, then the lowest id.
pub fn bottleneck(r: &RunResult) -> Option<Bottleneck> {
    let mut best: Option<(u32, f64, &str, Bottleneck)> = None;
    for n in &r.nodes {
        let mut hit = None;
        for s in 0..n.util_series.len() {
            let drops = n.drop_series.get(s).copied().unwrap_or(0.0) > 0.0;
            let growing = s > 0 && n.queue_series.get(s) > n.queue_series.get(s - 1);
            let down = n.down_seconds.contains(&(s as u32));
            let reason = if down {
                Some(BottleneckReason::Down)
            } else if n.util_series[s] >= 0.95 && drops {
                Some(BottleneckReason::QueueOverflow)
            } else if n.util_series[s] >= 0.95 && growing {
                Some(BottleneckReason::Saturated)
            } else {
                None
            };
            if let Some(reason) = reason {
                hit = Some((s as u32, n.util_series[s], reason));
                break;
            }
        }
        let Some((s, util, reason)) = hit else {
            continue;
        };
        let better = match &best {
            None => true,
            Some((bs, bu, bid, _)) => {
                s < *bs || (s == *bs && (util > *bu || (util == *bu && n.id.as_str() < *bid)))
            }
        };
        if better {
            best = Some((
                s,
                util,
                n.id.as_str(),
                Bottleneck {
                    node: n.id.clone(),
                    kind: n.kind,
                    from_s: s,
                    utilisation: util,
                    reason,
                },
            ));
        }
    }
    best.map(|(.., b)| b)
}

/// Round half to even: 52.5 → 52 (so 3 of 4 tests gives 52 / 70, as in the mockup).
fn part(v: f64) -> u32 {
    v.round_ties_even().max(0.0) as u32
}

/// The score (ADR 0007, review A3):
/// - correctness 70 × accepted / total;
/// - cost 20 × clamp(ref_cost / cost, 0, 1) × accepted / total;
/// - latency 10 × min(1, mean over accepted tests of clamp(1 − p95 / target, 0, 1) / 0.5),
///   0 if none passed.
pub fn score(tests: &[TestResult], cost: f64, ref_cost: f64) -> Score {
    let total = tests.len().max(1) as f64;
    let accepted: Vec<&TestResult> = tests
        .iter()
        .filter(|t| t.verdict == Verdict::Accepted)
        .collect();
    let share = accepted.len() as f64 / total;
    let correctness = part(70.0 * share);
    let cost_ratio = if cost > 0.0 {
        (ref_cost / cost).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let cost = part(20.0 * cost_ratio * share);
    let latency = if accepted.is_empty() {
        0
    } else {
        let headroom: f64 = accepted
            .iter()
            .map(|t| match t.limits.p95_ms {
                Some(target) if target > 0.0 => (1.0 - t.p95_ms / target).clamp(0.0, 1.0),
                _ => 1.0,
            })
            .sum::<f64>()
            / accepted.len() as f64;
        part(10.0 * (headroom / 0.5).min(1.0))
    };
    Score {
        correctness,
        cost,
        latency,
        total: correctness + cost + latency,
    }
}

/// Run one test of `problem` on `design`.
pub fn run_test(
    problem: &ProblemDef,
    test: &TestDef,
    design: &Design,
    registry: &Registry,
) -> Result<RunResult, SimError> {
    run_test_with(problem, test, design, registry, &options(problem))
}

/// `run_test` with explicit simulator options (the accuracy check runs unsampled).
pub fn run_test_with(
    problem: &ProblemDef,
    test: &TestDef,
    design: &Design,
    registry: &Registry,
    opts: &SimOptions,
) -> Result<RunResult, SimError> {
    ka_sim::simulate(
        design,
        registry,
        &problem.scenario(test),
        test_seed(&problem.id, &test.id),
        opts,
    )
}

/// Judge `design` on every test of `problem`. `ref_cost` is the reference design's monthly
/// cost on the first test (from `content.db`).
pub fn judge(
    problem: &ProblemDef,
    design: &Design,
    registry: &Registry,
    ref_cost: f64,
) -> Judgement {
    judge_with(problem, design, registry, ref_cost, &options(problem))
}

/// The simulator options for a problem: its own `sample_cap` if it sets one (review A1).
pub fn options(problem: &ProblemDef) -> SimOptions {
    match problem.sample_cap {
        Some(cap) => SimOptions { sample_cap: cap },
        None => SimOptions::default(),
    }
}

/// `judge` with explicit simulator options (review A1: the gate's accuracy check judges every
/// Set 1 design sampled and unsampled and expects the same verdicts).
pub fn judge_with(
    problem: &ProblemDef,
    design: &Design,
    registry: &Registry,
    ref_cost: f64,
    opts: &SimOptions,
) -> Judgement {
    let lint = lint(problem, design, registry);
    let invalid = |t: &TestDef| TestResult {
        id: t.id.clone(),
        name: t.name.clone(),
        name_es: t.name_es.clone(),
        hidden: t.hidden,
        verdict: Verdict::InvalidDesign,
        p95_ms: 0.0,
        error_rate: 0.0,
        availability: 0.0,
        stale_rate: 0.0,
        lost: 0.0,
        throughput_rps: 0.0,
        cost_month: 0.0,
        limits: problem.pass_for(t),
        bottleneck: None,
        replay_at_s: None,
        seed: test_seed(&problem.id, &t.id),
    };
    if !lint.is_empty() {
        let tests: Vec<TestResult> = problem.tests.iter().map(invalid).collect();
        return Judgement {
            problem_id: problem.id.clone(),
            verdict: Verdict::InvalidDesign,
            lint,
            score: score(&tests, 0.0, ref_cost),
            tests,
            cost_month: 0.0,
            ref_cost,
        };
    }
    let runs: Vec<Result<RunResult, SimError>> = problem
        .tests
        .iter()
        .map(|t| run_test_with(problem, t, design, registry, opts))
        .collect();
    // Budget and the cost score use the first test's steady cost.
    let cost = runs
        .first()
        .and_then(|r| r.as_ref().ok())
        .map_or(0.0, |r| r.cost_month);
    let mut tests = Vec::new();
    let mut lint_late = Vec::new();
    for (t, r) in problem.tests.iter().zip(runs) {
        match r {
            Err(e) => {
                lint_late.push(e.to_string());
                tests.push(invalid(t));
            }
            Ok(r) => {
                let limits = problem.pass_for(t);
                let v = verdict(&r, &limits, cost, problem.nfr.budget_usd);
                let b = if v == Verdict::Accepted {
                    None
                } else {
                    bottleneck(&r)
                };
                let replay_at_s = (v != Verdict::Accepted).then(|| {
                    b.as_ref()
                        .map_or(0.0, |b| f64::from(b.from_s).max(2.0) - 2.0)
                });
                tests.push(TestResult {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    name_es: t.name_es.clone(),
                    hidden: t.hidden,
                    verdict: v,
                    p95_ms: r.latency.p95_ms,
                    error_rate: r.error_rate,
                    availability: r.availability,
                    stale_rate: r.stale_rate,
                    lost: r.lost,
                    throughput_rps: r.throughput_rps,
                    cost_month: r.cost_month,
                    limits,
                    bottleneck: b,
                    replay_at_s,
                    seed: test_seed(&problem.id, &t.id),
                });
            }
        }
    }
    let verdict = tests
        .iter()
        .map(|t| t.verdict)
        .min()
        .unwrap_or(Verdict::Accepted);
    Judgement {
        problem_id: problem.id.clone(),
        verdict,
        lint: lint_late,
        score: score(&tests, cost, ref_cost),
        tests,
        cost_month: cost,
        ref_cost,
    }
}
