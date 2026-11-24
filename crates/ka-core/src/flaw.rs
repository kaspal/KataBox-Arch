//! Find the flaw (M5.3): `content/flaws/<id>/` holds `flaw.toml`, `hints.md` and
//! `hints.es.md` (three tiers), `broken.kbarch` (a working-looking design with one flaw) and
//! `fixed.kbarch` (the reference fix). The `[scenario]` is a problem's traffic part (NFRs,
//! request types, externals, tests, pass limits), so a drill is judged exactly like a
//! problem: as the problem `flaw--<id>`.

use serde::{Deserialize, Serialize};

use crate::problem::{Difficulty, Nfr, ProblemDef, TestDef};
use crate::registry::ComponentKind;
use crate::scenario::{External, PassCriteria, RequestType};

/// The problem id a drill is judged and run as.
pub fn flaw_problem_id(id: &str) -> String {
    format!("flaw--{id}")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlawScenario {
    pub nfr: Nfr,
    pub request_types: Vec<RequestType>,
    #[serde(default)]
    pub externals: Vec<External>,
    pub tests: Vec<TestDef>,
    pub pass: PassCriteria,
    #[serde(default)]
    pub sample_cap: Option<f64>,
    /// M10: async SQL failovers lose the lag window's writes.
    #[serde(default)]
    pub replication_loss: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlawDef {
    pub id: String,
    /// Library order.
    pub number: u32,
    pub title: String,
    pub title_es: String,
    /// What the system is, in a few sentences.
    pub story: String,
    pub story_es: String,
    /// What goes wrong, as a user would report it (no spoilers).
    pub symptom: String,
    pub symptom_es: String,
    /// The test the broken design fails.
    pub target_test: String,
    /// Why it failed and what the fix changes (shown once solved).
    pub explanation: String,
    pub explanation_es: String,
    /// The kinds the flaw is about (links to the Guide).
    pub kinds: Vec<ComponentKind>,
    pub scenario: FlawScenario,
}

impl FlawDef {
    /// The drill as a problem, for the judge and live runs.
    pub fn problem(&self) -> ProblemDef {
        ProblemDef {
            id: flaw_problem_id(&self.id),
            number: self.number,
            set: 0,
            difficulty: Difficulty::Medium,
            title: self.title.clone(),
            title_es: self.title_es.clone(),
            teaches: String::new(),
            teaches_es: String::new(),
            tags: Vec::new(),
            requires: Vec::new(),
            nfr: self.scenario.nfr.clone(),
            request_types: self.scenario.request_types.clone(),
            externals: self.scenario.externals.clone(),
            model_limits: Vec::new(),
            tests: self.scenario.tests.clone(),
            pass: self.scenario.pass.clone(),
            sample_cap: self.scenario.sample_cap,
            track: Default::default(),
            challenge_ratio: None,
            connections: false,
            replication_loss: self.scenario.replication_loss,
        }
    }
}

/// A drill as `content.db` stores it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlawDoc {
    pub def: FlawDef,
    pub hints_en: Vec<String>,
    pub hints_es: Vec<String>,
    /// `.kbarch` JSON.
    pub broken: String,
    pub fixed: String,
    /// The fixed design's monthly cost and p95 on the first test (the cost bar and the
    /// reference point, as for problems).
    pub ref_cost: f64,
    pub ref_p95_ms: f64,
}

/// One row of the Find-the-flaw list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FlawSummary {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub title_es: String,
    pub kinds: Vec<ComponentKind>,
    pub solved: bool,
}
