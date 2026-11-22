//! "Which one do I need?" decision guides (`content/decide/<id>.toml`) and the glossary
//! (`content/glossary.toml`). Every human-readable field has a Spanish twin.

use serde::{Deserialize, Serialize};

use crate::registry::ComponentKind;

/// One answer: it leads to another question, or ends at a component kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct DecideOption {
    pub label: String,
    pub label_es: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ComponentKind>,
    /// Why this answer leads there (shown with a result).
    pub why: String,
    pub why_es: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct DecideQuestion {
    pub id: String,
    pub text: String,
    pub text_es: String,
    pub options: Vec<DecideOption>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct DecideFlow {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub title_es: String,
    pub intro: String,
    pub intro_es: String,
    /// The first question's id.
    pub start: String,
    /// `[[question]]` tables in the TOML.
    #[serde(alias = "question")]
    pub questions: Vec<DecideQuestion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct GlossaryTerm {
    pub id: String,
    pub term: String,
    pub term_es: String,
    pub definition: String,
    pub definition_es: String,
    /// A Guide entry that explains it further.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub see: Option<ComponentKind>,
}

/// `content/glossary.toml`: `[[term]]` tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glossary {
    #[serde(rename = "term")]
    pub terms: Vec<GlossaryTerm>,
}
