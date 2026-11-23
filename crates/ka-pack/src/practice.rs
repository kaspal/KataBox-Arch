//! Practice content (M5): the numbers cheat sheet (`numbers.toml`), and from later steps
//! estimation drills, Find-the-flaw drills and learning paths. Each loads into [`Practice`],
//! is checked by [`validate_practice`] and becomes `docs` rows in `content.db` (schema 2).

use std::collections::BTreeSet;
use std::path::Path;

use ka_core::Registry;
use ka_core::estimate::{DrillDef, DrillFile, DrillSet};
use ka_core::numbers::{NUMBER_UNITS, NumberFigure, NumberSheet, SourceKind, source_kind};
use ka_store::DocRow;

use crate::problems::ProblemDir;
use crate::{Issue, TODO_MARKER, issue};

/// Everything under the practice content.
#[derive(Debug, Clone, Default)]
pub struct Practice {
    /// The numbers cheat sheet, in file order (empty if `numbers.toml` is absent).
    pub numbers: Vec<NumberFigure>,
    /// Estimation drills by problem (`estimation/<problem-id>.toml`), in problem-id order.
    pub estimation: Vec<(String, Vec<DrillDef>)>,
    /// Find-the-flaw drills (`flaws/<id>/`), in folder order.
    pub flaws: Vec<crate::flaws::FlawDir>,
    /// M10 outage replays.
    pub outages: Vec<crate::outages::OutageDir>,
    /// Learning paths (`paths.toml`), in file order.
    pub paths: Vec<ka_core::paths::PathDef>,
    /// What a path may point at: the concept cards and patterns that exist (folder names).
    pub concept_ids: BTreeSet<String>,
    pub pattern_ids: BTreeSet<String>,
    /// KataBox cross-links (M6.7): the snapshot of KataBox ids, and the links.
    pub katabox: ka_core::crosslinks::KataboxIds,
    pub crosslinks: Vec<ka_core::crosslinks::LinkDef>,
}

fn folder_names(dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

pub fn load_paths(root: &Path) -> Result<Vec<ka_core::paths::PathDef>, Vec<Issue>> {
    let path = root.join("paths.toml");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text =
        std::fs::read_to_string(&path).map_err(|e| vec![issue("paths.toml", e.to_string())])?;
    let file: ka_core::paths::PathFile =
        toml::from_str(&text).map_err(|e| vec![issue("paths.toml", e.to_string())])?;
    Ok(file.path)
}

/// Every item names something that exists; ids, numbers and text are sound.
pub fn validate_paths(p: &Practice, problems: &[ProblemDir], registry: &Registry) -> Vec<Issue> {
    use ka_core::paths::{PathItemKind as K, parse_item};
    let mut issues = Vec::new();
    let kind_exists = |k: &str| registry.kinds.iter().any(|s| s.kind.id() == k);
    let problem_exists = |id: &str| problems.iter().any(|x| x.def.id == id);
    let mut ids = BTreeSet::new();
    let mut numbers = BTreeSet::new();
    for path in &p.paths {
        let mut bad = |m: String| issues.push(issue("paths.toml", format!("`{}`: {m}", path.id)));
        if !kebab(&path.id) || !ids.insert(path.id.clone()) {
            bad("ids must be kebab-case and unique".into());
        }
        if !numbers.insert(path.number) {
            bad(format!("number {} is used twice", path.number));
        }
        for (name, text) in [
            ("title", &path.title),
            ("title_es", &path.title_es),
            ("blurb", &path.blurb),
            ("blurb_es", &path.blurb_es),
        ] {
            if text.trim().is_empty() || text.contains(TODO_MARKER) {
                bad(format!("`{name}` is empty or unfinished"));
            }
        }
        if path.items.len() < 3 {
            bad("a path needs at least three items".into());
        }
        let mut seen = BTreeSet::new();
        for item in &path.items {
            if !seen.insert(item.clone()) {
                bad(format!("`{item}` appears twice"));
            }
            let ok = match parse_item(item) {
                None => false,
                Some((K::Guide, k)) => kind_exists(k),
                Some((K::Concept, k)) => p.concept_ids.contains(k),
                Some((K::Pattern, id)) => p.pattern_ids.contains(id),
                Some((K::Problem, id)) => problem_exists(id),
                Some((K::Estimate, id)) => p.estimation.iter().any(|(pid, _)| pid == id),
                Some((K::Flaw, id)) => p.flaws.iter().any(|f| f.def.id == id),
                Some((K::Replay, id)) => p.outages.iter().any(|o| o.def.id == id),
                Some((K::Quiz, target)) => match target.split_once(':') {
                    Some(("guide", k)) => kind_exists(k),
                    Some(("pattern", id)) => p.pattern_ids.contains(id),
                    _ => false,
                },
            };
            if !ok {
                bad(format!("`{item}` doesn't name anything that exists"));
            }
        }
    }
    issues
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
                "source `{}`: a https:// URL, `needed: <URL>`, `arithmetic`, `simulator` or `rule of thumb`",
                f.source
            ));
        }
    }
    issues
}

