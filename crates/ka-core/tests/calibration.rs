//! `docs/calibration.md` lists every calibrated figure the registry holds, with the same
//! value (ADR 0005). A row's status (`cited` or `source: needed`) matches `CITED`.

use ka_core::{
    CITED, CalibrationStatus, MODEL, RULE_OF_THUMB, calibration_rows, calibration_status, registry,
};

fn is_listed(id: &str) -> bool {
    MODEL.contains(&id) || RULE_OF_THUMB.contains(&id)
}

const DOC: &str = include_str!("../../../docs/calibration.md");

/// Table rows: `| id | value | unit | source |`, with the id in backticks.
fn doc_rows() -> Vec<(String, String, String)> {
    DOC.lines()
        .filter_map(|l| {
            let cells: Vec<&str> = l
                .trim()
                .strip_prefix('|')?
                .split('|')
                .map(str::trim)
                .collect();
            let id = cells.first()?.strip_prefix('`')?.strip_suffix('`')?;
            Some((
                id.to_owned(),
                cells.get(1)?.to_string(),
                cells.get(3)?.to_string(),
            ))
        })
        .collect()
}

#[test]
fn every_calibrated_figure_has_a_row_with_its_value() {
    let doc = doc_rows();
    let mut missing = Vec::new();
    for r in calibration_rows(&registry()) {
        match doc.iter().find(|(id, ..)| *id == r.id) {
            None => missing.push(format!("| `{}` | {} | | source: needed |", r.id, r.value)),
            Some((_, v, _)) => {
                let doc_v: f64 = v.replace(',', "").parse().unwrap_or(f64::NAN);
                assert_eq!(
                    doc_v, r.value,
                    "{} is {} in the registry, {v} in the doc",
                    r.id, r.value
                );
            }
        }
    }
    assert!(
        missing.is_empty(),
        "rows missing from docs/calibration.md:\n{}",
        missing.join("\n")
    );
}

#[test]
fn the_doc_has_no_stale_rows() {
    let ids: Vec<String> = calibration_rows(&registry())
        .into_iter()
        .map(|r| r.id)
        .collect();
    for (id, ..) in doc_rows() {
        assert!(
            ids.contains(&id),
            "docs/calibration.md lists {id}, which the registry no longer has"
        );
    }
}

#[test]
fn statuses_match_the_sources() {
    let r = registry();
    for (id, _, source) in doc_rows() {
        let needed = source.starts_with("source: needed");
        assert!(needed || !source.is_empty(), "{id} has no source");
        // Every row's status follows the lists in the registry; the source cell says the same.
        let status = calibration_status(&id);
        assert_eq!(
            status == CalibrationStatus::Needed,
            needed || !(CITED.contains(&id.as_str()) || is_listed(&id)),
            "{id}: status vs source"
        );
        match status {
            CalibrationStatus::Model => {
                assert!(
                    source.starts_with("model:"),
                    "{id}: a modelling choice reads `model: <why>`"
                )
            }
            CalibrationStatus::RuleOfThumb => assert!(
                source.starts_with("rule of thumb"),
                "{id}: a rule of thumb says so"
            ),
            _ => {}
        }
        // A prop's inspector tag carries the same status.
        let tag = r
            .kinds
            .iter()
            .flat_map(|k| &k.props)
            .filter_map(|p| p.calibration.as_ref())
            .find(|c| c.id == id)
            .map(|c| c.status);
        if let Some(s) = tag {
            assert_eq!(s, status, "{id}: inspector tag vs lists");
            assert_eq!(
                s == CalibrationStatus::Needed,
                needed,
                "{id}: status vs source"
            );
        }
    }
}
