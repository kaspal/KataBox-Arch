//! Problem folders: `content/problems/<id>/` with
//! `problem.toml`, `statement.md`, `statement.es.md`, `hints.md`, `hints.es.md`,
//! `editorial.md`, `editorial.es.md`, `reference.kbarch` and `naive.kbarch`.

use std::collections::BTreeSet;
use std::path::Path;

use ka_core::problem::ProblemDef;
use ka_core::{Design, Registry};

use crate::{Issue, TODO_MARKER, issue};

pub const FILES: [&str; 9] = [
    "problem.toml",
    "statement.md",
    "statement.es.md",
    "hints.md",
    "hints.es.md",
    "editorial.md",
    "editorial.es.md",
    "reference.kbarch",
    "naive.kbarch",
];

#[derive(Debug, Clone)]
pub struct ProblemDir {
    /// `problems/<id>` (relative to the content root).
    pub dir: String,
    pub def: ProblemDef,
    pub statement_en: String,
    pub statement_es: String,
    pub hints_en: Vec<String>,
    pub hints_es: Vec<String>,
    pub editorial_en: String,
    pub editorial_es: String,
    pub reference: Design,
    pub naive: Design,
    /// Every file's text, for the unfinished-template check.
    texts: Vec<(String, String)>,
}

/// `## 1`, `## 2`, `## 3` sections, in order.
pub fn parse_hints(md: &str) -> Result<Vec<String>, String> {
    let mut tiers: Vec<(u32, String)> = Vec::new();
    for line in md.lines() {
        if let Some(n) = line.trim().strip_prefix("## ") {
            let n: u32 = n
                .trim()
                .parse()
                .map_err(|_| format!("hint headings are `## 1` to `## 3`, found {line:?}"))?;
            tiers.push((n, String::new()));
        } else if let Some((_, body)) = tiers.last_mut() {
            body.push_str(line);
            body.push('\n');
        } else if !line.trim().is_empty() {
            return Err("text before the first `## 1`".into());
        }
    }
    let numbers: Vec<u32> = tiers.iter().map(|(n, _)| *n).collect();
    if numbers != [1, 2, 3] {
        return Err(format!(
            "three tiers `## 1`, `## 2`, `## 3` expected, found {numbers:?}"
        ));
    }
    tiers
        .into_iter()
        .map(|(n, b)| {
            let b = b.trim().to_owned();
            if b.is_empty() {
                Err(format!("hint {n} is empty"))
            } else {
                Ok(b)
            }
        })
        .collect()
}

fn sections(md: &str) -> usize {
    md.lines().filter(|l| l.starts_with("## ")).count()
}

/// Read every problem folder under `root/problems`. Missing or unreadable files are issues.
pub fn load_problems(root: &Path, registry: &Registry) -> Result<Vec<ProblemDir>, Vec<Issue>> {
    let base = root.join("problems");
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<_> = std::fs::read_dir(&base)
        .map_err(|e| vec![issue("problems", e.to_string())])?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    let mut issues = Vec::new();
    for name in dirs {
        match load_one(&base.join(&name), &format!("problems/{name}"), registry) {
            Ok(p) => out.push(p),
            Err(mut e) => issues.append(&mut e),
        }
    }
    if issues.is_empty() {
        Ok(out)
    } else {
        Err(issues)
    }
}

