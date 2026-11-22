//! Pattern library entries (`content/patterns/<id>/`): `pattern.toml`, `pattern.md` and
//! `pattern.es.md` ("How it works"), and `template.kbarch`, the group of cards "Insert
//! pattern" places on the canvas. Every human-readable field has a Spanish twin.

use serde::{Deserialize, Serialize};

use crate::registry::ComponentKind;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct PatternEntry {
    pub id: String,
    /// Library order.
    pub number: u32,
    pub title: String,
    pub title_es: String,
    pub one_liner: String,
    pub one_liner_es: String,
    /// The problem it solves, in a sentence or two.
    pub problem: String,
    pub problem_es: String,
    pub use_when: Vec<String>,
    pub use_when_es: Vec<String>,
    pub gains: Vec<String>,
    pub gains_es: Vec<String>,
    pub costs: Vec<String>,
    pub costs_es: Vec<String>,
    /// The kinds that make it up (all of them are in the template).
    pub kinds: Vec<ComponentKind>,
    /// Problem ids where it is the key move.
    #[serde(default)]
    pub practice: Vec<String>,
    /// How much of it the simulator shows ("Fully", "Partly: …"): honest about what is modelled.
    pub in_simulator: String,
    pub in_simulator_es: String,
    /// Three quick-check questions (M5.5), served without their answers.
    #[serde(default)]
    pub quiz: Vec<crate::quiz::QuizQuestion>,
}
