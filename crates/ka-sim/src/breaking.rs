//! The breaking point (plan M1.6): how much traffic a design holds before it fails, and
//! which card fails first.
//!
//! The scenario's mix runs in fixed 20 s windows at traffic multipliers starting at ×0.25 and
//! growing ×1.5 until the first failing window, then bisects to within 5%. A window fails
//! when its p95 is above the target or its error rate above the limit. Everything is
//! deterministic per seed: the same design always reports the same limit.

use ka_core::scenario::{Curve, Scenario};
use ka_core::{Design, Registry};
use serde::{Deserialize, Serialize};

use crate::compile::{SimError, SimOptions, compile};
use crate::result::RunResult;

/// When a window counts as failing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Limits {
    pub p95_ms: f64,
    pub error_rate: f64,
}

impl Default for Limits {
    /// The Sandbox's limits (problems use their own targets).
    fn default() -> Self {
        Limits {
            p95_ms: 200.0,
            error_rate: 0.01,
        }
    }
}

/// Why the first card failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    /// Every server busy.
    Saturated,
    /// Its queue filled and it dropped requests.
    QueueOverflow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FirstFailure {
    pub node: String,
    pub reason: FailureReason,
    /// Seconds into the first failing window.
    pub at_s: f64,
    pub utilisation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BreakingPoint {
    /// Real requests per second the design holds (the last passing window), and its multiple
    /// of the scenario's own rate.
    pub limit_rps: f64,
    pub multiplier: f64,
    /// The scenario's own rate (multiplier 1).
    pub base_rps: f64,
    /// It never failed up to the search's ceiling: the limit is at least this.
    pub beyond_ceiling: bool,
    /// The first card to give way in the first failing window, if one did (a design can
    /// also fail on latency alone).
    pub first_failure: Option<FirstFailure>,
    /// What failed in that window.
    pub failing_p95_ms: f64,
    pub failing_error_rate: f64,
    /// Stale reads over reads in the last passing window (replica lag, cache-aside).
    pub stale_rate: f64,
    /// Windows simulated.
    pub runs: u32,
}

pub const WINDOW_S: f64 = 20.0;
pub const START: f64 = 0.25;
pub const GROWTH: f64 = 1.5;
/// Bisect until the passing and failing multipliers are within 5%.
pub const PRECISION: f64 = 0.05;
/// Stop growing at this multiple of the scenario's rate.
pub const CEILING: f64 = 1024.0;

fn window(scenario: &Scenario, x: f64) -> Scenario {
    let mut s = scenario.clone();
    s.duration_s = WINDOW_S;
    s.traffic = Curve::Flat(x);
    s.chaos.clear();
    s
}

fn fails(r: &RunResult, limits: Limits) -> bool {
    r.latency.p95_ms > limits.p95_ms || r.error_rate > limits.error_rate
}

/// The first card to give way: the earliest second in which a card is saturated (≥ 95%
/// busy) or drops; ties go to the busiest, then the lowest id.
pub fn first_failure(r: &RunResult) -> Option<FirstFailure> {
    let mut best: Option<(usize, f64, &str, FailureReason)> = None;
    for n in &r.nodes {
        let first = (0..n.util_series.len())
            .find(|&s| n.util_series[s] >= 0.95 || n.drop_series.get(s).is_some_and(|d| *d > 0.0));
        let Some(s) = first else { continue };
        let util = n.util_series[s];
        let reason = if n.drop_series.get(s).is_some_and(|d| *d > 0.0) {
            FailureReason::QueueOverflow
        } else {
            FailureReason::Saturated
        };
        let better = match best {
            None => true,
            Some((bs, bu, bid, _)) => {
                s < bs || (s == bs && (util > bu || (util == bu && n.id.as_str() < bid)))
            }
        };
        if better {
            best = Some((s, util, n.id.as_str(), reason));
        }
    }
    best.map(|(s, util, id, reason)| FirstFailure {
        node: id.to_owned(),
        reason,
        at_s: s as f64,
        utilisation: util,
    })
}

pub fn find_limit(
    design: &Design,
    registry: &Registry,
    scenario: &Scenario,
    seed: u64,
    limits: Limits,
) -> Result<BreakingPoint, SimError> {
    let opts = SimOptions::default();
    let base_rps: f64 = compile(design, registry, &window(scenario, 1.0), &opts)?
        .types
        .iter()
        .map(|t| t.rate)
        .sum();
    let mut runs = 0;
    let mut run = |x: f64| -> Result<RunResult, SimError> {
        runs += 1;
        crate::simulate(design, registry, &window(scenario, x), seed, &opts)
    };
    // Grow until a window fails.
    let mut pass = 0.0;
    let mut stale_rate = 0.0;
    let mut x = START;
    let mut failing = loop {
        let r = run(x)?;
        if fails(&r, limits) {
            break Some((x, r));
        }
        pass = x;
        stale_rate = r.stale_rate;
        if x >= CEILING {
            break None;
        }
        x = (x * GROWTH).min(CEILING);
    };
    // Bisect between the last pass and the first failure.
    if let Some((mut fail_x, mut fail_r)) = failing.take() {
        while pass > 0.0 && (fail_x - pass) / fail_x > PRECISION {
            let mid = (pass + fail_x) / 2.0;
            let r = run(mid)?;
            if fails(&r, limits) {
                fail_x = mid;
                fail_r = r;
            } else {
                pass = mid;
                stale_rate = r.stale_rate;
            }
        }
        return Ok(BreakingPoint {
            limit_rps: base_rps * pass,
            multiplier: pass,
            base_rps,
            beyond_ceiling: false,
            first_failure: first_failure(&fail_r),
            failing_p95_ms: fail_r.latency.p95_ms,
            failing_error_rate: fail_r.error_rate,
            stale_rate,
            runs,
        });
    }
    Ok(BreakingPoint {
        limit_rps: base_rps * pass,
        multiplier: pass,
        base_rps,
        beyond_ceiling: true,
        first_failure: None,
        failing_p95_ms: 0.0,
        failing_error_rate: 0.0,
        stale_rate,
        runs,
    })
}