fn load_one(dir: &Path, rel: &str, registry: &Registry) -> Result<ProblemDir, Vec<Issue>> {
    let mut issues = Vec::new();
    let mut texts = Vec::new();
    for f in FILES {
        match std::fs::read_to_string(dir.join(f)) {
            Ok(t) => texts.push((format!("{rel}/{f}"), t)),
            Err(e) => issues.push(issue(
                format!("{rel}/{f}"),
                format!("missing or unreadable: {e}"),
            )),
        }
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let text = |f: &str| {
        texts
            .iter()
            .find(|(p, _)| p.ends_with(&format!("/{f}")))
            .map(|(_, t)| t.clone())
            .unwrap_or_default()
    };
    let def: ProblemDef = toml::from_str(&text("problem.toml"))
        .map_err(|e| vec![issue(format!("{rel}/problem.toml"), e.to_string())])?;
    let hints = |f: &str| parse_hints(&text(f)).map_err(|m| issue(format!("{rel}/{f}"), m));
    let design = |f: &str| {
        ka_app::kbarch::parse(text(f).as_bytes(), registry)
            .map_err(|e| issue(format!("{rel}/{f}"), format!("{:?}: {}", e.code, e.message)))
    };
    let (hints_en, hints_es, reference, naive) = (
        hints("hints.md"),
        hints("hints.es.md"),
        design("reference.kbarch"),
        design("naive.kbarch"),
    );
    for r in [&hints_en.as_ref().err(), &hints_es.as_ref().err()]
        .into_iter()
        .flatten()
    {
        issues.push((*r).clone());
    }
    for r in [&reference.as_ref().err(), &naive.as_ref().err()]
        .into_iter()
        .flatten()
    {
        issues.push((*r).clone());
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(ProblemDir {
        dir: rel.into(),
        statement_en: text("statement.md"),
        statement_es: text("statement.es.md"),
        editorial_en: text("editorial.md"),
        editorial_es: text("editorial.es.md"),
        hints_en: hints_en.unwrap_or_default(),
        hints_es: hints_es.unwrap_or_default(),
        reference: reference.expect("checked"),
        naive: naive.expect("checked"),
        def,
        texts,
    })
}

/// Structural checks (no judging): ids, EN/ES parity, routes, designs, templates.
pub fn validate_problems(problems: &[ProblemDir], registry: &Registry) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut numbers = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for p in problems {
        let d = &p.def;
        let at = |f: &str| format!("{}/{f}", p.dir);
        let mut bad = |f: &str, m: String| issues.push(issue(at(f), m));
        let folder = p.dir.rsplit('/').next().unwrap_or_default();
        if d.id != folder {
            bad(
                "problem.toml",
                format!("id {:?} differs from the folder {folder:?}", d.id),
            );
        }
        if !ids.insert(d.id.clone()) {
            bad("problem.toml", format!("duplicate id {:?}", d.id));
        }
        if !numbers.insert((d.track, d.set, d.number)) {
            bad(
                "problem.toml",
                format!(
                    "{} set {} number {} is taken",
                    d.track.id(),
                    d.set,
                    d.number
                ),
            );
        }
        for (en, es, what) in [
            (&d.title, &d.title_es, "title"),
            (&d.teaches, &d.teaches_es, "teaches"),
        ] {
            if en.trim().is_empty() || es.trim().is_empty() {
                bad("problem.toml", format!("{what} needs English and Spanish"));
            }
        }
        // Zone chaos: `@primary`, or a Region/AZ of the reference (players can't be expected
        // to match other names, so `@primary` is the one to use; see problem-authoring.md).
        for t in &d.tests {
            for c in &t.chaos {
                let zone = match c {
                    ka_core::scenario::ChaosEvent::Partition { zone, .. }
                    | ka_core::scenario::ChaosEvent::Split { zone, .. } => zone,
                    _ => continue,
                };
                let known = zone == "@primary"
                    || p.reference
                        .groups
                        .iter()
                        .any(|g| &g.id == zone || &g.label == zone);
                if !known {
                    bad(
                        "problem.toml",
                        format!(
                            "test {}: zone {zone:?} is neither @primary nor a group of the reference",
                            t.id
                        ),
                    );
                }
            }
        }
        for m in loose_limits(d) {
            bad("problem.toml", m);
        }
        // Request types.
        let mut type_ids = BTreeSet::new();
        let mut share = 0.0;
        for t in &d.request_types {
            if !type_ids.insert(t.id.as_str()) {
                bad("problem.toml", format!("request type {:?} twice", t.id));
            }
            share += t.share;
            if t.name.is_some() != t.name_es.is_some() {
                bad(
                    "problem.toml",
                    format!("request type {:?}: name needs both languages", t.id),
                );
            }
            for step in &t.route {
                if let Some(name) = step
                    .trim_end_matches('?')
                    .split('|')
                    .find_map(|a| a.strip_prefix("external:"))
                    && !d.externals.iter().any(|x| x.name == name)
                {
                    bad(
                        "problem.toml",
                        format!(
                            "request type {:?} calls external {name:?}, which isn't declared",
                            t.id
                        ),
                    );
                }
            }
        }
        if d.request_types.is_empty() {
            bad("problem.toml", "no request types".into());
        } else if (share - 1.0).abs() > 1e-6 {
            bad(
                "problem.toml",
                format!("request type shares add up to {share}, not 1"),
            );
        }
        // Tests.
        let mut test_ids = BTreeSet::new();
        for t in &d.tests {
            if !test_ids.insert(t.id.as_str()) {
                bad("problem.toml", format!("test {:?} twice", t.id));
            }
            if t.name.trim().is_empty() || t.name_es.trim().is_empty() {
                bad(
                    "problem.toml",
                    format!("test {:?} needs English and Spanish names", t.id),
                );
            }
            for m in t.type_multipliers.keys() {
                if !type_ids.contains(m.as_str()) {
                    bad(
                        "problem.toml",
                        format!("test {:?} multiplies unknown request type {m:?}", t.id),
                    );
                }
            }
        }
        if !d.tests.iter().any(|t| t.hidden) {
            bad("problem.toml", "needs at least one hidden test".into());
        }
        if !d.tests.iter().any(|t| !t.hidden) {
            bad("problem.toml", "needs at least one visible test".into());
        }
        // EN/ES parity of the prose.
        for (en, es, f) in [
            (&p.statement_en, &p.statement_es, "statement.es.md"),
            (&p.editorial_en, &p.editorial_es, "editorial.es.md"),
        ] {
            if es.trim().is_empty() || en.trim().is_empty() {
                bad(f, "empty".into());
            } else if sections(en) != sections(es) {
                bad(
                    f,
                    format!(
                        "{} sections, the English has {}",
                        sections(es),
                        sections(en)
                    ),
                );
            }
        }
        // Designs: valid, and every request type has a path through the reference and the
        // naive design (the naive one fails a hidden test, never the lint).
        for (design, f) in [
            (&p.reference, "reference.kbarch"),
            (&p.naive, "naive.kbarch"),
        ] {
            if let Some(i) = ka_core::validate(design, registry).first() {
                bad(f, format!("invalid design: {i:?}"));
                continue;
            }
            if let Some(t) = d.tests.first()
                && let Err(e) = ka_sim::compile(
                    design,
                    registry,
                    &d.scenario(t),
                    &ka_sim::SimOptions::default(),
                )
            {
                bad(f, e.to_string());
            }
        }
        for k in &d.requires {
            if !p.reference.nodes.iter().any(|n| n.kind == *k) {
                bad(
                    "reference.kbarch",
                    format!("the problem requires a {k:?} card"),
                );
            }
        }
        for (path, text) in &p.texts {
            if text.contains(TODO_MARKER) {
                issues.push(issue(
                    path.clone(),
                    format!("unfinished: contains {TODO_MARKER}"),
                ));
            }
        }
    }
    issues.sort();
    issues
}

impl ProblemDir {
    /// The `content.db` row. `ref_cost` and `ref_p95_ms` are the reference's monthly cost and
    /// p95 on the first test.
    pub fn row(&self, registry: &Registry) -> Result<ka_store::ProblemRow, String> {
        let d = &self.def;
        let first = d.tests.first().ok_or("no tests")?;
        let run =
            ka_judge::run_test(d, first, &self.reference, registry).map_err(|e| e.to_string())?;
        Ok(ka_store::ProblemRow {
            id: d.id.clone(),
            number: i64::from(d.number),
            set: i64::from(d.set),
            difficulty: serde_json::to_value(d.difficulty)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            title_en: d.title.clone(),
            title_es: d.title_es.clone(),
            teaches_en: d.teaches.clone(),
            teaches_es: d.teaches_es.clone(),
            tags: d.tags.clone(),
            nfr: serde_json::to_string(&d.nfr).map_err(|e| e.to_string())?,
            def: serde_json::to_string(d).map_err(|e| e.to_string())?,
            statement_en: self.statement_en.clone(),
            statement_es: self.statement_es.clone(),
            hints_en: self.hints_en.clone(),
            hints_es: self.hints_es.clone(),
            editorial_en: self.editorial_en.clone(),
            editorial_es: self.editorial_es.clone(),
            reference: String::from_utf8(
                ka_app::kbarch::to_bytes(&self.reference, registry).map_err(|e| e.message)?,
            )
            .map_err(|e| e.to_string())?,
            ref_cost: run.cost_month,
            ref_p95_ms: run.latency.p95_ms,
            track: d.track.id().into(),
            // M8.5 sets this from the problem's `challenge.kbarch`.
            challenge_ratio: None,
        })
    }
}

/// Review F3: an error limit a learner can read. Above 10% (or availability under 90%) the
/// limit is usually covering retried attempts, which aren't errors; judge `lost` instead.
pub fn loose_limits(def: &ka_core::problem::ProblemDef) -> Vec<String> {
    let mut out = Vec::new();
    for t in &def.tests {
        let p = def.pass_for(t);
        if p.error_rate.is_some_and(|e| e > 0.1) {
            out.push(format!("test {}: an error limit above 10%", t.id));
        }
        if p.availability.is_some_and(|a| a < 0.9) {
            out.push(format!("test {}: an availability limit under 90%", t.id));
        }
    }
    out
}
