//! What a run produces (ADR 0006). Counts are weighted back to real traffic when the run
//! was sampled. Every vector is in design order, so results compare byte for byte.

use serde::{Deserialize, Serialize};

use ka_core::ComponentKind;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Percentiles {
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RunResult {
    pub seed: u64,
    pub duration_s: f64,
    /// The share of requests simulated (1 = all).
    pub sample_factor: f64,
    pub requests: f64,
    pub ok: f64,
    pub errors: f64,
    /// Abusive requests a rate limiter turned away (success, not errors).
    pub shed: f64,
    pub error_rate: f64,
    pub availability: f64,
    pub throughput_rps: f64,
    /// The latency each request type is measured by (response or completion), merged.
    pub latency: Percentiles,
    pub response: Percentiles,
    /// For request types with async work.
    pub completion: Option<Percentiles>,
    pub lost: f64,
    pub dead_lettered: f64,
    /// Failed attempts that were retried: not errors, since the message lived on (review F3).
    #[serde(default)]
    pub retried: f64,
    pub stale_reads: f64,
    pub stale_rate: f64,
    /// S7: writes that raced on a contended key or landed on a stale primary in a split.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub conflicts: f64,
    /// S1: the least share of the target connections connected in any judged second (from
    /// `ramp_s` + 10 s to the end), and connection attempts refused (capacity, accept rate).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub connected_min: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub conn_drops: f64,
    /// S2: time to first token over every model call, and calls a provider refused (429).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttft: Option<Percentiles>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rate_limited: f64,
    /// S4: every job run (type, start, seconds to finish; None if it never finished), and
    /// tasks started again (failures, spot interruptions).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jobs: Vec<JobResult>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tasks_retried: f64,
    /// S5: windowed records that arrived past their window's allowed lateness (not lost,
    /// left out of the results), their share of windowed records, and results emitted twice
    /// after a crash replayed from a checkpoint.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub late: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub late_rate: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub duplicates: f64,
    pub cost_month: f64,
    pub per_type: Vec<TypeResult>,
    pub nodes: Vec<NodeResult>,
    /// One entry per simulated second.
    pub series: Vec<SecondResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TypeResult {
    pub id: String,
    pub requests: f64,
    pub errors: f64,
    pub latency: Percentiles,
    pub response: Percentiles,
    pub completion: Option<Percentiles>,
    /// S2: time to first token, for types that call a model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttft: Option<Percentiles>,
}

/// S4: one job run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct JobResult {
    pub ty: String,
    pub start_s: f64,
    pub completion_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NodeResult {
    pub id: String,
    pub kind: ComponentKind,
    /// Busy time over capacity, 0..=1, over the whole run.
    pub utilisation: f64,
    pub max_queue: f64,
    pub drops: f64,
    pub served: f64,
    pub cost_month: f64,
    /// Per second: utilisation, queue length (max in the second), drops.
    pub util_series: Vec<f64>,
    pub queue_series: Vec<f64>,
    pub drop_series: Vec<f64>,
    /// Seconds (indices) in which the card was down (chaos); omitted when never down.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub down_seconds: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SecondResult {
    pub t: u32,
    pub requests: f64,
    pub ok: f64,
    pub errors: f64,
    pub p95_ms: f64,
}

/// Fields added in M8 are left out of the JSON when zero, so older results (and goldens)
/// read and write the same bytes.
pub fn is_zero(v: &f64) -> bool {
    *v == 0.0
}
