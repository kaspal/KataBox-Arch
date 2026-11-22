//! The numbers cheat sheet (`content/numbers.toml`, M5.1): rough speeds and sizes for
//! back-of-the-envelope estimates. Every figure has a plain-language note and a source: a
//! URL, `needed: <suggested page>` while it is unchecked, `arithmetic` for a derived
//! constant, or `simulator` for a figure read from the registry's calibration.

use serde::{Deserialize, Serialize};

/// The sheet's sections, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum NumberGroup {
    Memory,
    Storage,
    Network,
    Servers,
    Sizes,
    Time,
}

/// Units a figure may use.
pub const NUMBER_UNITS: &[&str] = &[
    "ns", "us", "ms", "s", "B", "KB", "MB", "GB", "TB", "rps", "Gbps", "MB/s", "count",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct NumberFigure {
    pub id: String,
    pub group: NumberGroup,
    pub label: String,
    pub label_es: String,
    pub value: f64,
    pub unit: String,
    pub note: String,
    pub note_es: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumberSheet {
    pub figure: Vec<NumberFigure>,
}

/// What a figure's source says about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Url,
    Needed,
    Arithmetic,
    Simulator,
    Invalid,
}

pub fn source_kind(source: &str) -> SourceKind {
    let s = source.trim();
    if s.starts_with("https://") {
        SourceKind::Url
    } else if s
        .strip_prefix("needed:")
        .is_some_and(|r| r.trim().starts_with("https://"))
    {
        SourceKind::Needed
    } else if s == "arithmetic" {
        SourceKind::Arithmetic
    } else if s == "simulator" {
        SourceKind::Simulator
    } else {
        SourceKind::Invalid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources_are_classified() {
        assert_eq!(source_kind("https://a.b"), SourceKind::Url);
        assert_eq!(source_kind("needed: https://a.b"), SourceKind::Needed);
        assert_eq!(source_kind("needed: someday"), SourceKind::Invalid);
        assert_eq!(source_kind("arithmetic"), SourceKind::Arithmetic);
        assert_eq!(source_kind("simulator"), SourceKind::Simulator);
        assert_eq!(source_kind("trust me"), SourceKind::Invalid);
    }
}
