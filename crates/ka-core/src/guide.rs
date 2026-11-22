//! Component Guide entries (`content/guide/<kind>/entry.toml`, plus `entry.md` and
//! `entry.es.md` for "What it does"). Every human-readable field has a Spanish twin.
//!
//! Numbers never live in an entry: each "In the simulator" row names the registry value it
//! describes (a prop, a hidden simulator default or a price), and the page shows that value
//! for every flavour, so the Guide and the simulator can't drift apart (`ka-pack` refuses
//! digits in the rows' notes).

use serde::{Deserialize, Serialize};

use crate::registry::{ComponentKind, Flavour};

/// What a simulator row shows: a registry prop, a hidden default, or a price.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum SimValue {
    /// A prop key of the kind (its default per flavour, overrides applied).
    Prop(String),
    /// A `SimDefaults` field: `servers_per_instance`, `service_ms`, `p99_ratio`,
    /// `queue_per_server`, `failover_s`, `write_slots`.
    Sim(String),
    /// A `Cost` field: `per_instance_month`, `per_gb_month`, `per_million`.
    Cost(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct SimRow {
    #[serde(flatten)]
    pub value: SimValue,
    /// What the value does in the simulation (no digits: the number comes from the registry).
    pub note: String,
    pub note_es: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct CompareRow {
    pub label: String,
    pub label_es: String,
    /// One cell per compared flavour, in `flavours_compared.flavours` order.
    pub cells: Vec<String>,
    pub cells_es: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct FlavoursCompared {
    pub flavours: Vec<Flavour>,
    pub rows: Vec<CompareRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Pitfall {
    pub title: String,
    pub title_es: String,
    pub body: String,
    pub body_es: String,
    pub fix: String,
    pub fix_es: String,
    /// A problem id where a test shows it, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seen_in: Option<String>,
}

/// A vendor page a flavour's figures were (or should be) checked against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub flavour: Flavour,
    pub title: String,
    /// The page. Unchecked links (written without fetching) are `checked = false` and listed
    /// for review; the page says so.
    pub url: String,
    #[serde(default)]
    pub checked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct GuideEntry {
    pub kind: ComponentKind,
    /// "A fast, in-memory copy of the data you read most."
    pub one_liner: String,
    pub one_liner_es: String,
    pub use_when: Vec<String>,
    pub use_when_es: Vec<String>,
    pub skip_when: Vec<String>,
    pub skip_when_es: Vec<String>,
    pub sim: Vec<SimRow>,
    /// The model in one line ("hit rate ≈ share of reads whose key is in the hot set").
    pub formula: String,
    pub formula_es: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flavours_compared: Option<FlavoursCompared>,
    pub pitfalls: Vec<Pitfall>,
    pub gains: Vec<String>,
    pub gains_es: Vec<String>,
    pub costs: Vec<String>,
    pub costs_es: Vec<String>,
    pub works_with: Vec<ComponentKind>,
    /// Problem ids where this kind is a key move.
    #[serde(default)]
    pub practice: Vec<String>,
    pub interview_tip: String,
    pub interview_tip_es: String,
    pub sources: Vec<Source>,
    /// Three quick-check questions (M5.5). Never sent to the UI with the entry: the app
    /// serves them without their answers.
    #[serde(default)]
    pub quiz: Vec<crate::quiz::QuizQuestion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sim_row_names_one_registry_value() {
        let r: SimRow =
            toml::from_str("prop = \"memory_gb\"\nnote = \"n\"\nnote_es = \"n\"").unwrap();
        assert_eq!(r.value, SimValue::Prop("memory_gb".into()));
        let r: SimRow =
            toml::from_str("sim = \"p99_ratio\"\nnote = \"n\"\nnote_es = \"n\"").unwrap();
        assert_eq!(r.value, SimValue::Sim("p99_ratio".into()));
        assert!(toml::from_str::<SimRow>("note = \"n\"\nnote_es = \"n\"").is_err());
    }
}
