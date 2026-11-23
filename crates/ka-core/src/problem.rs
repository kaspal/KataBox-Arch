//! A practice problem as `content/problems/<id>/problem.toml` defines it (ADR 0008): the
//! numbers it promises (NFRs), the traffic it sends, the tests it runs and what passing means.
//! The judge (`ka-judge`), the packer (`ka-pack`) and the app read the same types.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::registry::ComponentKind;
use crate::scenario::{ChaosEvent, Curve, External, KeySkew, PassCriteria, RequestType, Scenario};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

/// The catalogue a problem belongs to (M8): the core sets, the AI systems track or the data
/// engineering track. Problems are unique by (track, set, number).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Track {
    #[default]
    Core,
    Ai,
    De,
}

impl Track {
    pub fn id(self) -> &'static str {
        match self {
            Track::Core => "core",
            Track::Ai => "ai",
            Track::De => "de",
        }
    }

    pub fn is_core(&self) -> bool {
        *self == Track::Core
    }
}

/// The non-functional requirements shown in the brief.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Nfr {
    /// Requests per second at a traffic multiplier of 1.
    pub peak_rps: f64,
    /// Share of reads, for the brief (the request types define the real mix).
    #[serde(default)]
    pub read_ratio: Option<f64>,
    pub p95_ms: f64,
    /// 0–1, e.g. 0.999.
    pub availability: f64,
    /// Monthly budget in teaching dollars (ADR 0006).
    pub budget_usd: f64,
}

/// One test: a traffic shape, optional chaos and optional stricter (or looser) limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct TestDef {
    pub id: String,
    pub name: String,
    pub name_es: String,
    /// Hidden tests show their name only until the problem is solved.
    #[serde(default)]
    pub hidden: bool,
    #[serde(default = "default_duration")]
    pub duration_s: f64,
    #[serde(default)]
    pub traffic: Curve,
    #[serde(default)]
    pub chaos: Vec<ChaosEvent>,
    /// Per request type multipliers (a burst of one kind of traffic, abusive traffic).
    #[serde(default)]
    pub type_multipliers: BTreeMap<String, f64>,
    #[serde(default)]
    pub key_skew_override: Option<KeySkew>,
    #[serde(default)]
    pub cold_caches: bool,
    /// Overrides of the problem's pass criteria for this test only.
    #[serde(default)]
    pub pass: Option<PassCriteria>,
}

fn default_duration() -> f64 {
    60.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct ProblemDef {
    pub id: String,
    pub number: u32,
    pub set: u32,
    /// `core` (the default), `ai` or `de`.
    #[serde(default, skip_serializing_if = "Track::is_core")]
    pub track: Track,
    pub difficulty: Difficulty,
    pub title: String,
    pub title_es: String,
    pub teaches: String,
    pub teaches_es: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Kinds a valid design must contain (e.g. metrics), checked by the judge's lint.
    #[serde(default)]
    pub requires: Vec<ComponentKind>,
    pub nfr: Nfr,
    pub request_types: Vec<RequestType>,
    #[serde(default)]
    pub externals: Vec<External>,
    pub tests: Vec<TestDef>,
    pub pass: PassCriteria,
    /// Simulated arrivals per second above which the simulator samples (ADR 0006). Set only
    /// where the review A1 accuracy check showed sampling changes a verdict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_cap: Option<f64>,
    /// The "same SLO, half the cost" budget as a share of the reference's cost, when 0.5
    /// isn't reachable: the proven `challenge.kbarch` cost over the reference's, rounded up to
    /// the next 0.05 (at most 0.80; above that the problem has no challenge). Unset = 0.5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge_ratio: Option<f64>,
}

impl ProblemDef {
    pub fn test(&self, id: &str) -> Option<&TestDef> {
        self.tests.iter().find(|t| t.id == id)
    }

    /// The simulator scenario of one test.
    pub fn scenario(&self, test: &TestDef) -> Scenario {
        Scenario {
            peak_rps: self.nfr.peak_rps,
            duration_s: test.duration_s,
            request_types: self.request_types.clone(),
            externals: self.externals.clone(),
            traffic: test.traffic.clone(),
            chaos: test.chaos.clone(),
            type_multipliers: test.type_multipliers.clone(),
            key_skew_override: test.key_skew_override,
            cold_caches: test.cold_caches,
        }
    }

    /// What passing one test means: the problem's criteria, with the test's overrides, and
    /// the NFR's p95 and availability where the problem sets no stricter limit.
    pub fn pass_for(&self, test: &TestDef) -> PassCriteria {
        let mut base = self.pass.clone();
        if base.p95_ms.is_none() {
            base.p95_ms = Some(self.nfr.p95_ms);
        }
        if base.availability.is_none() {
            base.availability = Some(self.nfr.availability);
        }
        match &test.pass {
            Some(o) => base.merged(o),
            None => base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOML: &str = r#"
id = "url-shortener"
number = 1
set = 1
difficulty = "easy"
title = "URL Shortener"
title_es = "Acortador de URLs"
teaches = "Caching, replicas, failover"
teaches_es = "Caché, réplicas, failover"
tags = ["caching", "read-heavy"]

[nfr]
peak_rps = 12000
read_ratio = 0.95
p95_ms = 80
availability = 0.999
budget_usd = 2500

[[request_types]]
id = "redirect"
share = 0.95
route = ["edge?", "compute", "cache?", "datastore"]
key_space = 10_000_000
key_skew = { zipf = 1.1 }

[[request_types]]
id = "create"
share = 0.05
route = ["edge?", "compute", "datastore:write"]

[[tests]]
id = "normal"
name = "Normal day"
name_es = "Un día normal"

[[tests]]
id = "viral"
name = "Link goes viral"
name_es = "El enlace se hace viral"
hidden = true
traffic = { spike = { at = 20, x = 4, for = 15 } }
chaos = [{ kill = { kind = "sql", role = "primary", at = 30 } }]
pass = { p95_ms = 120 }

[pass]
error_rate = 0.001
"#;

    #[test]
    fn a_problem_reads_from_toml_with_its_tests_and_chaos() {
        let p: ProblemDef = toml::from_str(TOML).unwrap();
        assert_eq!(p.difficulty, Difficulty::Easy);
        assert_eq!(p.request_types.len(), 2);
        let viral = p.test("viral").unwrap();
        assert!(viral.hidden);
        assert_eq!(viral.duration_s, 60.0);
        match &viral.chaos[0] {
            ChaosEvent::Kill { target, at, .. } => {
                assert_eq!(target.kind, Some(ComponentKind::Sql));
                assert_eq!(*at, 30.0);
            }
            other => panic!("{other:?}"),
        }
        let s = p.scenario(viral);
        assert_eq!(s.peak_rps, 12000.0);
        assert!(s.traffic.peak() > 3.9);
    }

    #[test]
    fn pass_criteria_take_the_nfr_then_the_test_override() {
        let p: ProblemDef = toml::from_str(TOML).unwrap();
        let normal = p.pass_for(p.test("normal").unwrap());
        assert_eq!(normal.p95_ms, Some(80.0));
        assert_eq!(normal.availability, Some(0.999));
        assert_eq!(normal.error_rate, Some(0.001));
        let viral = p.pass_for(p.test("viral").unwrap());
        assert_eq!(viral.p95_ms, Some(120.0));
        assert_eq!(viral.error_rate, Some(0.001));
    }

    #[test]
    fn unknown_fields_are_refused() {
        let bad = TOML.replace("budget_usd = 2500", "budget_usd = 2500\nbudget = 1");
        assert!(toml::from_str::<ProblemDef>(&bad).is_err());
    }
}