pub fn load_estimation(root: &Path) -> Result<Vec<(String, Vec<DrillDef>)>, Vec<Issue>> {
    let dir = root.join("estimation");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| vec![issue("estimation", e.to_string())])?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("toml"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    let mut issues = Vec::new();
    for f in files {
        let id = f
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned();
        let name = format!("estimation/{id}.toml");
        match std::fs::read_to_string(&f)
            .map_err(|e| e.to_string())
            .and_then(|t| toml::from_str::<DrillFile>(&t).map_err(|e| e.to_string()))
        {
            Ok(file) => out.push((id, file.drill)),
            Err(e) => issues.push(issue(name, e)),
        }
    }
    if issues.is_empty() {
        Ok(out)
    } else {
        Err(issues)
    }
}

/// `{name}` placeholders in a text.
fn placeholders(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        out.push(rest[open + 1..open + close].to_owned());
        rest = &rest[open + close + 1..];
    }
    out
}

/// A drill is answerable and honest: its formula parses and uses exactly its inputs, every
/// input is shown in the prompt (both languages), placeholders name real inputs, and the
/// answer is a positive number.
pub fn validate_estimation(
    sets: &[(String, Vec<DrillDef>)],
    problems: &[ProblemDir],
) -> Vec<Issue> {
    let mut issues = Vec::new();
    for (pid, drills) in sets {
        let file = format!("estimation/{pid}.toml");
        if !problems.iter().any(|p| &p.def.id == pid) {
            issues.push(issue(&file, format!("no problem `{pid}`")));
        }
        if drills.is_empty() {
            issues.push(issue(&file, "no drills"));
        }
        let mut seen = BTreeSet::new();
        for d in drills {
            let mut bad = |m: String| issues.push(issue(&file, format!("`{}`: {m}", d.id)));
            if !kebab(&d.id) || !seen.insert(d.id.clone()) {
                bad("ids must be kebab-case and unique".into());
            }
            for (name, text) in [
                ("prompt", &d.prompt),
                ("prompt_es", &d.prompt_es),
                ("solution", &d.solution),
                ("solution_es", &d.solution_es),
            ] {
                if text.trim().is_empty() || text.contains(TODO_MARKER) {
                    bad(format!("`{name}` is empty or unfinished"));
                }
                for ph in placeholders(text) {
                    let ok = d.inputs.contains_key(&ph)
                        || (ph == "answer" && name.starts_with("solution"));
                    if !ok {
                        bad(format!("`{name}` shows `{{{ph}}}`, which isn't an input"));
                    }
                }
            }
            for (name, text) in [("prompt", &d.prompt), ("prompt_es", &d.prompt_es)] {
                let shown = placeholders(text);
                for input in d.inputs.keys() {
                    if !shown.contains(input) {
                        bad(format!("`{name}` doesn't show the input `{input}`"));
                    }
                }
            }
            if !NUMBER_UNITS.contains(&d.unit.as_str()) {
                bad(format!("unknown unit `{}`", d.unit));
            }
            if !(0.05..=2.0).contains(&d.tolerance) {
                bad(format!("tolerance {} outside 0.05–2", d.tolerance));
            }
            match ka_core::estimate::parse(&d.formula) {
                Err(e) => bad(format!("formula: {e}")),
                Ok(expr) => {
                    let mut used = Vec::new();
                    ka_core::estimate::names(&expr, &mut used);
                    for u in &used {
                        if !d.inputs.contains_key(u) {
                            bad(format!("the formula uses `{u}`, which isn't an input"));
                        }
                    }
                    for input in d.inputs.keys() {
                        if !used.contains(input) {
                            bad(format!("the input `{input}` isn't used by the formula"));
                        }
                    }
                    if let Err(e) = ka_core::estimate::answer(d) {
                        bad(e);
                    }
                }
            }
        }
    }
    issues
}

