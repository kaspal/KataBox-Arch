//! Learning paths (M5.4): `content/paths.toml`, ordered tracks mixing Guide entries, concept
//! cards, patterns, problems, estimation drills, Find-the-flaw drills and quizzes. An item is
//! written `<kind>:<target>`, e.g. `guide:cache`, `problem:url-shortener`,
//! `quiz:pattern:cache-aside`. Progress is derived from what the user has already done.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathDef {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub title_es: String,
    pub blurb: String,
    pub blurb_es: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathFile {
    pub path: Vec<PathDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum PathItemKind {
    Guide,
    Concept,
    Pattern,
    Problem,
    Estimate,
    Flaw,
    Replay,
    Quiz,
}

/// A parsed item: its kind and target (`quiz` targets are `guide:<kind>` or
/// `pattern:<id>`).
pub fn parse_item(item: &str) -> Option<(PathItemKind, &str)> {
    let (kind, target) = item.split_once(':')?;
    let kind = match kind {
        "guide" => PathItemKind::Guide,
        "concept" => PathItemKind::Concept,
        "pattern" => PathItemKind::Pattern,
        "problem" => PathItemKind::Problem,
        "estimate" => PathItemKind::Estimate,
        "flaw" => PathItemKind::Flaw,
        "replay" => PathItemKind::Replay,
        "quiz" => PathItemKind::Quiz,
        _ => return None,
    };
    (!target.is_empty()).then_some((kind, target))
}

/// One step of a path as the UI shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PathItem {
    pub kind: PathItemKind,
    pub target: String,
    /// Content titles (patterns, problems, drills); Guide and concept items are named by
    /// the UI from the component kind.
    pub title: Option<String>,
    pub title_es: Option<String>,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PathView {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub title_es: String,
    pub blurb: String,
    pub blurb_es: String,
    pub items: Vec<PathItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_parse() {
        assert_eq!(
            parse_item("guide:cache"),
            Some((PathItemKind::Guide, "cache"))
        );
        assert_eq!(
            parse_item("quiz:pattern:cache-aside"),
            Some((PathItemKind::Quiz, "pattern:cache-aside"))
        );
        assert_eq!(parse_item("video:intro"), None);
        assert_eq!(parse_item("guide:"), None);
        assert_eq!(parse_item("guide"), None);
    }
}
