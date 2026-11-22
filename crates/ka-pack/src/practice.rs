//! Practice content (M5): the numbers cheat sheet (`numbers.toml`), and from later steps
//! estimation drills, Find-the-flaw drills and learning paths. Each loads into [`Practice`],
//! is checked by [`validate_practice`] and becomes `docs` rows in `content.db` (schema 2).

use std::collections::BTreeSet;
use std::path::Path;

use ka_core::Registry;
use ka_core::numbers::{NUMBER_UNITS, NumberFigure, NumberSheet, SourceKind, source_kind};
use ka_store::DocRow;

use crate::problems::ProblemDir;
use crate::{Issue, TODO_MARKER, issue};

/// Everything under the practice content.
#[derive(Debug, Clone, Default)]
pub struct Practice {
    /// The numbers cheat sheet, in file order (empty if `numbers.toml` is absent).
    pub numbers: Vec<NumberFigure>,
}

fn kebab(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}

pub fn load_numbers(root: &Path) -> Result<Vec<NumberFigure>, Vec<Issue>> {
    let path = root.join("numbers.toml");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text =
        std::fs::read_to_string(&path).map_err(|e| vec![issue("numbers.toml", e.to_string())])?;
    let sheet: NumberSheet =
        toml::from_str(&text).map_err(|e| vec![issue("numbers.toml", e.to_string())])?;
    Ok(sheet.figure)
}

/// Ids unique and kebab-case; text in both languages; a known unit; a positive value; a
/// source that is a URL, `needed: <URL>`, `arithmetic` or `simulator`.
pub fn validate_numbers(figures: &[NumberFigure]) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut seen = BTreeSet::new();
    for f in figures {
        let mut bad = |m: String| issues.push(issue("numbers.toml", format!("`{}`: {m}", f.id)));
        if !kebab(&f.id) {
            bad("the id must be kebab-case".into());
        }
        if !seen.insert(f.id.clone()) {
            bad("duplicate id".into());
        }
        for (name, text) in [
            ("label", &f.label),
            ("label_es", &f.label_es),
            ("note", &f.note),
            ("note_es", &f.note_es),
        ] {
            if text.trim().is_empty() {
                bad(format!("`{name}` is empty"));
            }
            if text.contains(TODO_MARKER) {
                bad(format!("`{name}` is unfinished"));
            }
        }
        if !NUMBER_UNITS.contains(&f.unit.as_str()) {
            bad(format!(
                "unknown unit `{}` (one of {NUMBER_UNITS:?})",
                f.unit
            ));
        }
        if !(f.value.is_finite() && f.value > 0.0) {
            bad(format!("the value must be positive, not {}", f.value));
        }
        if source_kind(&f.source) == SourceKind::Invalid {
            bad(format!(
                "source `{}`: a https:// URL, `needed: <URL>`, `arithmetic` or `simulator`",
                f.source
            ));
        }
    }
    issues
}

pub fn load_practice(root: &Path, _registry: &Registry) -> Result<Practice, Vec<Issue>> {
    Ok(Practice {
        numbers: load_numbers(root)?,
    })
}

pub fn validate_practice(
    p: &Practice,
    _problems: &[ProblemDir],
    _registry: &Registry,
) -> Vec<Issue> {
    validate_numbers(&p.numbers)
}

/// The `docs` rows for `content.db`.
pub fn practice_docs(p: &Practice) -> Result<Vec<DocRow>, String> {
    let mut rows = Vec::new();
    if !p.numbers.is_empty() {
        rows.push(DocRow {
            kind: "numbers".into(),
            id: "sheet".into(),
            json: serde_json::to_string(&p.numbers).map_err(|e| e.to_string())?,
        });
    }
    Ok(rows)
}

/// One line for `validate`.
pub fn summary(p: &Practice) -> String {
    format!("{} number(s) valid", p.numbers.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ka_core::numbers::NumberGroup;

    fn fig(id: &str) -> NumberFigure {
        NumberFigure {
            id: id.into(),
            group: NumberGroup::Memory,
            label: "L".into(),
            label_es: "L".into(),
            value: 100.0,
            unit: "ns".into(),
            note: "n".into(),
            note_es: "n".into(),
            source: "needed: https://example.org".into(),
        }
    }

    #[test]
    fn numbers_are_checked() {
        assert!(validate_numbers(&[fig("a"), fig("b")]).is_empty());
        let mut bad = fig("B_d");
        bad.unit = "furlongs".into();
        bad.value = 0.0;
        bad.source = "a book".into();
        bad.label_es = String::new();
        let issues = validate_numbers(&[bad, fig("a"), fig("a")]);
        let text: Vec<String> = issues.iter().map(|i| i.message.clone()).collect();
        for want in [
            "kebab-case",
            "unknown unit",
            "positive",
            "source",
            "label_es",
            "duplicate",
        ] {
            assert!(text.iter().any(|t| t.contains(want)), "{want}: {text:#?}");
        }
    }
}