/// A TOML file under the content root, or its default when absent.
fn optional_toml<T: serde::de::DeserializeOwned + Default>(
    root: &Path,
    name: &str,
) -> Result<T, Vec<Issue>> {
    let path = root.join(name);
    if !path.exists() {
        return Ok(T::default());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| vec![issue(name, e.to_string())])?;
    toml::from_str(&text).map_err(|e| vec![issue(name, e.to_string())])
}

/// Every link starts at a page that exists, once, and resolves in the KataBox snapshot.
pub fn validate_crosslinks(
    p: &Practice,
    problems: &[ProblemDir],
    registry: &Registry,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut seen = BTreeSet::new();
    for l in &p.crosslinks {
        let at = "crosslinks.toml";
        let exists = match l.from.split_once(':') {
            Some(("guide", k)) => registry.kinds.iter().any(|s| s.kind.id() == k),
            Some(("pattern", id)) => p.pattern_ids.contains(id),
            Some(("problem", id)) => problems.iter().any(|x| x.def.id == id),
            _ => false,
        };
        if !exists {
            issues.push(issue(
                at,
                format!("{}: no such Guide entry, pattern or problem", l.from),
            ));
        }
        if !seen.insert(l.from.as_str()) {
            issues.push(issue(at, format!("{}: linked twice", l.from)));
        }
        if l.to.is_empty() {
            issues.push(issue(at, format!("{}: no targets", l.from)));
        }
        let mut targets = BTreeSet::new();
        for t in &l.to {
            if p.katabox.resolve(t).is_none() {
                issues.push(issue(
                    at,
                    format!("{}: {t} is not in katabox-ids.toml", l.from),
                ));
            }
            if !targets.insert(t.as_str()) {
                issues.push(issue(at, format!("{}: {t} twice", l.from)));
            }
        }
    }
    if !p.crosslinks.is_empty() && p.katabox.katabox_commit.trim().is_empty() {
        issues.push(issue("katabox-ids.toml", "katabox_commit is missing"));
    }
    issues
}

pub fn load_practice(root: &Path, registry: &Registry) -> Result<Practice, Vec<Issue>> {
    let links: ka_core::crosslinks::CrosslinkFile = optional_toml(root, "crosslinks.toml")?;
    Ok(Practice {
        katabox: optional_toml(root, "katabox-ids.toml")?,
        crosslinks: links.link,
        numbers: load_numbers(root)?,
        estimation: load_estimation(root)?,
        flaws: crate::flaws::load_flaws(root, registry)?,
        outages: crate::outages::load_outages(root, registry)?,
        paths: load_paths(root)?,
        concept_ids: folder_names(&root.join("concepts")),
        pattern_ids: folder_names(&root.join("patterns")),
    })
}

pub fn validate_practice(p: &Practice, problems: &[ProblemDir], registry: &Registry) -> Vec<Issue> {
    let mut issues = validate_numbers(&p.numbers);
    issues.extend(validate_estimation(&p.estimation, problems));
    issues.extend(crate::flaws::validate_flaws(&p.flaws, registry));
    issues.extend(crate::outages::validate_outages(&p.outages, registry));
    issues.extend(validate_paths(p, problems, registry));
    issues.extend(validate_crosslinks(p, problems, registry));
    issues
}

/// `validate --run` for practice content: every flaw drill shows its flaw and its fix works.
/// Warnings are printed; issues fail.
pub fn run_practice(p: &Practice, registry: &Registry, verbose: bool) -> Vec<Issue> {
    let mut issues = Vec::new();
    for f in &p.flaws {
        let (found, warnings, details) = crate::flaws::check_flaw(f, registry);
        for w in warnings {
            println!("warning: {w}");
        }
        if verbose || !found.is_empty() {
            println!(
                "{}: {}",
                f.def.id,
                if found.is_empty() {
                    "flaw shown, fix accepted"
                } else {
                    "FAILED"
                }
            );
            if verbose {
                for d in details {
                    println!("{d}");
                }
            }
        }
        issues.extend(found);
    }
    for o in &p.outages {
        let (found, details) = crate::outages::check_outage(o, registry, verbose);
        if verbose || !found.is_empty() {
            println!(
                "{}: {}",
                o.def.id,
                if found.is_empty() {
                    "incident replayed, fix holds"
                } else {
                    "FAILED"
                }
            );
            if verbose {
                for d in details {
                    println!("{d}");
                }
            }
        }
        issues.extend(found);
    }
    issues
}

