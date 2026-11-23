//! Outage replays (M10): `content/outages/<id>/` holds `outage.toml`, `hints.md` and
//! `hints.es.md` (three tiers), `as_it_was.kbarch` (the design as it was when the incident
//! happened) and `fixed.kbarch` (the design after the fix). A replay is a Find-the-flaw drill
//! with a story: "Watch it happen" plays `[[step]]`s in the walkthrough player (each callout
//! number is checked against the simulator by `ka-pack validate --run`), then "Fix it" opens
//! the as-it-was design as the problem `outage--<id>`, judged on the scenario's tests: the
//! incident itself, a normal day and an aftershock.
//!
//! Replays are composites: they tell the general shape of well-known incident types, in the
//! simulator's terms. `real_world` links (public postmortems) are added by the citations step
//! and stay empty until then.

use serde::{Deserialize, Serialize};

use crate::flaw::FlawScenario;
use crate::problem::{Difficulty, ProblemDef};
use crate::registry::ComponentKind;
use crate::story::StoryStep;

/// The problem id a replay is judged and run as.
pub fn outage_problem_id(id: &str) -> String {
    format!("outage--{id}")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutageDef {
    pub id: String,
    /// Library order.
    pub number: u32,
    pub difficulty: Difficulty,
    pub title: String,
    pub title_es: String,
    /// One line for the list.
    pub summary: String,
    pub summary_es: String,
    /// What happened, told plainly (shown before "Watch it happen").
    pub story: String,
    pub story_es: String,
    /// What to take away (shown once fixed).
    pub lesson: String,
    pub lesson_es: String,
    /// The test that replays the incident (the as-it-was design fails it).
    pub target_test: String,
    /// The kinds the incident is about (links to the Guide).
    pub kinds: Vec<ComponentKind>,
    /// Public write-ups of real incidents of this type (added by the citations step).
    #[serde(default)]
    pub real_world: Vec<String>,
    pub scenario: FlawScenario,
    /// The "Watch it happen" walkthrough: snapshots are `as_it_was.kbarch` or `fixed.kbarch`,
    /// runs play the scenario's tests.
    #[serde(alias = "step")]
    pub steps: Vec<StoryStep>,
}

impl OutageDef {
    /// The replay as a problem, for the judge and live runs.
    pub fn problem(&self) -> ProblemDef {
        ProblemDef {
            id: outage_problem_id(&self.id),
            number: self.number,
            set: 0,
            difficulty: self.difficulty,
            title: self.title.clone(),
            title_es: self.title_es.clone(),
            teaches: String::new(),
            teaches_es: String::new(),
            tags: Vec::new(),
            requires: Vec::new(),
            nfr: self.scenario.nfr.clone(),
            request_types: self.scenario.request_types.clone(),
            externals: self.scenario.externals.clone(),
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

/// A replay as `content.db` stores it (a `docs` row of kind `outage`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutageDoc {
    pub def: OutageDef,
    pub hints_en: Vec<String>,
    pub hints_es: Vec<String>,
    /// `.kbarch` JSON.
    pub as_it_was: String,
    pub fixed: String,
    /// The fixed design's monthly cost and p95 on the first test.
    pub ref_cost: f64,
    pub ref_p95_ms: f64,
}

/// One row of the Replays list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct OutageSummary {
    pub id: String,
    pub number: u32,
    pub difficulty: Difficulty,
    pub title: String,
    pub title_es: String,
    pub summary: String,
    pub summary_es: String,
    pub kinds: Vec<ComponentKind>,
    pub solved: bool,
}