/// The `docs` rows for `content.db`.
pub fn practice_docs(p: &Practice, registry: &Registry) -> Result<Vec<DocRow>, String> {
    let mut rows = Vec::new();
    if !p.numbers.is_empty() {
        rows.push(DocRow {
            kind: "numbers".into(),
            id: "sheet".into(),
            json: serde_json::to_string(&p.numbers).map_err(|e| e.to_string())?,
        });
    }
    for (pid, drills) in &p.estimation {
        let answers = drills
            .iter()
            .map(ka_core::estimate::answer)
            .collect::<Result<Vec<_>, _>>()?;
        let set = DrillSet {
            problem_id: pid.clone(),
            drills: drills.clone(),
            answers,
        };
        rows.push(DocRow {
            kind: "estimation".into(),
            id: pid.clone(),
            json: serde_json::to_string(&set).map_err(|e| e.to_string())?,
        });
    }
    for path in &p.paths {
        rows.push(DocRow {
            kind: "path".into(),
            id: path.id.clone(),
            json: serde_json::to_string(path).map_err(|e| e.to_string())?,
        });
    }
    for l in &p.crosslinks {
        let links: Vec<_> = l.to.iter().filter_map(|t| p.katabox.resolve(t)).collect();
        rows.push(DocRow {
            kind: "crosslinks".into(),
            id: l.from.clone(),
            json: serde_json::to_string(&links).map_err(|e| e.to_string())?,
        });
    }
    for f in &p.flaws {
        let doc = crate::flaws::flaw_doc(f, registry)?;
        rows.push(DocRow {
            kind: "flaw".into(),
            id: f.def.id.clone(),
            json: serde_json::to_string(&doc).map_err(|e| e.to_string())?,
        });
    }
    for o in &p.outages {
        let doc = crate::outages::outage_doc(o, registry)?;
        rows.push(DocRow {
            kind: "outage".into(),
            id: o.def.id.clone(),
            json: serde_json::to_string(&doc).map_err(|e| e.to_string())?,
        });
    }
    Ok(rows)
}

/// One line for `validate`.
pub fn summary(p: &Practice) -> String {
    let drills: usize = p.estimation.iter().map(|(_, d)| d.len()).sum();
    format!(
        "{} number(s), {drills} estimation drill(s) for {} problem(s), {} flaw drill(s), {} replay(s), {} learning path(s), {} KataBox link(s) valid",
        p.numbers.len(),
        p.estimation.len(),
        p.flaws.len(),
        p.outages.len(),
        p.paths.len(),
        p.crosslinks.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crosslinks_must_start_at_a_real_page_and_resolve() {
        let p = Practice {
            pattern_ids: ["cache-aside".to_string()].into(),
            katabox: toml::from_str(
                "katabox_commit = \"abc\"\n[[learn]]\nid = \"linear/lru-cache\"\ntitle = \"LRU\"\ntitle_es = \"LRU\"\n",
            )
            .unwrap(),
            crosslinks: vec![
                ka_core::crosslinks::LinkDef { from: "guide:cache".into(), to: vec!["learn:linear/lru-cache".into()] },
                ka_core::crosslinks::LinkDef { from: "pattern:cache-aside".into(), to: vec!["learn:linear/lru-cache".into()] },
                ka_core::crosslinks::LinkDef { from: "guide:teleport".into(), to: vec!["learn:x/y".into()] },
                ka_core::crosslinks::LinkDef { from: "guide:cache".into(), to: vec![] },
            ],
            ..Practice::default()
        };
        let text: Vec<String> = validate_crosslinks(&p, &[], &ka_core::registry())
            .iter()
            .map(ToString::to_string)
            .collect();
        let has = |s: &str| text.iter().any(|t| t.contains(s));
        assert!(has("guide:teleport: no such Guide entry"), "{text:#?}");
        assert!(has("learn:x/y is not in katabox-ids.toml"), "{text:#?}");
        assert!(has("guide:cache: linked twice"), "{text:#?}");
        assert!(has("guide:cache: no targets"), "{text:#?}");
        assert_eq!(text.len(), 4, "{text:#?}");
    }
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
